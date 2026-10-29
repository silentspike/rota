#!/usr/bin/env python3
"""Find archive and help test records in a production build (issue #1177).

The test records in data/archive/test-entries.json and data/help/test-help.json
exist for the test suite and the development build. They must not reach the
player. This tool derives markers from those two files and searches the built
output for them.

A marker is an ASCII run of at least MIN_LENGTH characters taken from any text
of a test record, or a test record id with the prefix ``test-``. A run that also
occurs in the shipped catalogue (archive.json, help.json) is dropped: it would
report the catalogue, not the test data.

Exit status: 0 no finding, 1 at least one finding, 2 nothing to check (no
markers derived, or the build directory is missing or empty).
"""

import argparse
import json
import re
import sys
from pathlib import Path

MIN_LENGTH = 16
TEST_FILES = ("data/archive/test-entries.json", "data/help/test-help.json")
CATALOGUE_FILES = ("data/archive/archive.json", "data/help/help.json")
TEXT_FIELDS = ("title", "body", "effect", "values", "synergies")
BUILD_SUFFIXES = (".js", ".html", ".css", ".json")
ASCII_RUN = re.compile(r"[\x20-\x7e]+")


def records(document):
    """Every record of a data file: archive entries, help entries, introductions."""
    for key in ("entries", "introductions"):
        for item in document.get(key) or []:
            if isinstance(item, dict):
                yield item


def texts(document):
    """Every text a record carries, plus the file comment."""
    comment = document.get("_comment")
    if isinstance(comment, str):
        yield comment
    for item in records(document):
        for field in TEXT_FIELDS:
            value = item.get(field)
            if isinstance(value, str):
                yield value


def catalogue_text(root):
    """All texts of the shipped catalogue, joined, for the subtraction."""
    parts = []
    for name in CATALOGUE_FILES:
        path = root / name
        if path.is_file():
            parts.extend(texts(json.loads(path.read_text(encoding="utf-8"))))
    return "\n".join(parts)


def markers(root):
    """The markers of the test records, sorted, without catalogue duplicates."""
    shipped = catalogue_text(root)
    found = set()
    for name in TEST_FILES:
        document = json.loads((root / name).read_text(encoding="utf-8"))
        for item in records(document):
            ident = item.get("id")
            if isinstance(ident, str) and ident.startswith("test-"):
                found.add(ident)
        for text in texts(document):
            for run in ASCII_RUN.findall(text):
                run = run.strip()
                if len(run) >= MIN_LENGTH and run not in shipped:
                    found.add(run)
    return sorted(found)


def findings(dist, marks):
    """Pairs (marker, file) for every marker found in the build."""
    out = []
    for path in sorted(dist.rglob("*")):
        if not path.is_file() or path.suffix not in BUILD_SUFFIXES:
            continue
        content = path.read_text(encoding="utf-8", errors="replace")
        for mark in marks:
            if mark in content:
                out.append((mark, path))
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--dist", type=Path, default=None, help="build directory (default ROOT/dist)")
    args = parser.parse_args(argv)
    dist = args.dist if args.dist is not None else args.root / "dist"

    marks = markers(args.root)
    if not marks:
        print("FEHLER: keine Merkmale aus den Pruefdateien gebildet", file=sys.stderr)
        return 2
    if not dist.is_dir() or not any(p.suffix in BUILD_SUFFIXES for p in dist.rglob("*")):
        print(f"FEHLER: kein Bau unter {dist}", file=sys.stderr)
        return 2

    hits = findings(dist, marks)
    for mark, path in hits:
        print(f"FUND {path.relative_to(dist)}: {mark}")
    print(f"{len(hits)} Funde, {len(marks)} Merkmale geprueft in {dist}")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
