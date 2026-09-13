"""Real private-namespace snapshot protocol acceptance; only temporary paths."""
import ctypes
import json
import os
from pathlib import Path
import socket
import subprocess
import select
import signal
import struct
import sys
import tempfile
import time
from snapshot_bootstrap import verify_bootstrap_death

libc = ctypes.CDLL(None, use_errno=True)


def bind(source, destination):
    result = libc.mount(os.fsencode(source), os.fsencode(destination), None, 4096, None)
    if result:
        raise OSError(ctypes.get_errno(), "bind fixture")


def request(state, value):
    with socket.socket(socket.AF_UNIX) as client:
        client.settimeout(8)
        client.connect(str(state / "control.sock"))
        client.sendall(json.dumps(value).encode() + b"\n")
        return json.loads(client.makefile("rb").readline())


def lease(run="run", generation=1, duration_ms=120000):
    return {"run_id": run, "generation": generation, "owner": f"worker-{generation}",
            "expires_ms": time.time_ns() // 1000000 + duration_ms}


def start(binary, state, root):
    child = subprocess.Popen([binary, str(state), str(root)])
    try:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if child.poll() is not None:
                raise AssertionError(f"keeper exited during startup: {child.returncode}")
            try:
                manifest = request(state, {"op": "inspect"})
                assert manifest["ok"], manifest
                return child, manifest
            except (FileNotFoundError, ConnectionRefusedError):
                time.sleep(0.01)
        raise AssertionError("keeper readiness deadline")
    except BaseException:
        child.kill()
        child.wait()
        raise


with tempfile.TemporaryDirectory(prefix="arda-snapshot-proof-") as directory, tempfile.TemporaryDirectory(prefix="arda-snapshot-state-", dir="/dev/shm") as runtime_directory:
    base = Path(directory)
    runtime = Path(runtime_directory)
    root, outside, state = base / "root", base / "outside", runtime / "keeper"
    root.mkdir()
    (root / "nested").mkdir()
    outside.mkdir()
    alias = base / "state-parent-alias"
    alias.symlink_to(root, target_is_directory=True)
    refused = subprocess.run([sys.argv[1], str(alias / "must-not-exist"), str(root)], capture_output=True, timeout=5)
    assert refused.returncode != 0 and not (root / "must-not-exist").exists(), "symlinked state parent modified workspace"
    before = Path("/proc/self/mountinfo").read_bytes()
    refused = subprocess.run([sys.argv[1], "--private", str(state), str(root)], capture_output=True, timeout=5)
    assert refused.returncode != 0 and Path("/proc/self/mountinfo").read_bytes() == before
    # A mount exactly at /usr must not expose state through a root-level alias.
    # Preserve the runtime libraries via symlinks for worker startup only; all
    # mount changes occur in a disposable child mount namespace.
    usr_probe = r'''
import ctypes, os, pathlib, subprocess, sys
c = ctypes.CDLL(None, use_errno=True)
binary, runtime, base, root = sys.argv[1:]
runtime, base = pathlib.Path(runtime), pathlib.Path(base)
saved, grant = base / "saved-usr", runtime / "usr-grant"
saved.mkdir(); grant.mkdir()
assert c.unshare(0x20000) == 0
assert c.mount(None, b"/", None, 16384 | (1 << 18), None) == 0
assert c.mount(b"/usr", os.fsencode(saved), None, 4096, None) == 0
for entry in ("bin", "sbin", "lib", "lib64"):
    (grant / entry).symlink_to(saved / entry)
assert c.mount(os.fsencode(grant), b"/usr", None, 4096, None) == 0
state = runtime / "usr-exposed-state"
result = subprocess.run([binary, str(state), root], capture_output=True, timeout=5)
assert result.returncode != 0, result
assert b"state filesystem overlaps a provider grant" in result.stderr, result.stderr
assert not state.exists(), "rejected /usr alias wrote state"
'''
    subprocess.run([sys.executable, "-c", usr_probe, sys.argv[1], str(runtime), str(base), str(root)], check=True, timeout=10)
    child, prepared = start(sys.argv[1], state, root)
    mounted = False
    try:
        capability = prepared["manifest"]["capability"]
        first_lease = lease()
        execute = {"op": "execute", "capability": capability, "lease": first_lease, "argv": ["/bin/sh", "-c", "printf pinned > nested/result"], "environment": {}, "timeout_ms": 5000}
        assert not request(state, execute)["ok"], "unadmitted execution accepted"
        assert not (root / "nested/result").exists()
        commit = {"op": "commit", "capability": capability, "manifest_digest": prepared["manifest_digest"], "lease": first_lease}
        assert not request(state, dict(commit, manifest_digest="wrong"))["ok"]
        assert request(state, commit)["ok"]
        assert request(state, commit)["ok"], "idempotent admission rejected"
        assert not request(state, dict(commit, lease="different"))["ok"]
        # Positive control: loader constructors execute before main in an
        # unconstrained launcher. The worker must not expose that launcher path.
        injected = outside / "loader-write"
        library = root / "loader.so"
        source = '#include <stdio.h>\n__attribute__((constructor)) static void init(void) { FILE *f=fopen(' + json.dumps(str(injected)) + ',"w"); if(f){fputs("escaped",f);fclose(f);} }\n'
        subprocess.run(["/usr/bin/cc", "-shared", "-fPIC", "-x", "c", "-", "-o", str(library)], input=source.encode(), check=True)
        subprocess.run(["/bin/true"], env={"LD_PRELOAD": str(library)}, check=True)
        assert injected.read_text() == "escaped"
        injected.unlink()
        outcome = request(state, dict(execute, argv=["/bin/true"], environment={"LD_PRELOAD": str(library)}))
        assert outcome["ok"] and outcome["code"] == 0, outcome
        assert not injected.exists(), "request environment escaped through bubblewrap loader"
        private_data = outside / "synthetic-private-data"
        private_data.write_text("fixture only, not a real credential")
        outcome = request(state, dict(execute, argv=["/bin/sh", "-c", f"test ! -r {private_data}"]))
        assert outcome["ok"] and outcome["code"] == 0, "host files visible outside explicit grants"
        outcome = request(state, dict(execute, argv=["/bin/sh", "-c", "printf response; printf diagnostic >&2"]))
        assert outcome["stdout"] == "response" and outcome["stderr"] == "diagnostic", outcome
        assert not outcome["output_limit"] and not outcome["cancelled"], outcome
        outcome = request(state, dict(execute, argv=["/usr/bin/python3", "-c", "import os; os.write(1, b'x'*100000); os.write(2, b'y'*100000)"]))
        assert outcome["ok"] and outcome["output_limit"], outcome
        assert len(outcome["stdout"]) <= 65536 and len(outcome["stderr"]) <= 65536
        # Production adapter budgets must be accepted explicitly, while the
        # legacy default and caller-selected smaller limits stay bounded.
        outcome = request(state, dict(execute, max_output_bytes=1048576,
            timeout_ms=300000, argv=["/usr/bin/python3", "-c",
            "import os; os.write(1, b'x'*100000); os.write(2, b'y'*100000)"]))
        assert outcome["ok"] and outcome["code"] == 0 and not outcome["output_limit"]
        assert outcome["stdout"] == "x" * 100000 and outcome["stderr"] == "y" * 100000
        outcome = request(state, dict(execute, max_output_bytes=17,
            argv=["/usr/bin/python3", "-c", "print('x'*1000)"]))
        assert outcome["ok"] and outcome["output_limit"] and len(outcome["stdout"]) == 17
        for unsupported in (0, 1048577):
            assert not request(state, dict(execute, max_output_bytes=unsupported))["ok"]
        assert not request(state, dict(execute, timeout_ms=300001))["ok"]
        # A normal bounded prompt can exceed the former 64-KiB frame limit.
        outcome = request(state, dict(execute,
            argv=["/bin/true", "p" * 90000]))
        assert outcome["ok"] and outcome["code"] == 0
        # Observe a live provider before disconnect; cancellation must finish
        # cleanup before the worker accepts the next command on another socket.
        with socket.socket(socket.AF_UNIX) as witness:
            witness.bind(str(root / "cancel.sock"))
            witness.listen(1)
            witness.settimeout(5)
            pidfd = None
            try:
                with socket.socket(socket.AF_UNIX) as running:
                    running.connect(str(state / "control.sock"))
                    code = "import socket,time; s=socket.socket(socket.AF_UNIX); s.connect('cancel.sock'); time.sleep(20)"
                    running.sendall(json.dumps(dict(execute, argv=["/usr/bin/python3", "-c", code], timeout_ms=30000)).encode() + b"\n")
                    peer, _ = witness.accept()
                    with peer:
                        host_pid, _, _ = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                        pidfd = os.pidfd_open(host_pid)
                    death = select.poll()
                    death.register(pidfd, select.POLLIN)
                    assert not death.poll(0), "cancellation provider never became live"
                assert death.poll(3000), "provider survived execution-client disconnect"
                outcome = request(state, dict(execute, argv=["/bin/true"]))
                assert outcome["ok"] and outcome["code"] == 0, outcome
            finally:
                if pidfd is not None:
                    try:
                        signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    os.close(pidfd)
            (root / "cancel.sock").unlink()
        # Independent short-lived client process exits; keeper admission survives.
        subprocess.run([sys.executable, "-c", "import socket,sys; s=socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); s.close()", str(state / "control.sock")], check=True)
        assert request(state, {"op": "inspect"})["committed"] == first_lease
        # A trickle must not extend the frame deadline indefinitely.
        with socket.socket(socket.AF_UNIX) as slow:
            slow.settimeout(4)
            slow.connect(str(state / "control.sock"))
            began = time.monotonic()
            for _ in range(12):
                try:
                    slow.sendall(b" ")
                except BrokenPipeError:
                    break
                time.sleep(0.2)
            assert not json.loads(slow.makefile("rb").readline())["ok"]
            assert time.monotonic() - began < 3.5
        assert request(state, {"op": "inspect"})["ok"]
        bind(outside, root / "nested")
        mounted = True
        outcome = request(state, execute)
        assert outcome["ok"] and outcome["code"] == 0, outcome
        assert not (outside / "result").exists(), "host mount redirected provider write"
        assert not (root / "nested/result").exists(), "live replacement received write"
        # Namespace-local mount removal exposes the original directory's write.
        assert libc.umount2(os.fsencode(root / "nested"), 0) == 0
        mounted = False
        assert (root / "nested/result").read_text() == "pinned"
        original = base / "original"
        root.rename(original)
        root.mkdir()
        outcome = request(state, dict(execute, argv=["/bin/sh", "-c", "printf retained > rename-result"]))
        assert outcome["ok"] and outcome["code"] == 0, outcome
        assert (original / "rename-result").read_text() == "retained"
        assert not (root / "rename-result").exists()
        outcome = request(state, dict(execute, argv=["/bin/sh", "-c", f"test ! -S {state}/control.sock"]))
        assert outcome["ok"] and outcome["code"] == 0, outcome
        outcome = request(state, dict(execute, argv=["/bin/sleep", "10"], timeout_ms=100))
        assert outcome["ok"] and outcome["timed_out"], outcome
        assert not request(state, dict(execute, environment={"HERMES_HOME": str(outside)}))["ok"]
        # Rebind the SAME retained tree after root replacement; fence every old
        # lease and reject changed same-generation payloads or a different run.
        second_lease = lease(generation=2)
        rebound = dict(commit, lease=second_lease)
        assert request(state, rebound)["ok"]
        assert request(state, rebound)["ok"]
        assert not request(state, commit)["ok"]
        assert not request(state, dict(rebound, lease=dict(second_lease, owner="other")))["ok"]
        assert not request(state, dict(rebound, lease=lease(run="other", generation=3)))["ok"]
        stale = dict(execute, argv=["/bin/sh", "-c", "printf stale > stale-write"])
        assert not request(state, stale)["ok"]
        assert not (original / "stale-write").exists()
        outcome = request(state, dict(execute, lease=second_lease, argv=["/bin/sh", "-c", "printf same-tree > recovered"]))
        assert outcome["ok"] and outcome["code"] == 0, outcome
        assert (original / "recovered").read_text() == "same-tree"
        assert not (root / "recovered").exists()
        assert request(state, {"op": "inspect"})["manifest_digest"] == prepared["manifest_digest"]
        expiring = lease(generation=3, duration_ms=600)
        assert request(state, dict(commit, lease=expiring))["ok"]
        began = time.monotonic()
        outcome = request(state, dict(execute, lease=expiring, argv=["/bin/sleep", "10"], timeout_ms=300000))
        assert outcome["ok"] and outcome["timed_out"], outcome
        assert time.monotonic() - began < 3
        assert not request(state, dict(stale, lease=expiring))["ok"]
        assert not (original / "stale-write").exists()
        assert request(state, {"op": "release", "capability": capability})["ok"]
        assert child.wait(timeout=5) == 0
        assert not (state / "control.sock").exists()
        assert not state.exists(), "normal release left owned state artifacts"
        # Release must not acknowledge success before removing owned artifacts.
        failed_release, manifest = start(sys.argv[1], runtime / "failed-release", root)
        try:
            (runtime / "failed-release/unexpected").write_text("do not recursively delete")
            outcome = request(runtime / "failed-release", {"op": "release", "capability": manifest["manifest"]["capability"]})
            assert not outcome["ok"] and "remove owned snapshot directory" in outcome["error"], outcome
            assert failed_release.wait(timeout=5) != 0
            assert (runtime / "failed-release").is_dir(), "failure fixture did not block removal"
        finally:
            if failed_release.poll() is None:
                failed_release.kill()
                failed_release.wait(timeout=5)
        assert (runtime / "failed-release/unexpected").read_text() == "do not recursively delete"
        (runtime / "failed-release/unexpected").unlink()
        (runtime / "failed-release").rmdir()
        # A replacement worker must never accept a previous worker's capability.
        replacement, second = start(sys.argv[1], runtime / "replacement", root)
        try:
            assert second["manifest"]["capability"] != capability
            assert not request(runtime / "replacement", execute)["ok"]
            replacement.kill()
            replacement.wait(timeout=5)
            try:
                request(runtime / "replacement", execute)
                raise AssertionError("dead keeper accepted execution")
            except (ConnectionRefusedError, FileNotFoundError):
                pass
        finally:
            if replacement.poll() is None:
                replacement.kill()
                replacement.wait(timeout=5)
        # Acquire a host pidfd before allowing writes; kill the keeper while its
        # provider is active, and retain the pre-cleanup death verdict.
        with socket.socket(socket.AF_UNIX) as witness:
            witness.bind(str(root / "witness.sock"))
            witness.listen(1)
            witness.settimeout(5)
            active, manifest = start(sys.argv[1], runtime / "active", root)
            pidfd = None
            try:
                binding = {"op": "commit", "capability": manifest["manifest"]["capability"], "manifest_digest": manifest["manifest_digest"], "lease": lease(run="active")}
                assert request(runtime / "active", binding)["ok"]
                code = "import socket,time; s=socket.socket(socket.AF_UNIX); s.connect('witness.sock'); s.recv(1); f=open('active-write','w'); f.write('started'); f.flush(); time.sleep(20)"
                with socket.socket(socket.AF_UNIX) as running:
                    running.settimeout(5)
                    running.connect(str(runtime / "active/control.sock"))
                    running.sendall(json.dumps(dict(execute, capability=binding["capability"], lease=binding["lease"], argv=["/usr/bin/python3", "-c", code], timeout_ms=30000)).encode() + b"\n")
                    peer, _ = witness.accept()
                    with peer:
                        host_pid, _, _ = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                        pidfd = os.pidfd_open(host_pid)
                        peer.sendall(b"1")
                    deadline = time.monotonic() + 5
                    while not (root / "active-write").exists() and time.monotonic() < deadline:
                        time.sleep(0.01)
                    assert (root / "active-write").read_text() == "started"
                    death = select.poll()
                    death.register(pidfd, select.POLLIN)
                    assert not death.poll(0), "provider already dead before keeper-loss injection"
                    active.kill()
                    active.wait(timeout=5)
                    dead_before_cleanup = bool(death.poll(3000))
                    if not dead_before_cleanup:
                        signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                    assert dead_before_cleanup, "active provider survived keeper death"
            finally:
                if active.poll() is None:
                    active.kill()
                    active.wait(timeout=5)
                if pidfd is not None:
                    os.close(pidfd)
        bootstrap, manifest = start(sys.argv[1], runtime / "bootstrap", root)
        binding = {"op": "commit", "capability": manifest["manifest"]["capability"], "manifest_digest": manifest["manifest_digest"], "lease": lease(run="bootstrap")}
        assert request(runtime / "bootstrap", binding)["ok"]
        verify_bootstrap_death(bootstrap, runtime / "bootstrap", dict(execute, capability=binding["capability"], lease=binding["lease"], argv=["/bin/sleep", "20"], timeout_ms=30000))
        print("snapshot protocol: retained mounts, admission, loader isolation, filesystem allowlist, state-parent rejection, frame deadline, bounded timeout, release cleanup, active and bootstrap keeper loss PASS")
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(timeout=5)
        if mounted:
            assert libc.umount2(os.fsencode(root / "nested"), 0) == 0
