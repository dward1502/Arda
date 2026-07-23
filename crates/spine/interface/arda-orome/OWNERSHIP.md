# arda-orome ownership

Crate: `crates/spine/interface/arda-orome`
Owner: HADES / interface layer
Status: active
Boundary: provider runtime abstraction, dispatch receipt, context cache, and ambient routing surfaces.

This crate owns:
- typed provider runtime contracts and dispatch receipt semantics
- provider-level timeout/retry/fallback behavior
- shared bounded async context cache
- ambient interface adapters that do not perform persistence or process lifecycle

This crate does not own:
- transport-exclusive binding or daemon lifecycle
- policy enforcement beyond returning typed dispatch outcomes
- direct external service persistence

Preferred consumer path:
- `arda-manwe` / `arda-varda` through explicit `ProviderRuntime` / `DispatchReceipt` interfaces
- `arda-orome` root re-exports as canonical public path
