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

**Outcome:** request visible render data through the native bootstrap engine and prove caller-owned buffers, then establish the authority rule for native render invalidations.

Status: **basic render capability and invalidation/revision safety qualified; transfer architecture deliberately unfrozen**.

Qualified evidence:
- caller-owned tile buffers render correctly through LibreOfficeKit;
- a read-back-verified formatting mutation changes rendered pixels while paragraph text is unchanged;
- `LOK_CALLBACK_INVALIDATE_TILES` is emitted for the render-changing mutation;
- native callbacks are observed off the owning thread in the pinned environment;
- unchanged-code runs place the invalidation on opposite sides of the mutation-return boundary while both occur before Office's modeled `R1` commit;
- native invalidations are therefore advisory render dirtiness, never semantic-revision authority;
- ADR-0009 requires callback ingestion to remain bounded/thread-safe and render work to be gated by product-owned authority.

Remaining:
- measure realistic tile payload size/frequency under scrolling/zoom/edit workloads;
- decide copy versus shared memory and batching from evidence;
- implement the R0B mutation fence/event sequencing that prevents pre-commit dirty callbacks from exposing new render state as an old revision.

## R0A.6 Input/edit/save round trip

**Outcome:** drive a text edit through the engine seam, save DOCX, reopen and verify semantic effect.

Status: **qualified**.

## R0A.7 Semantic projection and reconciliation evidence

**Outcome:** obtain bounded live semantics with explicit authority/revision freshness, then qualify enough structural identity behaviour to avoid freezing engine-native IDs into permanent history anchors.

Status: **substantially qualified; permanent logical anchor/reconciliation schema intentionally not frozen yet**.

### Qualified evidence

- saved-file paragraph semantics are qualified across edit/save/reopen;
- DOCX `w14:paraId` / `w14:textId` are rejected as authoritative identity;
- public LOK accessibility/selection APIs are rejected as whole-document semantic enumeration;
- same-instance access to the exact live LOK-owned Writer `XTextDocument` is qualified on LibreOffice 24.2.7.2;
- the retained semantic view observes unsaved mutation through the original LOK authority;
- the bounded native-neutral semantic projection crosses the isolated process boundary inside the 1024-byte `DETR` payload limit;
- projection version 2 carries an explicit qualification revision;
- engine-facing semantic observations carry `DocumentRevision`;
- product-facing `SessionObservation<T>` is non-forgeable and carries application-owned `AuthorityGeneration` plus `DocumentRevision`;
- retained observations are rejected after both ordinary revision advance and successful authority replacement/reopen, including the `R0 -> fresh R0` case;
- failed opens and rejected transactions do not spuriously replace/stale current authority;
- interior split/merge proves exact semantic restoration can destroy the original first-paragraph Writer object;
- paragraph-boundary insertion/deletion independently reproduces the same non-invertible object relationship;
- formatting-only `ParaAdjust = CENTER` preserves all three live paragraph objects while advancing semantic revision;
- duplicate-text qualification proves two distinct live paragraphs can have byte-for-byte identical content while only one receives a non-text mutation;
- identity probe tokens are explicitly view-local and can reuse the same numeric tuple after close/reopen and full worker restart;
- same live Writer object is therefore strong positive continuity evidence only inside one retained authority; inequality, content equality and naked token equality are all non-decisive for durable identity;
- a public-UNO Writer move/reorder qualification was attempted and explicitly closed after the pinned headless environment kept `.uno:MoveDown` disabled even with verified numbering-rule setup; private-symbol hacks are not accepted as architecture evidence;
- the internal 24.2 bridge remains isolated behind a native-neutral proxy/module ABI;
- command shutdown and clean stdin EOF both qualify deterministic status-0 retirement after a live semantic session.

### Remaining acceptance

1. Define the smallest **product-owned** reconciliation/anchor evidence model justified by the measurements above, without mirroring UNO or content hashes.
2. Exercise that model across explicit save/reload/checkpoint recovery rather than treating view-local Writer continuity as persistence.
3. Make history/recovery consumers depend only on product-owned lineage/structure/semantic evidence and authority scope.

A permanent paragraph/history anchor must not serialize or depend on UNO references, probe tokens, file-format IDs or text equality as identity.

## R0A.8 Worker failure recovery

**Outcome:** forcibly kill worker during a session; shell/application harness remains alive and can restart/reopen from an explicit checkpoint/recovery policy.

Current evidence:
- forced worker death is contained;
- host can observe non-success status + EOF;
- fresh worker restart/reopen works;
- semantic authority can be reacquired at a fresh revision;
- product-owned `AuthorityGeneration` prevents an old `R0` observation becoming current merely because the replacement engine also starts at `R0`.

Remaining:
- define explicit checkpoint/reopen lineage and reconciliation semantics;
- prove application/session recovery rather than only process restartability;
- prove stale asynchronous/render work from the dead authority cannot enter the recovered session.

## R0A.9 Compatibility harness

**Outcome:** fixture manifest, open/edit/save/reopen pipeline, preservation/semantic assertions and artifact capture for later Word-oracle comparison.

Status: **next practical implementation frontier**.

Build this around normalized semantic assertions rather than binary-package equality. Reuse the existing deterministic DOCX generators and native semantic contracts instead of creating another parallel probe framework.

## R0A.10 UI framework qualification

**Outcome:** small prototype exercising native windowing, IME/text input, accessibility tree, scroll/viewport composition, menus/clipboard and high DPI.

Status: **not yet frozen; Slint remains a candidate, not a decision**.

Decision produces/supersedes ADR-0005.

## R0A exit gate

Do not begin broad R1 UI feature work until:

- open/edit/render/save are proven through the intended boundaries;
- structural semantic identity/reconciliation is understood enough to avoid freezing weak history anchors;
- application-level worker recovery/checkpoint semantics are proven;
- render invalidations are safely sequenced beneath application authority;
- the compatibility harness can enforce normalized preservation semantics;
- the UI framework is selected from qualification evidence rather than convenience.
