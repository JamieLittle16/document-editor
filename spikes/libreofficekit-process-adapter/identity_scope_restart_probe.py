#!/usr/bin/env python3
"""Qualify that Writer identity-probe tokens are scoped to one live semantic view."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

from process_harness import (
    COMMAND_CLOSE,
    COMMAND_IDENTITY_PROBE_SNAPSHOT,
    NativeAdapter,
    STATUS_ENGINE_STATE,
    STATUS_OK,
    probe_texts,
    probe_tokens,
    require_unique_probe_tokens,
)

DUPLICATE_TEXT = "Duplicate paragraph identity evidence"
EXPECTED_PARAGRAPHS = (
    DUPLICATE_TEXT,
    DUPLICATE_TEXT,
    "Unique structural neighbour",
)


def require_snapshot(
    adapter: NativeAdapter,
    request_id: int,
    stage: str,
) -> tuple[int, ...]:
    revision, snapshot = adapter.identity_probe_snapshot(request_id)
    repeat_revision, repeat = adapter.identity_probe_snapshot(request_id + 1)
    if revision != 0 or repeat_revision != 0:
        raise RuntimeError(f"{stage} did not begin in fresh revision R0")
    if snapshot != repeat:
        raise RuntimeError(f"{stage} identity projection is not repeatable")
    if probe_texts(snapshot) != EXPECTED_PARAGRAPHS:
        raise RuntimeError(f"{stage} semantic mismatch: {snapshot!r}")
    require_unique_probe_tokens(snapshot, stage)

    semantic_revision, semantic = adapter.semantic_snapshot(request_id + 2)
    if semantic_revision != 0 or semantic != EXPECTED_PARAGRAPHS:
        raise RuntimeError(f"ordinary semantic projection disagrees with {stage}")
    return probe_tokens(snapshot)


def close_document(adapter: NativeAdapter, request_id: int) -> None:
    payload = adapter.request(request_id, bytes([COMMAND_CLOSE]))
    if payload != bytes([STATUS_OK, COMMAND_CLOSE]):
        raise RuntimeError(f"unexpected close response: {payload!r}")

    # A successful close must destroy the retained semantic view before any new
    # identity namespace is acquired.
    closed = adapter.request(request_id + 1, bytes([COMMAND_IDENTITY_PROBE_SNAPSHOT]))
    if len(closed) < 2 or closed[0:2] != bytes(
        [STATUS_ENGINE_STATE, COMMAND_IDENTITY_PROBE_SNAPSHOT]
    ):
        raise RuntimeError(
            "identity projection remained available after semantic-view destruction: "
            f"{closed!r}"
        )


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: identity_scope_restart_probe.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    executable = Path(sys.argv[1]).resolve()
    install = Path(sys.argv[2]).resolve()
    input_docx = Path(sys.argv[3]).resolve()

    with tempfile.TemporaryDirectory(prefix="document-editor-identity-scope-") as temp:
        root = Path(temp)

        same_worker = NativeAdapter(executable, install, root / "same-worker")
        same_worker.open_document(1, input_docx)
        first_tokens = require_snapshot(same_worker, 2, "first live semantic view")

        close_document(same_worker, 5)
        same_worker.open_document(7, input_docx)
        reopened_tokens = require_snapshot(same_worker, 8, "reopened semantic view")
        same_worker.graceful_shutdown(11)

        fresh_worker = NativeAdapter(executable, install, root / "fresh-worker")
        fresh_worker.open_document(20, input_docx)
        restarted_tokens = require_snapshot(fresh_worker, 21, "fresh-worker semantic view")
        fresh_worker.graceful_shutdown(24)

        # The qualification module deliberately allocates probe tokens from 1 in
        # every WriterSemanticView. Reuse is therefore expected evidence of why a
        # naked u64 token cannot cross a view/worker boundary. We compare the tuple
        # shape, not hard-coded numeric values.
        if first_tokens != reopened_tokens:
            raise RuntimeError(
                "expected qualification-token value reuse after same-worker reopen; "
                f"first={first_tokens!r} reopened={reopened_tokens!r}"
            )
        if first_tokens != restarted_tokens:
            raise RuntimeError(
                "expected qualification-token value reuse after fresh-worker restart; "
                f"first={first_tokens!r} restarted={restarted_tokens!r}"
            )

        if len(first_tokens) != 3:
            raise RuntimeError("identity-scope fixture did not contain exactly three paragraphs")
        if EXPECTED_PARAGRAPHS.count(DUPLICATE_TEXT) != 2:
            raise RuntimeError("identity-scope fixture lost duplicate-text ambiguity")

        print(f"native_adapter_scope_tokens_first={first_tokens}")
        print(f"native_adapter_scope_tokens_reopen={reopened_tokens}")
        print(f"native_adapter_scope_tokens_restart={restarted_tokens}")
        print("native_adapter_scope_semantic_view_destroyed_on_close=ok")
        print("native_adapter_scope_same_worker_token_values_reused=observed")
        print("native_adapter_scope_fresh_worker_token_values_reused=observed")
        print("native_adapter_scope_all_views_revision=R0")
        print("native_adapter_scope_semantics_reacquired=ok")
        print("native_adapter_scope_duplicate_content_candidates=2")
        print("native_adapter_identity_scope_status=observed")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
