# Project State

Last updated: 2026-09-03

## Current phase

**R0A — architecture/contracts and high-risk spikes.**

## Accepted strategic decisions

- document editor first;
- eventual one-suite shell with separate editor modules/engines;
- Rust-led application architecture;
- LibreOffice Writer/LibreOfficeKit as quarantined bootstrap engine;
- heavyweight engine out of process;
- exactly one complete authoritative document model;
- strong documentation/ADR/debt discipline;
- UI framework deliberately not frozen until qualification spike;
- native/OpenDoc-style engine is a future migration candidate, not initial authority;
- minimal non-swappable correctness kernel surrounded by modular product features;
- bundled features use explicit feature/service contracts wherever practical;
- external plugins later reuse product contracts behind a capability-based sandbox rather than receiving engine access.

## Implemented in repository skeleton

- Rust workspace;
- strongly typed protocol revision and transaction primitives;
- replaceable `DocumentEngine` trait;
- deterministic mock engine;
- revision-conflict test coverage;
- UI-agnostic document session;
- desktop and document-worker harness placeholders;
- CI quality gates;
- initial product/architecture/engineering documentation;
- stable feature and service identifiers;
- declarative bundled/external feature manifests;
- deterministic feature graph resolution;
- explicit enable/disable semantics;
- declared dependencies and conflicts;
- replaceable service/provider selection;
- dependency/provider-before-consumer activation ordering;
- tests for invalid feature graphs and provider ambiguity;
- ADR-0006 and normative feature/extension architecture documentation.

## Immediate next engineering spikes

1. Finish wiring the feature resolver into an application-level feature host without adding a runtime plugin dependency.
2. Establish a minimal real LibreOfficeKit worker build on Linux.
3. Define process transport and protocol envelope after measuring payload classes.
4. Open a DOCX in the worker and obtain page/size metadata.
5. Render one page/region and transfer it to a host harness.
6. Drive one text edit and save/reopen.
7. Extract a minimal semantic snapshot and determine identity stability.
8. Crash/kill the worker and prove shell/session recovery behaviour.
9. Build the first compatibility fixture runner.
10. Run UI framework qualification (Slint leading candidate, alternatives measured).
11. Add generated/property tests for larger feature graphs before external plugin loading work begins.

## Explicitly not started

- production UI;
- native document engine;
- collaboration;
- runtime loading of third-party plugins;
- WASM runtime selection;
- plugin package/marketplace design;
- spreadsheets/presentations.

## Current feature-system boundary

R0A resolves feature metadata only. It deliberately does **not** yet instantiate features, load WASM, expose UI contribution schemas or grant OS capabilities. Those decisions remain behind later qualification/ADR work.

The kernel/feature boundary is defined in `docs/architecture/FEATURES_AND_EXTENSIONS.md` and ADR-0006.
