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

Transactions are admitted completely before mutation begins. The current text-fixture subset validates revision, resource limits, ranges, UTF-8 boundaries and overlap before changing engine state. A rejected transaction must leave both document state and logical revision untouched.

## Protocol offsets are not semantic anchors

R0A currently has a deliberately narrow `TextOffset(u64)` for the mock/text-fixture engine protocol. It represents a fixed-width UTF-8 byte offset and exists to prove deterministic transaction/revision behaviour across a process-safe value boundary.

It is **not** the final position primitive exposed to product features, history, comments, proofreading or collaboration.

Keeping this distinction explicit prevents a temporary bootstrap representation from silently becoming the permanent document model:

```text
TextOffset
  = low-level fixed-width byte position used by narrow bootstrap operations

Anchor
  = semantic/stable position used by product features across revisions
```

## Anchors

Raw global character offsets are insufficient. An anchor should conceptually contain:

```text
object identity
local grapheme position
affinity / edge behaviour
source revision
```

The engine maps anchors through mutations.

We should prefer persistent semantic/object identities where the active engine can expose them reliably. When the bootstrap engine cannot provide a stable identity directly, the adapter must surface that limitation rather than manufacture false stability.

## Rules

1. A stale anchor is never assumed valid against a newer revision.
2. Suggestions include expected source text or equivalent preconditions.
3. Applying a suggestion validates its preconditions.
4. Position mapping is explicit output of mutation handling.
5. User/product positions are defined in grapheme/text-model terms, never accidental byte indices.
6. Protocol-local fixed-width offsets may exist for constrained operations, but they do not escape as semantic anchors.
7. All edits in a transaction are validated before the first mutation occurs.
8. Invalid/overlapping transactions do not partially apply.
9. Resource limits are explicit policy, not hidden implementation accidents.

## Bootstrap limitation

LibreOffice's native undo/identity semantics may not map perfectly onto our final transaction algebra. R0A must prove the smallest faithful adapter without maintaining a competing authoritative model.

The real LibreOfficeKit spike has now proved a primitive text mutation can persist through DOCX save/reopen. That is capability evidence only: the production adapter still needs a principled mapping from product transactions/anchors to engine operations and invalidation output.
