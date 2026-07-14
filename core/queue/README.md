---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Queue

**Location:** `/var/home/mythos/Annunimas/core/queue`
**Created:** 2026-04-12

## Overview

This directory contains task and message queues.
Queues manage the flow of work through the system.

## Structure

- **Files:** 1
- **Subdirectories:** 0
- **Total Size:** 1,776 bytes

**File Types:**
  - .jsonl: 1 file(s)

## Key Files

**All Files:**
- [`queue.jsonl`](/var/home/mythos/Annunimas/core/queue/queue.jsonl) - 1,776 bytes

## Usage

This directory contains operational data that drives system workflows. Modify with caution.

## Maintenance

- **Backup:** Regular backups are recommended for critical data
- **Cleanup:** Use system tools for cleanup operations
- **Monitoring:** Changes to these files may affect system behavior

## Related Components

- [annunimas-prometheus](/var/home/mythos/Annunimas/crates/annunimas-prometheus) - Queue processing
