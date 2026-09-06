# ADR-0012: Recovery reissues authority and preserves accepted-operation lineage

Status: accepted for R0A

Date: 2026-09-06

## Context

R0A already proved that a document-engine worker can die without killing the host, a fresh worker can start and reopen the document, and `AuthorityGeneration` prevents an observation from an old `R0` becoming current merely because the replacement engine starts its local revision clock at `R0` again.

That process evidence is necessary but not sufficient for application recovery. The product still needs explicit answers to:

- when old asynchronous/render/search work becomes invalid;
- what checkpoint state represents;
- how accepted user operations after a checkpoint are ordered;
- when a replacement authority becomes current;
- what happens if checkpoint load or replay fails;
- how recovery lineage avoids depending on Writer/UNO/file-format object identity.

The long-term product also wants Git-like history and richer semantic operations. R0A must therefore establish recovery provenance without prematurely declaring UTF-8 offsets, UNO references, OOXML IDs or content hashes to be permanent document identity.

## Decision

### 1. `AuthorityGeneration` remains the only authority-incarnation clock

Recovery does not introduce a competing worker epoch or recovery epoch.

The exact ephemeral scope of current work is:

```text
SessionAuthorityStamp = (AuthorityGeneration, DocumentRevision)
```

Only the retained `DocumentSession` can mint a current stamp. Semantic observations expose the same stamp, and asynchronous/render/search work may retain a copy as request provenance.

`SessionAuthorityStamp` is ephemeral correctness scope, not durable document identity.

### 2. Engine loss withdraws authority immediately

When the supervisor replaces a dead engine/worker binding, the session immediately has **no open authority**.

During this interval:

- old semantic observations are rejected;
- old asynchronous/render stamps are rejected;
- no replacement `AuthorityGeneration` is committed merely because a new process object exists.

A new generation is committed only after a replacement engine successfully opens/restores authoritative document state.

### 3. Checkpoints carry explicit source provenance and journal cursor

A `RecoveryCheckpoint<T>` contains:

```text
CheckpointSequence
source SessionAuthorityStamp
journal_cursor AcceptedOperationSequence
payload T
```

Checkpoint capture is allowed only from a current session observation. Stale observations cannot mint checkpoints.

`CheckpointSequence` is currently session-local ordering. It is deliberately not a persistent document ID or globally unique history identity.

The generic payload keeps checkpoint lineage independent of Writer/UNO and of any final persistence encoding. R0A uses semantic text only as a small orchestration qualification payload; production recovery may bind the same lineage to saved/checkpoint package artifacts and richer semantic state.

### 4. Every newly accepted user transaction receives immutable accepted-operation lineage

A successful session transaction returns a `SessionTransactionApplied` record containing:

```text
AcceptedOperationSequence
source SessionAuthorityStamp
result SessionAuthorityStamp
original DocumentTransaction
```

The sequence is reserved before mutation can begin but committed only after engine success. Therefore rejected transactions:

- do not advance `DocumentRevision`;
- do not consume accepted-operation sequence values;
- do not create recovery-journal gaps.

The original current text transaction is retained as audit/recovery evidence, but its UTF-8 offsets are **not** declared to be durable semantic anchors. Future structured operations may use different replay payloads while preserving the accepted-operation lineage contract.

### 5. Recovery requires a complete contiguous post-checkpoint journal

Before a replacement authority is opened, the session validates the retained journal tail against the checkpoint:

- first replayed operation is exactly `checkpoint.journal_cursor + 1`;
- sequences are contiguous;
- first operation source equals the checkpoint source authority/revision;
- each later operation source equals the previous operation result;
- each recorded transaction's expected revision agrees with its recorded source;
- the supplied tail reaches the session's latest accepted operation sequence.

An incomplete, reordered or inconsistent journal is rejected **before** replacement authority is published.

### 6. Replay reconstructs accepted intent; it does not accept new intent

During R0A text-fixture qualification, original accepted transactions are replayed onto the replacement authority by rebasing only their engine-local `expected_revision` to the replacement authority's current local revision.

