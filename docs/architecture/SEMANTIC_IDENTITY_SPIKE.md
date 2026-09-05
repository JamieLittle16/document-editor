# R0A Semantic Snapshot and Identity Spike

Status: qualification evidence, not a production document-model contract.

## Purpose

The application needs semantic identities that can support selection recovery, Git-like history, comments, diagnostics and eventually collaboration without depending on transient byte offsets, render coordinates or engine object addresses.

R0A therefore measures candidate identity and reconciliation signals before any permanent anchor model is designed.

The evidence now answers five questions:

1. can DOCX paragraph metadata serve as durable product identity? **No.**
2. can public LibreOfficeKit view/accessibility APIs provide deterministic whole-document live semantics? **Not in the qualified headless configuration.**
3. can a richer native semantic layer reach the exact Writer document already owned by LibreOfficeKit without creating a second authority? **Yes, in the pinned 24.2 environment.**
4. can normalized revision-stamped live semantics cross the isolated engine process boundary without exposing UNO types? **Yes, for the bounded R0A paragraph projection.**
5. does Writer paragraph UNO-object identity behave like durable logical identity under split/merge? **No. It is useful local continuity evidence, but a semantic split/merge round trip can destroy the original object identity.**

A production paragraph identity/reconciliation model is still deliberately unresolved.

## Fixture

The deterministic DOCX qualification fixture contains three Writer paragraphs with known text and explicitly seeded Word 2010 paragraph metadata:

```text
paragraph 1: w14:paraId=13579BDF, w14:textId=2468ACE0
paragraph 2: w14:paraId=89ABCDEF, w14:textId=10293847
paragraph 3: w14:paraId=A1B2C3D4, w14:textId=55667788
```

These values are test probes only. They are not application identifiers.

The saved-file semantic projection used by CI is intentionally small:

```text
Paragraph {
    external_para_id?: string,
    external_text_id?: string,
    text: string,
}
```

The projection is read directly from `word/document.xml` before and after the real LibreOfficeKit edit/save/reopen qualification.

## Qualified saved-file observation

Reference environment:

```text
Ubuntu 24.04.4
LibreOffice 24.2.7.2
BuildId 420(Build:2)
```

After LibreOfficeKit loaded the DOCX, inserted the R0A text marker, saved a new DOCX and reopened it:

```text
input paragraphs:             3
round-trip paragraphs:        3
paragraph order/text:         preserved
edit locality:                paragraph 1
w14:paraId values present:    0
w14:paraId preserved:         0 / 3
w14:textId preserved:         0 / 3
```

The semantic paragraph texts remained in the same order and the edit marker appeared only at the start of paragraph 1. All seeded `w14:paraId` and `w14:textId` attributes were absent from the saved round-trip OOXML.

### Conclusion: OOXML IDs

**DOCX `w14:paraId` and `w14:textId` are rejected as the product's semantic identity mechanism.**

An identity mechanism that disappears during the qualified bootstrap engine's ordinary save path cannot be authoritative for product state.

The exact stripping behaviour is not a permanent compatibility requirement. Future LibreOffice versions may preserve, regenerate or otherwise transform these fields. CI records the observation but does not require LibreOffice to keep stripping them forever.

Content equality alone is also insufficient. Duplicate paragraphs, splits, merges, moves and edits make text matching ambiguous in real documents.

## Rejected public LibreOfficeKit live-semantic routes

After rejecting file-format IDs, R0A tried to extract semantics directly from the live Writer instance without introducing UNO/internal dependencies.

### Accessibility-focused paragraph traversal

The adapter enabled accessibility, moved to the document start, read `getA11yFocusedParagraph()`, issued `.uno:GoToNextPara`, and attempted to enumerate the three fixture paragraphs.

Observed result:

```text
expected live paragraphs: 3
observed live paragraphs: 1
observed content: paragraph 1 only
```

`getA11yFocusedParagraph()` exposed the focused paragraph, but the headless navigation sequence did not synchronously advance the accessibility focus seen by the API. The call is therefore focused-view state, not a proven whole-document enumerator.

### Select-all + text-selection snapshot

The adapter then issued `.uno:SelectAll` and immediately called `getTextSelection("text/plain;charset=utf-8", ...)`.

Observed result:

```text
selection call: returned
selected live text: empty
```

