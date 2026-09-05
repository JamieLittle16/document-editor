# R0A Writer Structural Identity Qualification

Status: qualified engine evidence, not a production identity or history-anchor contract.

## Question

Before Office defines durable paragraph identities, history anchors, comment anchors or collaboration references, we need to know what identity continuity the bootstrap Writer engine actually preserves under structural edits.

The experiment therefore asks a deliberately narrow question:

> For one live Writer authority, which paragraph UNO objects survive a deterministic paragraph split followed by a deterministic merge?

The result is evidence for a future reconciliation layer. It is not permission to expose UNO identity to product code.

## Reference environment

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
```

The qualification uses the same isolated LibreOfficeKit process, version-pinned unloadable semantic module, bounded `DETR` control boundary and deterministic three-paragraph fixture already described by `SEMANTIC_IDENTITY_SPIKE.md`.

## Probe semantics

Inside one retained `WriterSemanticView`, the qualification module assigns a monotonically increasing **view-local probe token** to each paragraph UNO object using UNO same-object equality.

A token means only:

> this observation refers to the same UNO object as an earlier observation in this same live semantic view.

It does **not** mean:

- product `ParagraphId`;
- persistence identity;
- history identity;
- stable identity across worker restart;
- a pointer or engine address exposed across the process boundary.

The host sees only native-neutral tokens and paragraph text. Repeated observations without mutation must produce the same token relation before any structural result is interpreted.

## Deterministic sequence

The fixture starts as:

```text
P0 = "Document Editor LibreOfficeKit R0A probe"
P1 = "This fixture is generated deterministically in CI."
P2 = "Stable semantic identity must be measured, not assumed."
```

The qualification performs:

1. observe at revision `R0`;
2. observe again at `R0` and require exact probe repeatability;
3. split `P0` after character offset `8`;
4. observe at revision `R1` and require exact split semantics;
5. observe again at `R1` and require probe repeatability;
6. merge the first two paragraphs by deleting the paragraph boundary;
7. observe at revision `R2` and require the original three paragraph texts to be restored;
8. observe again at `R2` and require probe repeatability.

Every successful structural mutation advances the native qualification revision exactly once.

## Qualified observation

Two independent CI executions on the pinned environment produced the same relation.

A representative token trace was:

```text
R0 before split:       (1, 2, 3)
R1 after split:        (4, 1, 2, 3)
R2 after merge:        (4, 2, 3)
```

The numeric token values are diagnostic only and are **not** pinned. CI pins the relation:

```text
R0 -> R1 split relation
0 -> 1
1 -> 2
2 -> 3

R1 -> R2 merge relation
0 -> 0
1 -> deleted
2 -> 1
3 -> 2

R0 -> R2 round-trip relation
0 -> deleted
1 -> 1
2 -> 2
```

Interpreted semantically:

- splitting the first paragraph creates a **new left-fragment paragraph object**;
- the original first-paragraph object survives as the **right fragment**;
- untouched later paragraph objects survive and shift right by one position;
- merging the two split fragments keeps the **left/new object** and destroys the **right/original object**;
- the final paragraph text is identical to the initial paragraph text, but the original first-paragraph UNO object identity is gone.

## Architectural conclusion

**Writer UNO object identity is useful local continuity evidence, but it is not a durable logical identity.**

The split/merge sequence is structurally non-invertible at the engine-object level: a semantic round trip that restores the original paragraph text does not restore the original paragraph object.

Therefore Office must not define Git-like history, comments, collaboration anchors or durable selections as aliases of Writer object identity.

The eventual identity/reconciliation layer should instead treat engine identity as one evidence channel among several, alongside transaction lineage, structural neighbourhood, semantic content and explicit product-owned identity where justified.

This is particularly important for history: undo/redo or branch replay may restore equivalent document semantics while the bootstrap engine chooses different internal objects. Product history must remain logically stable across that implementation choice.

## CI contract

`structural_identity_contract.py` executes the real native process harness and pins only:

- probe repeatability without mutation;
- exact `R0 -> R1 -> R2` revision progression;
- exact split and merge paragraph semantics;
- the three structural identity relations above.

CI deliberately does not pin:

- numeric probe-token values;
- UNO addresses;
- object allocation order beyond the observed relation;
- DOCX package bytes;
- raster hashes;
- any permanent product schema.

If a future LibreOffice version changes the relation, qualification should fail visibly. The result must then be re-measured and the reconciliation design reassessed rather than silently accepting changed engine behaviour.

## What this unlocks

The project can now design the next identity experiments with one important invariant established: **semantic equivalence and engine-object identity are different dimensions**.

The next qualification sequence should cover:

1. insertion/deletion adjacent to retained paragraphs;
2. paragraph move/reorder;
3. formatting-only edits;
4. duplicate-text paragraphs to defeat naive content matching;
5. save/reload and worker restart, where all live UNO identity should be assumed unavailable until measured;
6. callback/invalidation ordering relative to semantic revision changes.

Only after those measurements should Office freeze a product-owned paragraph/anchor reconciliation model and begin making the durable Git-like history store depend on it.

## Non-goals

This qualification does not authorize:

- a production `ParagraphId` based on UNO equality;
- persistence of probe tokens;
- leaking UNO references or engine addresses outside the version-pinned native module;
- content hashes as identities;
- text offsets as durable history/comment/collaboration anchors;
- assuming split/merge behaviour generalizes to tables, lists, fields, comments, tracked changes or other Writer structures;
- assuming the pinned 24.2 relation holds on future LibreOffice versions without requalification.
