# arda-outpost-scout ownership

## Owned here

- Bounded Cargo/app/outpost survey and crate-status observations.
- Fixed research source-policy ID, query/expiry/result limits, and source URL
  validation.
- One operator-configured SearXNG client boundary and research report shape.
- Advisory conversion into `arda.outpost.observation.v1`.
- Vairë ingestion/recall bridge and return of canonical memory receipt IDs.
- Warden scout HTTP routes and topic-runner CLI.

## Owned elsewhere

- Shared observation schema and non-execution authority classes:
  `arda-outpost-protocol`.
- Durable memory implementation and IDs: `arda-vaire`.
- Root-daemon proxy configuration and timeout: `arda-engine` harness.
- SearXNG engine/domain configuration and network policy: Warden operations.
- Athena scout request/finding ledgers and scout runtime projection: their
  presently missing producer owner, to be resolved by the active Pi5 plan.
- Read-only scout presentation: ARDA HUD.
- Council deliberation, task promotion, queue mutation, approval, dispatch, and
  execution: governance/runtime owners, never scout.

## Authority boundary

Scout evidence may inform review only. `ResearchReport` and `SurveyReport`
convert to observations with `AuthorityClass::Advisory`; the protocol contract
explicitly denies execution for that class. Receipt creation proves persistence,
not approval or promotion.

Adding a model/tool selector, queue writer, approval field, execution-capable
authority, a new policy identifier, or a second network provider is a governed
cross-consumer change requiring focused tests and plan reconciliation.