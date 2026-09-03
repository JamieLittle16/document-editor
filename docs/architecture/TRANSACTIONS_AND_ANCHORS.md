# Transactions, Revisions and Anchors

## Goal

Undo, recovery, comments, diagnostics, asynchronous proofreading, AI edits and later collaboration all require a coherent mutation model.

## Revision

A document session has a monotonically increasing logical revision. State-dependent background work always identifies the source revision.

## Transaction

A transaction is an atomic user-meaningful group of document operations. Examples:

- typing one coherent insertion group;
- replacing selected text;
- applying a paragraph style;
- accepting a proofreading suggestion;
- inserting a table;
- an AI rewrite accepted by the user.

A transaction records enough information to support history, invalidation and—where the active engine permits—an inverse or engine-level undo correspondence.

## Anchors

Raw global character offsets are insufficient. An anchor should conceptually contain:

```text
object identity
local grapheme position
affinity / edge behaviour
source revision
```

The engine maps anchors through mutations.

## Rules

1. A stale anchor is never assumed valid against a newer revision.
2. Suggestions include expected source text or equivalent preconditions.
3. Applying a suggestion validates its preconditions.
4. Position mapping is explicit output of mutation handling.
5. Unicode positions are defined in grapheme/text-model terms, never accidental byte indices.

## Bootstrap limitation

LibreOffice's native undo/identity semantics may not map perfectly onto our final transaction algebra. R0A must prove the smallest faithful adapter without maintaining a competing authoritative model.
