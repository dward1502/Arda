"""Hash regular artifacts beneath retained cwd; never follow symlinks.

Executed with isolated system Python inside the same bounded worker as the job.
No provider environment, imports, or executable search path is inherited.
"""
import hashlib
import json
import os
import stat
import sys

rows = []
for artifact in json.loads(sys.argv[1]):
    parts = artifact["path"].split("/")
    if any(part in ("", ".", "..") for part in parts):
        raise ValueError("artifact path must be normalized and relative")
    directory = os.open(".", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory)
            os.close(directory)
            directory = child
        fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
        with os.fdopen(fd, "rb") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise ValueError("artifact must be a regular file")
            digest = hashlib.sha256()
            while chunk := stream.read(65536):
                digest.update(chunk)
            if "sha256:" + digest.hexdigest() != artifact["digest"]:
                raise ValueError("artifact digest mismatch")
            rows.append({"path": artifact["path"], "sha256": digest.hexdigest()})
    finally:
        os.close(directory)
print(json.dumps(rows))
