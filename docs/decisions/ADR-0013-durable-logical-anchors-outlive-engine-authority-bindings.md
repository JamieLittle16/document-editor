# ADR-0013: Durable logical anchors outlive engine authority bindings

Status: accepted for R0A

Date: 2026-09-06

## Context

Office needs stable references for Git-like history, comments, selections, recovery and later collaboration without turning bootstrap-engine implementation details into product identity.

R0A Writer qualification established several facts that rule out the obvious shortcuts:

- an interior paragraph split followed by merge can restore identical text while destroying the original Writer paragraph object;
- paragraph-boundary insertion followed by deletion independently reproduces that non-invertible object relation;
- formatting-only mutation can preserve the same Writer object while semantic revision advances;
- distinct live paragraphs can contain identical text;
- save/reopen and full worker restart end the scope in which native object/probe-token equality has meaning;
- Writer-local probe-token values may be numerically reused after authority replacement;
- file-format paragraph identifiers were not preserved by the qualified LibreOffice round trip;
- recovery deliberately publishes a fresh `AuthorityGeneration` while preserving product-owned accepted-operation lineage.

Therefore no engine object, file-format identifier, content hash, text equality or byte offset can be the durable identity of a logical history anchor.

## Decision

### 1. Durable anchor identity is product-owned

A durable anchor is identified by:

```text
LogicalAnchorId = (HistoryLineageId, local_anchor_sequence)
```

`HistoryLineageId` is a product-owned history namespace. `local_anchor_sequence` is monotonically allocated within that namespace.

Neither component is derived from Writer/UNO identity, OOXML IDs, text, hashes, offsets, process IDs, authority generations or engine revisions.

The allocator cursor is itself product metadata. Persisting/restoring the cursor prevents anchor-ID reuse after reload without consulting the engine.

### 2. Durable identity and live binding are different objects

The durable record is:

```text
DurableLogicalAnchor<H> = (LogicalAnchorId, product reconciliation hint H)
```

The current live attachment is separate:

```text
LiveAnchorBinding<T> = (LogicalAnchorId, SessionAuthorityStamp, transient target T)
```

A save/reload, worker restart or checkpoint recovery may replace `LiveAnchorBinding<T>` while preserving exactly the same `LogicalAnchorId`.

`SessionAuthorityStamp` remains ephemeral correctness scope. It must not be serialized as the durable anchor identity.

### 3. Reconciliation hints are evidence, not identity

`DurableLogicalAnchor<H>` deliberately leaves the product hint schema generic in R0A. Future rich projections may store normalized structural and semantic hints appropriate to paragraphs, table cells, fields or other logical structures.

Changing, enriching or recomputing a hint must not change the anchor identity.

The product layer must not place raw UNO references, engine pointers, native probe tokens or naked file-format object IDs into a durable hint merely because the generic type permits arbitrary Rust values. Such values are outside this contract.

### 4. Reconciliation is conservative and evidence-based

R0A defines three positive evidence classes in descending precedence:

1. **explicit product operation lineage** — an accepted product operation maps the prior logical target to a specific current candidate;
2. **same-authority engine-object continuity** — the pinned engine reports that the target is the same live object, and both observations belong to the same `AuthorityGeneration`;
3. **unique structural + semantic agreement** — product-normalized neighbourhood and semantic evidence jointly identify exactly one candidate.

Semantic equality alone is never enough.

Structural similarity alone is never enough.

Engine-object equality is useful only as a positive same-authority signal. The product records the neutral fact that continuity was observed; it does not persist the native token that established it.

### 5. Engine continuity cannot cross authority replacement

Same-engine-object continuity evidence is invalid when the prior binding and candidate belong to different `AuthorityGeneration` values.

This is enforced by the reconciliation API rather than left as caller convention.

After reopen/restart/recovery, reconciliation must use product-owned lineage and/or product-normalized structural/semantic evidence.

### 6. Ambiguity is a first-class result

The reconciler must not guess between multiple plausible candidates.

It returns one of:

