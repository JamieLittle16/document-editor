# Document Engine Transport

Status: **R0A bounded control framing + qualified out-of-band render data-plane architecture; domain serializer and OS transport backend are not frozen**

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

R0A freezes the framing behaviour required to test process isolation and the architectural separation between small control messages and large render payloads. It does **not** select the final domain-message serializer, socket/pipe abstraction or platform-specific shared-memory mapping API.

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

No production control payload limit is frozen in `document-transport`; the caller supplies policy appropriate to the negotiated worker protocol.

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

Those behaviours belong to the supervised process/session layer.

## Large rendering data

R0A render-transfer qualification closes the control/data-plane question at the architectural level.

Real Writer measurements on the pinned environment produced:

```text
1× 256px tile:       262,144 bytes
2× 256px tile:     1,048,576 bytes
1× 1024×768 view:  3,145,728 bytes
2× 1024×768 view: 12,582,912 bytes
```

These values are orders of magnitude larger than ordinary control messages and reproduced independently. Raw render bytes therefore **must not** be serialized inline through ordinary `DETR` frames.

The selected split is:

```text
small bounded control message
        |
        +-- commands / semantic data / diagnostics
        |
        +-- render request + authority/revision + buffer lease descriptor
        |
        `-- render completion/error + lease descriptor

host-owned bounded reusable render-buffer pool
        |
        `-- bulk pixels written out of band by the worker
```

The supervisor/host owns buffer allocation, capacity and lifetime. A worker receives a scoped write lease and publishes a result only through a valid completion. Worker death or authority replacement invalidates unfinished leases.

Descriptors must be validated for buffer/lease identity, authority generation, document revision, offset, capacity, width, height, stride, byte length and pixel format. Raw pointers are never protocol values.

The production mapping backend remains replaceable. Linux, Windows and macOS may use different shared-memory/handle-transfer mechanisms behind the same bounded lease contract. Reusable slots are preferred over creating a new mapping for every tile.

See ADR-0011 and `docs/engineering/RENDER_TRANSFER_QUALIFICATION.md`.

## Executable tests

The framing crate includes tests for:

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

Native CI additionally qualifies real Writer render geometry and buffer population without making hosted-runner timings a performance gate.

## Next slice

R0B should implement the selected render-buffer lease model behind the supervised worker boundary:

- bounded host-owned pool;
- platform mapping backend;
- lease generations and stale-completion rejection;
- worker-death reclamation;
- authority/revision validation;
- mutation/event fencing before invalidation-driven rendering.

The first implementation remains provisional enough to tune pool and tile sizing from viewport workloads without reopening the control/data-plane architecture.
