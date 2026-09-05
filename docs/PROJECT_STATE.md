# Project State

Last updated: 2026-09-05

## Current phase

**R0A — architecture/contracts and high-risk spikes.**

The modular feature kernel, bounded worker/process foundation, semantic-observation freshness contract, pinned LibreOffice same-authority semantic/lifecycle qualification, and two structural identity sequences are now established.

Interior split/merge and paragraph-boundary insertion/deletion both prove that live Writer object identity is useful continuity evidence but **not durable logical identity**. In both cases a semantic `R0 -> R1 -> R2` round trip restores exact paragraph text while replacing the original first-paragraph Writer object. The next identity frontier is move/reorder, formatting-only edits, duplicate-text ambiguity and save/reload/restart reconciliation before durable history anchors or recovery semantics are frozen.

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
- semantic identity must be qualified through structural edits and reload before a durable anchor model is frozen;
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
- product-owned reconciliation is required even for simple paragraph-boundary insertion/deletion because exact semantic restoration can still leave different Writer objects.

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
- executable architecture dependency guard;
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
- live native qualification proves `R0 -> mutation -> R1` and fresh process/reopen -> new `R0`.

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

### Structural paragraph identity qualification

- qualification-only identity-probe projection using view-local tokens assigned by UNO same-object equality;
- probe-token uniqueness and repeatability asserted before structural evidence is interpreted;
- deterministic first-paragraph interior split at character offset `8` with exact semantic and revision assertions;
- deterministic merge with exact semantic restoration and revision assertions;
- twice-reproduced pinned interior split/merge relation showing the original first object is destroyed by the semantic round trip;
- deterministic paragraph-end boundary split reuses the same native primitive to insert an empty adjacent paragraph without a new ABI/wire command;
- twice-reproduced boundary insertion/deletion sequence restores exact `(P0, P1, P2)` semantics at `R2` while reproducing the same non-invertible object relation;
- untouched later paragraphs retain object continuity in both measured sequences;
- separate CI contracts pin structural relations without pinning numeric probe tokens or addresses;
- `STRUCTURAL_IDENTITY_QUALIFICATION.md` records the evidence and its history/reconciliation consequence.

### Version-pinned semantic module and native reclamation

- `writer_semantics_24_2.cxx` is an unloadable version-pinned compatibility module containing UNO/internal LibreOffice dependencies;
- `writer_semantics_module_abi.hxx` defines a tiny qualification-only C ABI with no UNO types;
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

This proves process restartability and failure observation. It does not yet prove recovery of an open logical document/session after engine loss.

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

Numeric tokens are diagnostic only. The shared relation proves that exact semantic restoration does not imply restoration of Writer object identity, even for empty-paragraph insertion/deletion.

## Immediate next engineering spikes

1. Qualify move/reorder while preserving distinct semantic content and neighbourhood evidence.
2. Qualify formatting-only edits and determine whether paragraph object continuity changes when text/structure do not.
3. Add duplicate-text fixtures so reconciliation cannot accidentally depend on content equality.
4. Requalify semantic identity/reconciliation across **save/reload** and worker restart separately from live-instance behaviour.
5. Exercise LibreOfficeKit callbacks/invalidation and map ordering, threading and coalescing behaviour.
6. Relate callbacks/invalidation to semantic revisions so host caches can be invalidated safely.
7. Measure tile/render payload patterns to decide copy/shared-memory and batching thresholds.
8. Build the first compatibility fixture runner around normalized semantic assertions rather than binary-package equality.
9. Run UI framework qualification (Slint remains a leading candidate; selection must remain evidence-driven).
10. Add generated/property tests for larger feature graphs before external plugin loading work begins.
11. Define additive contribution registries only when the first real product feature needs them; do not invent a generic callback bus.
12. Write the production native-adapter/supervisor/unsafe-boundary ADR only after the remaining identity and callback measurements constrain the design.
13. Design durable history anchors, restart reconciliation and document-session recovery only after the structural identity/reload evidence is sufficient to define product-owned identity semantics.

## Explicitly not started / deliberately unfrozen

- production UI;
- production Rust-to-LibreOffice FFI;
- production process-supervisor API;
- production stable paragraph/object identity;
- production semantic anchor model;
- durable Git-like transaction/history store;
- final engine domain-message wire encoding;
- final cross-platform socket/pipe abstraction;
- request concurrency/cancellation policy;
- shared-memory render transport;
- document-session recovery after a real engine crash;
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

Live Writer paragraph object continuity is now measured through interior split/merge and boundary insertion/deletion. Both are useful reconciliation evidence but explicitly not product identity because exact semantic round trips can replace an engine object.

The production native adapter remains deliberately unfrozen while **move/reorder, formatting-only, duplicate-text, reload/restart identity, callback behaviour and production versioning/supervisor policy** are still being measured.

## Current protocol boundary

Request IDs, revisions and temporary text offsets use fixed-width integers. Transactions validate range/resource invariants before mutation.

`TextOffset` remains a narrow bootstrap value, not the future history/comment/collaboration anchor. Failed `w14` identity experiments, successful same-authority UNO access and two non-invertible structural identity sequences all reinforce the same rule: incidental file-format or engine identities do not become product semantic authority by convenience.

`SemanticObservation<T>` and `DocumentRevision` are product-facing freshness concepts. The current native semantic/identity projections and version bytes are qualification codec, not a frozen schema.

## Current transport boundary

`document-transport` owns bounded stream framing for opaque control bytes only. It proves request correlation, framing-version checks, payload admission, short-read/write behaviour and precise EOF/truncation semantics without selecting the final message serializer or shared-memory policy.

The frame concept is exercised across both the Cargo-built mock worker and native LibreOffice process. The native adapter additionally proves that bounded revision-stamped semantic bytes and qualification-only identity evidence can cross that seam without leaking engine implementation types.

The next boundary to qualify is **reconciliation under move/reorder and non-structural formatting, then reload/restart**, not another transport or acquisition mechanism.
