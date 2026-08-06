#!/usr/bin/env python3
"""Collect and evaluate the seven-day RELIC/CITADEL soak gate."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import subprocess
import sys
import tempfile
from typing import Any

SCHEMA = "arda.relic-citadel.soak-sample.v1"
DEFAULT_ROOT = pathlib.Path.home() / ".local/state/arda/relic-soak"
LOCAL_SCENE = pathlib.Path(os.environ.get("XDG_RUNTIME_DIR", "/tmp")) / "arda-relic-bridge/scene.json"
REMOTE_SCENE = "/home/citadel/annunimas_embodied/relic/public/scene.json"
SERVICES_LOCAL = ("arda.service", "arda-relic-bridge.service")
SERVICES_REMOTE = ("relic.service", "citadel-kiosk.service")
# Current undervoltage, frequency-cap, and hard-throttle flags. Historical flags
# remain recorded in each sample, while the accepted ambient-heat posture
# (`0xe0008`) is allowed when only the current soft-temperature bit is set.
HARD_THROTTLE_MASK = 0x7


def run(command: list[str]) -> str:
    return subprocess.run(
        command,
        check=True,
        capture_output=True,
        text=True,
        timeout=20,
    ).stdout.strip()


def systemd_properties(command: list[str], services: tuple[str, ...]) -> dict[str, dict[str, Any]]:
    output = run(
        command
        + [
            "show",
            *services,
            "--property=Id,ActiveState,SubState,NRestarts,MemoryCurrent,ActiveEnterTimestamp",
        ]
    )
    records: dict[str, dict[str, Any]] = {}
    for block in output.split("\n\n"):
        values: dict[str, Any] = dict(
            line.split("=", 1) for line in block.splitlines() if "=" in line
        )
        if not values.get("Id"):
            continue
        for key in ("NRestarts", "MemoryCurrent"):
            try:
                values[key] = int(values.get(key, "0"))
            except ValueError:
                values[key] = None
        records[values["Id"]] = values
    return records


def scene_record(path: pathlib.Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    return {
        "path": str(path),
        "bytes": path.stat().st_size,
        "schema_version": payload.get("schema_version"),
        "scene_state": payload.get("scene_state"),
        "forms": len(payload.get("forms", [])),
        "status_text": payload.get("status_text"),
    }


def remote_snapshot(host: str) -> dict[str, Any]:
    probe = f'''python3 - <<'PY'
import json, pathlib, shutil
p=pathlib.Path("{REMOTE_SCENE}")
d=json.loads(p.read_text())
thermal=pathlib.Path("/sys/class/thermal/thermal_zone0/temp")
print(json.dumps({{
 "scene": {{"path": str(p), "bytes": p.stat().st_size, "schema_version": d.get("schema_version"), "scene_state": d.get("scene_state"), "forms": len(d.get("forms", [])), "status_text": d.get("status_text")}},
 "temperature_millidegrees_c": int(thermal.read_text().strip()),
 "disk": dict(zip(("total_bytes","used_bytes","free_bytes"), shutil.disk_usage("/")))
}}))
PY
vcgencmd get_throttled 2>/dev/null || true'''
    lines = run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=6", host, probe]).splitlines()
    snapshot = json.loads(lines[0])
    throttle = next((line for line in lines[1:] if line.startswith("throttled=")), "throttled=unknown")
    value = throttle.split("=", 1)[1]
    snapshot["throttled"] = value
    snapshot["throttled_value"] = int(value, 16) if value.startswith("0x") else None
    snapshot["services"] = systemd_properties(
        ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=6", host, "systemctl", "--user"],
        SERVICES_REMOTE,
    )
    return snapshot


def sample(root: pathlib.Path, host: str) -> pathlib.Path:
    now = dt.datetime.now(dt.timezone.utc)
    payload = {
        "contract": SCHEMA,
        "sampled_at_utc": now.isoformat(),
        "local": {
            "services": systemd_properties(["systemctl", "--user"], SERVICES_LOCAL),
            "scene": scene_record(LOCAL_SCENE),
        },
        "citadel": remote_snapshot(host),
        "budgets": {
            "scene_max_bytes": 50 * 1024,
            "hard_throttle_mask": hex(HARD_THROTTLE_MASK),
        },
    }
    root.mkdir(parents=True, exist_ok=True)
    destination = root / now.strftime("sample-%Y%m%dT%H%M%SZ.json")
    with tempfile.NamedTemporaryFile("w", dir=root, delete=False, encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")
        temporary = pathlib.Path(handle.name)
    temporary.replace(destination)
    return destination


def evaluate(root: pathlib.Path) -> tuple[bool, list[str]]:
    samples = [json.loads(path.read_text(encoding="utf-8")) for path in sorted(root.glob("sample-*.json"))]
    failures: list[str] = []
    if len(samples) < 8:
        failures.append(f"need at least 8 daily samples; found {len(samples)}")
    if samples:
        first = dt.datetime.fromisoformat(samples[0]["sampled_at_utc"])
        last = dt.datetime.fromisoformat(samples[-1]["sampled_at_utc"])
        if last - first < dt.timedelta(days=7):
            failures.append(f"sample window is {last - first}; need at least 7 days")
    for index, item in enumerate(samples):
        stamp = item["sampled_at_utc"]
        for side in ("local", "citadel"):
            for name, service in item[side]["services"].items():
                if service.get("ActiveState") != "active":
                    failures.append(f"{stamp}: {name} is {service.get('ActiveState')}")
            scene = item[side]["scene"]
            if scene.get("schema_version") != "arda.relic.scene-adapter.v1":
                failures.append(f"{stamp}: {side} scene schema invalid")
            if scene.get("bytes", 0) > 50 * 1024:
                failures.append(f"{stamp}: {side} scene exceeds 50 KiB")
        throttle = item["citadel"].get("throttled_value")
        if throttle is None or throttle & HARD_THROTTLE_MASK:
            failures.append(f"{stamp}: failing/unknown throttle state {item['citadel'].get('throttled')}")
        if index:
            previous = samples[index - 1]
            for side in ("local", "citadel"):
                for name, service in item[side]["services"].items():
                    old = previous[side]["services"].get(name, {})
                    if service.get("NRestarts") != old.get("NRestarts"):
                        failures.append(f"{stamp}: {name} restart count changed")
    return not failures, failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("sample", "evaluate"))
    parser.add_argument("--root", type=pathlib.Path, default=DEFAULT_ROOT)
    parser.add_argument("--host", default=os.environ.get("ARDA_RELIC_REMOTE_HOST", "citadel"))
    args = parser.parse_args()
    if args.action == "sample":
        path = sample(args.root, args.host)
        print(f"relic_citadel_soak_sample={path}")
        return 0
    passed, failures = evaluate(args.root)
    print(json.dumps({"passed": passed, "failures": failures}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
