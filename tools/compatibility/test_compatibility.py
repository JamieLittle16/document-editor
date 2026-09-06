"""Unit tests for the normalized compatibility harness."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

from tools.compatibility.docx_semantics import project_paragraph_text
from tools.compatibility.runner import (
    CompatibilityError,
    ParagraphTextProjection,
    assert_projection,
    load_manifest,
)


class CompatibilityHarnessTests(unittest.TestCase):
    def test_docx_projection_normalizes_runs_and_ignores_ids(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.docx"
            document = """<?xml version="1.0" encoding="UTF-8"?>
<w:document
 xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
 xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml">
  <w:body>
    <w:p w14:paraId="ABCDEF01"><w:r><w:t>Hello</w:t></w:r><w:r><w:t> world</w:t></w:r></w:p>
    <w:p w14:paraId="ABCDEF02"><w:r><w:t>Second</w:t></w:r></w:p>
  </w:body>
</w:document>
"""
            with ZipFile(path, "w", ZIP_DEFLATED) as archive:
                archive.writestr("word/document.xml", document)

            projection = project_paragraph_text(path)

            self.assertEqual(projection.paragraphs, ("Hello world", "Second"))
            self.assertEqual(
                projection.as_json(),
                {
                    "projection": "docx-paragraph-text-v1",
                    "paragraphs": ["Hello world", "Second"],
                },
            )

    def test_manifest_loads_strict_versioned_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixtures.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": "office.compatibility-manifest.v1",
                        "fixtures": [
                            {
                                "id": "writer-basic",
                                "generator": "writer-r0a-basic-v1",
                                "operation": "lok-r0a-prefix-edit-v1",
                                "projection": "docx-paragraph-text-v1",
                                "expected_before": ["before"],
                                "expected_after": ["after"],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            manifest = load_manifest(path)

            self.assertEqual(len(manifest.fixtures), 1)
            fixture = manifest.fixtures[0]
            self.assertEqual(fixture.fixture_id, "writer-basic")
            self.assertEqual(fixture.expected_before, ("before",))
            self.assertEqual(fixture.expected_after, ("after",))

    def test_manifest_rejects_unknown_fields(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixtures.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": "office.compatibility-manifest.v1",
                        "fixtures": [
                            {
                                "id": "writer-basic",
                                "generator": "writer-r0a-basic-v1",
                                "operation": "lok-r0a-prefix-edit-v1",
                                "projection": "docx-paragraph-text-v1",
                                "expected_before": ["before"],
                                "expected_after": ["after"],
                                "implicit_magic": True,
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CompatibilityError, "keys changed"):
                load_manifest(path)

    def test_manifest_rejects_duplicate_fixture_ids(self) -> None:
        fixture = {
            "id": "duplicate",
            "generator": "writer-r0a-basic-v1",
            "operation": "lok-r0a-prefix-edit-v1",
            "projection": "docx-paragraph-text-v1",
            "expected_before": ["before"],
            "expected_after": ["after"],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixtures.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": "office.compatibility-manifest.v1",
                        "fixtures": [fixture, fixture],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(CompatibilityError, "duplicate fixture id"):
                load_manifest(path)

    def test_projection_assertion_rejects_semantic_mismatch(self) -> None:
        actual = ParagraphTextProjection(("actual",))

        with self.assertRaisesRegex(CompatibilityError, "semantic projection mismatch"):
            assert_projection(actual, ("expected",), "fixture after")


if __name__ == "__main__":
    unittest.main()
