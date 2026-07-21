# Manwë Router Current-State Analysis

## Executive summary

`manwe` is being repositioned from the legacy `annunimas-charon` runtime into the stable local inference gateway for the Arda project.

The clearest architectural intent is:

* expose one dependable OpenAI-compatible endpoint on `127.0.0.1:7171`;
* keep the default runtime small, local, and operationally predictable;
* preserve the more advanced Charon routing and governance machinery behind an `adaptive` feature;
* allow Arda components to integrate with a stable gateway while the adaptive system is repaired and reorganized incrementally.

This is a sound migration strategy. The main issue is that the documentation currently describes two different states of the crate:

1. a thin static gateway that compiles and runs; and
2. a broad adaptive routing platform that is partially implemented, partially stubbed, and inconsistently reported as either compiling or broken.

The next phase should therefore focus less on adding features and more on establishing a single authoritative architecture, build status, and boundary between the stable and adaptive surfaces.

---

## 1. Current architectural identity

The strongest and most consistent definition of `manwe` is:

> A local OpenAI-compatible inference gateway that provides a stable boundary between Arda consumers and one or more upstream model providers.

The stable crate root contains the binary entry point, configuration, provider catalog, transport interfaces, gateway types, routing authority traits, compatibility shims, and legacy Charon bridge models.

The runtime currently binds to `127.0.0.1:7171` and exposes health, model-listing, and chat-completion endpoints.

This stable gateway is not merely another internal service. It is intended to be the frozen local contract used by the rest of Arda.

That makes `manwe` primarily a **boundary component**, not just a routing algorithm crate.

---

## 2. The two-surface architecture

The crate is organized around two conceptual surfaces.

### Stable root

The default surface provides:

* CLI and runtime startup;
* TOML-backed provider configuration;
* provider and model resolution;
* OpenAI-compatible forwarding;
* health and model discovery;
* bridge types and authority traits for future integration.

The static path currently forwards requests to a configured provider, removes the local `provider/model` prefix where necessary, and optionally attaches bearer credentials.

This surface appears intentionally conservative. It gives Arda one endpoint even when governance, economics, adaptive scoring, quotas, and provider coordination are unavailable.

### Adaptive subtree

The `src/adaptive/` tree contains the inherited or reconstructed Charon intelligence layer:

* provider capabilities;
* health probes;
* route policies;
* scoring and selection;
* candidate caching;
* session and route history;
* quotas;
* multi-armed bandit state;
* runtime mutation and persistence;
* observability and metrics;
* provider reconciliation;
* HTTP and IPC administrative transports;
* external driver integrations.

The feature is gated behind `--features adaptive`, and the architecture document identifies this subtree as the location for active routing and governance development.

Conceptually, this separation is one of the strongest parts of the redesign. It prevents the legacy adaptive system from blocking delivery of a usable gateway.

---

## 3. Runtime behavior

The current stable runtime is deliberately thin.

A request to `/v1/chat/completions`:

1. requires a model;
2. resolves a provider using the model prefix or the configured default;
3. transforms the local model identifier when required;
4. forwards the request upstream;
5. optionally adds provider authentication.

The stable runtime does **not** currently provide adaptive rerouting, quota meshing, governed provider selection, or significant request transformation.

The default model naming convention is:

```text
provider/model
```

The default provider configuration appears to target a local Ollama OpenAI-compatible endpoint.

This gives the crate a useful immediate role: a common compatibility layer that can hide provider-specific endpoints from downstream applications.

---

## 4. Position inside Arda

`manwe` is already treated as a shared runtime dependency rather than an isolated experiment.

Current consumers include:

* `arda-engine`, which supervises the process and proxies model discovery;
* `arda-hud`, which reads model and health information;
* `arda-launcher`, which assumes the gateway exists on port `7171`;
* the workspace service registry, which classifies `manwe` as the inference gateway.

Process supervision belongs to `arda-engine`; `manwe` does not daemonize itself.

This is an appropriate separation of concerns:

* **manwe** owns gateway behavior;
* **arda-engine** owns lifecycle and supervision;
* **arda-hud** owns operator visibility;
* **arda-launcher** owns user-facing startup assumptions.

The risk is that port, endpoint, and runtime assumptions may become duplicated across these consumers. Those assumptions should eventually be represented as shared contracts rather than hardcoded independently.

---

## 5. What appears mature

Several architectural decisions already look solid.

### A. Stable boundary before intelligent routing

Making the local gateway work independently of adaptive routing is the right migration sequence. It reduces the blast radius of repairing the legacy Charon system.

### B. Feature-gated reconstruction

