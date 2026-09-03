# Features and Extensions

Status: **Normative R0A architecture**

Related: [ARCHITECTURE.md](ARCHITECTURE.md), [AI_AND_PLUGINS.md](AI_AND_PLUGINS.md), [UI_AND_COMMANDS.md](UI_AND_COMMANDS.md), [ADR-0006](../decisions/ADR-0006-modular-feature-kernel.md)

## 1. Problem owned by this subsystem

The feature system lets the product remain highly customisable without turning optional product behaviour into architectural coupling.

It owns:

- stable feature identities;
- feature enable/disable policy;
- declared feature dependencies and conflicts;
- replaceable service/provider slots;
- deterministic activation ordering;
- contribution registration and lifecycle policy;
- the common composition model used by first-party bundled features and, where safe, third-party extensions.

The intent is that major product features can be removed, replaced or extended without surgery on unrelated subsystems.

Examples include spelling providers, grammar providers, command palettes, history UI, diagnostics, AI integrations, citation tooling, export implementations, sidebars, themes, templates and future workflow features we have not designed yet.

## 2. What this subsystem does not own

It does **not** make every part of the application optional.

The minimal kernel owns correctness and authority invariants that must exist for the application to have coherent semantics. The following are kernel responsibilities, not replaceable plugins:

1. document/session authority and revision ordering;
2. transaction validation and command admission;
3. durable journal/recovery invariants;
4. engine process lifecycle and protocol isolation;
5. capability/permission enforcement;
6. feature catalogue resolution and lifecycle supervision;
7. resource-budget enforcement required for process safety;
8. the minimal application lifecycle needed to start, recover and shut down safely.

A provider may implement behaviour *behind* a kernel-controlled slot, but it may not replace the invariant itself. For example, cloud storage may be a provider; atomic-save/recovery guarantees remain kernel policy.

The distinction is:

```text
kernel = invariants and authority
feature = product behaviour
provider = replaceable implementation behind a kernel/product contract
adapter = integration with an external implementation or platform
```

## 3. Architectural shape

```text
                        Product profile / user configuration
                                      |
                                      v
+--------------------------------------------------------------------+
|                         Feature Resolver                           |
| IDs | dependencies | conflicts | service/provider selection        |
+----------------------------------+---------------------------------+
                                   |
                         validated activation plan
                                   |
                                   v
+--------------------------------------------------------------------+
|                         Feature Host                               |
| lifecycle | contributions | capability grants | failure isolation  |
+------------------------+---------------------+---------------------+
                         |                     |
              bundled feature modules     external extensions
                  (in process where          (sandboxed; likely
                   appropriate)               WASM later)
                         |                     |
                         +----------+----------+
                                    |
                          typed product APIs only
                                    |
                                    v
+--------------------------------------------------------------------+
|                         Minimal Kernel                             |
| commands | transactions | sessions | history journal | engine      |
| lifecycle | capability enforcement | recovery | resource policy    |
+--------------------------------------------------------------------+
```

Bundled and external features share **composition concepts**, not necessarily ABI, packaging or trust.

## 4. Why bundled features use extension contracts

First-party code is where accidental coupling usually begins. Therefore, whenever the performance and correctness cost is negligible, bundled features should contribute through the same product-level extension points that an external feature could conceptually use.

Examples:

```text
spellcheck UI  -> requires service: language.spellcheck
basic checker  -> provides service: language.spellcheck
advanced one   -> provides service: language.spellcheck

history panel  -> contributes panel + commands
command palette -> consumes command catalogue
citation tool  -> contributes commands + diagnostics + sidebar
```

This yields three benefits:

- custom product profiles can disable unwanted features;
- alternative implementations can be selected without modifying consumers;
- architecture tests can detect undeclared coupling between features.

This is **not** a requirement to put first-party modules behind serialization or WASM. Bundled modules may use zero-cost/static Rust integration while respecting the same logical contracts.

