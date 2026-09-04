# ADR-0006: Minimal Kernel with Modular Bundled Features

Status: **Accepted**

Date: 2026-09-03

## Context

The product is intended to become substantially more customisable than traditional office suites while remaining maintainable for years. We also want first-party functionality to be easy to disable, replace or extend, including capabilities we have not yet anticipated.

A conventional architecture where every feature directly depends on `app-core`, UI globals and engine internals would make this progressively harder. The opposite extreme—making literally everything a dynamically loaded plugin—would make correctness-critical document, recovery and security invariants optional and introduce unnecessary runtime cost.

## Decision

Adopt a **minimal non-swappable kernel surrounded by modular feature implementations**.

The kernel owns only invariants and authority that must remain coherent:

- document/session authority and revision ordering;
- command/transaction admission and validation;
- persistent journal/recovery guarantees;
- engine protocol/process lifecycle;
- capability/security enforcement;
- feature graph resolution/lifecycle supervision;
- mandatory resource/safety policy;
- minimal application lifecycle.

Product behaviour outside those invariants should be implemented as bundled feature modules wherever practical. Bundled features use the same product-level composition concepts intended for external extensions: stable feature IDs, declared dependencies/conflicts, additive contribution points and replaceable service/provider slots.

Bundled features are **not required** to pay a WASM, IPC or serialization boundary. Sharing a product contract does not require sharing a runtime mechanism or trust model.

External extensions will later run behind a capability-based sandbox and may implement compatible product/service contracts without receiving direct engine implementation access.

Feature graph resolution must complete successfully before feature activation begins. Resolution is deterministic and rejects missing dependencies, explicitly disabled required dependencies, conflicts, missing/ambiguous service providers and cycles.

## Consequences

### Positive

- First-party features are discouraged from forming hidden dependency webs.
- Product profiles can disable unwanted functionality.
- Selected capabilities can be replaced without modifying consumers.
- Future features have established extension points rather than requiring application surgery.
- External plugins can reuse product concepts without defining the product architecture.
- Performance-sensitive bundled features can remain statically linked and strongly typed.
- Invalid feature configuration cannot leave the application half-activated.

### Costs

- Features require manifests and explicit contracts even when direct calls would initially be faster to write.
- Service boundaries must be designed carefully to avoid becoming generic service locators.
- The kernel/feature boundary must be reviewed when new infrastructure is introduced.
- We must maintain deterministic lifecycle and compatibility tests.

## Rejected alternatives

### Everything is application code

Rejected because it encourages direct dependencies and makes disabling/replacing features expensive.

### Everything is a runtime plugin

Rejected because document authority, transactions, recovery, security and engine lifecycle are invariants rather than optional product features; dynamic boundaries would also impose needless cost on trusted hot paths.

### Global service locator

Rejected because dependencies become runtime-discovered and invisible to architecture review/testing.

### Registration-order provider selection

Rejected because "last plugin wins" is nondeterministic from the user's perspective and fragile under installation/update order.

## Guardrail

A feature may depend on another feature or stable product service, but it may not reach into another feature's private implementation to avoid declaring that dependency.

If a new capability appears to require bypassing this rule, write an ADR before adding the bypass.
