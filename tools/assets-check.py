#!/usr/bin/env python3
"""Prueft, ob ASSETS.md vollstaendig ist und keine unerlaubte Lizenz enthaelt.

ASSETS.md sagt: "Jede Datei unter assets/, docs/refs/ und docs/style-candidates/
steht hier drin." Dieses Skript prueft genau das - und zwar mechanisch, damit
die Zusage nicht nur eine Absichtserklaerung ist.

Zwei Pruefungen:

  1. ABDECKUNG  Jede Datei unter den drei Wurzeln muss von einem Eintrag in
                ASSETS.md erfasst sein. Als Eintrag zaehlt jeder Pfad oder
                jedes Muster, das im Dokument in `Backticks` steht.

  2. LIZENZ     Im Bestandsteil von ASSETS.md und in den mitgelieferten
                Original-Lizenztexten darf keine der ausgeschlossenen
                Lizenzformen vorkommen ("personal use", "non-commercial").

Aufruf:

    python3 tools/assets-check.py               # alle Wurzeln
    python3 tools/assets-check.py docs/style-candidates   # nur eine Wurzel

Rueckgabewert 0, wenn alles sauber ist, sonst 1.
"""

from __future__ import annotations

import fnmatch
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS_MD = os.path.join(REPO, "ASSETS.md")

# Wurzeln, fuer die ASSETS.md Vollstaendigkeit zusagt.
ROOTS = ["assets", "docs/refs", "docs/style-candidates"]

# Dateien, die keine Assets sind und deshalb keinen Herkunftseintrag brauchen.
SKIP_NAMES = {".gitkeep", ".gitignore", ".DS_Store"}

# Ausgeschlossene Lizenzformen (ASSETS.md, Abschnitt "Lizenzregel").
FORBIDDEN = [
    ("personal use", "„Free for personal use\" ist ausgeschlossen"),
    ("non-commercial", "„non-commercial\" ist ausgeschlossen"),
    ("noncommercial", "„noncommercial\" ist ausgeschlossen"),
]


def read_assets_md() -> str:
    with open(ASSETS_MD, encoding="utf-8") as fh:
        return fh.read()


def coverage_patterns(md: str) -> list[str]:
    """Alle Pfade und Muster aus den Backticks des BESTANDS-Teils.

    Nur ab "## Bestand" - der Kopf des Dokuments nennt die Wurzeln selbst in
    Backticks (`assets/`), und die wuerden sonst pauschal alles abdecken.
    """
    idx = md.find("## Bestand")
    body = md[idx:] if idx >= 0 else md
    pats = []
    for token in re.findall(r"`([^`\n]+)`", body):
        token = token.strip()
        if not token or " " in token and "*" not in token and "/" not in token:
            continue
        # Kommagetrennte Aufzaehlungen in einer Zelle aufloesen.
        for part in re.split(r",\s*", token):
            part = part.strip().strip("`")
            if part and is_file_entry(part):
                pats.append(part)
    return pats


HAS_EXT = re.compile(r"\.[A-Za-z0-9]{2,5}$")


def is_file_entry(pat: str) -> bool:
    """Nur Eintraege, die eine DATEI meinen, zaehlen als Abdeckung.

    Reine Verzeichnisnamen wie `assets/` oder `assets/img/` kommen im Fliesstext
    vor und wuerden sonst pauschal alles darunter abdecken - die Pruefung waere
    wertlos. Ein Eintrag zaehlt, wenn er ein Platzhalterzeichen enthaelt oder
    auf eine Dateiendung endet.
    """
    if any(ch in pat for ch in "*?"):
        return True
    return bool(HAS_EXT.search(pat))


