#!/usr/bin/env python3
"""Normalize Tauri Linux packages for fixed-epoch reproducible builds."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
import struct
import tarfile
import tempfile
from pathlib import Path

AR_MAGIC = b"!<arch>\n"
AR_HEADER_BYTES = 60
DEB_MEMBERS = ("debian-binary", "control.tar.gz", "data.tar.gz")
RPM_LEAD_BYTES = 96
RPM_HEADER_MAGIC = b"\x8e\xad\xe8"
RPMTAG_BUILDTIME = 1006
RPMTAG_FILEMTIMES = 1034
RPMSIGTAG_SHA256 = 273


def _read_ar(path: Path) -> dict[str, bytes]:
    payload = path.read_bytes()
    if not payload.startswith(AR_MAGIC):
        raise ValueError(f"{path} is not an ar archive")
    offset = len(AR_MAGIC)
    members: dict[str, bytes] = {}
    while offset < len(payload):
        header = payload[offset : offset + AR_HEADER_BYTES]
        if len(header) != AR_HEADER_BYTES or header[-2:] != b"`\n":
            raise ValueError(f"{path} has a malformed ar member header")
        name = header[:16].decode("ascii").strip().removesuffix("/")
        try:
            size = int(header[48:58].decode("ascii").strip())
        except ValueError as error:
            raise ValueError(f"{path} has an invalid ar member size") from error
        offset += AR_HEADER_BYTES
        members[name] = payload[offset : offset + size]
        offset += size + (size % 2)
    return members


def _write_ar_member(name: str, payload: bytes, epoch: int) -> bytes:
    encoded_name = f"{name}/"
    if len(encoded_name) > 16:
        raise ValueError(f"ar member name is too long: {name}")
    header = (
        encoded_name.ljust(16)
        + str(epoch).ljust(12)
        + "0".ljust(6)
        + "0".ljust(6)
        + oct(0o100644)[2:].ljust(8)
        + str(len(payload)).ljust(10)
        + "`\n"
    ).encode("ascii")
    return header + payload + (b"\n" if len(payload) % 2 else b"")


def _normalize_tar_gz(payload: bytes, epoch: int) -> bytes:
    source = io.BytesIO(gzip.decompress(payload))
    normalized = io.BytesIO()
    with tarfile.open(fileobj=source, mode="r:") as archive:
        members = archive.getmembers()
        file_payloads: dict[str, bytes] = {}
        for member in members:
            if not member.isfile():
                continue
            extracted = archive.extractfile(member)
            if extracted is None:
                raise ValueError(f"unable to read DEB tar member: {member.name}")
            file_payloads[member.name] = extracted.read()
        with tarfile.open(fileobj=normalized, mode="w", format=tarfile.GNU_FORMAT) as output:
            for member in sorted(members, key=lambda item: item.name):
                member.mtime = epoch
                member.uid = 0
                member.gid = 0
                member.uname = ""
                member.gname = ""
                member.pax_headers = {}
                data = io.BytesIO(file_payloads[member.name]) if member.isfile() else None
                output.addfile(member, data)
    compressed = io.BytesIO()
    with gzip.GzipFile(fileobj=compressed, mode="wb", filename="", mtime=0) as output:
        output.write(normalized.getvalue())
    return compressed.getvalue()


def normalize_deb(input_path: Path, output_path: Path, epoch: int) -> None:
    if epoch < 0:
        raise ValueError("epoch must be non-negative")
    members = _read_ar(input_path)
    missing = set(DEB_MEMBERS).difference(members)
    if missing:
        raise ValueError(f"DEB is missing required members: {', '.join(sorted(missing))}")
    normalized = {
        "debian-binary": members["debian-binary"],
        "control.tar.gz": _normalize_tar_gz(members["control.tar.gz"], epoch),
        "data.tar.gz": _normalize_tar_gz(members["data.tar.gz"], epoch),
    }
    archive = AR_MAGIC + b"".join(
        _write_ar_member(name, normalized[name], epoch) for name in DEB_MEMBERS
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=output_path.parent, delete=False) as temporary:
        temporary.write(archive)
        temporary_path = Path(temporary.name)
    os.replace(temporary_path, output_path)


def _parse_rpm_header(
    payload: bytes | bytearray, offset: int
) -> tuple[int, list[tuple[int, int, int, int, int]]]:
    if payload[offset : offset + 3] != RPM_HEADER_MAGIC:
        raise ValueError(f"invalid RPM header at byte {offset}")
    count, store_bytes = struct.unpack(">II", payload[offset + 8 : offset + 16])
    store_offset = offset + 16 + (count * 16)
    entries = []
    for index in range(count):
        entry_offset = offset + 16 + (index * 16)
        tag, kind, data_offset, item_count = struct.unpack(
            ">IIII", payload[entry_offset : entry_offset + 16]
        )
        entries.append((tag, kind, data_offset, item_count, store_offset + data_offset))
    return store_offset + store_bytes, entries


def normalize_rpm(input_path: Path, output_path: Path, epoch: int) -> None:
    if epoch < 0 or epoch > 0xFFFFFFFF:
        raise ValueError("epoch must fit an RPM unsigned 32-bit timestamp")
    payload = bytearray(input_path.read_bytes())
    signature_end, signature_entries = _parse_rpm_header(payload, RPM_LEAD_BYTES)
    header_offset = (signature_end + 7) & ~7
    header_end, header_entries = _parse_rpm_header(payload, header_offset)

    build_time_entries = [entry for entry in header_entries if entry[0] == RPMTAG_BUILDTIME]
    if (
        len(build_time_entries) != 1
        or build_time_entries[0][1] != 4
        or build_time_entries[0][3] != 1
    ):
        raise ValueError("RPM must contain exactly one INT32 BUILDTIME entry")
    build_time_offset = build_time_entries[0][4]
    payload[build_time_offset : build_time_offset + 4] = struct.pack(">I", epoch)

    file_mtime_entries = [entry for entry in header_entries if entry[0] == RPMTAG_FILEMTIMES]
    if len(file_mtime_entries) > 1 or (
        file_mtime_entries
        and (file_mtime_entries[0][1] != 4 or file_mtime_entries[0][3] < 1)
    ):
        raise ValueError("RPM FILEMTIMES entry must be a non-empty INT32 array")
    if file_mtime_entries:
        file_mtime_offset = file_mtime_entries[0][4]
        file_mtime_count = file_mtime_entries[0][3]
        payload[file_mtime_offset : file_mtime_offset + (file_mtime_count * 4)] = (
            struct.pack(">I", epoch) * file_mtime_count
        )

    digest_entries = [entry for entry in signature_entries if entry[0] == RPMSIGTAG_SHA256]
    if len(digest_entries) != 1 or digest_entries[0][1] != 6:
        raise ValueError("RPM must contain exactly one SHA256 signature string")
    digest_offset = digest_entries[0][4]
    terminator = payload.find(b"\0", digest_offset, signature_end)
    if terminator - digest_offset != 64:
        raise ValueError("RPM SHA256 signature must be a 64-character hex string")
    digest = hashlib.sha256(payload[header_offset:header_end]).hexdigest().encode("ascii")
    payload[digest_offset:terminator] = digest

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=output_path.parent, delete=False) as temporary:
        temporary.write(payload)
        temporary_path = Path(temporary.name)
    os.replace(temporary_path, output_path)


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    deb = subparsers.add_parser("normalize-deb")
    deb.add_argument("--input", type=Path, required=True)
    deb.add_argument("--output", type=Path, required=True)
    deb.add_argument("--epoch", type=int, required=True)
    rpm = subparsers.add_parser("normalize-rpm")
    rpm.add_argument("--input", type=Path, required=True)
    rpm.add_argument("--output", type=Path, required=True)
    rpm.add_argument("--epoch", type=int, required=True)
    args = parser.parse_args()
    if args.command == "normalize-deb":
        normalize_deb(args.input, args.output, args.epoch)
    else:
        normalize_rpm(args.input, args.output, args.epoch)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
