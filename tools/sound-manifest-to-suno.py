#!/usr/bin/env python3
"""Aus dem Klang-Verzeichnis ein Auftrags-Manifest fuer suno_api.py bauen.

`data/sound-manifest.json` ist die eine Wahrheit (Zweck, Ziel-Laenge, Prompt).
Dieses Werkzeug uebersetzt daraus das schlanke Format, das `suno_api.py batch`
erwartet — damit Prompts nicht zweimal gepflegt werden und die Erzeugung
nachvollziehbar bleibt.

    tools/sound-manifest-to-suno.py --out batch.json spell-kindle spell-pyre
    tools/sound-manifest-to-suno.py --out batch.json --group spells
    tools/sound-manifest-to-suno.py --list

Dreiteilige Klaenge werden in drei Auftraege zerlegt: Ansatz und Ausklang als
One-Shot mit Ziel-Laenge, der Koerper im Loop-Modus (dort steuert das Spiel die
Laenge durch Wiederholen, deshalb ohne Ziel-Laenge und ohne Beschnitt).
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "data" / "sound-manifest.json"


def jobs_for(entry: dict) -> list[dict]:
    """Ein Verzeichnis-Eintrag ergibt einen oder drei Auftraege."""
    if entry["kind"] == "oneshot":
        job = {"name": entry["id"], "prompt": entry["prompt"]}
        if entry.get("targetSeconds"):
            job["target_s"] = entry["targetSeconds"]
        return [job]

    if entry["kind"] == "sustained":
        # Reiner Loop ohne Ansatz und Ausklang — etwa eine Musik-Schicht. Dann
        # traegt die Datei den Namen der Kennung, ohne "-body".
        if not entry.get("attackFile") and not entry.get("releaseFile"):
            return [{"name": entry["id"], "prompt": entry["bodyPrompt"], "loop": True}]

        out = []
        if entry.get("attackFile"):
            out.append(
                {
                    "name": f"{entry['id']}-attack",
                    "prompt": entry["attackPrompt"],
                    "target_s": entry.get("attackSeconds", 0.5),
                }
            )
        out.append({"name": f"{entry['id']}-body", "prompt": entry["bodyPrompt"], "loop": True})
        if entry.get("releaseFile"):
            out.append(
                {
                    "name": f"{entry['id']}-release",
                    "prompt": entry["releasePrompt"],
                    "target_s": entry.get("releaseSeconds", 0.6),
                }
            )
        return out

    sys.exit(f"Unbekannte Art '{entry['kind']}' bei '{entry['id']}'")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ids", nargs="*", help="Kennungen aus dem Verzeichnis")
    parser.add_argument("--group", help="ganze Gruppe statt einzelner Kennungen")
    parser.add_argument("--out", help="Zieldatei (ohne Angabe: Ausgabe auf stdout)")
    parser.add_argument("--list", action="store_true", help="nur auflisten, was es gibt")
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text())
    sounds = manifest["sounds"]

    if args.list:
        for entry in sounds:
            print(f"{entry['id']:32s} {entry['group']:10s} {entry['kind']}")
        print(f"\n{len(sounds)} Eintraege in {MANIFEST}")
        return

    if args.group:
        chosen = [entry for entry in sounds if entry["group"] == args.group]
    elif args.ids:
        by_id = {entry["id"]: entry for entry in sounds}
        unknown = [i for i in args.ids if i not in by_id]
        if unknown:
            sys.exit(f"Nicht im Verzeichnis: {', '.join(unknown)}")
        chosen = [by_id[i] for i in args.ids]
    else:
        sys.exit("Weder Kennungen noch --group angegeben.")

    jobs = [job for entry in chosen for job in jobs_for(entry)]
    text = json.dumps(jobs, ensure_ascii=False, indent=2) + "\n"

    if args.out:
        Path(args.out).write_text(text)
        print(f"{len(jobs)} Auftraege aus {len(chosen)} Eintraegen -> {args.out}")
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
