# LibreOfficeKit Native Process Adapter Spike

Status: **Active R0A qualification artifact; not the production native adapter contract**

## Purpose

R0A now proves that a process-shaped native adapter can own LibreOfficeKit, expose a small bounded byte protocol, contain native failures, return revision-stamped live semantics from the exact Writer document already owned by that process, retire deterministically on normal supervisor paths, and restart after abnormal death.

`spikes/libreofficekit-process-adapter/` provides that evidence without introducing Rust FFI or `unsafe` code into product crates.

The adapter deliberately combines four high-risk qualifications in one disposable process seam:

1. bounded cross-language control framing;
2. LibreOffice lifecycle/crash containment;
3. same-instance Writer semantic access and bounded projection;
4. authority revision/freshness evidence across real native mutation.

## Deliberate quarantine

The adapter is C++ and lives outside the Rust workspace. LibreOffice headers, `lok::Office`, `lok::Document`, UNO references and tiled-rendering ABI details remain absent from:

- `document-protocol`;
- `document-transport`;
- `document-engine-api`;
- `document-session`;
- `app-core`;
- product-facing `document-worker` contracts.

The Rust workspace continues to enforce `unsafe_code = "forbid"`.

### Version-pinned semantic compatibility module

The internal Writer semantic dependency is now split into three layers:

```text
adapter.cxx
    LibreOfficeKit process owner + disposable DETR command protocol

writer_semantics_proxy.cxx / writer_semantics_24_2.hxx
    native-neutral semantic-view proxy; no UNO types

writer_semantics_24_2.cxx
    unloadable LibreOffice 24.2 compatibility module
    owns UNO + internal comphelper ABI dependency
```

`writer_semantics_module_abi.hxx` defines the tiny qualification-only C ABI between the proxy and compatibility module. It contains no UNO or LibreOffice types.

The compatibility module is loaded only after LibreOfficeKit has started. Its semantic view is released and the module is unloaded before the owning Writer document and LibreOfficeKit office are retired.

The exact LibreOffice 24.2 declaration of internal `comphelper::getProcessComponentContext()` remains confined to `writer_semantics_24_2.cxx`. Production adoption requires an explicit versioning/compatibility ADR; this spike is not permission to expose internal LibreOffice types or symbols to product code.

## Cross-language frame compatibility

The native adapter independently implements the R0A `document-transport` frame envelope:

```text
magic        DETR       4 bytes
frame ver    1          u16 little-endian
kind         request=1 / response=2
flags        0
request ID              u64 little-endian
payload len             u32 little-endian
payload                 bounded opaque bytes
```

The payload limit is 1024 bytes, matching the disposable Rust worker-process spike.

A Python host harness constructs and validates the same frame layout independently with `struct.Struct("<4sHBBQI")`. This is a genuine cross-language compatibility check rather than Rust reading bytes written by Rust.

## Disposable command payload

The command layer is explicitly R0A-only and must be replaced when the real domain-message representation is selected.

| Command | Request | Successful response |
| --- | --- | --- |
| engine info | `[1]` | `[0, 1] + LibreOffice version JSON` |
| open Writer document | `[2] + UTF-8 path` | `[0, 2, 1] + width:u64-le + height:u64-le` |
| close | `[3]` | `[0, 3]` |
| shutdown | `[4]` | `[0, 4]`, then successful worker retirement |
| semantic snapshot | `[5]` | `[0, 5, projection=2, revision:u64-le, count:u16-le] + bounded paragraphs` |
| qualification prefix edit | `[6] + UTF-8 text` | `[0, 6]` |

Commands 5 and 6 exist only to prove normalized live semantics and revision freshness against the real Writer authority. They are not proposed product commands.

Temporary statuses are:

- `0` OK;
- `1` invalid request;
- `2` document load failed;
- `3` incompatible document;
- `4` engine state error;
- `5` qualification limit exceeded.

These values are not a frozen protocol-v1 error model.

## Bounded semantic projection

Temporary semantic projection version 2 contains only authority revision plus ordered paragraph UTF-8 text:

```text
status:u8
command:u8
projection_version:u8 = 2
revision:u64-le
paragraph_count:u16-le
repeat paragraph_count times:
    byte_length:u16-le
    utf8_text[byte_length]
```

The entire response must fit the 1024-byte control-frame payload bound. The compatibility module produces a native-neutral bounded paragraph encoding; the adapter adds the process status, command, projection version and current document revision.

The module therefore cannot redefine the process protocol by itself, and neither side serializes UNO objects.

This remains qualification evidence. Style, language, list/table structure and stable semantic identities are intentionally absent until their invariants are measured.

## Same-instance authority and revision proof

When a Writer document opens successfully, the compatibility module obtains exactly one `XTextDocument` from the **already-running LibreOfficeKit process**. It does not bootstrap another office and does not load a second document.

The process harness requires this sequence:

1. open the deterministic three-paragraph fixture;
2. semantic snapshot returns revision `0` and the exact three paragraphs;
3. apply an unsaved prefix edit through the original `lok::Document`;
4. semantic snapshot returns revision `1`;
5. paragraph 1 contains exactly the prefix mutation while paragraphs 2-3 remain unchanged;
6. close the document;
7. semantic access is rejected with typed engine-state status;
8. restart the native process and reopen the fixture;
9. the fresh authority begins again at revision `0` and returns the original paragraphs.

This proves both same-authority observation and the feasibility of explicit semantic freshness at the real native boundary.

The native revision is qualification-local and is not globally unique across documents or process lifetimes. Product code consumes the separate `SemanticObservation<T>` / `DocumentRevision` abstraction documented by ADR-0007.

## Semantic module ABI and lifetime

The qualification-only module ABI exposes four operations:

