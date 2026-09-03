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
- same-instance Writer semantic access is now qualified for the pinned LibreOffice 24.2.7.2 environment, but its internal process-context ABI is qualification machinery rather than a production API;
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
- version-pinned native UNO qualification that acquires exactly one Writer `XTextDocument` from the already-running LibreOfficeKit process rather than bootstrapping another office;
- semantic enumeration of the three fixture paragraphs from that same live Writer model;
- proof that the same retained UNO `XTextDocument` observes an unsaved prefix mutation made through the original LibreOfficeKit `Document` while preserving paragraph count/order and unrelated paragraph text;
- same-instance Writer semantic qualification promoted from non-gating discovery to a required LibreOffice CI step;
- the LibreOffice 24.2 `comphelper::getProcessComponentContext()` declaration remains local to the qualification probe because its ABI is internal and version-specific;
- dedicated `SEMANTIC_IDENTITY_SPIKE.md` documenting rejected identity/snapshot candidates, the successful same-instance bridge, and the remaining identity/reconciliation frontier.

## Qualified LibreOfficeKit reference environment

Green open/render/edit/save/reopen run:

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

Green R0A process run on the Rust CI gate:

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

The standalone native adapter has been qualified against the same stock LibreOffice environment:

```text
cross-language DETR framing: OK
private LibreOffice profile: OK
profile files created: 35
typed missing-document load failure: OK
invalid command without engine crash: OK
graceful engine-process exit: OK
forced exit with Writer document open: observed
fresh engine-process restart: OK
same fixture reopened after restart: OK
Writer size before crash: 12474 x 17406 TWIPs
Writer size after restart: 12474 x 17406 TWIPs
```

This is process-boundary evidence. The R0A native command payload is disposable and is not the final domain-message protocol.

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

These failures do not weaken the already-green engine/process boundary. They show that the tested public LOK view/accessibility APIs are not sufficient evidence for deterministic whole-document semantic extraction in this headless configuration.

A deeper same-instance native qualification has now succeeded. The probe loads the fixture once through LibreOfficeKit, obtains exactly one Writer `XTextDocument` from that same process, enumerates its three paragraphs, performs an unsaved prefix edit through the original LOK `Document`, and requires the same retained UNO reference to observe exactly that edit:

```text
same process component context: OK
Writer XTextDocument count: 1
paragraphs before edit: 3
paragraphs after edit: 3
same retained UNO reference sees unsaved LOK edit: OK
same-instance bridge: OK
```

This proves **live same-authority semantic access** in the pinned reference environment. It does not prove stable product paragraph/object identity. The qualification currently uses LibreOffice's internal `comphelper::getProcessComponentContext()` ABI, whose exact 24.2 signature is declared only inside the spike. That mechanism is intentionally not a production dependency or cross-process contract.

## Immediate next engineering spikes

1. Use the proven same-instance Writer access to return the smallest useful **bounded native-neutral semantic snapshot** across the isolated engine boundary. Start with ordered paragraph text plus only structural metadata needed by the next experiment; do not mirror UNO objects.
2. Tag live semantic snapshots with explicit document/revision context so host-side caches cannot treat them as timeless state.
3. Exercise insertion, deletion, split, merge, move and formatting-only edit sequences and measure candidate paragraph/object identity signals without freezing a product `ParagraphId`.
4. Requalify identity/reconciliation across save/reload separately from live-instance behaviour; the failed DOCX `w14` candidates remain ruled out as authority.
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

The deeper same-instance question is now resolved for the pinned R0A reference environment: a native semantic layer can reach the exact Writer document already owned by LOK and observe unsaved mutations on it. The mechanism currently touches a LibreOffice-internal, version-specific process-context ABI and therefore remains qualification-only.

The production native adapter remains deliberately unfrozen while bounded semantic projection, stable identity, callback behaviour and the internal-ABI versioning strategy are still being measured.

## Current protocol boundary

The protocol value layer is explicitly process-safe at the primitive level: request IDs, revisions and temporary text offsets use fixed-width integers, and transactions validate resource/range invariants before mutation. `TextOffset` is a narrow bootstrap value only; it is not the future semantic anchor model used by history/comments/collaboration.

The failed `w14` identity experiment strengthens that rule: neither temporary offsets nor incidental file-format IDs are accepted as semantic authority. Likewise, successful access to UNO references does not make UNO object identity part of the product protocol.

## Current transport boundary

`document-transport` owns only bounded stream framing for opaque control bytes. It proves request correlation, framing-version checks, payload admission, short-read/write behaviour and precise EOF/truncation semantics without choosing the final message serializer or OS process channel.

That framing concept has now been exercised across both the Cargo-built mock worker and the standalone native LibreOffice process adapter. Their disposable R0A command codecs prove process lifecycle and qualification behaviour only; they must not become the product domain protocol by inertia.

The next boundary to qualify is **bounded same-instance semantic projection**: normalized Writer structure must cross the isolated engine boundary without exposing LibreOffice implementation types, creating a second document authority or prematurely defining stable product identity.
