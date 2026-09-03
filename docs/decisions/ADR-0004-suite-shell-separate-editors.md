# ADR-0004: One suite shell, separate editor modules and engines

- Status: Accepted
- Date: 2026-09-03

## Context

Users benefit from one coherent Office-like product, while documents, spreadsheets and presentations have fundamentally different semantic engines.

## Decision

The eventual product has one shared desktop shell/platform but separate domain editor modules and process-isolated engines. Shared infrastructure exists only where semantics genuinely overlap.

## Consequences

- consistent settings, identity, plugins, AI, workspace and command experience;
- no giant multi-domain editor core;
- future modules can migrate away from LibreOffice independently;
- suite work is deferred until the document editor is excellent.