Keeping adaptive behavior behind a feature allows the codebase to retain valuable legacy components without claiming that they are production-ready.

### C. Clear supervision ownership

`arda-engine` supervising `manwe` avoids embedding daemon-management behavior into the gateway crate.

### D. Provider-neutral downstream contract

Downstream clients need to understand only the local OpenAI-compatible API, not Ollama, hosted OpenAI-compatible services, Hermes, or future providers.

### E. Separation of transport and decision logic

The presence of distinct transport, routing, provider, configuration, and service-state modules suggests the intended domain boundaries are broadly correct, even though some implementation details remain unfinished.

---

## 6. Major inconsistencies in the documentation

The most important finding is that the documents do not agree on the current build state.

### Adaptive compilation status

`BREAKDOWN.md` states:

* default `cargo check -p manwe` passes;
* adaptive compilation fails;
* there are unresolved Arda dependencies;
* there are parser failures;
* sibling-module visibility problems remain;
* type inference failures remain.

However, `README.md` states that the crate builds with `--features adaptive` after targeted fixes, though warnings and lint noise remain.

These claims cannot both represent the same repository state unless one document was written after the other or the build depends on unrecorded local changes.

Before further design work, the project needs a reproducible verification record containing:

```text
git commit
rustc version
cargo version
enabled features
exact command
result
warnings/errors
```

Without that, the current baseline is ambiguous.

### Static gateway versus adaptive gateway

The README initially describes `manwe` as owning adaptive routing, governance, status, and transport responsibilities.

The status document, by contrast, says the stable runtime intentionally does no adaptive routing or quota meshing.

This is understandable at the implementation level, but the project language should distinguish:

* **crate responsibility**;
* **default compiled behavior**;
* **feature-gated experimental behavior**;
* **future target behavior**.

At present, those four concepts are blended together.

### Configuration source ambiguity

The static documentation describes a local `manwe.toml`, while the adaptive README describes provider configuration under `config/governance/charon.providers.toml` plus runtime state and bootstrapped defaults.
Both may be valid for different modes, but the precedence model is not explicit.

---

## 7. Technical risks

### A. The adaptive subtree is too broad to repair as one unit

The adaptive service contains routing, quotas, bandits, persistence, metrics, events, provider administration, reconciliation, drivers, proxying, and transport endpoints.

That is a large recovery surface. Repairing it as one feature risks repeatedly fixing one subsystem only to expose failures in another.

A staged feature decomposition would be safer, for example:

```text
adaptive-core
adaptive-state
adaptive-policy
adaptive-observability
adaptive-drivers
adaptive-admin
```

These do not necessarily need to become permanent Cargo features, but the recovery effort should be structured in similarly bounded layers.

### B. Cross-module visibility problems indicate unclear ownership

The reported `pub(super)` failures across sibling modules suggest internal APIs grew organically rather than around a deliberate service boundary.

The fix should not simply be changing everything to `pub(crate)`. Shared operations should be moved onto a service façade or explicit domain components.

### C. External Arda dependencies are insufficiently isolated

The adaptive code refers directly to `arda_economics`, `arda_governance`, and `arda_vaire`, and those references are part of the reported compilation failures.

These dependencies should ideally sit behind traits or adapter modules. Core route selection should be testable without constructing the entire Arda governance and economics environment.

### D. Runtime-only provider validation

Credential, bind, and provider errors are discovered at runtime rather than during startup validation.

For a central gateway, configuration should fail early with a structured diagnostic report.

### E. Placeholder behavior is reachable

The routing adapter is described as a shim that currently returns a not-wired error.

Reachable placeholders are dangerous when the type structure makes the feature appear complete. Experimental paths should either be explicitly unavailable or return strongly typed capability-state information.

### F. Lack of integration tests

The default build reports no unit or documentation tests, and the suggested improvement list calls for static forwarding tests covering provider resolution, malformed models, bad upstream responses, and unreachable providers.
For a gateway, these are baseline contract tests rather than optional improvements.

### G. Terminology still carries legacy Charon identity

Types such as `CharonService`, `CharonRemote`, and Charon authority traits remain central.

This may be acceptable during migration, but a decision is needed:

* Is Charon now a subsystem inside Manwë? No should be part of manwe, charon is just a name from annunimas
* Is it a compatibility namespace? ManweService ManweRemote
* Is it the name of the adaptive engine? no
* Is it temporary legacy terminology? yes

Without a naming model, the rebrand will remain incomplete and future developers will struggle to understand which concepts are current.

---

## 8. Recommended conceptual model

A useful architecture model for the project would be:

### Manwë Gateway