- report ABI version;
- acquire the current Writer semantic view;
- release the semantic view;
- encode bounded paragraphs into caller-owned bytes.

The proxy validates the ABI version before acquiring a view. Missing symbols, module-load failure, ABI mismatch, semantic acquisition failure, projection-limit failure and malformed module output become typed/diagnostic adapter failures rather than undefined cross-boundary behaviour.

Raw UNO references never cross this ABI.

## Native runtime reclamation result

The internal semantic bridge exposed a pinned LibreOffice 24.2 lifecycle property that is important to distinguish precisely.

CI measured the following sequence successfully:

```text
semantic view release
compatibility-module unload
lok::Document destruction
lok::Office destruction
```

A temporary process-finalization marker then proved that LibreOffice faults **after** those owned objects have been destroyed, during later process-global static finalization (`LocaleDataWrapper` / `rtl_uString_release`). The diagnostic was removed once the ordering was established.

Therefore the R0A worker uses the already-intended process isolation boundary for the remaining global runtime state:

```text
release semantic view/module
destroy document
destroy lok::Office
std::_Exit(status)
```

`std::_Exit` is deliberately after normal native-object destruction. It is not a shortcut around document/Office cleanup.

This behaviour is documented in `NATIVE_RUNTIME_RECLAMATION_QUALIFICATION.md` and is **not** a production rule for every LibreOffice version or future native engine.

## Private profile ownership

Each adapter instance receives an explicit LibreOffice profile URL at process startup. The host harness also supplies a separate temporary `HOME`.

Qualification fails unless:

- the explicit profile gains LibreOffice-created files after engine use;
- the isolated HOME does not gain `.config/libreoffice`.

This demonstrates per-worker profile isolation without freezing the final profile-directory lifecycle.

## Real engine lifecycle exercised

### Graceful command shutdown

The harness starts the worker, exercises engine info/open/semantic observation/mutation/close/error handling, sends the shutdown command, requires the successful response to be flushed, then requires process status `0` and clean stdout EOF.

### Clean stdin EOF

A separate worker opens the real Writer fixture and acquires a semantic snapshot. The host then closes stdin without sending `SHUTDOWN`.

The worker must explicitly release its semantic/module/document/Office state, exit with status `0`, and produce clean stdout EOF. This qualifies the other normal supervisor retirement path.

### Semantic-limit rejection

A separate worker applies bounded prefix edits until the semantic snapshot would exceed the 1024-byte control-frame budget. The snapshot must be rejected with typed limit status while the worker remains healthy enough to answer engine-info and shut down successfully.

### Forced death with live Writer state

A fresh worker opens the DOCX and is force-killed while document and semantic state are live. The host requires a non-success process status and stdout EOF.

This proves the native engine remains contained by the worker process. It does not yet reconstruct the lost logical document session.

### Fresh restart

After forced death, a new worker/profile opens the same DOCX, reacquires the exact semantic snapshot at fresh revision `0`, and retires normally.

### Invalid command

A fresh worker receives an unknown disposable command, returns typed invalid-request status, remains healthy, and retires normally.

## Important failure distinction

The process contract intentionally distinguishes three things:

- protocol response;
- stdout EOF;
- child process exit status.

A shutdown response alone is not evidence of successful worker retirement. EOF alone is not evidence of a graceful engine exit. A production supervisor must observe both transport closure and process status.

The R0A reclamation rule also preserves intentional non-zero process statuses for transport/internal failures instead of allowing a later bootstrap-runtime static-finalizer crash to overwrite them.

## Why this is not Rust FFI yet

Writing production FFI now would still freeze unresolved choices about:

- generated bindings versus hand-written ABI declarations;
- callback threading and cancellation;
- production process supervision;
- engine-version compatibility loading;
- semantic identity/reconciliation;
- native crash recovery and checkpoint policy;
- rendering-transfer granularity/shared memory.

The C++ process spike gives us executable evidence before those decisions harden.

## CI acceptance

The Ubuntu 24.04 native qualification job builds and runs:

- the direct LibreOfficeKit capability/round-trip probe;
- generated UNO SDK headers from the installed LibreOffice registries;
- the version-pinned unloadable Writer semantic module;
- the native-neutral process adapter/proxy;
- the Python lifecycle/semantic harness.

The job is green only when it observes all of the following:

- cross-language `DETR` frame compatibility;
- 64-bit request correlation;
- real LibreOffice version response;
- Writer DOCX open with positive dimensions;
- same-instance semantic-view acquisition;
- exact bounded three-paragraph snapshot at revision `0`;
- unsaved LOK mutation visible in the retained semantic view at revision `1`;
- edit locality to paragraph 1;
- semantic view/module removal on document close;
- typed semantic-size limit rejection without worker death;
- typed nonexistent-file load failure;
- explicit profile ownership and no HOME fallback;
- command-based graceful shutdown with status `0` and clean EOF;
- clean-stdin-EOF retirement after a live semantic session with status `0`;
- forced death with non-success status;
- fresh worker restart/reopen/semantic reacquisition;
- typed invalid command without worker death.

## What remains

The next evidence frontier is no longer revision tagging or native acquisition. It is **semantic identity/reconciliation under structural change**:

1. paragraph split and merge first;
2. insertion/deletion around retained paragraphs;
3. move/reorder;
4. formatting-only edits;
5. save/reload identity reconciliation;
6. callback/invalidation ordering and relation to semantic revisions;
7. crash recovery from explicit checkpoints;
8. render payload size/frequency and shared-memory thresholds.

UNO object identity may be measured internally during these experiments, but raw UNO references, pointer values or temporary observation tokens must not become product `ParagraphId`s or history anchors.

Only after those measurements should the production native adapter/supervisor ADRs and durable history-anchor model be frozen.
