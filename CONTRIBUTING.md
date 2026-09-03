# Contributing

This project is architecture-led. Before implementing a subsystem, read:

- `docs/product/PRODUCT_CONSTITUTION.md`
- `docs/architecture/ARCHITECTURE.md`
- `docs/engineering/ENGINEERING_CONTRACT.md`
- relevant ADRs and subsystem docs.

## Change expectations

- include tests for behaviour/invariants;
- update docs in the same change when a contract changes;
- do not add dependencies without documenting why the dependency belongs and how it is contained;
- do not create duplicate mechanisms when an existing service can be extended cleanly;
- keep platform/engine-specific code behind adapters;
- include performance evidence for changes to measured hot paths.

## Commit/PR structure

Prefer narrowly reviewable changes with explicit acceptance criteria. Architecture changes should generally land separately from large mechanical implementations.
