# First 90 Days

Goal: within roughly 90 days of serious implementation, reach a build that we voluntarily use instead of ONLYOFFICE for ordinary documents. This is a direction and sequencing tool, not a promise tied to calendar duration.

## R0A — Contracts and risk spikes

Exit only when these are demonstrated, not merely designed:

- Rust workspace and CI green;
- versioned engine API/protocol prototype;
- isolated worker lifecycle;
- load a real DOCX through LibreOfficeKit in a worker;
- render at least one page/region into our process;
- input/edit through the adapter;
- save/reopen and verify the edit;
- kill/restart worker without killing shell harness;
- collect a semantic slice (text/paragraph/style information);
- establish compatibility fixture format;
- establish latency/trace instrumentation;
- choose UI framework only after an accessibility/platform/rendering spike.

## R0B — Session core

- document tabs/session manager;
- revision tracking;
- mutation/transaction envelope;
- hit testing/caret/selection contract;
- viewport scheduler and bounded render cache;
- open/save state machine;
- recovery checkpoint/journal prototype;
- deterministic mock engine for fast tests.

## R0C — Editing prototype

- usable paginated viewport;
- typing/deletion/selection;
- scrolling/zoom;
- basic character and paragraph formatting;
- undo/redo integration;
- robust worker error surfaces;
- real DOCX round-trip regression tests.

**Milestone:** recognisable working word processor.

## R1A — Daily-driver basics

- styles and style inspector;
- lists;
- tables;
- images;
- sections/page setup;
- headers/footers;
- find/replace and outline;
- PDF and print;
- application settings and keyboard map;
- polished file/recent-document workflow.

## R1B — Language and reliability

- native low-latency spelling path;
- custom/project dictionaries;
- grammar backend service;
- revision-safe diagnostics;
- atomic save qualification;
- crash injection and worker recovery;
- external-file modification detection;
- performance qualification on large documents.

## R1C — Switching challenge

For at least a week of normal work, use our editor by default. Every reason to open Word/ONLYOFFICE becomes one of:

1. R1 blocker;
2. explicitly accepted post-R1 compatibility tail;
3. product/UX opportunity.

R1 exits when the blockers are low enough that using the product is a choice, not a test ritual.

## Parallel tracks throughout

### Compatibility laboratory
Continuously grow Word/LibreOffice/ONLYOFFICE fixtures and round-trip checks.

### Documentation
Every subsystem change updates its contract/ADR/debt record in the same PR when architecture or behaviour changes.

### Performance
Every hot path gets measurable budgets and regression tests before optimisation folklore grows around it.

### AI development
Agents work from narrow written contracts; independent streams may proceed in parallel only where ownership boundaries make merge conflicts and architectural divergence unlikely.
