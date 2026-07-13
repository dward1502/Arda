# arda-core

Shared **core library** for the Arda / ARDA ecosystem. It is the crate where
domain types, configuration, and receipt/data structures common to the Arda
launcher and the surrounding ARDA services are meant to converge.

> Status: scaffold. Right now it contains only the default `add` example and its
> test. The intended surface (shared domain types, receipts, config) has not yet
> been implemented.

## Intended Role

- Host reusable Rust types that the launcher (`apps/arda-launcher/src-tauri`)
  and the sibling ARDA repos can depend on without duplicating logic.
- Provide stable, inspectable data shapes — consistent with ARDA's
  "receipt-first" design principles (every action leaves a reviewable record).
- Keep the native side of the launcher thin by pushing shared logic here.

## Building & Testing

This crate is part of the workspace cargo build.

```bash
# from repo root
cargo build            # builds arda + arda-core
cargo test -p arda-core
```

## Layout

```
crates/arda-core/
├── Cargo.toml         # crate manifest (edition 2024, no deps yet)
├── README.md          # this file
└── src/
    └── lib.rs         # library root (currently the `add` example)
```

## Relationship to ARDA

`arda-core` is the shared substrate for the ARDA ecosystem (see `crates/README.md`
at the repo root for the full map and reading order). As the launcher and the
other ARDA services mature, the types they agree on should land here rather than
in each repo independently.

## Status

- Compiles and tests pass (`cargo test -p arda-core`).
- No domain logic yet — placeholder scaffold only.
- Pending: define the shared types (domain, config, receipts) once integration
  with the launcher and sibling repos begins.
