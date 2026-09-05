# R0A Backlog — PR-sized work programme

These are ordered to burn down architectural uncertainty before product breadth.

## R0A.1 Workspace + contracts

**Outcome:** green Rust workspace, CI, architecture docs, mock engine and revision-conflict semantics.

Status: **qualified**.

Acceptance:
- fmt/check/test/clippy green in CI;
- no production GUI/LO dependency yet;
- documentation entry points discoverable.

## R0A.2 LibreOffice build/adapter spike

**Outcome:** minimal native adapter can start LibreOfficeKit and expose bounded/versioned qualification capability through an isolated worker-owned boundary.

Status: **qualified for the pinned R0A environment**.

Acceptance evidence:
- pinned LibreOffice 24.2.7.2 environment documented;
- no LO type appears in product Rust crates;
- lifecycle errors are explicit;
- version-specific internal semantic dependencies are quarantined in an unloadable compatibility module;
- normal owned-object teardown and process-global runtime reclamation are distinguished explicitly.

## R0A.3 Worker transport spike

**Outcome:** host starts worker, negotiates/uses a bounded control envelope, preserves request correlation and distinguishes clean/abnormal process termination.

Status: **qualified** for the R0A `DETR` control-frame spike.

The final domain serializer and cross-platform channel remain deliberately unfrozen.

## R0A.4 DOCX open + metadata

**Outcome:** worker/native qualification opens a fixture and returns document/layout metadata without UI involvement.

Status: **qualified**.

## R0A.5 Render path

**Outcome:** request visible render data through the native bootstrap engine and prove caller-owned buffers.

Status: **basic capability qualified; transfer architecture deliberately unfrozen**.

Remaining:
- measure realistic tile payload size/frequency;
- decide copy versus shared memory and batching from evidence.

## R0A.6 Input/edit/save round trip

**Outcome:** drive a text edit through the engine seam, save DOCX, reopen and verify semantic effect.

Status: **qualified**.

## R0A.7 Semantic projection

**Outcome:** obtain bounded live semantics with explicit revision freshness, then qualify structural identity/reconciliation before any permanent semantic-anchor model is frozen.

### Qualified evidence

- saved-file paragraph semantics are qualified across edit/save/reopen;
- DOCX `w14:paraId` / `w14:textId` are rejected as authoritative identity;
- public LOK accessibility/selection APIs are rejected as whole-document semantic enumeration;
- same-instance access to the exact live LOK-owned Writer `XTextDocument` is qualified on LibreOffice 24.2.7.2;
- the retained semantic view observes unsaved mutation through the original LOK authority;
- the bounded native-neutral semantic projection crosses the isolated process boundary inside the 1024-byte `DETR` payload limit;
- projection version 2 carries an explicit qualification revision;
- product-facing semantic reads return `SemanticObservation<T>` stamped with `DocumentRevision`;
- `DocumentSession` rejects stale retained observations after successful mutation and does not stale them after rejected mutation;
- real native qualification proves `R0 -> successful mutation -> R1`, plus fresh process/reopen -> fresh `R0`;
- semantic access is tied to document lifetime and removed on close;
- oversized semantic observations are typed limit failures while the worker remains healthy;
- a fresh process can restart/reopen and reacquire the original semantic snapshot after forced death;
- the internal 24.2 bridge is isolated behind a native-neutral proxy/module ABI;
- the adapter executable itself does not link UNO/`libmergedlo`;
- command shutdown and clean stdin EOF both qualify deterministic status-0 retirement after a live semantic session.

### Remaining acceptance

1. **Paragraph split/merge identity qualification** — measure same-instance object survival/replacement relations without exposing raw UNO identity or creating a product `ParagraphId`.
2. Extend to insertion/deletion around retained paragraphs, move/reorder and formatting-only edits.
3. Determine the smallest structural evidence needed for deterministic reconciliation rather than mirroring UNO.
4. Requalify identity/reconciliation across save/reload separately from live-instance behaviour.
5. Use the resulting evidence to constrain durable semantic/history anchor design.

Do not start a permanent history-anchor type before items 1–4 are understood.

## R0A.8 Worker failure recovery

**Outcome:** forcibly kill worker during a session; shell/application harness remains alive and can restart/reopen from an explicit checkpoint/recovery policy.

Current evidence:
- forced worker death is contained;
- host can observe non-success status + EOF;
- fresh worker restart/reopen works;
- semantic authority can be reacquired at a fresh revision.

Remaining:
- define checkpoint/reopen semantics only after structural identity/reconciliation evidence constrains what can be recovered;
- prove application/session recovery rather than only process restartability.

## R0A.9 Compatibility harness

**Outcome:** fixture manifest, open/edit/save/reopen pipeline, preservation/semantic assertions and artifact capture for later Word-oracle comparison.

Build this around normalized semantic assertions rather than binary-package equality.

## R0A.10 UI framework qualification

**Outcome:** small prototype exercising native windowing, IME/text input, accessibility tree, scroll/viewport composition, menus/clipboard and high DPI.

Decision produces/supersedes ADR-0005.

## R0A exit gate

Do not begin broad R1 UI feature work until:

- open/edit/render/save are proven through the intended boundaries;
- structural semantic identity/reconciliation is understood enough to avoid freezing weak history anchors;
- application-level worker recovery/checkpoint semantics are proven;
- the UI framework is selected from qualification evidence rather than convenience.
