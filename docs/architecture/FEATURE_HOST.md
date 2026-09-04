# Feature Host

Status: **Normative R0A implementation contract**

Related: [FEATURES_AND_EXTENSIONS.md](FEATURES_AND_EXTENSIONS.md), [ARCHITECTURE.md](ARCHITECTURE.md), [ADR-0006](../decisions/ADR-0006-modular-feature-kernel.md)

## 1. Problem owned

`feature-host` owns supervised runtime lifecycle for **trusted bundled features** after declarative feature resolution.

It proves that first-party product modules can be composed rather than hard-wired while preserving deterministic startup, deterministic shutdown and recoverable failure semantics.

It owns:

- binding one bundled implementation to one stable feature manifest;
- rejecting external/untrusted features from the in-process path;
- resolving a complete profile before activation;
- activating dependencies/providers before consumers;
- reverse-order shutdown;
- rollback after activation failure;
- tracking cleanup failures explicitly as a faulted host state;
- exposing only read-only resolved provider/profile context during activation.

## 2. Explicitly not owned

The bundled host does not own:

- feature dependency resolution algorithms (`extension-runtime` owns them);
- stable IDs/manifests (`extension-api` owns them);
- external plugin loading or sandboxing;
- WASM runtime selection;
- plugin package/signature/update formats;
- UI contribution schemas;
- authoritative document state;
- command/transaction admission;
- engine process lifecycle;
- persistence/recovery journals.

External extensions must later use a separate sandbox host. Sharing manifest concepts is not permission to cross the trust boundary.

## 3. Public interfaces

### `BundledFeature`

A trusted statically linked feature exposes:

- stable `id()`;
- `activate(read_only_context)`;
- `deactivate()`.

`deactivate()` must be safe after a partially failed activation because the host invokes cleanup on the failing feature before rolling back previously active dependencies.

### `FeatureHost`

Owns:

- manifest catalogue;
- bundled feature instances;
- active feature ordering;
- current resolved plan;
- coarse lifecycle state.

The lifecycle is:

```text
Ready
  | start(valid profile)
  v
Running
  | stop(clean)
  v
Ready

Ready
  | activation failure + clean rollback
  v
Ready

Running/Ready startup
  | cleanup failure
  v
Faulted
  | stop() retries remaining cleanup
  +------------------------------> Ready (when cleanup succeeds)
```

## 4. Invariants

1. Resolution completes before any feature activation.
2. External manifests never execute through the trusted bundled host.
3. Manifest ID equals implementation ID.
4. An implementation is registered once per stable feature ID.
5. Provider/dependency ordering comes only from the resolved plan.
6. A feature that fails activation is immediately asked to deactivate itself.
7. Previously activated features roll back in reverse activation order.
8. Cleanup continues after individual deactivation failures so all possible resources are released.
9. A cleanup failure is never reported as a clean rollback/shutdown.
10. The host remains `Faulted` while resources may still be active.
11. Features receive no mutable host state or authoritative document state.
12. Runtime catalogue mutation is forbidden while running or faulted.

## 5. Dependency direction

```text
feature-host -> extension-runtime -> extension-api
          \--> extension-api

bundled feature implementation -> extension-api / typed product APIs
```

`feature-host` must remain independent of LibreOffice, UI frameworks and external plugin runtimes.

## 6. Concurrency/process assumptions

R0A lifecycle is deliberately synchronous and deterministic at the application orchestration layer.

This does **not** permit feature activation to perform arbitrary blocking work on a UI thread. The eventual application composition root must invoke lifecycle at safe startup/shutdown points and features must move expensive/background work behind supervised async services.

Document-derived asynchronous work remains revision-tagged under the broader architecture rules.

## 7. Resource bounds

The bundled host retains:

- one manifest and one feature instance per registered bundled feature;
- one active feature ID per active feature;
- one resolved plan for the current run.

It introduces no unbounded queue, task pool or callback fan-out.

Contribution counts and external plugin budgets remain deferred until contribution schemas/sandboxing are selected.

## 8. Failure and recovery

### Resolution failure

No feature activates. Host remains `Ready`.

### Activation failure

The failing feature is asked to deactivate first, then already-active features roll back in reverse order.

If all cleanup succeeds, the host returns to `Ready`.

If any cleanup fails, the failing feature IDs remain tracked and the host becomes `Faulted`.

### Stop failure

Shutdown continues across every active feature. Failed cleanups remain tracked. Repeated `stop()` calls retry only still-active/faulted features.

The host never claims successful cleanup while a feature has reported cleanup failure.

## 9. Testing

Required R0A tests prove:

- resolved activation order;
- reverse shutdown order;
- zero activation after resolution failure;
- cleanup of the feature whose activation failed;
- rollback of earlier features;
- faulted state after rollback cleanup failure;
- cleanup retry semantics;
- explicit selected provider activation before consumer;
- provider visibility through read-only activation context;
- rejection of external features from the trusted host;
- manifest/implementation identity agreement.

Later additions require lifecycle property tests over generated graphs and failure injection across every activation/deactivation position.

## 10. Performance qualification

The host executes only lifecycle/control-plane work. It is not a per-keystroke dispatch layer.

Bundled features remain free to use statically linked typed Rust calls on hot paths after activation. The host must never become a universal dynamic service locator consulted inside rendering/layout/editing loops.

## 11. Security position

`FeatureOrigin::External` is rejected by `FeatureHost::register`.

This is intentional defence in depth. When the external plugin host exists, it will consume compatible product concepts across an explicit sandbox/capability boundary rather than turning third-party code into an in-process `BundledFeature`.

## 12. Technical debt / deferred work

Deliberately deferred:

- external sandbox host;
- WASM/runtime selection;
- hot reconfiguration while documents are open;
- per-document feature profiles;
- contribution registries (commands/panels/diagnostics/settings);
- supervised background task handles;
- structured feature-specific diagnostic codes beyond the host-level typed failure categories.

These require separate contracts/ADRs rather than ad-hoc additions to `BundledFeature`.