## 5. Composition modes

We deliberately support different shapes instead of forcing every extension point into one mechanism.

### 5.1 Additive contributions

Multiple features may add independent items:

- commands;
- menus/command-surface metadata;
- panels;
- diagnostics;
- document inspectors;
- templates;
- import/export formats;
- settings pages.

Additive contributions do not compete for ownership.

### 5.2 Single-provider service slots

Some behaviours need one selected provider per product profile or document/session:

- `language.spellcheck`;
- `language.grammar`;
- future `document.export.pdf` implementation selection;
- optional AI provider routing;
- future storage/sync provider contracts.

Features declare `provides(service)` and `requires(service)`. If a required service has multiple active providers, configuration must choose one rather than relying on registration order.

There must be no "last plugin wins" behaviour.

### 5.3 Pipelines

Ordered/chained provider pipelines may eventually be useful for diagnostics, import normalization or automation. They are **not** part of R0A. We will add them only with an ADR and explicit ordering/error semantics; a list of callbacks is not an architecture.

## 6. Resolution before activation

The runtime must build a complete activation plan before starting any feature.

Resolution validates:

1. all explicitly configured feature IDs exist;
2. hard dependencies exist;
3. explicitly disabled dependencies are not silently re-enabled;
4. active conflicts are rejected;
5. required service slots have a provider;
6. ambiguous service slots have an explicit preference;
7. preferred providers actually provide the requested service;
8. dependency/provider ordering is acyclic;
9. activation order is deterministic.

If resolution fails, **zero features activate**.

This avoids partial startup and makes configuration failures reproducible.

## 7. Disable semantics

A feature may be:

- default-enabled by the product profile;
- explicitly enabled;
- explicitly disabled;
- implicitly enabled because an enabled feature depends on it;
- implicitly enabled because it is selected as a service provider.

Explicit disable is strong. If an active feature requires an explicitly disabled dependency or selected provider, resolution fails with a useful explanation instead of overriding the user's choice.

Kernel components cannot be disabled through this system.

## 8. Replace/swap semantics

Consumers depend on a stable `ServiceId`, never a concrete implementation.

For example:

```text
language.proofreading-ui
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

A profile can select the advanced provider without changing proofreading UI code. A future third-party provider can occupy the same slot if it satisfies the capability/security/API contract.

Provider preference is configuration, not source-level dependency injection scattered through UI code.

## 9. Public interfaces

R0A introduces two deliberately dependency-light crates:

### `extension-api`

Owns stable product-level values:

- `FeatureId`;
- `ServiceId`;
- `FeatureOrigin`;
- `FeatureManifest`.

It must remain free of LibreOffice, Qt/Slint, IPC transport and plugin-runtime implementation types.

### `extension-runtime`

Owns deterministic composition:

- `FeatureRegistry`;
- `FeatureSelection`;
- `ResolvedFeatures`;
- registration and resolution errors.

It resolves an activation plan; it does not yet load WASM, instantiate UI panels or grant OS permissions.

A later feature-host layer will consume `ResolvedFeatures`.

## 10. Dependency direction

```text
feature implementations ---> extension-api
feature host ----------+----> extension-runtime ---> extension-api
app-core --------------+

