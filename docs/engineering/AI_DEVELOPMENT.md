# AI-Assisted Development Protocol

Heavy AI use is a force multiplier only when architecture is constrained enough that parallel agents do not manufacture incompatible local solutions.

## Human/architect responsibilities

- product priorities;
- architecture and invariants;
- expensive dependency choices;
- threat/compatibility model;
- acceptance criteria;
- integration judgement;
- UX judgement;
- deciding whether complexity is justified.

## Good agent tasks

- implement a documented protocol component;
- write deterministic tests for a specified invariant;
- build a benchmark harness;
- add corpus generators;
- implement isolated UI components against stable view-model APIs;
- write adapters behind an already-defined interface;
- investigate measured failures with reproducible inputs;
- maintain documentation inventories.

## Bad agent tasks

- “build the document engine”;
- introduce a second state model because an API was inconvenient;
- add a new dependency to save a small amount of code without review;
- invent new cross-layer abstractions while another agent owns the adjacent subsystem;
- optimise without measurements;
- silently weaken tests to make CI green.

## Parallelisation rule

Parallel workstreams need disjoint ownership or an explicit shared contract. If two agents routinely modify the same architectural seam, the seam is not stable enough for parallel implementation.

## Review checklist for agent code

- obeys dependency direction;
- adds no hidden unbounded queue/cache;
- cancellation/error paths covered;
- no duplicate infrastructure;
- tests assert behaviour rather than implementation trivia;
- docs updated when contracts changed;
- benchmark impact understood for hot paths;
- no accidental licensing/security expansion.
