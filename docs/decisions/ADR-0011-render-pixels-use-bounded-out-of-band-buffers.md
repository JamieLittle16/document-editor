# ADR-0011: Render pixels use bounded reusable out-of-band buffers

Status: accepted for R0A

Date: 2026-09-06

## Context

Office deliberately separates a small, bounded document-engine control plane from large rendering payloads. Before this ADR, `ENGINE_TRANSPORT.md` already stated that rendered pixels should not be forced through the `DETR` frame merely because a control frame exists, but the actual transfer architecture remained open pending real workload measurements.

R0A now has repeated measurements from the pinned LibreOffice 24.2.7.2 Writer engine using caller-owned raster buffers. The qualification workload renders a logical 1024×768 viewport as a 4×3 grid of 256×256 logical tiles at 1× and 2× backing density.

The stable measured byte geometry is:

```text
1× tile        256 × 256 × 4 B =      262,144 B   (256 KiB)
2× tile        512 × 512 × 4 B =    1,048,576 B   (1 MiB)
1× viewport  1024 × 768 × 4 B =    3,145,728 B   (3 MiB)
2× viewport  2048 × 1536 × 4 B =  12,582,912 B   (12 MiB)
```

The same fixture's current page raster is approximately 3.86 MiB at 1× and 15.46 MiB at 2×. These structural values reproduced unchanged across independent native runs. Paint timing moved between runners, as expected, and remains diagnostic only.

The current disposable native control path caps payloads at roughly 1 KiB. Relative to that control envelope, one 1× tile is 256× larger, one 2× tile is 1024× larger, a 1× viewport is 3072× larger and a 2× viewport is 12288× larger. Even if the final control limit changes, the architectural scale separation is clear: raster bytes are bulk local data, not control messages.

R0A invalidation qualification also established that native render invalidations are advisory and can race around a semantic mutation boundary. Render transfer must therefore remain subordinate to product-owned authority rather than allowing a completed buffer write to become semantic authority by itself.

## Decision

Office uses **small revision-tagged control messages plus bounded reusable out-of-band render buffers** for raster results produced by an isolated document worker.

The durable architecture selects the control/data-plane split and buffer lifecycle. It does **not** freeze a particular operating-system shared-memory primitive.

### Control plane

The ordinary framed channel carries only bounded metadata such as:

- request and response correlation;
- document/session authority scope;
- document revision;
- render region and scale;
- buffer/lease identifier;
- buffer offset and capacity;
- width, height, stride and pixel format;
- completion/error state.

Raw pixel bytes do not travel inline through ordinary control frames.

### Buffer ownership and bounds

The supervisor/host owns the render-buffer pool and its memory budget. This gives the product process authoritative control over allocation, capacity, reuse and reclamation even when a worker crashes.

The pool is bounded by at least:

- total mapped bytes;
- slot count;
- maximum slot size;
- maximum outstanding leases per worker/document.

The exact production defaults are workload policy and are not frozen by this ADR.

Reusable slots are preferred to creating a new OS mapping/object for every tile. A 256-pixel tile is a qualification workload, not a permanent tile-size contract.

### Lease lifecycle

A render slot follows an explicit ownership protocol equivalent to:

```text
available
   -> leased-to-worker
   -> ready-for-host
   -> retained/presented
   -> recyclable
```

A worker may write only within a valid lease and declared capacity. The host/UI must not read a slot while the worker owns the write lease.

A successful completion message is the publication boundary that makes the written bytes eligible for host consumption. A worker death, protocol loss or authority replacement invalidates every unfinished lease associated with that worker/authority. Partially written memory is never promoted to a valid render result merely because bytes exist in the mapping.

### Authority and stale-result rejection

Every render request/result is subordinate to product-owned authority. The host validates enough information to reject stale or cross-session data, including:

- request correlation;
- buffer identifier and lease generation;
- `AuthorityGeneration` or equivalent product-owned authority scope;
- `DocumentRevision`;
- render region/scale identity;
- width/height/stride/offset/length/pixel-format consistency;
- integer-overflow and capacity checks.

An engine callback can mark regions dirty, but cannot advance revision or publish a buffer as current. ADR-0009 remains normative: invalidations are advisory render dirtiness, and R0B must place an explicit mutation/event fence between native callback ingestion and rendering under the committed application authority.

### Mapping backend remains replaceable

The production implementation may use platform-appropriate local mechanisms, for example:

- Linux anonymous/memfd-style shared mappings with descriptor passing;
- Windows file-mapping/section handles with safe handle duplication;
- equivalent macOS shared-memory or mapped-file primitives.

These mechanisms are implementation backends behind the same bounded lease model. Raw pointers are never serialized as protocol identity.

### Raster is a bootstrap representation, not the permanent engine model

Writer/LibreOfficeKit supplies raster tiles today. A future native engine may return display lists, GPU resources or another render representation. The viewport/application contract should preserve the same authority, bounded-lifetime and stale-result rules without assuming raster tiles forever.

## Consequences

### Positive

- multi-megabyte raster payloads no longer compete with small control/semantic messages;
- repeated copies through pipe/socket framing are avoided by architecture rather than micro-optimisation;
- host-owned budgets contain render memory even if a worker misbehaves or dies;
- crash recovery can invalidate outstanding leases without trusting worker cleanup;
- revision/authority validation remains explicit at the publication boundary;
- OS-specific IPC/mapping technology stays replaceable;
- reusable buffers make future batching and viewport scheduling possible without one allocation/mapping per tile.

### Costs

- the worker protocol needs a buffer-lease lifecycle in addition to request/response control framing;
- cross-platform mapping/handle transfer requires backend-specific code and tests;
- buffer reuse requires explicit synchronization and stale-lease protection;
- memory sizing, slot geometry and scheduling still need workload tuning;
- GPU-native future paths may require another data-plane backend while preserving the same authority semantics.

## Invariants

1. Ordinary engine control frames do not carry bulk raster pixel bodies.
2. Render-buffer allocation and total memory are bounded by host-owned policy.
3. The worker never owns durable buffer identity; it receives scoped leases.
4. The host does not consume a slot until a valid completion publishes it.
5. Worker death/authority replacement invalidates unfinished leases from that authority.
6. Render results are tagged/validated against product-owned authority and document revision.
7. Buffer descriptors contain explicit geometry/format/capacity information; no raw pointers cross the process boundary.
8. Qualification tile size and CI paint timings are not production contracts.
9. Native invalidation callbacks remain advisory and cannot publish semantic or render authority.
10. Platform mapping technology remains behind a replaceable backend.

## Required R0B implementation evidence

Before the real viewport treats this path as production-ready, tests must prove:

- bounded pool admission and exhaustion behavior;
- lease generation prevents stale completion after slot reuse;
- invalid offset/stride/length/geometry cannot escape a slot;
- worker death invalidates all outstanding leases and allows safe pool reclamation;
- stale completions from an old `AuthorityGeneration` are rejected;
- revision mismatch cannot populate authoritative hit-test/layout state;
- no host read occurs while the worker owns a write lease;
- visible, prefetch and background scheduling can share the pool without unbounded growth.

## Non-decisions

This ADR does not freeze:

- a 256×256 production tile size;
- the final buffer pool byte/slot limits;
- a specific Linux/Windows/macOS mapping API;
- compression for local raster transfer;
- a remote-rendering transport;
- a final GPU/display-list representation;
- paint-time performance thresholds.
