# Architecture

Status: **Normative direction; interfaces may still evolve during R0A**

## System shape

```text
Desktop shell / UI
        |
    App core ---------------- Feature host / contributions
        |                               |
 Document session -------- Language / Search / Diagnostics / other features
        |
 Command + transaction admission
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

The product is intentionally split into a **minimal correctness kernel** and **modular product features**. See [FEATURES_AND_EXTENSIONS.md](FEATURES_AND_EXTENSIONS.md).

## Hard boundaries

### UI boundary

UI code owns presentation and input. It does not own document semantics, persistence, history authority, engine state, feature resolution or language analysis.

UI surfaces consume command/contribution metadata. A toolbar, shortcut, command palette or plugin should not each invent a separate mutation path.

### Application/kernel boundary

Application orchestration consumes typed commands and immutable/revisioned state. It does not call LibreOffice APIs.

The minimal kernel owns invariants that cannot be disabled as features:

- document/session authority and revision ordering;
- command/transaction admission and validation;
- durable journal/recovery guarantees;
- engine protocol/process lifecycle;
- capability/security enforcement;
- feature graph resolution/lifecycle supervision;
- mandatory resource/safety policy.

### Feature boundary

Product behaviour outside kernel invariants should be modular where practical. Bundled features declare stable identities, dependencies/conflicts and replaceable service requirements rather than reaching into other feature implementations.

Features mutate documents only through the normal command/transaction path and consume revision-tagged projections/snapshots.

Bundled features may be statically linked for performance. External plugins later use sandboxed adapters. Shared product contracts do not imply identical runtime mechanisms.

### Engine boundary

The engine protocol deals in product concepts: documents, transactions, ranges, anchors, layout regions, semantic snapshots, export operations and engine revisions.

It never exposes UNO/VCL/SFX/Writer implementation objects.

### Process boundary

Heavy document engines run out of process. Blocking engine work cannot block the UI thread. Engine failure is recoverable at the session level.

External extension execution may gain its own process/WASM isolation boundary later; this does not weaken the engine boundary.

## One authority rule

There is exactly one complete authoritative document state.

During the bootstrap phase that authority is the LibreOffice-backed worker. Rust-side semantic structures are revision-tagged projections used for navigation, search, proofreading, diagnostics and product features.

We do **not** independently edit a second document model and attempt continuous two-way reconciliation.

Authority may move to a native engine only after explicit fidelity gates are met.

Features and plugins never become alternative document authorities.

## Dependency direction

```text
desktop -> app-core -> document-session -> document-engine-api -> document-protocol
                      |               \
                      |                \-> language/search/diagnostics/features
                      \-> extension-runtime -> extension-api

bundled features ---------------------------> extension-api + typed product APIs
external plugin adapter --------------------> extension-api + capability APIs
engine adapters ----------------------------> document-engine-api
```

Lower layers never depend on UI layers.

No feature may depend on another feature's private implementation merely because both are first-party code.

## Invariants

1. No synchronous engine call on the UI thread.
2. No LibreOffice implementation type above the engine adapter.
3. All asynchronous document-derived results identify the source revision they were computed from.
4. Stale results are discarded or explicitly rebased; never applied blindly.
5. Every cache has an explicit resource bound.
6. Every parser/worker/plugin boundary has resource limits appropriate to its trust level.
7. Engine crashes do not take down the desktop shell.
8. Document saves are atomic from the user's perspective.
9. Unsupported fidelity loss is surfaced.
10. All user-visible mutations have an auditable transaction/command origin.
11. Feature composition resolves completely before activation.
12. Feature dependencies and service providers are explicit and deterministic.
13. Kernel correctness invariants cannot be disabled through the feature system.
14. Optional feature failure cannot corrupt authoritative document state.
15. A replaceable implementation is selected through a stable service contract, never registration order.

## Customisability principle

The product should be more customisable than conventional office suites without exposing implementation chaos.

We prefer:

```text
stable command + service + contribution contracts
                    |
        +-----------+-----------+
        |                       |
 first-party feature      external extension
```

over:

```text
plugin reaches into arbitrary UI/engine globals
```

This gives us room for future functionality we have not anticipated while preserving testable boundaries.

## Suite architecture

The eventual product is one cohesive suite shell with independent editor modules and domain engines:

```text
Suite shell
  |- Documents -> document worker/engine
  |- Spreadsheets -> spreadsheet worker/engine   (future)
  `- Presentations -> presentation worker/engine (future)
```

Shared platform infrastructure may include settings, commands, identity, history infrastructure, AI, plugin hosting, update system, file/recent workspace, theming and accessibility. Domain semantics are not shared merely because two editors happen to draw rectangles or text.

The extension/service vocabulary may be shared where semantics truly match, while domain-specific contracts remain separate.
