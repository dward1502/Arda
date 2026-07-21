# DIRECTORY

Top-level layout for `/var/home/mythos/Annunimas`.

## Key rules

- `crates/annunimas-*` — symlinks to `/var/home/mythos/Eregion/Arda/crates/annunimas-*`
- Canonical code lives in Arda; Annunimas is the operational/historical surface
- Do not edit code under `crates/` from Annunimas; edit in Arda instead
- Real operational state lives in `data/`, `core/`, `config/`, `human/`

## Traversal

1. Start here: `DIRECTORY_INDEX.md`
2. Jump with: `TRACT_INDEX.json`
3. Folder maps: `apps/README.md`, `audit/README.md`, `config/README.md`, `core/README.md`, `data/README.md`, `docs/README.md`, `human/README.md`, `scripts/README.md`, `tests/README.md`

## Migration note

Batch 2 moved duplicated `annunimas-*` crates out of this repo and replaced
each with symlinks. See `CRATE_DEDUPE_MANIFEST.json` for the moved list.
Canonical sources are now under `/var/home/mythos/Eregion/Arda/crates/`.
