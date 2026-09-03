# R0A Semantic Snapshot and Identity Spike

Status: qualification evidence, not a production document-model contract.

## Purpose

The application needs semantic identities that can support selection recovery, history, comments, diagnostics and eventually collaboration without depending on transient byte offsets, render coordinates or engine object addresses.

R0A therefore tests candidate identity signals before any permanent anchor model is designed.

The first experiment deliberately asks a narrow question: can DOCX paragraph identity metadata be reused as a stable paragraph identity while LibreOffice is the authoritative bootstrap engine?

## Fixture

The deterministic DOCX qualification fixture contains three Writer paragraphs with known text and explicitly seeded Word 2010 paragraph metadata:

```text
paragraph 1: w14:paraId=13579BDF, w14:textId=2468ACE0
paragraph 2: w14:paraId=89ABCDEF, w14:textId=10293847
paragraph 3: w14:paraId=A1B2C3D4, w14:textId=55667788
```

These values are test probes only. They are not application identifiers.

The semantic projection used by the test is intentionally small:

```text
Paragraph {
    external_para_id?: string,
    external_text_id?: string,
    text: string,
}
```

The projection is read directly from `word/document.xml` before and after the existing real LibreOfficeKit edit/save/reopen qualification.

## Qualified observation

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
matched semantic paragraphs:  3
w14:paraId values present:    0
w14:paraId preserved:         0 / 3
w14:textId preserved:         0 / 3
```

The semantic paragraph texts remained identifiable and in the same order. The edit marker appeared at the start of paragraph 1. All seeded `w14:paraId` and `w14:textId` attributes were absent from the saved round-trip OOXML.

## Conclusion

**DOCX `w14:paraId` and `w14:textId` are rejected as the product's semantic identity mechanism.**

This conclusion is architectural: an identity mechanism that disappears during the qualified bootstrap engine's ordinary save path cannot be authoritative for product state.

The exact stripping behaviour is *not* a permanent compatibility requirement. Future LibreOffice versions may preserve, regenerate or otherwise transform these fields. CI records the observation but must not require LibreOffice to keep stripping them forever.

Likewise, content equality alone is not a sufficient identity system. Duplicate paragraphs, splits, merges, moves and edits make text matching ambiguous in real documents.

## What this spike proves

- a small semantic paragraph projection can be compared independently of binary DOCX bytes;
- the three test paragraphs survive the qualified edit/save/reopen path semantically;
- the tested edit is localised to the first paragraph's text in the saved projection;
- external OOXML paragraph IDs cannot currently be relied upon for stable product identity;
- binary package size and rendered raster hashes remain observations rather than semantic goldens.

## What remains unresolved

- live, unsaved semantic snapshot extraction from the authoritative Writer instance;
- paragraph/object identity while the document remains open;
- identity through insertion, deletion, split, merge, move and formatting-only edits;
- identity through save/reload when external file metadata is absent or rewritten;
- anchors inside a paragraph rather than only block identity;
- tables, lists, fields, comments, tracked changes, images and other Writer structures;
- reconciliation after engine-process loss and restart.

## Next experiment

The next R0A identity experiment should move inward rather than invent another file-format heuristic:

1. expose a minimal **live semantic snapshot** from the quarantined LibreOffice-side adapter;
2. keep all LibreOffice/UNO/internal types on the native side;
3. return only normalized, bounded semantic records across the process boundary;
4. inspect what stable engine-side properties exist for paragraphs and richer objects;
5. exercise those records through controlled edit sequences and save/reload;
6. only then design the product's semantic identity/anchor model.

If the bootstrap engine provides no durable identity suitable for the product, the adapter will need an explicit identity/reconciliation layer rather than leaking engine addresses or falling back to text offsets.

## Non-goals

This spike does not authorize:

- using `w14:paraId` as a product `ParagraphId`;
- using `TextOffset` as a history/comment/collaboration anchor;
- exposing UNO or LibreOffice object references to Rust product code;
- treating paragraph text hashes as identities;
- freezing a permanent semantic snapshot wire schema;
- requiring LibreOffice to reproduce today's OOXML serialization details.
