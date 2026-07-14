# Arda AGENTS.md

This file is the source of truth for assistants and agents working in Arda.
Read it before editing code, docs, or planning work.

## Repository Layout
- Arda canonical root: `/var/home/mythos/Eregion/Arda`
- Reference architecture lives in `~/Annunimas`
- Do not modify `~/Annunimas` unless the user explicitly asks

## Tooling
- Use `pnpm run tauri dev` and `pnpm run tauri build` by default
- If Tauri packaging breaks, fix the root cause instead of adding workarounds
- Verify runtime/build state before updating docs or plans

## Work Style
- Prefer direct terminal-style output: no filler, no permission prompts
- Verify with evidence; don’t claim success without a real check
- Make small targeted fixes, not broad drive-by refactors
- Save durable corrections to memory/skills when they prevent repeating instructions
