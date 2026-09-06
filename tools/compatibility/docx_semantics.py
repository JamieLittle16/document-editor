"""Small, deterministic DOCX semantic projections for compatibility fixtures."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from xml.etree import ElementTree
from zipfile import BadZipFile, ZipFile

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
PARAGRAPH = f"{{{W}}}p"
TEXT = f"{{{W}}}t"
DOCUMENT_XML = "word/document.xml"


class DocxProjectionError(RuntimeError):
    """The package cannot produce the requested normalized semantic projection."""


@dataclass(frozen=True)
class ParagraphTextProjection:
    """Ordered paragraph text, intentionally independent of OOXML/native object IDs."""

    paragraphs: tuple[str, ...]

    def as_json(self) -> dict[str, object]:
        return {
            "projection": "docx-paragraph-text-v1",
            "paragraphs": list(self.paragraphs),
        }


def project_paragraph_text(path: Path) -> ParagraphTextProjection:
    """Project ordered `w:t` text for every `w:p` in `word/document.xml`.

    This deliberately narrow v1 projection is enough for the first compatibility
    fixtures and is stable across ZIP/package serialization differences. It does
    not claim to model tabs, drawings, fields, styles, lists, tables or layout;
    later projections can extend the manifest without changing this contract.
    """

    if not path.is_file():
        raise DocxProjectionError(f"DOCX does not exist: {path}")

    try:
        with ZipFile(path) as archive:
            bad_member = archive.testzip()
            if bad_member is not None:
                raise DocxProjectionError(
                    f"DOCX contains corrupt member {bad_member!r}: {path}"
                )
            if DOCUMENT_XML not in archive.namelist():
                raise DocxProjectionError(
                    f"DOCX has no {DOCUMENT_XML!r}: {path}"
                )
            document_xml = archive.read(DOCUMENT_XML)
    except BadZipFile as error:
        raise DocxProjectionError(f"invalid DOCX ZIP package: {path}") from error

    try:
        root = ElementTree.fromstring(document_xml)
    except ElementTree.ParseError as error:
        raise DocxProjectionError(
            f"invalid XML in {DOCUMENT_XML!r}: {path}"
        ) from error

    paragraphs = tuple(
        "".join(text.text or "" for text in paragraph.iter(TEXT))
        for paragraph in root.iter(PARAGRAPH)
    )
    return ParagraphTextProjection(paragraphs)
