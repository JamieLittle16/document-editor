# Project State

Last updated: 2026-09-06

## Current phase

**R0A — architecture/contracts and high-risk spikes.**

The modular feature kernel, bounded worker/process foundation, revision-stamped semantic authority, normalized compatibility harness, Writer render-transfer boundary, structural reconciliation evidence, product-owned durable logical-anchor model, invalidation/restart qualification and application-level checkpoint/replay recovery semantics are now established.

Interior split/merge and paragraph-boundary insertion/deletion prove that live Writer object identity is **not durable logical identity**: exact semantic round trips can replace the original first-paragraph Writer object. Formatting-only mutation supplies the complementary positive-continuity control, duplicate-text qualification prevents content equality becoming identity, and reopen/full-worker-restart qualification proves native identity-token scope ends with the live authority. The resulting rule is asymmetric: same-object equality is strong positive continuity evidence inside one live authority; inequality, text equality and naked native tokens are all non-decisive for durable identity.

ADR-0013 now turns that evidence into the first product-owned anchor contract. `LogicalAnchorId` values live in a product `HistoryLineageId` namespace and outlive replaceable `LiveAnchorBinding<T>` values scoped to exact session authority. Reconciliation is conservative: explicit accepted-operation lineage is strongest, same-engine-object continuity is allowed only inside one authority generation, unique structural + semantic agreement is the fallback, and ambiguous or unresolved cases are never guessed away.

The first Slint 1.17.1 UI viability slice is also qualified without entering product crates: Windows/macOS/Linux source builds pass, Linux native Winit/software windows pass at forced 1× and 2× DPI, and the caller-owned raster boundary survives unchanged. The production UI framework is **still deliberately unfrozen** pending real-platform IME/accessibility, desktop-integration and viewport-performance evidence.

The remaining R0A architectural uncertainty is therefore concentrated in the final evidence-backed UI framework selection. Persisted recovery/anchor storage, richer semantic anchor hints, production supervisor wiring and render-buffer implementation move into R0B rather than reopening the already-qualified authority/identity/control/data-plane semantics.

## Accepted strategic decisions

