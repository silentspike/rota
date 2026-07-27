#!/usr/bin/env python3
"""Technische Abnahme der erzeugten Klaenge.

Claude kann nicht hoeren. Was sich messen laesst, wird deshalb gemessen:
Laenge, Anfangs-Stille, Spitzenpegel und Lautheit — gegen die Vorgaben aus
`data/sound-manifest.json` und `docs/suno-sfx.md` 7.6.

    tools/sfx-verify.py                      # alles, was im Verzeichnis steht
    tools/sfx-verify.py spell-kindle boss-hit
    tools/sfx-verify.py --dir assets/sfx/e8-2/final --json

Rueckgabewert 0 heisst: jede gepruefte Datei haelt jede Grenze ein.

Grenzen und ihre Begruendung:

| Groesse         | Grenze          | Warum                                                   |
| --------------- | --------------- | ------------------------------------------------------- |
| Laenge          | Ziel +- 0,05 s  | sfx-post.sh schneidet exakt auf die Ziel-Laenge         |
| Anfangs-Stille  | <= 20 ms        | Stille vorn ist hoerbare Verzoegerung auf den Tastendruck |
| Spitzenpegel    | <= -0,5 dBFS    | darueber beginnt hoerbares Uebersteuern                 |
| Lautheit        | siehe unten     | EBU R128, damit kein Klang aus der Reihe faellt         |

**Zur Lautheit kurzer Klaenge.** EBU R128 misst in Bloecken von 400 ms und
verwirft alles unterhalb eines Gates. Fuer einen Klang von 0,25 s — und davon
haben wir mehrere, der Fehleingabe-Ton ist einer — ist die integrierte Lautheit
schlicht **nicht definiert**; ffmpeg meldet dann Werte wie -70 LUFS, die nichts
bedeuten. Deshalb:

| Laenge     | Geprueft wird                    | Toleranz  |
| ---------- | -------------------------------- | --------- |
| >= 3 s     | integrierte Lautheit (LUFS)      | +- 1,5    |
| 0,4 - 3 s  | integrierte Lautheit (LUFS)      | +- 3,0    |
| < 0,4 s    | Effektivwert (RMS) statt LUFS    | -20 +- 6 dBFS |

Dieselbe Ersatzmessung greift, wenn ffmpeg trotz ausreichender Laenge -70 LUFS
meldet: Bei knapp ueber 400 ms bleibt nach dem Gating mitunter kein Block
uebrig, und der Wert ist dann kein Pegel, sondern die Auskunft "nicht messbar".

Der Ein-Pass-`loudnorm` aus `sfx-post.sh` streut bei kurzem Material ohnehin um
ein bis zwei LUFS; eine engere Grenze wuerde nicht Qualitaet messen, sondern
Rauschen.

**Zu transienten Klaengen.** Splittern, Klicken und Schlagen haben einen hohen
Scheitelfaktor: Der Spitzenpegel steht am Anschlag, die Lautheit bleibt trotzdem
niedrig. Weiter anheben ginge nur ueber Kompression — und die nimmt genau das
weg, was den Klang ausmacht. Liegt der Spitzenpegel ueber -3 dBFS, gilt die
Aussteuerung deshalb als ausgereizt; die Zeile bekommt einen Stern statt einer
Beanstandung.
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "data" / "sound-manifest.json"

LENGTH_TOLERANCE_S = 0.05
MAX_LEAD_SILENCE_S = 0.020
MAX_PEAK_DBFS = -0.5
TARGET_LUFS = -16.0
SILENCE_FLOOR_DB = -50

# Unterhalb dieser Laenge ist die integrierte Lautheit nach R128 nicht
# aussagekraeftig (Bloecke von 400 ms).
LUFS_MIN_DURATION_S = 0.4
# ffmpeg meldet diesen Wert, wenn nach dem Gating kein Block uebrig bleibt.
LUFS_NOT_MEASURABLE = -60.0
LUFS_SHORT_DURATION_S = 3.0
LUFS_TOLERANCE_LONG = 1.5
LUFS_TOLERANCE_SHORT = 3.0

# Ab diesem Spitzenpegel gilt die Aussteuerung als ausgereizt: Lautheit laesst
# sich dann nur noch durch Kompression steigern.
PEAK_MAXED_DBFS = -3.0

# Ersatzmass fuer sehr kurze Klaenge.
TARGET_RMS_DBFS = -20.0
RMS_TOLERANCE_DB = 6.0


def ffprobe_duration(path: Path) -> float:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=nw=1:nk=1", str(path)],
        capture_output=True, text=True, check=True,
    )
    return float(out.stdout.strip())


def ffmpeg_stderr(path: Path, filt: str) -> str:
    out = subprocess.run(
        ["ffmpeg", "-v", "info", "-i", str(path), "-af", filt, "-f", "null", "-"],
        capture_output=True, text=True,
    )
    return out.stderr


def lead_silence(path: Path) -> float:
    """Stille am Anfang in Sekunden (Schwelle -50 dB)."""
    text = ffmpeg_stderr(path, f"silencedetect=n={SILENCE_FLOOR_DB}dB:d=0.005")
    # Beginnt die Datei mit Stille, meldet silencedetect silence_start bei ~0.
    starts = [float(m) for m in re.findall(r"silence_start: (-?[\d.]+)", text)]
    ends = [float(m) for m in re.findall(r"silence_end: ([\d.]+)", text)]
    if starts and starts[0] <= 0.001 and ends:
        return ends[0]
    return 0.0


def loudness_and_peak(path: Path) -> tuple[float, float]:
    """Integrierte Lautheit (LUFS) und Spitzenpegel (dBFS)."""
    text = ffmpeg_stderr(path, "ebur128=peak=true")
    blocks = re.split(r"Summary:", text)
    tail = blocks[-1]
    lufs = re.search(r"I:\s+(-?[\d.]+) LUFS", tail)
    peak = re.search(r"Peak:\s+(-?[\d.inf]+) dBFS", tail)
    peak_value = -99.0
    if peak and "inf" not in peak.group(1):
        peak_value = float(peak.group(1))
    return (float(lufs.group(1)) if lufs else 0.0), peak_value


def rms_dbfs(path: Path) -> float:
    """Effektivwert ueber die ganze Datei — Ersatzmass fuer sehr kurze Klaenge."""
    text = ffmpeg_stderr(path, "astats=measure_perchannel=none")
    match = re.search(r"RMS level dB:\s*(-?[\d.]+|-?inf)", text)
    if not match or "inf" in match.group(1):
        return -99.0
    return float(match.group(1))


def check(path: Path, target_s: float | None, is_loop: bool) -> dict:
    duration = ffprobe_duration(path)
    silence = lead_silence(path)
    lufs, peak = loudness_and_peak(path)
    rms = rms_dbfs(path)

    problems = []
    if target_s is not None and not is_loop and abs(duration - target_s) > LENGTH_TOLERANCE_S:
        problems.append(f"Laenge {duration:.3f}s weicht von Ziel {target_s:.3f}s ab")
    if silence > MAX_LEAD_SILENCE_S:
        problems.append(f"Anfangs-Stille {silence * 1000:.0f} ms")
    if peak > MAX_PEAK_DBFS:
        problems.append(f"Spitzenpegel {peak:.2f} dBFS")

    # Lautheit: welches Mass anwendbar ist, entscheidet die Laenge — und was
    # die Messung selbst sagt. Bei knapp ueber 400 ms bleibt vom Gating oft
    # kein einziger Block uebrig; ffmpeg meldet dann -70 LUFS. Dieser Wert ist
    # kein Pegel, sondern die Auskunft "nicht messbar".
    if duration < LUFS_MIN_DURATION_S or lufs <= LUFS_NOT_MEASURABLE:
        loudness_measure = "rms"
        if abs(rms - TARGET_RMS_DBFS) > RMS_TOLERANCE_DB:
            problems.append(f"Effektivwert {rms:.1f} dBFS (LUFS bei {duration:.2f}s nicht messbar)")
    else:
        loudness_measure = "lufs"
        tolerance = (
            LUFS_TOLERANCE_LONG if duration >= LUFS_SHORT_DURATION_S else LUFS_TOLERANCE_SHORT
        )
        # Ein sehr transienter Klang — Splittern, Klicken, Schlagen — hat einen
        # hohen Scheitelfaktor: Der Spitzenpegel steht am Anschlag, waehrend die
        # Lautheit niedrig bleibt. Weiter anheben ginge nur mit Kompression, und
        # die nimmt genau das weg, was den Klang ausmacht. Steht der Peak nahe
        # der Vollaussteuerung, ist die Lautheit ausgereizt und kein Mangel.
        transient = peak >= PEAK_MAXED_DBFS
        if abs(lufs - TARGET_LUFS) > tolerance and not (transient and lufs < TARGET_LUFS):
            problems.append(f"Lautheit {lufs:.1f} LUFS (Ziel {TARGET_LUFS} +- {tolerance})")
        elif transient and lufs < TARGET_LUFS - tolerance:
            loudness_measure = "lufs-transient"

    return {
        "file": str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path),
        "durationS": round(duration, 3),
        "targetS": target_s,
        "leadSilenceMs": round(silence * 1000, 1),
        "peakDbfs": round(peak, 2),
        "lufs": round(lufs, 1),
        "rmsDbfs": round(rms, 1),
        "loudnessMeasure": loudness_measure,
        "problems": problems,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ids", nargs="*", help="Kennungen aus dem Verzeichnis")
    parser.add_argument("--dir", help="Ordner statt assets/ (Dateien heissen wie die Kennung)")
    parser.add_argument("--json", action="store_true", help="Ergebnis als JSON")
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text())
    entries = manifest["sounds"]
    if args.ids:
        wanted = set(args.ids)
        entries = [entry for entry in entries if entry["id"] in wanted]
        unknown = wanted - {entry["id"] for entry in entries}
        if unknown:
            sys.exit(f"Nicht im Verzeichnis: {', '.join(sorted(unknown))}")

    results = []
    missing = []
    for entry in entries:
        parts: list[tuple[Path, float | None, bool]] = []
        if entry["kind"] == "oneshot":
            path = (
                Path(args.dir) / f"{entry['id']}.ogg" if args.dir else ROOT / "assets" / entry["file"]
            )
            parts.append((path, entry.get("targetSeconds"), False))
        else:
            for suffix, key, loop in (
                ("attack", "attackFile", False),
                ("body", "bodyFile", True),
                ("release", "releaseFile", False),
            ):
                if not entry.get(key):
                    continue
                path = (
                    Path(args.dir) / f"{entry['id']}-{suffix}.ogg"
                    if args.dir
                    else ROOT / "assets" / entry[key]
                )
                parts.append((path, entry.get(f"{suffix}Seconds"), loop))

        for path, target, loop in parts:
            if not path.exists():
                missing.append(str(path))
                continue
            results.append({"id": entry["id"], **check(path, target, loop)})

    if args.json:
        print(json.dumps({"checked": results, "missing": missing}, indent=2))
    else:
        print(
            f"{'Kennung':28s} {'Laenge':>8s} {'Ziel':>6s} {'Stille':>8s} {'Peak':>8s} "
            f"{'Lautheit':>16s}  Befund"
        )
        for r in results:
            target = f"{r['targetS']:.2f}" if r["targetS"] else "  —"
            # Bei sehr kurzen Klaengen ist LUFS bedeutungslos; dann steht hier
            # der Effektivwert, nach dem auch geurteilt wurde.
            if r["loudnessMeasure"] == "lufs":
                loudness = f"{r['lufs']:.1f} LUFS"
            elif r["loudnessMeasure"] == "lufs-transient":
                loudness = f"{r['lufs']:.1f} LUFS*"
            else:
                loudness = f"{r['rmsDbfs']:.1f} dBFS RMS"
            verdict = "in Ordnung" if not r["problems"] else "; ".join(r["problems"])
            print(
                f"{r['id']:28s} {r['durationS']:8.3f} {target:>6s} "
                f"{r['leadSilenceMs']:7.1f}m {r['peakDbfs']:8.2f} {loudness:>16s}  {verdict}"
            )
        if missing:
            print(f"\nNoch nicht erzeugt ({len(missing)}):")
            for path in missing:
                print(f"  {path}")

    failed = [r for r in results if r["problems"]]
    if failed:
        sys.exit(f"\n{len(failed)} von {len(results)} Dateien halten die Grenzen nicht ein.")
    if not results:
        sys.exit("Keine Datei geprueft.")


if __name__ == "__main__":
    main()
