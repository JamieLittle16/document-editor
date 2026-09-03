# Document Editor / Office Suite (provisional name)

A local-first, high-performance, Word-compatible document editor designed as the first product in a future cohesive office suite.

The repository name and branding are intentionally provisional. Architecture is not.

## Mission

Build the document editor we would voluntarily choose over Microsoft Word and ONLYOFFICE for everyday, technical, and long-form writing: fast, reliable, excellent at DOCX interoperability, unusually strong at proofreading and document diagnostics, and architecturally capable of evolving beyond its bootstrap engine.

## Architectural position

- **Rust-led application architecture.**
- **LibreOffice Writer is a bootstrap compatibility/layout engine**, isolated behind a versioned engine protocol.
- **LibreOffice is never the application architecture.** UNO/VCL/SFX/Writer implementation types do not cross the adapter boundary.
- **One authoritative document model at a time.** During the bootstrap phase, the compatibility engine owns complete document state; Rust maintains revisioned semantic projections for product features.
- **Heavyweight engines run out of process.** A document worker can crash, hang, or be restarted without taking down the shell or other documents.
- **Every mutation is conceptually transactional and revisioned.** This underpins undo, recovery, collaboration, plugins, diagnostics, and AI edits.
- **Local-first and offline-capable.** Cloud and AI are enhancements rather than requirements.
- **One suite shell eventually; separate domain modules and engines internally.** Documents come first. Spreadsheet and presentation work begins only after the document editor is excellent.

## Current status

**Phase R0A: architecture and technical spikes.**

The initial workspace deliberately contains no GUI framework or LibreOffice dependency. R0A first freezes the contracts that expensive dependencies must satisfy, then validates them with replaceable adapters.

## Repository map

```text
apps/desktop/                    future desktop shell
crates/app-core/                 editor-agnostic application orchestration
crates/document-protocol/        versioned protocol value types
crates/document-engine-api/      engine abstraction
crates/document-session/         revisions, anchors, session state
crates/document-engine-mock/     deterministic reference/mock engine
workers/document-worker/         isolated document-engine host

docs/product/                    what we are building and why
docs/architecture/               hard subsystem contracts
docs/engineering/                engineering, quality, debt and AI rules
docs/decisions/                  architecture decision records (ADRs)
docs/roadmap/                    release programme
```

## Start here

1. [`docs/product/PRODUCT_CONSTITUTION.md`](docs/product/PRODUCT_CONSTITUTION.md)
2. [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md)
3. [`docs/engineering/ENGINEERING_CONTRACT.md`](docs/engineering/ENGINEERING_CONTRACT.md)
4. [`docs/product/R1_FEATURE_MATRIX.md`](docs/product/R1_FEATURE_MATRIX.md)
5. [`docs/product/90_DAY_PLAN.md`](docs/product/90_DAY_PLAN.md)
6. [`docs/engineering/TECHNICAL_DEBT.md`](docs/engineering/TECHNICAL_DEBT.md)
7. [`docs/decisions/`](docs/decisions/)

## Build

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

## Documentation rule

A subsystem is not considered implemented if a capable new contributor or coding agent cannot discover its responsibilities, invariants, dependencies, test strategy and failure modes from the repository. See [`docs/engineering/DOCUMENTATION_POLICY.md`](docs/engineering/DOCUMENTATION_POLICY.md).

## Licence

Not chosen yet. The repository remains `UNLICENSED` until we deliberately decide the product and contribution licensing model; do not accidentally make a strategic licensing decision by copying a template.
