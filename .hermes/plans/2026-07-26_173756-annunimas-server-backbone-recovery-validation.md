# Annunimas Server Backbone Recovery Validation Plan

> **For Hermes:** Use `arda-fleet-state-sync` and execute this plan task-by-task. This is a bounded operational recovery check, not a configuration migration.

**Goal:** Reintroduce the `annunimas-server` backbone lane after the completed degraded-fleet verification, prove direct and Manwe-qualified inference, check automatic routing remains healthy, and make an evidence-based keep-on/roll-back decision.

**Architecture:** Preserve Arda/Manwe as the sole runtime-state authority. Start only the canonical remote `llama-server-bonsai27.service` lane on `annunimas-server`, reconcile Manwe, test the provider directly and through Manwe, and compare runtime/worktree evidence against a preflight baseline. Do not revive retired Charon services, retired ports, or a local `annunimas-server.service`—no such local unit is installed.

**Canonical contract:**

- Fleet node: `node-backbone-server`
- Remote host: `annunimas-server@100.102.250.115`
- Remote unit: `llama-server-bonsai27.service`
- Direct endpoint: `http://100.102.250.115:8095/v1`
- Provider: `edge_backbone`
- Model: `ternary-bonsai-27b-q2_0`
- Manwe endpoint: `http://127.0.0.1:5110`
- Canonical config: `config/fleet.toml:156-183`, `config/manwe.providers.toml:73-92`

**Non-goals:** No source edits, provider renames, model swaps, context expansion, enabling of retired lanes, queue mutation, commit, or push.

---

## Safety and stop conditions

Stop and roll back the remote lane if any of these occurs:

- The remote unit enters a restart loop, reports CUDA/GGUF allocation failure, or never becomes ready within 120 seconds.
- More than the canonical `:8095` inference listener appears among retired ports `:8081`, `:8093`, `:8094`, `:8095`.
- `/v1/models` does not report `ternary-bonsai-27b-q2_0`.
- A forced Manwe request falls back from `edge_backbone`; forced fallback must remain disabled.
- Starting the lane makes Manwe, metrics, Hermes, or either Arda timer unhealthy.
- Queue content changes.
- The probe creates unexplained source/config changes beyond known runtime receipt/projection files.

Rollback means `ssh "$REMOTE" 'systemctl --user stop llama-server-bonsai27.service'`; do not disable the unit unless its preflight state was disabled and it was accidentally enabled during execution.

---

### Task 1: Capture the pre-recovery baseline

**Objective:** Record enough evidence to distinguish recovery effects from the already-dirty worktree and concurrent runtime writers.

**Files:**

- Read: `config/fleet.toml:156-183`
- Read: `config/manwe.providers.toml:73-92`
- Create runtime-only evidence under: `/tmp/arda-backbone-recovery/`
- Do not edit repository files.

**Step 1: Establish constants and evidence directory**

```bash
cd /var/home/mythos/Eregion/Arda
export REMOTE='annunimas-server@100.102.250.115'
export DIRECT_BASE='http://100.102.250.115:8095/v1'
export MANWE_BASE='http://127.0.0.1:5110'
export EVIDENCE='/tmp/arda-backbone-recovery'
rm -rf "$EVIDENCE"
mkdir -p "$EVIDENCE"
```

Expected: empty evidence directory created outside the repository.

**Step 2: Capture Git status and dirty-file fingerprints**

```bash
git status --porcelain=v1 > "$EVIDENCE/git-before.txt"
git diff --check > "$EVIDENCE/diff-check-before.txt"
git status --porcelain=v1 | cut -c4- | while IFS= read -r path; do
  test -f "$path" && sha256sum "$path"
done > "$EVIDENCE/dirty-sha256-before.txt"
sha256sum core/projects/tasks/queue.jsonl > "$EVIDENCE/queue-before.sha256"
```

Expected: `git diff --check` exits 0; the queue baseline is recorded without modifying it.

**Step 3: Capture canonical local service state**

```bash
systemctl --user --machine="$(id -un)@.host" is-active \
  arda-manwe.service \
  arda-metrics-exporter.service \
  hermes-gateway.service \
  arda-aule-autopilot-read-only.timer \
  arda-manwe-inference-probe.timer | tee "$EVIDENCE/local-units-before.txt"
curl -fsS "$MANWE_BASE/healthz" > "$EVIDENCE/manwe-health-before.json"
curl -fsS http://127.0.0.1:9101/health/audit > "$EVIDENCE/audit-health-before.json"
```

