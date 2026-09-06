#!/usr/bin/env python3
"""Pin Writer identity-token scope evidence across close/reopen and worker restart."""

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


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: identity_scope_restart_contract.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    probe = Path(__file__).with_name("identity_scope_restart_probe.py")
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
    first = observations.get("native_adapter_scope_tokens_first")
    reopened = observations.get("native_adapter_scope_tokens_reopen")
    restarted = observations.get("native_adapter_scope_tokens_restart")
    if first is None or reopened is None or restarted is None:
        raise RuntimeError("identity-scope qualification omitted token observations")
    if not (first == reopened == restarted):
        raise RuntimeError(
            "qualification token values stopped demonstrating namespace reuse: "
            f"first={first!r} reopen={reopened!r} restart={restarted!r}"
        )

    required = {
        "native_adapter_scope_semantic_view_destroyed_on_close": "ok",
        "native_adapter_scope_same_worker_token_values_reused": "observed",
        "native_adapter_scope_fresh_worker_token_values_reused": "observed",
        "native_adapter_scope_all_views_revision": "R0",
        "native_adapter_scope_semantics_reacquired": "ok",
        "native_adapter_scope_duplicate_content_candidates": "2",
        "native_adapter_identity_scope_status": "observed",
    }
    for key, expected in required.items():
        actual = observations.get(key)
        if actual != expected:
            raise RuntimeError(
                f"identity-scope qualification lost {key}: "
                f"expected {expected!r}, observed {actual!r}"
            )

    print("native_adapter_identity_scope_restart_contract=qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
