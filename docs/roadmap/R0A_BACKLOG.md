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

**Outcome:** request visible render data through the native bootstrap engine, prove caller-owned buffers, establish authority-safe invalidation behavior and select the worker/host raster transfer architecture from measured workload evidence.

Status: **render capability, invalidation/revision safety and out-of-band transfer architecture qualified**.

Qualified evidence:
- caller-owned tile buffers render correctly through LibreOfficeKit;
- a read-back-verified formatting mutation changes rendered pixels while paragraph text is unchanged;
- `LOK_CALLBACK_INVALIDATE_TILES` is emitted for the render-changing mutation;
- native callbacks are observed off the owning thread in the pinned environment;
- unchanged-code runs place the invalidation on opposite sides of the mutation-return boundary while both occur before Office's modeled `R1` commit;
- native invalidations are therefore advisory render dirtiness, never semantic-revision authority;
- ADR-0009 requires callback ingestion to remain bounded/thread-safe and render work to be gated by product-owned authority;
- two independent unchanged-code Writer render-transfer runs reproduce the same structural raster geometry and caller-owned-buffer checksums;
- the qualification workload measures a 256 KiB 1× tile, 1 MiB 2× tile, 3 MiB 1× visible viewport and 12 MiB 2× visible viewport;
- current fixture page raster volume is approximately 3.86 MiB at 1× and 15.46 MiB at 2×;
- real Writer 12-tile paint timing is recorded diagnostically but intentionally not used as a hosted-CI performance threshold;
- ordinary control-frame pixel transfer is rejected by scale: even one measured tile is orders of magnitude larger than the current small control envelope;
- ADR-0011 selects small authority/revision-tagged control descriptors plus host-owned bounded reusable out-of-band render buffers with scoped worker leases;
- the OS-specific shared-memory/mapping backend, production tile size and pool sizing remain replaceable/tunable rather than frozen into application architecture.

Remaining implementation/tuning moves to R0B:
- implement a host-owned bounded buffer pool and platform mapping backend;
- add lease generations, geometry/capacity validation and worker-death reclamation;
- implement the mutation/event fence that prevents pre-commit dirty callbacks from exposing new render state as an old revision;
- tune tile/pool/prefetch policy against broader scroll, zoom, edit and high-DPI workloads without reopening the control/data-plane split.

## R0A.6 Input/edit/save round trip

**Outcome:** drive a text edit through the engine seam, save DOCX, reopen and verify semantic effect.

Status: **qualified**.

## R0A.7 Semantic projection, reconciliation evidence and durable paragraph anchors

**Outcome:** obtain bounded live semantics with explicit authority/revision freshness, qualify structural identity behavior, and define the smallest engine-independent durable paragraph identity model justified by that evidence.

