# Architecture

Status: **Normative direction; interfaces may still evolve during R0A**

## System shape

```text
Desktop shell / UI
        |
    App core
        |
 Document session -------- Language / Search / Diagnostics
        |
 Versioned engine protocol
        |
  Document worker process
        |
  Engine adapter
        |
 LibreOffice Writer (bootstrap)
```

Later engine implementations may include OpenDoc or a native engine. The application must not need to know which implementation is active.

## Hard boundaries

### UI boundary

UI code owns presentation and input. It does not own document semantics, persistence, history, engine state, or language analysis.

### Application boundary

Application orchestration consumes typed commands and immutable/revisioned state. It does not call LibreOffice APIs.

### Engine boundary

The engine protocol deals in product concepts: documents, transactions, ranges, anchors, layout regions, semantic snapshots, export operations, and engine revisions.

It never exposes UNO/VCL/SFX/Writer implementation objects.

### Process boundary

Heavy document engines run out of process. Blocking engine work cannot block the UI thread. Engine failure is recoverable at the session level.

## One authority rule

There is exactly one complete authoritative document state.

During the bootstrap phase that authority is the LibreOffice-backed worker. Rust-side semantic structures are revision-tagged projections used for navigation, search, proofreading, diagnostics and product features.

We do **not** independently edit a second document model and attempt continuous two-way reconciliation.

Authority may move to a native engine only after explicit fidelity gates are met.

## Dependency direction

```text
desktop -> app-core -> document-session -> document-engine-api -> document-protocol
                                      \-> language/search/diagnostics
engine adapters -----------------------> document-engine-api
```

Lower layers never depend on UI layers.

## Invariants

1. No synchronous engine call on the UI thread.
2. No LibreOffice implementation type above the engine adapter.
3. All asynchronous results identify the source revision they were computed from.
4. Stale results are discarded or explicitly rebased; never applied blindly.
5. Every cache has an explicit resource bound.
6. Every parser/worker has resource limits.
7. Engine crashes do not take down the desktop shell.
8. Document saves are atomic from the user's perspective.
9. Unsupported fidelity loss is surfaced.
10. All user-visible mutations have an auditable transaction/command origin.

## Suite architecture

The eventual product is one cohesive suite shell with independent editor modules and domain engines:

```text
Suite shell
  |- Documents -> document worker/engine
  |- Spreadsheets -> spreadsheet worker/engine   (future)
  `- Presentations -> presentation worker/engine (future)
```

Shared platform infrastructure may include settings, commands, identity, AI, plugin hosting, update system, file/recent workspace, theming and accessibility. Domain semantics are not shared merely because two editors happen to draw rectangles or text.
