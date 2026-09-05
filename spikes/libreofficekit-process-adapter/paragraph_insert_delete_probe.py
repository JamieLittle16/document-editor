#!/usr/bin/env python3
"""Measure Writer paragraph identity across boundary insertion and deletion."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

from process_harness import (
    EXPECTED_PARAGRAPHS,
    NativeAdapter,
    format_relation,
    identity_relation,
    probe_texts,
    probe_tokens,
    require_unique_probe_tokens,
)


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: paragraph_insert_delete_probe.py ADAPTER INSTALL_PATH INPUT.docx"
        )

    executable = Path(sys.argv[1]).resolve()
    install = Path(sys.argv[2]).resolve()
    input_docx = Path(sys.argv[3]).resolve()
    boundary_offset = len(EXPECTED_PARAGRAPHS[0])

    with tempfile.TemporaryDirectory(prefix="document-editor-insert-delete-") as temp:
        adapter = NativeAdapter(executable, install, Path(temp))
        adapter.open_document(1, input_docx)

        before_revision, before = adapter.identity_probe_snapshot(2)
        before_repeat_revision, before_repeat = adapter.identity_probe_snapshot(3)
        if before_revision != 0 or before_repeat_revision != 0:
            raise RuntimeError("insert/delete baseline changed revision without mutation")
        if before != before_repeat:
            raise RuntimeError("insert/delete baseline identity probe is not repeatable")
        if probe_texts(before) != EXPECTED_PARAGRAPHS:
            raise RuntimeError(f"insert/delete baseline semantic mismatch: {before!r}")
        require_unique_probe_tokens(before, "insert/delete baseline")

        # Splitting exactly at the first paragraph end inserts an empty paragraph
        # between paragraph 1 and paragraph 2 without introducing another native
        # mutation primitive or transport command.
        adapter.split_first_paragraph(4, boundary_offset)
        inserted_revision, inserted = adapter.identity_probe_snapshot(5)
        inserted_repeat_revision, inserted_repeat = adapter.identity_probe_snapshot(6)
        expected_inserted = (
            EXPECTED_PARAGRAPHS[0],
            "",
            EXPECTED_PARAGRAPHS[1],
            EXPECTED_PARAGRAPHS[2],
        )
        if inserted_revision != 1 or inserted_repeat_revision != 1:
            raise RuntimeError("paragraph insertion did not advance revision exactly once")
        if inserted != inserted_repeat:
            raise RuntimeError("identity probe is not repeatable after paragraph insertion")
        if probe_texts(inserted) != expected_inserted:
            raise RuntimeError(f"Writer boundary insertion semantics mismatch: {inserted!r}")
        require_unique_probe_tokens(inserted, "after boundary insertion")

        inserted_semantic_revision, inserted_semantic = adapter.semantic_snapshot(7)
        if inserted_semantic_revision != 1 or inserted_semantic != expected_inserted:
            raise RuntimeError("normal semantic projection disagrees with insertion identity probe")

        # Merging paragraph 1 with the inserted empty paragraph removes only the
        # inserted boundary and restores the original semantic paragraph sequence.
        adapter.merge_first_two_paragraphs(8)
        deleted_revision, deleted = adapter.identity_probe_snapshot(9)
        deleted_repeat_revision, deleted_repeat = adapter.identity_probe_snapshot(10)
        if deleted_revision != 2 or deleted_repeat_revision != 2:
            raise RuntimeError("paragraph deletion did not advance revision exactly once")
        if deleted != deleted_repeat:
            raise RuntimeError("identity probe is not repeatable after paragraph deletion")
        if probe_texts(deleted) != EXPECTED_PARAGRAPHS:
            raise RuntimeError(f"Writer insertion/deletion round trip changed semantics: {deleted!r}")
        require_unique_probe_tokens(deleted, "after inserted paragraph deletion")

        deleted_semantic_revision, deleted_semantic = adapter.semantic_snapshot(11)
        if deleted_semantic_revision != 2 or deleted_semantic != EXPECTED_PARAGRAPHS:
            raise RuntimeError("normal semantic projection disagrees after paragraph deletion")

        relation_before_inserted = identity_relation(before, inserted)
        relation_inserted_deleted = identity_relation(inserted, deleted)
        relation_before_deleted = identity_relation(before, deleted)
        adapter.graceful_shutdown(12)

        print(f"native_adapter_insert_delete_tokens_before={probe_tokens(before)}")
        print(f"native_adapter_insert_delete_tokens_inserted={probe_tokens(inserted)}")
        print(f"native_adapter_insert_delete_tokens_deleted={probe_tokens(deleted)}")
        print(
            "native_adapter_identity_relation_before_inserted="
            + format_relation(relation_before_inserted)
        )
        print(
            "native_adapter_identity_relation_inserted_deleted="
            + format_relation(relation_inserted_deleted)
        )
        print(
            "native_adapter_identity_relation_before_deleted="
            + format_relation(relation_before_deleted)
        )
        print("native_adapter_insert_delete_probe_repeatable=ok")
        print("native_adapter_insert_empty_paragraph_semantics=ok")
        print("native_adapter_delete_inserted_paragraph_semantics=ok")
        print("native_adapter_insert_delete_revision_progression=R0-R1-R2")
        print("native_adapter_insert_delete_status=qualified")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