Status: **qualified for the first product-owned durable paragraph-anchor model**.

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
- `SessionAuthorityStamp` extends the same product-owned authority/revision provenance to asynchronous render/search/diagnostic work without inventing another epoch mechanism;
- retained observations and authority stamps are rejected after ordinary revision advance, explicit authority withdrawal and successful authority replacement/reopen, including the `R0 -> fresh R0` case;
- accepted transactions receive monotonic product-owned operation sequence numbers only after engine acceptance; rejected operations consume neither revision nor operation sequence;
- recovery checkpoints record exact source authority plus the accepted-operation cursor represented by the checkpoint;
- checkpoint recovery validates a complete contiguous accepted-operation tail before opening replacement authority and replays already-accepted intent without issuing new operation IDs;
- partial replay failure withdraws replacement authority rather than publishing a partially reconstructed document;
- interior split/merge proves exact semantic restoration can destroy the original first-paragraph Writer object;
- paragraph-boundary insertion/deletion independently reproduces the same non-invertible object relationship;
- formatting-only `ParaAdjust = CENTER` preserves all three live paragraph objects while advancing semantic revision;
- duplicate-text qualification proves two distinct live paragraphs can have byte-for-byte identical content while only one receives a non-text mutation;
- identity probe tokens are explicitly view-local and can reuse the same numeric tuple after close/reopen and full worker restart;
- same live Writer object is therefore strong positive continuity evidence only inside one retained authority; inequality, content equality and naked token equality are all non-decisive for durable identity;
- a public-UNO Writer move/reorder qualification was attempted and explicitly closed after the pinned headless environment kept `.uno:MoveDown` disabled even with verified numbering-rule setup; private-symbol hacks are not accepted as architecture evidence;
- the internal 24.2 bridge remains isolated behind a native-neutral proxy/module ABI;
- command shutdown and clean stdin EOF both qualify deterministic status-0 retirement after a live semantic session;
- `document-anchors` now owns durable `DocumentLineageId + ParagraphAnchorSequence` identities with **zero internal repository dependencies**;
- duplicate paragraph text receives distinct product anchors; semantic text remains reconciliation evidence rather than an identity key;
- the product structural policy is explicit: ordinary paragraph mutation preserves identity, insertion mints, deletion retires, split preserves the left anchor and mints the right, merge preserves the left and retires the right;
- the product split/merge rule deliberately does not mirror Writer object-survival behavior;
- retired anchor sequences are never reused, including after persisted snapshot reload;
- `ParagraphAnchorSnapshot` is a fixed-width, versioned, bounded durable artifact with explicit paragraph/per-paragraph/total semantic-byte admission limits;
- an explicit filesystem checkpoint write/read round trip preserves product lineage and paragraph anchors for the exact saved semantic projection;
- same-lineage exact rebind refuses reordered, externally changed or foreign-lineage projections rather than searching by text or guessing;
- ADR-0013 and `DURABLE_PARAGRAPH_ANCHORS.md` record the product-owned identity contract.

R0A therefore freezes **ownership and the minimal paragraph identity key**, not every future reconciliation/history representation.

Deferred evolution above this foundation:
- richer structural/history-assisted reconciliation for independently modified external files;
- semantic anchors below paragraph granularity;
- persistent history DAG/commit representation;
- structural command adapters for the complete editing command surface;
- filesystem placement, atomic replacement and durability policy for production anchor/checkpoint artifacts.

None of those deferred items may reintroduce UNO references, probe tokens, file-format IDs or text equality as logical identity.

## R0A.8 Worker failure recovery

**Outcome:** forcibly kill worker during a session; shell/application harness remains alive and can restart/reopen from an explicit checkpoint/recovery policy.

Status: **recovery authority/checkpoint/journal semantics qualified; durable storage and production supervisor wiring move to R0B**.

Qualified evidence:
- forced worker death is contained;
- host can observe non-success status + EOF;
- fresh worker restart/reopen works;
- semantic authority can be reacquired at a fresh revision;
- product-owned `AuthorityGeneration` prevents an old `R0` observation becoming current merely because the replacement engine also starts at `R0`;
- authority can be explicitly withdrawn while a dead engine/worker binding is replaced, making all prior semantic and asynchronous authority stamps non-current immediately;
- a replacement authority is published only after checkpoint restore and complete journal replay succeed;
- failed checkpoint open does not consume an authority generation or publish replacement authority;
- incomplete/gapped journal evidence is rejected before replacement authority is opened;
- replay failure after checkpoint open withdraws the partially reconstructed authority;
- accepted-operation sequence continuity survives recovery, while the replacement engine is free to restart its local `DocumentRevision` clock;
- old asynchronous/render-style authority stamps remain rejected after recovery under the new generation;
- ADR-0012 records recovery as **new ephemeral authority + preserved accepted-operation lineage**, not restoration of engine object identity;
- ADR-0011's render lease model provides the corresponding data-plane rule: all unfinished render leases from a dead/replaced authority are invalid and may never publish into the recovered session.

R0B implementation work:
- bind the qualified state machine to persisted checkpoint artifacts and an on-disk accepted-operation journal;
- qualify journal/checkpoint durability, batching, atomic replacement and crash/power-loss behavior without adding typing latency;
- wire the session recovery state machine into the production worker supervisor and UI recovery surfaces;
- garbage-collect superseded durable recovery artifacts only after a newer durable checkpoint/save boundary is proven safe.

## R0A.9 Compatibility harness

