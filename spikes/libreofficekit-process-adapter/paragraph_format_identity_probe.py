#!/usr/bin/env python3
"""Measure Writer paragraph object continuity across a verified formatting-only edit."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

from process_harness import (
    EXPECTED_PARAGRAPHS,
    NativeAdapter,
    STATUS_OK,
    format_relation,
    identity_relation,
    probe_texts,
    probe_tokens,
    require_unique_probe_tokens,
)

COMMAND_CENTER_FIRST_PARAGRAPH = 10


def center_first_paragraph(adapter: NativeAdapter, request_id: int) -> None:
    payload = adapter.request(request_id, bytes([COMMAND_CENTER_FIRST_PARAGRAPH]))
    if payload != bytes([STATUS_OK, COMMAND_CENTER_FIRST_PARAGRAPH]):
        raise RuntimeError(f"unexpected paragraph-format response: {payload!r}")


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: paragraph_format_identity_probe.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    executable = Path(sys.argv[1]).resolve()
    install = Path(sys.argv[2]).resolve()
    input_docx = Path(sys.argv[3]).resolve()

    with tempfile.TemporaryDirectory(prefix="document-editor-format-identity-") as temp:
        adapter = NativeAdapter(executable, install, Path(temp))
        adapter.open_document(1, input_docx)

        before_revision, before = adapter.identity_probe_snapshot(2)
        before_repeat_revision, before_repeat = adapter.identity_probe_snapshot(3)
        if before_revision != 0 or before_repeat_revision != 0:
            raise RuntimeError("formatting baseline changed revision without mutation")
        if before != before_repeat:
            raise RuntimeError("formatting baseline identity probe is not repeatable")
        if probe_texts(before) != EXPECTED_PARAGRAPHS:
            raise RuntimeError(f"formatting baseline semantic mismatch: {before!r}")
        require_unique_probe_tokens(before, "formatting baseline")

        before_semantic_revision, before_semantic = adapter.semantic_snapshot(4)
        if before_semantic_revision != 0 or before_semantic != EXPECTED_PARAGRAPHS:
            raise RuntimeError("normal semantic projection disagrees with formatting baseline")

        # The native semantic module reports success only after Writer's
        # ParaAdjust property reads back as CENTER. This command therefore
        # represents one verified formatting-only mutation, not mere dispatch.
        center_first_paragraph(adapter, 5)

        after_revision, after = adapter.identity_probe_snapshot(6)
        after_repeat_revision, after_repeat = adapter.identity_probe_snapshot(7)
        if after_revision != 1 or after_repeat_revision != 1:
            raise RuntimeError("formatting mutation did not advance revision exactly once")
        if after != after_repeat:
            raise RuntimeError("identity probe is not repeatable after formatting mutation")
        if probe_texts(after) != EXPECTED_PARAGRAPHS:
            raise RuntimeError(f"formatting-only mutation changed paragraph text: {after!r}")
        require_unique_probe_tokens(after, "after formatting mutation")

        after_semantic_revision, after_semantic = adapter.semantic_snapshot(8)
        if after_semantic_revision != 1 or after_semantic != EXPECTED_PARAGRAPHS:
            raise RuntimeError("normal semantic projection changed under formatting-only mutation")

        relation = identity_relation(before, after)
        adapter.graceful_shutdown(9)

        print(f"native_adapter_format_tokens_before={probe_tokens(before)}")
        print(f"native_adapter_format_tokens_after={probe_tokens(after)}")
        print(
            "native_adapter_identity_relation_before_format="
            + format_relation(relation)
        )
        print("native_adapter_format_probe_repeatable=ok")
        print("native_adapter_format_text_semantics_unchanged=ok")
        print("native_adapter_format_revision_progression=R0-R1")
        print("native_adapter_first_paragraph_center_readback=ok")
        print("native_adapter_format_identity_status=observed")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
