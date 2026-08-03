# Warden → Varda external-lane deployment

This runbook installs the bounded caller for the governed research lane. The caller imports at most one unconsumed Warden observation per timer invocation through Varda's `POST /external_lane/import` route.

## Components

- Varda HTTP service: `http://127.0.0.1:5111` by default.
- Crawl4AI markdown service: `http://127.0.0.1:11235` by default.
- Varda service unit: `config/systemd/arda-varda.service`.
- Crawl4AI service unit: `config/systemd/arda-crawl4ai.service`.
- Warden receipt ledger: `data/warden/research_receipts.jsonl`.
- Varda evaluation ledger: `data/varda/external_evaluations.jsonl`.
- Caller: `config/systemd/arda-varda-external-lane.service`.
- Schedule: `config/systemd/arda-varda-external-lane.timer`.

The systemd files are repository artifacts only. They are not enabled automatically and do not install Varda or Crawl4AI.

## Install operator configuration

```bash
install -d -m 700 "$HOME/.config/arda/varda"
install -m 600 config/systemd/arda-varda-external-lane.env.example \
  "$HOME/.config/arda/varda/external-lane.env"
```

Edit the environment file if either service uses a non-default address. It must define:

```text
ARDA_VARDA_URL=http://127.0.0.1:5111
ARDA_CRAWL4AI_URL=http://127.0.0.1:11235
ARDA_EXTERNAL_LANE_FILTER=fit
```

`ARDA_CRAWL4AI_URL` is the service base URL; Varda appends `/md` when making the crawl request.

## Verify prerequisites before enabling

Read the actual operator configuration, then check the exact configured endpoints:

```bash
set -a
. "$HOME/.config/arda/varda/external-lane.env"
set +a

curl --fail --silent --show-error "$ARDA_VARDA_URL/status"
curl --fail --silent --show-error "$ARDA_CRAWL4AI_URL/health"
systemd-analyze verify \
  config/systemd/arda-varda-external-lane.service \
  config/systemd/arda-varda-external-lane.timer
```

Do not enable the timer if either prerequisite endpoint is unavailable. A failed invocation does not advance the Warden cursor, but repeated timer failures should be treated as an operations issue.

## Install and run

```bash
install -d -m 700 "$HOME/.config/systemd/user"
install -m 644 config/systemd/arda-varda-external-lane.service \
  "$HOME/.config/systemd/user/"
install -m 644 config/systemd/arda-varda-external-lane.timer \
  "$HOME/.config/systemd/user/"
systemctl --user daemon-reload
systemctl --user start arda-varda-external-lane.service
systemctl --user status arda-varda-external-lane.service --no-pager
```

Only after the one-shot succeeds:

```bash
systemctl --user enable --now arda-varda-external-lane.timer
systemctl --user list-timers arda-varda-external-lane.timer --no-pager
```

## Verify durable behavior

```bash
journalctl --user -u arda-warden-scout.service -n 50 --no-pager
journalctl --user -u arda-warden-research.service -n 50 --no-pager
wc -l data/warden/research_receipts.jsonl data/warden/research_suggestions.jsonl
```

The authoritative checks are the JSONL ledgers, not the HTTP response alone. Each receipt chain contains `suggestion → dispatch → observation → acknowledgement` with SHA-256 content and provenance hashes. The scout fetches canonical content through Crawl4AI before recording the observation `content_hash`, so Varda's `/external_lane/import` handler validates both `content_hash` and `provenance_hash` against freshly-fetched canonical content.

Re-running the service for the same observation must not create a second evaluation receipt or advance the cursor twice. The `ResearchReceiptLedger.append_complete_chain` deduplicates by `idempotency_key`. Any canonical fetch, provenance, expiry, or approval failure must leave the observation available for later diagnosis/retry.

## Current verification state

As of 2026-08-03, the Warden Research recurring-watchlist beta is deployed to the Pi5 scout node (`node-pi5-warden`), reachable via Tailscale at `100.110.85.37:8092`. The scout binary was rebuilt from current HEAD to include the canonical content fetch through Crawl4AI (`POST /md`), fixing the content-hash mismatch that previously caused `ContentHashMismatch` in Varda's external-lane import.

Deployed units on the Pi5:
- `arda-warden-scout.service` — scout API, bound to Tailscale IP `:8092`, active.
- `arda-warden-research.service` + `.timer` — runs `run-topics` every 24h, writes receipt chains to `data/warden/research_receipts.jsonl`.

**Ledger state:**
```
data/warden/research_receipts.jsonl  — 4 complete chains (suggestion → dispatch → observation → acknowledgement)
data/warden/research_suggestions.jsonl — 4 suggestions
```

**Varda local verification (development host):**
- `arda-varda.service`, `arda-crawl4ai.service` enabled and active.
- Varda `/status` and Crawl4AI `/health` both return HTTP 200.
- `cargo test -p arda-outpost-scout --all-features` → 5 passed, 0 failed.
- `cargo test -p arda-varda --all-features -- --test-threads=1` → all tests pass, including `external_lane.rs::live_crawl_import_fetches_next_chain_and_advances_after_receipt`.

**Known coordination note:** The scout and research service unit files hardcode the Tailscale IP `100.110.85.37`. If the Pi5's Tailscale address changes, update `config/systemd/arda-warden-scout.service` and `config/systemd/arda-warden-research.service` before redeploying.
