# R1 Daily-Driver Feature Matrix

Status labels: `R0`, `R1`, `R2+`, `Research`.

| Area | Capability | Target | Acceptance idea |
|---|---|---:|---|
| Files | Open/save DOCX | R0 | round-trip curated corpus without silent critical loss |
| Files | Atomic save + recovery | R0/R1 | forced kill and crash-injection tests |
| View | Paginated print layout | R0 | smooth navigation, correct page geometry |
| View | Zoom + continuous scrolling | R0 | no blocking render path |
| Editing | Text insertion/deletion/selection | R0 | Unicode + undo/recovery stress |
| Editing | Undo/redo | R0/R1 | deterministic mutation histories |
| Formatting | Character formatting | R1 | Word round-trip fixtures |
| Formatting | Paragraph formatting | R1 | Word round-trip fixtures |
| Styles | Apply/create/update/inspect styles | R1 | direct-formatting source visible |
| Lists | Bullets/numbering/multilevel | R1 | compatibility corpus |
| Tables | Create/edit/resize/merge/split | R1 | keyboard + large-table tests |
| Images | Insert/resize/crop/basic wrap | R1 | anchor/wrap compatibility tests |
| Layout | Sections/page breaks/margins | R1 | pagination fixtures |
| Layout | Headers/footers | R1 | odd/even/first-page fixtures |
| Search | Fast text search/replace | R1 | first useful result under budget |
| Navigation | Outline/headings/pages | R1 | 500-page workload |
| Language | Fast spelling | R1 | UK/US language profile + custom terms |
| Language | Grammar backend | R1 | asynchronous, no typing stalls |
| Output | PDF | R1 | publication-quality regression corpus |
| Output | Print | R1 | OS integration qualification |
| Review | Comments | R2+ | Word interoperability |
| Review | Track changes | R2+ | professional review corpus |
| Technical | Equations | R2+ | OMML/MathML/LaTeX workflows |
| Technical | Citations/references | R2+ | reference integrity diagnostics |
| Diagnostics | Document health | R2+ | structure/layout/language/reference rules |
| AI | Revision-safe suggestions | R2+ | stale-suggestion rejection tests |
| Plugins | Capability WASM model | R3+ | permission/security qualification |
| Suite | Spreadsheet module | Later | only after document editor quality gate |
| Suite | Presentation module | Later | only after document editor quality gate |
