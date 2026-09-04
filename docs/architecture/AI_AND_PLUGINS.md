# AI and Plugin Architecture

Status: **Normative direction; runtime technology remains to be qualified**

Related: [FEATURES_AND_EXTENSIONS.md](FEATURES_AND_EXTENSIONS.md), [SECURITY.md](SECURITY.md), [TRANSACTIONS_AND_ANCHORS.md](TRANSACTIONS_AND_ANCHORS.md)

## Shared rule

AI and plugins are clients of typed product APIs. They do not receive direct LibreOffice/engine implementation access.

The common **feature composition model** is defined in [FEATURES_AND_EXTENSIONS.md](FEATURES_AND_EXTENSIONS.md). This document focuses on external-code trust, permissions and AI-specific behaviour.

Bundled first-party features and third-party extensions may share stable feature IDs, service contracts and contribution concepts, but they do not automatically share the same runtime or trust boundary.

## AI modes

The product should support policy states such as:

- Off;
- Local only;
- Ask before network;
- Approved cloud providers.

AI is an optional product capability. Core editing, history, recovery and file interoperability must not depend on a cloud AI service.

## AI suggestions

A mutating suggestion contains:

- source revision;
- anchored affected range/object identity;
- expected source state;
- proposed transaction;
- provenance/provider metadata sufficient for audit UI.

Applying a suggestion validates preconditions. A suggestion computed against old content never blindly overwrites newer edits.

AI edits enter through the same command/transaction admission path as human edits. AI providers do not receive a privileged document mutation mechanism.

## External plugin model

External plugins should run in a capability-based sandbox, preferably WASM if qualification confirms that it meets performance, packaging and desktop-integration requirements.

Plugins request explicit permissions such as:

```text
document.read.selection
document.read.full
document.modify
ui.command
ui.panel
network:<declared origin>
```

No unrestricted filesystem, network or native-process access by default.

A feature manifest declaring a dependency or service requirement does **not** grant a security capability. Composition and permission are independent checks.

## Trust tiers

### Bundled feature

May run in process when doing so is safe and efficient. It still uses documented product contracts and must not bypass transaction/document authority merely because it is first-party code.

### External sandboxed extension

Runs behind the plugin host and receives only explicitly granted capabilities. Failure must not corrupt document authority or crash the engine/shell.

### Native/legacy extension

Not part of the initial plugin architecture. If native extension support is ever added, it requires a separate ADR because it materially weakens isolation and compatibility guarantees.

## Service providers

An external extension may eventually provide a replaceable service such as a language checker or AI provider. The feature resolver chooses the provider through stable service IDs; the sandbox/capability layer decides whether the chosen implementation is permitted to run and what data it may access.

There is no registration-order or "last plugin wins" provider selection.

## Suite evolution

The permission system can later add domain-specific scopes such as:

```text
spreadsheet.read_cells
spreadsheet.modify_cells
presentation.read_slides
presentation.modify_slides
```

without weakening document security or exposing editor-engine internals.

## Deferred decisions

R0A intentionally does not yet freeze:

- WASM runtime;
- plugin package/signature format;
- extension marketplace/update mechanism;
- serialized manifest format;
- UI contribution schema;
- long-running background-extension lifecycle.

These require focused qualification and ADRs before implementation.
