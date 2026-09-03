#!/usr/bin/env python3
"""Project minimal DOCX paragraph semantics and measure identity through LibreOffice."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree
from zipfile import ZipFile

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
W14 = "http://schemas.microsoft.com/office/word/2010/wordml"
PARA_ID = f"{{{W14}}}paraId"
TEXT_ID = f"{{{W14}}}textId"
TEXT = f"{{{W}}}t"
PARAGRAPH = f"{{{W}}}p"

EXPECTED = (
    ("13579BDF", "2468ACE0", "Document Editor LibreOfficeKit R0A probe"),
    ("89ABCDEF", "10293847", "This fixture is generated deterministically in CI."),
    ("A1B2C3D4", "55667788", "Stable semantic identity must be measured, not assumed."),
)
EDIT_MARKER = "R0A_EDIT_MARKER_7F3D"


@dataclass(frozen=True)
class Paragraph:
    para_id: str | None
    text_id: str | None
    text: str


def snapshot(path: Path) -> tuple[Paragraph, ...]:
    with ZipFile(path) as archive:
        root = ElementTree.fromstring(archive.read("word/document.xml"))
    paragraphs: list[Paragraph] = []
    for node in root.iter(PARAGRAPH):
        paragraphs.append(
            Paragraph(
                para_id=node.attrib.get(PARA_ID),
                text_id=node.attrib.get(TEXT_ID),
                text="".join(text.text or "" for text in node.iter(TEXT)),
            )
        )
    return tuple(paragraphs)


def find_containing(paragraphs: tuple[Paragraph, ...], needle: str) -> Paragraph:
    matches = [paragraph for paragraph in paragraphs if needle in paragraph.text]
    if len(matches) != 1:
        raise SystemExit(
            f"expected exactly one paragraph containing {needle!r}; found {len(matches)}"
        )
    return matches[0]


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {Path(sys.argv[0]).name} INPUT.docx ROUNDTRIP.docx", file=sys.stderr)
        return 2

    input_path = Path(sys.argv[1])
    roundtrip_path = Path(sys.argv[2])
    before = snapshot(input_path)
    after = snapshot(roundtrip_path)

    if len(before) != len(EXPECTED):
        raise SystemExit(f"fixture paragraph count changed: expected {len(EXPECTED)}, got {len(before)}")

    for paragraph, (expected_para_id, expected_text_id, expected_text) in zip(
        before, EXPECTED, strict=True
    ):
        if paragraph != Paragraph(expected_para_id, expected_text_id, expected_text):
            raise SystemExit(f"fixture semantic projection changed unexpectedly: {paragraph!r}")

    para_ids = [paragraph.para_id for paragraph in after if paragraph.para_id is not None]
    if len(para_ids) != len(set(para_ids)):
        raise SystemExit("round-trip DOCX contains duplicate w14:paraId values")

    matched_after = tuple(find_containing(after, expected_text) for _, _, expected_text in EXPECTED)
    preserved_para_ids = sum(
        paragraph.para_id == expected_para_id
        for paragraph, (expected_para_id, _, _) in zip(matched_after, EXPECTED, strict=True)
    )
    preserved_text_ids = sum(
        paragraph.text_id == expected_text_id
        for paragraph, (_, expected_text_id, _) in zip(matched_after, EXPECTED, strict=True)
    )

    if not any(EDIT_MARKER in paragraph.text for paragraph in after):
        raise SystemExit("round-trip semantic snapshot does not contain the LibreOffice edit marker")

    print(f"semantic_snapshot_input_paragraphs={len(before)}")
    print(f"semantic_snapshot_roundtrip_paragraphs={len(after)}")
    print(f"semantic_snapshot_matched_paragraphs={len(matched_after)}")
    print(f"semantic_snapshot_para_ids_present={len(para_ids)}")
    print(f"semantic_snapshot_para_ids_preserved={preserved_para_ids}/{len(EXPECTED)}")
    print(f"semantic_snapshot_text_ids_preserved={preserved_text_ids}/{len(EXPECTED)}")
    for index, paragraph in enumerate(matched_after, start=1):
        print(
            "semantic_snapshot_after_"
            f"{index}=paraId:{paragraph.para_id or '-'},textId:{paragraph.text_id or '-'},"
            f"text:{paragraph.text}"
        )
    print("semantic_snapshot_status=measured")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