Selection mutation behaviour therefore does not imply deterministic whole-document extraction in this headless process configuration.

### Conclusion: public LOK snapshot surface

**The tested public LibreOfficeKit view/accessibility surface is not accepted as a deterministic whole-document live semantic snapshot API.**

The failed commands were removed rather than retained as weak or misleading capabilities.

## Qualified same-instance Writer semantic access

R0A then tested a deeper native-only route. The qualification:

1. initializes one LibreOffice process through `lok::lok_cpp_init`;
2. loads the deterministic DOCX exactly once through that LibreOfficeKit authority;
3. obtains the already-running LibreOffice process component context;
4. obtains the process `Desktop` and requires exactly one live Writer `XTextDocument`;
5. retains that semantic view for the lifetime of the open LOK document;
6. enumerates the three paragraphs through UNO semantic interfaces;
7. performs a prefix edit through the original LibreOfficeKit `Document` **without saving**;
8. re-enumerates through the retained semantic view;
9. requires paragraph count/order to remain stable, paragraph 1 to contain the unsaved marker, and paragraphs 2-3 to remain unchanged.

This is stronger than observing the same file twice. No separate UNO bootstrap is performed, no second document is loaded, and the semantic view observes a mutation that has not passed through save/reopen.

### Version-pinned implementation boundary

The bridge reaches the process context through LibreOffice's internal `comphelper::getProcessComponentContext()` symbol. The exact LibreOffice **24.2** signature returns the component-context reference by value and differs from newer LibreOffice source.

The dependency is confined to the unloadable compatibility module:

```text
spikes/libreofficekit-process-adapter/writer_semantics_24_2.cxx
```

`writer_semantics_module_abi.hxx` exposes only a small native-neutral qualification ABI. UNO references and internal LibreOffice types do not cross into the adapter executable or Rust product crates.

Therefore:

- same-instance semantic access is a hard qualification for the pinned LibreOffice 24.2.7.2 environment;
- the internal symbol is **not** a product API;
- Rust product crates gain no UNO dependency and no `unsafe` boundary;
- a production native semantic adapter requires an explicit versioned compatibility layer and ADR;
- upgrading LibreOffice must requalify this ABI instead of assuming source compatibility.

## Qualified bounded process-boundary snapshot

The same-instance result is integrated into the killable native process adapter.

Semantic projection version 2 is deliberately disposable R0A qualification data:

```text
status:u8
command:u8
projection_version:u8
revision:u64-le
paragraph_count:u16-le
repeat paragraph_count times:
    byte_length:u16-le
    utf8_text[byte_length]
```

The complete response must fit the adapter's existing **1024-byte** control-frame payload bound. Oversized semantic results are rejected with a typed qualification-limit response rather than becoming an accidental unbounded transport path.

The host harness proves:

```text
open fixture: OK
bounded snapshot before edit: R0 + exact 3 fixture paragraphs
unsaved LOK prefix edit: OK
bounded snapshot after edit: R1 + prefix visible only in paragraph 1
same retained native semantic view: preserved across the edit
close document: semantic access removed
force-kill with live Writer state: observed
fresh process restart/reopen: OK
fresh bounded snapshot after restart: fresh R0 + original 3 fixture paragraphs
```

Only native-neutral bytes cross `DETR`. No UNO reference, engine address or Writer implementation object leaves the native process.

### Conclusion: snapshot boundary

**The project has a proven bounded, revision-stamped live semantic observation seam across the isolated engine process boundary.**

This establishes where semantic information can flow and how freshness can be checked. It does not freeze the permanent semantic schema or identity model.

## Qualified live structural identity observation

The next experiment retained the same semantic authority and added qualification-only paragraph-object probes plus two deterministic structural operations:

- split the first paragraph at character offset `8`;
- merge the resulting first two paragraphs by deleting their paragraph boundary.

Inside one live semantic view, a monotonically increasing probe token represents UNO same-object equality only. Tokens are view-local evidence and are never product identity.

Two independent CI executions reproduced the same relation.

Representative token trace:

```text
R0 before split:       (1, 2, 3)
R1 after split:        (4, 1, 2, 3)
R2 after merge:        (4, 2, 3)
```

CI pins only the relation, not the numeric token values:

```text
R0 -> R1 split
0 -> 1
1 -> 2
2 -> 3

R1 -> R2 merge
0 -> 0
1 -> deleted
2 -> 1
3 -> 2

R0 -> R2 semantic round trip
0 -> deleted
1 -> 1
2 -> 2
```

The first paragraph's original Writer object survives the split as the **right fragment**. Writer creates a new object for the **left fragment**. Merging the two fragments then preserves the left/new object and destroys the right/original object.

The final paragraph text is identical to the original paragraph text, but the original first-paragraph Writer object identity no longer exists.

### Conclusion: engine object identity

**UNO object identity is useful local continuity evidence, but it is not durable logical identity.**

A structural semantic round trip is not identity-invertible at the bootstrap-engine object level. Office history, comments, collaboration anchors and durable selections must therefore live above engine object identity.

The exact qualification, CI contract and architectural consequences are recorded in `STRUCTURAL_IDENTITY_QUALIFICATION.md`.

## What this spike proves

- a small semantic paragraph projection can be compared independently of binary DOCX bytes;
- the three fixture paragraphs survive the qualified edit/save/reopen path semantically;
- the persisted edit is localised to paragraph 1;
- external OOXML paragraph IDs cannot currently be relied upon for stable product identity;
- focused accessibility state is not a proven whole-document enumerator in the headless LOK process;
- `SelectAll` plus `getTextSelection()` is not a proven whole-document live-text extractor there;
- the exact live Writer document loaded by LOK can be acquired through a native UNO semantic model in the pinned 24.2 environment;
- the retained semantic view observes unsaved mutation made through the LOK document authority;
- no second office/document is needed for the qualified semantic path;
- revision-stamped ordered paragraph text can cross the isolated process boundary in a hard-bounded native-neutral response;
- semantic access is removed when the owning document closes and can be freshly reacquired after process restart;
- paragraph UNO same-object equality is repeatable within an unchanged live view;
- split/merge preserves some paragraph objects but is not logically invertible;
- semantic equality after a structural round trip does not imply restoration of engine-object identity;
- binary package size, probe-token numbers and rendered raster hashes remain observations rather than semantic goldens.

## What remains unresolved

- identity through insertion/deletion adjacent to retained paragraphs;
- identity through paragraph move/reorder;
- formatting-only edit behaviour;
- duplicate-text reconciliation where content matching is ambiguous;
- identity/reconciliation through save/reload when external file metadata is absent or rewritten;
- anchors inside a paragraph rather than only block identity;
- structural projection for tables, lists, fields, comments, tracked changes, images and other Writer structures;
- callback/invalidation ordering and its relation to semantic revisions;
- reconciliation after engine-process loss and restart;
- the production, versioned native compatibility mechanism for same-instance UNO access;
- the product-owned logical identity and reconciliation rules that will support durable history.

## Next experiments

The next R0A identity work should extend the same evidence path rather than invent another acquisition or transport mechanism:

1. insertion/deletion around retained paragraphs;
2. move/reorder;
3. formatting-only changes;
4. duplicate-text fixtures;
5. save/reload and worker-restart reconciliation;
6. callback/invalidation ordering relative to semantic revisions.

The constraints remain strict:

- retain the same-instance semantic authority and bounded process boundary;
- measure engine behaviour before inventing adapter IDs;
- distinguish stable engine evidence from product reconciliation policy;
- do not define product `ParagraphId` or durable anchors until the structural sequence establishes the necessary invariants;
- keep the internal 24.2 process-context dependency isolated until a production compatibility ADR is justified.

If Writer exposes no durable identity suitable for the product, Office will need an explicit product-owned identity/reconciliation layer rather than leaking engine addresses, hashing paragraph text or falling back to text offsets.

## Non-goals

This spike does not authorize:

- using `w14:paraId` as a product `ParagraphId`;
- using `TextOffset` as a history/comment/collaboration anchor;
- exposing UNO or LibreOffice object references to Rust product code;
- treating paragraph text hashes as identities;
- treating UNO object/reference identity or view-local probe tokens as product semantic identity;
- freezing the R0A paragraph snapshot encoding as the permanent wire schema;
- requiring LibreOffice to reproduce today's OOXML serialization details;
- launching a second office/document and treating its semantics as if they came from the live LOK authority;
- relying on accessibility focus or selection side effects as structural document traversal;
- promoting `comphelper::getProcessComponentContext()` into production code without explicit versioning and architectural review.