- document editor first;
- eventual one-suite shell with separate editor modules/engines;
- Rust-led application architecture;
- LibreOffice Writer/LibreOfficeKit as a quarantined bootstrap engine;
- heavyweight engine out of process;
- exactly one complete authoritative document model;
- strong documentation/ADR/debt discipline;
- UI framework deliberately not frozen until qualification evidence exists;
- native/OpenDoc-style engine is a future migration candidate, not initial authority;
- minimal non-swappable correctness kernel surrounded by modular product features;
- bundled features use explicit feature/service contracts wherever practical without paying dynamic-plugin overhead merely for modularity;
- external plugins later reuse product contracts behind capability/sandbox boundaries rather than receiving engine authority;
- trusted bundled feature lifecycle is supervised by a dedicated host rather than ad-hoc startup callbacks;
- LibreOfficeKit integration is qualified outside the Rust workspace before unsafe/native production contracts are frozen;
- process/wire protocol values use fixed-width types rather than host-width `usize` values;
- transaction resource limits and validation are explicit admission policy rather than implicit implementation behaviour;
- process framing is a separate bounded control-plane layer and does not select the permanent document-message serializer;
- large render payloads are not forced through inline control frames merely because a frame codec exists;
- worker EOF, protocol shutdown and worker exit status are separate evidence;
- process restartability is qualified before document-session recovery semantics are designed;
- file-format IDs and engine object addresses are evidence inputs, never product semantic identities by default;
- structural/reload qualification must precede any durable anchor decision; ADR-0013 now freezes only the product-owned identity/live-binding separation justified by that evidence;
- public view/accessibility APIs are not promoted into semantic document APIs unless whole-document behaviour is directly qualified;
- deeper native semantics operate on the same authoritative Writer instance rather than silently creating a second document authority;
- live semantic observations are revision-stamped and must be freshness-checked before retained/asynchronous results can affect current application state;
- same-instance Writer semantic access is qualified for the pinned LibreOffice 24.2.7.2 environment, but its internal process-context ABI is qualification machinery rather than a production API;
- the version-specific semantic dependency is isolated in an unloadable compatibility module behind a native-neutral ABI/proxy;
- live semantic observations crossing the process boundary are bounded, normalized and implementation-neutral rather than serialized UNO objects;
- production adoption of a LibreOffice-internal semantic bridge requires explicit versioning and an ADR rather than a project-wide wrapper grown from a spike;
- the isolated native worker is the bootstrap-engine runtime-reclamation boundary when pinned engine global finalizers are empirically unsafe, but Office-owned semantic/document/LOK objects must still be explicitly destroyed first;
- Writer UNO same-object identity may be consumed as local reconciliation evidence, but must not define product `ParagraphId`, durable history identity or persistence identity;
- semantic equivalence and bootstrap-engine object identity are distinct dimensions: history/replay must remain logically stable even when equivalent edits yield different engine objects;
- product-owned reconciliation is required even for simple paragraph-boundary insertion/deletion because exact semantic restoration can still leave different Writer objects;
- same live Writer object is strong positive continuity evidence, but a changed/missing object is non-decisive and must be reconciled using product-owned lineage/structure/semantic evidence;
- durable logical anchors use product-owned `HistoryLineageId + local sequence` identity and keep ephemeral authority/revision/engine targets in replaceable live bindings;
- duplicate semantics, conflicting strong evidence and insufficient evidence remain explicit `Ambiguous`/`Unresolved` reconciliation outcomes rather than hidden heuristics;
- formatting-only changes advance semantic revision even when the current paragraph-text projection is unchanged, so revision freshness cannot be inferred from text equality;
- recovery publishes a new ephemeral authority only after checkpoint restoration plus complete contiguous accepted-operation replay; accepted-operation lineage survives recovery even when the replacement engine restarts its local revision clock;
- render invalidations are advisory dirtiness beneath product-owned authority and cannot independently advance semantic state;
- the UI toolkit must remain behind a replaceable presentation adapter; toolkit types/codegen/MSRV constraints may not leak into application, history, session, recovery or engine architecture.

## Implemented in repository skeleton

### Core/application architecture

- Rust workspace;
- strongly typed protocol revision and transaction primitives;
- replaceable `DocumentEngine` trait;
- deterministic mock engine;
- revision-conflict test coverage;
- UI-agnostic document session;
- desktop and document-worker harnesses;
- CI quality gates;
- executable architecture dependency guard with separate production/build and explicit test-only internal dependency policies;
- initial product/architecture/engineering documentation and ADR discipline.

### Modular feature kernel

- stable feature and service identifiers;
- declarative bundled/external feature manifests;
- deterministic feature graph resolution;
- explicit enable/disable semantics;
- declared dependencies and conflicts;
- replaceable service/provider selection;
- dependency/provider-before-consumer activation ordering;
- application-level trusted bundled `FeatureHost`;
- activation rollback and reverse shutdown;
- faulted-state tracking and cleanup retry after deactivation failure;
- explicit rejection of external features from the in-process bundled host;
- tests for invalid feature graphs, provider ambiguity and lifecycle failure injection;
- ADR-0006 plus normative feature/extension and feature-host documentation.

### Revision-aware semantic observations

- product-facing immutable `SemanticObservation<T>` values carry the `DocumentRevision` from which they were derived;
- semantic reads no longer discard their authority revision;
- `DocumentSession` owns an explicit O(1) freshness/currentness gate;
- retained observations become deterministically stale after a successful mutation;
- rejected/failed transactions do not spuriously stale observations;
- engine/session revision disagreement is treated as an authority invariant violation rather than silently reconciled;
- desktop/session call sites consume the revision-aware observation contract;
- ADR-0007 documents the product-facing freshness invariant;
- real native semantic projection version 2 carries the same qualification revision signal across the LibreOffice process boundary;
- live native qualification proves `R0 -> mutation -> R1` and fresh process/reopen -> new `R0`;
- formatting-only native qualification proves revision advances even when the current paragraph-text projection remains unchanged;
- product-facing `AuthorityGeneration` distinguishes replacement authority from an old authority even when both engines expose the same local revision value;
- `SessionAuthorityStamp` carries authority/revision provenance for asynchronous render/search/diagnostic work;
- accepted mutations receive monotonic product-owned operation sequence numbers only after engine acceptance.

