// sigil: REPAIR
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, MutexGuard, OnceLock,
};
use std::time::{Duration, Instant};

#[derive(Default)]
struct GateState {
    in_flight: AtomicUsize,
}

struct GatePermit {
    gate: Arc<GateState>,
}

impl Drop for GatePermit {
    fn drop(&mut self) {
        self.gate.in_flight.fetch_sub(1, Ordering::Release);
    }
}

#[derive(Debug, Default)]
struct PressureSnapshot {
    local_joule_pressure: bool,
    local_joule_usage_percent: Option<f64>,
    pressure_status: Option<String>,
    disk_used_pct: Option<f64>,
    swap_ok: bool,
    mem_available_pct: Option<f64>,
}

fn registry() -> &'static Mutex<HashMap<&'static str, Arc<GateState>>> {
    #[allow(clippy::type_complexity)]
    static REGISTRY: OnceLock<Mutex<HashMap<&'static str, Arc<GateState>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn pressure_cache() -> &'static Mutex<Option<(Instant, Arc<PressureSnapshot>)>> {
    #[allow(clippy::type_complexity)]
    static CACHE: OnceLock<Mutex<Option<(Instant, Arc<PressureSnapshot>)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(label, "recovering from poisoned background mutex");
            poisoned.into_inner()
        }
    }
}

fn gate_for(label: &'static str) -> Arc<GateState> {
    let mut registry = lock_or_recover(registry(), label);
    registry
        .entry(label)
        .or_insert_with(|| Arc::new(GateState::default()))
        .clone()
}

impl GateState {
    fn try_acquire(self: &Arc<Self>, limit: usize) -> Option<GatePermit> {
        let limit = limit.max(1);
        loop {
            let current = self.in_flight.load(Ordering::Acquire);
            if current >= limit {
                return None;
            }
            if self
                .in_flight
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Some(GatePermit {
                    gate: Arc::clone(self),
                });
            }
        }
    }
}

pub fn try_run_bounded<F, T>(label: &'static str, limit: usize, work: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    let (effective_limit, snapshot) = effective_limit(limit);
    let gate = gate_for(label);
    let Some(_permit) = gate.try_acquire(effective_limit) else {
        emit_shed_receipt(label, "sync", limit, effective_limit, snapshot.as_ref());
        tracing::warn!(
            label,
            limit = limit.max(1),
            effective_limit,
            local_joule_pressure = snapshot.local_joule_pressure,
            local_joule_usage_percent = snapshot.local_joule_usage_percent,
            pressure_status = snapshot.pressure_status.as_deref().unwrap_or("unknown"),
            disk_used_pct = snapshot.disk_used_pct,
            mem_available_pct = snapshot.mem_available_pct,
            swap_ok = snapshot.swap_ok,
            "bounded work rejected because concurrency gate is saturated"
        );
        return None;
    };
    Some(work())
}

pub async fn try_run_bounded_async<F, Fut, T>(
    label: &'static str,
    limit: usize,
    work: F,
) -> Option<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let (effective_limit, snapshot) = effective_limit(limit);
    let gate = gate_for(label);
    let Some(_permit) = gate.try_acquire(effective_limit) else {
        emit_shed_receipt(label, "async", limit, effective_limit, snapshot.as_ref());
        tracing::warn!(
            label,
            limit = limit.max(1),
            effective_limit,
            local_joule_pressure = snapshot.local_joule_pressure,
            local_joule_usage_percent = snapshot.local_joule_usage_percent,
            pressure_status = snapshot.pressure_status.as_deref().unwrap_or("unknown"),
            disk_used_pct = snapshot.disk_used_pct,
            mem_available_pct = snapshot.mem_available_pct,
            swap_ok = snapshot.swap_ok,
            "bounded async work rejected because concurrency gate is saturated"
        );
        return None;
    };
    Some(work().await)
}

pub fn spawn_bounded_background<F, Fut>(label: &'static str, limit: usize, factory: F) -> bool
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (effective_limit, snapshot) = effective_limit(limit);
    let gate = gate_for(label);
    let Some(permit) = gate.try_acquire(effective_limit) else {
        emit_shed_receipt(
            label,
            "background",
            limit,
            effective_limit,
            snapshot.as_ref(),
        );
        tracing::warn!(
            label,
            limit = limit.max(1),
            effective_limit,
            local_joule_pressure = snapshot.local_joule_pressure,
            local_joule_usage_percent = snapshot.local_joule_usage_percent,
            pressure_status = snapshot.pressure_status.as_deref().unwrap_or("unknown"),
            disk_used_pct = snapshot.disk_used_pct,
            mem_available_pct = snapshot.mem_available_pct,
            swap_ok = snapshot.swap_ok,
            "background task dropped because concurrency gate is saturated"
        );
        return false;
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let _permit = permit;
            factory().await;
        });
        true
    } else {
        std::thread::spawn(move || {
            let _permit = permit;
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                tracing::debug!(
                    label,
                    "failed to build fallback runtime for background task"
                );
                return;
            };
            runtime.block_on(factory());
        });
        true
    }
}

