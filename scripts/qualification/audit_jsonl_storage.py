#!/usr/bin/env python3
"""Read-only JSONL integrity inventory; never prints record contents or deletes data.

Scan only the bytes present when each file is opened. An unterminated final
record is reported separately: an active writer may still be completing it.
Rotation/truncation during the scan makes the result inconclusive.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path


def reject_nonfinite(value):
    raise ValueError(f"Non-finite JSON constant: {value}")


def audit(path):
    path = Path(path)
    result = {"path": str(path), "valid_records": 0, "blank_lines": 0,
              "malformed_records": 0, "malformed_examples": [],
              "unterminated_tail": None}
    with path.open("rb") as handle:
        before = os.fstat(handle.fileno())
        result.update(snapshot_bytes=before.st_size, mtime_ns=before.st_mtime_ns)
        remaining = before.st_size
        offset = 0
        digest = hashlib.sha256()
        while remaining:
            line = handle.readline(remaining)
            if not line:
                break
            digest.update(line)
            remaining -= len(line)
            terminated = line.endswith(b"\n")
            try:
                if line.strip():
                    json.loads(line.decode("utf-8"), parse_constant=reject_nonfinite)
                    valid = True
                else:
                    valid = None
            except (ValueError, UnicodeError, RecursionError):
                valid = False
            if not terminated:
                result["unterminated_tail"] = {
                    "offset": offset, "bytes": len(line), "valid_json": valid,
                }
            elif valid is True:
                result["valid_records"] += 1
            elif valid is None:
                result["blank_lines"] += 1
            else:
                result["malformed_records"] += 1
                if len(result["malformed_examples"]) < 5:
                    result["malformed_examples"].append({
                        "offset": offset, "bytes": len(line),
                        "sha256": hashlib.sha256(line).hexdigest(),
                    })
            offset += len(line)
        after = os.fstat(handle.fileno())
    try:
        current = path.stat()
        replaced = (current.st_dev, current.st_ino) != (before.st_dev, before.st_ino)
    except FileNotFoundError:
        replaced = True
    result.update(bytes_read=offset, sha256=digest.hexdigest(),
                  appended_during_scan=after.st_size > before.st_size,
                  scan_inconclusive=bool(remaining or replaced or after.st_size < before.st_size
                      or (after.st_size == before.st_size and after.st_mtime_ns != before.st_mtime_ns)))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    results = []
    for path in args.paths:
        try:
            results.append(audit(path))
        except OSError as error:
            results.append({"path": str(path), "error": type(error).__name__,
                            "scan_inconclusive": True})
    print(json.dumps(results, indent=2))
    return int(any(row.get("malformed_records") or row.get("scan_inconclusive") for row in results))


if __name__ == "__main__":
    raise SystemExit(main())
