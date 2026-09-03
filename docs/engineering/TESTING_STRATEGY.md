# Testing Strategy

Status: **Normative**

Testing is part of architecture. The objective is not merely high line coverage; it is to make the product's invariants, interoperability claims, recovery guarantees and performance expectations continuously falsifiable.

## 1. Testing principles

1. **Test contracts at the lowest useful layer.** Pure rules belong in fast unit/property tests; do not require LibreOffice to test a revision graph.
2. **Test expensive boundaries with real implementations.** Mocks never prove DOCX fidelity, process isolation or LibreOffice behaviour.
3. **Every production failure should create a durable regression fixture when practical.**
4. **Determinism by default.** Time, randomness, process failure and asynchronous ordering should be injectable in tests.
5. **No feature is complete with only happy-path tests.** Cancellation, stale revisions, partial I/O, crashes, malformed inputs and resource exhaustion matter.
6. **Compatibility is measured, not asserted.** "Opens DOCX" is not a compatibility standard.
7. **Architecture rules become executable checks where practical.** Documentation alone is insufficient for dependency boundaries.
8. **Performance regressions are correctness regressions for interactive hot paths.**

## 2. Test pyramid for this product

### Tier A — pure unit tests

Runs on every change and should remain extremely fast.

Examples:

- typed ID validation;
- transaction grouping;
- revision graph operations;
- anchors/range transformations;
- feature graph resolution;
- command enablement;
- cache/resource-policy logic;
- import/export normalization helpers.

Target: exhaustive edge cases for small deterministic state machines.

### Tier B — property/model tests

Generated inputs exercise invariants beyond hand-picked cases.

Examples:

- apply -> undo -> original;
- apply -> undo -> redo -> same state/hash;
- arbitrary feature graph resolution is deterministic;
- no valid activation order violates declared dependencies;
- journal replay equals uninterrupted execution;
- anchor transforms preserve intended identity or fail explicitly;
- serialization round trips preserve protocol values.

Property tests should use fixed/reported seeds on failure so CI failures reproduce locally.

### Tier C — deterministic component/integration tests

Use real product layers with controlled adapters.

Examples:

- `DocumentSession` + mock engine;
- feature host + fake capabilities/providers;
- journal + fake crash points;
- IPC codec + loopback transport;
- renderer cache + deterministic fake renderer;
- command bus + history transaction recorder.

The mock engine is a reference test double, not evidence of real document compatibility.

### Tier D — real engine adapter tests

Run against a pinned LibreOffice/LibreOfficeKit environment.

Required scenarios include:

- launch worker;
- open supported document;
- query metadata;
- render region/page;
- hit-test/input/edit;
- save/export;
- close/reopen;
- stale revision rejection;
- cancellation;
- worker crash/restart;
- malformed/untrusted document handling;
- bounded behaviour on large inputs.

These tests should distinguish product bugs from known upstream engine behaviour.

### Tier E — compatibility laboratory

A versioned corpus of DOCX/ODT and later XLSX/PPTX fixtures.

For each relevant fixture capture, where applicable:

- semantic expectations;
- page count;
- text ordering;
- styles/numbering;
- tables;
- headers/footers;
- footnotes/endnotes;
- comments/revisions;
- fields;
- images/shapes;
- equations;
- fonts/fallback;
- layout reference renders;
- round-trip differences.

Fixtures should include synthetic minimal cases and real-world documents that we are legally permitted to store/use.

A format feature is not "supported" until the corpus has representative tests.

### Tier F — visual/layout differential tests

Render known pages/regions with controlled fonts/environment and compare against accepted references using explicit tolerances.

Do not rely on a single whole-page pixel threshold. Classify differences where possible:

- glyph rasterization noise;
- line/page-break change;
- object displacement;
- missing content;
- style/color change.

The long-term native engine must pass the same suite as the bootstrap engine before authority moves.

### Tier G — fuzz/security tests

Fuzz every untrusted structured boundary:

- document/protocol decoders;
- external plugin manifests/messages;
- import metadata;
- clipboard/drop inputs where practical;
- archive/package handling;
- extension package metadata.

Required properties include no panic, no unbounded allocation from tiny input, no invalid authority mutation and useful rejection errors.

### Tier H — crash/recovery fault injection

