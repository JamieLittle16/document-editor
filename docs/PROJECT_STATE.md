# Project State

Last updated: 2026-09-03

## Current phase

**R0A — architecture/contracts and high-risk spikes.**

## Accepted strategic decisions

- document editor first;
- eventual one-suite shell with separate editor modules/engines;
- Rust-led application architecture;
- LibreOffice Writer/LibreOfficeKit as quarantined bootstrap engine;
- heavyweight engine out of process;
- exactly one complete authoritative document model;
- strong documentation/ADR/debt discipline;
- UI framework deliberately not frozen until qualification spike;
- native/OpenDoc-style engine is a future migration candidate, not initial authority;
- minimal non-swappable correctness kernel surrounded by modular product features;
- bundled features use explicit feature/service contracts wherever practical;
- external plugins later reuse product contracts behind a capability-based sandbox rather than receiving engine access;
- trusted bundled feature lifecycle is supervised by a dedicated host rather than ad-hoc startup callbacks;
- LibreOfficeKit integration is qualified outside the Rust workspace before any unsafe/native adapter contract is frozen;
- process/wire protocol values use fixed-width types rather than host-width `usize` values;
- transaction resource limits and validation are explicit admission policy rather than implicit implementation behaviour;
- process framing is a separate bounded control-plane layer and does not select the permanent document-message serializer;
- large render payloads are not forced through inline control frames merely because a frame codec exists;
- worker EOF and worker exit status are separate evidence: stream closure alone is never treated as proof of graceful engine completion;
- process restartability is qualified before document-session recovery semantics are designed;
- file-format IDs and engine object addresses are evidence inputs, never product semantic identities by default;
- semantic identity must be qualified through edits and reload before an anchor model is frozen;
- public view/accessibility APIs are not promoted into semantic document APIs unless whole-document behaviour is directly qualified;
- deeper native semantics must operate on the same authoritative Writer instance rather than silently creating a second document authority;
- same-instance Writer semantic access is qualified for the pinned LibreOffice 24.2.7.2 environment, but its internal process-context ABI is qualification machinery rather than a production API;
- the same-instance semantic dependency is isolated in a version-labelled native translation unit behind a native-neutral PIMPL surface;
- live semantic observations crossing the process boundary must be bounded, normalized and implementation-neutral rather than serialized UNO objects;
- production adoption of any LibreOffice-internal semantic bridge requires explicit versioning and an ADR rather than a project-wide wrapper grown from a spike.

## Implemented in repository skeleton

