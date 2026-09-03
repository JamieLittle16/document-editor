# Future Suite Architecture

## Decision

One cohesive installed product/shell; separate domain editors and engines.

```text
Office shell/platform
  |- Document module -> document engine
  |- Spreadsheet module -> spreadsheet engine (future)
  `- Presentation module -> presentation engine (future)
```

## Shared platform candidates

- workspace/recent files;
- settings/themes;
- command palette;
- identity/account/sync adapters;
- plugin host;
- AI broker;
- update system;
- common accessibility/platform integration;
- selected typography/graphics primitives where semantics truly overlap.

## Explicit anti-pattern

Do not create a `GiantEditor` with document/spreadsheet/presentation conditionals throughout a shared semantic core.

A document table and spreadsheet grid are not one abstraction merely because both display cells.

## Embedding

Cross-editor embedded/linkable content should use explicit typed representations and render/update services rather than a hidden OLE-style second application UI living inside another document.
