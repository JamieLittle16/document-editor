# ADR-0008: Authority-scoped semantic observations

Status: accepted for R0A

Date: 2026-09-06

## Context

ADR-0007 established revision-stamped engine semantic observations so asynchronous or cached results can be rejected after a document mutation. That contract is correct within one continuous authority, but later R0A identity qualification exposed a second lifetime dimension.

The pinned Writer qualification now proves all of the following:

- a retained `WriterSemanticView` is destroyed when its document closes;
- reopening the same fixture creates a new semantic view whose qualification revision is again `R0`;
- a completely fresh worker process also opens at `R0`;
- view-local Writer identity-probe token values can be numerically reused across both boundaries;
- duplicate paragraphs can have exactly equal text while still being distinct live objects.

Therefore neither `DocumentRevision`, a naked engine token nor content equality identifies the authority incarnation that produced an observation.

The existing `DocumentSession::require_current` compared only `DocumentRevision`. An observation produced at old authority `R0` could therefore become apparently current after a successful reopen/rebind whose new authority also starts at `R0`.

This is a product correctness issue for future asynchronous search, diagnostics, AI results, comments, recovery and collaboration. It must be fixed above the bootstrap engine rather than by making LibreOffice define product identity.

## Decision

`document-engine-api::SemanticObservation<T>` remains engine-neutral and revision-stamped exactly as established by ADR-0007.

`document-session` adds a second product-owned scope:

```text
AuthorityGeneration
```

An `AuthorityGeneration` identifies one incarnation of the session's binding to an authoritative engine document. It is a fixed-width monotonic value owned by the application/session layer.

A generation is **not**:

- a durable logical `DocumentId`;
- a paragraph/object identity;
- an engine process ID;
- a Writer/UNO identifier;
- a persisted file-format identifier.

`DocumentSession` wraps validated engine observations in an immutable `SessionObservation<T>` carrying:

```text
(authority_generation, engine_semantic_observation)
```

so its effective freshness key is:

```text
(authority generation, document revision)
```

Only `DocumentSession` can mint a `SessionObservation`; there is no public constructor. Transformations such as `map` preserve both provenance dimensions.

Every **successful authoritative open/reopen/rebind** advances the session authority generation exactly once. The generation is reserved before calling the engine so exhaustion can fail without touching the current authority, but it is committed only after the engine reports success.

A failed open therefore does not change the current authority generation or revision. A failed/rejected transaction likewise changes neither authority generation nor revision.

`DocumentSession::require_current` checks in this order:

1. an authority is currently open;
2. the observation's authority generation equals the current generation;
3. the observation's revision equals the current revision.

Authority is checked before revision so an old `R0` can never become current merely because a replacement authority also starts at `R0`.

## Invariants

1. Engine observations remain stamped with the exact `DocumentRevision` from which their value was read.
2. A session observation additionally belongs to exactly one application-owned authority generation.
3. Two observations with the same revision but different authority generations are not interchangeable.
4. A successful authority replacement advances generation even if the new engine revision numerically equals the old revision.
5. A failed authority replacement leaves the previous generation/revision current.
6. A successful ordinary transaction changes revision but not authority generation.
7. A failed/rejected transaction changes neither freshness dimension.
8. Consumers cannot manufacture a current session observation by copying revision numbers or engine identifiers.
9. Engine-specific identity remains optional reconciliation evidence scoped to its live authority and never defines the generation.
10. Authority generation does not decide durable semantic object identity across generations; explicit reconciliation is still required.

## Testing requirement

The executable contract must cover at minimum:

- a fresh semantic read stamped with the current authority generation and revision;
- transformations preserving both stamps;
- a retained observation rejected after an ordinary revision advance;
- authority generation remaining unchanged across an ordinary transaction;
- an old `R0` observation rejected after successful reopen at a new `R0`;
- the reopened observation accepted under the new generation;
- a rejected transaction leaving the old observation current;
- an injected failed open leaving the old authority generation/revision and observation current;
- engine/session revision disagreement still being rejected before a session observation is minted.

The native qualification in `IDENTITY_SCOPE_RESTART_QUALIFICATION.md` remains the empirical evidence that justifies the generation dimension.

## Consequences

### Positive

- stale asynchronous results cannot resurrect after worker/document restart simply because revisions reset;
- freshness remains O(1), allocation-free and engine-round-trip-free;
- engine adapters remain replaceable and do not own application identity;
- the session now has the authority-incarnation primitive required by recovery and worker rebinding;
- future semantic reconciliation can explicitly separate same-authority evidence from cross-authority matching;
- duplicate content and reused engine token values can no longer masquerade as provenance.

### Costs

- application-facing semantic observations use a session wrapper rather than exposing the engine observation directly;
- authority replacement paths must advance generation consistently;
- future multi-document/session persistence still requires a separate durable logical document identity if one is needed.

## Relationship to ADR-0007

ADR-0007 remains valid at the engine boundary: `SemanticObservation<T>` is still revision-stamped and engines still must return coherent revision/value pairs.

This ADR strengthens the **session-level** freshness rule. Where ADR-0007 described currentness as revision comparison within the owning session, currentness is now defined by the pair:

```text
(authority generation, document revision)
```

## Non-decisions

This ADR does not decide:

- durable `DocumentId` format or persistence;
- production `ParagraphId` or other semantic object IDs;
- split/merge/reorder reconciliation policy;
- permanent history/comment/collaboration anchor schema;
- whether authority generations are ever persisted outside a live application session;
- the final worker protocol for authority rebinding;
- the future native/OpenDoc engine representation.

Those remain separate product-level decisions built on this freshness invariant.