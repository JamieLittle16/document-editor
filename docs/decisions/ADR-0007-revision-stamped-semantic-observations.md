# ADR-0007: Revision-stamped semantic observations

Status: accepted for R0A

Date: 2026-09-05

## Context

Office derives semantic state from a mutable document authority for features such as search, diagnostics, comments, history, selection recovery, AI assistance and later collaboration. Those results can outlive the instant at which they were computed, especially once feature work becomes asynchronous or cached.

The R0A engine/session stack already has a monotonic `DocumentRevision` and optimistic transaction admission, but semantic reads previously returned bare values. A bare semantic value gives the consumer no way to distinguish current state from a result produced before a later transaction.

The semantic-identity qualification also shows that engine object identity and incidental DOCX IDs are not suitable product authority. Revision freshness must therefore be explicit without prematurely defining `ParagraphId`, permanent anchors or a product `DocumentId`.

## Decision

All semantic observations exposed by `document-engine-api` are stamped with the exact authoritative `DocumentRevision` from which they were read.

R0A introduces `SemanticObservation<T>` with:

- an immutable `DocumentRevision`;
- the observed native-neutral value;
- no engine object/reference identity;
- no paragraph/object identity claim;
- no implicit conversion that discards the revision.

`DocumentEngine::semantic_text` returns `SemanticObservation<String>` rather than `String`.

`DocumentSession` verifies that a freshly returned observation agrees with the session's known authoritative revision. A disagreement is an engine/session coherence failure, not ordinary staleness.

A retained observation must pass `DocumentSession::require_current` before it is used to affect current application/UI state. The check compares only fixed-width revision values and requires no engine round trip or allocation.

The owning `DocumentSession` is the R0A document authority, so document association is implicit in the session that produced and validates the observation. We deliberately do not invent a permanent product-level `DocumentId` until multi-document authority/lifetime semantics require one. A future cross-session cache must add explicit document/session provenance rather than treating revision numbers as globally unique.

## Invariants

1. The revision stamp describes the same state as the observed value.
2. A successful authoritative mutation advances the revision before newly observed semantic state is exposed.
3. A failed/rejected transaction does not advance the session revision and therefore does not invalidate an otherwise current observation.
4. A retained observation from revision `R` is stale once the session advances to a different revision.
5. Consumers must not erase or ignore provenance at asynchronous/cache boundaries.
6. Revision stamping does not establish semantic object identity, anchoring or reconciliation.
7. Engine-specific references, addresses and UNO objects remain behind the native boundary.

## Testing requirement

The contract is executable. Tests must cover at minimum:

- semantic reads stamped with the exact current revision;
- a retained observation rejected after a successful mutation;
- a newly read observation accepted at the new revision;
- a rejected transaction leaving the previous observation current;
- an engine/session revision disagreement being rejected as an internal coherence failure when an engine implementation can exercise that path.

Future asynchronous feature tests should deliberately produce a result at revision `R`, advance the document to `R+1`, then prove the old result cannot update current state.

## Consequences

### Positive

- search/diagnostic/AI/history features gain one shared freshness primitive;
- stale-result rejection is deterministic and O(1);
- no host cache needs to infer freshness from text equality, timestamps or engine callbacks;
- the contract composes with the existing transaction revision model;
- revision provenance is established before stable semantic identity is frozen.

### Costs

- semantic consumers must carry the observation wrapper rather than a naked value;
- multi-document caching will require explicit document/session provenance in addition to `DocumentRevision`;
- engine implementations must stamp reads atomically with the state they return.

## Non-decisions

This ADR does not decide:

- stable paragraph/object IDs;
- split/merge/move reconciliation;
- permanent history/comment/collaboration anchors;
- persistence of semantic IDs across save/reload;
- the production LibreOffice compatibility layer;
- the final process-wire semantic snapshot schema.

Those remain evidence-driven follow-up work. The next semantic qualification should exercise structural edits and reconciliation while preserving this revision-freshness invariant.