- Rust workspace;
- strongly typed protocol revision and transaction primitives;
- replaceable `DocumentEngine` trait;
- deterministic mock engine;
- revision-conflict test coverage;
- UI-agnostic document session;
- desktop and document-worker harnesses;
- CI quality gates;
- initial product/architecture/engineering documentation;
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
- executable architecture dependency guard in CI;
- ADR-0006 plus normative feature/extension and feature-host documentation;
- standalone LibreOfficeKit qualification probe outside the Rust workspace;
- deterministic source-generated DOCX probe fixture;
- stock Ubuntu 24.04 LibreOfficeKit open/layout/render/save/reopen qualification;
- isolated LibreOffice user-profile URI in qualification;
- caller-owned BGRA/RGBA tile-buffer qualification;
- DOCX round-trip structural validation;
- real UTF-8 text mutation persistence through DOCX save/reopen and OOXML semantic validation;
- fixed-width `RequestId(u64)`, `DocumentRevision(u64)` and `TextOffset(u64)` protocol primitives;
- explicit `TransactionLimits` and pre-mutation validation for edit count, replacement bytes, UTF-8 boundaries and overlap;
- regression coverage proving rejected multi-edit transactions leave document state/revision untouched;
- std-only `document-transport` control-frame crate;
- fixed 20-byte versioned frame header with request/response role and `RequestId` correlation;
- explicit `FrameLimits` admission on both reader and writer;
- payload-size rejection from the header before payload allocation/read;
- typed clean-EOF versus truncated-header/truncated-payload semantics;
- tests for short reads/writes and malformed magic/version/kind/flags;
- dedicated `ENGINE_TRANSPORT.md` architecture contract keeping framing separate from domain encoding and shared-memory policy;
- real Cargo-built `document-worker` child-process qualification over stdin/stdout using `document-transport`;
- deliberately disposable `R0A_*` command codec isolated inside the worker spike;
- worker-loop tests for request correlation, invalid commands, wrong frame role and shutdown ordering;
- real child-process tests for 64-bit request-ID preservation, graceful shutdown and clean stdin EOF;
- forced worker-death test requiring a non-success process status plus observed stdout EOF;
- fresh-worker restart test after forced child death;
- dedicated `DOCUMENT_WORKER_PROCESS_SPIKE.md` documenting what process behaviour is proven and what remains deliberately unfrozen;
- standalone C++ LibreOfficeKit process adapter outside the Rust workspace;
- the native adapter independently exercises the bounded `DETR` frame envelope without exposing LibreOffice types to Rust;
- real LibreOffice private-profile ownership qualification with an isolated process HOME;
- typed real-engine load failure and invalid-command handling;
- graceful real-engine shutdown plus forced death while a Writer document is live;
- fresh LibreOffice process restart and document reopen after forced death;
- deterministic three-paragraph semantic identity fixture with seeded `w14:paraId` and `w14:textId` candidate values;
- minimal OOXML semantic paragraph projection for qualification;
- CI assertions for paragraph cardinality, order, text preservation and edit locality across real LibreOffice edit/save/reopen;
- recorded rejection of DOCX `w14:paraId` / `w14:textId` as product identity candidates after LibreOffice removed all seeded values on the qualified save path;
- public-LOK live-semantic discovery showing that accessibility focus did not provide deterministic paragraph traversal and `SelectAll + getTextSelection()` returned no whole-document text in the headless adapter configuration;
- removal of those unproven live-semantic commands from the mandatory native adapter qualification surface;
- version-pinned same-process Writer semantic access isolated in `writer_semantics_24_2.cxx`, with no UNO/internal LibreOffice types in its native-neutral header;
- acquisition of exactly one Writer `XTextDocument` from the already-running LibreOfficeKit process rather than bootstrapping another office;
- semantic enumeration of the three fixture paragraphs from that same live Writer model;
- a disposable versioned semantic projection containing ordered UTF-8 paragraphs only;
- explicit 1024-byte process-frame bound on the complete semantic snapshot, with typed limit rejection instead of an unbounded response path;
- process-harness proof that the snapshot exactly matches the three fixture paragraphs before edit;
- process-harness proof that the retained same-instance semantic view observes an unsaved LOK prefix mutation in paragraph 1 while paragraphs 2-3 remain unchanged;
- semantic view lifetime tied to the open Writer document, with semantic requests rejected after close;
- forced process death while Writer and semantic state are live, followed by fresh process restart/reopen and successful semantic snapshot reacquisition;
- removal of the now-redundant standalone UNO bridge probe after its capability was integrated into the native process adapter;
- dedicated `SEMANTIC_IDENTITY_SPIKE.md` documenting rejected identity/snapshot candidates, the successful same-instance bounded semantic seam, and the remaining identity/reconciliation frontier.

## Qualified LibreOfficeKit reference environment

Reference open/render/edit/save/reopen environment:

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

Raster hashes and round-trip package byte counts have changed when the deterministic fixture changed and are intentionally **not** semantic goldens.

## Qualified worker-process behaviour

R0A process behaviour on the Rust CI gate:

```text
bounded frame codec: OK
real Cargo-built child process: OK
64-bit request correlation across OS pipes: OK
graceful shutdown response + successful exit: OK
clean stdin EOF + successful exit: OK
forced child death + non-success exit: OK
stdout EOF after forced death: OK
fresh child restart after forced death: OK
workspace check/tests/Clippy: OK
```

This proves process restartability and failure observation. It does **not** yet prove recovery of an open document/session after engine loss.

## Qualified native LibreOffice process behaviour

The standalone native adapter qualification covers the stock LibreOffice reference environment:

```text
cross-language DETR framing: OK
private LibreOffice profile: OK
typed missing-document load failure: OK
invalid command without engine crash: OK
graceful engine-process exit: OK
forced exit with Writer document open: observed
fresh engine-process restart: OK
same fixture reopened after restart: OK
```

The adapter also contains the bounded same-instance semantic qualification described below. The R0A native command payload remains disposable and is not the final domain-message protocol.

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

Therefore DOCX `w14:paraId` and `w14:textId` are **rejected as authoritative product semantic identities**. CI does not require LibreOffice to continue stripping them in future versions; the permanent assertion is semantic paragraph preservation for this fixture, not a serialization quirk.

Two public LibreOfficeKit-only attempts to obtain a live whole-document semantic/text snapshot were also rejected by qualification:

```text
getA11yFocusedParagraph + GoToNextPara: first paragraph only
SelectAll + getTextSelection: empty selected text
```

Those failures established that view/accessibility APIs are not the semantic document seam in this headless configuration.

The deeper path is integrated into the native process adapter. It loads the fixture once through LibreOfficeKit, obtains exactly one Writer `XTextDocument` from that same process, retains an opaque native semantic view, and returns only a bounded native-neutral paragraph projection over the existing `DETR` frame:

```text
projection version: 1
payload bound: 1024 bytes
snapshot before edit: exact 3 fixture paragraphs
unsaved LOK prefix edit: visible only in paragraph 1
paragraphs 2-3 after edit: unchanged
semantic access after close: rejected
fresh process restart/reopen: semantic snapshot reacquired
```

