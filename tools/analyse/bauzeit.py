#!/usr/bin/env python3
"""Warum baut Worker-1 sechsmal langsamer als Worker-2? Messen, nicht annehmen.

Task #14 aus der Durchsatz-Analyse. Die erste Auswertung hatte nur die MEDIANE
gegenuebergestellt (58,3 s gegen 10,1 s) und die Ursache VERMUTET: "gezielt
--test X gegen breit --all-targets". Eine Vermutung mit einer Zahl daneben sieht
aus wie ein Befund.

Hier wird die Vermutung geprueft: Jeder cargo-remote-Aufruf wird nach seiner
FORM klassifiziert, und die Dauer je Form verglichen. Ist die Vermutung richtig,
muss der Unterschied INNERHALB derselben Form verschwinden.
"""
from __future__ import annotations

import json
import re
import sys
from datetime import datetime
from pathlib import Path

WORKER = {
    "Worker-1": "-work-rota-worktrees-e1",
    "Worker-2": "-work-rota-worktrees-e2",
    "Worker-3": "-work-rota-worktrees-e9",
}
BASIS = Path.home() / ".claude" / "projects"
# Beginn der Sim-Umsetzung, wie in der Durchsatz-Analyse verankert.
ANKER = "2026-08-18T11:33:20Z"


def zeit(s: str) -> datetime | None:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00"))
    except (ValueError, AttributeError):
        return None


def form(befehl: str) -> str:
    """Die FORM des Aufrufs — das ist die vermutete Ursache."""
    if "--all-targets" in befehl:
        return "all-targets"
    if re.search(r"--test\s+\S", befehl):
        return "gezielt --test"
    if "--no-run" in befehl:
        return "nur bauen"
    if " clippy" in befehl:
        return "clippy"
    if " fmt" in befehl:
        return "fmt"
    if " build" in befehl:
        return "build"
    return "sonstiges test"


def sammeln(rolle: str, verz: str) -> list[dict]:
    d = BASIS / verz
    if not d.is_dir():
        return []
    anker = zeit(ANKER)
    # Werkzeugaufruf -> Ergebnis ueber toolUseID verbinden. Ohne die Verbindung
    # gibt es keine Dauer, nur einen Zeitstempel.
    aufrufe: dict[str, dict] = {}
    ergebnis: list[dict] = []

    for p in sorted(d.glob("*.jsonl")):
        with p.open(encoding="utf-8", errors="replace") as f:
            for zeile in f:
                if "cargo" not in zeile and "toolUseResult" not in zeile:
                    continue
                try:
                    e = json.loads(zeile)
                except json.JSONDecodeError:
                    continue
                t = zeit(e.get("timestamp", ""))
                if t is None or (anker and t < anker):
                    continue

                inhalt = e.get("message", {}).get("content")
                if isinstance(inhalt, list):
                    for teil in inhalt:
                        if not isinstance(teil, dict):
                            continue
                        if teil.get("type") == "tool_use" and teil.get("name") == "Bash":
                            befehl = (teil.get("input") or {}).get("command", "")
                            if "cargo remote" in befehl or "cargo-remote" in befehl:
                                aufrufe[teil.get("id", "")] = {"start": t, "befehl": befehl}
                        elif teil.get("type") == "tool_result":
                            a = aufrufe.pop(teil.get("tool_use_id", ""), None)
                            if a:
                                dauer = (t - a["start"]).total_seconds()
                                if 0 < dauer < 3600:
                                    ergebnis.append({
                                        "rolle": rolle,
                                        "dauer": dauer,
                                        "form": form(a["befehl"]),
                                    })
    return ergebnis


def median(xs: list[float]) -> float:
    if not xs:
        return 0.0
    s = sorted(xs)
    n = len(s)
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2


def main() -> int:
    alle: list[dict] = []
    for rolle, verz in WORKER.items():
        teil = sammeln(rolle, verz)
        alle += teil
        print(f"  {rolle}: {len(teil)} cargo-remote-Aufrufe", file=sys.stderr)

    if not alle:
        print("KEINE DATEN — Anker oder Muster pruefen.")
        return 1

    formen = sorted({a["form"] for a in alle})
    rollen = sorted({a["rolle"] for a in alle})

    print("\nDAUER JE AUFRUF, Median in Sekunden (Anzahl in Klammern)\n")
    kopf = "  Form".ljust(20) + "".join(r.ljust(20) for r in rollen)
    print(kopf)
    print("  " + "-" * (len(kopf) - 2))
    for f in formen:
        zeile = f"  {f}".ljust(20)
        for r in rollen:
            xs = [a["dauer"] for a in alle if a["form"] == f and a["rolle"] == r]
            zeile += (f"{median(xs):6.1f} ({len(xs):3d})".ljust(20) if xs
                      else "—".ljust(20))
        print(zeile)

    print("\n  GESAMT".ljust(22), end="")
    for r in rollen:
        xs = [a["dauer"] for a in alle if a["rolle"] == r]
        print(f"{median(xs):6.1f} ({len(xs):3d})".ljust(20), end="")
    print("\n")

    print("ANTEIL DER FORMEN je Rolle (Prozent der Aufrufe)\n")
    for r in rollen:
        gesamt = [a for a in alle if a["rolle"] == r]
        teile = []
        for f in formen:
            n = len([a for a in gesamt if a["form"] == f])
            if n:
                teile.append(f"{f} {100 * n / len(gesamt):.0f}%")
        print(f"  {r:12s} {' · '.join(teile)}")

    print("\nDIE ENTSCHEIDENDE ZAHL — Gesamt-Median gegen Median INNERHALB der Form:")
    print("  Verschwindet der Unterschied innerhalb einer Form, war die Wahl der")
    print("  Form die Ursache. Bleibt er, liegt es an etwas anderem.\n")
    for f in formen:
        werte = {r: median([a["dauer"] for a in alle
                            if a["form"] == f and a["rolle"] == r])
                 for r in rollen}
        echte = {r: v for r, v in werte.items() if v > 0}
        if len(echte) < 2:
            continue
        hoch, tief = max(echte.values()), min(echte.values())
        print(f"  {f:20s} Spanne {tief:6.1f} bis {hoch:6.1f} s   "
              f"Faktor {hoch / tief:.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
