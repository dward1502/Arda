from __future__ import annotations

import gzip
import hashlib
import io
import struct
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from arda_reproducible_packages import (
    RPM_HEADER_MAGIC,
    RPM_LEAD_BYTES,
    _parse_rpm_header,
    _read_ar,
    normalize_deb,
    normalize_rpm,
)


def _tar_gz(entries: dict[str, bytes], mtime: int) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.GNU_FORMAT) as archive:
        for name, payload in entries.items():
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            info.mode = 0o644
            info.mtime = mtime
            archive.addfile(info, io.BytesIO(payload))
    return gzip.compress(buffer.getvalue(), mtime=mtime)


def _ar_member(name: str, payload: bytes, mtime: int) -> bytes:
    label = f"{name}/".ljust(16)
    header = (
        label
        + str(mtime).ljust(12)
        + "0".ljust(6)
        + "0".ljust(6)
        + oct(0o100644)[2:].ljust(8)
        + str(len(payload)).ljust(10)
        + "`\n"
    ).encode("ascii")
    return header + payload + (b"\n" if len(payload) % 2 else b"")


def _write_deb(path: Path, mtime: int) -> None:
    members = {
        "debian-binary": b"2.0\n",
        "control.tar.gz": _tar_gz({"control": b"Package: arda-test\n"}, mtime),
        "data.tar.gz": _tar_gz({"usr/bin/arda-test": b"binary"}, mtime),
    }
    path.write_bytes(
        b"!<arch>\n"
        + b"".join(_ar_member(name, payload, mtime) for name, payload in members.items())
    )


def _rpm_header(entries: list[tuple[int, int, int, int]], store: bytes) -> bytes:
    return (
        RPM_HEADER_MAGIC
        + b"\x01"
        + bytes(4)
        + struct.pack(">II", len(entries), len(store))
        + b"".join(struct.pack(">IIII", *entry) for entry in entries)
        + store
    )


def _write_rpm(path: Path, build_time: int) -> None:
    main_header = _rpm_header(
        [(1006, 4, 0, 1), (1034, 4, 4, 2)],
        struct.pack(">III", build_time, build_time, build_time),
    )
    digest = hashlib.sha256(main_header).hexdigest().encode("ascii") + b"\0"
    signature = _rpm_header([(273, 6, 0, 1)], digest)
    signature += bytes((-len(signature)) % 8)
    path.write_bytes(bytes(RPM_LEAD_BYTES) + signature + main_header + b"payload")


class ReproduciblePackageTests(unittest.TestCase):
    def test_normalized_debs_are_identical_despite_input_timestamps(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "first.deb"
            second = root / "second.deb"
            first_output = root / "first-normalized.deb"
            second_output = root / "second-normalized.deb"
            _write_deb(first, 100)
            _write_deb(second, 200)

            normalize_deb(first, first_output, 42)
            normalize_deb(second, second_output, 42)

            self.assertEqual(first_output.read_bytes(), second_output.read_bytes())
            members = _read_ar(first_output)
            self.assertEqual(set(members), {"debian-binary", "control.tar.gz", "data.tar.gz"})
            with tarfile.open(fileobj=io.BytesIO(gzip.decompress(members["data.tar.gz"]))) as archive:
                self.assertEqual(archive.getnames(), ["usr/bin/arda-test"])

    def test_normalized_rpms_are_identical_and_have_a_valid_header_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "first.rpm"
            second = root / "second.rpm"
            first_output = root / "first-normalized.rpm"
            second_output = root / "second-normalized.rpm"
            _write_rpm(first, 100)
            _write_rpm(second, 200)

            normalize_rpm(first, first_output, 42)
            normalize_rpm(second, second_output, 42)

            self.assertEqual(first_output.read_bytes(), second_output.read_bytes())
            payload = first_output.read_bytes()
            signature_end, signature_entries = _parse_rpm_header(payload, RPM_LEAD_BYTES)
            header_offset = (signature_end + 7) & ~7
            header_end, header_entries = _parse_rpm_header(payload, header_offset)
            build_time = next(entry for entry in header_entries if entry[0] == 1006)
            self.assertEqual(
                struct.unpack(">I", payload[build_time[4] : build_time[4] + 4])[0], 42
            )
            signature = next(entry for entry in signature_entries if entry[0] == 273)
            expected_digest = hashlib.sha256(payload[header_offset:header_end]).hexdigest().encode()
            self.assertEqual(payload[signature[4] : signature[4] + 64], expected_digest)


if __name__ == "__main__":
    unittest.main()
