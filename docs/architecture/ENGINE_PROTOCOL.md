# Document Engine Protocol

Status: **R0A design target; value-type invariants are executable, wire encoding is not frozen**

## Purpose

The engine protocol is the long-lived seam between our product architecture and replaceable document engines. It must be smaller and more stable than any engine's native API.

## Protocol properties

- versioned;
- explicit fixed-width request/response IDs;
- every state-dependent response carries an engine revision;
- serialized values never depend on host pointer width (`usize` is forbidden at the boundary);
- operations are explicitly bounded before mutation;
- no engine implementation types cross the boundary;
- cancellation/timeouts are representable;
- diagnostics distinguish transport, engine, compatibility, validation and user errors;
- large render payloads may use shared memory or equivalent after the semantics are proven.

## Current executable value types

`document-protocol` intentionally contains dependency-light Rust value types only. R0A now includes:

- `RequestId(u64)` for future transport correlation;
- `DocumentRevision(u64)`;
- `TextOffset(u64)` for the narrow bootstrap text-fixture protocol;
- `ProtocolVersion`;
- capability values;
- text edits/transactions;
- explicit `TransactionLimits`;
- typed validation errors.

### Why `TextOffset` is fixed-width

The earlier R0A skeleton used `usize` for UTF-8 offsets. That is valid for an in-process prototype but invalid as a process/wire contract because its width depends on the host architecture.

`TextOffset(u64)` is therefore a protocol value. Engines convert it to native indices **only after** checking it against the current source text and UTF-8 character boundaries.

This does not make raw byte offsets our final user-facing document-position model. Stable semantic anchors remain a separate higher-level problem; see `TRANSACTIONS_AND_ANCHORS.md`.

## Transaction admission

Mutation must be all-or-nothing from the protocol layer's perspective.

Before the first edit is applied, the engine validates:

1. expected revision;
2. transaction edit-count limit;
3. per-replacement byte limit;
4. aggregate replacement-payload limit;
5. every range lies inside the source text;
6. every endpoint is a UTF-8 character boundary;
7. edit ranges do not overlap.

Limits are passed explicitly as `TransactionLimits`; they are not invisible global constants in `document-protocol`. The deterministic mock engine currently uses a documented R0A policy (4096 edits, 16 MiB per replacement, 64 MiB aggregate) purely to prove bounded admission. Production values require workload/compatibility qualification before protocol v1.

A failed validation leaves document contents and revision unchanged; this is regression-tested in the mock engine.

## Initial operation families

### Lifecycle

- `OpenDocument`
- `CloseDocument`
- `Checkpoint`
- `GetCapabilities`

### Semantic state

- `GetDocumentMetadata`
- `GetSemanticSnapshot`
- `GetOutline`
- `GetStyles`
- `GetComments`
- `GetRevisions`

### Mutation

- `ApplyTransaction`
- `Undo`
- `Redo`

Early convenience operations such as `InsertText` or `ApplyStyle` should compile into transactions rather than become unrelated mutation paths.

### Interaction/layout

- `HitTest`
- `GetSelectionGeometry`
- `GetCaretGeometry`
- `RenderRegion`
- `RenderThumbnail`

### File/output

- `Save`
- `SaveAs`
- `ExportPdf`
- `PrintDescription`

## Revision semantics

A mutation is submitted against a known revision:

```text
ApplyTransaction {
    expected_revision: 418,
    transaction: ...
}
```

A successful response produces 419 and describes invalidations/position mappings. If the engine is no longer at 418, it returns an explicit revision conflict rather than guessing.

Revision exhaustion semantics remain unresolved for protocol v1; R0A uses a `u64` revision counter and does not treat wraparound/saturation as a realistic working-state event.

## Compatibility capability negotiation

Different engines will support different feature sets. Capabilities are explicit, versioned data rather than `if libreoffice` branches throughout the product.

## Transport split

The logical protocol and byte transport are separate concerns:

```text
product operation
      |
      v
versioned protocol value/message
      |
      v
bounded control-frame transport
      |
      +---- small payload: inline bytes
      |
      `---- large render payload: future shared-memory descriptor
```

The next R0A transport slice should first prove bounded framing, request correlation, partial reads/writes, EOF/truncation behaviour and payload-class separation without selecting the final domain-message serialization format.

## R0A unresolved decisions

- domain-message wire representation (candidate evaluation: postcard/bincode-like custom format, Cap'n Proto, FlatBuffers, Protobuf; no choice merely for popularity);
- OS transport (local sockets/pipes initially; exact cross-platform abstraction not frozen);
- shared-memory mechanism for large rendering data;
- exact transaction algebra beyond the text-fixture subset;
- semantic snapshot granularity;
- stable object identity extraction from the bootstrap engine;
- cancellation representation and request lifecycle;
- protocol-v1 revision exhaustion semantics.

These require technical spikes before freezing protocol v1.
