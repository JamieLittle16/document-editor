#!/usr/bin/env python3
"""Pin the qualified duplicate-text paragraph identity ambiguity relation."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

EXPECTED_RELATION = "0->0;1->1;2->2"


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
            "usage: duplicate_text_identity_contract.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    probe = Path(__file__).with_name("duplicate_text_identity_probe.py")
    completed = subprocess.run(
        [sys.executable, str(probe), *sys.argv[1:]],
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        check=False,
    )

    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return completed.returncode

    observations = parse_observations(completed.stdout)
    relation = observations.get("native_adapter_duplicate_identity_relation")
    if relation != EXPECTED_RELATION:
        raise RuntimeError(
            "qualified duplicate-text formatting identity relation changed: "
            f"expected {EXPECTED_RELATION!r}, observed {relation!r}"
        )

    required = {
        "native_adapter_duplicate_equal_text_distinct_live_objects": "ok",
        "native_adapter_duplicate_content_candidates": "2",
        "native_adapter_duplicate_text_semantics_unchanged": "ok",
        "native_adapter_duplicate_revision_progression": "R0-R1",
        "native_adapter_duplicate_first_paragraph_center_readback": "ok",
        "native_adapter_duplicate_identity_status": "observed",
    }
    for key, expected in required.items():
        actual = observations.get(key)
        if actual != expected:
            raise RuntimeError(
                f"duplicate-text qualification lost {key}: "
                f"expected {expected!r}, observed {actual!r}"
            )

    print("native_adapter_duplicate_text_identity_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
