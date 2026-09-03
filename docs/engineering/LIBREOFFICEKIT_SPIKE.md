# LibreOfficeKit R0A Qualification Spike

Status: **Active technical spike; not a production API contract**

## Purpose

LibreOffice is the bootstrap compatibility/layout engine, but the product must not inherit its architecture. Before adding a Rust FFI crate or freezing engine protocol operations around LibreOfficeKit, we need executable evidence of the smallest boundary that works on our reference Linux environment.

The standalone probe lives in `spikes/libreofficekit-probe/` and is intentionally outside the Rust workspace.

## Questions this spike answers

1. Can we initialise LibreOfficeKit from an explicit installation directory without embedding the LibreOffice application UI?
2. Can we use an isolated user-profile URI suitable for per-worker process isolation?
3. Can we open a valid DOCX through LibreOfficeKit?
4. Can we identify the loaded document as Writer/text?
5. Can we obtain non-zero layout dimensions?
6. Can caller-owned memory receive a rendered tile?
7. Can we save to DOCX and reopen the resulting package?
8. What actual LibreOffice version/build did the qualification environment execute?

## Deliberate architecture boundary

The spike is C++, not Rust.

Reasons:

- LibreOfficeKit exposes an ABI-oriented C API and a small C++ wrapper;
- tiled rendering is currently part of the `LOK_USE_UNSTABLE_API` surface;
- direct Rust FFI would necessarily introduce `unsafe` code;
- we do not yet know which subset deserves a stable adapter;
- product-level engine protocol types must not mirror external ABI structs/function tables.

The production Rust workspace therefore keeps `unsafe_code = "forbid"` while this experiment runs.

No LibreOffice header/type may be added to `document-engine-api`, `document-protocol`, `document-session`, `app-core` or feature crates as a consequence of this spike.

## Probe flow

```text
CI
 |
 +-- install LibreOfficeKit headers + no-GUI Writer runtime
 |
 +-- generate deterministic minimal DOCX
 |
 +-- compile standalone C++ probe
 |
 +-- lok_cpp_init(install path, isolated profile URL)
 |
 +-- documentLoad(DOCX)
 |
 +-- validate text document
 |
 +-- initializeForRendering
 |
 +-- getDocumentSize + getTileMode
 |
 +-- paintTile into caller-owned 256x256x4 buffer
 |
 +-- saveAs(roundtrip.docx)
 |
 +-- destroy original document
 |
 +-- reopen roundtrip.docx
 `-- validate package structure
```

The render hash printed by CI is diagnostic evidence, **not** a golden image checksum. Font/package changes can legitimately change raster output while the boundary remains correct.

## Current external-API observations

The upstream C++ wrapper constructs an `Office` through `lok_cpp_init()` and loads documents through `Office::documentLoad()`. The wrapper owns/destroys the underlying C handles.

Current upstream tiled rendering (`initializeForRendering`, `getDocumentSize`, `paintTile`, `getTileMode`) is exposed under `LOK_USE_UNSTABLE_API`. This is a strong reason not to treat the LibreOfficeKit rendering ABI as our product protocol.

The loader accepts an explicit installation path and user-profile URL. Worker startup must eventually create a private profile location rather than sharing the user's normal LibreOffice profile.

## Acceptance criteria

The CI job fails unless all of the following are true:

- LibreOfficeKit initialises;
- version information is returned;
- input DOCX loads;
- document type is text;
- width and height are positive;
- tile mode is a recognised 4-byte RGBA/BGRA mode;
- `paintTile` modifies the caller-owned buffer;
- DOCX `saveAs` succeeds;
- round-trip DOCX reopens as text;
- reopened layout size is positive;
- round-trip file is a valid ZIP package containing `word/document.xml`.

## Failure semantics

This is a qualification job. Any probe failure is a red CI result and blocks us from claiming the boundary works on the reference environment.

It does not yet define runtime recovery. The production engine worker will later convert init/load/render failures into typed engine/transport diagnostics and be restartable independently of the desktop shell.

## Resource bounds

The current probe allocates exactly one 256 x 256 x 4 render buffer (256 KiB) and opens one document at a time. It has no unbounded queue or long-lived cache of our own.

The LibreOffice process/library may allocate substantially more internally; worker memory qualification comes later and must be measured before production limits are chosen.

## Tests and evidence

The CI job itself is the integration test. The DOCX fixture is generated using only Python's standard library with pinned ZIP timestamps and deterministic entry order, avoiding opaque binary fixtures.

The Rust architecture guard remains a separate gate and confirms this spike did not introduce forbidden product-layer dependencies.

## What remains after this spike

Even after this job is green, R0A still needs:

1. process transport/worker supervisor;
2. a narrow production LibreOffice adapter seam;
3. edit/transaction experiment;
4. semantic snapshot and identity experiment;
5. callback/invalidation experiment;
6. crash/kill/restart recovery test;
7. rendering payload measurements to decide copy vs shared memory;
8. compatibility fixture runner;
9. explicit policy for the unavoidable unsafe/native adapter boundary.

Only after those measurements should we write the ADR that freezes the production LibreOffice adapter/FFI shape.
