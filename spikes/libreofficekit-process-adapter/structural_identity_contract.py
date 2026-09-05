#!/usr/bin/env python3
"""Pin qualified Writer structural-identity relations without promoting probe tokens to API."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

EXPECTED_RELATIONS = {
    "native_adapter_identity_relation_before_split": "0->1;1->2;2->3",
    "native_adapter_identity_relation_split_merge": "0->0;1->-;2->1;3->2",
    "native_adapter_identity_relation_before_merge": "0->-;1->1;2->2",
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
            "usage: structural_identity_contract.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    harness = Path(__file__).with_name("process_harness.py")
    completed = subprocess.run(
        [sys.executable, str(harness), *sys.argv[1:]],
        env=os.environ.copy(),
        capture_output=True,
        text=True,
        check=False,
    )

    # Preserve the underlying qualification trace in CI. The wrapper adds only
    # relation assertions; it does not replace the harness as the diagnostic source.
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return completed.returncode

    observations = parse_observations(completed.stdout)
    for key, expected in EXPECTED_RELATIONS.items():
        actual = observations.get(key)
        if actual != expected:
            raise RuntimeError(
                f"qualified Writer structural identity relation changed: "
                f"{key} expected {expected!r}, observed {actual!r}"
            )

    if observations.get("native_adapter_structural_revision_progression") != "R0-R1-R2":
        raise RuntimeError("structural identity sequence lost exact revision progression")
    if observations.get("native_adapter_identity_probe_repeatable") != "ok":
        raise RuntimeError("identity probe repeatability qualification is missing")
    if observations.get("native_adapter_split_semantics") != "ok":
        raise RuntimeError("split semantic qualification is missing")
    if observations.get("native_adapter_merge_semantics") != "ok":
        raise RuntimeError("merge semantic qualification is missing")

    print("native_adapter_structural_identity_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
