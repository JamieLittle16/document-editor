#!/usr/bin/env python3
"""Pin only the stable safety invariants of Writer render invalidation callbacks."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def parse_observations(stdout: str) -> dict[str, str]:
    observations: dict[str, str] = {}
    for line in stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator:
            observations[key] = value
    return observations


def require(observations: dict[str, str], key: str, expected: str) -> None:
    actual = observations.get(key)
    if actual != expected:
        raise RuntimeError(
            f"invalidation qualification lost {key}: "
            f"expected {expected!r}, observed {actual!r}"
        )


def integer(observations: dict[str, str], key: str) -> int:
    raw = observations.get(key)
    if raw is None:
        raise RuntimeError(f"invalidation qualification did not report {key}")
    try:
        return int(raw)
    except ValueError as error:
        raise RuntimeError(
            f"invalidation qualification reported non-integer {key}: {raw!r}"
        ) from error


def main() -> int:
    if len(sys.argv) != 5:
        raise SystemExit(
            "usage: invalidation_revision_contract.py "
            "PROBE INSTALL_PATH PROFILE_URL INPUT.docx"
        )

    probe = Path(sys.argv[1])
    completed = subprocess.run(
        [str(probe), *sys.argv[2:]],
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        check=False,
    )

    # Preserve the complete native trace. The contract below intentionally does
    # not pin callback count, rectangle payload, exact delivery phase, or event
    # ordering: unchanged-code qualification already observed the tile
    # invalidation on opposite sides of the mutation-return boundary.
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return completed.returncode

    observations = parse_observations(completed.stdout)
    for key, expected in {
        "native_callback_semantic_revision_progression": "R0-R1",
        "native_callback_format_readback_verified": "ok",
        "native_callback_text_semantics_unchanged": "ok",
        "native_callback_render_hash_changed": "yes",
        "native_callback_observation_status": "observed",
        "native_callback_first_invalidation_host_revision": "0",
    }.items():
        require(observations, key, expected)

    total_events = integer(observations, "native_callback_total_events")
    invalidations = integer(observations, "native_callback_invalidate_tiles_count")
    during_call = integer(
        observations, "native_callback_invalidations_during_mutation_call"
    )
    before_revision = integer(
        observations, "native_callback_invalidations_after_return_before_revision"
    )
    off_owner_thread = integer(observations, "native_callback_off_owner_thread_events")

    if total_events < 1:
        raise RuntimeError("verified Writer mutation emitted no callback evidence")
    if invalidations < 1:
        raise RuntimeError("render-changing Writer mutation emitted no tile invalidation")
    if during_call + before_revision < 1:
        raise RuntimeError(
            "no tile invalidation was observed before Office's modeled revision commit"
        )
    if off_owner_thread < 1:
        raise RuntimeError(
            "pinned Writer baseline no longer demonstrates cross-thread callback delivery"
        )

    # The two independent observations intentionally differed here:
    # `returned-before-revision` in one execution and `mutation-call` in the
    # other. Either is compatible with the architectural conclusion; neither is
    # a revision or transaction ordering guarantee.
    phase = observations.get("native_callback_first_invalidation_phase")
    if phase not in {"mutation-call", "returned-before-revision"}:
        raise RuntimeError(
            "first pre-commit invalidation left the qualified race window: "
            f"observed {phase!r}"
        )

    print("native_callback_invalidation_revision_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
