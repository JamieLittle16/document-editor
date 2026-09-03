# Features and Extensions

Status: **Normative R0A architecture**

Related: [ARCHITECTURE.md](ARCHITECTURE.md), [FEATURE_HOST.md](FEATURE_HOST.md), [AI_AND_PLUGINS.md](AI_AND_PLUGINS.md), [UI_AND_COMMANDS.md](UI_AND_COMMANDS.md), [ADR-0006](../decisions/ADR-0006-modular-feature-kernel.md)

## 1. Purpose

The feature system keeps the product highly customisable without turning optional product behaviour into architectural coupling.

It owns:

- stable feature identities;
- enable/disable policy;
- declared dependencies and conflicts;
- replaceable service/provider slots;
- deterministic activation ordering;
- supervised bundled-feature lifecycle;
- the common composition model used by first-party modules and, where safe, future third-party extensions.

Major product behaviour should be removable, replaceable or extensible without surgery on unrelated subsystems. Likely modular features include spelling, grammar, command palette, history timeline UI, version comparison UI, diagnostics, AI integrations, citation tooling, export implementations, sidebars, themes, templates and future workflow features.

## 2. The kernel is intentionally small and non-swappable

Customisability does **not** mean every invariant becomes a plugin.

The minimal kernel owns:

1. document/session authority and revision ordering;
2. transaction validation and command admission;
3. durable journal/recovery invariants;
4. engine process lifecycle and protocol isolation;
5. capability/permission enforcement;
6. feature catalogue resolution and lifecycle supervision;
7. process-safety resource policy;
8. minimal application startup/recovery/shutdown.

A provider may implement behaviour behind a kernel-controlled slot, but it may not replace the invariant itself. Cloud storage may be replaceable; atomic-save/recovery guarantees remain kernel policy.

```text
kernel   = invariants and authority
feature  = product behaviour
provider = replaceable implementation behind a stable service contract
adapter  = integration with an external implementation/platform
```

History is a useful example: revision/journal integrity is kernel-level, while the timeline, compare viewer, named versions, selective restore and alternative diff visualisations are modular product features.

## 3. Architectural shape

```text
                     Product profile / user configuration
                                   |
                                   v
+-------------------------------------------------------------------+
|                       Feature Resolver                            |
| IDs | dependencies | conflicts | service/provider selection       |
+---------------------------------+---------------------------------+
                                  |
                        validated activation plan
                                  |
                                  v
+-------------------------------------------------------------------+
|                    Trusted Bundled Feature Host                   |
| lifecycle | rollback | reverse shutdown | fault tracking          |
+---------------------------------+---------------------------------+
                                  |
                         bundled feature modules
                       (in process where safe)
                                  |
                           typed product APIs
                                  |
                                  v
+-------------------------------------------------------------------+
|                         Minimal Kernel                            |
| commands | transactions | sessions | journal | engine lifecycle   |
| capability enforcement | recovery | resource policy               |
+-------------------------------------------------------------------+

Future external extensions:

external package -> sandbox host -> capability-filtered product APIs
```

Bundled and external features share **composition concepts**, not ABI, loading mechanism or trust.

## 4. Why first-party features use extension contracts

First-party code is where accidental coupling usually starts. Whenever performance/correctness cost is negligible, bundled features should contribute through product-level extension points instead of directly reaching into unrelated modules.

Examples:

```text
proofreading UI -> requires service: language.spellcheck
basic checker   -> provides service: language.spellcheck
advanced checker-> provides service: language.spellcheck

history panel   -> contributes panel + commands
command palette -> consumes command catalogue
citation tool   -> contributes commands + diagnostics + sidebar
```

This lets profiles disable unwanted behaviour and alternative implementations replace providers without changing consumers.

It does **not** require WASM/serialization for first-party code. Bundled features may remain statically linked Rust and use direct typed calls on hot paths while respecting the same logical contracts.

## 5. Composition modes

### 5.1 Additive contributions

Multiple features may eventually add independent commands, panels, diagnostics, inspectors, templates, import/export formats or settings pages.

The contribution schemas are deliberately deferred until the first real feature needs them. We will not create a generic callback/event bus merely to claim extensibility.

### 5.2 Single-provider service slots

Some behaviours need one selected provider:

- `language.spellcheck`;
- `language.grammar`;
- future `document.export.pdf`;
- optional AI routing;
- future storage/sync contracts.

Features declare `provides(service)` and `requires(service)`. If a required service has multiple active providers, configuration must choose one. There is no registration-order or "last plugin wins" behaviour.

### 5.3 Pipelines

Ordered provider pipelines may later be useful for diagnostics/import normalisation/automation. They are not part of R0A and require a separate ADR with explicit ordering, cancellation and failure semantics.

## 6. Resolution before activation

`extension-runtime` builds the complete plan before `feature-host` starts anything.

Resolution validates:

1. configured IDs exist;
2. hard dependencies exist;
3. explicitly disabled dependencies are not silently re-enabled;
4. conflicts are rejected;
5. required services have providers;
6. ambiguous providers require an explicit preference;
7. preferred providers actually provide the service;
8. dependency/provider ordering is acyclic;
9. activation order is deterministic.

If resolution fails, **zero features activate**.

## 7. Disable and replacement semantics

