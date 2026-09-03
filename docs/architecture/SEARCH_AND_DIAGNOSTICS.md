# Search, Navigation and Document Diagnostics

## Goal

Treat large-document navigation more like an IDE than a 1990s word processor.

## Search index

The background index may cover:

- plain text;
- headings/outline;
- styles;
- tables;
- comments/revisions;
- bookmarks;
- equations;
- captions;
- citations;
- hyperlinks;
- fields;
- diagnostics.

Indexes are revisioned, incremental and rebuildable. They are caches, never authoritative document state.

## Query model

Ordinary text search is mandatory. Structured filters are an R2 goal, e.g.:

```text
style:Heading2 algebra
comment:unresolved
kind:equation
issue:spelling
```

## Document health

Diagnostics are grouped by concern:

- Language;
- Structure;
- Layout;
- References;
- Consistency;
- Accessibility;
- Publication.

Examples include broken references, heading hierarchy problems, unexpected direct formatting, tables outside printable width, undefined abbreviations and unwanted page splits.

## Architecture

Diagnostic providers consume revisioned semantic/layout projections and return immutable results. Providers do not mutate the document directly.

The diagnostic broker owns deduplication, suppression/ignore policy, stale-result handling and presentation priority.
