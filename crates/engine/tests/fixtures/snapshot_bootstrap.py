"""Stop real bubblewrap at fork before its child can arm death handling."""
import ctypes
import json
import os

import select
import signal
import socket
import time


def verify_bootstrap_death(worker, state, execute):
    libc = ctypes.CDLL(None, use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    handles = {worker.pid: os.pidfd_open(worker.pid)}
    live = {worker.pid}
    injected = False
    death = select.poll()

    def ptrace(op, pid, data=0):
        result = libc.ptrace(op, pid, None, ctypes.c_void_p(data))
        if result == -1:
            raise OSError(ctypes.get_errno(), f"ptrace {op} pid {pid}")

    def event():
        try:
            pid, status = os.waitpid(-1, os.WNOHANG | 0x40000000)  # __WALL
        except ChildProcessError:
            return None
        if not pid:
            return None
        if os.WIFEXITED(status) or os.WIFSIGNALED(status):
            live.discard(pid)
            if pid == worker.pid:
                worker.returncode = os.waitstatus_to_exitcode(status)
            return pid, None
        return pid, status

    try:
        # Auto-attach children and stop both sides at fork/clone. No injected
        # binary, sleeps, or production hook substitutes for the real bootstrap.
        ptrace(0x4206, worker.pid, 2 | 4 | 8)  # SEIZE, TRACEFORK/VFORK/CLONE
        with socket.socket(socket.AF_UNIX) as client:
            client.connect(str(state / "control.sock"))
            client.sendall(json.dumps(execute).encode() + b"\n")
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline and not injected:
                item = event()
                if item is None:
                    time.sleep(0.001)
                    continue
                pid, status = item
                if status is None:
                    raise AssertionError("launcher exited before bootstrap injection")
                kind = status >> 16
                if kind in (1, 2, 3):
                    child = ctypes.c_ulong()
                    ptrace(0x4201, pid, ctypes.addressof(child))  # GETEVENTMSG
                    handles[child.value] = os.pidfd_open(child.value)
                    live.add(child.value)

                    executable = os.readlink(f"/proc/{pid}/exe")
                    if executable == "/usr/bin/bwrap":
                        bootstrap_fd = handles[child.value]

                        death.register(bootstrap_fd, select.POLLIN)
                        assert not death.poll(0), "bootstrap child was already dead"
                        signal.pidfd_send_signal(handles[worker.pid], signal.SIGKILL)
                        injected = True
                        break
                ptrace(7, pid)  # CONT
            assert injected, "no real bubblewrap bootstrap fork observed"
            # Do not rescue the stopped launcher/child. Kernel parent-death and
            # PID namespace teardown must kill them before fixture cleanup.
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline and live:
                if event() is None:
                    time.sleep(0.001)
            assert not live, f"bootstrap tree survived keeper death: {live}"
            assert death.poll(0), "bootstrap pidfd did not report death"
    finally:
        # Exact acquired handles only: never signal recycled numeric PIDs.
        for fd in handles.values():
            try:
                signal.pidfd_send_signal(fd, signal.SIGKILL)
            except ProcessLookupError:
                pass
        deadline = time.monotonic() + 3
        while live and time.monotonic() < deadline:
            item = event()
            if item is None:
                time.sleep(0.001)
            elif item[1] is not None:
                try:
                    ptrace(7, item[0])
                except ProcessLookupError:
                    pass
        for fd in handles.values():
            os.close(fd)
