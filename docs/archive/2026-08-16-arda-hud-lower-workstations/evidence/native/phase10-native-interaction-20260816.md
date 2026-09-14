# Phase 10 native interaction evidence

Recorded: 2026-08-16

## Candidate and launch profile

- Binary: `/var/home/mythos/.cache/arda-build/target/release/arda_hud`
- Window: `ARDA HUD`, X11 id `35651587`, `1920 × 1080`
- Launch profile: `GDK_BACKEND=x11`, `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `__NV_DISABLE_EXPLICIT_SYNC=1`
- Accessibility capture: native WebKit document `arda-hud`; 81 boardroom controls before a focused workstation opened.

The default Wayland launch attempt remains historical blocked evidence. This compatibility launch exposed the current optimized application as a real native X11 window with accessibility controls and direct pixel capture.

## Pointer and owner routing

Native semantic controls opened the current optimized focused owners:

- Governance and Guardhouse: boardroom control count changed from 81 to 152 and the focused `Governance And Guardhouse Workstation` rendered.
- Fleet and Backbone: the focused AULË topology owner rendered rather than the former generic composition.
- Routing and Communications: the focused CHARON lane/provider owner rendered.
- Human, Business, and Personal: the focused private continuity owner rendered with all three horizons.
- A direct XTEST pointer action on the physical command/control bank changed the boardroom state to 115 controls and exposed the native `READY` status `Opening Approval Queue.`

The Fleet/Routing/Continuity traversal exposed a composition-selection defect during acceptance. The fix now classifies focused views from semantic slot ownership through `getFocusedWorkstationKind(...)`; the regression test and rebuilt native screenshots are part of this closeout.

## Keyboard and focus restoration

1. The native `Open Governance And Guardhouse Workstation` accessibility button opened the focused Governance surface.
2. Background Escape delivery correctly refused because the surface had no focus-free keyboard backend.
3. The same Escape action was delivered through the foreground X11 route.
4. Fresh native capture returned from 152 controls to the 81-control boardroom state.
5. The visible focus ring returned to `Open Governance And Guardhouse Workstation`.

This verifies native Escape closure and focus restoration for the exercised focused-owner path. Broader focus-order and reduced-motion state permutations remain contract-tested rather than inferred from this single traversal.

## Visual review

The direct 1920 × 1080 captures show:

- Governance as an evidence-first policy/review chamber with posture, source states, review index, and selected evidence detail.
- Fleet as a topology/node-index/selected-node surface with reachability and hardware evidence.
- Routing as a lane-to-provider/model flow surface with budget pressure, selected lane, and provider field.
- Continuity as a private Human/Business/Personal three-horizon surface with missing-path and stale-source cues.
- The boardroom Command Core bank and its physical action-transition state.

No window-level clipping or horizontal overflow is visible. Fleet and Routing continue below the captured viewport through their internal scroll regions; this is expected scrollable content, not content escaping the workstation. The Governance status rail is dense but remains inside its bounded surface. The Command Core action-transition capture opens the Sovereign World workstation while reporting `Opening ARDA Control`; it is evidence of the current routed action, not a claim that Sovereign World is the Command Core owner.

## Performance and motion boundary

The compatibility profile is intentionally static demand rendering, as declared by the native render-context path when `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Scheduled pixel-hash sampling at 60 Hz for 15 seconds observed 901 samples, zero changed idle samples, and zero late samples. Therefore an idle animation FPS cannot truthfully be reported for this profile; there is no continuous idle cadence to measure.

The adopted like-for-like native resource contract is measurable and passed over 60.055 seconds:

- aggregate CPU: 11.64% of one core, limit 40.0%;
- PSS peak: 408.04 MiB, limit 550.0 MiB;
- PSS growth: 14.86 MiB, limit 16.0 MiB;
- RSS peak: 574.84 MiB, observability ceiling 700.0 MiB;
- swap peak: 0 MiB.

Evidence:

- [`phase10-native-frame-timing-20260816.log`](phase10-native-frame-timing-20260816.log)
- [`phase10-native-profile-20260816.json`](phase10-native-profile-20260816.json)
- [`phase10-native-profile-20260816.log`](phase10-native-profile-20260816.log)
