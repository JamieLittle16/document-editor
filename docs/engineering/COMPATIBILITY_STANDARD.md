# DOCX Compatibility Standard

## Dimensions

Do not collapse compatibility into a single “opens in Word” boolean.

Measure:

1. semantic fidelity;
2. visual fidelity;
3. pagination fidelity;
4. OOXML preservation;
5. round-trip fidelity;
6. edit fidelity after modification;
7. interoperability of review metadata.

## Corpus categories

- paragraph/character properties;
- styles and inheritance;
- numbering/lists;
- simple and nested tables;
- sections/columns;
- headers/footers;
- page-break/widow/orphan rules;
- images, anchors and wrapping;
- drawings/shapes;
- fields/TOC/cross-references;
- footnotes/endnotes;
- equations/OMML;
- comments;
- tracked changes;
- content controls;
- international scripts and bidi;
- fonts/substitution;
- malformed-but-common real-world files.

## Oracles

Where licensing/automation permits, compare against Microsoft Word as the primary compatibility oracle. LibreOffice and ONLYOFFICE are valuable secondary comparisons, not substitutes for Word behaviour when Word interoperability is the claim.

## Round-trip principle

For every supported feature, test at least:

```text
Word-produced input -> our edit -> saved DOCX -> Word reopen
```

Opening a fixture without editing is insufficient.

## Loss reporting

Known unsupported constructs must produce explicit compatibility diagnostics where loss is possible. Unknown content should be preserved when technically feasible.