### Protocol and transaction admission

- fixed-width `RequestId(u64)`, `DocumentRevision(u64)` and `TextOffset(u64)` protocol primitives;
- explicit `TransactionLimits` and pre-mutation validation for edit count, replacement bytes, UTF-8 boundaries and overlap;
- regression coverage proving rejected multi-edit transactions leave document state/revision untouched;
- temporary offsets remain bootstrap values, not future semantic history anchors.

### Bounded transport and mock worker

- std-only `document-transport` control-frame crate;
- fixed 20-byte versioned frame header with request/response role and `RequestId` correlation;
- explicit `FrameLimits` admission on reader and writer;
- payload-size rejection from the header before payload allocation/read;
- typed clean-EOF versus truncated-header/truncated-payload semantics;
- tests for short reads/writes and malformed magic/version/kind/flags;
- dedicated `ENGINE_TRANSPORT.md` keeping framing separate from domain encoding/shared-memory policy;
- real Cargo-built `document-worker` child-process qualification over stdin/stdout;
- deliberately disposable `R0A_*` worker command codec;
- worker-loop tests for request correlation, invalid commands, wrong frame role and shutdown ordering;
- child-process tests for 64-bit request-ID preservation, graceful shutdown, clean stdin EOF, forced death and fresh restart;
- dedicated `DOCUMENT_WORKER_PROCESS_SPIKE.md`.

### LibreOfficeKit direct capability qualification

- standalone LibreOfficeKit qualification probe outside the Rust workspace;
- deterministic source-generated DOCX fixture;
- stock Ubuntu 24.04 LibreOfficeKit open/layout/render/save/reopen qualification;
- isolated LibreOffice user-profile URI;
- caller-owned BGRA/RGBA tile-buffer qualification;
- DOCX round-trip structural validation;
- real UTF-8 text mutation persistence through DOCX save/reopen and OOXML semantic validation.

### Native LibreOffice process qualification

- standalone C++ LibreOfficeKit process adapter outside the Rust workspace;
- independent bounded `DETR` frame implementation without exposing LibreOffice types to Rust;
- real LibreOffice private-profile ownership with isolated process HOME;
- typed real-engine load failure and invalid-command handling;
- graceful protocol shutdown and clean stdin EOF as separately qualified normal retirement paths;
- forced death while a Writer document is live;
- fresh LibreOffice process restart and document reopen after death;
- explicit preservation of intended worker exit status across native retirement.

### Semantic identity/same-authority evidence

- deterministic three-paragraph fixture with seeded `w14:paraId` and `w14:textId` candidates;
- minimal OOXML semantic paragraph projection;
- CI assertions for paragraph cardinality, order, text preservation and edit locality across real LibreOffice edit/save/reopen;
- rejection of DOCX `w14:paraId` / `w14:textId` as authoritative product identities after LibreOffice removed all seeded values on the qualified save path;
- public-LOK live-semantic discovery showing accessibility focus and `SelectAll + getTextSelection()` do not provide reliable whole-document semantics in the headless configuration;
- removal of those unproven approaches from the mandatory native qualification surface;
- acquisition of exactly one Writer `XTextDocument` from the already-running LibreOfficeKit process rather than bootstrapping another office;
- semantic enumeration of the fixture paragraphs from that same live Writer model;
- disposable bounded projection version 2 containing revision plus ordered UTF-8 paragraphs;
- explicit 1024-byte complete semantic-response bound and typed limit rejection;
- proof that the retained same-instance semantic view sees an unsaved LOK prefix mutation only in paragraph 1;
- semantic view lifetime tied to the open Writer document;
- restart/reopen and semantic reacquisition after forced process death;
- dedicated `SEMANTIC_IDENTITY_SPIKE.md` and `LIVE_SEMANTIC_REVISION_QUALIFICATION.md`.

