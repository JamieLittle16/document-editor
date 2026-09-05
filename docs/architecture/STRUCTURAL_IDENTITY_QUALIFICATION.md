# R0A Writer Structural Identity Qualification

Status: qualified engine evidence, not a production identity or history-anchor contract.

## Question

Before Office defines durable paragraph identities, history anchors, comment anchors or collaboration references, we need to know what identity continuity the bootstrap Writer engine actually preserves under structural edits.

The qualification now asks two deliberately narrow questions inside one live Writer authority:

1. which paragraph UNO objects survive an interior paragraph split followed by merge;
2. which paragraph UNO objects survive insertion of an empty paragraph at an existing boundary followed by deletion of that inserted paragraph.

The result is evidence for a future reconciliation layer. It is not permission to expose UNO identity to product code.

## Reference environment

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
```

The qualification uses the same isolated LibreOfficeKit process, version-pinned unloadable semantic module, bounded `DETR` control boundary and deterministic three-paragraph fixture described by `SEMANTIC_IDENTITY_SPIKE.md`.

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

## Fixture

```text
P0 = "Document Editor LibreOfficeKit R0A probe"
P1 = "This fixture is generated deterministically in CI."
P2 = "Stable semantic identity must be measured, not assumed."
```

Every successful structural mutation advances the native qualification revision exactly once.

## Qualification A: interior split then merge

The first sequence:

1. observes at `R0` and proves probe repeatability;
2. splits `P0` after character offset `8`;
3. observes exact split semantics at `R1` and proves repeatability;
4. merges the first two paragraphs by deleting their paragraph boundary;
5. observes restoration of the original three paragraph texts at `R2` and proves repeatability.

Two independent CI executions produced the same relation.

Representative token trace:

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

Interpretation:

- splitting the first paragraph creates a **new left-fragment paragraph object**;
- the original first-paragraph object survives as the **right fragment**;
- untouched later paragraph objects survive and shift right by one position;
- merging the two split fragments keeps the **left/new object** and destroys the **right/original object**;
- the final paragraph text is identical to the initial paragraph text, but the original first-paragraph UNO object identity is gone.

## Qualification B: boundary insertion then deletion

The second sequence runs in a fresh native process so the first experiment cannot contaminate object history.

It reuses the same structural primitive rather than adding another ABI or wire command: inserting a paragraph break exactly at the end of `P0` creates an empty paragraph between `P0` and `P1`.

The sequence:

1. observes `(P0, P1, P2)` at `R0` and proves repeatability;
2. inserts a paragraph break at the exact end boundary of `P0`;
3. requires `(P0, "", P1, P2)` at `R1` and proves repeatability;
4. deletes that inserted paragraph boundary by merging the first two paragraphs;
5. requires exact restoration of `(P0, P1, P2)` at `R2` and proves repeatability.

Two independent CI executions again produced exactly the same relation:

```text
representative R0 before insertion: (1, 2, 3)
representative R1 after insertion:  (4, 1, 2, 3)
representative R2 after deletion:   (4, 2, 3)
```

CI pins only the relation:

```text
R0 -> R1 insertion relation
0 -> 1
1 -> 2
2 -> 3

R1 -> R2 deletion relation
0 -> 0
1 -> deleted
2 -> 1
3 -> 2

R0 -> R2 round-trip relation
0 -> deleted
1 -> 1
2 -> 2
```

This is the same object-continuity pattern as the interior split/merge case even though the inserted paragraph is empty and the final semantic document is exactly restored.

Interpretation:

- paragraph-boundary insertion creates a new object for the retained left paragraph position;
- the original `P0` object becomes the new empty paragraph object to its right;
- deleting that empty paragraph preserves the new left object and destroys the original `P0` object;
- untouched `P1` and `P2` objects survive both operations;
- exact semantic restoration still does not restore the original `P0` engine object.

## Architectural conclusion

**Writer UNO object identity is useful local continuity evidence, but it is not durable logical identity.**

Both tested structural round trips are non-invertible at the engine-object level. This is no longer an edge case tied only to splitting paragraph content: even inserting and deleting an empty adjacent paragraph can restore exact document semantics while replacing an original paragraph object.

Therefore Office must not define Git-like history, comments, collaboration anchors or durable selections as aliases of Writer object identity.

The eventual identity/reconciliation layer should treat engine identity as one evidence channel among several, alongside transaction lineage, structural neighbourhood, semantic content and explicit product-owned identity where justified.

This is particularly important for history. Undo/redo, branch replay or crash recovery may restore equivalent semantics while the bootstrap engine chooses different internal objects. Product history must remain logically stable across that implementation choice.

## CI contracts

`structural_identity_contract.py` pins the interior split/merge experiment.

`paragraph_insert_delete_contract.py` runs the insertion/deletion observation probe and pins only the independently reproduced relation.

Together they require:

- probe repeatability without mutation;
- exact `R0 -> R1 -> R2` revision progression;
- exact semantic results for each structural operation;
- the structural identity relations above.

CI deliberately does not pin:

- numeric probe-token values;
- UNO addresses;
- object allocation order beyond the measured relation;
- DOCX package bytes;
- raster hashes;
- any permanent product schema.

If a future LibreOffice version changes a relation, qualification should fail visibly. The result must then be re-measured and the reconciliation design reassessed rather than silently accepting changed engine behaviour.

## What this unlocks

Two distinct structural sequences now establish the same invariant: **semantic equivalence and engine-object identity are different dimensions**.

The next qualification sequence should cover:

1. paragraph move/reorder;
2. formatting-only edits;
3. duplicate-text paragraphs to defeat naive content matching;
4. save/reload and worker restart, where live UNO identity must not be assumed to survive;
5. callback/invalidation ordering relative to semantic revision changes.

Only after those measurements should Office freeze a product-owned paragraph/anchor reconciliation model and make the durable Git-like history store depend on it.

## Non-goals

This qualification does not authorize:

- a production `ParagraphId` based on UNO equality;
- persistence of probe tokens;
- leaking UNO references or engine addresses outside the version-pinned native module;
- content hashes as identities;
- text offsets as durable history/comment/collaboration anchors;
- assuming these paragraph results generalize to tables, lists, fields, comments, tracked changes or other Writer structures;
- assuming the pinned 24.2 relations hold on future LibreOffice versions without requalification.
