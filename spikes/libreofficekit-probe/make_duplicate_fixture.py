#!/usr/bin/env python3
"""Create a deterministic DOCX fixture with intentionally ambiguous paragraph text."""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path

from make_fixture import CONTENT_TYPES, ROOT_RELS, ZIP_TIMESTAMP, write_entry

DUPLICATE_TEXT = "Duplicate paragraph identity evidence"
UNIQUE_TAIL = "Unique structural neighbour"

# No file-format paragraph IDs are present on purpose. The fixture is designed to
# force identity/reconciliation evidence to come from something other than text
# equality or imported OOXML identifiers.
DOCUMENT = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>{DUPLICATE_TEXT}</w:t></w:r></w:p>
    <w:p><w:r><w:t>{DUPLICATE_TEXT}</w:t></w:r></w:p>
    <w:p><w:r><w:t>{UNIQUE_TAIL}</w:t></w:r></w:p>
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>
"""


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
