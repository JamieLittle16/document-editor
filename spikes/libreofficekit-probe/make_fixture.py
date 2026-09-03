#!/usr/bin/env python3
"""Create a deterministic, minimal DOCX fixture using only the Python standard library."""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>
"""

ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"""

# w14:paraId / w14:textId are deliberately seeded with known values. They are
# compatibility evidence for the semantic-identity spike, not yet product IDs.
DOCUMENT = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document
    xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
    xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
    xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
    mc:Ignorable="w14">
  <w:body>
    <w:p w14:paraId="13579BDF" w14:textId="2468ACE0">
      <w:r><w:t>Document Editor LibreOfficeKit R0A probe</w:t></w:r>
    </w:p>
    <w:p w14:paraId="89ABCDEF" w14:textId="10293847">
      <w:r><w:t>This fixture is generated deterministically in CI.</w:t></w:r>
    </w:p>
    <w:p w14:paraId="A1B2C3D4" w14:textId="55667788">
      <w:r><w:t>Stable semantic identity must be measured, not assumed.</w:t></w:r>
    </w:p>
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>
"""

# ZIP timestamps cannot precede 1980. Pinning timestamps and entry order keeps the
# generated fixture byte-for-byte reproducible across CI runs.
ZIP_TIMESTAMP = (2020, 1, 1, 0, 0, 0)


def write_entry(archive: zipfile.ZipFile, name: str, text: str) -> None:
    info = zipfile.ZipInfo(name, ZIP_TIMESTAMP)
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = 0o100644 << 16
    archive.writestr(info, text.encode("utf-8"))


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} OUTPUT.docx", file=sys.stderr)
        return 2

    output = Path(sys.argv[1])
    output.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(output, "w") as archive:
        write_entry(archive, "[Content_Types].xml", CONTENT_TYPES)
        write_entry(archive, "_rels/.rels", ROOT_RELS)
        write_entry(archive, "word/document.xml", DOCUMENT)

    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
