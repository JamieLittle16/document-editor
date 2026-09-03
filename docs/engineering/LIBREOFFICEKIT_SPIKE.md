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
7. Can a primitive text mutation be applied through LibreOfficeKit and persist into the saved DOCX?
8. Can we save to DOCX and reopen the resulting package?
9. What actual LibreOffice version/build did the qualification environment execute?

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
 +-- paste unique UTF-8 text marker at current Writer cursor
 |
 +-- saveAs(roundtrip.docx)
 |
 +-- destroy original document
 |
 +-- reopen roundtrip.docx
 `-- validate package structure + persisted text marker
```

The edit marker is deliberately simple ASCII and the validator reconstructs text from OOXML `w:t` nodes rather than relying on raw XML byte layout. This proves that the mutation persisted semantically into the saved DOCX without pretending this primitive paste operation is already our product transaction model.

The render hash printed by CI is diagnostic evidence, **not** a golden image checksum. Font/package changes can legitimately change raster output while the boundary remains correct.

## Current external-API observations

The upstream C++ wrapper constructs an `Office` through `lok_cpp_init()` and loads documents through `Office::documentLoad()`. The wrapper owns/destroys the underlying C handles.

Current upstream tiled rendering (`initializeForRendering`, `getDocumentSize`, `paintTile`, `getTileMode`) is exposed under `LOK_USE_UNSTABLE_API`. This is a strong reason not to treat the LibreOfficeKit rendering ABI as our product protocol.

LibreOffice's own tiled/desktop test code and GTK LibreOfficeKit viewer use `text/plain;charset=utf-8` for plain-text paste. R0A uses that same existing boundary for a mutation smoke test only.

The loader accepts an explicit installation path and user-profile URL. Worker startup must eventually create a private profile location rather than sharing the user's normal LibreOffice profile.

Ubuntu 24.04 currently supplies LibreOfficeKit 24.2 in CI. Its C++ wrapper does not expose the newer `freeMemory()` convenience helper, so the probe intentionally uses the older compatible deallocator surface. This reinforces the requirement that our production adapter target a deliberately qualified compatibility baseline rather than current upstream headers alone.

## Acceptance criteria

The CI job fails unless all of the following are true:

- LibreOfficeKit initialises;
- version information is returned;
- input DOCX loads;
- document type is text;
- width and height are positive;
- tile mode is a recognised 4-byte RGBA/BGRA mode;
- `paintTile` modifies the caller-owned buffer;
- UTF-8 text paste succeeds;
- DOCX `saveAs` succeeds;
- round-trip DOCX reopens as text;
- reopened layout size is positive;
- round-trip file is a valid ZIP package containing `word/document.xml`;
- reconstructed OOXML text contains the unique edit marker.

## Failure semantics

This is a qualification job. Any probe failure is a red CI result and blocks us from claiming the boundary works on the reference environment.

It does not yet define runtime recovery. The production engine worker will later convert init/load/render/edit failures into typed engine/transport diagnostics and be restartable independently of the desktop shell.

## Resource bounds

The current probe allocates exactly one 256 x 256 x 4 render buffer (256 KiB) and opens one document at a time. It has no unbounded queue or long-lived cache of our own.

The LibreOffice process/library may allocate substantially more internally; worker memory qualification comes later and must be measured before production limits are chosen.

## Tests and evidence

The CI job itself is the integration test. The DOCX fixture is generated using only Python's standard library with pinned ZIP timestamps and deterministic entry order, avoiding opaque binary fixtures.

The round-trip validator uses Python's standard-library ZIP/XML parsers. It checks both OPC package integrity and semantic persistence of the edit marker.

The Rust architecture guard remains a separate gate and confirms this spike did not introduce forbidden product-layer dependencies.

### First green reference run

The first open/render/save/reopen qualification passed on Ubuntu 24.04 with:

```text
LibreOffice ProductVersion: 24.2
ProductExtension: .7.2
BuildId: 420(Build:2)
document type: text
layout size: 12474 x 17406 TWIPs
tile mode: BGRA
256x256 render FNV-1a: 0x299c15792be4f780
round-trip reopen: OK
round-trip DOCX bytes: 4983
```

These values record the environment that proved the boundary; only structural/semantic assertions become hard compatibility contracts unless a later visual fixture explicitly defines a golden result.

## What remains after this spike

After the persisted-edit test is green, R0A still needs:

1. process transport/worker supervisor;
2. a narrow production LibreOffice adapter seam;
3. semantic snapshot and identity experiment;
4. callback/invalidation experiment;
5. crash/kill/restart recovery test;
6. rendering payload measurements to decide copy vs shared memory;
7. compatibility fixture runner;
8. explicit policy for the unavoidable unsafe/native adapter boundary;
9. transaction-to-engine mutation mapping beyond the primitive paste smoke test.

Only after those measurements should we write the ADR that freezes the production LibreOffice adapter/FFI shape.
