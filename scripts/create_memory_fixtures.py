#!/usr/bin/env python3
"""Create deterministic files for Bellbird's local memory probe."""

from __future__ import annotations

import argparse
import base64
import random
import zipfile
from pathlib import Path

MAX_FILE_BYTES = 10 * 1024 * 1024
FIXTURE_BYTES = 1024 * 1024
DOCX_ENTROPY_BYTES = 9 * 1024 * 1024

CONTENT_TYPES = b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>'''

ROOT_RELATIONSHIPS = b'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>'''


def write_fixed_size_text(path: Path, unit: str) -> None:
    encoded_unit = unit.encode("utf-8")
    path.write_bytes((encoded_unit * (FIXTURE_BYTES // len(encoded_unit) + 1))[:FIXTURE_BYTES])


def write_docx(path: Path) -> None:
    random_bytes = random.Random(20261119).randbytes(DOCX_ENTROPY_BYTES)
    content = base64.b64encode(random_bytes)
    document_xml = (
        b'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        b'<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">'
        b"<w:body><w:p><w:r><w:t>"
        + content
        + b"</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"
    )

    fixed_timestamp = (2026, 1, 1, 0, 0, 0)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for name, contents in (
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", ROOT_RELATIONSHIPS),
            ("word/document.xml", document_xml),
        ):
            entry = zipfile.ZipInfo(name, date_time=fixed_timestamp)
            entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, contents)

    size = path.stat().st_size
    if not 8 * 1024 * 1024 <= size <= MAX_FILE_BYTES:
        raise RuntimeError(f"generated DOCX is {size} bytes; expected between 8 and 10 MiB")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_directory", type=Path)
    args = parser.parse_args()
    args.output_directory.mkdir(parents=True, exist_ok=True)

    write_fixed_size_text(
        args.output_directory / "plain-text-1mib.txt",
        "Bellbird memory fixture. The same text repeats to create a deterministic input.\n",
    )
    write_fixed_size_text(
        args.output_directory / "markdown-1mib.md",
        "# Memory fixture\n\nA deterministic paragraph for memory measurement.\n\n",
    )
    write_docx(args.output_directory / "structured-docx-near-limit.docx")

    for path in sorted(args.output_directory.iterdir()):
        if path.is_file():
            print(f"{path.name}: {path.stat().st_size} bytes")


if __name__ == "__main__":
    main()
