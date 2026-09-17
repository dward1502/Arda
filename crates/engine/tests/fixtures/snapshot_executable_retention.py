"""Retained worker re-exec must use its original image after installation."""
import json
import hashlib
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time


def request(state, payload):
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(10)
        client.connect(str(state / "control.sock"))
        client.sendall(json.dumps(payload).encode() + b"\n")
        return json.loads(client.makefile("rb").readline())


os.umask(0o077)
for replacement in (False, True):
    with tempfile.TemporaryDirectory(dir="/var/tmp") as directory, tempfile.TemporaryDirectory(dir="/dev/shm") as control:
        base = Path(directory)
        executable = base / "worker"
        shutil.copy2(sys.argv[1], executable)
        root = base / "workspace"
        root.mkdir()
        state = Path(control) / "snapshot"
        worker = subprocess.Popen([str(executable), str(state), str(root)])
        try:
            deadline = time.monotonic() + 10
            while True:
                assert worker.poll() is None, "worker exited during startup"
                try:
                    inspected = request(state, {"op": "inspect"})
                    break
                except (FileNotFoundError, ConnectionRefusedError):
                    assert time.monotonic() < deadline, "startup timeout"
                    time.sleep(0.01)
            assert inspected["ok"]
            capability = inspected["manifest"]["capability"]
            lease = {"run_id": "retained-image", "generation": 1, "owner": "fixture", "expires_ms": time.time_ns() // 1000000 + 60000}
            assert request(state, {"op": "commit", "capability": capability, "manifest_digest": inspected["manifest_digest"], "lease": lease})["ok"]
            payload = {"op": "execute", "capability": capability, "lease": lease, "argv": ["/bin/sh", "-c", "printf original-image"], "environment": {}, "timeout_ms": 5000}
            first = request(state, payload)
            assert first["ok"] and first["code"] == 0 and first["stdout"] == "original-image", first
            if replacement:
                substitute = base / "replacement"
                shutil.copy2("/usr/bin/false", substitute)
                os.replace(substitute, executable)
            else:
                executable.unlink()
            # A missing installation path and an executable replacement must both
            # leave the already-admitted image usable; never execute /usr/bin/false.
            outcome = request(state, payload)
            if "--legacy-repair" in sys.argv:
                assert not outcome["ok"] and "spawn snapshot provider" in outcome["error"], outcome
                with socket.socket(socket.AF_UNIX) as peer:
                    peer.connect(str(state / "control.sock"))
                    pid, _, _ = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                repair = Path(os.readlink(f"/proc/{pid}/exe"))
                assert repair == Path(str(executable) + " (deleted)")
                pidfd = os.pidfd_open(pid)
                try:
                    with open(f"/proc/{pid}/exe", "rb") as source, repair.open("xb") as destination:
                        original = source.read()
                        destination.write(original)
                        destination.flush()
                        os.fsync(destination.fileno())
                    assert hashlib.sha256(repair.read_bytes()).digest() == hashlib.sha256(original).digest()
                    repair.chmod(0o500)
                    outcome = request(state, payload)
                finally:
                    os.close(pidfd)
            assert outcome["ok"] and outcome["code"] == 0 and outcome["stdout"] == "original-image", outcome
            after = request(state, {"op": "inspect"})
            assert after["manifest_digest"] == inspected["manifest_digest"]
            assert after["committed"] == lease
            assert request(state, {"op": "release", "capability": capability})["ok"]
            assert worker.wait(timeout=5) == 0
        finally:
            if worker.poll() is None:
                worker.kill()
            worker.wait()
print("original retained image survives unlink and atomic executable replacement")