A feature may be default-enabled, explicitly enabled, explicitly disabled, implicitly enabled by dependency, or implicitly enabled because it is selected as a service provider.

Explicit disable is strong. If another active feature requires an explicitly disabled dependency/provider, resolution fails with a diagnostic rather than overriding the user's choice.

Kernel components cannot be disabled through this system.

Consumers depend on stable `ServiceId`s rather than implementation types:

```text
language.proofreading
        |
        | requires
        v
language.spellcheck
        ^
        |
  +-----+-------------------+
  |                         |
basic provider        advanced provider
```

A user/profile may swap providers without changing proofreading code.

## 8. Implemented R0A crates

### `extension-api`

Owns dependency-light product values:

- `FeatureId`;
- `ServiceId`;
- `FeatureOrigin`;
- `FeatureManifest`.

It must stay free of LibreOffice, UI framework, IPC transport and plugin-runtime types.

### `extension-runtime`

Owns deterministic composition:

- `FeatureRegistry`;
- `FeatureSelection`;
- `ResolvedFeatures`;
- typed registration/resolution errors.

It resolves plans but does not execute feature code.

### `feature-host`

Owns trusted bundled lifecycle:

- manifest/implementation binding;
- external-origin rejection from the in-process path;
- resolved activation;
- read-only provider context;
- failing-feature cleanup;
- reverse rollback/shutdown;
- fault tracking and cleanup retry.

See [FEATURE_HOST.md](FEATURE_HOST.md).

## 9. Dependency direction

```text
feature implementation ---> extension-api + typed product APIs
feature-host ----------+---> extension-runtime ---> extension-api
app-core --------------+     (later composition root integration)

extension-api -X-> app-core
extension-api -X-> UI framework
extension-api -X-> LibreOffice
extension-api -X-> engine adapter
feature-host   -X-> LibreOffice
feature-host   -X-> external plugin runtime
```

No feature may directly reach into another feature's private crate. Cross-feature interaction uses declared dependencies plus stable product/service contracts.

## 10. Trust and capabilities

Composition and security are separate.

A manifest declaring a service dependency grants no OS/security capability. Bundled features are trusted code. External extensions will require explicit permissions such as document read scopes, document modification, UI contributions and network origins.

`feature-host` rejects `FeatureOrigin::External`; external code must later cross the sandbox host. No plugin receives raw LibreOffice/UNO/VCL/SFX/Writer access.

## 11. Lifecycle and failure semantics

Bundled lifecycle is supervised by `feature-host`.

- resolve before activation;
- providers/dependencies activate before consumers;
- failure during activation triggers cleanup of the failing feature;
- already-active features then roll back in reverse order;
- deactivation failures do not stop cleanup attempts for other features;
- unresolved cleanup produces `Faulted`, never fake success;
- repeated stop attempts retry still-live features;
- catalogue mutation is forbidden while running/faulted;
- document mutation still goes through commands/transactions.

Features never obtain mutable authoritative document state.

External plugin failure policy will be stricter and process/sandbox isolated.

## 12. Resource and performance rules

R0A manifest/resolution/host structures are bounded by the registered feature catalogue and contain no unbounded queues/task pools.

Before external plugins ship, hard limits are required for manifest/package size, commands/panels/diagnostics contributed, memory, CPU/fuel/time, message size/rate, document snapshot access and network origins/quotas.

Feature abstraction must not impose dynamic dispatch, IPC or serialization inside hot editing/layout/rendering loops merely for ideological purity. The host is control-plane lifecycle infrastructure, not a universal runtime service locator.

Every hot-path extension point requires benchmark evidence for latency, allocations, cache impact and fan-out.

## 13. Testing

Current tests cover:

- stable identifier validation;
- duplicate registration rejection;
- default enablement and dependency closure;
- explicit dependency-disable failure;
- conflicts;
- missing/ambiguous services;
- selected provider activation;
- provider-before-consumer ordering;
- cycles;
- bundled host activation and reverse shutdown;
- zero activation after resolution failure;
- cleanup of a failing activator;
- rollback of prior features;
- fault state after cleanup failure;
- cleanup retry semantics;
- provider visibility in activation context;
- external-origin rejection from in-process host;
- manifest/implementation ID agreement;
- CI architecture dependency guard.

Required later: generated feature-graph property tests, sandbox capability tests, external manifest fuzzing, large-catalogue performance tests and compatibility tests proving optional features do not change document fidelity when disabled.

## 14. Deferred work

R0A intentionally does **not** define:

- external sandbox host/runtime;
- WASM runtime choice;
- serialized/package manifest format;
- package signing/update/marketplace;
- ABI stability guarantees;
- UI/command/settings contribution schemas;
- chained provider pipelines;
- hot reconfiguration;
- per-document versus global feature activation;
- structured background-task supervision for feature-owned async work.

These must be introduced through focused contracts/ADRs, not invented by the first feature that needs them.

## 15. Acceptance test for a major feature

A new major feature is architecturally healthy when:

1. it can be disabled without editing unrelated code;
2. an alternative implementation can occupy a documented service slot where replacement makes sense;
3. dependencies are declared rather than discovered through globals;
4. document mutations use transactions/commands;
5. it never sees engine implementation types;
6. failure/resource/concurrency semantics are explicit;
7. architecture and tests explain how it composes;
8. a new engineer or coding agent can discover the intended boundary without reverse-engineering implementation details.