Replay does **not** allocate new `AcceptedOperationSequence` values. Those operations were already accepted by the user before failure.

This distinction is important for future history: recovery reconstructs existing lineage rather than creating duplicate history commits.

The immutable original records retain the old authority/revision provenance even though the replacement engine has a fresh local revision clock.

### 7. Replacement authority is consumable only after complete recovery

Checkpoint load followed by complete replay yields a new authority generation and a final replacement revision.

If checkpoint load fails, no new authority is published.

If replay fails after partial engine mutation, the session withdraws replacement authority again. Partially reconstructed engine state is therefore inaccessible as current application state; recovery may retry from the checkpoint using another clean replacement worker.

### 8. Stale work remains stale across recovery

An old stamp is rejected:

```text
while authority is lost -> NoOpenDocument
after replacement succeeds -> AuthorityChanged
```

This is the same primitive future render-buffer completions, searches, diagnostics and other asynchronous results use. Recovery does not need subsystem-specific stale-work mechanisms.

## Consequences

### Positive

- worker replacement has an explicit safe state in which no authority is current;
- checkpoint and journal provenance are product-owned rather than engine-owned;
- accepted-operation ordering is gap-free and unaffected by rejected transactions;
- incomplete journals are detected before restore publication;
- partial replay failures cannot leak partially reconstructed state as current;
- old asynchronous/render work is rejected with the same allocation-free authority gate used by semantic observations;
- replacement revision clocks may restart independently without confusing old and new state;
- recovery replay does not duplicate accepted history lineage;
- future Git-like history can evolve semantic operation payloads without replacing the authority/checkpoint lineage model.

### Costs

- accepted user operations must be retained/durably journaled by the application layer rather than reconstructed from engine state after a crash;
- full application-process/power-loss recovery still needs a persisted checkpoint/journal encoding and durable session/document identity beyond current session-local sequences;
- future structural operations require replay-safe product-owned semantic anchors rather than relying permanently on current UTF-8 offsets;
- the worker/session manager must keep checkpoint durability and journal durability ordering explicit.

## Invariants

1. There is no current authority between engine loss and successful replacement restore.
2. `AuthorityGeneration` is the sole authority-incarnation counter.
3. Async/render/search work is valid only for its exact session authority stamp.
4. Checkpoints can be captured only from current authority.
5. Rejected transactions never consume accepted-operation sequence values.
6. Checkpoints record the accepted-operation cursor already represented by their payload.
7. Recovery never silently skips an operation the retained session knows was accepted.
8. Replay does not mint new accepted-operation IDs.
9. A replay failure withdraws replacement authority.
10. Engine/native/file-format identities do not define checkpoint or operation lineage.
11. Current UTF-8 edit offsets are a qualification replay payload, not a frozen durable anchor format.

## R0A evidence

The mandatory Rust tests qualify:

```text
G1/R0 open
  -> accepted operation #1 -> G1/R1
  -> checkpoint captures operation cursor #1
  -> accepted operations #2 and #3 -> G1/R3
  -> replacement worker/engine installed -> no current authority
  -> checkpoint opens as G2/R0
  -> operations #2 and #3 replay without new operation IDs
  -> recovered current authority G2/R2
  -> semantic state equals pre-crash accepted state
  -> next new user transaction receives operation #4
```

Additional tests require:

- stale observations cannot create checkpoints;
- rejected transactions leave operation sequences gap-free;
- incomplete journal tail is rejected before replacement open;
- checkpoint-open failure publishes no authority;
- replay failure withdraws partially reconstructed replacement authority;
- old async stamps fail during loss and after replacement.

## Non-decisions

This ADR does not freeze:

- the final durable checkpoint package/file format;
- journal fsync/batching cadence;
- a globally persistent `DocumentId`;
- a permanent history commit identifier;
- paragraph/block semantic anchor representation;
- current UTF-8 text edits as the future universal operation algebra;
- conflict/merge behavior after external modification;
- collaboration causality/CRDT/OT design.