```text
Rebound(binding, basis)
Ambiguous
Unresolved
```

Conflicting strong evidence is `Ambiguous`, not a score tie to be silently broken by document order, allocation order or hash value.

Insufficient evidence is `Unresolved`, not implicit deletion.

This keeps history correctness stronger than convenience. Higher layers may later define user-visible conflict resolution or operation-specific repair policy without changing the durable identity primitive.

### 7. Product lineage outranks incidental similarity

When exactly one candidate has an explicit accepted-operation lineage mapping, it outranks incidental structural/semantic similarity elsewhere.

If product lineage conflicts with same-authority engine continuity pointing to a different candidate, reconciliation is `Ambiguous`. The system does not silently choose one evidence source over a contradictory strong source.

## Consequences

### Positive

- Git-like history can remain stable when Writer reconstructs equivalent content using different native objects;
- recovery/reopen can preserve logical identity while correctly replacing ephemeral authority;
- duplicate text cannot collapse distinct logical anchors;
- engine identity remains useful where it is actually trustworthy without contaminating persistence;
- future semantic projections can evolve their reconciliation hints without changing durable IDs;
- ambiguous cases remain visible and recoverable rather than becoming silent history corruption;
- the contract is engine-neutral and therefore survives eventual replacement of the LibreOffice bootstrap engine.

### Costs

- Office must persist product-owned history-lineage and anchor-allocation metadata;
- structural/semantic projections must eventually provide enough normalized evidence to rebind rich anchors after reload;
- operation implementations that structurally transform content should emit explicit product lineage mappings when they know the logical continuation;
- ambiguous/unresolved anchors need higher-layer policy rather than a convenient universal fallback.

## Invariants

1. A `LogicalAnchorId` is product-owned and engine-neutral.
2. Durable anchor identity never contains `SessionAuthorityStamp`.
3. A live binding always carries the exact authority/revision under which its target was observed.
4. Replacing authority may replace the live binding without replacing the durable anchor ID.
5. UNO references, engine pointers, probe tokens and file-format IDs never define durable identity.
6. Text equality and content hashes never define durable identity.
7. Current text offsets never define durable identity.
8. Same-engine-object evidence is accepted only within one `AuthorityGeneration`.
9. Semantic-only or structural-only evidence cannot independently rebind an anchor.
10. Multiple strong/plausible candidates are `Ambiguous`; insufficient evidence is `Unresolved`.
11. Explicit product operation lineage is preferred over incidental similarity unless it conflicts with another strong continuity signal.
12. Reconciliation hints may evolve independently of the durable anchor ID.

## R0A executable evidence

The `app-core::reconciliation` contracts prove:

- anchor sequences are monotonic and resumable without engine identity;
- duplicate semantic candidates remain ambiguous;
- a unique structural + semantic candidate can rebind an anchor without native identity;
- same-authority engine continuity is accepted as positive evidence across a revision change;
- the same engine-continuity channel is rejected across authority replacement;
- explicit accepted-operation lineage outranks incidental structural/semantic similarity;
- multiple product-lineage candidates remain ambiguous;
- save/reload preserves the durable anchor ID while replacing the live authority binding;
- checkpoint recovery preserves the durable anchor ID while replacing the live authority binding.

The ordinary workspace architecture/fmt/check/test/clippy gate validates this model independently of LibreOffice. The existing native qualification remains responsible for the Writer observations that justify the evidence rules.

## Non-decisions

This ADR does not freeze:

- the persisted binary/text encoding of `HistoryLineageId`, `LogicalAnchorId` or reconciliation hints;
- how a new `HistoryLineageId` is generated in production;
- save-as/fork/copy semantics for history lineages;
- a globally persistent document/account identity;
- the final rich semantic structure/locator schema for paragraphs, tables, fields, comments or tracked changes;
- collaboration causality, CRDT or OT identity;
- user-facing policy for ambiguous/unresolved anchors;
- whether future history commits receive a separate globally unique identifier.

Those decisions can build on this identity/binding separation without reopening the engine-identity question.