fn effective_limit(limit: usize) -> (usize, Arc<PressureSnapshot>) {
    let base = limit.max(1);
    let snapshot = current_pressure_snapshot();
    if !pressure_admission_enabled() {
        return (base, snapshot);
    }

    let severe_factor = env_factor("ANNUNIMAS_PRESSURE_ADMISSION_SEVERE_FACTOR", 0.25);
    let degraded_factor = env_factor("ANNUNIMAS_PRESSURE_ADMISSION_DEGRADED_FACTOR", 0.5);
    let mem_degraded = env_factor(
        "ANNUNIMAS_PRESSURE_ADMISSION_MEM_AVAILABLE_DEGRADED_PCT",
        10.0,
    );
    let mem_severe = env_factor("ANNUNIMAS_PRESSURE_ADMISSION_MEM_AVAILABLE_SEVERE_PCT", 5.0);

    let severe = !snapshot.swap_ok
        || snapshot.local_joule_pressure
        || snapshot
            .local_joule_usage_percent
            .is_some_and(|value| value >= 100.0)
        || snapshot
            .pressure_status
            .as_deref()
            .is_some_and(|value| matches!(value, "alert" | "block"))
        || snapshot.disk_used_pct.is_some_and(|value| value >= 92.0)
        || snapshot
            .mem_available_pct
            .is_some_and(|value| value <= mem_severe);

    let degraded = severe
        || snapshot
            .pressure_status
            .as_deref()
            .is_some_and(|value| value != "ok")
        || snapshot
            .local_joule_usage_percent
            .is_some_and(|value| value >= 80.0)
        || snapshot.disk_used_pct.is_some_and(|value| value >= 90.0)
        || snapshot
            .mem_available_pct
            .is_some_and(|value| value <= mem_degraded);

    let effective = if severe {
        scaled_limit(base, severe_factor)
    } else if degraded {
        scaled_limit(base, degraded_factor)
    } else {
        base
    };

    (effective, snapshot)
}

fn scaled_limit(base: usize, factor: f64) -> usize {
    let scaled = ((base as f64) * factor).floor() as usize;
    scaled.clamp(1, base)
}

