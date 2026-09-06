# R0A Writer Structural Identity Qualification

Status: qualified engine evidence; product-owned durable-anchor policy is now recorded by ADR-0013.

## Question

Before Office defines durable paragraph identities, history anchors, comment anchors or collaboration references, we need to know what identity continuity the bootstrap Writer engine actually preserves under edits.

The qualification asks three deliberately narrow questions inside one live Writer authority:

1. which paragraph UNO objects survive an interior paragraph split followed by merge;
2. which paragraph UNO objects survive insertion of an empty paragraph at an existing boundary followed by deletion of that inserted paragraph;
3. which paragraph UNO objects survive a verified formatting-only paragraph mutation with no text or structural change.

The result is engine evidence for reconciliation. It is not permission to expose UNO identity to product code.

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

The host sees only native-neutral tokens and paragraph text. Repeated observations without mutation must produce the same token relation before any edit result is interpreted.

## Fixture

```text
P0 = "Document Editor LibreOfficeKit R0A probe"
P1 = "This fixture is generated deterministically in CI."
P2 = "Stable semantic identity must be measured, not assumed."
```

Every successful qualification mutation advances the native qualification revision exactly once.

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

## Qualification C: formatting-only paragraph mutation

The third sequence again runs in a fresh native process. It changes no paragraph text and introduces no structural boundary.

The version-pinned semantic module queries the first paragraph's `XPropertySet`, sets its `ParaAdjust` value to `CENTER`, and returns success only after reading the property back as `CENTER`. The process adapter advances the qualification revision only after that verified native mutation succeeds.

The sequence:

1. observes `(P0, P1, P2)` at `R0` and proves identity-probe repeatability;
2. confirms the ordinary semantic projection is also `(P0, P1, P2)` at `R0`;
3. applies and read-back verifies `ParaAdjust = CENTER` on `P0`;
4. requires revision `R1`;
5. requires both identity and ordinary semantic projections to remain exactly `(P0, P1, P2)`;
6. proves identity-probe repeatability again at `R1`.

Two independent CI executions reproduced the same relation:

```text
representative R0 before formatting: (1, 2, 3)
representative R1 after formatting:  (1, 2, 3)

pinned R0 -> R1 relation
0 -> 0
1 -> 1
2 -> 2
```

Interpretation:

- the first paragraph remains the same Writer UNO object after its verified alignment change;
- untouched second and third paragraphs also remain the same objects;
- formatting-only mutation advances semantic revision even though the paragraph-text projection is unchanged;
- object equality can therefore carry useful **positive** continuity evidence within one live authority.

This does not make object inequality the inverse signal. Qualifications A and B already prove that a logically continuous paragraph can acquire a different engine object after ordinary structural edits.

## Architectural conclusion

**Writer UNO object identity is useful local continuity evidence, but it is not durable logical identity.**

The combined evidence gives Office an asymmetric reconciliation rule:

```text
same live Writer object
    => strong positive evidence of logical continuity

different/missing live Writer object
    => not sufficient evidence of logical discontinuity
```

Formatting-only mutation preserves all observed paragraph objects in the pinned engine, while two ordinary structural round trips replace an original paragraph object despite exact semantic restoration.

Therefore Office must not define Git-like history, comments, collaboration anchors or durable selections as aliases of Writer object identity. ADR-0013 now freezes the corresponding product rule: durable `LogicalAnchorId` values are product-owned; live authority bindings are replaceable; reconciliation uses explicit product lineage first, same-authority native continuity as a positive local signal, and unique structural + semantic evidence as a fallback. Ambiguous or unresolved cases remain explicit rather than guessed.

This is particularly important for history. Undo/redo, branch replay or crash recovery may restore equivalent semantics while the bootstrap engine chooses different internal objects. Product history remains logically stable across that implementation choice.

## CI contracts

`structural_identity_contract.py` pins the interior split/merge experiment.

`paragraph_insert_delete_contract.py` pins the boundary insertion/deletion experiment.

`paragraph_format_identity_contract.py` pins the independently reproduced formatting-only relation plus verified `CENTER` read-back, exact `R0 -> R1` revision progression, unchanged paragraph-text semantics and probe repeatability.

Together they require:

- probe repeatability without mutation;
- exact revision progression for each mutation;
- exact semantic results for each qualification;
- the independently reproduced identity relations above.

Later R0A native qualification additionally covers duplicate-text ambiguity and identity-token scope across reopen/worker restart. The product-side `app-core::reconciliation` tests then require those facts to produce conservative durable-anchor behavior.

CI deliberately does not pin:

- numeric probe-token values;
- UNO addresses;
- object allocation order beyond the measured relations;
- DOCX package bytes;
- raster hashes;
- a rich permanent paragraph/table/field locator schema.

If a future LibreOffice version changes a relation, qualification should fail visibly. The result must then be re-measured and the reconciliation evidence adapters reassessed rather than silently accepting changed engine behaviour.

## What this unlocks

The completed R0A evidence establishes four durable invariants:

1. **semantic equivalence and engine-object identity are different dimensions**;
2. **same-object equality is useful positive continuity evidence during one live authority, but loss of equality is not a logical deletion signal**;
3. **native identity does not survive authority replacement as a product contract**;
4. **durable logical identity belongs to Office and is rebound to current engine state through conservative evidence**.

ADR-0013 and `app-core::reconciliation` are the first product contract built from that evidence. This is enough to let R0B history/recovery work depend on a stable identity/binding separation without waiting for a complete rich-document semantic schema.

Still intentionally unfrozen are the concrete persisted hint schema for richer structures, save-as/fork history-lineage policy, user-facing resolution of ambiguous anchors, and collaboration causality. Those can evolve above the durable identity primitive.

## Non-goals

This qualification does not authorize:

- a production `ParagraphId` based on UNO equality;
- persistence of probe tokens;
- leaking UNO references or engine addresses outside the version-pinned native module;
- content hashes as identities;
- text offsets as durable history/comment/collaboration anchors;
- treating formatting preservation of object identity as a guarantee for every property or Writer structure;
- assuming these paragraph results generalize to tables, lists, fields, comments, tracked changes or other Writer structures;
- assuming the pinned 24.2 relations hold on future LibreOffice versions without requalification.
