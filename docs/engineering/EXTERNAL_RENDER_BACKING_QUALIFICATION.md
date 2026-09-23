# External Render Backing Qualification

Status: **R0B qualification backend; not the production mapping backend**

Date: 2026-09-23

## Purpose

ADR-0011 requires bulk raster bytes to travel out of band from the document worker while the host retains bounded allocation, lease and publication authority. The in-memory backing proves host-side preparation semantics, but it does not prove that a second process can write a validated raster region without turning a pathname, pointer or engine object into product identity.

This qualification adds a portable file-backed process test before platform-specific shared mappings are introduced.

## Architecture under test

```text
host RenderBufferPool
        |
        | acquire(scope, requested bytes)
        v
RenderWriteLease(buffer id, generation, capacity)
        |
        | fixed-width geometry validation
        v
RenderRasterDescriptor
        |
        | prepare generation-specific external backing
        v
worker capability(path, offset, byte length)
        |
        | child process writes only validated region
        v
host completion
        |
        | lease generation + published-prefix validation
        v
ReadyRenderBuffer -> host raster read
```

The qualification crate depends only on `render-buffer-pool`. It has no dependency on document sessions, engines, transport, app-core or presentation code.

## Why generation-specific files

A stale completion message is not the only failure mode. A stale worker may still possess write access to storage after its logical lease has been invalidated.

For the portable qualification backend, each `RenderLeaseGeneration` receives a distinct external file object:

```text
buffer-0-generation-1.rgba
buffer-0-generation-2.rgba
```

The old process can continue writing generation 1 after generation 2 exists, but those bytes cannot mutate generation 2. The host still rejects generation-1 completion against the live generation-2 lease.

This is a **safety requirement**, not a mandate that production create a new file/mapping every lease. A reusable shared-memory backend may instead defer reuse until worker death, revoke/duplicate handles safely, use generation-separated mappings, or provide another mechanism with the same stale-writer isolation.

## Qualified cases

### Normal second-process publication

The host:

1. acquires a bounded write lease;
2. prepares external storage at the slot capacity and clears the requested prefix;
3. validates a fixed-width raster descriptor;
4. grants the child process only the backing capability plus validated offset/length;
5. waits for successful child completion;
6. publishes the lease;
7. reads the raster only through `ReadyRenderBuffer` + descriptor validation.

The test asserts that bytes before the raster range remain zero and the worker changes only the validated region.

### Stale worker across slot reuse

The stronger test deliberately keeps the old child alive:

1. generation 1 writes its external backing and waits;
2. the host invalidates the old scope;
3. the same logical slot is leased as generation 2;
4. generation 2 receives a distinct external backing/capability;
5. the host lets the stale generation-1 child write again;
6. generation-2 backing must remain unchanged;
7. the old descriptor fails generation binding against generation 2;
8. the old completion is rejected as a stale lease.

This proves that generation safety protects both **publication authority and backing storage selection** in the qualification path.

## Deliberate non-decisions

This backend is not a performance result and does not select:

- files as the production raster transport;
- filesystem path passing as the production capability protocol;
- one storage object per production lease;
- a Linux `memfd`/mmap API;
- Windows section/file-mapping handle semantics;
- a macOS mapping primitive;
- final tile or pool sizes.

The next backing layer should replace the file mechanism while preserving the same pool, descriptor, stale-writer and publication invariants.

## Production acceptance consequence

A platform mapping backend is not ready merely because stale completion messages are rejected. Before a slot's storage can be reused for a newer generation, the host must know that an older worker can no longer mutate the bytes selected for that newer generation.

That condition belongs to the supervised worker/backing lifecycle and must be covered by failure-injection tests.