### Paragraph identity/reconciliation qualification

- qualification-only identity-probe projection using view-local tokens assigned by UNO same-object equality;
- probe-token uniqueness and repeatability asserted before identity evidence is interpreted;
- deterministic first-paragraph interior split at character offset `8` with exact semantic and revision assertions;
- deterministic merge with exact semantic restoration and revision assertions;
- twice-reproduced pinned interior split/merge relation showing the original first object is destroyed by the semantic round trip;
- deterministic paragraph-end boundary split reuses the same native primitive to insert an empty adjacent paragraph without a new ABI/wire command;
- twice-reproduced boundary insertion/deletion sequence restores exact `(P0, P1, P2)` semantics at `R2` while reproducing the same non-invertible object relation;
- untouched later paragraphs retain object continuity in both structural sequences;
- qualification ABI v3 adds one formatting-only operation without exposing UNO types to the adapter;
- first-paragraph `ParaAdjust = CENTER` mutation is accepted only after native read-back verification;
- twice-reproduced formatting-only sequence advances `R0 -> R1`, leaves paragraph text/cardinality unchanged and preserves all three paragraph objects with relation `0->0;1->1;2->2`;
- duplicate-text qualification proves distinct live paragraphs can have identical content while remaining separately targetable;
- identity-token scope is explicitly limited to one retained semantic view; close/reopen and full worker restart can reuse the same diagnostic numeric tuple without implying continuity;
- public headless Writer paragraph move/reorder was explicitly investigated and closed as a qualification boundary after `.uno:MoveDown` remained unavailable despite verified list setup and the safe public alternatives/private ABI route were exhausted;
- separate CI contracts pin independently reproduced relations without pinning numeric probe tokens or addresses;
- combined native evidence establishes the asymmetric reconciliation rule: equality is strong positive continuity evidence, inequality/content equality/naked token equality are non-decisive;
- product `HistoryLineageId` plus monotonic `LogicalAnchorId` allocation now establish durable identity independently of engine authority;
- `DurableLogicalAnchor<H>` keeps persistable product hints separate from `LiveAnchorBinding<T>` authority/revision targets;
- reconciliation uses explicit accepted-operation lineage, same-authority engine continuity and unique structural + semantic evidence in conservative precedence order;
- same-engine-object evidence is rejected across authority generations;
- duplicate semantic candidates, conflicting strong evidence and insufficient evidence remain explicit `Ambiguous`/`Unresolved` outcomes;
- save/reload and checkpoint-recovery tests preserve the exact durable anchor ID while replacing live authority;
- `STRUCTURAL_IDENTITY_QUALIFICATION.md` records the engine evidence and ADR-0013 records the product identity/binding decision.

### Recovery authority/checkpoint semantics

- application-owned `AuthorityGeneration` scopes live semantic/asynchronous work independently of engine-local revision numbers;
- accepted operations receive monotonic product-owned sequence numbers only after successful engine acceptance;
- recovery checkpoints bind the exact source authority and accepted-operation cursor represented by the checkpoint;
- recovery validates a complete contiguous accepted-operation tail before opening replacement authority;
- replay reuses already-accepted operation identities rather than manufacturing new user operations;
- checkpoint-open failure publishes no replacement authority;
- replay failure withdraws the partially reconstructed replacement authority;
- successful recovery publishes a fresh authority only after checkpoint restore plus full replay succeed;
- accepted-operation lineage remains continuous even when the replacement engine restarts its own `DocumentRevision` at `R0`;
- stale observations and render-style authority stamps remain invalid after recovery;
- ADR-0012 records recovery as new ephemeral authority plus preserved accepted-operation lineage;
- durable checkpoint/journal encoding, crash-safe storage policy and production supervisor wiring remain R0B work.

### Version-pinned semantic module and native reclamation

