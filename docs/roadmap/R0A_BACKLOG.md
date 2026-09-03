# R0A Backlog — PR-sized work programme

These are ordered to burn down architectural uncertainty before product breadth.

## R0A.1 Workspace + contracts

**Outcome:** green Rust workspace, CI, architecture docs, mock engine and revision-conflict semantics.

Acceptance:
- fmt/check/test/clippy green in CI;
- no production GUI/LO dependency yet;
- documentation entry points discoverable.

## R0A.2 LibreOffice build/adapter spike

**Outcome:** minimal C/C++ adapter can start LibreOfficeKit and report version/capabilities through a Rust-callable or worker-owned boundary.

Acceptance:
- pinned supported LO version documented;
- no LO type appears in product Rust crates;
- lifecycle errors are explicit.

## R0A.3 Worker transport spike

**Outcome:** host starts worker, negotiates protocol version, sends bounded request, handles timeout/exit.

Compare candidate encoding/transport choices using actual expected control messages rather than aesthetics.

## R0A.4 DOCX open + metadata

**Outcome:** worker opens a fixture and returns page/document metadata without UI involvement.

## R0A.5 Render path

**Outcome:** request a visible page/region and receive render data in host harness.

Measure copy/IPC cost and only then choose shared-memory optimisation.

## R0A.6 Input/edit/save round trip

**Outcome:** host drives one text edit through the engine seam, saves DOCX, reopens and verifies semantic effect.

## R0A.7 Semantic projection

**Outcome:** extract paragraphs/text/style/language subset with revision tagging. Investigate object/anchor identity stability under edits.

Current evidence:
- saved-file paragraph semantics are qualified across edit/save/reopen;
- DOCX `w14:paraId` / `w14:textId` are rejected as authoritative identity;
- public LOK accessibility/selection APIs are rejected as whole-document semantic enumeration;
- same-instance access to the exact live LOK-owned Writer `XTextDocument` is qualified on pinned LibreOffice 24.2.7.2, including observation of an unsaved LOK mutation through the retained UNO reference.

Remaining acceptance:
- return a bounded native-neutral live semantic projection across the isolated engine boundary;
- exercise insertion/deletion/split/merge/move/formatting edit sequences;
- record candidate identity/reconciliation behaviour without freezing a product `ParagraphId` prematurely;
- requalify save/reload reconciliation separately from live-instance behaviour.

## R0A.8 Worker failure recovery

**Outcome:** forcibly kill worker during a session; shell harness remains alive and can restart/reopen checkpoint.

## R0A.9 Compatibility harness

**Outcome:** fixture manifest, open/edit/save/reopen pipeline, preservation/semantic assertions, artifact capture for later Word oracle comparison.

## R0A.10 UI framework qualification

**Outcome:** small prototype exercising native windowing, IME/text input, accessibility tree, scroll/viewport composition, menus/clipboard and high DPI.

Decision produces/supersedes ADR-0005.

## R0A exit gate

Do not begin broad R1 UI feature work until open/edit/render/save and worker recovery are proven through the intended boundaries.
