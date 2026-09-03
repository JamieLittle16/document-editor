# R0A Semantic Snapshot and Identity Spike

Status: qualification evidence, not a production document-model contract.

## Purpose

The application needs semantic identities that can support selection recovery, history, comments, diagnostics and eventually collaboration without depending on transient byte offsets, render coordinates or engine object addresses.

R0A therefore tests candidate identity signals before any permanent anchor model is designed.

The first experiment asked whether DOCX paragraph identity metadata could be reused as a stable paragraph identity while LibreOffice is the authoritative bootstrap engine. The second asked whether the public LibreOfficeKit view/accessibility surface could expose a deterministic whole-document **live, unsaved** semantic snapshot. The third asked whether the exact Writer document already loaded through LibreOfficeKit could be reached through a richer UNO semantic model **without creating a second document authority**.

All three questions are now constrained by executable evidence.

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

## Qualified same-instance Writer semantic access

R0A then tested a deeper native-only route. The qualification process:

1. initializes one LibreOffice process through `lok::lok_cpp_init`;
2. loads the deterministic DOCX exactly once through that LibreOfficeKit authority;
3. obtains the already-running LibreOffice process component context;
4. obtains the process `Desktop` and requires exactly one live Writer `XTextDocument`;
5. enumerates the three paragraphs from that `XTextDocument` through UNO semantic interfaces;
6. performs a prefix edit through the original LibreOfficeKit `Document` **without saving**;
7. re-enumerates through the same retained UNO `XTextDocument` reference;
8. requires paragraph count/order to remain stable, paragraph 1 to contain the unsaved marker, and paragraphs 2-3 to remain unchanged.

The qualified result is:

```text
same process component context: OK
Writer XTextDocument count: 1
paragraphs before edit: 3
paragraphs after edit: 3
same retained UNO reference sees unsaved LOK edit: OK
same-instance bridge: OK
```

This is stronger than merely observing the same file twice. No separate UNO bootstrap is performed, no second document is loaded, and the semantic reference observes a mutation that has not passed through save/reopen. The qualification therefore proves that, in the pinned reference environment, a richer native semantic layer can inspect the **same authoritative live Writer document** owned by the LibreOfficeKit process.

### Important implementation boundary

The bridge currently reaches the process context through LibreOffice's internal `comphelper::getProcessComponentContext()` symbol. The probe declares the exact LibreOffice **24.2** signature locally; that version returns the component-context reference by value. This matters because later LibreOffice source uses a different signature.

Therefore:

- the same-instance capability is a **hard CI qualification** for the pinned LibreOffice 24.2.7.2 reference environment;
- the internal symbol is **not** a product API and is not copied into a shared wrapper;
- UNO and LibreOffice references remain entirely native-side;
- Rust product crates gain no UNO dependency and no `unsafe` boundary from this result;
- a production native semantic adapter requires an explicit versioned compatibility layer and ADR before this mechanism can be adopted;
- upgrading the pinned LibreOffice version must requalify the native ABI rather than assuming source compatibility.

### Conclusion: semantic access

**Same-instance live Writer semantic access is now proven for the pinned R0A environment.**

This resolves the acquisition question. It does **not** resolve stable paragraph/object identity. UNO paragraph objects, text ranges, implementation addresses and other engine-local handles are not automatically product identities simply because we can now enumerate them.

## What this spike proves

- a small semantic paragraph projection can be compared independently of binary DOCX bytes;
- the three test paragraphs survive the qualified edit/save/reopen path semantically;
- the tested persisted edit is localised to the first paragraph's text;
- external OOXML paragraph IDs cannot currently be relied upon for stable product identity;
- focused accessibility state is not a proven whole-document enumerator in the headless LOK process;
- `SelectAll` plus `getTextSelection()` is not a proven whole-document live-text extractor in that process;
- the exact live Writer document loaded by LOK can be acquired and enumerated through a native UNO semantic model in the pinned 24.2 environment;
- the same retained UNO `XTextDocument` observes an unsaved mutation made through the LOK document authority;
- a second office/document is unnecessary for this qualified semantic-access path;
- binary package size and rendered raster hashes remain observations rather than semantic goldens;
- unsupported discovery behaviour is removed rather than weakened until it passes.

## What remains unresolved

- the bounded native-neutral semantic snapshot shape exposed across the engine process boundary;
- paragraph/object identity while the document remains open;
- identity through insertion, deletion, split, merge, move and formatting-only edits;
- identity through save/reload when external file metadata is absent or rewritten;
- anchors inside a paragraph rather than only block identity;
- tables, lists, fields, comments, tracked changes, images and other Writer structures;
- callback/invalidation ordering and its relation to semantic revisions;
- reconciliation after engine-process loss and restart;
- the production, versioned native compatibility mechanism for same-instance UNO access.

## Next experiment

The next R0A semantic experiment should build on the now-proven same-instance access rather than searching for another acquisition mechanism.

The design constraints are strict:

1. keep all LibreOffice/UNO/internal references inside the quarantined native engine process;
2. expose only normalized, bounded native-neutral records across the process boundary;
3. begin with the smallest useful Writer projection (paragraph order/text plus carefully selected structural metadata), not a mirror of the UNO object model;
4. tag semantic snapshots with explicit document/revision context rather than treating a query result as timeless;
5. run controlled insertion, deletion, split, merge, move and formatting-only edits and measure candidate identity signals;
6. distinguish stable engine-side properties from adapter reconciliation heuristics;
7. do not define a product `ParagraphId` or anchor contract until those edit sequences establish its invariants;
8. separately qualify save/reload reconciliation because live-instance stability does not imply persistence stability;
9. keep the current internal 24.2 process-context declaration local until a production native compatibility ADR is justified.

If the bootstrap engine provides no durable identity suitable for the product, the adapter will need an explicit identity/reconciliation layer rather than leaking engine addresses or falling back to text offsets.

## Non-goals

This spike does not authorize:

- using `w14:paraId` as a product `ParagraphId`;
- using `TextOffset` as a history/comment/collaboration anchor;
- exposing UNO or LibreOffice object references to Rust product code;
- treating paragraph text hashes as identities;
- treating UNO object/reference identity as product semantic identity;
- freezing a permanent semantic snapshot wire schema;
- requiring LibreOffice to reproduce today's OOXML serialization details;
- launching a second office/document and treating its semantics as if they came from the live LOK authority;
- relying on accessibility focus or selection side effects as structural document traversal;
- promoting `comphelper::getProcessComponentContext()` into production code without explicit versioning and architectural review.
