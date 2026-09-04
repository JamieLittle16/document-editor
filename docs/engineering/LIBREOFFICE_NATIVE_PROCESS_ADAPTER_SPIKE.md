# LibreOfficeKit Native Process Adapter Spike

Status: **Active R0A qualification artifact; not the production native adapter contract**

## Purpose

R0A has now proved that a process-shaped native adapter can own LibreOfficeKit, expose a small bounded byte protocol, survive normal teardown, make abnormal process death unambiguous to the host, and return a live semantic projection from the exact Writer document already owned by that LibreOfficeKit process.

`spikes/libreofficekit-process-adapter/` provides that evidence without introducing Rust FFI or `unsafe` code.

The adapter deliberately combines three high-risk qualifications in one disposable process seam:

1. bounded cross-language control framing;
2. LibreOffice lifecycle/crash containment;
3. same-instance Writer semantic access and bounded semantic projection.

## Deliberate quarantine

The adapter is C++ and lives outside the Rust workspace. LibreOffice headers, `lok::Office`, `lok::Document`, UNO references and tiled-rendering ABI details therefore remain absent from:

- `document-protocol`;
- `document-transport`;
- `document-engine-api`;
- `document-session`;
- `app-core`;
- `document-worker` product-facing contracts.

The Rust workspace continues to enforce `unsafe_code = "forbid"`.

The version-specific semantic bridge is additionally isolated behind:

```text
writer_semantics_24_2.hxx   native-neutral C++ PIMPL surface
writer_semantics_24_2.cxx   UNO + LibreOffice 24.2 internal ABI dependency
```

`writer_semantics_24_2.hxx` exposes only ordinary C++ strings/vectors and an opaque `WriterSemanticView`. The exact LibreOffice 24.2 declaration of internal `comphelper::getProcessComponentContext()` exists only in the version-labelled `.cxx` file.

This spike provides evidence for a later native-boundary ADR. It is not permission to copy LibreOffice structs, UNO references or internal symbols into the product protocol.

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

A Python host harness constructs and validates the same frame layout using `struct.Struct("<4sHBBQI")`. This gives us an independent compatibility check: the framing contract is not only Rust reading bytes written by Rust.

## Disposable command payload

The command layer is explicitly R0A-only and must be replaced when the real domain-message representation is selected.

| Command | Request | Successful response |
| --- | --- | --- |
| engine info | `[1]` | `[0, 1] + LibreOffice version JSON` |
| open Writer document | `[2] + UTF-8 path` | `[0, 2, 1] + width:u64-le + height:u64-le` |
| close | `[3]` | `[0, 3]` |
| shutdown | `[4]` | `[0, 4]`, then successful process exit |
| semantic snapshot | `[5]` | `[0, 5, projection=1, count:u16-le] + bounded length-prefixed UTF-8 paragraphs` |
| qualification prefix edit | `[6] + UTF-8 text` | `[0, 6]` |

Commands 5 and 6 exist to prove that normalized live semantics cross the actual process boundary and that the retained semantic view observes an **unsaved** edit made through LibreOfficeKit. They are not proposed product commands.

Errors begin with a typed status byte and echoed command byte, followed by bounded diagnostic text where useful.

Current temporary statuses are:

- `0` OK;
- `1` invalid request;
- `2` document load failed;
- `3` incompatible document;
- `4` engine state error;
- `5` qualification limit exceeded.

These values are **not protocol-v1 error codes**.

## Bounded semantic projection

The R0A semantic response is intentionally smaller than the UNO document model. Projection version 1 contains only ordered paragraph UTF-8 text:

```text
status:u8
command:u8
projection_version:u8
paragraph_count:u16-le
repeat paragraph_count times:
    byte_length:u16-le
    utf8_text[byte_length]
```

The entire response must fit the existing 1024-byte frame payload bound. The adapter checks the complete encoded size before returning the snapshot and emits a typed limit error rather than allowing the frame writer to fail after an oversized semantic allocation has become an implicit contract.

This is qualification evidence, not the permanent semantic snapshot schema. Style, language, list/table structure, IDs, anchors and revisions remain deliberately absent until their invariants are measured.

