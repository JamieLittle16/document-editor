# R0A Live Semantic Revision Qualification

Status: qualification evidence, not a permanent process-wire schema.

## Purpose

ADR-0007 establishes the product-side rule that semantic observations are not timeless values: they carry the exact `DocumentRevision` from which they were read, and retained observations must be checked against the current document authority before they can affect present UI or application state.

The mock/session tests prove that contract in Rust. This qualification proves the corresponding freshness signal against the real, isolated LibreOfficeKit Writer process used by the R0A native adapter spike.

The goal is deliberately narrow. It does **not** define paragraph identity, history anchors, collaboration identities, a global document identifier or the final engine wire protocol.

## Native qualification revision

For one open native Writer document, the R0A adapter owns a `u64` qualification revision with these rules:

1. successful document open starts at revision `0`;
2. a successful authoritative mutation advances the revision exactly once;
3. a rejected mutation must not advance it;
4. a semantic snapshot is stamped with the revision describing the state it returns;
5. close removes semantic access and discards the open-document revision;
6. a fresh open/reopen starts a fresh revision sequence at `0`.

The adapter is a single-command-loop process, so the revision stamp and semantic read are serialized through the same authority. This is qualification evidence for the future engine adapter; it is not permission to assume revision numbers are globally unique across documents or sessions.

## Temporary projection version 2

The disposable native semantic projection is versioned rather than silently changing the meaning of version 1.

Successful payload:

```text
status:u8 = OK
command:u8 = SEMANTIC_SNAPSHOT
projection_version:u8 = 2
revision:u64-le
paragraph_count:u16-le
repeat paragraph_count times:
    byte_length:u16-le
    utf8_text[byte_length]
```

The complete payload remains inside the existing 1024-byte `DETR` control-frame bound. Adding revision provenance therefore reduces, rather than expands, the remaining paragraph-byte budget.

This codec remains qualification-only. Product crates consume `SemanticObservation<T>` from `document-engine-api`; they do not depend on these native spike bytes.

## CI evidence

The process harness requires the following deterministic sequence against the pinned LibreOffice reference environment:

```text
open fixture
semantic snapshot -> revision 0 + exact fixture paragraphs
insert unsaved prefix through LibreOfficeKit
semantic snapshot -> revision 1 + exact edited paragraphs
close
semantic snapshot -> typed engine-state rejection
kill/restart process
reopen fixture
semantic snapshot -> revision 0 + original fixture paragraphs
```

The harness also retains the previous bounds, lifecycle, typed-load-failure, invalid-command and forced-restart qualifications.

This links three independent facts:

- the semantic view observes the same live Writer authority as LibreOfficeKit;
- the semantic bytes crossing the process boundary are bounded and engine-neutral;
- those bytes now carry explicit authority freshness rather than being treated as timeless state.

## Relationship to product revision semantics

The native qualification revision and Rust `DocumentRevision` express the same invariant but are not yet one frozen end-to-end protocol field.

R0A intentionally keeps that distinction visible:

- `document-engine-api::SemanticObservation<T>` is the product-facing abstraction;
- `DocumentSession::require_current` is the application-side stale-result gate;
- the native adapter revision is executable evidence that a real engine boundary can supply the required freshness signal;
- a production adapter will need to bind its authoritative mutation acknowledgements and semantic observations to one versioned revision contract.

No feature should infer freshness from text equality, timestamps, render invalidations, UNO object addresses or incidental file metadata.

## What this does not prove

Revision freshness is necessary for strong history and reconciliation, but it is not semantic identity.

This qualification does not answer what happens to a logical paragraph through:

- insertion or deletion around it;
- split;
- merge;
- move/reorder;
- formatting-only mutation;
- save/reload;
- engine-process loss and semantic reconciliation.

Those questions remain the next evidence frontier. In particular, equal text at two revisions does not imply equal identity, and a changed revision does not tell us which semantic objects survived the change.

## Next qualification

The next native semantic experiment should preserve this revision stamp while characterising structural identity/reconciliation under deterministic edit sequences, beginning with **paragraph split and merge** because they directly stress the assumptions needed by history anchors, comments and selection recovery.

Only the minimum engine-side structural properties needed to test concrete identity hypotheses should be added to the projection. Engine references, UNO object identities, text hashes and byte offsets must remain non-authoritative evidence inputs rather than product IDs.
