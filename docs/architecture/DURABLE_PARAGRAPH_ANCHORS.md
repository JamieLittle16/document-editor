# Durable Paragraph Anchors

Status: **R0A product identity foundation**

Normative decision: [ADR-0013](../decisions/ADR-0013-product-owned-durable-paragraph-anchors.md)

## Purpose

Office needs paragraph references that remain meaningful when the bootstrap engine replaces native objects, a worker restarts, or a saved document is reopened. Those references will eventually support history, durable selections, comments, diagnostics and collaboration.

The identity belongs to Office, not to Writer.

## Identity

A paragraph anchor is the pair:

```text
DocumentLineageId + ParagraphAnchorSequence
```

`DocumentLineageId` identifies one product-owned logical document lineage. `ParagraphAnchorSequence` is monotonic within that lineage. Retired sequences are never reused.

Neither field may be derived from:

- UNO references or object addresses;
- qualification probe tokens;
- DOCX/OOXML IDs;
- file paths;
- package hashes;
- paragraph text or text hashes;
- engine-local revision numbers.

## Structural continuity policy

Office defines continuity from semantic operations rather than copying Writer's object-lifetime behavior.

| Operation | Product anchor rule |
| --- | --- |
| content/format mutation of one paragraph | preserve its anchor |
| insert paragraph | mint a new anchor |
| delete paragraph | retire its anchor |
| split paragraph | preserve original anchor on left; mint right |
| merge adjacent paragraphs | preserve left; retire right |

The split rule is intentionally allowed to disagree with Writer's native same-object evidence. Writer identity can strengthen reconciliation evidence inside one live authority, but it cannot override product identity.

## Persistence artifact

`ParagraphAnchorSnapshot` is a bounded, versioned artifact containing:

- format magic/version/flags;
- product document lineage;
- next mint sequence;
- ordered live paragraph anchors;
- ordered semantic paragraph text as verification evidence.

The codec uses fixed-width little-endian fields and explicit limits for paragraph count, per-paragraph bytes and total semantic bytes. Malformed, truncated, oversized, duplicate-sequence and trailing-byte artifacts are rejected.

The snapshot stores the next mint sequence so deleted/merged anchors cannot be recycled after reload.

## Reconciliation rule

The initial persisted rebind path is deliberately conservative.

An exact rebind succeeds only when:

1. the caller supplies the same product document lineage;
2. paragraph cardinality is unchanged;
3. ordered semantic paragraph text exactly matches the stored snapshot evidence.

On mismatch, Office returns an unresolved reconciliation result. It does **not** search by text, fuzzy-match, reorder candidates or mint replacements behind the caller's back.

This is sufficient for a known-lineage checkpoint/save artifact whose semantic state has already been qualified as preserved. Independently modified external files are a different problem and require later structural/history reconciliation.

## Layering

`document-anchors` has no internal repository dependencies. The architecture guard enforces that boundary.

The crate therefore knows nothing about:

- LibreOffice or another document engine;
- process transport;
- UI frameworks;
- DOCX parsing;
- session authority generations;
- filesystem placement or atomic-write policy.

Persistence/application code supplies lineage identity and stores snapshots. Session/recovery code owns ephemeral authority. History code will consume anchors later. Keeping these roles separate is what allows the bootstrap engine to be replaced without rewriting product identity.

## Current acceptance evidence

Public contract tests prove:

- duplicate paragraphs receive distinct anchors;
- ordinary semantic change preserves a paragraph anchor;
- split then merge restores the original live anchor set under Office's policy;
- deleted/merged anchor sequences are not reused after snapshot encode/decode/rebind;
- a snapshot survives an explicit filesystem write/read checkpoint round trip;
- an unchanged same-lineage semantic projection rebinds to the same anchors;
- reordered, externally changed or foreign-lineage projections are rejected rather than guessed;
- bounded decoding rejects malformed and oversized artifacts.

## Deferred intentionally

This foundation does not yet define:

- an on-disk sidecar location or atomic persistence protocol;
- persistent history DAG encoding;
- below-paragraph anchors;
- external-file fuzzy reconciliation;
- collaboration/CRDT/OT semantics;
- lineage-ID generation policy;
- structural command adapters for every edit type.

Those can evolve above the stable rule that logical identity is product-owned and engine-independent.
