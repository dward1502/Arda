# Arda Project Adapter SDK

Stage 5 publishes one versioned boundary: `arda.project-adapter.v1`, JSON Lines over stdin/stdout. The protocol and Draft 2020-12 schema are authoritative:

- `spec/project-adapter/v1/protocol.md`
- `spec/project-adapter/v1/messages.schema.json`
- `spec/project-contract/v1/project-contract.schema.json`

The Python package under `sdk/python/arda_project_adapter/` is the complete executable reference server. The Rust and JavaScript SDK packages provide bounded JSONL framing, envelope/version validation, and deny-by-default capability negotiation; the engine owns the host-side JSONL client.

- Rust: `sdk/rust` (`cargo test -p arda-project-adapter-sdk`)
- JavaScript: `sdk/javascript` (`npm test --prefix sdk/javascript`)
- Python: `sdk/python/arda_project_adapter`

## Generate a project contract

```bash
python3 scripts/arda_adapter_ops.py template \
  --kind python \
  --name my-project \
  --output /path/to/project/arda-project.json
```

`--kind` accepts `rust`, `python`, or `javascript`. The generated contract defaults to:

- deny network access;
- no secret environment names;
- approval-required authority;
- project-scoped memory;
- Git-revert rollback.

Review and explicitly narrow or expand those declarations before onboarding. The generator refuses unsafe names, invalid UUIDs, and existing output files unless `--force` is supplied.

For reproducible fixtures, provide both identity fields:

```bash
python3 scripts/arda_adapter_ops.py template \
  --kind javascript \
  --name fixture-js \
  --project-id 550e8400-e29b-41d4-a716-446655440099 \
  --declared-at 2026-07-31T00:00:00Z \
  --output /tmp/arda-project.json
```

## Run conformance

The schema gate requires `jsonschema`; use the repository's established ephemeral `uv` environment:

```bash
uv run --with jsonschema python scripts/arda_adapter_ops.py conformance \
  --output docs/evidence/stage-5-release-candidate/adapters/conformance.json
```

The entrypoint validates all Rust/Python/JavaScript contract examples, executes all three SDK suites, executes the Rust engine process-boundary suite, and runs isolated Python and Rust golden repositories. Subprocess output is represented by SHA-256 rather than copied into the receipt.

A passing receipt proves the checked templates, three SDK packages, and executable reference path. It does not claim that a separately sourced external repository was available or that arbitrary third-party adapters are trusted.

## Adapter safety requirements

Adapters must:

1. emit one schema-valid JSON object per line and no stdout logging;
2. correlate every response to its request ID;
3. declare capabilities and fail closed when authority is absent;
4. honor timeout/cancellation and reap child processes;
5. use project-root-contained working directories;
6. receive only explicitly allow-listed environment values;
7. reject oversized frames and unknown protocol fields;
8. return provenance and recovery tokens where specified.

Use stderr for bounded human diagnostics. Never emit credentials, source content, provider responses, or raw personal data into protocol or conformance logs.
