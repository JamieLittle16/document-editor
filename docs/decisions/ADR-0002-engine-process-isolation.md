# ADR-0002: Run heavyweight document engines out of process

- Status: Accepted
- Date: 2026-09-03

## Context

The bootstrap engine can block, crash, consume substantial memory, or contain vulnerable parsers. Loading it into the shell would make its failure modes application failure modes.

## Decision

Host heavyweight document engines in worker processes controlled by a session manager.

## Consequences

Positive:
- crash/hang isolation;
- independent documents can make progress;
- stronger sandbox/resource control;
- cleaner replacement seam.

Negative:
- IPC complexity;
- rendering payload transfer cost;
- worker lifecycle/memory policy required.

## Follow-up

R0A benchmarks one-worker-per-document versus bounded hot worker policies before committing to a permanent residency strategy.
