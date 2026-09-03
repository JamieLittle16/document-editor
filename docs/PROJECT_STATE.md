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
- external plugins later reuse product contracts behind a capability-based sandbox rather than receiving engine access;
- trusted bundled feature lifecycle is supervised by a dedicated host rather than ad-hoc startup callbacks.

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
- application-level trusted bundled `FeatureHost`;
- activation rollback and reverse shutdown;
- faulted-state tracking and cleanup retry after deactivation failure;
- explicit rejection of external features from the in-process bundled host;
- tests for invalid feature graphs, provider ambiguity and lifecycle failure injection;
- executable architecture dependency guard in CI;
- ADR-0006 plus normative feature/extension and feature-host documentation.

## Immediate next engineering spikes

1. Establish a minimal real LibreOfficeKit worker build on Linux.
2. Define process transport and protocol envelope after measuring payload classes.
3. Open a DOCX in the worker and obtain page/size metadata.
4. Render one page/region and transfer it to a host harness.
5. Drive one text edit and save/reopen.
6. Extract a minimal semantic snapshot and determine identity stability.
7. Crash/kill the worker and prove shell/session recovery behaviour.
8. Build the first compatibility fixture runner.
9. Run UI framework qualification (Slint leading candidate, alternatives measured).
10. Add generated/property tests for larger feature graphs before external plugin loading work begins.
11. Define additive contribution registries only when the first real product feature needs commands/panels/diagnostics; do not invent a generic callback bus.

## Explicitly not started

- production UI;
- native document engine;
- collaboration;
- runtime loading of third-party plugins;
- WASM runtime selection;
- plugin package/marketplace design;
- hot feature reconfiguration while documents are active;
- spreadsheets/presentations.

## Current feature-system boundary

R0A now resolves feature metadata **and** supervises trusted bundled feature lifecycle. It deliberately does not load external code, expose UI contribution schemas or grant OS capabilities. External extensions remain behind a future sandbox host.

The kernel/feature boundary is defined in `docs/architecture/FEATURES_AND_EXTENSIONS.md`, the lifecycle contract in `docs/architecture/FEATURE_HOST.md`, and the strategic decision in ADR-0006.
