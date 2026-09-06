# Recovery and Persistence

## Non-negotiable outcome

A crash, worker failure or power interruption should lose as little accepted user input as technically practical and must not corrupt the last durable document.

Recovery is an application-owned correctness concern. Engine process restartability is useful evidence, but the engine is never allowed to define durable document identity or decide which asynchronous/history state is current after replacement.

## Layers

1. Original/user file.
2. Atomic save staging.
3. Application recovery journal of accepted operations.
4. Periodic recoverable checkpoints.
5. Session metadata/history index.

The layers deliberately have different durability and lifetime policies. A checkpoint is not the user file, a journal record is not a Writer undo record, and an engine-local revision is not a persistent document identity.

## Authority during recovery

Current application state is scoped by:

```text
(AuthorityGeneration, DocumentRevision)
```

`AuthorityGeneration` identifies one successful binding of the retained session to an authoritative engine document. A worker replacement does not create a new generation merely because a new process starts.

On worker/engine loss:

```text
old authority Gk/Rn
        |
        v
worker lost / engine replaced
        |
        v
NO CURRENT AUTHORITY
        |
        +-- old observations rejected
        +-- old render/search/async stamps rejected
        |
        v
checkpoint + complete journal restored successfully
        |
        v
new authority G(k+1)/Rfresh
```

This no-authority interval is intentional. It prevents partially restored or newly started engine state being observed as current before recovery has completed.

## Save

Never truncate the target file first.

Conceptually:

```text
serialize to sibling/temp staging file
validate basic output
flush data as required by platform policy
atomically replace target where filesystem semantics allow
update durable save marker
```

External user-file persistence and crash-recovery checkpoint persistence are related but distinct transactions.

## Recovery checkpoints

A recovery checkpoint has product-owned provenance:

```text
CheckpointSequence
source SessionAuthorityStamp
journal cursor (last accepted operation represented)
payload / durable checkpoint artifact
```

A checkpoint may be captured only from current authority. Stale semantic observations cannot mint recovery state.

`CheckpointSequence` is currently session-local ordering. It is **not** a globally persistent document ID, paragraph ID or Git-like history commit ID.

The R0A Rust qualification uses semantic text as a tiny checkpoint payload so the state machine can be proved without inventing a fake production package format. The production checkpoint payload will eventually be a durable engine-neutral/product-owned artifact appropriate to rich documents.

## Accepted-operation journal

Every successful new user transaction receives immutable application lineage:

```text
AcceptedOperationSequence
source SessionAuthorityStamp
result SessionAuthorityStamp
original accepted operation payload
```

The sequence is committed only after engine success. Rejected operations do not consume sequence values and do not create journal gaps.

The journal records *accepted intent*, not whatever happens to remain in one engine process's undo stack. It must therefore survive worker replacement independently of the worker.

Current `DocumentTransaction` UTF-8 edit ranges are the R0A operation payload because text is the only implemented editing surface. They are not promoted into a universal durable semantic-anchor scheme. Future structural operations/history may change the replay payload while preserving the same accepted-operation ordering and authority provenance.

## Recovery journal completeness

A checkpoint records the last accepted-operation sequence already included in its payload. Recovery must receive the complete contiguous journal tail after that cursor.

Before opening replacement authority, validate:

1. first replay record is exactly `checkpoint.cursor + 1`;
2. every later operation sequence is contiguous;
3. first source authority/revision matches the checkpoint source;
4. every later source equals the previous record result;
5. recorded operation revision metadata agrees with its source stamp;
6. the tail reaches the retained session's latest accepted-operation sequence.

Missing, reordered or inconsistent accepted input is not silently ignored.

## Replay semantics

Recovery replay reconstructs operations that were already accepted. It must not create duplicate accepted-operation IDs or pretend the user performed those actions again.

A replacement engine starts a fresh local revision clock. Therefore a replay adapter may translate/rebase **engine-local revision expectations** while leaving immutable original journal provenance unchanged.

For the R0A text qualification:

```text
old authority: checkpoint G1/R1, then ops #2/#3 -> G1/R3
replacement:   checkpoint opens G2/R0
replay #2/#3:  replacement reaches G2/R2
```

The different final `DocumentRevision` is correct: revision numbers are local to one authority generation. Operation lineage and recovered semantics—not numeric revision equality across workers—establish recovery continuity.

## Worker crash state machine

The session manager should be able to:

1. detect worker death;
2. immediately withdraw current authority;
3. preserve shell/session state, latest checkpoint and accepted-operation journal;
4. launch/install a replacement worker binding;
5. validate the complete journal tail against the checkpoint;
6. load the checkpoint into the replacement worker;
7. replay recoverable accepted operations where the operation adapter proves replay is safe;
8. verify resulting semantic/revision state;
9. publish the new `AuthorityGeneration` only through successful restore;
10. reject every stale async/render completion from the dead authority;
11. notify the user only if recovery is incomplete.

If checkpoint load fails, no replacement authority is published.

If replay fails after partially mutating the replacement engine, application authority is withdrawn again. That partial engine instance may be discarded and recovery retried from a clean replacement. Partial reconstruction is never exposed as current state.

## Render/search/async consequences

All asynchronous work should carry a session-minted `SessionAuthorityStamp`. This includes future render-buffer leases/completions from ADR-0011.

The same O(1) gate handles recovery staleness:

```text
during worker loss: old stamp -> NoOpenDocument
after recovery:     old stamp -> AuthorityChanged
```

Subsystems therefore do not invent independent restart generations.

## Durability ordering

The eventual durable implementation must make ordering explicit. At minimum it needs a policy equivalent to:

```text
accepted operation
    -> append journal record
    -> durable according to configured batching policy

periodic checkpoint
    -> serialize checkpoint artifact
    -> validate/flush checkpoint
    -> persist checkpoint metadata including journal cursor
    -> only then permit older covered journal segments to be reclaimed
```

Exact fsync cadence, batching and checkpoint frequency must be measured so normal typing latency remains excellent while accepted-input loss is bounded.

## External modification

Saving over an externally changed file without warning is forbidden. The product needs file identity/change detection and eventually a structured merge/compare workflow.

External modification reconciliation is separate from worker-crash replay. A journal that is safe to replay onto the exact checkpoint lineage is not automatically safe to replay onto an independently changed user file.

## R0A status

Qualified in Rust/session orchestration:

- non-forgeable authority stamps;
- immediate authority withdrawal on engine replacement;
- current-only checkpoint capture;
- gap-free accepted-operation sequencing;
- checkpoint journal cursor;
- contiguous complete-journal validation before restore;
- already-accepted replay without duplicate operation sequences;
- replacement local revision rebasing;
- no authority on checkpoint-open failure;
- no authority after partial replay failure;
- old semantic and async work rejected through the same authority gate;
- accepted-operation sequence continues monotonically after successful recovery.

Still intentionally unfrozen:

- durable rich-document checkpoint encoding;
- journal serialization/fsync policy;
- persistent logical document identity across full application restart;
- permanent semantic anchors for structural history/replay;
- external-file conflict merge semantics.

See ADR-0012 for the normative recovery lineage decision.
