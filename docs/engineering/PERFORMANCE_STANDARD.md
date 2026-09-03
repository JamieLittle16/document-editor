# Performance Qualification Standard

## Principle

Perceptual responsiveness is a product feature and a release gate.

## Provisional budgets

These are hypotheses to validate on representative hardware, not sacred constants.

| Path | Initial target |
|---|---:|
| UI input handling | <4 ms p95 |
| cached 120 Hz frame work | <8.3 ms |
| immediate typed visual feedback | <16 ms preferred |
| current-word spelling feedback | <50 ms |
| basic paragraph grammar result | <250 ms after idle dispatch |
| command palette open | <50 ms |
| indexed search first useful result | <50 ms |
| inspector/toolbar local response | <16 ms |

Save, pagination, engine rendering, grammar and AI are asynchronous and may take longer; they may not freeze direct interaction.

## Qualification workloads

- empty/simple document;
- 50-page mixed document;
- 500-page technical book;
- image-heavy report;
- table-heavy report;
- tracked-changes/review-heavy document;
- pathological but valid DOCX fixtures.

## Required instrumentation

Developer builds must expose or trace:

- frame/input timing;
- engine queue latency;
- IPC latency/bytes;
- tile request/completion/cache metrics;
- memory by process/cache;
- language job latency;
- search/index latency;
- save/checkpoint latency;
- layout invalidations;
- plugin/AI task time when present.

A regression must be explainable from traces, not guessed at.
