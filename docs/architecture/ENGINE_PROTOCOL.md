# Document Engine Protocol

Status: **R0A design target**

## Purpose

The engine protocol is the long-lived seam between our product architecture and replaceable document engines. It must be smaller and more stable than any engine's native API.

## Protocol properties

- versioned;
- explicit request/response IDs;
- every state-dependent response carries an engine revision;
- operations are bounded and serialisable;
- no engine implementation types cross the boundary;
- cancellation/timeouts are representable;
- diagnostics distinguish transport, engine, compatibility, validation and user errors;
- large render payloads may use shared memory or equivalent after the semantics are proven.

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

## Compatibility capability negotiation

Different engines will support different feature sets. Capabilities are explicit, versioned data rather than `if libreoffice` branches throughout the product.

## R0A unresolved decisions

- wire representation (candidate evaluation: postcard/bincode-like custom format, Cap'n Proto, FlatBuffers, Protobuf; no choice merely for popularity);
- transport (local sockets/pipes initially; shared memory for large rendering data if profiling justifies it);
- exact transaction algebra;
- semantic snapshot granularity;
- stable object identity extraction from the bootstrap engine.

These require technical spikes before freezing protocol v1.
