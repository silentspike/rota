#!/usr/bin/env bash
# sfx-post.sh — Nachbearbeitung der Suno-Rohdateien fuer den Spiel-Einsatz.
#
# Suno liefert ~1,8 s Container mit viel Stille (gemessen: bis zu 60 % Stille-Anteil).
# Fuer latenzfreies Abspielen auf Tastendruck MUSS die Stille am Anfang weg.
#
# Usage: sfx-post.sh <in-dir> <out-dir> [variante]
#   variante: a (default) oder b — welcher der zwei Suno-Entwuerfe uebernommen wird
#
# Macht pro Datei:
#   1. Stille am Anfang und Ende entfernen (-50 dB Schwelle)
#   2. Lautheit normalisieren (EBU R128, -16 LUFS — Spiel-Standard)
#   3. Als OGG Vorbis speichern (kleiner als MP3, im Browser latenzaermer dekodierbar)
set -eu

IN="${1:?Usage: sfx-post.sh <in-dir> <out-dir> [a|b]}"
OUT="${2:?out-dir fehlt}"
VAR="${3:-a}"
mkdir -p "$OUT"

n=0
for f in "$IN"/*-"$VAR".mp3; do
  [ -e "$f" ] || continue
  base=$(basename "$f"); name="${base%-$VAR.mp3}"
  target="$OUT/$name.ogg"
  ffmpeg -v error -y -i "$f" \
    -af "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0:stop_periods=-1:stop_threshold=-50dB:stop_duration=0.05,loudnorm=I=-16:TP=-1.5:LRA=11" \
    -c:a libvorbis -q:a 5 "$target"
  before=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")
  after=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$target")
  printf "  %-24s %5.2fs -> %5.2fs\n" "$name.ogg" "$before" "$after"
  n=$((n + 1))
done
echo "FERTIG: $n Dateien in $OUT"
