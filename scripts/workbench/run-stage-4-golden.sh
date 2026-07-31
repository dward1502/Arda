#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
evidence_dir="${1:-${repo_root}/docs/evidence/workbench-stage-4}"

mkdir -p "${evidence_dir}"
export ARDA_GOLDEN_EVIDENCE_DIR="${evidence_dir}"

cd "${repo_root}"
printf 'Stage 4 evidence directory: %s\n' "${evidence_dir}"

cargo test -p arda-engine --test workbench_rust_golden -- --test-threads=1
cargo test -p arda-engine --test workbench_python_golden -- --test-threads=1
cargo test -p arda-engine --test workbench_boundary_recovery -- --test-threads=1

python3 - "${evidence_dir}" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
required = [
    root / "rust-golden-result.json",
    root / "python-golden-result.json",
    root / "boundary-recovery-result.json",
]
for path in required:
    payload = json.loads(path.read_text(encoding="utf-8"))
    print(f"{path.name}: run_id={payload['run_id']}")
print("Stage 4 golden proofs passed.")
PY
