import json
import os
import time
from pathlib import Path

TOKEN = b"ARDA_TASK7_ACCEPTANCE=phase3-task7-final-exact-20260820"
OUT = Path(__file__).with_name("native-performance.json")
INTERVAL = 10
SAMPLES = 31
FIELDS = {"Rss", "Pss", "Private_Clean", "Private_Dirty", "Shared_Clean", "Shared_Dirty", "Anonymous", "Swap"}

def root_pid():
    for item in Path("/proc").iterdir():
        if not item.name.isdigit():
            continue
        try:
            if TOKEN in (item / "environ").read_bytes():
                return int(item.name)
        except OSError:
            pass
    raise RuntimeError("tagged HUD process not found")

def descendants(root):
    parents = {}
    for item in Path("/proc").iterdir():
        if not item.name.isdigit():
            continue
        try:
            stat = (item / "stat").read_text()
            end = stat.rfind(")")
            ppid = int(stat[end + 2:].split()[1])
            parents.setdefault(ppid, []).append(int(item.name))
        except (OSError, ValueError, IndexError):
            pass
    found, queue = [], [root]
    while queue:
        pid = queue.pop(0)
        if pid in found:
            continue
        found.append(pid)
        queue.extend(parents.get(pid, []))
    return found

def proc_sample(pid):
    base = Path("/proc") / str(pid)
    stat = (base / "stat").read_text()
    end = stat.rfind(")")
    fields = stat[end + 2:].split()
    ticks = int(fields[11]) + int(fields[12])
    memory = {key: 0 for key in FIELDS}
    for line in (base / "smaps_rollup").read_text().splitlines():
        key = line.split(":", 1)[0]
        if key in memory:
            memory[key] = int(line.split()[1])
    return {
        "pid": pid,
        "comm": (base / "comm").read_text().strip(),
        "cpu_ticks": ticks,
        "memory_kib": memory,
    }

root = root_pid()
result = {"schema_version": "arda.hud.native-performance.v1", "root_pid": root, "interval_seconds": INTERVAL, "samples": []}
for index in range(SAMPLES):
    processes = []
    for pid in descendants(root):
        try:
            processes.append(proc_sample(pid))
        except OSError:
            pass
    result["samples"].append({"index": index, "monotonic_seconds": time.monotonic(), "processes": processes})
    if index + 1 < SAMPLES:
        time.sleep(INTERVAL)
OUT.write_text(json.dumps(result, indent=2) + "\n")
print(OUT)
