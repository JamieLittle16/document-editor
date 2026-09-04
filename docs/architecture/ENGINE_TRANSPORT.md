# Document Engine Transport

Status: **R0A bounded framing contract; domain message encoding and OS transport are not frozen**

## Purpose

`document-transport` is the byte-stream framing layer between a host process and a document-engine worker. It exists to make control-plane I/O bounded, correlated and failure-explicit without teaching the transport anything about Writer, LibreOffice or document operations.

The transport is deliberately narrower than `document-protocol`:

```text
document operation / protocol value
              |
              v
       domain message bytes
              |
              v
      bounded control frame
              |
              v
        local byte stream
```

R0A freezes only the framing behaviour required to test process isolation. It does **not** select the final domain-message serializer, socket/pipe abstraction or shared-memory mechanism.

## Dependency boundary

`document-transport` depends only on `document-protocol`, currently for the fixed-width `RequestId` type.

It must not depend on:

- `document-engine-api`;
- any concrete engine;
- LibreOffice headers or types;
- session/application/UI crates;
- an external serialization framework.

The architecture guard enforces this edge explicitly.

## Control-frame format

All integer fields are little-endian.

| Bytes | Field | Meaning |
| ---: | --- | --- |
| 0..4 | magic | ASCII `DETR` |
| 4..6 | frame version | framing version, currently `1` |
| 6 | kind | `1=request`, `2=response` |
| 7 | flags | must currently be zero |
| 8..16 | request ID | fixed-width `u64` correlation ID |
| 16..20 | payload length | `u32` byte count |
| 20.. | payload | opaque control-plane bytes |

The framing version is separate from the document protocol version. Changing message semantics should not require changing the stream framing unless the actual frame layout/behaviour changes.

## Bounds and allocation rule

Every read/write operation receives an explicit `FrameLimits` policy.

The reader parses the fixed 20-byte header first and compares the announced payload length against `max_control_payload_bytes` **before allocating the payload buffer or reading payload bytes**.

This is a correctness and containment rule, not merely a performance optimisation. A malformed or compromised worker cannot announce an arbitrarily large frame and force an unbounded allocation.

The writer performs the same admission check before emitting any bytes.

No production payload limit is frozen in `document-transport`; the caller must supply one after workload qualification.

## Stream semantics

A clean stream close is represented only when EOF occurs before any byte of the next header has been received.

Once a frame has started, EOF is an error:

- partial header -> typed `TruncatedHeader`;
- partial payload -> typed `TruncatedPayload`.

Normal short reads and short writes are supported. The implementation loops until the header/payload is complete and retries interrupted reads through normal Rust I/O semantics.

Unknown frame versions, kinds and flags are rejected rather than guessed.

## Request correlation

Every request and response carries a `RequestId(u64)`.

The framing layer preserves the identifier but does not decide:

- how IDs are allocated;
- whether requests may be concurrent;
- cancellation semantics;
- timeouts;
- response ordering;
- event/subscription semantics.

Those behaviours belong to the supervised process/session layer and will be added only as the worker vertical slice requires them.

## Large rendering data

Rendered pixels should not be forced through this control frame merely because the framing exists.

The intended split remains:

```text
small control message
        |
        +-- metadata / commands / diagnostics -> framed inline bytes
        |
        `-- RenderRegion result -> future shared-memory/buffer descriptor
```

Shared memory is deliberately not selected yet. We first need real tile-size/frequency measurements and crash-lifetime tests.

## Executable tests

The crate includes tests for:

- request round-trip;
- response/request-ID correlation;
- short reads;
- short writes;
- clean EOF before a new frame;
- partial-header truncation;
- partial-payload truncation;
- oversized announced payload rejection before payload reading;
- oversized outgoing payload rejection;
- bad magic;
- unsupported frame version;
- unknown frame kind;
- unsupported flags.

These tests use only in-memory `Read`/`Write` implementations. OS process transport is the next layer, not hidden inside the codec.

## Next slice

The next R0A transport step is a supervised `document-worker` vertical slice that uses this framing over a real child-process stream while keeping the first domain codec explicitly provisional. That slice must prove worker startup, request/response correlation, EOF/exit detection and clean teardown before the LibreOffice adapter is placed behind it.
