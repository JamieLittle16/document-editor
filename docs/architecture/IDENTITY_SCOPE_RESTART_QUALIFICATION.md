# R0A Identity Scope Across Reopen and Worker Restart

Status: qualified engine evidence, not a production paragraph identity contract.

## Question

Can a Writer identity-probe token, or a semantic revision by itself, be compared across document close/reopen or worker restart as though it came from one continuous authority?

No.

The qualification proves that both values can be numerically reused after the old live authority has been destroyed. A product consumer that ignores authority scope can therefore mistake a new Writer object and a new `R0` for an old one.

## Reference environment

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
```

The test uses the duplicate-text fixture from `DUPLICATE_TEXT_IDENTITY_QUALIFICATION.md` so content itself cannot resolve the ambiguity.

## Existing scope contract

The version-pinned Writer semantic module deliberately defines identity-probe tokens as **view-local qualification evidence only**. Every newly acquired `WriterSemanticView` begins with:

```text
nextProbeToken = 1
```

A token is meaningful only while that retained view exists. It is not a product `ParagraphId`, persistence ID, history ID or process-global object key.

The process adapter's close path explicitly:

1. destroys the semantic view;
2. destroys the Writer document;
3. resets the qualification document revision to `R0`.

The qualification verifies this destruction rather than inferring it from later values: immediately after close, an identity-probe request must fail with the typed `no Writer document is open` engine-state response.

## Fixture

```text
P0 = "Duplicate paragraph identity evidence"
P1 = "Duplicate paragraph identity evidence"
P2 = "Unique structural neighbour"
```

P0 and P1 deliberately have identical text and no imported OOXML paragraph IDs.

## Sequence

The native qualification performs three observations.

### A. First live semantic view

In one fresh LibreOfficeKit worker:

1. open the fixture;
2. require revision `R0`;
3. observe the identity projection twice and require repeatability;
4. require three unique live tokens and exact paragraph semantics;
5. require the ordinary semantic projection to agree.

### B. Same-worker close and reopen

1. close the document;
2. require identity projection to become unavailable, proving the old semantic view is gone;
3. reopen the same fixture in the same worker;
4. acquire a new semantic view at fresh `R0`;
5. require repeatable identity and semantic projections again.

### C. Fresh worker process

1. retire the first worker cleanly;
2. start a completely new LibreOfficeKit worker with a separate profile;
3. open the same fixture;
4. acquire another fresh semantic view at `R0`;
5. require the same semantic correctness and duplicate-text ambiguity.

## Observed result

Two independent CI executions on unchanged code reproduced:

```text
native_adapter_scope_tokens_first=(1, 2, 3)
native_adapter_scope_tokens_reopen=(1, 2, 3)
native_adapter_scope_tokens_restart=(1, 2, 3)
native_adapter_scope_semantic_view_destroyed_on_close=ok
native_adapter_scope_same_worker_token_values_reused=observed
native_adapter_scope_fresh_worker_token_values_reused=observed
native_adapter_scope_all_views_revision=R0
native_adapter_scope_semantics_reacquired=ok
native_adapter_scope_duplicate_content_candidates=2
native_adapter_identity_scope_status=observed
native_adapter_identity_scope_restart_contract=qualified
```

The CI contract deliberately does **not** pin the numeric values `(1, 2, 3)`. It pins that the qualification-token tuple is reused between independently acquired views. The current numbers are diagnostic only.

## Result

The three observations have the same:

- paragraph text;
- paragraph order;
- qualification revision `R0`;
- numeric identity-probe token tuple in the pinned implementation.

But they do **not** share one live Writer identity namespace.

The first semantic view is explicitly destroyed before the second exists, and the third belongs to a different LibreOffice process entirely. Therefore:

```text
same numeric engine token across authority boundaries
    != same engine object

same document revision across authority boundaries
    != same authoritative state incarnation
```

Combined with the duplicate-text qualification:

```text
same paragraph text
    != unique logical candidate
```

No one of these signals can establish durable identity after restart.

## Product consequence

Office needs a product-owned **authority incarnation/generation** in addition to `DocumentRevision`.

The current revision-only freshness rule is insufficient for a reopened authority:

```text
old authority: observation at R0
        ↓ close / worker loss
new authority: freshly reopened at R0
```

If freshness compares only the revision number, the old `R0` observation can appear current again even though the authority that produced it no longer exists.

The product freshness key must therefore distinguish:

```text
(authority generation, document revision)
```

rather than `DocumentRevision` alone.

The authority generation belongs to the Office application/session layer, not to LibreOffice. A bootstrap engine may contribute ephemeral reconciliation evidence, but it must not define the lifetime or namespace of product semantic observations.

This is deliberately not called a durable `DocumentId`. An authority generation identifies one binding/incarnation of a logical session to an authoritative engine state. Future logical document identity may survive several such generations.

## History and recovery consequence

The same rule applies to future paragraph anchors and Git-like history.

After worker replacement, Office must reconcile the new semantic state using product-owned lineage and structural/semantic evidence. It cannot reconnect an old history/comment/selection anchor by comparing:

- naked Writer tokens;
- revision numbers;
- paragraph text;
- content hashes;
- current ordinal positions.

Those can all be evidence, but their authority scope must be explicit.

## CI contract

`identity_scope_restart_contract.py` requires:

- repeatable identity projection in every live view;
- exact `R0` revision in all three fresh authorities;
- explicit identity-projection failure after close;
- exact semantic reacquisition after reopen and restart;
- duplicate-text ambiguity retained;
- qualification-token tuple reuse across same-worker reopen;
- qualification-token tuple reuse across fresh-worker restart.

If a future engine stops reusing the current token values, that implementation detail may change. The architectural invariant remains: a token from one destroyed authority has no meaning in another authority unless Office itself explicitly reconciles them.

## Next product slice

The immediate permanent follow-up is to strengthen `document-session` semantic freshness:

1. introduce an application-owned authority-generation value;
2. advance it on every successful authoritative open/rebind;
3. wrap engine semantic observations with that generation;
4. require both generation and revision to match before a retained observation can affect current state;
5. prove an old `R0` observation is rejected after a successful reopen that also returns `R0`;
6. preserve the existing revision-staleness and failed-transaction behaviour.

That change should be recorded in a dedicated ADR because it becomes part of the permanent product correctness kernel.