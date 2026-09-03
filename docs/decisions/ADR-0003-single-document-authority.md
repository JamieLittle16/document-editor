# ADR-0003: Maintain exactly one authoritative complete document model

- Status: Accepted
- Date: 2026-09-03

## Context

A tempting bootstrap design is to maintain a Rust document model and a LibreOffice document model simultaneously and continuously synchronise both. That creates conflict, identity and fidelity failure modes before we possess a native engine.

## Decision

During the LibreOffice phase, the engine worker is authoritative for complete document state. Rust stores revisioned semantic projections and indexes, not an independently editable duplicate.

## Consequences

- fewer synchronisation bugs;
- language/search/diagnostics still get efficient semantic data;
- native authority requires an explicit future migration/fidelity gate rather than accidental drift.