This proves **live same-authority semantic observation across the isolated process boundary** in the pinned reference environment. It does not prove stable product paragraph/object identity.

The qualification currently reaches the process context through LibreOffice's internal `comphelper::getProcessComponentContext()` ABI. Its exact 24.2 signature is confined to `writer_semantics_24_2.cxx`; the header seen by the rest of the native spike contains no UNO types. The mechanism is intentionally not a production dependency or cross-process contract.

## Immediate next engineering spikes

1. Add explicit **document/revision context** to semantic observations before host-side semantic caching becomes real. The revision tag must describe authority freshness, not imply that the temporary R0A snapshot encoding is permanent.
2. Using the retained same-instance semantic view, exercise deterministic **insertion, deletion, split, merge, move and formatting-only** edit sequences and measure candidate paragraph/object identity signals without freezing a product `ParagraphId`.
3. Determine which observed engine-side properties, if any, are stable enough to be evidence inputs versus which require an adapter reconciliation layer.
4. Requalify identity/reconciliation across **save/reload** separately from live-instance behaviour; DOCX `w14` IDs remain ruled out as authority.
5. Exercise LibreOfficeKit callbacks/invalidation and map their ordering, threading and coalescing behaviour.
6. Relate callback/invalidation events to semantic revisions so the host can know when cached semantic/render state is stale.
7. Measure tile/render payload patterns to decide copy versus shared memory and batching thresholds.
8. Build the first compatibility fixture runner around normalized semantic assertions rather than binary-package equality.
9. Run UI framework qualification (Slint leading candidate, alternatives measured).
10. Add generated/property tests for larger feature graphs before external plugin loading work begins.
11. Define additive contribution registries only when the first real product feature needs commands/panels/diagnostics; do not invent a generic callback bus.
12. Write the production native-adapter/unsafe-boundary ADR only after semantic identity and callback measurements constrain the design, including the versioning strategy for any internal LibreOffice bridge.
13. Design document-session recovery only after live semantic identity and restart reconciliation evidence exist.

## Explicitly not started

- production UI;
- production Rust-to-LibreOffice FFI;
- production process-supervisor API;
- production stable paragraph/object identity;
- production semantic anchor model;
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

R0A now resolves feature metadata **and** supervises trusted bundled feature lifecycle. It deliberately does not load external code, expose UI contribution schemas or grant OS capabilities. External extensions remain behind a future sandbox host.

The kernel/feature boundary is defined in `docs/architecture/FEATURES_AND_EXTENSIONS.md`, the lifecycle contract in `docs/architecture/FEATURE_HOST.md`, and the strategic decision in ADR-0006.

## Current engine-spike boundary

R0A has proved that stock LibreOfficeKit can be used headlessly for Writer document loading, layout, tile rendering, primitive text mutation and DOCX round-tripping without exposing LibreOffice types to product Rust code. It has also proved that real LibreOffice state can be contained in a killable/restartable native process with explicit profile ownership.

Public LOK view/accessibility APIs have **not** proved sufficient for whole-document live semantics. That negative result remains part of the boundary: the production adapter must not fake semantic enumeration through caret movement, accessibility focus or selection side effects.

The deeper same-instance question and the first process-boundary projection question are now resolved for the pinned R0A environment: a native semantic layer can reach the exact Writer document already owned by LOK, observe an unsaved LOK edit, and return normalized ordered paragraph text through a hard-bounded native-neutral response. The internal process-context ABI remains isolated and version-specific.

The production native adapter remains deliberately unfrozen while **stable identity/reconciliation, revision freshness, callback behaviour and the internal-ABI versioning strategy** are still being measured.

## Current protocol boundary

The protocol value layer is explicitly process-safe at the primitive level: request IDs, revisions and temporary text offsets use fixed-width integers, and transactions validate resource/range invariants before mutation. `TextOffset` is a narrow bootstrap value only; it is not the future semantic anchor model used by history/comments/collaboration.

The failed `w14` identity experiment strengthens that rule: neither temporary offsets nor incidental file-format IDs are accepted as semantic authority. Likewise, successful access to UNO references does not make UNO object identity part of the product protocol.

The current semantic projection version byte and paragraph encoding are **qualification codec**, not a frozen product schema.

## Current transport boundary

`document-transport` owns only bounded stream framing for opaque control bytes. It proves request correlation, framing-version checks, payload admission, short-read/write behaviour and precise EOF/truncation semantics without choosing the final message serializer or OS process channel.

That framing concept has now been exercised across both the Cargo-built mock worker and the standalone native LibreOffice process adapter. The native adapter additionally proves that normalized live semantic bytes can traverse the same bounded control seam without leaking engine implementation types.

The next boundary to qualify is **semantic identity/reconciliation under structural change**, not another transport or acquisition mechanism.