- `writer_semantics_24_2.cxx` is an unloadable version-pinned compatibility module containing UNO/internal LibreOffice dependencies;
- `writer_semantics_module_abi.hxx` defines a tiny qualification-only C ABI with no UNO types and versions qualification surfaces explicitly;
- `writer_semantics_proxy.cxx` owns module loading, ABI validation, semantic-view release and bounded native-neutral decoding;
- the adapter executable itself does not link UNO or `libmergedlo`;
- CI proves the module builds, loads, acquires the live Writer authority and unloads after semantic-view release;
- diagnostic qualification proved `lok::Office` destruction returns before the later LibreOffice process-global static-finalizer fault;
- normal worker retirement explicitly releases semantic view/module, document and `lok::Office`, then skips only the unsafe later global-finalizer phase through process-level reclamation;
- graceful shutdown and clean stdin EOF both qualify successful status `0` plus clean stdout EOF after a live semantic session;
- `NATIVE_RUNTIME_RECLAMATION_QUALIFICATION.md` records the exact evidence and prevents this pinned workaround from becoming an unexplained permanent architecture rule.

## Qualified LibreOfficeKit reference environment

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
Writer layout: 12474 x 17406 TWIPs
Tile mode: BGRA
Primitive UTF-8 edit: OK
Persisted edit in OOXML text: OK
Round-trip reopen: OK
```

Raster hashes and round-trip package byte counts are intentionally not semantic goldens.

## Qualified worker-process behaviour

Rust/mock worker CI:

```text
bounded frame codec: OK
real Cargo-built child process: OK
64-bit request correlation across OS pipes: OK
graceful shutdown response + successful exit: OK
clean stdin EOF + successful exit: OK
forced child death + non-success exit: OK
stdout EOF after forced death: OK
fresh child restart after forced death: OK
workspace architecture/fmt/check/tests/Clippy: OK
```

This proves process restartability and failure observation. Application-level recovery semantics now complement it with explicit authority withdrawal, checkpoint provenance and contiguous accepted-operation replay; persisted crash-safe artifacts and production supervisor wiring remain separate R0B implementation work.

## Qualified native LibreOffice process behaviour

Pinned real-engine CI now requires:

```text
cross-language DETR framing: OK
private LibreOffice profile: OK
typed missing-document load failure: OK
invalid command without worker death: OK
same-instance semantic module acquisition: OK
semantic revision R0 before ordinary mutation: OK
semantic revision R1 after successful ordinary mutation: OK
identity probe repeatability without mutation: OK
interior split/merge R0 -> R1 -> R2: OK
pinned interior split/merge identity relations: OK
boundary insertion/deletion R0 -> R1 -> R2: OK
exact insertion semantics (P0, "", P1, P2): OK
exact deletion semantic restoration (P0, P1, P2): OK
pinned boundary insertion/deletion identity relations: OK
formatting-only ParaAdjust CENTER read-back: OK
formatting-only R0 -> R1 revision progression: OK
formatting-only paragraph text/cardinality unchanged: OK
pinned formatting identity relation 0->0;1->1;2->2: OK
duplicate-text ambiguity qualification: OK
identity-token scope across reopen/full worker restart: OK
invalidation callback safety beneath semantic revision authority: OK
semantic size-limit rejection without worker death: OK
semantic module/view removal on close: OK
graceful command shutdown + status 0 + clean EOF: OK
clean stdin EOF after live semantic session + status 0: OK
forced exit with Writer document open: observed/non-success
fresh engine-process restart: OK
fresh reopened semantic revision R0: OK
same fixture semantic snapshot reacquired: OK
```

The R0A native command payload remains disposable and is not the final domain-message protocol.

## Qualified semantic snapshot / identity observation

The deterministic fixture contains three paragraphs with seeded Word 2010 `w14:paraId` and `w14:textId` values. After a real LibreOfficeKit edit/save/reopen on LibreOffice 24.2.7.2:

```text
input paragraphs: 3
round-trip paragraphs: 3
semantic paragraph order/text: preserved
edit locality: paragraph 1
w14:paraId values present after save: 0
seeded w14:paraId preserved: 0 / 3
seeded w14:textId preserved: 0 / 3
```

DOCX `w14` IDs are therefore rejected as authoritative product semantic identities. CI treats semantic preservation as the durable assertion, not LibreOffice's current serialization quirk.

The deeper live path returns:

```text
projection version: 2
revision: u64 qualification revision
payload bound: 1024 bytes
snapshot before edit: R0 + exact 3 fixture paragraphs
unsaved LOK prefix edit: R1 + changed paragraph 1 only
paragraphs 2-3 after edit: unchanged
semantic access after close: rejected
fresh process restart/reopen: fresh R0 snapshot reacquired
```

Both qualified structural sequences reproduce:

```text
representative R0 tokens: (1, 2, 3)
representative R1 tokens: (4, 1, 2, 3)
representative R2 tokens: (4, 2, 3)

