#!/usr/bin/env python3
"""Prove paragraph text equality is insufficient identity/reconciliation evidence."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

from process_harness import (
    NativeAdapter,
    STATUS_OK,
    format_relation,
    identity_relation,
    probe_texts,
    probe_tokens,
    require_unique_probe_tokens,
)

COMMAND_CENTER_FIRST_PARAGRAPH = 10
DUPLICATE_TEXT = "Duplicate paragraph identity evidence"
EXPECTED_PARAGRAPHS = (
    DUPLICATE_TEXT,
    DUPLICATE_TEXT,
    "Unique structural neighbour",
)


def center_first_paragraph(adapter: NativeAdapter, request_id: int) -> None:
    payload = adapter.request(request_id, bytes([COMMAND_CENTER_FIRST_PARAGRAPH]))
    if payload != bytes([STATUS_OK, COMMAND_CENTER_FIRST_PARAGRAPH]):
        raise RuntimeError(f"unexpected duplicate-text formatting response: {payload!r}")


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: duplicate_text_identity_probe.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    executable = Path(sys.argv[1]).resolve()
    install = Path(sys.argv[2]).resolve()
    input_docx = Path(sys.argv[3]).resolve()

    with tempfile.TemporaryDirectory(prefix="document-editor-duplicate-identity-") as temp:
        adapter = NativeAdapter(executable, install, Path(temp))
        adapter.open_document(1, input_docx)

        before_revision, before = adapter.identity_probe_snapshot(2)
        before_repeat_revision, before_repeat = adapter.identity_probe_snapshot(3)
        if before_revision != 0 or before_repeat_revision != 0:
            raise RuntimeError("duplicate-text baseline changed revision without mutation")
        if before != before_repeat:
            raise RuntimeError("duplicate-text baseline identity probe is not repeatable")
        if probe_texts(before) != EXPECTED_PARAGRAPHS:
            raise RuntimeError(f"duplicate-text baseline semantic mismatch: {before!r}")
        require_unique_probe_tokens(before, "duplicate-text baseline")

        if before[0][1] != before[1][1]:
            raise RuntimeError("duplicate fixture did not contain equal first/second paragraph text")
        if before[0][0] == before[1][0]:
            raise RuntimeError("equal-text paragraphs unexpectedly shared one live identity token")

        before_semantic_revision, before_semantic = adapter.semantic_snapshot(4)
        if before_semantic_revision != 0 or before_semantic != EXPECTED_PARAGRAPHS:
            raise RuntimeError("normal semantic projection disagrees with duplicate baseline")

        # Mutate only the first paragraph through the already-qualified formatting
        # operation. The command succeeds only after ParaAdjust reads back CENTER.
        # Paragraph text therefore remains intentionally unable to reveal which of
        # the two equal-text paragraphs was the mutation target.
        center_first_paragraph(adapter, 5)

        after_revision, after = adapter.identity_probe_snapshot(6)
        after_repeat_revision, after_repeat = adapter.identity_probe_snapshot(7)
        if after_revision != 1 or after_repeat_revision != 1:
            raise RuntimeError("duplicate-text formatting mutation did not advance R0 -> R1")
        if after != after_repeat:
            raise RuntimeError("duplicate-text identity probe is not repeatable after mutation")
        if probe_texts(after) != EXPECTED_PARAGRAPHS:
            raise RuntimeError("duplicate-text formatting mutation changed paragraph text")
        require_unique_probe_tokens(after, "duplicate-text after formatting")

        after_semantic_revision, after_semantic = adapter.semantic_snapshot(8)
        if after_semantic_revision != 1 or after_semantic != EXPECTED_PARAGRAPHS:
            raise RuntimeError("normal semantic projection disagrees after duplicate mutation")

        equal_text_candidates = sum(1 for _, text in after if text == DUPLICATE_TEXT)
        if equal_text_candidates != 2:
            raise RuntimeError(
                "duplicate-text ambiguity unexpectedly disappeared after formatting mutation"
            )

        relation = identity_relation(before, after)
        adapter.graceful_shutdown(9)

        print(f"native_adapter_duplicate_tokens_before={probe_tokens(before)}")
        print(f"native_adapter_duplicate_tokens_after={probe_tokens(after)}")
        print(
            "native_adapter_duplicate_identity_relation="
            + format_relation(relation)
        )
        print("native_adapter_duplicate_equal_text_distinct_live_objects=ok")
        print(f"native_adapter_duplicate_content_candidates={equal_text_candidates}")
        print("native_adapter_duplicate_text_semantics_unchanged=ok")
        print("native_adapter_duplicate_revision_progression=R0-R1")
        print("native_adapter_duplicate_first_paragraph_center_readback=ok")
        print("native_adapter_duplicate_identity_status=observed")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
