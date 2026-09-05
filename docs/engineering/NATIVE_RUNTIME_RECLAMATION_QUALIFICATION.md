# R0A Native Runtime Reclamation Qualification

Status: qualification evidence for the pinned LibreOffice 24.2 environment; **not** a permanent production shutdown contract.

## Purpose

Office deliberately keeps the heavyweight bootstrap engine out of process. That boundary must preserve two different kinds of lifecycle evidence:

1. whether Office-owned native objects can be released correctly; and
2. whether process-global runtime finalizers supplied by the bootstrap engine are safe to execute afterwards.

Those are not the same property.

The R0A same-instance Writer semantic spike exposed a concrete distinction on Ubuntu 24.04 / LibreOffice 24.2.7.2. This document records the measured behaviour and the temporary containment rule used by the qualification adapter.

## What was measured

The native process adapter starts LibreOfficeKit, opens a Writer document, dynamically loads a version-pinned semantic compatibility module, obtains the exact live `XTextDocument` from the existing LibreOfficeKit process, returns bounded semantic observations, and then closes the semantic/document state.

The compatibility module contains all UNO and LibreOffice-internal dependencies. The process adapter executable itself does not link UNO or `libmergedlo`.

The following destruction sequence was exercised:

```text
semantic view release
    -> compatibility module unload
    -> lok::Document destruction
    -> lok::Office destruction
    -> process finalization
```

A one-run diagnostic registered a process-finalization marker after semantic acquisition. CI observed the marker **before** the LibreOffice fault:

```text
native_adapter_lifecycle=entered_process_finalization
... LibreOffice LocaleDataWrapper / rtl_uString_release fault ...
```

Therefore the crash occurs after automatic `lok::Office` destruction has returned, during later process-wide static finalization. It is not evidence that Office failed to release its semantic view, module, document, or `lok::Office` object.

The diagnostic marker was removed after establishing this ordering.

## Temporary R0A containment rule

For the pinned qualification adapter, normal worker retirement now performs explicit owned-object destruction in this order:

```text
WriterSemanticView.reset()
    -> unload version-pinned semantic module
lok::Document.reset()
lok::Office.reset()
std::_Exit(status)
```

`std::_Exit` is used **only after the Office-owned/native RAII objects above have been explicitly destroyed**. It skips the subsequently demonstrated-broken process-global static-finalizer phase and lets the operating system reclaim the already-isolated worker process.

This is materially different from using `_Exit` as a shortcut around document or engine cleanup. The qualification requires normal destruction of the semantic view, document, and LibreOfficeKit office object first.

## Why this fits the current architecture

The process is already the failure-containment and native-runtime ownership boundary. R0A does not host LibreOffice inside the application process.

Using process reclamation for bootstrap-engine global statics therefore preserves the important application-level invariants:

- the application process never inherits LibreOffice global state;
- semantic/UNO implementation details remain inside the native worker;
- the semantic compatibility module is unloadable and version-pinned;
- protocol shutdown can complete deterministically before worker retirement;
- process exit status remains meaningful instead of being overwritten by a later bootstrap-runtime crash;
- a fresh worker can restart with a clean native runtime.

This does **not** imply that a future Office-native engine, a different LibreOffice release, or a future production adapter should use the same retirement mechanism.

## Protocol versus runtime shutdown

The host-visible shutdown contract remains explicit:

1. host sends the bounded shutdown command;
2. worker releases open semantic/document state;
3. worker sends and flushes the successful shutdown response;
4. worker destroys its `lok::Office` object;
5. worker exits with status `0` without running the unsafe process-global static finalizers;
6. host observes stdout EOF and successful process status.

Clean stdin EOF is also a normal supervisor path. The qualification harness requires a worker with a live semantic session to retire with status `0` and clean stdout EOF after the host closes stdin.

Transport/internal failure paths use the same explicit native-object retirement helper while preserving their intended non-zero process status.

## What CI must prove

The native process qualification is not green merely because a shutdown response was emitted. It must establish:

- semantic module build and dynamic load succeed;
- same-instance semantic reads still match the Writer document;
- revision-stamped observations still advance with successful mutation;
- semantic module/view release occurs before document retirement;
- explicit shutdown returns its successful response and process status `0`;
- clean stdin EOF after a live semantic session returns process status `0`;
- stdout reaches clean EOF after both normal retirement paths;
- force-killed workers still report non-success;
- a fresh worker still restarts and reacquires the semantic snapshot.

## Production implications

The production native-adapter ADR remains deliberately deferred. When that boundary is frozen, it must decide explicitly whether the selected bootstrap-engine/version requires process-level runtime reclamation.

A production design must not silently generalize this R0A result into one of these false claims:

- `lok::Office` itself cannot be destroyed safely;
- unloading the semantic compatibility module is what crashes;
- Office may skip document/native-object cleanup;
- every LibreOffice version requires `_Exit`;
- process EOF alone proves graceful shutdown.

The actual qualified statement is narrower:

> In the pinned LibreOffice 24.2.7.2 environment, after same-process internal semantic access, Office-owned semantic/document/LOK objects can be released, but later LibreOffice process-global static finalization faults. The isolated native worker therefore uses the process boundary to reclaim only that remaining global runtime state.

## Next evidence frontier

With authority freshness and native-runtime containment separated cleanly, the next high-risk question remains **semantic identity/reconciliation under structural edits**, beginning with paragraph split and merge.

Those experiments must continue to treat UNO object identity as internal evidence only. Raw UNO references, addresses, or temporary observation tokens must not become product `ParagraphId`s or history anchors by accident.
