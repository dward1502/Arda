#!/usr/bin/python3
"""Linux mechanism probe, not installed-Hermes acceptance.

Exercise completed mount capture, SCM_RIGHTS transfer, helper reap, namespace
re-entry and bubblewrap execution after a hostile descendant mount replacement.
Run under unshare --user --map-root-user --mount --propagation private.
"""
import array
import ctypes
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile

LIBC = ctypes.CDLL(None, use_errno=True)


def checked(result):
    if result == -1:
        raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))


def capture(fd, root, staging):
    channel = socket.socket(fileno=int(fd))
    # This ephemeral mode retains the whole private namespace. Its root FD
    # therefore belongs to the frozen mount tree, unlike a caller-namespace FD.
    tree = Path(root)
    fds = [os.open(p, os.O_RDONLY | os.O_CLOEXEC) for p in
           ["/proc/self/ns/user", "/proc/self/ns/mnt", str(tree)]]
    channel.sendmsg([b"capture-v1"], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", fds))])
    for descriptor in fds:
        os.close(descriptor)
    channel.close()


def launch(root, user_fd, mount_fd, tree_fd):
    checked(LIBC.setns(int(user_fd), 0x10000000))
    checked(LIBC.setns(int(mount_fd), 0x00020000))
    os.chdir("/")
    os.close(int(user_fd))
    os.close(int(mount_fd))
    script = """import os
from pathlib import Path
for name in os.listdir('/proc/self/fd'):
    try:
        target = os.readlink('/proc/self/fd/' + name)
    except FileNotFoundError:
        continue
    assert not target.startswith(('user:[', 'mnt:[')), target
Path('nested/proof').write_text('original captured tree')
print('provider succeeded; no namespace descriptors leaked')
"""
    os.execve("/usr/bin/bwrap", ["bwrap", "--unshare-user", "--unshare-pid",
              "--disable-userns", "--cap-drop", "ALL", "--ro-bind", "/", "/",
              "--proc", "/proc", "--dev", "/dev", "--bind-fd", tree_fd, root,
              "--chdir", root, "--", "/usr/bin/python3", "-I", "-c", script], {})


def probe():
    with tempfile.TemporaryDirectory(prefix="arda-capture-probe-") as temporary:
        root = Path(temporary) / "workspace"
        nested = root / "nested"
        nested.mkdir(parents=True)
        outside = Path(temporary) / "outside"
        outside.mkdir()
        staging = Path(temporary) / "staging"
        staging.mkdir()
        parent, child = socket.socketpair()
        parent.settimeout(10)
        process = subprocess.Popen(["unshare", "--user", "--map-root-user", "--mount",
                                    "--propagation", "private", sys.executable, __file__,
                                    "capture", str(child.fileno()), str(root), str(staging)],
                                   pass_fds=[child.fileno()], env={"PATH": "/usr/bin:/bin"})
        child.close()
        fds = array.array("i")
        mounted = False
        try:
            data, ancillary, flags, _ = parent.recvmsg(32, socket.CMSG_SPACE(3 * fds.itemsize), socket.MSG_CMSG_CLOEXEC)
            for level, kind, payload in ancillary:
                assert (level, kind) == (socket.SOL_SOCKET, socket.SCM_RIGHTS)
                fds.frombytes(payload)
            assert data == b"capture-v1" and len(fds) == 3 and not flags & socket.MSG_CTRUNC
            assert process.wait(timeout=10) == 0
            assert all(not os.get_inheritable(fd) for fd in fds)
            # Capture owner has exited. Mutate the caller's mount tree only now.
            subprocess.run(["mount", "--bind", str(outside), str(nested)], check=True)
            mounted = True
            subprocess.run([sys.executable, __file__, "launch", str(root), *map(str, fds)],
                           pass_fds=fds, env={}, check=True, timeout=10)
            assert not (outside / "proof").exists(), "write escaped into replacement"
            subprocess.run(["umount", str(nested)], check=True)
            mounted = False
            assert (nested / "proof").read_text() == "original captured tree"
            print("PASS: reaped capture helper, FD transfer, namespace re-entry, original-tree write, replacement untouched")
        finally:
            parent.close()
            if process.poll() is None:
                process.kill()
                process.wait()
            if mounted:
                subprocess.run(["umount", str(nested)], check=True)
            for descriptor in fds:
                os.close(descriptor)


if __name__ == "__main__":
    if len(sys.argv) == 1:
        probe()
    elif sys.argv[1] == "capture":
        capture(*sys.argv[2:])
    elif sys.argv[1] == "launch":
        launch(*sys.argv[2:])
    else:
        raise SystemExit("unknown mode")
