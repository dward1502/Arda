# Arda patch: RUSTSEC-2024-0429

This directory vendors the crates.io `glib 0.18.5` package solely to backport the
upstream fix from gtk-rs-core pull request 1343.

Upstream package:

- crate: `glib 0.18.5`
- crates.io checksum: `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5`
- license: MIT
- advisory: `RUSTSEC-2024-0429` / `GHSA-wrw7-89jp-8q8g`
- upstream fix commit: `e2f5aefcc60492b7f51a2ddcf1b649ef73f54bf4`

Arda-owned source delta:

- `src/variant_iter.rs`: make the C out-argument pointer mutable and pass
  `&mut p`, exactly matching the fix released in `glib 0.20.0`.

No API, ABI, feature, version, or dependency change is intended. This is an
isolated Stage 5 review candidate, not an upstream-maintenance claim. The
advisory exception may be accepted for release only while the deterministic
source check, optimized regression test, launcher/HUD native gates, and local
support boundary remain valid. Replace this fork with a maintained upstream
Tauri/GTK chain as soon as one is available.
