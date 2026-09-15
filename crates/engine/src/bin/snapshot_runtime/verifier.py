"""Worker-owned artifact verifier: paths are data, never executable code."""
import hashlib
import json
import os
import stat
import sys

rows = []
for path in json.loads(sys.argv[1]):
    directory = os.open(".", os.O_PATH | os.O_DIRECTORY)
    try:
        parts = path.split("/")
        if any(part in ("", ".", "..") for part in parts):
            raise ValueError("invalid artifact path")
        for part in parts[:-1]:
            child = os.open(part, os.O_PATH | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory)
            os.close(directory)
            directory = child
        artifact = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
    finally:
        os.close(directory)
    with os.fdopen(artifact, "rb", closefd=True) as handle:
        if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
            raise ValueError("artifact is not a regular file")
        digest = hashlib.sha256()
        while chunk := handle.read(65536):
            digest.update(chunk)
    rows.append({"path": path, "sha256": digest.hexdigest()})
print(json.dumps(rows))
