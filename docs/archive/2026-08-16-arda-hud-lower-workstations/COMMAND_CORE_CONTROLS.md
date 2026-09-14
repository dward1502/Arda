---
soterion:
  sigil: "PLATE"
  role: "command_core_control_consolidation"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: ▣ Phase 2 integrates utility access into the Command Core front plate, removes the detached control row, and preserves callback and accessibility authority.

# Phase 2 — Command Core Control Consolidation

## Result

The fixed `boardroom.control.center` surface now contains two explicit control banks:

- command: **GO**, **STOP**, **ROUTE**, **ENTER**;
- utility: **SETTINGS**, **TERMINAL**, **HERMES**.

The detached Settings, Hermes CLI, Hermes Dashboard, and Service Health button zones are no longer members of the boardroom spatial layout. Service Health remains a read-only signal feeding the Command Core instrument and Fleet/Backbone authority; it is no longer a duplicate launcher.

## Preserved authority

No utility action ID or top-level callback was replaced:

| Front-plate control | Existing action ID | Existing callback path | Verification contract |
|---|---|---|---|
| SETTINGS | `open_settings` | `onOpenSettings` | settings workspace load status |
| TERMINAL | `open_hermes_cli` | `onOpenHermesCli` | native terminal window lifecycle |
| HERMES | `open_hermes_dashboard` | `onOpenHermesDashboard` | Hermes dashboard load state |

`dispatchBoardroomCommandCoreControl(...)` is the single typed dispatcher. Its table-driven test proves each utility action invokes exactly one intended callback and no sibling callback.

## Spatial migration

The retired IDs are absent from `BOARDROOM_SPATIAL_ZONES`:

- `boardroom.button.service_health`
- `boardroom.button.settings`
- `boardroom.button.hermes_cli`
- `boardroom.button.hermes`

`normalizeBoardroomZonePositionOverrides(...)` already admits only live zone IDs. Its migration test now proves old detached-row overrides are dropped while `boardroom.control.center` overrides survive.

The new utility controls are children of the fixed Command Core surface. They deliberately do not become separately draggable boardroom zones.

## Accessibility

`BoardroomAccessibilityControls.tsx` remains the semantic alternative. The exact controls remain:

- `Open settings`
- `Open Hermes CLI`
- `Open Hermes dashboard`

Native AT-SPI capture after the change found:

- 36 named semantic controls;
- zero duplicate meaningful control names;
- all three required utility controls;
- the full-screen `1920 × 1080` ARDA HUD frame.

Artifact: [`evidence/phase2-native-atspi-20260816.json`](evidence/phase2-native-atspi-20260816.json).

The desktop driver still exposes no native windows. A direct AT-SPI press probe for Settings timed out, so no end-to-end native window-launch claim is made from that probe. Callback identity, exactly-once dispatch, semantic availability, compile validity, and existing accessibility tests are verified.

## Visual contract

The visual-regression contract now requires:

- `command-core-command-bank`;
- `command-core-utility-bank`;
- `detached-control-row-absent`.

The historical PNG baselines were not regenerated because current screenshot capture is unavailable. They must not be treated as current pixel proof; Phase 10 retains the native visual closeout gate.

## Verification

Executed after implementation:

```text
pnpm --dir apps/arda-hud test
135 test files passed
532 tests passed

pnpm --dir apps/arda-hud build
TypeScript passed
Vite transformed 2614 modules
Build completed

git diff --check
passed
```

The existing Vite mixed static/dynamic import warning for `boardroomSlotSettings.ts` remains non-blocking and was not introduced by this phase.

## Files changed

- `apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`
- `apps/arda-hud/src/scene/boardroom/boardroomPhysicalControls.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomPhysicalControls.test.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.test.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomVisualRegression.ts`
- `apps/arda-hud/src/scene/boardroom/boardroomVisualRegression.test.ts`

The user-owned queue projections remain untouched.
