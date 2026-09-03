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
- trusted bundled feature lifecycle is supervised by a dedicated host rather than ad-hoc startup callbacks;
- LibreOfficeKit integration is qualified outside the Rust workspace before any unsafe/native adapter contract is frozen.

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
- ADR-0006 plus normative feature/extension and feature-host documentation;
- standalone LibreOfficeKit qualification probe outside the Rust workspace;
- deterministic source-generated DOCX probe fixture;
- stock Ubuntu 24.04 LibreOfficeKit open/layout/render/save/reopen qualification;
- isolated LibreOffice user-profile URI in qualification;
- caller-owned BGRA/RGBA tile-buffer qualification;
- DOCX round-trip structural validation;
- primitive UTF-8 text mutation persistence qualification (pending/covered by current CI head).

## Qualified LibreOfficeKit reference environment

First green open/render/save/reopen run:

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
Writer layout: 12474 x 17406 TWIPs
Tile mode: BGRA
256 x 256 render FNV-1a: 0x299c15792be4f780
Round-trip reopen: OK
Round-trip DOCX bytes: 4983
```

These are recorded qualification observations, not all permanent golden values. Structural, semantic and visual compatibility contracts will be defined separately.

## Immediate next engineering spikes

1. Complete/keep green the persisted LibreOfficeKit text-edit round-trip qualification.
2. Define process transport and protocol envelope after measuring payload classes.
3. Replace the mock-only worker harness with a supervised process-transport vertical slice while keeping the real engine adapter quarantined.
4. Extract a minimal semantic snapshot and determine identity stability across edits/reload.
5. Exercise LibreOfficeKit callbacks/invalidation and map their ordering/threading behaviour.
6. Crash/kill the worker and prove shell/session recovery behaviour.
7. Measure tile/render payload patterns to decide copy versus shared memory and batching thresholds.
8. Build the first compatibility fixture runner.
9. Run UI framework qualification (Slint leading candidate, alternatives measured).
10. Add generated/property tests for larger feature graphs before external plugin loading work begins.
11. Define additive contribution registries only when the first real product feature needs commands/panels/diagnostics; do not invent a generic callback bus.
12. Write the unsafe/native adapter ADR only after the remaining LibreOfficeKit measurements constrain the design.

## Explicitly not started

- production UI;
- production Rust-to-LibreOffice FFI;
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

## Current engine-spike boundary

R0A has proved that stock LibreOfficeKit can be used headlessly for Writer document loading, layout, tile rendering and DOCX round-tripping without exposing LibreOffice types to product Rust code. The active probe now additionally requires a real text edit to persist through the saved OOXML package.

This is evidence for the engine boundary, not permission to bypass it. The production adapter/transport remains deliberately unfrozen.
