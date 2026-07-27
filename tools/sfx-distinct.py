#!/usr/bin/env python3
"""Prueft, ob sich Klaenge einer Gruppe deutlich voneinander unterscheiden.

Gedacht fuer Gruppen, in denen Verwechslung teuer waere — allen voran die vier
Beute-Klaenge: Der Spieler soll an einem Klang erkennen, was gefallen ist,
bevor er den Gegenstand sieht (GAME-DESIGN 13.6, 17.1).

    tools/sfx-distinct.py loot-common loot-rare loot-epic loot-legendary

Verglichen werden drei Groessen, die zusammen beschreiben, wie ein Klang wirkt:

| Groesse                | Was sie erfasst                                    |
| ---------------------- | -------------------------------------------------- |
| Laenge                 | wie lange der Klang steht                          |
| Spektraler Schwerpunkt | wie hell er klingt                                 |
| Bandverteilung         | wo die Energie liegt (Tiefen, Mitten, Hoehen, Luft) |

Zwei Klaenge gelten als unterscheidbar, wenn sie sich in **mindestens einer**
Groesse deutlich trennen: Laenge um Faktor 1,5, Schwerpunkt um 25 % oder
Bandverteilung um 0,15 (Summe der Betragsdifferenzen ueber vier Baender).

Der Massstab ersetzt kein Ohr. Er faengt den Fall ab, dass zwei Klaenge
rechnerisch fast dasselbe sind — was beim Erzeugen aus aehnlichen Prompts
regelmaessig passiert.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "data" / "sound-manifest.json"

SAMPLE_RATE = 48000
BANDS = [(0, 250), (250, 2000), (2000, 6000), (6000, 24000)]
BAND_NAMES = ["tief", "mitte", "hoch", "luft"]

LENGTH_FACTOR = 1.5
CENTROID_RATIO = 0.25
BAND_DISTANCE = 0.15


def load_samples(path: Path) -> np.ndarray:
    """Datei als Mono-Float lesen."""
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le", "-ac", "1",
         "-ar", str(SAMPLE_RATE), "-"],
        capture_output=True, check=True,
    ).stdout
    return np.frombuffer(raw, dtype=np.float32)


def features(path: Path) -> dict:
    samples = load_samples(path)
    if samples.size == 0:
        raise SystemExit(f"{path} ist leer")

    spectrum = np.abs(np.fft.rfft(samples * np.hanning(samples.size)))
    freqs = np.fft.rfftfreq(samples.size, 1 / SAMPLE_RATE)
    total = spectrum.sum() or 1.0

    centroid = float((freqs * spectrum).sum() / total)
    bands = []
    for low, high in BANDS:
        mask = (freqs >= low) & (freqs < high)
        bands.append(float(spectrum[mask].sum() / total))

    return {
        "seconds": samples.size / SAMPLE_RATE,
        "centroidHz": centroid,
        "bands": bands,
    }


def compare(a: dict, b: dict) -> tuple[bool, str]:
    longer, shorter = max(a["seconds"], b["seconds"]), min(a["seconds"], b["seconds"])
    length_factor = longer / shorter if shorter > 0 else float("inf")

    hi, lo = max(a["centroidHz"], b["centroidHz"]), min(a["centroidHz"], b["centroidHz"])
    centroid_ratio = (hi - lo) / hi if hi > 0 else 0.0

    band_distance = sum(abs(x - y) for x, y in zip(a["bands"], b["bands"]))

    reasons = []
    if length_factor >= LENGTH_FACTOR:
        reasons.append(f"Laenge x{length_factor:.2f}")
    if centroid_ratio >= CENTROID_RATIO:
        reasons.append(f"Schwerpunkt {centroid_ratio * 100:.0f} %")
    if band_distance >= BAND_DISTANCE:
        reasons.append(f"Baender {band_distance:.2f}")

    if reasons:
        return True, ", ".join(reasons)
    return False, (
        f"Laenge x{length_factor:.2f}, Schwerpunkt {centroid_ratio * 100:.0f} %, "
        f"Baender {band_distance:.2f} — alles unter der Schwelle"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ids", nargs="+", help="Kennungen aus dem Verzeichnis")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text())
    by_id = {entry["id"]: entry for entry in manifest["sounds"]}

    data = {}
    for sound_id in args.ids:
        entry = by_id.get(sound_id)
        if not entry:
            sys.exit(f"Nicht im Verzeichnis: {sound_id}")
        path = ROOT / "assets" / entry["file"]
        if not path.exists():
            sys.exit(f"Noch nicht erzeugt: {path}")
        data[sound_id] = features(path)

    if not args.json:
        print(f"{'Kennung':24s} {'Laenge':>8s} {'Schwerpunkt':>12s}   " + "  ".join(f"{n:>6s}" for n in BAND_NAMES))
        for sound_id, f in data.items():
            bands = "  ".join(f"{b:6.3f}" for b in f["bands"])
            print(f"{sound_id:24s} {f['seconds']:7.3f}s {f['centroidHz']:11.0f} Hz   {bands}")
        print()

    problems = []
    pairs = []
    ids = list(data)
    for i, first in enumerate(ids):
        for second in ids[i + 1:]:
            ok, why = compare(data[first], data[second])
            pairs.append({"a": first, "b": second, "distinct": ok, "why": why})
            if not args.json:
                mark = "unterscheidbar" if ok else "ZU AEHNLICH"
                print(f"  {first:22s} / {second:22s}  {mark}: {why}")
            if not ok:
                problems.append(f"{first} / {second}")

    if args.json:
        print(json.dumps({"features": data, "pairs": pairs}, indent=2))

    if problems:
        sys.exit(f"\nZu aehnlich: {', '.join(problems)}")


if __name__ == "__main__":
    main()
