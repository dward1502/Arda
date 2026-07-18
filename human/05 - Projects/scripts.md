---
title: "Scripts"
last_updated: 2026-05-14
soterion:
  type: project_summary
  category: summaries
  project: annunimas
  agent_access: public
  mnemosyne_priority: high
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# scripts/

**Purpose**: Automation scripts for Annunimas system

## Scripts (6 total)

| Script | Status | Purpose |
|--------|--------|---------|
| `boot.sh` | Stub | System boot (placeholder) |
| `monitor_fleet.sh` | Stub | Fleet monitoring (placeholder) |
| `role_switch.sh` | Stub | Role switching (placeholder) |
| `backup.sh` | Stub | Backup operations (placeholder) |
| `update_index.sh` | **Active** | Generates navigation docs from crate READMEs |
| `update_memory.sh` | Active | Runs graphthulhu to update knowledge graph |

## Key Scripts

### update_index.sh (Active - 242 lines)
Generates navigation documentation:
- `CRATE_INDEX.md` - Crate table from README frontmatter
- `NAV_PATHS.txt` - All file paths (machine-friendly)
- `file-tree.md` - Condensed tree overview
- `NAV_GRAPH.json` - JSON graph of folders/files

Extracts YAML frontmatter from crate READMEs:
- crate, kind, agent, realm, status, capabilities

### update_memory.sh (Active - 3 lines)
Runs graphthulhu to update knowledge graph:
```bash
cargo run --package annunimas-graphthulhu --bin graphthulhu -- --dir data/memory
```

---

## Thoughts

**Good:**
- update_index.sh is a solid, working script
- Proper YAML frontmatter parsing
- Generates multiple useful navigation files
- update_memory.sh connects to graphthulhu

**Needs Work:**
- 4 of 6 scripts are stubs (just echo)
- backup.sh, role_switch.sh, monitor_fleet.sh, boot.sh need implementation
- No error handling in update_memory.sh
- No cron/automation for these scripts

**Improve:**
- Implement stub scripts
- Add error handling
- Add to cron for automated updates
- Consider combining into single management script