fn pressure_admission_enabled() -> bool {
    std::env::var("ANNUNIMAS_PRESSURE_ADMISSION_ENABLED")
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

fn env_factor(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(default)
}

fn current_pressure_snapshot() -> Arc<PressureSnapshot> {
    let ttl_ms = std::env::var("ANNUNIMAS_PRESSURE_ADMISSION_CACHE_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(1000);
    let ttl = Duration::from_millis(ttl_ms.max(100));
    let cache = pressure_cache();
    {
        let guard = lock_or_recover(cache, "pressure_admission_cache");
        if let Some((captured_at, snapshot)) = guard.as_ref() {
            if captured_at.elapsed() < ttl {
                return Arc::clone(snapshot);
            }
        }
    }

    let snapshot = Arc::new(load_pressure_snapshot());
    let mut guard = lock_or_recover(cache, "pressure_admission_cache");
    *guard = Some((Instant::now(), Arc::clone(&snapshot)));
    snapshot
}

fn load_pressure_snapshot() -> PressureSnapshot {
    let budget_path = std::env::var("ANNUNIMAS_PRESSURE_ADMISSION_BUDGET_PATH")
        .or_else(|_| std::env::var("ANNUNIMAS_LAUNCH_PREFLIGHT_BUDGET_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("core/state/runtime_budget_policy.json"));
    let pressure_path = std::env::var("ANNUNIMAS_PRESSURE_ADMISSION_PRESSURE_PATH")
        .or_else(|_| std::env::var("ANNUNIMAS_LAUNCH_PREFLIGHT_PRESSURE_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("core/state/runtime_admission_pressure.json"));

    let budget = read_json_file(&budget_path);
    let pressure = read_json_file(&pressure_path);

    let budget_summary = budget.get("summary").and_then(Value::as_object);
    let user_plan_budget = budget.get("user_plan_budget").and_then(Value::as_object);
    let pressure_observed = pressure.get("observed").and_then(Value::as_object);

    PressureSnapshot {
        local_joule_pressure: budget_summary
            .and_then(|summary| summary.get("local_joule_pressure"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        local_joule_usage_percent: user_plan_budget
            .and_then(|budget| budget.get("local_joulework_usage_percent"))
            .and_then(Value::as_f64),
        pressure_status: pressure
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string),
        disk_used_pct: pressure_observed
            .and_then(|observed| observed.get("disk_used_pct"))
            .and_then(Value::as_f64),
        swap_ok: swap_ok(),
        mem_available_pct: mem_available_pct(),
    }
}

fn read_json_file(path: &PathBuf) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null)
}

fn swap_ok() -> bool {
    fs::read_to_string("/proc/swaps")
        .ok()
        .map(|raw| raw.lines().skip(1).any(|line| !line.trim().is_empty()))
        .unwrap_or(false)
}

fn mem_available_pct() -> Option<f64> {
    let raw = fs::read_to_string("/proc/meminfo").ok()?;
    let mut available_kib = None;
    let mut total_kib = None;
    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("MemAvailable:") {
            available_kib = parse_meminfo_kib(value);
        } else if let Some(value) = line.strip_prefix("MemTotal:") {
            total_kib = parse_meminfo_kib(value);
        }
    }
    let available = available_kib?;
    let total = total_kib?;
    if total == 0 {
        return None;
    }
    Some((available as f64 / total as f64) * 100.0)
}

fn parse_meminfo_kib(raw: &str) -> Option<u64> {
    raw.split_whitespace().next()?.parse::<u64>().ok()
}

fn emit_shed_receipt(
    label: &'static str,
    mode: &'static str,
    configured_limit: usize,
    effective_limit: usize,
    snapshot: &PressureSnapshot,
) {
    let path = std::env::var("ANNUNIMAS_PRESSURE_ADMISSION_RECEIPTS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            workspace_root().join("data/prometheus/runtime_admission_shed_receipts.jsonl")
        });
    let entry = serde_json::json!({
        "ts_utc": chrono::Utc::now().to_rfc3339(),
        "label": label,
        "mode": mode,
        "event": "shed",
        "configured_limit": configured_limit.max(1),
        "effective_limit": effective_limit.max(1),
        "pressure": {
            "local_joule_pressure": snapshot.local_joule_pressure,
            "local_joule_usage_percent": snapshot.local_joule_usage_percent,
            "pressure_status": snapshot.pressure_status,
            "disk_used_pct": snapshot.disk_used_pct,
            "mem_available_pct": snapshot.mem_available_pct,
            "swap_ok": snapshot.swap_ok,
        }
    });

    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = writeln!(file, "{}", entry);
}

fn workspace_root() -> PathBuf {
    std::env::var("ANNUNIMAS_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|path| path.parent())
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        })
}

#[cfg(test)]
mod tests {
    use super::{
        current_pressure_snapshot, gate_for, pressure_cache, registry, scaled_limit,
        spawn_bounded_background, try_run_bounded, try_run_bounded_async,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::{oneshot, Mutex};

    #[test]
    fn scaled_limit_never_drops_below_one() {
        assert_eq!(scaled_limit(1, 0.25), 1);
        assert_eq!(scaled_limit(4, 0.25), 1);
        assert_eq!(scaled_limit(8, 0.5), 4);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bounded_background_gate_rejects_excess_parallel_tasks() {
        let hits = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = oneshot::channel::<()>();
        let rx = Arc::new(Mutex::new(Some(rx)));

        let first_hits = Arc::clone(&hits);
        let first_rx = Arc::clone(&rx);
        let first = spawn_bounded_background("test_gate_rejects_excess", 1, move || async move {
            first_hits.fetch_add(1, Ordering::SeqCst);
            let mut guard = first_rx.lock().await;
            if let Some(rx) = guard.take() {
                let _ = rx.await;
            }
        });

        let second_hits = Arc::clone(&hits);
        let second = spawn_bounded_background("test_gate_rejects_excess", 1, move || async move {
            second_hits.fetch_add(1, Ordering::SeqCst);
        });

        assert!(first);
        assert!(!second);
        let _ = tx.send(());
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn bounded_sync_gate_rejects_excess_parallel_tasks() {
        let first = try_run_bounded("test_sync_gate_rejects_excess", 1, || 42);
        let second = try_run_bounded("test_sync_gate_rejects_excess", 1, || 7);
        assert_eq!(first, Some(42));
        assert_eq!(second, Some(7));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bounded_async_gate_runs_work() {
        let out = try_run_bounded_async("test_async_gate_runs", 1, || async { 99 }).await;
        assert_eq!(out, Some(99));
    }

    #[test]
    fn gate_registry_recovers_from_poisoned_mutex() {
        let _ = std::thread::spawn(|| {
            let _guard = registry().lock().unwrap();
            panic!("poison gate registry");
        })
        .join();

        let gate = gate_for("test_registry_poison_recovery");
        assert!(gate.try_acquire(1).is_some());
    }

    #[test]
    fn pressure_cache_recovers_from_poisoned_mutex() {
        let _ = std::thread::spawn(|| {
            let _guard = pressure_cache().lock().unwrap();
            panic!("poison pressure cache");
        })
        .join();

        let _snapshot = current_pressure_snapshot();
        let guard = match pressure_cache().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        assert!(guard.is_some());
    }
}
