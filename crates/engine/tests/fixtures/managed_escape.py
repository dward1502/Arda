"""Adversarial local fixture, not Hermes/provider acceptance."""
import ctypes
import errno
import json
import os
from pathlib import Path
import signal
import socket
import time


def main():
    if os.fork():
        while True:
            time.sleep(1)
    os.setsid()
    if os.fork():
        os._exit(0)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    libc = ctypes.CDLL(None, use_errno=True)
    results = {}
    results['nested_userns_denied'] = libc.unshare(0x10000000) == -1
    results['cgroup_mount_denied'] = libc.mount(b'none', b'/var/tmp', b'cgroup2', 0, None) == -1
    for label, path in json.loads(Path('escape-targets.json').read_text()).items():
        try:
            if label.startswith('bus'):
                with socket.socket(socket.AF_UNIX) as sock:
                    sock.settimeout(1)
                    sock.connect(path)
            else:
                with open(path, 'w') as stream:
                    stream.write(str(os.getpid()))
            results[label] = False
        except OSError as error:
            results[label] = error.errno in (errno.ENOENT, errno.EACCES, errno.EPERM, errno.EROFS, errno.ENOTDIR, errno.ECONNREFUSED)
    # No inherited directory, cgroup or manager socket capability may bypass mounts.
    results['no_extra_fds'] = all(int(fd) <= 2 for fd in os.listdir('/proc/self/fd') if os.path.exists('/proc/self/fd/' + fd))
    results['namespace_pid'] = os.getpid()
    Path('escape-report.tmp').write_text(json.dumps(results))
    os.rename('escape-report.tmp', 'escape-report.json')
    # Bounded even if the confinement under test regresses; normal completion is SIGKILL.
    time.sleep(30)
    os._exit(0)