The stable local process and API contract.

Responsibilities:

* listen on the local inference endpoint;
* validate and accept OpenAI-compatible requests;
* expose health and model discovery;
* resolve configured providers;
* forward requests;
* normalize errors;
* expose runtime capability status.

### Manwë Router

The routing decision engine.

Responsibilities:

* determine eligible providers;
* apply routing policy;
* score candidates;
* select fallbacks;
* preserve session affinity;
* produce an explainable route decision.

This should preferably be usable as a library independent of Axum.

### Manwë State

The runtime provider and routing state layer.

Responsibilities:

* provider health;
* capabilities;
* quotas;
* route history;
* bandit state;
* candidate caches;
* persisted overlays.

### Manwë Governance Adapters

Connections to the wider Arda systems.

Responsibilities:

* economics;
* governance policy;
* memory or historical context;
* treaties and authorization;
* event publication.

### Manwë Transports

External interfaces:

* OpenAI-compatible HTTP;
* administrative HTTP;
* Unix IPC;
* possibly gRPC if there is a concrete consumer.

This model preserves the current direction but gives each inherited Charon subsystem a clear destination.

---

## 9. Recommended near-term recovery sequence

### Phase 1: Establish the truth

Create one generated or manually maintained verification document showing:

* default build status;
* adaptive build status;
* test status;
* feature matrix;
* known warnings;
* exact repository revision.

Update the README, STATUS, ARCHITECTURE, and BREAKDOWN documents from that same record.

### Phase 2: Lock the stable gateway contract

Add integration tests for:

* `/healthz`;
* `/v1/models`;
* successful forwarding;
* missing model;
* unknown provider;
* default-provider behavior;
* provider-prefix stripping;
* authentication propagation;
* unreachable upstream;
* malformed upstream JSON;
* upstream streaming behavior.

### Phase 3: Define the naming and compatibility policy

Document whether `Charon*` names are:

* retained public concepts;
* private compatibility types;
* deprecated names;
* or candidates for direct renaming.

### Phase 4: Reconstruct adaptive routing from the inside out

Suggested order:

1. adaptive types and error model;
2. provider capabilities and health;
3. immutable route candidates;
4. route policy;
5. deterministic scoring;
6. deterministic selection;
7. fallback behavior;
8. sessions and history;
9. quotas;
10. bandit learning;
11. persisted state;
12. governance/economics adapters;
13. administrative transports;
14. external drivers.

Adaptive routing should initially compile and test using in-memory mock adapters, without requiring the full Arda workspace.

### Phase 5: Introduce explicit capability reporting

The gateway should be able to state something like:

```json
{
  "mode": "static",
  "adaptive_routing": false,
  "governance": false,
  "quota_mesh": false,
  "configured_providers": 1,
  "healthy_providers": 1
}
```

That is clearer than allowing downstream systems to infer capability from build flags or endpoint failures.

---

## 10. Questions the vision document should answer

When comparing this baseline to the next document, the key questions will be:

1. Is Manwë primarily a gateway, a router, or the complete inference control plane?
2. Which capabilities must exist in the default build?
3. Is adaptive routing optional, experimental, or ultimately mandatory?
4. Where does routing authority live: Manwë itself or Arda governance services?
5. What is the intended relationship between Manwë and Charon terminology?
6. Is provider state authoritative in configuration, runtime persistence, governance, or reconciliation across all three?
7. Is `provider/model` the permanent model identity format?
8. What guarantees must the local endpoint make to consumers?
9. Which failures should trigger fallback and which should be returned immediately?
10. How explainable must route decisions be?
11. Are quotas scoped by user, agent, process, model, provider, or treaty?
12. Is Manwë expected to support a single local machine or eventually coordinate across a fleet?
13. Is gRPC part of the target architecture or merely inherited code?
14. Which state must survive process restarts?
15. What are the security and authorization boundaries for admin endpoints and IPC?

---

## Overall assessment

The project has a strong architectural foundation but an unstable definition of its current implementation state.

The best decision made so far is the frozen local gateway boundary. It allows the Arda ecosystem to depend on Manwë before the inherited Charon adaptive stack is fully recovered.

The greatest risk is allowing the feature-rich adaptive module tree to define the perceived architecture before its contracts, dependencies, and compilation state are stabilized.

The immediate objective should be:

> Make Manwë an unquestionably reliable local gateway first, then rebuild adaptive intelligence behind narrow, testable interfaces.

The vision document can then be evaluated against a clean distinction between:

* what Manwë reliably does now;
* what inherited Charon code suggests it may do;
* and what the Arda architecture actually requires it to become.
