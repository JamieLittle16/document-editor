#!/usr/bin/env python3
"""Pin qualified Writer paragraph insertion/deletion relations without promoting probe tokens to API."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

EXPECTED_RELATIONS = {
    "native_adapter_identity_relation_before_inserted": "0->1;1->2;2->3",
    "native_adapter_identity_relation_inserted_deleted": "0->0;1->-;2->1;3->2",
    "native_adapter_identity_relation_before_deleted": "0->-;1->1;2->2",
}


def parse_observations(stdout: str) -> dict[str, str]:
    observations: dict[str, str] = {}
    for line in stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator:
            observations[key] = value
    return observations


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: paragraph_insert_delete_contract.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    probe = Path(__file__).with_name("paragraph_insert_delete_probe.py")
    completed = subprocess.run(
        [sys.executable, str(probe), *sys.argv[1:]],
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        check=False,
    )

    # Keep the observation probe as the diagnostic source. This wrapper adds
    # only the relations that have been reproduced on independent native runs.
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return completed.returncode

    observations = parse_observations(completed.stdout)
    for key, expected in EXPECTED_RELATIONS.items():
        actual = observations.get(key)
        if actual != expected:
            raise RuntimeError(
                "qualified Writer paragraph insertion/deletion relation changed: "
                f"{key} expected {expected!r}, observed {actual!r}"
            )

    required = {
        "native_adapter_insert_delete_probe_repeatable": "ok",
        "native_adapter_insert_empty_paragraph_semantics": "ok",
        "native_adapter_delete_inserted_paragraph_semantics": "ok",
        "native_adapter_insert_delete_revision_progression": "R0-R1-R2",
        "native_adapter_insert_delete_status": "qualified",
    }
    for key, expected in required.items():
        actual = observations.get(key)
        if actual != expected:
            raise RuntimeError(
                f"paragraph insertion/deletion qualification lost {key}: "
                f"expected {expected!r}, observed {actual!r}"
            )

    print("native_adapter_insert_delete_identity_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
