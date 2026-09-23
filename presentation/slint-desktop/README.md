# Provisional Slint Desktop Shell

This is the first runnable graphical Office editor shell.

It is intentionally a **replaceable presentation adapter**, not a UI-framework selection decision:

- the product correctness workspace remains Rust 1.85 with `unsafe_code = "forbid"`;
- this adapter uses Rust 1.92 only because the qualified Slint 1.17.1 candidate requires it;
- document authority stays in `app-core -> document-session -> DocumentEngine`;
- widget state never becomes the authoritative document model;
- user edits enter through `AppCore::replace_document_text`, which owns protocol/revision translation.

The current engine is the deterministic mock engine so this vertical slice can qualify editor interaction independently of the heavyweight process adapter. The next product slice replaces that injected engine with the supervised bootstrap Writer worker and feeds the qualified render data plane into the viewport.

Run from this directory:

```bash
cargo run
```

On Linux with the Winit/software backend, install the same fontconfig/XKB runtime dependencies qualified by the UI spike.
