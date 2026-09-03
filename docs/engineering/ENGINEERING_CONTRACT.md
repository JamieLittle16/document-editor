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

## Performance discipline

Optimise measured bottlenecks. Do not introduce speculative complexity merely because a low-level technique sounds faster. Conversely, do not accept avoidable overhead because current hardware hides it.

## Testing layers

- unit tests for pure logic;
- protocol/property tests;
- deterministic mock-engine integration tests;
- real LibreOffice adapter tests;
- DOCX corpus round trips;
- visual/layout differential tests;
- fuzzing of file/protocol boundaries;
- crash/recovery injection;
- performance regression qualification;
- accessibility/platform smoke tests.

## Code principles

- explicit ownership and state machines;
- immutable snapshots where appropriate;
- small interfaces;
- dependency inversion at expensive boundaries;
- no hidden global state in our own architecture;
- strongly typed IDs/revisions instead of interchangeable integers;
- exhaustive errors over stringly-typed failure handling;
- unsafe Rust forbidden by default; isolated exceptions require an ADR and dedicated invariants if ever needed.
