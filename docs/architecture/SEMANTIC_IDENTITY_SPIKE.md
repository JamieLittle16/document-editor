# R0A Semantic Snapshot and Identity Spike

Status: qualification evidence, not a production document-model contract.

## Purpose

The application needs semantic identities that can support selection recovery, history, comments, diagnostics and eventually collaboration without depending on transient byte offsets, render coordinates or engine object addresses.

R0A therefore tests candidate identity signals before any permanent anchor model is designed.

The evidence now covers four questions:

1. can DOCX paragraph metadata serve as durable product identity? **No.**
2. can public LibreOfficeKit view/accessibility APIs provide deterministic whole-document live semantics? **Not in the qualified headless configuration.**
3. can a richer native semantic layer reach the exact Writer document already owned by LibreOfficeKit without creating a second authority? **Yes, in the pinned 24.2 environment.**
4. can a normalized live semantic snapshot cross the isolated engine process boundary without exposing UNO types? **Yes, for the bounded R0A paragraph projection.**

Stable paragraph/object identity is still deliberately unresolved.

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

After LibreOfficeKit loaded the DOCX, inserted the existing R0A text marker, saved a new DOCX and reopened it:

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

The exact stripping behaviour is *not* a permanent compatibility requirement. Future LibreOffice versions may preserve, regenerate or otherwise transform these fields. CI records the observation but does not require LibreOffice to keep stripping them forever.

Likewise, content equality alone is not a sufficient identity system. Duplicate paragraphs, splits, merges, moves and edits make text matching ambiguous in real documents.

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

The dependency is confined to:

```text
spikes/libreofficekit-process-adapter/writer_semantics_24_2.cxx
```

The matching header exposes only an opaque C++ `WriterSemanticView` plus ordinary strings/vectors. UNO references and internal LibreOffice types do not cross that boundary.

Therefore:

- same-instance semantic access is a hard qualification for the pinned LibreOffice 24.2.7.2 environment;
- the internal symbol is **not** a product API;
- Rust product crates gain no UNO dependency and no `unsafe` boundary;
- a production native semantic adapter requires an explicit versioned compatibility layer and ADR;
- upgrading LibreOffice must requalify this ABI instead of assuming source compatibility.

## Qualified bounded process-boundary snapshot

The same-instance result is now integrated into the existing killable native process adapter rather than maintained as a second standalone bridge executable.

The process adapter exposes a deliberately disposable R0A semantic command whose successful payload is:

```text
status:u8
command:u8
projection_version:u8
paragraph_count:u16-le
repeat paragraph_count times:
    byte_length:u16-le
    utf8_text[byte_length]
```

Projection version 1 contains only ordered paragraph text. The complete response must fit the adapter's existing **1024-byte** control-frame payload bound. Oversized semantic results are rejected with a typed qualification-limit response rather than becoming an accidental unbounded transport path.

The host harness proves:

```text
open fixture: OK
bounded snapshot before edit: exact 3 fixture paragraphs
unsaved LOK prefix edit: OK
bounded snapshot after edit: prefix visible only in paragraph 1
same retained native semantic view: preserved across the edit
close document: semantic access removed
force-kill with live Writer state: observed
fresh process restart/reopen: OK
fresh bounded snapshot after restart: original 3 fixture paragraphs
```

Only native-neutral bytes cross `DETR`. No UNO reference, engine address or Writer implementation object leaves the native process.

### Conclusion: snapshot boundary

**The project now has a proven bounded live semantic observation seam across the isolated engine process boundary.**

This establishes where semantic information can flow. It does not establish what the permanent semantic schema or identity model should be.

The standalone discovery probe has been removed; same-instance acquisition and process-boundary semantic qualification now live in one native adapter path, reducing duplicate mechanisms.

## What this spike proves

- a small semantic paragraph projection can be compared independently of binary DOCX bytes;
- the three fixture paragraphs survive the qualified edit/save/reopen path semantically;
- the persisted edit is localised to paragraph 1;
- external OOXML paragraph IDs cannot currently be relied upon for stable product identity;
- focused accessibility state is not a proven whole-document enumerator in the headless LOK process;
- `SelectAll` plus `getTextSelection()` is not a proven whole-document live-text extractor there;
- the exact live Writer document loaded by LOK can be acquired through a native UNO semantic model in the pinned 24.2 environment;
- the retained semantic view observes an unsaved mutation made through the LOK document authority;
- no second office/document is needed for the qualified semantic path;
- ordered paragraph text can cross the actual isolated process boundary in a hard-bounded native-neutral response;
- semantic access is removed when the owning document closes and can be freshly reacquired after process restart;
- binary package size and rendered raster hashes remain observations rather than semantic goldens.

## What remains unresolved

- stable paragraph/object identity while the document remains open;
- identity through insertion, deletion, split, merge, move and formatting-only edits;
- identity through save/reload when external file metadata is absent or rewritten;
- anchors inside a paragraph rather than only block identity;
- explicit document/revision tagging for live semantic snapshots;
- structural projection for tables, lists, fields, comments, tracked changes, images and other Writer structures;
- callback/invalidation ordering and its relation to semantic revisions;
- reconciliation after engine-process loss and restart;
- the production, versioned native compatibility mechanism for same-instance UNO access.

## Next experiment

The next R0A semantic experiment should test **identity and reconciliation**, not add another way to acquire or transport paragraph text.

The design constraints are strict:

1. retain the current same-instance semantic view and bounded process boundary;
2. add only the structural metadata needed to test a concrete identity hypothesis;
3. tag observations with explicit document/revision context before host-side caching becomes real;
4. run deterministic insertion, deletion, split, merge, move and formatting-only edit sequences;
5. measure candidate engine-side identity/property behaviour before inventing an adapter ID;
6. distinguish stable engine properties from reconciliation heuristics;
7. do not define a product `ParagraphId` or anchor contract until those edit sequences establish its invariants;
8. separately qualify save/reload reconciliation because live-instance stability does not imply persistence stability;
9. keep the internal 24.2 process-context dependency isolated until a production compatibility ADR is justified.

If Writer exposes no durable identity suitable for the product, the adapter will need an explicit identity/reconciliation layer rather than leaking engine addresses, hashing paragraph text or falling back to text offsets.

## Non-goals

This spike does not authorize:

- using `w14:paraId` as a product `ParagraphId`;
- using `TextOffset` as a history/comment/collaboration anchor;
- exposing UNO or LibreOffice object references to Rust product code;
- treating paragraph text hashes as identities;
- treating UNO object/reference identity as product semantic identity;
- freezing the R0A paragraph snapshot encoding as the permanent wire schema;
- requiring LibreOffice to reproduce today's OOXML serialization details;
- launching a second office/document and treating its semantics as if they came from the live LOK authority;
- relying on accessibility focus or selection side effects as structural document traversal;
- promoting `comphelper::getProcessComponentContext()` into production code without explicit versioning and architectural review.
