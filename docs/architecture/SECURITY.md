# Security Architecture

Documents are hostile input.

## Engine worker

The bootstrap office engine receives only the document/resources it needs. Target controls include:

- no ambient network access;
- filesystem access brokered/restricted where platform APIs permit;
- archive/resource limits;
- CPU/memory watchdogs;
- IPC schema validation;
- process isolation from the shell.

## OOXML package limits

Defend against archive bombs, pathological nesting, oversized XML parts, image bombs and malicious external relationships. Limits are explicit policy with compatibility escape mechanisms only when user-approved.

## Macros

Initial DOCM behaviour: preserve macro payload where feasible; do not execute it.

## Plugins

Future plugins use a capability-based sandbox, preferably WASM. No arbitrary native plugin execution in the main process.

## AI/network

Cloud AI is opt-in policy. Selected context is explicit. Full-document upload must never be an invisible side effect of a local editing command.
