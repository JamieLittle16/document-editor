# Dependency Policy

Every significant dependency is classified as:

- **Foundation:** expected to remain long-term;
- **Bootstrap:** accelerates product delivery but is intentionally replaceable;
- **Utility:** narrow, contained capability.

## Review fields

A dependency addition documents:

- problem solved;
- classification;
- licence/attribution obligations;
- security surface;
- maintenance/community status;
- binary/runtime cost;
- platform support;
- whether it enters a hot path;
- abstraction/replacement boundary;
- why existing dependencies or small local code are insufficient.

## Current strategic candidates

| Candidate | Classification | State |
|---|---|---|
| LibreOffice/LibreOfficeKit | Bootstrap | accepted direction |
| UI framework | Replaceable foundation candidate | qualification pending |
| LanguageTool | Bootstrap backend | not yet integrated |
| OpenDoc/native engine | Research/future foundation candidate | not authoritative |
| WASM runtime | Future foundation candidate | not selected |

## Rule

Dependencies do not become architecture by accident. Product/domain crates should depend on interfaces owned by this repository where an external implementation is plausibly replaceable.