Expected: all five units/timers report `active`; Manwe reports `"ok":true`; audit health reports `"status":"ok"`.

---

### Task 2: Verify remote reachability and the exact recovery target

**Objective:** Ensure the physical host and canonical unit are reachable before any mutation.

**Step 1: Check Tailscale and SSH reachability**

```bash
tailscale ping -c 3 100.102.250.115 | tee "$EVIDENCE/tailscale-ping.txt"
ssh -o BatchMode=yes -o ConnectTimeout=10 "$REMOTE" 'hostname; id -un; systemctl --user is-system-running || true' \
  | tee "$EVIDENCE/remote-access.txt"
```

Expected: the Tailscale node responds and SSH identifies the intended host/user. If the node itself is powered off or unreachable, stop; powering it on is a separate physical prerequisite.

**Step 2: Capture unit and GPU state without starting anything**

```bash
ssh "$REMOTE" '
  systemctl --user show llama-server-bonsai27.service \
    -p LoadState -p UnitFileState -p ActiveState -p SubState \
    -p MainPID -p NRestarts -p FragmentPath -p ExecStart;
  nvidia-smi;
  nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv,noheader || true
' | tee "$EVIDENCE/remote-before.txt"
```

Expected: `LoadState=loaded`; preserve the observed `UnitFileState`; no assumption is made from restart counters alone.

**Step 3: Confirm retired lanes remain absent**

```bash
ssh "$REMOTE" "ss -ltnp | grep -E ':(8081|8093|8094|8095)\\b' || true" \
  | tee "$EVIDENCE/listeners-before.txt"
```

Expected while degraded: no listener on `:8095`; no retired listener on `:8081`, `:8093`, or `:8094`.

---

### Task 3: Start only the canonical backbone lane

**Objective:** Start the existing canonical service without changing enablement or any Arda configuration.

**Step 1: Start the unit**

```bash
ssh "$REMOTE" 'systemctl --user start llama-server-bonsai27.service'
```

Expected: command exits 0. Do not run `enable`, `enable --now`, or start any other model service.

**Step 2: Wait up to 120 seconds for readiness**

```bash
deadline=$((SECONDS + 120))
until curl -4 --noproxy '*' -fsS --connect-timeout 2 --max-time 5 \
  "$DIRECT_BASE/models" > "$EVIDENCE/direct-models.json"; do
  if (( SECONDS >= deadline )); then
    ssh "$REMOTE" 'systemctl --user status llama-server-bonsai27.service --no-pager -l; journalctl --user -u llama-server-bonsai27.service -n 80 --no-pager' \
      | tee "$EVIDENCE/remote-start-failure.txt"
    ssh "$REMOTE" 'systemctl --user stop llama-server-bonsai27.service'
    exit 1
  fi
  sleep 2
done
```

Expected: `/v1/models` becomes available within 120 seconds.

**Step 3: Validate exact model and listener uniqueness**

```bash
python3 - <<'PY'
import json
p = '/tmp/arda-backbone-recovery/direct-models.json'
data = json.load(open(p))
ids = {item.get('id') for item in data.get('data', [])}
assert 'ternary-bonsai-27b-q2_0' in ids, ids
print('model_ok=ternary-bonsai-27b-q2_0')
PY
ssh "$REMOTE" "ss -ltnp | grep -E ':(8081|8093|8094|8095)\\b'" \
  | tee "$EVIDENCE/listeners-after-start.txt"
```

Expected: exact model assertion passes; only `:8095` is listening among the four candidate inference ports.

---

### Task 4: Reconcile Manwe and verify provider readiness

**Objective:** Make Manwe observe the recovered lane without restarting the router or changing provider configuration.

**Step 1: Request live reconciliation**

```bash
curl -fsS -X POST "$MANWE_BASE/reload_config" > "$EVIDENCE/manwe-reload.json"
curl -fsS -X POST "$MANWE_BASE/reconcile_catalogs" > "$EVIDENCE/manwe-reconcile.json"
```

