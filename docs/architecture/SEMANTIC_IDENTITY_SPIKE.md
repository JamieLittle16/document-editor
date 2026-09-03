# R0A Semantic Snapshot and Identity Spike

Status: qualification evidence, not a production document-model contract.

## Purpose

The application needs semantic identities that can support selection recovery, history, comments, diagnostics and eventually collaboration without depending on transient byte offsets, render coordinates or engine object addresses.

R0A therefore tests candidate identity signals before any permanent anchor model is designed.

The first experiment asked whether DOCX paragraph identity metadata could be reused as a stable paragraph identity while LibreOffice is the authoritative bootstrap engine. The second experiment asked whether the public LibreOfficeKit view/accessibility surface can expose a deterministic whole-document **live, unsaved** semantic snapshot from that authoritative instance.

Both candidate routes are now constrained by executable evidence.

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

The projection is read directly from `word/document.xml` before and after the existing real LibreOfficeKit edit/save/reopen qualification.

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

## Public LibreOfficeKit live-semantic discovery

After rejecting file-format IDs, R0A moved inward and tried to extract semantics directly from the live Writer instance without introducing UNO/internal dependencies.

Two deliberately narrow public-LOK attempts were run against the same qualified environment.

### Attempt 1: accessibility-focused paragraph enumeration

The native adapter enabled accessibility, moved to the document start, read `getA11yFocusedParagraph()`, issued `.uno:GoToNextPara`, and attempted to enumerate the three fixture paragraphs.

Observed result:

```text
expected live paragraphs: 3
observed live paragraphs: 1
observed content: paragraph 1 only
```

`getA11yFocusedParagraph()` successfully exposed the focused paragraph, but the headless `.uno:GoToNextPara` sequence did not synchronously advance the accessibility focus seen by the API. The call therefore remains useful as focused-view state, not as a proven whole-document enumeration primitive.

### Attempt 2: select-all + text-selection snapshot

The second attempt narrowed the claim to plain live text. The adapter issued `.uno:SelectAll` and immediately called `getTextSelection("text/plain;charset=utf-8", ...)`.

Observed result:

```text
selection call: returned
selected live text: empty
```

This was despite the existing `.uno:SelectAll` + `paste(...)` edit path already being qualified for mutation. In this headless process configuration, selection mutation behaviour therefore does not imply that `getTextSelection()` provides deterministic whole-document extraction.

### Conclusion: public LOK snapshot surface

**The currently qualified public LibreOfficeKit view/accessibility surface is not accepted as a deterministic whole-document live semantic snapshot API.**

This is a capability conclusion for the tested LibreOffice 24.2.7.2 environment, not a claim that no future LibreOfficeKit API could provide such a surface.

The failed discovery commands were removed from the mandatory native process harness. The harness continues to gate only behaviour that is actually proven: initialization, load, typed failure, bounded framing, graceful shutdown, forced death and fresh restart. Failed semantic discovery remains documented evidence rather than a fake supported command.

## What this spike proves

- a small semantic paragraph projection can be compared independently of binary DOCX bytes;
- the three test paragraphs survive the qualified edit/save/reopen path semantically;
- the tested persisted edit is localised to the first paragraph's text;
- external OOXML paragraph IDs cannot currently be relied upon for stable product identity;
- focused accessibility state is not a proven whole-document enumerator in the headless LOK process;
- `SelectAll` plus `getTextSelection()` is not a proven whole-document live-text extractor in that process;
- binary package size and rendered raster hashes remain observations rather than semantic goldens;
- unsupported discovery behaviour is removed from the gating adapter rather than weakened until it passes.

## What remains unresolved

- live, unsaved semantic snapshot extraction from the authoritative Writer instance;
- access to the same live Writer model through a richer native API without creating a second authoritative document instance;
- paragraph/object identity while the document remains open;
- identity through insertion, deletion, split, merge, move and formatting-only edits;
- identity through save/reload when external file metadata is absent or rewritten;
- anchors inside a paragraph rather than only block identity;
- tables, lists, fields, comments, tracked changes, images and other Writer structures;
- reconciliation after engine-process loss and restart.

## Next experiment

The next R0A identity experiment should qualify a **deeper native semantic shim**, rather than invent another file-format heuristic or coercing view APIs into a document-model API.

The design constraints are strict:

1. the semantic shim must operate on the **same authoritative Writer document instance** used by the engine process;
2. a separate UNO-bootstrap office/document is not acceptable evidence for shared authority;
3. LibreOffice/UNO/internal object references remain native-side and never cross the process boundary;
4. only normalized, bounded semantic records may cross the boundary;
5. the spike must prove how the native shim obtains the current Writer model before it is allowed to enumerate paragraphs;
6. paragraph enumeration should then use Writer/UNO semantic interfaces rather than caret/view navigation;
7. controlled edit sequences must test identity/reconciliation behaviour before any product `ParagraphId` or anchor contract is designed.

LibreOffice's UNO text model does provide semantic paragraph enumeration once an `XTextDocument`/`XText` for the correct document is available. The open problem is therefore **same-instance acquisition**, not whether UNO can enumerate paragraphs in principle.

If the bootstrap engine provides no durable identity suitable for the product, the adapter will need an explicit identity/reconciliation layer rather than leaking engine addresses or falling back to text offsets.

## Non-goals

This spike does not authorize:

- using `w14:paraId` as a product `ParagraphId`;
- using `TextOffset` as a history/comment/collaboration anchor;
- exposing UNO or LibreOffice object references to Rust product code;
- treating paragraph text hashes as identities;
- freezing a permanent semantic snapshot wire schema;
- requiring LibreOffice to reproduce today's OOXML serialization details;
- launching a second office/document and treating its semantics as if they came from the live LOK authority;
- relying on accessibility focus or selection side effects as structural document traversal.
