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

# 🪙
# ATHENA Quick Reference

Status: complete (reviewed 2026-04-30; v0.1 baseline; coin-marked for cleanup/archive targeting)
Owner: athena
Human plan: `human/plans/ATHENA.md`
Crate: `crates/annunimas-athena`
Core runtime: `core/state/athena_runtime.json`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

ATHENA owns ingest, digest, deep analysis, and policy-readiness promotion for the sovereign knowledge corpus.

## Current Contract

- ingest/query/deep/digest command surface is live
- JSONL read paths are hardened against malformed lines
- JSONL appends are serialized to avoid interleaving corruption
- planning-task generation can emit work from evidence
- human and machine source books now preserve deep digestion across deduplicated re-ingest
- Scrapling runtime and provider policy are now materialized in `core/state/scrapling_runtime_contract.json`

## Integration Program

- keep `crawl4ai` as the live ATHENA crawl runtime until a bounded Scrapling runtime contract exists
- treat Scrapling as the preferred long-term crawl/extraction direction, not yet the sovereign default
- promote policy-ready ATHENA evidence into implementation briefs and deterministic planning tasks
- absorb framework learnings into sovereign crate, adapter, and workflow contracts rather than external shells
- bind ATHENA outputs directly into runtime-governor, package, and APOLLO execution surfaces

## Current Frontier

- materialize a formal Scrapling runtime contract and provider-selection policy
- execute the bounded Scrapling promotion gates without displacing `crawl4ai` prematurely
- harden ATHENA around workstation-first execution, deterministic task emission, bounded memory lanes, and measurable runtime effectiveness

## Hardening Contract

- workstation is the canonical ATHENA execution authority
- laptop is operator ingress and optional fallback, not the default deep-ingest executor
- source provenance must survive ingest through policy-ready promotion and task emission
- memory lanes stay bounded across episodic, source-book, policy-ready, and implementation-ready surfaces
- task emission must be deterministic, idempotent, and receipt-backed
- ATHENA runtime must remain SELinux-safe, admission-gated, and observable

## Primary Runtime Surfaces

- `data/athena/digest.jsonl`
- `data/athena/books/`
- `data/athena/policy_readiness.jsonl`
- `human/library/athena/sources/`

## Readable Context

Use `human/plans/ATHENA.md` for the operator-facing plan narrative and graph node.