Expected: both requests succeed. If either endpoint rejects the request, inspect its JSON response before considering a Manwe restart; do not restart reflexively.

**Step 2: Capture provider and health evidence**

```bash
curl -fsS "$MANWE_BASE/providers" > "$EVIDENCE/manwe-providers.json"
curl -fsS "$MANWE_BASE/healthz" > "$EVIDENCE/manwe-health-recovered.json"
python3 - <<'PY'
import json
p = json.load(open('/tmp/arda-backbone-recovery/manwe-providers.json'))
text = json.dumps(p)
assert 'edge_backbone' in text, 'edge_backbone absent from Manwe provider state'
assert 'ternary-bonsai-27b-q2_0' in text, 'canonical model absent from Manwe provider state'
print('catalog_contains=edge_backbone/ternary-bonsai-27b-q2_0')
PY
```

Expected: Manwe catalog contains the canonical provider/model and overall health remains `ok`.

---

### Task 5: Run a strict qualified backbone inference

**Objective:** Prove Manwe can route to the recovered backbone without silently falling back.

**Step 1: Send one bounded forced request**

```bash
curl -4 --noproxy '*' --fail-with-body --connect-timeout 3 --max-time 90 \
  -D "$EVIDENCE/forced-headers.txt" \
  -o "$EVIDENCE/forced-body.json" \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"ternary-bonsai-27b-q2_0",
    "messages":[{"role":"user","content":"Reply with exactly: BACKBONE_OK"}],
    "max_tokens":16,
    "stream":false,
    "force_provider_id":"edge_backbone",
    "force_model_id":"ternary-bonsai-27b-q2_0",
    "allow_forced_provider_fallback":false,
    "tool_use_required":false,
    "source_surface":"backbone_recovery_validation"
  }' \
  "$MANWE_BASE/v1/chat/completions"
```

Expected: HTTP 200 within 90 seconds.

**Step 2: Assert selected route from response headers**

```bash
grep -i '^x-manwe-provider-id: edge_backbone' "$EVIDENCE/forced-headers.txt"
grep -i '^x-manwe-model-id: ternary-bonsai-27b-q2_0' "$EVIDENCE/forced-headers.txt"
python3 -m json.tool "$EVIDENCE/forced-body.json" >/dev/null
```

Expected: both exact headers are present and the response body is valid JSON. Any other provider is a failure because forced fallback was disabled.

**Step 3: Capture route-history evidence**

```bash
curl -fsS "$MANWE_BASE/route_history" > "$EVIDENCE/route-history-after-forced.json"
```

Expected: recent route evidence identifies `edge_backbone` and `ternary-bonsai-27b-q2_0` for the validation request.

---

### Task 6: Verify automatic routing and system health after recovery

**Objective:** Confirm recovery adds a healthy candidate without breaking ordinary automatic routing.

**Step 1: Run one bounded automatic local-origin request**

```bash
curl -4 --noproxy '*' --fail-with-body --connect-timeout 3 --max-time 90 \
  -D "$EVIDENCE/auto-headers.txt" \
  -o "$EVIDENCE/auto-body.json" \
  -H 'Content-Type: application/json' \
  -d '{
    "model":"auto",
    "messages":[{"role":"user","content":"Reply with exactly: AUTO_OK"}],
    "max_tokens":16,
    "stream":false,
    "origin_preference":"local",
    "tool_use_required":false,
    "source_surface":"backbone_recovery_validation"
  }' \
  "$MANWE_BASE/v1/chat/completions"
```

Expected: HTTP 200 with valid `x-manwe-provider-id` and `x-manwe-model-id` headers. Automatic routing is not required to choose `edge_backbone`; the qualified request already proves that lane. It must choose a healthy eligible route without destabilizing Manwe.

**Step 2: Run the installed Manwe inference probe**

```bash
systemctl --user --machine="$(id -un)@.host" start arda-manwe-inference-probe.service
systemctl --user --machine="$(id -un)@.host" show arda-manwe-inference-probe.service \
  -p Result -p ExecMainStatus -p ActiveState -p SubState \
  | tee "$EVIDENCE/manwe-probe-result.txt"
```

Expected: `Result=success`, `ExecMainStatus=0`, and the oneshot finishes `inactive/dead` normally.

