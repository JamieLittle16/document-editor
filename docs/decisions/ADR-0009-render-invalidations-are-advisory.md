# ADR-0009: Native render invalidations are advisory, revision-subordinate events

Status: accepted for R0A

Date: 2026-09-06

## Context

Office's bootstrap Writer engine emits LibreOfficeKit callbacks for client-side work such as tile invalidation. A future paginated viewport needs those events to keep rendered tiles fresh, but application semantics already have a separate product-owned authority model:

```text
(AuthorityGeneration, DocumentRevision)
```

The native callback boundary is concurrent with engine mutation. R0A qualification used a read-back-verified first-paragraph `ParaAdjust = CENTER` mutation that changed rendered output without changing paragraph text. Two independent executions against unchanged LibreOffice 24.2.7.2 code produced the same invalidation and cross-thread facts but differed on the exact callback delivery phase: one tile invalidation arrived after mutation return but before Office's modeled `R1` commit, while the other arrived while the mutation call was still active. In both cases the callback observed modeled host revision `R0`.

Therefore native callback timing cannot define transaction or semantic-revision ordering.

## Decision

Native render invalidations are **advisory dirty-region evidence**. They are subordinate to Office's product-owned authority and never constitute semantic revision authority.

The following rules are permanent architecture invariants.

### 1. Command/session path owns semantic authority

Only the authoritative document command/session path may commit a successful mutation and advance `DocumentRevision`. Worker restart/reopen authority remains governed by `AuthorityGeneration`.

A LibreOffice callback must never advance either value.

### 2. Native callback handling is an ingestion boundary

The native callback handler may do only bounded, non-blocking qualification-neutral work needed to preserve the event, such as:

- copy callback data whose engine-owned lifetime ends with the callback;
- normalize known dirty-region data;
- enqueue/coalesce bounded render dirtiness into thread-safe worker state;
- record diagnostics/trace metadata.

It must not directly mutate UI, application session state, history state or product identity state.

### 3. Render/cache work is authority-scoped

Rendered artifacts and render requests must be associated with product-owned document authority, eventually including at least:

```text
AuthorityGeneration
DocumentRevision
render/viewport parameters
```

A tile produced for one authority/revision must not become valid merely because its geometry matches a later authority/revision.

### 4. Mutation-adjacent invalidations require a fence

A native invalidation may race before Office has committed the successful transaction revision even though the engine's rendered state has already changed. The worker/shell boundary must therefore prevent a pre-commit callback from causing the UI to consume new engine render state while the application still considers the document to be the old revision.

The exact implementation can be chosen in R0B, but it must provide an equivalent ordering/fencing property. Plausible mechanisms include:

- worker-local event sequencing around each mutation command;
- staging dirty regions until transaction success and revision commit are published;
- stamping normalized dirty events with the committed product revision after the command boundary;
- requiring render requests to carry an expected authority/revision and rejecting mismatches.

The architecture depends on the invariant, not on one of these mechanisms specifically.

### 5. Callback cardinality and timing are not product contracts

Office must not assume:

- one edit produces one invalidation;
- invalidations arrive synchronously;
- invalidations arrive only after an engine call returns;
- callback order is stable across runs;
- callback rectangles are stable semantic identifiers;
- callbacks arrive on the worker's owning/request thread.

Callback coalescing and scheduling remain implementation details.

## Consequences

### Positive

- semantic authority remains independent of LibreOffice scheduling quirks;
- the future render cache can be correct across mutation races and worker replacement;
- native callback threads stay isolated from UI/application ownership;
- the same architecture can accommodate a future non-LibreOffice engine whose invalidation model differs;
- render invalidations can be aggressively coalesced without corrupting document semantics.

### Costs

- the worker/event protocol needs an explicit mutation fence or equivalent sequencing mechanism;
- render artifacts need authority-aware keys rather than geometry alone;
- callback payloads need bounded copying/normalization before their native lifetime ends;
- viewport implementation cannot simply forward native callbacks directly to the UI.

## Testing requirements

R0A CI must retain a native qualification that proves:

- a read-back-verified mutation changes rendered output;
- paragraph-text semantics can remain unchanged;
- at least one tile invalidation is emitted;
- a tile invalidation can be observed before Office's modeled revision commit;
- the pinned baseline demonstrates callback delivery away from the owning thread;
- semantic revision still advances only through the modeled command path.

The test must not pin exact callback phase, count, rectangle or ordering.

Future R0B worker/render tests must deliberately inject dirty events on both sides of a mutation completion boundary and prove that no tile from uncommitted or stale authority becomes visible as current UI state.

## Non-decisions

This ADR does not decide:

- the final worker event codec;
- tile size or cache eviction policy;
- shared memory versus copied render payloads;
- callback batching/coalescing thresholds;
- viewport scheduler details;
- the UI framework.
