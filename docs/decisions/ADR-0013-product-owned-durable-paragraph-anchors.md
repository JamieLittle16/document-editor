# ADR-0013: Product-owned durable paragraph anchors

Status: Accepted for the first R0A durable-anchor slice

Date: 2026-09-06

## Context

R0A structural identity qualification established that LibreOffice Writer object identity is useful evidence only inside one retained live authority:

- interior split/merge and paragraph-boundary insertion/deletion can restore exact paragraph text while replacing the original Writer paragraph object;
- formatting-only mutation can preserve the live Writer paragraph objects while still advancing semantic revision;
- duplicate paragraphs can carry identical text while remaining distinct objects;
- view-local native identity tokens can reuse the same numeric values after close/reopen and full worker restart;
- file-format paragraph IDs and text equality are therefore insufficient as durable product identities.

Office nevertheless needs durable logical references for history, selections, comments, diagnostics and eventually collaboration. Those references must survive worker replacement and save/reload without turning bootstrap-engine implementation details into product architecture.

The application-level recovery work already separates ephemeral authority from accepted-operation lineage. The remaining identity question is therefore narrower: what is the smallest durable paragraph identity model we can safely own now?

## Decision

Introduce a dependency-free `document-anchors` crate containing product-owned document lineage and paragraph-anchor primitives.

A paragraph anchor is:

```text
DocumentLineageId + ParagraphAnchorSequence
```

`DocumentLineageId` is a 128-bit product-owned durable value supplied and persisted by the application/persistence layer. It is never derived from a path, package hash, Writer object, UNO reference, native probe token, semantic text or file-format ID.

`ParagraphAnchorSequence` is monotonic inside one lineage. Sequence zero is reserved. Retired sequences are never reused, including after snapshot reload.

The initial structural continuity policy is product-defined:

- ordinary semantic/content or formatting changes preserve the paragraph anchor;
- insertion mints a new paragraph anchor;
- deletion retires the deleted anchor;
- split preserves the original anchor on the **left** fragment and mints a fresh right-fragment anchor;
- merge preserves the left anchor and retires the right anchor.

This policy intentionally does not mirror Writer's observed split object-survival behavior. Native identity remains evidence, not authority.

## Durable snapshot artifact

The crate provides a bounded, versioned binary `ParagraphAnchorSnapshot` artifact containing:

- format magic/version/flags;
- `DocumentLineageId`;
- next paragraph-anchor sequence;
- ordered live paragraph anchors;
- ordered semantic paragraph text as reconciliation evidence.

All integer fields use fixed-width little-endian encoding. Decoding and encoding require explicit paragraph, per-paragraph byte and total semantic-byte limits.

Semantic text in this artifact is **evidence, not identity**. The exact-rebind path never searches for a paragraph by text and never hashes text into an ID.

## Conservative rebind rule

The first durable rebind operation is intentionally strict.

A persisted snapshot may be rebound only when:

1. the caller supplies the same expected product document lineage;
2. paragraph cardinality matches exactly;
3. the ordered semantic paragraph projection matches exactly.

If any of those conditions fails, rebind returns an unresolved error. It does not guess, fuzzy-match, reorder candidates or silently mint replacement identities.

This exact path is for known-lineage save/reload/checkpoint artifacts. Independently modified external files require a later structural/history reconciliation policy and must not be treated as equivalent merely because some text happens to match.

## Architecture boundary

`document-anchors` has no internal repository dependencies and the architecture guard enforces that fact.

The crate must not depend on:

- LibreOffice/UNO or any engine implementation;
- process transport;
- UI toolkit types;
- DOCX/OOXML identifiers;
- application session authority types;
- persistence filesystem policy.

Application/history/recovery layers may consume these product-owned anchors later. The identity primitive itself stays below those consumers.

## Consequences

### Positive

- durable paragraph identity is now explicitly product-owned;
- worker restart and engine replacement cannot change logical paragraph IDs by themselves;
- split/merge behavior is deterministic even when Writer object behavior differs;
- duplicate paragraph content no longer creates an identity collision;
- retired IDs remain retired across persisted snapshot reload;
- malformed or oversized snapshot artifacts are rejected before unbounded allocation;
- richer reconciliation can be added later without changing the identity key format.

### Costs

- the application/persistence layer must eventually mint and durably store `DocumentLineageId` values;
- generic byte-offset transactions cannot automatically update structural paragraph anchors safely; semantic command adapters are required before those operations can mutate the anchor table in production;
- exact rebind deliberately fails on changed projections, so external-modification reconciliation remains future work.

## Non-goals

This ADR does **not** select:

- a persistent history DAG/commit format;
- collaboration/CRDT/OT identity semantics;
- external-file fuzzy reconciliation;
- a filesystem/sidecar location for anchor snapshots;
- random/UUID generation implementation for `DocumentLineageId`;
- semantic anchors below paragraph granularity;
- a permanent encoding for every future document object type.

Those decisions remain above or beyond this minimal durable paragraph-identity foundation.
