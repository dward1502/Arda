// sigil: REPAIR
use crate::soterion::{parse_header_from_path, SoterionIndex};
use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc::{self, Sender};
use tracing::{info, warn};

pub struct SoterionWatcher {
    tx: Sender<PathBuf>,
    watcher: Option<RecommendedWatcher>,
}

impl SoterionWatcher {
    pub fn new(index: Arc<Mutex<SoterionIndex>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<PathBuf>(32);

        let watcher_index = index.clone();
        tokio::spawn(async move {
            while let Some(path) = rx.recv().await {
                if let Ok(Some(meta)) = parse_header_from_path(&path) {
                    let mut idx = lock_or_recover(&watcher_index, "soterion_index");
                    idx.add(path.to_string_lossy().into_owned(), &meta);
                    idx.save(default_soterion_index_path()).ok(); // Best-effort save
                    info!("Soterion watcher updated index for {}", path.display());
                }
            }
        });

        Self { tx, watcher: None }
    }

    pub fn start_watching(&mut self, paths: Vec<impl AsRef<Path>>) -> Result<()> {
        let tx = self.tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if let notify::EventKind::Modify(_) | notify::EventKind::Create(_) = event.kind
                    {
                        for path in event.paths {
                            if path.extension().is_some_and(|e| e == "md" || e == "jsonl") {
                                let _ = tx.try_send(path);
                            }
                        }
                    }
                }
                Err(e) => warn!("Watcher error: {}", e),
            },
            Config::default(),
        )?;

        for p in paths {
            watcher.watch(p.as_ref(), RecursiveMode::Recursive)?;
            info!("Watching Soterion directory: {}", p.as_ref().display());
        }

        self.watcher = Some(watcher);
        Ok(())
    }
}

fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!(label, "recovering from poisoned soterion watcher mutex");
            poisoned.into_inner()
        }
    }
}

fn default_soterion_index_path() -> String {
    std::env::var("ARDA_SOTERION_INDEX_PATH")
        .unwrap_or_else(|_| "data/soterion_index.json".to_string())
}

#[cfg(test)]
mod tests {
    use super::{default_soterion_index_path, lock_or_recover, SoterionWatcher};
    use crate::soterion::SoterionIndex;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
    use uuid::Uuid;

    fn temp_path(label: &str, ext: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "arda-core-soterion-watcher-{label}-{}.{}",
            Uuid::new_v4(),
            ext
        ))
    }

    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let lock = env_lock().lock().expect("env lock");
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                _lock: lock,
                key,
                previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn watcher_processes_markdown_path_from_channel_and_persists_index() {
        let index_path = temp_path("index", "json");
        let source_path = temp_path("source", "md");
        let _env = EnvGuard::set(
            "ARDA_SOTERION_INDEX_PATH",
            index_path.to_string_lossy().as_ref(),
        );

        fs::write(
            &source_path,
            r#"---
soterion:
  sigil: "𓁿"
  realm: "knowledge"
  tags: ["watcher", "unit"]
---
body
"#,
        )
        .expect("write source");

        let index = Arc::new(Mutex::new(SoterionIndex::new()));
        let watcher = SoterionWatcher::new(Arc::clone(&index));
        watcher
            .tx
            .send(source_path.clone())
            .await
            .expect("send path");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let idx = lock_or_recover(index.as_ref(), "test_index");
        let source = source_path.to_string_lossy().to_string();
        assert!(idx.find_by_sigil("𓁿").contains(&source));
        assert!(idx.find_by_realm("knowledge").contains(&source));
        assert!(idx.find_by_tag("watcher").contains(&source));
        drop(idx);

        let persisted = fs::read_to_string(&index_path).expect("persisted index");
        assert!(persisted.contains("knowledge"));
        assert!(persisted.contains("watcher"));

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_file(index_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn watcher_recovers_from_poisoned_index_mutex() {
        let index_path = temp_path("poisoned-index", "json");
        let source_path = temp_path("poisoned-source", "md");
        let _env = EnvGuard::set(
            "ARDA_SOTERION_INDEX_PATH",
            index_path.to_string_lossy().as_ref(),
        );

        fs::write(
            &source_path,
            r#"---
soterion:
  sigil: "𓂀"
  realm: "ingest"
---
body
"#,
        )
        .expect("write source");

        let index = Arc::new(Mutex::new(SoterionIndex::new()));
        let poisoned_index = Arc::clone(&index);
        let _ = std::thread::spawn(move || {
            let _guard = poisoned_index.lock().unwrap();
            panic!("poison watcher index");
        })
        .join();

        let watcher = SoterionWatcher::new(Arc::clone(&index));
        watcher
            .tx
            .send(source_path.clone())
            .await
            .expect("send path");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let idx = lock_or_recover(index.as_ref(), "test_index");
        let source = source_path.to_string_lossy().to_string();
        assert!(idx.find_by_sigil("𓂀").contains(&source));
        drop(idx);

        assert!(fs::metadata(&index_path).is_ok());

        let _ = fs::remove_file(source_path);
        let _ = fs::remove_file(index_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn start_watching_retains_watcher_handle() {
        let dir = temp_path("watch-dir", "tmp");
        fs::create_dir_all(&dir).expect("create dir");

        let index = Arc::new(Mutex::new(SoterionIndex::new()));
        let mut watcher = SoterionWatcher::new(index);
        watcher.start_watching(vec![&dir]).expect("start watching");

        assert!(watcher.watcher.is_some());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn default_soterion_index_path_prefers_env_override() {
        let path = temp_path("env-index", "json");
        let _env = EnvGuard::set(
            "ARDA_SOTERION_INDEX_PATH",
            path.to_string_lossy().as_ref(),
        );
        assert_eq!(default_soterion_index_path(), path.to_string_lossy());
    }
}
