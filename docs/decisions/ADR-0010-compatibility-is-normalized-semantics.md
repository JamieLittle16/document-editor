# ADR-0010: Compatibility assertions use normalized semantics, not binary package equality

Status: accepted for R0A

Date: 2026-09-06

## Context

Office uses LibreOffice Writer as a quarantined bootstrap document authority, but the product architecture is explicitly intended to survive replacement of that engine. DOCX is also a ZIP/XML package whose byte representation can change for reasons that do not change document meaning: entry ordering, compression, generated IDs, namespace serialization and producer-specific metadata are all examples.

Earlier R0A qualification already demonstrated one concrete instance: LibreOffice 24.2.7.2 removed seeded `w14:paraId` and `w14:textId` values while preserving the paragraph semantics relevant to the tested edit. Raster and package hashes likewise remain useful diagnostics but are not generally stable semantic identities.

A compatibility framework based on binary package equality would therefore make Office depend on serialization accidents and on the current bootstrap engine. Conversely, a single giant hand-written semantic model would prematurely duplicate the document engine and make the test framework its own second authority.

## Decision

Office compatibility fixtures are defined by **small, explicit, versioned normalized semantic projections**.

A compatibility fixture declares:

- a stable fixture identifier;
- a reviewed generator/fixture source;
- an operation adapter;
- a named normalized projection;
- expected semantic state before the operation;
- expected semantic state after the operation.

The manifest is declarative. Generator, operation and projection values are registry selectors rather than executable shell fragments.

### Normalization is explicit and composable

Each projection has narrow documented meaning. The initial `docx-paragraph-text-v1` projection covers only ordered paragraph text. It does not silently claim that styles, lists, tables, images or layout are irrelevant.

When a new semantic dimension becomes product-relevant, it receives a new explicit projection or a versioned extension rather than being inferred from package bytes.

### Binary artifacts remain diagnostic

Input/output packages, logs and hashes may be retained for debugging and reproducibility, but package byte equality and diagnostic hashes are not pass/fail compatibility goldens unless a future narrowly scoped contract explicitly requires exact bytes.

### Engine adapters do not change fixture meaning

The first R0A operation adapter uses LibreOfficeKit. A future native engine or external Word oracle should consume the same fixture expectations wherever its capabilities overlap.

Engine-specific control mechanisms may differ behind adapters; the normalized compatibility expectation remains product-owned.

### Compatibility tooling is bounded

Manifest size, fixture count, semantic payload size, generated package size and execution times are explicitly bounded in the harness. A malformed fixture must fail admission rather than create unbounded CI work.

## Consequences

### Positive

- LibreOffice serialization quirks do not become product compatibility requirements;
- the same fixture corpus can compare the bootstrap engine, future native engine and external oracles;
- failures localize to named semantic projections rather than opaque package diffs;
- compatibility infrastructure does not become a second complete document model;
- diagnostic artifacts remain available when semantic assertions fail;
- the fixture corpus can expand incrementally with actual product scope.

### Costs

- each new product-relevant semantic dimension requires a deliberate normalized projection;
- normalization code itself becomes reviewed test infrastructure and must be versioned carefully;
- package-level regressions that are not yet represented semantically may initially require separate specialist checks;
- external oracle adapters will need to map their outputs into the same normalized result vocabulary.

## Invariants

1. Binary DOCX equality is not the default compatibility definition.
2. Engine object identity, pointers, qualification tokens and incidental file-format IDs are not fixture semantic identity.
3. A projection name has stable documented meaning; breaking changes use a new version.
4. Fixture manifests do not execute arbitrary commands.
5. Diagnostic hashes/artifacts are separate from semantic pass/fail assertions.
6. The harness must remain bounded and deterministic enough for mandatory CI.
7. Specialist engine qualifications may coexist with the harness when they test facts outside the current normalized product projection, but they must not duplicate the ordinary compatibility gate indefinitely.

## Testing requirements

R0A CI must:

- unit-test manifest admission and normalized DOCX projection without requiring LibreOffice;
- run at least one real Writer fixture through generate -> open/edit/save/reopen -> normalized before/after assertion;
- emit machine-readable fixture results and preserve diagnostic artifacts on failure;
- continue specialist identity qualification separately where it measures non-product engine behavior.

## Non-decisions

This ADR does not yet decide:

- the complete set of R1 semantic projections;
- a Word automation/oracle implementation;
- whether future fixtures are source-generated or committed binary files;
- the final cross-platform native-engine test transport;
- visual pixel-diff policy for explicitly visual compatibility tests.
