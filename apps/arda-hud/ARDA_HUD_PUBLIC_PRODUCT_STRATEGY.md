---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-06-26"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: draft | reviewed: 2026-06-26

# ARDA HUD Public Product Strategy

## Core Opinion

ARDA HUD plus Annunimas has real potential as a public-facing tool, but the
first public product should not be "the full sovereign agent system." The
stronger first product is:

> A local-first AI command center for developers, creators, and technical
> operators.

Annunimas can remain the backend nervous system. ARDA HUD should become the
visible, usable front door.

## Why This Is Worth Continuing

ARDA HUD is already more than a concept. It has:

- React, TypeScript, Vite, and Tauri desktop foundations.
- Three.js/WebGL boardroom and world scene direction.
- Runtime data contracts against `core/state`.
- Source provenance and freshness metadata.
- Queue, source-map, and runtime state surfaces.
- Boardroom slot and surface layout contracts.
- Packaging, launch, and native validation paths.
- Tests around important frontend libraries.

That means the project has crossed from "idea" into "early product system."
The remaining challenge is focus: turn the architecture into a small number of
closed human workflows that are reliable and easy to explain.

## Public Framing

Use plain language in public:

- Local AI command center.
- Personal AI operating desk.
- Local model router and workflow dashboard.
- Freelance developer cockpit.
- Privacy-first automation surface.
- Creative project operating system.

Keep the Annunimas/ARDA mythology as brand depth, but translate the internals:

| Internal Name | Public Description |
|---------------|--------------------|
| Manwe | AI model router |
| Hermes | Communication bridge |
| Chronos | Calendar and reminder engine |
| HADES | Maintenance and cleanup automation |
| Prometheus | Planning and orchestration |
| Warden | Health and safety monitor |
| Athena | Knowledge ingestion |
| Mnemosyne | Memory and continuity |
| Apollo | Workflow runner |

## The First Killer Workflow

The first workflow should be:

> Plan my day from calendar, inbox, GitHub, and project queue.

This makes the system understandable immediately. A user should open ARDA HUD
and see:

- Today's calendar.
- Emails requiring action.
- GitHub issues, PRs, and review comments.
- Active freelance/client tasks.
- Reminders and deadlines.
- Blocked items.
- Suggested next actions.
- Items needing approval.
- What data is fresh, stale, missing, or derived.

The system should answer:

- What do I need to do today?
- What changed while I was away?
- What needs my approval?
- What is blocked?
- What can AI safely do for me right now?
- What model handled this?
- What data left my machine?

## Product Rule

Do not measure progress by number of agents or services. Measure progress by
closed loops.

A closed loop looks like:

> Email arrives -> system classifies it -> creates or updates a task -> reminds
> the operator -> helps execute -> drafts a response -> archives the result ->
> updates memory.

Five reliable loops are more valuable than twenty half-connected agents.

## Trust Model

Start with read-only and draft-only integrations.

For Gmail, calendar, GitHub, and client workflows:

- Read data.
- Summarize data.
- Propose tasks.
- Draft replies.
- Ask for approval.
- Do not send, delete, commit, invoice, or message externally without explicit
  operator approval.

Trust is a product feature. Users need to see what the system knows, where it
came from, and what it wants to do next.

## ARDA HUD Differentiators

The strongest differentiator is not "agents." It is inspectable autonomy.

ARDA HUD can stand out by showing:

- Data source provenance.
- Freshness status.
- Safe refresh commands.
- Runtime health.
- Model/provider route metadata.
- Local vs cloud execution.
- Task queue state.
- Approval gates.
- Agent activity and reasoning summaries.

Most AI products hide these details. ARDA HUD can make them legible.

## Build-In-Public Angle

The teaching angle is strong:

> I am building a local-first AI command center in public and showing how to set
> up local models, routing, dashboards, automations, and personal workflows
> without depending entirely on cloud AI.

Potential content series:

- Building a local AI command center from scratch.
- Setting up local models and model routing.
- Building a Tauri AI dashboard.
- Connecting Gmail and calendar safely.
- Showing provenance and stale data in AI systems.
- Turning emails into tasks.
- Managing freelance developer workload with AI.
- Local-first vs cloud AI tradeoffs.
- Monitoring and recovering agent systems.
- Building creative project workflows with AI.

## Monetization Path

Best path:

1. Free build-in-public content.
2. Paid local AI setup guide or course.
3. Templates and config packs.
4. Cohort: "Build your own local AI command center."
5. Consulting for developers, creators, and small businesses.
6. Later: polished ARDA HUD distribution or paid pro workflows.

The teaching journey can become profitable before the full software is ready
for broad public installation.

## Immediate Product Gaps

Before public positioning, clean up:

- Path drift in docs: `apps/arda-hud` vs `/var/home/mythos/Eregion/Arda-HUD`.
- Demo mode with sample `core/state` fixtures.
- Plain-language quickstart.
- A single-screen daily command center.
- Gmail/calendar/GitHub connector plan.
- Approval-first action model.
- Public-friendly screenshots or video demo.
- Clear local-only vs cloud-routed data indicators.

## Recommended Next Build Target

Build the "Daily Command Center" inside ARDA HUD.

Minimum useful version:

- Reads `core/state/queue_summary.json`.
- Reads Manwe/provider status from `core/state/charon_router.json`.
- Shows today's priorities.
- Shows stale/missing/fresh source status.
- Has an inbox/calendar placeholder adapter contract.
- Has an approval queue.
- Has a "plan my day" action that produces a draft plan, not automatic
  external actions.

Once this works, record and publish the build process.

## Final Take

Yes, this should be continued. The backend is broad and still partly unfinished,
but ARDA HUD gives it a product shape.

The near-term mission is to turn Annunimas from an agent orchestration system
into a personal operating system for one working human. Freelance software
development is the right anchor use case because it has clear value:

- fewer missed client follow-ups
- better daily prioritization
- faster status reports
- cleaner task capture
- better project memory
- local/private AI workflows

If ARDA HUD can make those workflows visible, trustworthy, and repeatable, it
can become both a useful product and a strong teaching platform.
