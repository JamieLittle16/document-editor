# AI and Plugin Architecture

## Shared rule

AI and plugins are clients of typed product APIs. They do not receive direct LibreOffice/engine implementation access.

## AI modes

The product should support policy states such as:

- Off;
- Local only;
- Ask before network;
- Approved cloud providers.

## AI suggestions

A mutating suggestion contains source revision, anchored affected range, expected source state and proposed transaction. Applying it validates preconditions. A suggestion computed against old content never blindly overwrites newer edits.

## Plugin model

Future plugins should run in a capability-based sandbox, preferably WASM, and request explicit permissions such as:

```text
document.read.selection
document.read.full
document.modify
ui.command
ui.panel
network:<declared origin>
```

No unrestricted filesystem, network or native-process access by default.

## Suite evolution

The permission system can later add domain-specific scopes (`spreadsheet.read_cells`, `presentation.modify_slides`) without weakening document security.
