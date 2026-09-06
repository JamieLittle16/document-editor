# R0A Writer render-transfer qualification

Status: **qualified for the pinned LibreOffice 24.2.7.2 environment**

Date: 2026-09-06

## Purpose

This qualification measures the raster data volume and real `paintTile()` behavior of the bootstrap Writer engine before Office freezes a worker-to-host render-transfer architecture.

The experiment is deliberately split into two evidence classes:

1. **structural evidence** — pixel format, geometry, byte volume and successful caller-owned rendering;
2. **performance observations** — real paint timing on disposable GitHub-hosted runners.

Only the first class is pinned as a CI contract. Runner timing is retained as diagnostic evidence and is not a pass/fail threshold.

## Workload

`spikes/libreofficekit-probe/render_transfer_probe.cxx` opens the same deterministic Writer fixture used by the normalized compatibility harness and paints a logical 1024×768 viewport.

The viewport is divided into a 4×3 grid of 256×256 logical tiles. The same logical tile regions are painted at:

- 1× density into 256×256 backing buffers;
- 2× density into 512×512 backing buffers.

Each pixel uses LibreOfficeKit's four-byte RGBA/BGRA tile format. Five complete 12-tile passes are timed after document load. A sampled checksum proves the caller-owned buffers were populated. The probe also reports the current page dimensions and corresponding raw page-raster sizes.

`render_transfer_contract.py` then validates the stable workload geometry and successful rendering while refusing to encode a timing SLA.

## Stable structural result

Both independent unchanged-code executions reproduced:

```text
document:               12474 × 17406 twips
pixel mode:              BGRA
bytes per pixel:         4
logical viewport:        1024 × 768 px
logical viewport:        15360 × 11520 twips

1× tile backing:         256 × 256 px
1× bytes per tile:       262,144 B
1× 12-tile pass:         3,145,728 B
1× visible viewport:     3,145,728 B

2× tile backing:         512 × 512 px
2× bytes per tile:       1,048,576 B
2× 12-tile pass:         12,582,912 B
2× visible viewport:     12,582,912 B

1× current page raster:  832 × 1161 px = 3,863,808 B
2× current page raster:  1664 × 2322 px = 15,455,232 B
```

The 2× backing geometry doubles each pixel dimension and therefore quadruples raw byte volume, as expected.

The qualification workload's 256px logical tile is not a recommended or frozen production tile size. It is simply a concrete, common-sized unit that exposes the scale of the local raster data plane.

## Independent timing observations

First native run:

```text
1× 12-tile pass: min 976 µs, p50 1084 µs
2× 12-tile pass: min 1929 µs, p50 2098 µs
```

Unchanged rerun on a different hosted runner:

```text
1× 12-tile pass: min 1260 µs, p50 1400 µs
2× 12-tile pass: min 2301 µs, p50 2481 µs
```

The checksums and all structural byte geometry reproduced exactly while timings moved materially between runner environments. This is why CI requires only positive internally consistent timing observations, not a threshold.

These numbers also describe engine paint cost, not end-to-end UI latency. They do not include future worker scheduling, shared-buffer synchronization, composition, GPU upload or presentation.

## Why inline control-frame pixels are rejected

The disposable native R0A control path currently has a roughly 1 KiB payload ceiling. Relative to that envelope:

```text
1× 256px tile      =   256× the current control payload cap
2× 256px tile      =  1024× the current control payload cap
1× visible viewport = 3072× the current control payload cap
2× visible viewport = 12288× the current control payload cap
```

The final control limit does not need to stay at 1 KiB for the conclusion to hold. The payload classes differ by orders of magnitude and have different lifetime/synchronization requirements.

Therefore raw raster bytes are a local bulk **data plane**, while commands, descriptors, authority tags and completion signals remain on the bounded **control plane**.

ADR-0011 records the resulting architecture: host-owned bounded reusable out-of-band render buffers with scoped worker leases, while the platform-specific shared-memory/mapping backend remains replaceable.

## Relationship to invalidation/revision qualification

ADR-0009 already establishes that native invalidation callbacks can arrive off-thread and race across the mutation-return boundary. A dirty-region callback therefore cannot publish a new buffer or advance revision.

The render-transfer path must preserve this ordering:

```text
callback / command says region needs work
        ↓
application authority is committed/fenced
        ↓
render request carries authority + revision
        ↓
worker writes only its leased buffer
        ↓
completion publishes a candidate result
        ↓
host revalidates lease + authority + revision
        ↓
viewport/cache may consume it
```

A worker crash or authority replacement invalidates unfinished leases and stale completions.

## What the CI contract pins

Mandatory native CI currently requires:

- real Writer document opens;
- tile mode is RGBA or BGRA;
- four bytes per pixel;
- exact qualification viewport/tile geometry;
- exact raw byte arithmetic for the workload;
- exact 1× → 2× geometric/byte scaling;
- positive real paint timing observations;
- non-zero caller-owned-buffer checksum;
- all prior semantic/identity/restart/invalidation qualifications remain green.

CI intentionally does **not** pin:

- absolute paint time;
- package or raster hash as compatibility truth;
- production tile size;
- buffer-pool sizing;
- an OS shared-memory API.

## Next implementation evidence

R0B should now implement the architecture rather than continue debating copy-vs-shared-memory at the control-frame level. The next proof obligations are:

1. host-owned bounded render-buffer pool;
2. scoped buffer leases and generation-safe reuse;
3. explicit geometry/stride/capacity validation;
4. worker-death reclamation;
5. authority/revision rejection of stale completions;
6. mutation fence between advisory invalidations and render scheduling;
7. viewport priority scheduling under a bounded pool.

Broader scroll/zoom/edit traces can tune tile/pool sizes later without reopening the control/data-plane decision.
