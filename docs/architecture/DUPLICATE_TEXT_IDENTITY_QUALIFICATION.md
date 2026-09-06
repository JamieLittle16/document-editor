# R0A Duplicate-Text Paragraph Identity Qualification

Status: qualified engine evidence, not a production identity or history-anchor contract.

## Question

Can paragraph text, or any direct hash/fingerprint of paragraph text, serve as an identity key when Office reconciles semantic observations across edits?

No.

The qualification deliberately constructs two simultaneous Writer paragraphs with exactly the same text, then applies a verified non-text mutation to only one of them. The paragraph-text projection remains unchanged and therefore still contains two equally valid content matches, while same-live-object evidence continues to distinguish the two Writer objects.

This is evidence for the product-owned reconciliation layer. It does not authorize persistence of Writer object identity.

## Reference environment

```text
Ubuntu: 24.04.4
LibreOffice: 24.2.7.2
BuildId: 420(Build:2)
```

The probe uses the existing isolated LibreOfficeKit process, version-pinned unloadable Writer semantic module, bounded `DETR` control boundary and qualification-only view-local identity tokens.

## Fixture

A dedicated deterministic DOCX contains:

```text
P0 = "Duplicate paragraph identity evidence"
P1 = "Duplicate paragraph identity evidence"
P2 = "Unique structural neighbour"
```

The fixture intentionally contains no `w14:paraId`, `w14:textId` or other imported paragraph identifier. The ambiguity is therefore not masked by file-format metadata.

## Sequence

In one fresh Writer authority the qualification:

1. opens the duplicate-text fixture at `R0`;
2. observes the identity projection twice and requires exact repeatability;
3. requires P0 and P1 text to be byte-for-byte equal;
4. requires P0 and P1 to have distinct live Writer identity-probe tokens;
5. verifies the ordinary semantic projection is exactly `(P0, P1, P2)`;
6. applies the already-qualified formatting-only mutation `ParaAdjust = CENTER` to P0;
7. accepts that mutation only after Writer reads the property back as `CENTER`;
8. requires semantic revision `R0 -> R1` exactly once;
9. observes identity and ordinary semantic projections again;
10. requires all three paragraph texts to remain unchanged;
11. requires the duplicate text still to produce exactly two content candidates;
12. requires live object identity to remain repeatable and distinct.

The first exact-head CI execution observed:

```text
native_adapter_duplicate_tokens_before=(1, 2, 3)
native_adapter_duplicate_tokens_after=(1, 2, 3)
native_adapter_duplicate_identity_relation=0->0;1->1;2->2
native_adapter_duplicate_equal_text_distinct_live_objects=ok
native_adapter_duplicate_content_candidates=2
native_adapter_duplicate_text_semantics_unchanged=ok
native_adapter_duplicate_revision_progression=R0-R1
native_adapter_duplicate_first_paragraph_center_readback=ok
native_adapter_duplicate_identity_status=observed
native_adapter_duplicate_text_identity_contract=qualified
```

A second independent native CI execution on unchanged code reproduced the same pinned contract.

The numeric token values above are diagnostic only. CI pins the relation and semantic properties, not the token numbers themselves.

## Result

Two distinct paragraphs can have exactly equal semantic text at the same revision.

After a verified mutation to only the first paragraph:

- paragraph text remains exactly equal between P0 and P1;
- a content-only matcher still sees **two candidates**;
- the document revision has changed;
- live same-object evidence identifies which object continued from which prior observation.

Therefore none of the following is a valid paragraph identity on its own:

- paragraph text;
- a hash of paragraph text;
- equality of the current text projection;
- ordinal position plus text treated as a permanent ID.

Text and content hashes may still be useful reconciliation evidence, but they are evidence, not identity.

## Architectural consequence

The existing asymmetric Writer rule remains:

```text
same live Writer object
    => strong positive evidence of continuity within that live authority

different/missing live Writer object
    => non-decisive; reconcile using other evidence
```

This qualification adds a second independent rule:

```text
same paragraph text
    => potentially many logical candidates; never sufficient identity evidence
```

A durable Office reconciliation model therefore needs product-owned context such as transaction lineage, structural neighbourhood, semantic features beyond raw text, authority/session scope and ultimately explicit product identity where justified.

This matters directly to Git-like history, comments, tracked selections, recovery and collaboration. A history anchor must remain attached to the intended logical paragraph even when another paragraph has identical content, Writer replaces an engine object during a structural edit, or a worker restart destroys all live object references.

## CI contract

`duplicate_text_identity_contract.py` pins:

```text
identity relation: 0->0;1->1;2->2
equal-text paragraphs are distinct live objects: OK
content candidates after mutation: 2
paragraph-text semantics unchanged: OK
revision progression: R0-R1
first-paragraph CENTER read-back: OK
```

CI deliberately does not pin:

- numeric probe-token values;
- UNO references or addresses;
- imported file-format IDs;
- a production `ParagraphId` schema;
- a particular future reconciliation algorithm.

If a future bootstrap engine or LibreOffice version changes the measured live-object relation, the qualification should be re-measured. The product-level conclusion that duplicate content cannot be identity remains independent of that implementation detail.

## Next qualification

The next identity experiment should cross an authority boundary:

1. observe a semantic/identity snapshot;
2. destroy the semantic view by close/reload;
3. reacquire the same semantic document in a new view;
4. repeat across a fresh worker process;
5. prove that qualification tokens are scoped to their live view even when numeric token values are reused;
6. combine that scope result with duplicate-text ambiguity so restart reconciliation cannot accidentally depend on naked engine-token equality or content equality.

That evidence is the final prerequisite before freezing the first product-owned paragraph/anchor reconciliation model.