**Outcome:** fixture manifest, open/edit/save/reopen pipeline, preservation/semantic assertions and artifact capture for later Word-oracle comparison.

Status: **first normalized compatibility vertical slice qualified**.

Qualified evidence:
- declarative `office.compatibility-manifest.v1` with strict schema admission;
- registry-selected generators/operations/projections rather than executable manifest commands;
- explicit `docx-paragraph-text-v1` normalization independent of ZIP bytes, OOXML paragraph IDs and Writer object identity;
- bounded fixture count, semantic payload, DOCX size and execution time;
- deterministic per-fixture input/round-trip/log/result artifacts plus run summary;
- real LibreOffice generate -> open/edit/save/reopen -> normalized before/after assertions in mandatory native CI;
- compatibility-generated artifacts are successfully reused by the specialist structural identity, insertion/deletion, formatting, restart and invalidation qualifications;
- package SHA-256 values are diagnostic only, never semantic goldens;
- manifest/projection unit tests run without LibreOffice in ordinary CI;
- ADR-0010 records normalized product semantics as the compatibility definition.

Remaining expansion is product breadth rather than harness architecture:
- add focused versioned projections/fixtures for formatting, lists, tables, images and page/section layout as those surfaces enter implementation;
- add an independent external oracle adapter when Word-oracle infrastructure is available;
- avoid reintroducing one-off ordinary round-trip scripts when a fixture belongs in this harness.

## R0A.10 UI framework qualification

**Outcome:** small prototype exercising native windowing, IME/text input, accessibility tree, scroll/viewport composition, menus/clipboard and high DPI.

Status: **first Slint viability slice qualified; production framework remains deliberately unfrozen**.

Qualified evidence:
- Slint 1.17.1 candidate is isolated under `spikes/` with Rust 1.92; Office product crates remain on Rust 1.85;
- the editor-shaped candidate compiles and tests on Ubuntu, Windows and macOS;
- Ubuntu also passes candidate fmt and pedantic clippy with `-D warnings`;
- real Linux Winit/software window creation succeeds under Xvfb at forced 1× and 2× scale factors;
- the 1× native window reports `1100x800`, while the 2× native window reports `2200x1600`;
- both DPI runs preserve the exact 262,144-byte caller-owned raster and deterministic checksum used by the UI qualification workload;
- accessibility support is enabled in the native candidate and explicit landmarks/IDs compile across the target platforms;
- Linux qualification exposed explicit fontconfig and XKB/X11 packaging dependencies, now declared in CI;
- Slint-generated Rust cannot live beneath a crate-level `forbid(unsafe_code)` because generated code scopes its own allowance; only the isolated candidate crate uses `unsafe_code = "deny"`, while Office product crates remain `forbid`;
- changing forced `SLINT_SCALE_FACTOR` requires recompiling the candidate crate because Slint compiler passes and the Winit backend both consume scale information;
- the ordinary Office Rust 1.85 and full native LibreOffice qualification gates remain green with the candidate quarantined.

This is a **viability result, not a selection**. ADR-0005 remains normative.

Remaining selection evidence:
1. exercise real-platform IME/international input on Windows, macOS and Linux;
2. exercise actual screen-reader/accessibility-tree behavior, including large/virtualized UI focus;
3. make clipboard, drag/drop, native file-dialog and menu integration explicit;
4. measure large document viewport composition, scrolling, resizing and zoom behavior sufficiently to reject obvious jank;
5. close packaging/licensing/toolchain questions, including the long-term Rust MSRV and Slint attribution route;
6. compare against at least the strongest control alternative if any unresolved Slint limitation remains material;
7. supersede ADR-0005 only after the framework decision is justified by this evidence.

## R0A exit gate

Do not begin broad R1 UI feature work until:

- open/edit/render/save are proven through the intended boundaries;
- product-owned paragraph identity is independent of engine/file-format identity and conservative persisted rebind is proven;
- application-level worker recovery/checkpoint semantics are proven;
- render invalidations are safely sequenced beneath application authority;
- the compatibility harness can enforce normalized preservation semantics;
- the UI framework is selected from qualification evidence rather than convenience.
