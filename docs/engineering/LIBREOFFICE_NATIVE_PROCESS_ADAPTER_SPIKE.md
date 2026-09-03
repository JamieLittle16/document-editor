# LibreOfficeKit Native Process Adapter Spike

Status: **Active R0A qualification artifact; not the production native adapter contract**

## Purpose

R0A has already proved two independent facts:

1. the Rust `document-worker` can be supervised as a real child process over bounded `DETR` frames, including graceful exit, abnormal death and restart;
2. LibreOfficeKit can open, lay out, render, edit, save and reopen Writer DOCX files in a quarantined standalone C++ probe.

The remaining architectural question is whether those facts compose cleanly: can a process-shaped native adapter own LibreOfficeKit and expose only a small bounded byte protocol while surviving normal teardown and making abnormal process death unambiguous to the host?

`spikes/libreofficekit-process-adapter/` answers that question without introducing Rust FFI or `unsafe` code.

## Deliberate quarantine

The adapter is C++ and lives outside the Rust workspace. LibreOffice headers, `lok::Office`, `lok::Document` and tiled-rendering ABI details therefore remain absent from:

- `document-protocol`;
- `document-transport`;
- `document-engine-api`;
- `document-session`;
- `app-core`;
- `document-worker` product-facing contracts.

The Rust workspace continues to enforce `unsafe_code = "forbid"`.

This spike provides evidence for a later native-boundary ADR. It is not permission to copy LibreOffice structs or functions into the product protocol.

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

Its payload limit is 1024 bytes, matching the disposable Rust worker-process spike.

A Python host harness constructs and validates the same frame layout using `struct.Struct("<4sHBBQI")`. This gives us a useful independent compatibility check: the framing contract is no longer only Rust reading bytes written by Rust.

## Disposable command payload

The command layer is explicitly R0A-only and must be replaced when the real domain-message representation is selected.

| Command | Request | Successful response |
| --- | --- | --- |
| engine info | `[1]` | `[0, 1] + LibreOffice version JSON` |
| open Writer document | `[2] + UTF-8 path` | `[0, 2, 1] + width:u64-le + height:u64-le` |
| close | `[3]` | `[0, 3]` |
| shutdown | `[4]` | `[0, 4]`, then successful process exit |

Errors begin with a typed status byte and echoed command byte, followed by bounded diagnostic text where useful.

Current temporary statuses are:

- `0` OK;
- `1` invalid request;
- `2` document load failed;
- `3` incompatible document;
- `4` engine state error.

These values are **not protocol-v1 error codes**.

## Private profile ownership

Each adapter instance receives an explicit LibreOffice profile URL at process startup. The host harness also gives the process a separate temporary `HOME`.

The qualification fails unless:

- the explicit profile gains LibreOffice-created files after engine use;
- the isolated HOME does not gain `.config/libreoffice`.

This does not yet define the production profile-directory lifecycle, but it demonstrates that per-worker profile isolation is technically viable and observable.

## Real engine lifecycle exercised

The harness performs four process scenarios.

### Graceful process

1. start native adapter;
2. request LibreOffice version information using a 64-bit request ID;
3. open the deterministic DOCX fixture;
4. require positive Writer layout dimensions;
5. close the document;
6. request a nonexistent DOCX and require a typed load-failure response rather than process death;
7. request graceful shutdown;
8. require successful child exit and clean response-stream EOF.

### Forced death with an open document

1. start a fresh adapter/profile;
2. open the DOCX successfully;
3. force-kill the process while the document is live;
4. require a non-success process status;
5. observe stdout EOF.

This proves that LibreOfficeKit state is contained by the adapter process. It does **not** yet restore the lost document session.

### Fresh restart

After the forced death, the harness starts a new adapter with a new private profile, opens the same DOCX successfully and shuts down normally.

This proves real-engine process restartability after a crash.

### Invalid command

A fresh adapter receives an unknown disposable command and must return a typed invalid-request response while remaining healthy enough to shut down normally.

## Important failure distinction

The same rule established by the mock worker process now holds with LibreOfficeKit actually loaded:

> stream EOF is transport evidence; the child process status determines whether the engine process ended normally or abnormally.

A production supervisor must observe both.

## Why this is not Rust FFI yet

Writing FFI now would force choices about:

- generated bindings versus hand-written ABI declarations;
- ownership wrappers;
- callback threading;
- panic/error crossing rules;
- native library discovery/loading;
- native crash containment;
- adapter API granularity.

We still have unresolved callback, semantic-identity and rendering-transfer measurements. Keeping this process adapter in C++ gives us the lifecycle evidence without prematurely freezing those choices.

## CI acceptance

The existing Ubuntu 24.04 LibreOffice qualification job builds both:

- the original direct capability probe;
- the native process adapter.

The job then runs the Python process harness against stock Ubuntu LibreOffice packages and the same deterministic DOCX fixture.

The process-adapter step fails unless all of the following are observed:

- cross-language `DETR` frame compatibility;
- 64-bit request correlation;
- real LibreOffice version response;
- Writer DOCX open with positive dimensions;
- typed nonexistent-file load failure;
- explicit-profile use;
- no fallback to the isolated HOME LibreOffice profile;
- graceful shutdown and successful exit;
- forced death while a document is open and non-success exit;
- clean EOF after forced death;
- fresh real-engine restart and reopen;
- typed invalid command without worker death.

## What remains

The next evidence should focus on document semantics rather than another transport abstraction:

1. minimal semantic snapshot extraction;
2. identity stability across edit/save/reopen;
3. callback and invalidation ordering/thread affinity;
4. crash recovery from an explicit checkpoint/reopen policy;
5. render payload size/frequency measurements;
6. shared-memory versus copy threshold qualification.

Only after those experiments should we freeze the production native adapter and supervisor ADRs.
