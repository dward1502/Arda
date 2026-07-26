import json
import subprocess
from pathlib import Path


SCRIPT = Path("scripts/audit/setup_console_audit.py")


def write_portability_receipt(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "contract": "arda.portability_config_hygiene_audit.v1",
                "summary": {
                    "active_blocker_findings": 3,
                    "classification_counts": {
                        "active_config_must_parameterize": 1,
                        "active_script_must_parameterize": 1,
                        "active_source_must_fix": 1,
                    },
                    "pattern_counts": {
                        "loopback_endpoint": 1,
                        "private_lan_ip_endpoint": 1,
                        "hardcoded_home_mythos": 1,
                        "hardcoded_var_home_mythos": 0,
                    },
                    "top_active_blockers": [
                        {"path": "config/example.toml", "findings": 2},
                    ],
                },
            }
        ),
        encoding="utf-8",
    )


def test_setup_console_audit_emits_receipt_summary_and_state(tmp_path: Path) -> None:
    root = tmp_path
    for rel in [
        "AGENTS.md",
        "ARDA_ROOT_PROTOCOL.md",
        "docs/CODEMAP.md",
        "scripts/runtime_build_env.sh",
        "config/manwe.providers.toml",
        "core/state/environment_profile.schema.json",
    ]:
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fixture\n", encoding="utf-8")

    portability = root / "audit/PORTABILITY_AUDIT_2026-05-24/summary.json"
    write_portability_receipt(portability)

    out_dir = root / "audit/setup-console"
    state_path = root / "core/state/setup_console_readiness.json"
    result = subprocess.run(
        [
            "python3",
            str(SCRIPT.resolve()),
            "--root",
            str(root),
            "--out-dir",
            str(out_dir),
            "--state-path",
            str(state_path),
            "--portability-receipt",
            str(portability),
        ],
        check=True,
        text=True,
        capture_output=True,
    )

    stdout = json.loads(result.stdout)
    assert stdout["gate_status"] == "warn"
    receipt_path = out_dir / "setup_console_readiness_receipt.json"
    summary_path = out_dir / "SUMMARY.md"
    assert receipt_path.exists()
    assert summary_path.exists()
    assert state_path.exists()

    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert receipt["mode"] == "read_only"
    assert receipt["mutation_policy"] == "receipts_only_no_source_config_or_service_rewrites"
    check_ids = {check["check_id"] for check in receipt["checks"]}
    assert "portability.receipt" in check_ids
    assert "endpoint.assumptions" in check_ids

    summary = summary_path.read_text(encoding="utf-8")
    assert "Setup Console Readiness Audit" in summary
    assert "Scope guard" in summary


def test_setup_console_audit_warns_when_portability_receipt_missing(tmp_path: Path) -> None:
    root = tmp_path
    (root / "AGENTS.md").write_text("fixture\n", encoding="utf-8")
    out_dir = root / "audit/setup-console"
    state_path = root / "core/state/setup_console_readiness.json"

    subprocess.run(
        [
            "python3",
            str(SCRIPT.resolve()),
            "--root",
            str(root),
            "--out-dir",
            str(out_dir),
            "--state-path",
            str(state_path),
            "--portability-receipt",
            str(root / "missing.json"),
        ],
        check=True,
        text=True,
        capture_output=True,
    )

    receipt = json.loads((out_dir / "setup_console_readiness_receipt.json").read_text(encoding="utf-8"))
    portability_checks = [check for check in receipt["checks"] if check["check_id"] == "portability.receipt"]
    assert portability_checks
    assert portability_checks[0]["status"] == "warn"
    assert receipt["gate_status"] == "warn"


def test_setup_console_audit_projects_zero_active_portability_blockers(tmp_path: Path) -> None:
    root = tmp_path
    for rel in [
        "AGENTS.md",
        "ARDA_ROOT_PROTOCOL.md",
        "docs/CODEMAP.md",
        "scripts/runtime_build_env.sh",
        "config/manwe.providers.toml",
        "core/state/environment_profile.schema.json",
    ]:
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("fixture\n", encoding="utf-8")

    portability = root / "audit/PORTABILITY_AUDIT_2026-05-24/summary.json"
    portability.parent.mkdir(parents=True, exist_ok=True)
    portability.write_text(
        json.dumps(
            {
                "contract": "arda.portability_config_hygiene_audit.v1",
                "summary": {
                    "active_blocker_findings": 0,
                    "findings_total": 12,
                    "classification_counts": {"documentation_example_review": 12},
                    "top_active_blockers": [],
                },
            }
        ),
        encoding="utf-8",
    )

    out_dir = root / "audit/setup-console"
    state_path = root / "core/state/setup_console_readiness.json"
    subprocess.run(
        [
            "python3",
            str(SCRIPT.resolve()),
            "--root",
            str(root),
            "--out-dir",
            str(out_dir),
            "--state-path",
            str(state_path),
            "--portability-receipt",
            str(portability),
        ],
        check=True,
        text=True,
        capture_output=True,
    )

    receipt = json.loads(state_path.read_text(encoding="utf-8"))
    assert receipt["gate_status"] == "pass"
    assert receipt["portability_status"] == {
        "status": "pass",
        "active_blocker_findings": 0,
        "findings_total": 12,
        "label": "zero active portability blockers",
        "source": "audit/PORTABILITY_AUDIT_2026-05-24/summary.json",
    }
