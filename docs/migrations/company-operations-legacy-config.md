# Company Operations legacy configuration migration

Status: migrated; `config/business/ceo_startup.yaml` is evidence only and is not runtime authority.

| Legacy setting | Disposition | Current authority |
|---|---|---|
| startup install/diagnostic/preflight | Retired from Company Operations | Arda launcher, service registry, and normal runtime health gates |
| `mission_control` URL/process/start profiles | Retired | No current Company Operations dependency; paths, port 9100, usernames, and Python startup commands are stale |
| `ollama serve` startup | Retired | Manwë provider runtime and supervised service configuration |
| usage reports under `Tools/` / `Operations/` | Retired | `arda-economics` and current observability projections |
| Warden startup governance scan | Mapped | Warden/Varda evidence may inform proposals; it cannot create commitments or executable work |
| legacy governance ledger path | Retired | Versioned Arda receipts and canonical data projections |
| HUD build/launch scripts and KhazadForge binary | Retired | `apps/arda-hud`, `pnpm run tauri dev`, and `pnpm run tauri build` |

The live Company Operations policy and storage map is `config/business/company-operations.toml`. Dead mission-control paths are intentionally not preserved. External adapter credentials remain outside this repository in adapter-local stores or an OS keyring.
