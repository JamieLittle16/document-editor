# Product Constitution

Status: **Normative**
Revision: **0.1**

## Mission

Build a local-first, high-performance, Word-compatible document editor that a demanding user voluntarily prefers to Microsoft Word and ONLYOFFICE. It must be excellent for ordinary correspondence, university/work documents, technical material, books, and long-form publishing.

Compatibility is a boundary condition, not an instruction to copy historical internals.

## Product hierarchy

When priorities conflict, optimise in this order:

1. Never lose user work.
2. Never silently corrupt user work.
3. Preserve format semantics and compatibility.
4. Keep direct interaction responsive.
5. Make frequent operations effortless.
6. Make serious writing materially better than incumbent tools.
7. Add advanced capability.
8. Expand into a broader office platform.

## Switching test

The editor reaches daily-driver quality only when we voluntarily choose it for normal work.

The stronger acceptance workload is a 400–600 page technical publication containing styles, sections, equations, tables, figures, references, headers/footers, contents, page constraints, and extensive proofreading.

It must make that workload *more pleasant* than Word or ONLYOFFICE, not merely possible.

## Product principles

### Perceptual performance

No user input waits synchronously for an engine, disk, network, language model, plugin, or background analysis. Slow services may answer later; they may not make the interaction loop slow.

### Local-first

Opening, editing, saving, printing, searching, ordinary spelling, history and recovery work offline and without an account.

### User-owned files

DOCX is first-class. We do not force users into a proprietary cloud representation to obtain a good experience.

### Preserve unknown content

Unsupported OOXML is preserved where feasible or reported explicitly. Silent destructive round-tripping is unacceptable.

### Structured, reviewable automation

AI and plugins propose or execute typed, permissioned transactions. They do not invisibly rewrite arbitrary document bytes.

### Architectural replaceability

Bootstrap dependencies must not become permanent merely because they were convenient first.

### Focus before breadth

The first product is a document editor. Spreadsheets and presentations are future modules inside the same suite shell, not excuses to dilute the first product.

## How we intend to beat incumbents

- interaction responsiveness and predictable performance;
- strong local recovery and meaningful version history;
- comprehensible styles and direct-formatting diagnostics;
- first-class spelling, grammar, terminology and consistency analysis;
- IDE-quality navigation and semantic search for large documents;
- excellent equations, references and technical-writing workflows;
- document-health diagnostics covering structure, layout and references;
- privacy-respecting, revision-safe AI;
- secure capability-based plugins rather than unrestricted macros;
- explicit compatibility diagnostics instead of hidden loss.

## Non-goals for early releases

- spreadsheet editor;
- presentation editor;
- email client;
- database frontend;
- arbitrary VBA execution;
- desktop-publishing replacement;
- diagram suite;
- cloud account requirement;
- giant template marketplace.
