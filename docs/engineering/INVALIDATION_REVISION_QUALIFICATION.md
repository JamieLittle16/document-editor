# LibreOfficeKit invalidation/revision qualification

Status: qualified for the pinned R0A LibreOffice baseline

Date: 2026-09-06

## Question

Office needs LibreOfficeKit render invalidations so a future viewport can discard stale tiles, but those callbacks cross a native concurrency boundary. Before defining the worker event protocol or render cache, R0A needed to determine whether callback delivery has a reliable ordering relationship with a verified document mutation and Office's authoritative semantic revision.

The critical architectural question is not whether LibreOffice emits invalidations. It is whether a callback can safely be interpreted as transaction completion or revision authority.

## Pinned environment

The qualification runs in mandatory CI on Ubuntu 24.04 against LibreOffice 24.2.7.2 (`420(Build:2)`). It uses the same deterministic three-paragraph Writer fixture and the same unloadable, version-pinned semantic compatibility module as the existing native semantic qualifications.

## Probe design

The probe deliberately avoids queued keyboard/caret input. It uses the already-qualified synchronous first-paragraph formatting mutation:

```text
ParaAdjust = CENTER
```

The compatibility module reads the property back before reporting success. This gives the experiment an independently verified engine-mutation boundary.

For each fresh native process the probe:

1. opens the deterministic Writer fixture;
2. acquires the same live Writer semantic authority;
3. records the baseline paragraph projection and a rendered tile hash;
4. registers a LibreOfficeKit callback recorder and discards startup/view-state events;
5. labels the host authority as `R0` and enters the measured mutation phase;
6. applies the verified formatting mutation;
7. exposes a short qualification-only window after the mutation returns but before host revision commit;
8. advances the modeled Office revision to `R1` only after verified mutation success;
9. proves paragraph-text semantics remain unchanged;
10. re-renders and proves raster output changed;
11. records callback type, payload, delivery phase, callback-time host revision and whether delivery occurred on the owning thread.

The sleep windows exist only to expose possible asynchronous delivery. They are not production synchronization primitives.

## Independent observations

The clean probe was executed twice against unchanged code.

Both runs reproduced these stable facts:

```text
verified ParaAdjust CENTER read-back: OK
paragraph text: unchanged
render hash: changed
semantic revision model: R0 -> R1
callback events: 4
LOK_CALLBACK_INVALIDATE_TILES events: 1
all observed callbacks: off owner thread
first invalidation callback-time host revision: R0
invalidation rectangle: 284, 1724, 10465, 255
```

The important result is that the exact delivery phase **did not reproduce**.

One execution observed:

```text
invalidations during mutation call: 0
invalidations after mutation return, before R1 commit: 1
first invalidation phase: returned-before-revision
```

The independent unchanged-code rerun observed:

```text
invalidations during mutation call: 1
invalidations after mutation return, before R1 commit: 0
first invalidation phase: mutation-call
```

Therefore the callback can race across the mutation-return boundary. The pinned engine does not provide a deterministic callback phase that Office may use as transaction ordering.

## Qualified conclusion

`LOK_CALLBACK_INVALIDATE_TILES` is useful render-dirtiness evidence, but it is **not semantic revision authority**.

In the qualified environment:

- callback delivery can occur on a different thread from the owning/request thread;
- callback delivery can occur while the verified mutation call is still active or after it returns;
- a callback can describe render state affected by a mutation while Office still correctly considers the application authority to be `R0`;
- callback phase/order therefore cannot decide when a transaction committed;
- a formatting mutation can change rendered output while the paragraph-text projection remains byte-for-byte unchanged.

The independent phase drift is intentionally part of the evidence. R0A must not turn either observed scheduling outcome into an engine contract.

## Mandatory contract

`invalidation_revision_contract.py` pins only the safety properties needed by Office:

- the verified formatting mutation succeeds;
- paragraph text remains unchanged;
- rendered output changes;
- the modeled semantic authority advances `R0 -> R1` only through the command path;
- at least one tile invalidation is observed;
- at least one tile invalidation is observed before the modeled revision commit;
- the pinned baseline continues to demonstrate cross-thread callback delivery;
- the first measured invalidation is observed while the modeled host authority is still `R0`.

It intentionally does **not** pin:

- exact callback count;
- exact invalidation rectangle;
- exact event ordering;
- exact delivery phase on either side of the mutation-return boundary;
- a one-mutation/one-callback correspondence.

Those are implementation details, not product invariants.

## Product/worker consequence

The future native callback handler must be a narrow ingestion boundary. It may copy and normalize bounded invalidation data into a thread-safe queue; it must not:

- advance `DocumentRevision`;
- create or replace `AuthorityGeneration`;
- mutate application/session state directly;
- touch UI state directly;
- assume callback payload memory outlives the callback;
- render immediately merely because the callback arrived.

The authoritative command/session path remains responsible for transaction success and revision progression. Render scheduling consumes invalidation evidence only after the corresponding application authority is committed.

A future viewport/render protocol should therefore carry enough product-owned authority to reject stale or cross-incarnation work, naturally including authority generation and document revision. The worker event sequence must also provide a mutation fence/barrier so a pre-commit native callback cannot make the shell consume newly mutated render state while it still believes it is displaying the old revision.

## Non-decisions

This qualification does not yet choose:

- the final worker event wire format;
- callback coalescing policy;
- tile cache geometry;
- copy versus shared-memory render payload transport;
- batching thresholds;
- the final renderer/UI framework.

Those can now be measured without relying on an invalid callback-order assumption.
