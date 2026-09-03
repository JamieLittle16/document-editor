# Documentation Policy

Documentation is part of implementation.

## Every subsystem document should answer

1. What problem does this subsystem own?
2. What does it explicitly *not* own?
3. What are its public interfaces?
4. What invariants must always hold?
5. What other layers may depend on it?
6. What concurrency/process assumptions exist?
7. What are its resource bounds?
8. How does it fail and recover?
9. How is it tested?
10. How is its performance qualified?
11. What technical debt does it contain?
12. Which ADRs justify non-obvious choices?

## ADR policy

Use ADRs for decisions that are expensive to reverse, surprising, contentious, or likely to be rediscovered. ADRs are immutable historical records: supersede them with a new ADR instead of rewriting history.

## PR documentation rule

A PR that changes an invariant, public protocol, process boundary, persistence behaviour, compatibility claim, security model or major dependency must update documentation in the same PR.

## Agent-readiness test

Before delegating a subsystem to an AI coding agent, ask:

> Could a capable agent read the repository and independently infer the intended boundary and acceptance criteria without inventing architecture?

If not, specification is incomplete.

## No duplicate infrastructure

Before adding a new abstraction or service, search the docs and code inventory. If a similar capability exists, extend or explicitly supersede it; do not quietly create a second parallel mechanism.
