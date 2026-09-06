# Rendering and Viewport

## Principle

The UI owns the viewport. The document engine supplies renderable content and geometry asynchronously.

Render artifacts are subordinate to product-owned document authority. Engine callbacks, worker memory and buffer completion never become semantic authority merely by existing.

## Bootstrap pipeline

```text
viewport
   -> required regions
   -> priority scheduler
   -> authority/revision-tagged render request
   -> bounded host-owned buffer lease
   -> worker paints out of band
   -> completion publishes candidate result
   -> host revalidates lease + authority + revision
   -> revision-scoped cache / viewport
```

Priority classes:

1. currently visible;
2. immediate scroll neighbourhood;
3. predictive prefetch;
4. thumbnails/background.

## Raster data plane

R0A real-Writer qualification demonstrates that raster results are bulk data:

```text
1× 256px backing tile:    256 KiB
2× 256px backing tile:      1 MiB
1× 1024×768 viewport:        3 MiB
2× 1024×768 viewport:       12 MiB
```

Raw pixel bodies therefore do not travel through ordinary engine control frames. ADR-0011 selects bounded reusable out-of-band buffers with small control-plane descriptors.

The host/supervisor owns the buffer pool, byte budget and slot lifecycle. Workers receive scoped write leases. A slot is not consumable until a valid completion publishes it, and worker death/authority replacement invalidates unfinished leases.

A render descriptor/result must carry or be validated against enough information to establish:

- request correlation;
- buffer ID and lease generation;
- product `AuthorityGeneration`;
- `DocumentRevision`;
- logical render region and scale;
- width, height and stride;
- byte offset/length/capacity;
- explicit pixel format.

Arithmetic overflow, capacity violations and stale lease/authority/revision values are hard rejections.

The 256px qualification tile is not a production tile-size decision. Pool size, tile geometry and prefetch policy remain benchmark-driven.

## Invalidation and mutation fencing

LibreOfficeKit invalidation callbacks are advisory dirty-region signals. R0A qualification shows they can arrive off the owner thread and on either side of the native mutation-return boundary while Office still considers the document to be the old revision.

Therefore callback ingestion may enqueue/coalesce bounded dirtiness, but it must not:

- advance semantic revision;
- publish a render slot;
- directly mutate UI/session/history authority;
- cause new engine state to be consumed under an old revision.

R0B must add an explicit mutation/event fence so invalidation-driven render requests are emitted only beneath the committed application authority.

## UI-owned overlays

Where correctness permits, the shell should own lightweight responsive overlays such as:

- caret;
- selection affordances;
- spelling/grammar underlines;
- search highlights;
- comment/review indicators;
- remote cursors in future collaboration.

They must be derived from geometry valid for the rendered authority/revision.

## Cache rules

- cache memory is bounded;
- keys include enough authority/document/layout revision information to prevent stale reuse;
- a completed worker buffer is only a candidate until authority/revision/lease validation succeeds;
- stale raster tiles can remain briefly as visual fallback during scrolling but are never treated as authoritative hit-test geometry;
- worker death invalidates unfinished leases and any result whose authority scope no longer matches the session;
- cache policy is benchmarked on large documents and high-DPI workloads.

## Native future

A future engine may provide display lists, GPU-oriented primitives or other render resources rather than raster tiles. The viewport contract should preserve the same bounded lifetime, authority/revision validation and stale-result rejection without requiring the future engine to emulate Writer's raster representation.