**Step 3: Recheck all canonical services and health endpoints**

```bash
systemctl --user --machine="$(id -un)@.host" is-active \
  arda-manwe.service \
  arda-metrics-exporter.service \
  hermes-gateway.service \
  arda-aule-autopilot-read-only.timer \
  arda-manwe-inference-probe.timer | tee "$EVIDENCE/local-units-after.txt"
curl -fsS "$MANWE_BASE/healthz" > "$EVIDENCE/manwe-health-after.json"
curl -fsS http://127.0.0.1:9101/health/audit > "$EVIDENCE/audit-health-after.json"
```

Expected: all units remain active and both health surfaces remain healthy.

---

### Task 7: Check isolation and make the keep-on/rollback decision

**Objective:** Close the recovery test with explicit evidence and no accidental repository cleanup.

**Step 1: Verify queue immutability**

```bash
sha256sum core/projects/tasks/queue.jsonl > "$EVIDENCE/queue-after.sha256"
diff -u "$EVIDENCE/queue-before.sha256" "$EVIDENCE/queue-after.sha256"
```

Expected: no difference.

**Step 2: Compare status and fingerprints**

```bash
git status --porcelain=v1 > "$EVIDENCE/git-after.txt"
git diff --check > "$EVIDENCE/diff-check-after.txt"
git status --porcelain=v1 | cut -c4- | while IFS= read -r path; do
  test -f "$path" && sha256sum "$path"
done > "$EVIDENCE/dirty-sha256-after.txt"
diff -u "$EVIDENCE/git-before.txt" "$EVIDENCE/git-after.txt" \
  > "$EVIDENCE/git-status-delta.diff" || true
diff -u "$EVIDENCE/dirty-sha256-before.txt" "$EVIDENCE/dirty-sha256-after.txt" \
  > "$EVIDENCE/dirty-content-delta.diff" || true
```

Expected: `git diff --check` remains clean. Review both deltas; do not restore pre-existing dirty files. Classify expected Manwe runtime receipts/projections separately from unexplained source/config changes.

**Step 3: Capture final remote stability**

```bash
ssh "$REMOTE" '
  systemctl --user show llama-server-bonsai27.service \
    -p ActiveState -p SubState -p MainPID -p NRestarts -p UnitFileState;
  systemctl --user status llama-server-bonsai27.service --no-pager -l;
  journalctl --user -u llama-server-bonsai27.service -n 40 --no-pager;
  nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv,noheader || true
' | tee "$EVIDENCE/remote-final.txt"
```

Expected: stable `active/running`, no new restart loop, and one model process consistent with the canonical service.

**Step 4: Apply the decision gate**

Keep `llama-server-bonsai27.service` running only if all acceptance criteria pass:

1. One `:8095` listener; retired ports remain absent.
2. Direct `/v1/models` reports the exact canonical model.
3. Manwe provider catalog contains the canonical provider/model.
4. Strict forced request returns `edge_backbone`/`ternary-bonsai-27b-q2_0` with no fallback.
5. Automatic local-origin request succeeds.
6. Manwe probe and all canonical services/timers remain healthy.
7. Audit health remains `ok`.
8. Queue hash is unchanged.
9. Worktree deltas contain no unexplained source/config mutations.

If any criterion fails:

```bash
ssh "$REMOTE" 'systemctl --user stop llama-server-bonsai27.service'
```

Then verify Manwe returns to the already-proven degraded-fleet state; retain `/tmp/arda-backbone-recovery/` as failure evidence.

If every criterion passes, leave the service running in its pre-existing enablement state. Do not run `enable` unless a separate operator decision changes the startup policy.

---

## Final closeout report

Report these exact facts:

- Remote unit initial/final active and enabled states.
- Direct model ID and listener uniqueness.
- Forced request HTTP result, latency if available, provider/model headers, and route receipt evidence.
- Automatic request selected provider/model.
- Manwe probe result and all service/timer states.
- Audit-health result.
- Before/after queue hash.
- Worktree status/fingerprint delta classification.
- Final decision: `left running` or `rolled back to stopped`, with the failed criterion if rolled back.

No repository file should be committed as part of this operational check. If canonical config or documentation disagrees with validated live state, stop and create a separate targeted drift-repair task rather than editing during recovery validation.