pinned R0 -> R1 relation: 0->1;1->2;2->3
pinned R1 -> R2 relation: 0->0;1->-;2->1;3->2
pinned R0 -> R2 relation: 0->-;1->1;2->2
```

The qualified formatting-only control reproduces:

```text
representative R0 tokens: (1, 2, 3)
representative R1 tokens: (1, 2, 3)
pinned R0 -> R1 relation: 0->0;1->1;2->2
ParaAdjust CENTER read-back: OK
paragraph-text semantics: unchanged
```

Numeric tokens are diagnostic only. Together with duplicate-content and restart-scope evidence, these results prove that exact semantic restoration does not imply restoration of Writer object identity, content equality does not establish identity, and native tokens cannot outlive their live authority. ADR-0013 converts those facts into a product-owned durable identity/rebinding contract.

## Qualified UI-framework viability evidence

Slint 1.17.1 currently survives the first R0A executable viability gate while remaining quarantined under `spikes/ui-framework-slint/` on its own Rust 1.92 toolchain.

Current CI evidence:

```text
Ubuntu format/check/test/pedantic Clippy: OK
Windows check/test: OK
macOS check/test: OK
Linux Winit/software native window 1x: scale=1, physical=1100x800
Linux Winit/software native window 2x: scale=2, physical=2200x1600
caller-owned qualification raster: 262144 bytes at both scales
raster checksum: 6744427103266065219 at both scales
accessibility feature/explicit landmarks in candidate: enabled/compiled
ordinary Office Rust 1.85 + native LibreOffice gates: unchanged and green
```

The qualification also records real integration costs rather than hiding them: Linux X11 needs explicit fontconfig/XKB runtime dependencies; Slint generated Rust requires the isolated candidate crate to use `unsafe_code = "deny"` rather than product-wide `forbid`; and forced scale-factor changes require candidate recompilation because Slint compiler passes as well as Winit consume scale information.

This evidence establishes viability, **not framework selection**. ADR-0005 remains normative until real-platform IME, screen-reader/accessibility, desktop integration, viewport performance, licensing and toolchain evidence is sufficient.

## Immediate next engineering spikes

1. Continue UI framework qualification with real Windows/macOS/Linux IME/international-input and screen-reader/accessibility fixtures.
2. Make clipboard, drag/drop, native file-dialog/menu integration explicit and measure large viewport scroll/resize/zoom behavior.
3. Resolve UI packaging/licensing/MSRV costs and compare the strongest control alternative if any material Slint concern survives; then supersede ADR-0005 with an evidence-backed selection or explicit continuation decision.
4. Begin R0B implementation of the already-selected render data plane and recovery/identity architecture: bounded host-owned render buffers, durable checkpoint/journal/anchor storage and production worker-supervisor/UI recovery wiring.
5. Evolve richer normalized anchor hints only alongside the first real structured product surfaces; do not invent a speculative universal paragraph/table/field schema.
6. Add generated/property tests for larger feature graphs before external plugin loading work begins.
7. Define additive contribution registries only when the first real product feature needs them; do not invent a generic callback bus.

## Explicitly not started / deliberately unfrozen

- production UI framework integration;
- production Rust-to-LibreOffice FFI;
- production process-supervisor API and UI recovery surface;
- production rich paragraph/table/field anchor locator and persistence encoding;
- save-as/copy/fork policy for durable history lineages;
- durable Git-like transaction/history store;
- persisted crash-safe checkpoint/journal/anchor encoding and retention policy;
- final engine domain-message wire encoding;
- final cross-platform socket/pipe abstraction;
- request concurrency/cancellation policy;
- concrete shared-memory/mapped render-buffer backend and pool tuning;
- native document engine;
- collaboration;
- runtime loading of third-party plugins;
- WASM runtime selection;
- plugin package/marketplace design;
- hot feature reconfiguration while documents are active;
- spreadsheets/presentations.

## Current feature-system boundary

R0A resolves feature metadata and supervises trusted bundled feature lifecycle. It deliberately does not load arbitrary external code, expose speculative UI contribution schemas or grant OS capabilities. External extensions remain behind a future sandbox/capability host.

The kernel/feature boundary is defined in `docs/architecture/FEATURES_AND_EXTENSIONS.md`, lifecycle in `docs/architecture/FEATURE_HOST.md`, and the strategic decision in ADR-0006.

Bundled feature modularity is an ownership/dependency rule, not a requirement for runtime indirection. Trusted first-party modules may be statically composed/inlined while preserving feature/service contracts.

## Current engine-spike boundary

Stock LibreOfficeKit is qualified headlessly for Writer loading, layout, tile rendering, primitive mutation and DOCX round-tripping behind a killable process with explicit profile ownership.

Public view/accessibility APIs are not accepted as whole-document semantic authority. The pinned deeper bridge can reach the exact Writer document already owned by LOK, observe unsaved state and return bounded normalized semantics with revision freshness.

The internal ABI is version-specific and quarantined behind an unloadable compatibility module. The process worker owns bootstrap-runtime containment, including the measured process-global-finalizer reclamation rule for LibreOffice 24.2.

Live Writer paragraph object continuity is measured through interior split/merge, boundary insertion/deletion and formatting-only alignment mutation; duplicate-content ambiguity and reopen/full-worker-restart token scope are also qualified. The attempted public headless paragraph-reorder path is recorded as an explicit negative capability boundary rather than filled with unsafe private-symbol assumptions. These are reconciliation signals, never product identity; ADR-0013 now defines the product-owned identity and live-binding layer above them.

Native invalidation ordering/threading is also qualified as advisory render dirtiness beneath application-owned authority. The production native adapter remains deliberately unfrozen only where implementation details still genuinely remain open: versioning/supervisor packaging, durable storage integration and the eventual bootstrap-engine replacement strategy.

## Current protocol boundary

Request IDs, revisions and temporary text offsets use fixed-width integers. Transactions validate range/resource invariants before mutation.

`TextOffset` remains a narrow bootstrap value, not the future history/comment/collaboration anchor. Failed file-format identity experiments, qualified live-object evidence, duplicate-content ambiguity and restart scope all reinforce the same rule: incidental file-format, content or engine identities do not become product semantic authority by convenience.

`SemanticObservation<T>`, `AuthorityGeneration`, `DocumentRevision`, `SessionAuthorityStamp`, accepted-operation sequence numbers and product `LogicalAnchorId` values now cover distinct provenance/identity jobs. Recovery checkpoints bind authority plus an accepted-operation cursor and only publish replacement authority after complete contiguous replay. Durable anchors outlive that ephemeral authority and are rebound through product evidence; current native semantic/identity projections and qualification command/version bytes remain disposable codecs, not a frozen schema.

## Current transport and render-data boundary

`document-transport` owns bounded stream framing for opaque control bytes only. It proves request correlation, framing-version checks, payload admission, short-read/write behaviour and precise EOF/truncation semantics without selecting the final message serializer.

The frame concept is exercised across both the Cargo-built mock worker and native LibreOffice process. The native adapter additionally proves that bounded revision-stamped semantic bytes and qualification-only identity evidence can cross that seam without leaking engine implementation types.

Large render payloads are now architecturally separated from that control plane by ADR-0011: small authority/revision-tagged descriptors coordinate host-owned bounded reusable out-of-band raster buffers with scoped worker leases. R0B implements the OS-specific mapping/pool backend and tuning; it does not reopen the control/data-plane split.
