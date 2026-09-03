# Language, Proofreading and Document Intelligence

## Ownership

The language subsystem owns spelling, grammar, terminology, consistency and optional AI-assisted writing diagnostics. It does not own document mutation or UI rendering.

## Pipeline

```text
revisioned semantic text
        |
  language scheduler
   /       |        \
lexical  grammar   document analysis
   \       |        /
     revisioned diagnostics
             |
       UI presentation
```

## Latency tiers

### Tier 0: lexical / immediate

Target: current token feedback without perceptible typing delay.

- spelling;
- case/word-shape errors;
- repeated word;
- user/project dictionary;
- language profile selection.

### Tier 1: deterministic grammar

Runs after idle/debounce and incrementally on affected sentence/paragraph ranges.

A bootstrap backend may use LanguageTool behind a service boundary. The product API must not depend on LanguageTool-specific rule IDs or JVM lifecycle.

### Tier 2: document intelligence

Background checks across wider context:

- UK/US spelling consistency;
- preferred/rejected terminology;
- abbreviation introduced before definition;
- heading capitalisation convention;
- repeated phrase analysis;
- punctuation/quotation conventions;
- number/range formatting;
- equation/caption/reference consistency.

### Tier 3: optional AI

Potentially remote or local model analysis for clarity, ambiguity, concision, structure and technical explanation. Every result is revision-bound and reviewable.

## Diagnostic contract

A diagnostic contains at minimum:

- stable diagnostic ID for the current analysis generation;
- kind/provider;
- source revision;
- anchor/range;
- severity/confidence;
- human-readable explanation;
- zero or more structured suggestions;
- expected source text or equivalent precondition for mutation-capable suggestions.

## Rules

1. Language analysis never blocks typing.
2. Stale diagnostics are discarded/rebased explicitly.
3. Accepting a suggestion goes through the normal transaction path.
4. Custom vocabulary is scoped (global/project/document) rather than stored in one opaque global dictionary.
5. Network AI is optional and permissioned.
