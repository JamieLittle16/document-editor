# Recovery and Persistence

## Non-negotiable outcome

A crash, worker failure or power interruption should lose as little accepted user input as technically practical and must not corrupt the last durable document.

## Layers

1. Original/user file.
2. Atomic save staging.
3. Application recovery journal.
4. Periodic recoverable checkpoints.
5. Session metadata/history index.

## Save

Never truncate the target file first.

Conceptually:

```text
serialize to sibling/temp staging file
validate basic output
flush data as required by platform policy
atomically replace target where filesystem semantics allow
update durable save marker
```

## Recovery journal

Accepted transactions are journaled independently of explicit Save. Journal durability policy must be measured so it does not introduce typing latency.

## Worker crash

The session manager should be able to:

1. detect worker death;
2. preserve shell/session state;
3. launch a replacement worker;
4. load the most recent durable checkpoint;
5. replay recoverable accepted operations where safe;
6. verify revision/state consistency;
7. notify the user only if recovery is incomplete.

## External modification

Saving over an externally changed file without warning is forbidden. The product needs file identity/change detection and eventually a structured merge/compare workflow.
