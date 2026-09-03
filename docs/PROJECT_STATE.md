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
- native/OpenDoc-style engine is a future migration candidate, not initial authority.

## Implemented in repository skeleton

- Rust workspace;
- strongly typed protocol revision and transaction primitives;
- replaceable `DocumentEngine` trait;
- deterministic mock engine;
- revision-conflict test coverage;
- UI-agnostic document session;
- desktop and document-worker harness placeholders;
- CI quality gates;
- initial product/architecture/engineering documentation.

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

## Explicitly not started

- production UI;
- native document engine;
- collaboration;
- plugins;
- spreadsheets/presentations.
