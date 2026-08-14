#!/usr/bin/env python3
"""Measure aggregate CPU, RSS, PSS, and smaps ownership for a tagged HUD tree.

Launch the HUD with a unique ``ARDA_HUD_PROFILE_TOKEN`` in its environment, then
run this script with the same token. Optional environment variables control the
JSON destination, sample interval, and duration.
"""

import json
import os
import time
from pathlib import Path

TOKEN = os.environ["ARDA_HUD_PROFILE_TOKEN"]
OUT = Path(os.environ.get("ARDA_HUD_PROFILE_OUT", "/tmp/arda-hud-native-profile.json"))
INTERVAL = float(os.environ.get("ARDA_HUD_PROFILE_INTERVAL", "30"))
DURATION = float(os.environ.get("ARDA_HUD_PROFILE_DURATION", "300"))
HZ = os.sysconf(os.sysconf_names["SC_CLK_TCK"])
PAGE_KIB = os.sysconf("SC_PAGE_SIZE") / 1024


def read_bytes(path: Path) -> bytes:
    try:
        return path.read_bytes()
    except (OSError, PermissionError):
        return b""


def find_root() -> int | None:
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        env = read_bytes(proc / "environ")
        cmd = read_bytes(proc / "cmdline").replace(b"\0", b" ")
        if TOKEN.encode() in env and b"arda_hud" in cmd:
            return int(proc.name)
    return None


def process_tree(root: int) -> list[int]:
    pids = {root}
    changed = True
    while changed:
        changed = False
        for proc in Path("/proc").iterdir():
            if not proc.name.isdigit() or int(proc.name) in pids:
                continue
            try:
                stat = (proc / "stat").read_text()
                after = stat[stat.rfind(")") + 2 :].split()
                ppid = int(after[1])
            except (OSError, ValueError, IndexError):
                continue
            if ppid in pids:
                pids.add(int(proc.name))
                changed = True
    return sorted(pids)


def classify(command: str) -> str:
    if "NetworkProcess" in command:
        return "webkit_network"
    if "WebKitWebProcess" in command:
        return "webkit_web"
    if "WebKitGPUProcess" in command:
        return "webkit_gpu"
    if "arda_hud" in command:
        return "launcher"
    return "other"


def proc_sample(pid: int) -> dict[str, object] | None:
    proc = Path("/proc") / str(pid)
    try:
        stat = (proc / "stat").read_text()
        after = stat[stat.rfind(")") + 2 :].split()
        ticks = int(after[11]) + int(after[12])
        command = (
            read_bytes(proc / "cmdline")
            .replace(b"\0", b" ")
            .decode(errors="replace")
            .strip()
        )
        rss_kib = int(after[21]) * PAGE_KIB
    except (OSError, ValueError, IndexError):
        return None

    memory: dict[str, float | int] = {"rss_kib": round(rss_kib, 2)}
    try:
        for line in (proc / "smaps_rollup").read_text().splitlines():
            if ":" not in line:
                continue
            key, rest = line.split(":", 1)
            if key in {
                "Rss",
                "Pss",
                "Pss_Anon",
                "Pss_File",
                "Pss_Shmem",
                "Private_Clean",
                "Private_Dirty",
                "Shared_Clean",
                "Shared_Dirty",
                "Anonymous",
                "Swap",
            }:
                memory[key.lower() + "_kib"] = int(rest.split()[0])
    except (OSError, ValueError, IndexError):
        pass
    return {
        "pid": pid,
        "role": classify(command),
        "command": command,
        "cpu_ticks": ticks,
        **memory,
    }


def main() -> None:
    deadline = time.monotonic() + 30
    root = None
    while time.monotonic() < deadline and root is None:
        root = find_root()
        if root is None:
            time.sleep(0.25)
    if root is None:
        raise SystemExit("profile root not found")

    started = time.monotonic()
    samples = []
    while True:
        now = time.monotonic()
        rows = [row for pid in process_tree(root) if (row := proc_sample(pid))]
        totals: dict[str, float] = {}
        for row in rows:
            for key, value in row.items():
                if (key.endswith("_kib") or key == "cpu_ticks") and isinstance(
                    value, (int, float)
                ):
                    totals[key] = round(totals.get(key, 0) + float(value), 2)
        samples.append(
            {
                "elapsed_seconds": round(now - started, 3),
                "processes": rows,
                "totals": totals,
            }
        )
        remaining = DURATION - (now - started)
        if remaining <= 0:
            break
        time.sleep(min(INTERVAL, remaining))

    elapsed = samples[-1]["elapsed_seconds"] - samples[0]["elapsed_seconds"]
    first_ticks = samples[0]["totals"].get("cpu_ticks", 0)
    last_ticks = samples[-1]["totals"].get("cpu_ticks", 0)
    cpu_percent = ((last_ticks - first_ticks) / HZ / elapsed * 100) if elapsed else 0
    metric_keys = sorted(
        {key for sample in samples for key in sample["totals"] if key.endswith("_kib")}
    )
    summary = {
        "root_pid": root,
        "measurement_seconds": round(elapsed, 3),
        "sample_interval_seconds": INTERVAL,
        "samples": len(samples),
        "aggregate_cpu_one_core_percent": round(cpu_percent, 2),
    }
    for key in metric_keys:
        values = [sample["totals"].get(key, 0) for sample in samples]
        stem = key.removesuffix("_kib") + "_mib"
        summary[f"{stem}_start"] = round(values[0] / 1024, 2)
        summary[f"{stem}_end"] = round(values[-1] / 1024, 2)
        summary[f"{stem}_peak"] = round(max(values) / 1024, 2)
        summary[f"{stem}_growth"] = round((values[-1] - values[0]) / 1024, 2)

    role_summary = {}
    roles = sorted({process["role"] for sample in samples for process in sample["processes"]})
    for role in roles:
        role_summary[role] = {}
        for key in metric_keys:
            values = [
                sum(
                    float(process.get(key, 0))
                    for process in sample["processes"]
                    if process["role"] == role
                )
                for sample in samples
            ]
            stem = key.removesuffix("_kib") + "_mib"
            role_summary[role][f"{stem}_start"] = round(values[0] / 1024, 2)
            role_summary[role][f"{stem}_end"] = round(values[-1] / 1024, 2)
            role_summary[role][f"{stem}_peak"] = round(max(values) / 1024, 2)

    result = {
        "contract": "arda.hud.native-process-profile.v1",
        "token": TOKEN,
        "summary": summary,
        "roles": role_summary,
        "samples": samples,
    }
    OUT.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"output": str(OUT), "summary": summary, "roles": role_summary}, indent=2))


if __name__ == "__main__":
    main()
