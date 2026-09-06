# Normalized compatibility harness

Status: R0A foundation

Date: 2026-09-06

## Purpose

Office needs a compatibility suite that can survive all three of these changes without being rewritten:

1. LibreOffice serialization changes that preserve document meaning;
2. replacement of the bootstrap Writer engine with a future native engine;
3. addition of external oracle runs against Word/other office suites.

For that reason the compatibility harness compares **normalized product-relevant semantics**, not DOCX ZIP bytes, native object identities or incidental file-format IDs.

The first vertical slice lives in:

```text
compatibility/fixtures.json
tools/compatibility/docx_semantics.py
tools/compatibility/runner.py
```

It is intentionally small. The architecture is the reusable part; the first semantic projection covers only ordered paragraph text.

## Manifest v1

The committed manifest is data, not executable code:

```json
{
  "schema": "office.compatibility-manifest.v1",
  "fixtures": [
    {
      "id": "writer-basic-text-prefix-roundtrip",
      "generator": "writer-r0a-basic-v1",
      "operation": "lok-r0a-prefix-edit-v1",
      "projection": "docx-paragraph-text-v1",
      "expected_before": ["..."],
      "expected_after": ["..."]
    }
  ]
}
```

The runner validates the schema and fixture keys strictly. Unknown schema fields are errors rather than silently ignored compatibility requirements.

### IDs are registry selectors, not shell commands

`generator`, `operation` and `projection` are stable selector IDs interpreted by the runner. The manifest cannot inject arbitrary executable paths or shell fragments.

This keeps compatibility data declarative and lets implementation adapters change behind a versioned identifier.

## First normalized projection

`docx-paragraph-text-v1` extracts ordered `w:p` paragraphs and concatenates their `w:t` runs.

It deliberately ignores:

- ZIP entry order/compression/timestamps;
- `w14:paraId` and `w14:textId`;
- Writer/UNO object identity;
- package byte count;
- raster hashes;
- styles, lists, tables, drawings and layout that this v1 projection does not yet model.

Ignoring an attribute is not a statement that the attribute is unimportant to the product. It means that attribute must receive an explicit normalized projection before a fixture claims to test it.

Future compatibility surfaces should therefore be added as focused versioned projections (for example paragraph style, list structure, table structure or page geometry) rather than turning paragraph text into an implicit universal document model.

## Real-engine execution

The first operation adapter reuses the qualified R0A LibreOfficeKit boundary probe:

```text
input DOCX
  -> open in Writer
  -> apply deterministic prefix edit
  -> save DOCX
  -> reopen in Writer
  -> output DOCX
```

The harness requires the native operation to report its successful edit/reopen contract and independently projects both the generated input package and the round-trip output package.

The existing semantic-identity script still runs after the compatibility fixture because it answers an additional specialist question about imported OOXML IDs. It no longer owns the ordinary open/edit/save semantic gate.

## Bounded execution

The R0A runner places explicit limits on:

- manifest bytes;
- fixture count;
- paragraph count;
- UTF-8 bytes per expected paragraph;
- generated/round-trip DOCX size;
- fixture-generator runtime;
- LibreOffice operation runtime.

These are harness admission limits, not final product document limits. Their purpose is to prevent a malformed fixture from silently turning CI into an unbounded workload.

## Artifacts

Each fixture receives a deterministic artifact directory containing:

```text
input.docx
roundtrip.docx
generator.stdout.txt
generator.stderr.txt
probe.stdout.txt
probe.stderr.txt
result.json
```

The harness also writes `summary.json` for the run.

`result.json` includes SHA-256 package hashes for diagnostics. **Those hashes are not compatibility goldens.** A semantically equivalent Writer version is allowed to serialize a different DOCX package.

CI retains the compatibility artifact tree when a native job fails so a regression has enough evidence to reproduce without rerunning blindly.

## Result schema

Per-fixture results currently use:

```text
office.compatibility-result.v1
```

A passing result records:

- fixture ID;
- generator/operation IDs;
- normalized before projection;
- normalized after projection;
- diagnostic input/output package hashes;
- pass status.

Machine-readable results are intended to become the common input for future cross-engine and external-oracle comparison.

## Local invocation

After building the existing R0A LibreOfficeKit probe, the harness can be invoked from the repository root with:

```bash
python3 -m tools.compatibility.runner \
  --manifest compatibility/fixtures.json \
  --probe /path/to/document-editor-lok-probe \
  --install-path /usr/lib/libreoffice/program \
  --profile-root /tmp/document-editor-compatibility-profiles \
  --artifact-root /tmp/document-editor-compatibility
```

The runner uses only the Python standard library.

Unit tests that do not require LibreOffice run in the ordinary CI job:

```bash
python3 -m unittest tools.compatibility.test_compatibility
```

## Evolution rules

1. Do not compare binary DOCX equality as a product compatibility assertion.
2. Do not promote engine IDs, pointers or qualification tokens into fixture identity.
3. Add a versioned normalized projection when a new semantic dimension matters.
4. Keep fixture manifests declarative; executable behavior belongs in reviewed adapters/registries.
5. Preserve old projection meaning. Breaking normalization changes require a new projection/schema version.
6. Capture artifacts for diagnosis, but separate diagnostics from pass/fail semantics.
7. Prefer a few small orthogonal fixtures over one giant document whose failure cannot be localized.
8. When a second engine/oracle is added, run the same manifest expectations wherever possible rather than creating engine-specific golden documents.

## Next expansion

After this first text round-trip fixture is stable, useful additions are:

- paragraph and character formatting preservation;
- list/numbering structure;
- table structure and cell text;
- image/anchor preservation;
- section/page geometry;
- headers/footers;
- comments/track changes when those become product scope;
- a second oracle adapter for independent cross-engine comparison.

The harness should grow with product semantics, not with LibreOffice implementation detail.