## Same-instance authority proof

When a Writer document opens successfully, the adapter acquires exactly one `XTextDocument` from the **already-running LibreOfficeKit process**. No separate UNO bootstrap is performed and no second document is loaded.

The resulting `WriterSemanticView` is retained for the lifetime of the open LOK document. The process harness then:

1. requests a semantic snapshot and requires the three known fixture paragraphs;
2. sends the qualification prefix edit through the original `lok::Document`;
3. does **not** save or reopen;
4. requests another snapshot through the same retained semantic view;
5. requires only paragraph 1 to contain the prefix while paragraphs 2-3 remain unchanged;
6. closes the document and requires semantic snapshot access to fail with engine-state status.

That sequence proves the semantic view observes the same authoritative live document rather than a separately loaded copy.

The implementation currently depends on LibreOffice's internal `comphelper::getProcessComponentContext()` ABI. The exact 24.2 signature returns the component-context reference by value and differs from newer LibreOffice source. The dependency is therefore version-pinned qualification machinery. Production adoption requires an explicit compatibility/versioning design and ADR.

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
5. require the bounded live semantic snapshot to match all three fixture paragraphs;
6. apply an unsaved LOK prefix edit and require the retained semantic view to observe exactly that change;
7. close the document and require semantic access to disappear with it;
8. request a nonexistent DOCX and require a typed load-failure response rather than process death;
9. request graceful shutdown;
10. require successful child exit and clean response-stream EOF.

### Forced death with an open document

1. start a fresh adapter/profile;
2. open the DOCX successfully;
3. force-kill the process while the document and semantic view are live;
4. require a non-success process status;
5. observe stdout EOF.

This proves that LibreOfficeKit and UNO semantic state are contained by the adapter process. It does **not** yet restore the lost logical document session.

### Fresh restart

After forced death, the harness starts a new adapter with a new private profile, opens the same DOCX successfully, requires the original three-paragraph semantic snapshot, and shuts down normally.

This proves real-engine process restartability plus semantic re-acquisition after a crash. It is not yet identity reconciliation across an unsaved crash.

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
- adapter API granularity;
- versioning internal LibreOffice semantic access.

We still have unresolved callback, semantic-identity and rendering-transfer measurements. Keeping this process adapter in C++ gives us the evidence without prematurely freezing those choices.

## CI acceptance

The Ubuntu 24.04 LibreOffice qualification job builds:

- the original direct LibreOfficeKit capability/round-trip probe;
- generated UNO SDK headers from the installed pinned LibreOffice registries;
- the native process adapter plus `writer_semantics_24_2.cxx`.

The job then runs the Python process harness against stock Ubuntu LibreOffice packages and the same deterministic DOCX fixture.

The process-adapter step fails unless all of the following are observed:

- cross-language `DETR` frame compatibility;
- 64-bit request correlation;
- real LibreOffice version response;
- Writer DOCX open with positive dimensions;
- same-instance Writer semantic view acquisition;
- exact three-paragraph bounded live semantic snapshot;
- same retained semantic view observing an unsaved LOK edit;
- edit locality to paragraph 1;
- semantic view removal on document close;
- typed nonexistent-file load failure;
- explicit-profile use;
- no fallback to the isolated HOME LibreOffice profile;
- graceful shutdown and successful exit;
- forced death while a document is open and non-success exit;
- clean EOF after forced death;
- fresh real-engine restart, reopen and semantic re-acquisition;
- typed invalid command without worker death.

## What remains

The next evidence should focus on semantic identity/reconciliation rather than adding another acquisition or transport abstraction:

1. candidate paragraph/object identity signals across insertion, deletion, split, merge, move and formatting-only edits;
2. explicit document/revision tagging for semantic snapshots;
3. identity/reconciliation across save/reload;
4. callback and invalidation ordering/thread affinity;
5. crash recovery from an explicit checkpoint/reopen policy;
6. render payload size/frequency measurements;
7. shared-memory versus copy threshold qualification.

Only after those experiments should we freeze the production native adapter and supervisor ADRs.
