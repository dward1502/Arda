---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "operations_acceptance"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-17"
---

> 🜏 Soterion: 📜 operations_acceptance | owner: HERMES | status: active | reviewed: 2026-08-17

# Hermes Continuity and Surface Handoff Acceptance

**Disposition:** Ambient-agent Phase 2 is accepted on the declared single-operator local profile on 2026-08-17. This proves one genuine phone-originated Hermes conversation continuing through the native desktop/HUD path with the same authenticated Hermes lineage and durable commitment references. It does not claim multi-user, remote, public-release, or independent-evaluator qualification.

## Private evidence boundary

Exact session, platform message, chat, user, and handoff identifiers remain only
in owner-only local runtime stores and this operator-local mode-`0600` receipt:

`$HOME/.local/state/arda/acceptance/phase2-hermes-continuity-20260817.json`

Receipt SHA-256: `038c20834bd1a5e2f18b3da9ad87acd4510f96d04a668e3c932db6c596c91ae0`.

The tracked record retains only bounded digests:

- session-lineage digest: `978ee2737d858c0aa92624b8d702adf89d9c750f4a5b63e9a4f9a7e75b89ae0f`;
- handoff-id digest: `bd7bd3144582c03a4d8172b846f17f529a79dfaf75dbd59f4fe70d61da1c5f89`;
- source-event digest: `a42b1cb36dcd828c77cde2dfa498e99aaef95c35ff30dcc485fb1b4520e2a936`.

No message content, platform credentials, raw transcript, user id, or chat id is
committed. Continuity event, handoff, and Vairë runtime files are ignored by Git
and enforced at mode `0600`; their directories are mode `0700`.

## Genuine phone-to-desktop proof

1. The operator sent the requested continuity commitment from the existing authenticated Hermes Discord conversation on a phone.
2. Hermes retained the existing native session lineage rather than creating a copied summary session.
3. The deployed `arda-operator-bridge` `0.3.0` hook emitted transcript-free continuity metadata while returning normal conversation to Hermes.
4. Arda recorded the event with `shared_room` privacy. The HUD projection reported a fresh active Hermes session while withholding topic, commitment, and Vairë-scope references from the shared source surface.
5. A governed handoff was prepared for `desktop:arda-hud`. The native HUD exposed `Continue here` only for that matching lineage/session.
6. The operator explicitly selected `Continue here`; Arda recorded the handoff as `accepted` with granted consent and a durable acceptance receipt.
7. Vairë recorded the same lineage, current session, commitment reference, desktop surface history, handoff receipt, memory scope, domain policy, and provenance without storing transcript text.
8. Hermes' native handoff watcher completed delivery back to Discord using the same stored session lineage. The phone-facing synthetic turn loaded the prior history and summarized the active continuity work.

## Restart, replay, expiry, and privacy evidence

- Arda was restarted after acceptance. The accepted handoff and Vairë record remained available.
- Hermes Gateway was restarted. Its watchdog became fresh and the native Hermes handoff state remained `completed`.
- Reposting the exact continuity event returned `replayed=true` and the same durable receipt authority rather than duplicating the event.
- A separate short-lived handoff reached `prepared`, expired, and then failed acceptance with HTTP `409 conflict`.
- Shared-room projection returned `private_refs_withheld=true` and empty topic, commitment, and memory-scope arrays.
- The accepted Vairë record contains a commitment reference and `desktop:arda-hud` surface history while serializing no `transcript` field.

## Installed runtime identities

- Arda runtime SHA-256: `fb16b93f2a4ca0dbbdb4d6e6cca09ca41ba6e1b9b0d6574edc56d42662057780`.
- Native HUD SHA-256: `ef657b683d0cbdae158560a6884dec167a2dbb9dec2572013113e684d637bad9`.
- Hermes bridge SHA-256: `b16af57d78212e1cd8b6777d992d19d4d7eac75f1d1139d6527e3a9dfc41bfb0`.
- `arda.service`, `hermes-gateway.service`, `arda-hud.service`, and `arda-session.target` were active after the acceptance restarts.

## Automated gates

- Oromë surface-handoff focused tests: 8/8 passed; full crate tests passed; strict default-feature Clippy passed.
- Engine continuity integration tests: 3/3 passed; strict package Clippy passed with dependencies excluded because unrelated `arda-outpost-protocol` Clippy debt remains outside this phase.
- Hermes bridge: 9/9 tests and `py_compile` passed.
- Vairë: 67 unit tests plus integration suites passed; strict Clippy passed.
- HUD: 143 test files and 583 tests passed; TypeScript, Vite production build, and lint command passed with pre-existing warnings.
- Runtime user-unit verifier passed with the durable canonical `operator:mythos` identity.

## Evidence boundary

This closes the Phase 2 continuity contract for the personal local profile: genuine phone ingress, stable authenticated Hermes lineage, reference-only Arda/Vairë continuity, explicit native HUD acceptance, phone delivery through Hermes semantics, restart survival, replay safety, expiry denial, and shared-surface privacy. A copied summary, synthetic fixture, or transient `hermes chat` worker was not used as acceptance evidence.
