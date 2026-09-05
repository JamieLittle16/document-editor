# R0A Document Worker Process Spike

Status: **Active qualification spike; process semantics are under test and the command codec is disposable**

## Purpose

We have separately proved:

- document protocol primitives can be fixed-width and bounded;
- control messages can be framed over arbitrary Rust `Read`/`Write` streams;
- LibreOfficeKit can open, render, edit and round-trip a DOCX.

The next risk is process isolation itself. Before putting LibreOfficeKit behind a Rust worker adapter, R0A must prove that the host/worker boundary behaves correctly when the worker starts, exits normally, dies unexpectedly and is started again.

This spike therefore runs the existing deterministic mock engine in a **real child `document-worker` process** using stdin/stdout as the byte stream for `document-transport` frames.

## What this spike is not

The one-byte command payloads in `workers/document-worker/src/lib.rs` are explicitly disposable. They are named `R0A_*` in code so they cannot be mistaken for protocol-v1 message definitions.

This spike does **not** select:

- the permanent domain serializer;
- the permanent process-supervisor API;
- local sockets versus anonymous/named pipes;
- request concurrency;
- cancellation/timeouts;
- shared-memory rendering transport;
- the production LibreOffice adapter.

Those decisions remain downstream of evidence.

## Process mode

The worker retains its existing no-argument diagnostic mode. The process spike is enabled only with:

```text
document-worker --r0a-stdio-spike
```

In that mode:

```text
host stdin writer
      |
      v
bounded request frame
      |
      v
document-worker child
      |
      +-- disposable command decode
      |
      +-- deterministic MockDocumentEngine
      |
      v
bounded response frame
      |
      v
host stdout reader
```

Stderr is intentionally not part of the binary protocol and remains available for process diagnostics.

## Disposable command codec

Current R0A payloads are deliberately tiny:

| Request | Meaning | Successful response |
| --- | --- | --- |
| `[1]` | read mock engine protocol capabilities | `[0, 1, major:u16-le, minor:u16-le]` |
| `[2]` | graceful shutdown | `[0, 2]`, then worker exits success |
| other | invalid spike command | `[1, command]` (or `[1, 0]` for malformed length) |

The outer `document-transport` frame carries the real fixed-width `RequestId`, so response correlation is exercised independently of this temporary payload encoding.

The spike frame limit is 1024 bytes. This is intentionally far larger than the two current requests yet small enough that the experiment cannot drift into large-payload transport by accident.

## Failure semantics under test

### Clean stdin EOF

EOF before the next frame means the host has closed its command stream. The worker exits successfully.

### Graceful shutdown command

The worker first writes the correlated shutdown response, flushes stdout, then exits successfully. Commands already buffered after the shutdown frame are deliberately not processed.

### Forced worker death

The process-level test force-kills a worker that has already answered a request, waits for a non-success exit status, and observes stdout EOF.

The key rule is:

> EOF is transport evidence; the child exit status determines whether that EOF followed a clean or abnormal process termination.

A future supervisor must therefore observe both the stream and the child process. It must never reinterpret arbitrary EOF as graceful engine completion.

### Restart

After forced death, the test starts a completely fresh worker process and requires it to answer a new capability request and then shut down cleanly. No state from the dead child is reused.

This proves **process restartability**, not yet document-session recovery. Reopening/checkpoint restoration comes later when persistence semantics are connected to the supervisor.

## Tests

In-process worker-loop tests cover:

- request ID preservation;
- graceful shutdown stops processing later buffered frames;
- a response-kind frame from the host is rejected;
- an unknown disposable command returns an explicit invalid-request response instead of crashing the worker.

Real child-process integration tests cover:

- actual executable spawn with piped stdin/stdout;
- capabilities request/response across the OS process boundary;
- preservation of a 64-bit request ID;
- graceful shutdown response and successful exit status;
- clean stdin-EOF exit;
- force-kill and non-success exit detection;
- EOF after forced death;
- fresh worker restart after forced death.

The process tests use `CARGO_BIN_EXE_document-worker`, so `cargo test --workspace --all-targets` builds and exercises the exact worker binary produced by the workspace rather than a fake subprocess script.

## Architecture boundary

For this spike, `document-worker` may depend on:

```text
document-engine-api
document-engine-mock
document-protocol
document-transport
```

The architecture guard records those edges explicitly. `document-transport` itself remains unaware of workers and engines.

The direct mock dependency is R0A scaffolding. It should disappear when the real engine adapter is selected behind the engine boundary.

## Acceptance gate

This spike is accepted only when the normal Rust CI gate is fully green:

- architecture guard;
- rustfmt;
- workspace check;
- workspace/all-target tests, including real child-process tests;
- Clippy with warnings denied.

The independent LibreOfficeKit qualification job must remain green in the same branch, proving that process-spike work did not contaminate or break the external-engine experiment.

## After this spike

Once process semantics are green, the next useful step is **not** to invent a general supervisor framework. We should place the smallest LibreOffice adapter executable path behind the proven worker process boundary and then qualify:

1. real engine startup/teardown in a child process;
2. typed init/load failures;
3. open-document lifetime and private LibreOffice profile ownership;
4. forced worker death while a document is open;
5. checkpoint/reopen recovery behaviour;
6. callback/invalidation ordering;
7. render payload sizes and transfer strategy.

Only then will there be enough evidence to freeze the production supervisor and native-adapter ADRs.
