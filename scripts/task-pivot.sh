#!/usr/bin/env bash
set -euo pipefail

# Preserve the entrypoint for old callers, but never reopen historical authority.
printf '%s\n' 'task-pivot is retired; use authenticated Engine objective intake/control. Legacy queue history is read-only.' >&2
exit 1
