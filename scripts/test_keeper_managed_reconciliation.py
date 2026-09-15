#!/usr/bin/env python3
"""Isolated real-systemd admission-loss proof; no provider or production services."""
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
KEEPER = ROOT / "target/debug/arda-snapshot-keeper"
WORKER = ROOT / "target/debug/arda-snapshot-worker"


def run(*args, check=True):
    result = subprocess.run([str(a) for a in args], capture_output=True, text=True, timeout=45)
    if check and result.returncode:
        raise RuntimeError(f"{args[0]} failed: {result.stderr}")
    return result


def main():
    unit = f"arda-reconcile-test-{uuid.uuid4().hex}.service"
    with tempfile.TemporaryDirectory(dir="/var/tmp") as d, tempfile.TemporaryDirectory(dir="/dev/shm") as r:
        durable, runtime = Path(d), Path(r)
        workspace = durable / "workspace"
        # Workspace cannot overlap protected durable storage.
        with tempfile.TemporaryDirectory(dir="/var/tmp") as w:
            workspace = Path(w)
            run(KEEPER, "--initialize", durable)
            owner = (durable / "owner.identity").read_text()
            base = [KEEPER, "reconcile"]
            common = ["--durable", durable, "--runtime", runtime, "--owner", owner, "--run", "uncertain"]
            try:
                run("systemd-run", "--user", f"--unit={unit}", "--service-type=notify",
                    "-p", "NotifyAccess=main", "-p", "ProtectControlGroups=yes",
                    "-p", "Delegate=no", "-p", "KillMode=control-group",
                    "-p", "RuntimeDirectoryPreserve=yes", "-p", "TimeoutStartSec=15",
                    f"--setenv=ARDA_KEEPER_SYSTEMD_UNIT={unit}", KEEPER, durable, runtime, WORKER)
                with socket.socket(socket.AF_UNIX) as stream:
                    stream.settimeout(15)
                    stream.connect(str(runtime / "keeper.sock"))
                    # Deliberately invalid identity: persist managed preparing
                    # admission, then fail closed before a usable authority exists.
                    stream.sendall((json.dumps({"op": "prepare", "run": "uncertain", "workspace": str(workspace), "identity": "invalid"}) + "\n").encode())
                    response = json.loads(stream.makefile().readline())
                    assert response["ok"] is False
                db = sqlite3.connect(durable / "owner.sqlite3")
                assert db.execute("SELECT state,authority FROM snapshots WHERE run='uncertain'").fetchone() == ("preparing", None)
                assert db.execute("SELECT count(*) FROM snapshot_managed_ownership").fetchone() == (1,)
                db.close()
                assert run(*base, "inspect", *common, check=False).returncode != 0
                run("systemctl", "--user", "stop", unit)
                run("systemctl", "--user", "mask", "--runtime", unit)
                inspected = json.loads(run(*base, "inspect", *common, "--json").stdout)
                assert inspected["blockers"] is None, inspected
                proof = durable / "stop-proof.json"
                run(*base, "stop-proof", *common, "--output", proof, "--confirm-managed-stop")
                request = str(uuid.uuid4())
                revoke = [*base, "revoke", *common, "--expect-record-digest", inspected["record_digest"],
                          "--managed-stop-evidence", proof, "--request-id", request,
                          "--operator", "isolated-test", "--reason", "failed pre-authority admission",
                          "--confirm-terminal-revocation"]
                first = json.loads(run(*revoke).stdout)
                assert first["worker_cleanup_ack"] is False
                assert first["artifacts_retained"] is True
                assert json.loads(run(*revoke).stdout) == first
                changed = revoke.copy()
                changed[changed.index("isolated-test")] = "conflicting-operator"
                assert run(*changed, check=False).returncode != 0
                db = sqlite3.connect(durable / "owner.sqlite3")
                assert db.execute("SELECT state,authority FROM snapshots WHERE run='uncertain'").fetchone() == ("reconciled_revoked", None)
                assert db.execute("SELECT count(*) FROM snapshot_reconciliations").fetchone() == (1,)
                db.close()
                print("PASS: real managed preparing admission, stop proof, atomic tombstone, exact retry, conflict rejection")
            finally:
                run("systemctl", "--user", "stop", unit, check=False)
                run("systemctl", "--user", "unmask", "--runtime", unit, check=False)
                run("systemctl", "--user", "reset-failed", unit, check=False)
                state = run("systemctl", "--user", "show", unit, "-p", "MainPID", "--value", check=False).stdout.strip()
                assert state in ("", "0"), "isolated keeper remains running"


if __name__ == "__main__":
    main()
