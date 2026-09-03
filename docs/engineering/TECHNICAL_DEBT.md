# Technical Debt Register

Technical debt is recorded before it becomes folklore. Severity is architectural impact, not criticism of the upstream project.

| ID | Debt / risk | Severity | Containment | Removal / review trigger |
|---|---|---:|---|---|
| TD-001 | LibreOffice historical C++ architecture | Very high | out-of-process worker + narrow adapter | native engine reaches fidelity gate |
| TD-002 | SolarMutex / substantial serialized engine behaviour | High | independent workers; never block UI | benchmark native/alternate engine |
| TD-003 | UNO/LOK API complexity/churn | High | versioned adapter; pin supported LO versions | each LO upgrade |
| TD-004 | Bootstrap worker memory footprint | Medium-high | measured hot/warm/cold lifecycle | multi-document benchmark |
| TD-005 | Tile/render latency from bootstrap engine | Medium | async priority scheduler + bounded cache | viewport profiling |
| TD-006 | Stable semantic identity extraction may be imperfect | High | revision-scoped anchors + validation | comments/AI/review feature gates |
| TD-007 | Word behaviours are partly undocumented | Permanent | compatibility corpus/oracles | every compatibility release |
| TD-008 | UI framework is not yet selected | Medium | keep app core UI-agnostic | R0A UI spike |
| TD-009 | LanguageTool/JVM may be useful but heavy | Medium | service boundary/backend interface | native grammar coverage improves |
| TD-010 | Native/OpenDoc engine currently below Word-grade fidelity | High today | shadow/test use only | corpus score gate |
| TD-011 | Collaboration can distort local model if introduced early | High | defer behind transaction model | post daily-driver |
| TD-012 | Plugin ecosystem can become security/UX debt | High | capability WASM model | plugin phase |
| TD-013 | AI can generate large volumes of inconsistent code | Very high | contracts, ownership, review, qualification | continuous |
| TD-014 | Premature suite expansion | Very high | document-first constitutional rule | document R3 quality gate |
| TD-015 | Licensing strategy not yet chosen | High | repository remains UNLICENSED | before public contribution/release |

## Debt rule

A new workaround that violates an architectural invariant requires either:

- redesign before merge; or
- an explicit debt entry containing owner, containment, and removal trigger.

“Temporary” without an exit condition is not accepted.