extension-api -X-> app-core
extension-api -X-> UI framework
extension-api -X-> LibreOffice
extension-api -X-> document engine adapter
```

The extension contracts are intentionally lower-level than feature implementations.

No feature may directly reach into another feature's private crate. Cross-feature interaction happens through declared dependencies plus public product/service contracts.

## 11. Capability and security model

Composition and security are separate decisions.

A manifest saying a feature requires a service does not grant security capability. External extensions also need explicit permissions described in [AI_AND_PLUGINS.md](AI_AND_PLUGINS.md).

Bundled features may be trusted to run in process, but should still declare sensitive capabilities where doing so improves auditability. External extensions do not become trusted merely because they use the same feature manifest model.

No plugin receives raw LibreOffice/UNO/VCL/SFX/Writer access.

## 12. Concurrency and lifecycle

R0A resolves metadata only and is synchronous/deterministic.

Future feature activation must obey:

- lifecycle is supervised by the feature host;
- activation happens only after successful graph resolution;
- provider dependencies activate before consumers;
- feature background work is revision-tagged when document-derived;
- feature work cannot block the UI thread;
- external plugin failure must not corrupt document authority;
- deactivation must release registrations/resources deterministically;
- document mutation still passes through the command/transaction path.

Features never obtain an unsupervised mutable reference to authoritative document state.

## 13. Resource bounds

The manifest/resolver graph is bounded by the number of installed/compiled features and uses deterministic ordered collections.

Before external plugins ship, the feature host must define hard bounds for:

- number and size of installed manifests;
- commands/panels/diagnostics contributed per extension;
- plugin memory;
- CPU/fuel/time budgets;
- outbound message size/rate;
- document snapshot/range access;
- network origins and request quotas.

Unbounded extension resource use is a release blocker.

## 14. Failure and recovery

Resolution errors are configuration errors and occur before activation.

Runtime feature failure policy will distinguish:

- optional feature failure: disable/quarantine the feature and surface diagnostics;
- selected provider failure: fail the dependent capability cleanly and, only where explicitly safe, offer a configured fallback;
- kernel failure: recover/restart at the owning process/session boundary rather than pretending it is a plugin failure.

Fallback must never silently change document semantics or fidelity.

## 15. Testing

R0A tests cover:

- stable identifier validation;
- duplicate registration rejection;
- default enablement;
- dependency closure;
- explicit dependency disable failure;
- conflict rejection;
- missing service rejection;
- ambiguous provider rejection;
- explicit provider selection;
- implicit activation of a selected provider;
- provider-before-consumer ordering;
- dependency cycle detection.

Future required test layers:

1. property tests over generated feature graphs;
2. architecture tests that reject forbidden crate dependencies;
3. lifecycle tests proving failed activation cannot leak registrations;
4. sandbox capability tests;
5. fuzzing of external manifests and IPC/plugin messages;
6. performance tests with large extension catalogues;
7. compatibility tests proving disabling optional features does not alter core document fidelity.

## 16. Performance qualification

Feature abstraction must not put dynamic dispatch, IPC or serialization into hot document/layout paths merely for ideological purity.

Bundled features may be statically linked and use typed Rust calls. External extensions may pay an isolation boundary. The product contract is shared; the execution mechanism may differ.

Before adding a hot-path extension point, benchmark:

- baseline operation latency;
- dispatch overhead;
- allocation count;
- cache impact;
- worst-case fan-out.

If an extension hook measurably harms typing/rendering latency, redesign the hook rather than accepting permanent tax.

## 17. Technical debt and deliberately deferred work

R0A does **not** yet define:

- runtime feature instances/lifecycle traits;
- serialized manifest format;
- plugin package format/signing;
- WASM runtime choice;
- ABI stability guarantees;
- UI contribution schema;
- settings schema;
- chained provider pipelines;
- plugin marketplace/update system;
- per-document versus global feature activation.

These are intentionally deferred until the product contracts and relevant technology spikes are clearer. They must not be invented ad hoc by the first feature needing them.

## 18. Architecture acceptance test

A new major feature is architecturally healthy when we can answer yes to all of these:

1. Can it be disabled without editing unrelated feature code?
2. Can an alternative implementation be supplied through a documented contract where replacement makes sense?
3. Are its dependencies declared rather than discovered through globals?
4. Does it mutate documents only through transactions/commands?
5. Does it avoid engine implementation types?
6. Are failure, resource and concurrency semantics explicit?
7. Could an engineer find the relevant contracts and tests without reverse-engineering the implementation?

If not, the feature is not finished.