Inject failure at persistence and engine lifecycle boundaries:

```text
before journal append
mid append
before fsync/checkpoint
mid save/export
worker killed during render
worker killed after transaction accepted
shell restart after acknowledged mutation
```

The contract is defined in `RECOVERY_AND_PERSISTENCE.md`; tests must prove acknowledged edits are not silently lost beyond that contract.

### Tier I — performance qualification

Maintain representative benchmarks for:

- key-to-visible-edit latency;
- selection/cursor movement;
- scrolling/render tile latency;
- open/save/export;
- large-document navigation/search;
- history append/replay;
- feature dispatch overhead;
- plugin boundary overhead;
- memory by document size/page count;
- idle CPU and background analysis budgets.

Benchmarks must report distributions where latency matters (for example p50/p95/p99), not only means.

## 3. Architecture tests

Architecture constraints are release requirements.

R0A includes a repository architecture guard that validates allowed internal crate dependencies. Adding a new crate or internal dependency requires updating the explicit policy in the same change, forcing architectural review instead of silently growing the graph.

Later guards should cover:

- no LibreOffice/UNO/VCL/SFX/Writer types outside engine adapter code;
- UI crates cannot become document authorities;
- extension API remains independent of UI and engine implementations;
- feature modules do not import each other's private implementation crates;
- all document mutation entry points flow through command/transaction admission;
- unsafe Rust remains forbidden unless a documented exception exists.

A guard should fail with an explanation and point to the relevant architecture document/ADR.

## 4. Feature-system tests

Every new extension point requires tests for its composition semantics.

At minimum:

- enable;
- disable;
- dependency closure;
- dependency disabled;
- missing dependency;
- conflict;
- missing provider;
- ambiguous provider;
- explicit provider replacement;
- deterministic ordering;
- activation failure rollback once lifecycle exists;
- deactivation/resource cleanup once lifecycle exists;
- capability denial for external plugins once sandboxing exists.

Generated graph tests are required before the feature catalogue becomes large.

## 5. History/undo tests

History is a flagship feature and a data-integrity subsystem.

Required eventual properties:

```text
apply(A); undo(A) == original
apply(A); undo(A); redo(A) == after_A
apply(A); apply(B); undo(B); redo(B) == after_B
undo; new_edit preserves prior branch
checkpoint + replay == uninterrupted state
selective restore does not mutate unrelated objects
```

Tests must cover semantic grouping (typing bursts versus formatting commands), branch preservation, named revisions, crash recovery, and collaborative compensating transactions when collaboration arrives.

History corruption is release-blocking.

## 6. Test fixture discipline

Every fixture should document:

- purpose;
- origin/licensing status;
- expected behaviour;
- relevant bug/issue if regression-derived;
- required fonts/resources;
- whether it is normative or diagnostic only.

Avoid giant opaque fixture directories with no inventory.

## 7. CI lanes

### Required on every PR

- architecture guard;
- formatting;
- compile/check all targets;
- unit/integration tests that do not require heavyweight external installs;
- clippy with warnings denied.

### Required before compatibility-impacting merge/release

- pinned LibreOffice adapter suite;
- relevant compatibility corpus subset;
- crash/recovery suite where persistence changed;
- security/fuzz regression suite where parsing boundaries changed.

### Scheduled/nightly once infrastructure exists

- full document corpus;
- visual differential suite;
- long fuzz runs;
- large generated feature/history/property workloads;
- performance trend suite;
- leak/long-session soak tests.

## 8. Flaky test policy

A flaky test is a defect in the test or product concurrency model.

Do not normalize rerunning until green. A quarantined flaky test must have an issue, owner/reason and expiry condition; release-critical guarantees cannot be quarantined indefinitely.

## 9. Coverage policy

Coverage is diagnostic, not a target that replaces reasoning.

We care most about:

- invariant/branch coverage in state machines;
- error and recovery paths;
- protocol variants;
- compatibility feature matrix coverage;
- generated/property domains.

A subsystem with 95% line coverage but no crash/recovery test is less trustworthy than one with lower line coverage and complete invariant tests.

## 10. Definition of done

A change is not complete until tests demonstrate the new behaviour and the failure modes appropriate to its layer. If it alters a public contract or invariant, tests and documentation change together.
