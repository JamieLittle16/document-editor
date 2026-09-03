# Engineering Contract

Status: **Normative**

## Quality rules

1. No synchronous document-engine call on the UI thread.
2. No LibreOffice implementation type above the engine adapter.
3. No second authoritative document model during the bootstrap phase.
4. Every asynchronous state-derived result carries a source revision.
5. Every cache and queue has a documented bound/backpressure policy.
6. Every parser/worker has resource limits.
7. Every persistent mutation path participates in recovery semantics.
8. Saving is atomic from the user's perspective.
9. User-visible format support requires compatibility tests, not anecdotes.
10. A feature is incomplete until error paths and cancellation are specified.
11. Experimental engines never receive unique authority over irreplaceable user content without a fidelity gate.
12. Dependencies require licence, security, maintenance and replacement analysis.
13. Upstream LibreOffice modifications require explicit justification and must remain a small patch stack.
14. Plugins and AI use typed product APIs rather than internal engine access.
15. Architecture is documented in the same change that materially alters it.
16. New internal crate dependency edges must satisfy the executable architecture policy; do not bypass the guard to make a build pass.
17. Major product features declare composition/dependency boundaries rather than reaching into unrelated feature implementations.
18. Kernel correctness invariants are not made optional merely to satisfy an "everything is a plugin" aesthetic.
19. A change that alters a contract or invariant changes its tests and documentation in the same PR.

## Performance discipline

Optimise measured bottlenecks. Do not introduce speculative complexity merely because a low-level technique sounds faster. Conversely, do not accept avoidable overhead because current hardware hides it.

Abstraction is not permission to tax hot paths. Bundled feature modules may use strongly typed/static Rust integration while respecting the same logical product contracts used by replaceable providers.

## Testing layers

The full normative strategy is [`TESTING_STRATEGY.md`](TESTING_STRATEGY.md).

Required layers include:

- unit tests for pure logic;
- protocol/property tests;
- deterministic mock-engine integration tests;
- executable architecture guards;
- real LibreOffice adapter tests;
- DOCX corpus round trips;
- visual/layout differential tests;
- fuzzing of file/protocol/plugin boundaries;
- crash/recovery injection;
- performance regression qualification;
- accessibility/platform smoke tests.

## Code principles

- explicit ownership and state machines;
- immutable snapshots where appropriate;
- small interfaces;
- dependency inversion at expensive boundaries;
- no hidden global state in our own architecture;
- no untyped global service locator;
- strongly typed IDs/revisions instead of interchangeable integers;
- exhaustive errors over stringly-typed failure handling;
- deterministic resolution/order where configuration affects composition;
- unsafe Rust forbidden by default; isolated exceptions require an ADR and dedicated invariants if ever needed.