def covered(rel_in_root: str, rel_in_repo: str, pats: list[str]) -> bool:
    """Ein Eintrag deckt eine Datei ab, wenn der Pfad relativ zur Wurzel oder
    relativ zum Repo auf das Muster passt.

    Bewusst KEIN Rueckfall auf den blossen Dateinamen: sonst deckt ein Eintrag
    `manifest.json` jede gleichnamige Datei irgendwo im Baum ab, und die
    Pruefung meldet Vollstaendigkeit, wo keine ist.
    """
    candidates = {rel_in_root, rel_in_repo}
    for pat in pats:
        p = pat.rstrip("/")
        wild = any(ch in p for ch in "*?")
        for cand in candidates:
            # Ohne Platzhalter exakt vergleichen - sonst deutet fnmatch die
            # eckigen Klammern in Namen wie "Cinzel[wght].ttf" als Zeichenklasse.
            if wild:
                if fnmatch.fnmatch(cand, p):
                    return True
            elif cand == p:
                return True
    return False


def check_coverage(roots: list[str], pats: list[str]) -> dict[str, list[str]]:
    missing: dict[str, list[str]] = {}
    for root in roots:
        abs_root = os.path.join(REPO, root)
        if not os.path.isdir(abs_root):
            continue
        for dirpath, dirnames, filenames in os.walk(abs_root):
            dirnames[:] = [d for d in dirnames if d != "__MACOSX"]
            for fn in sorted(filenames):
                if fn in SKIP_NAMES:
                    continue
                abs_path = os.path.join(dirpath, fn)
                rel_repo = os.path.relpath(abs_path, REPO)
                rel_root = os.path.relpath(abs_path, abs_root)
                if not covered(rel_root, rel_repo, pats):
                    missing.setdefault(root, []).append(rel_repo)
    return missing


def check_licenses(md: str, roots: list[str]) -> list[str]:
    problems = []

    # Nur der Bestandsteil - die Lizenzregel oben nennt die verbotenen Formen
    # notwendigerweise selbst.
    idx = md.find("## Bestand")
    body = md[idx:] if idx >= 0 else md
    for needle, why in FORBIDDEN:
        for line in body.splitlines():
            low = line.lower()
            if needle in low and "ausgeschlossen" not in low and "verworfen" not in low:
                problems.append(f"ASSETS.md (Bestand): {why} - Zeile: {line.strip()[:110]}")

    # Die mitgelieferten Original-Lizenztexte gegenpruefen.
    for root in roots:
        abs_root = os.path.join(REPO, root)
        for dirpath, _dirnames, filenames in os.walk(abs_root):
            for fn in filenames:
                if not fn.lower().endswith(".txt"):
                    continue
                path = os.path.join(dirpath, fn)
                try:
                    with open(path, encoding="utf-8", errors="replace") as fh:
                        text = fh.read().lower()
                except OSError:
                    continue
                for needle, why in FORBIDDEN:
                    if needle in text:
                        rel = os.path.relpath(path, REPO)
                        problems.append(f"{rel}: enthaelt \"{needle}\" - {why}")
    return problems


def main(argv: list[str]) -> int:
    roots = argv[1:] or ROOTS
    md = read_assets_md()
    pats = coverage_patterns(md)

    print(f"ASSETS.md: {len(pats)} Eintragsmuster gefunden")
    print(f"Geprueft: {', '.join(roots)}\n")

    missing = check_coverage(roots, pats)
    lic = check_licenses(md, roots)

    total_files = 0
    for root in roots:
        abs_root = os.path.join(REPO, root)
        if os.path.isdir(abs_root):
            total_files += sum(
                len([f for f in fs if f not in SKIP_NAMES])
                for _, ds, fs in os.walk(abs_root)
                if not any(part == "__MACOSX" for part in _.split(os.sep))
            )

    if missing:
        print("FEHLT IN ASSETS.md")
        for root, files in missing.items():
            print(f"  {root}/ - {len(files)} Datei(en) ohne Eintrag:")
            for f in files[:40]:
                print(f"    {f}")
            if len(files) > 40:
                print(f"    ... und {len(files) - 40} weitere")
        print()
    else:
        print(f"Abdeckung: alle {total_files} Dateien haben einen Eintrag")

    if lic:
        print("LIZENZPROBLEME")
        for p in lic:
            print(f"  {p}")
        print()
    else:
        print("Lizenzen: keine ausgeschlossene Lizenzform gefunden")

    return 1 if (missing or lic) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
