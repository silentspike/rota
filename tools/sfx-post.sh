#!/usr/bin/env bash
# sfx-post.sh — Nachbearbeitung der Suno-Rohdateien fuer den Spiel-Einsatz.
#
# Suno liefert ~1,8 s Container mit viel Stille (gemessen bis 60 % Stille-Anteil)
# und trifft Ziel-Laengen nie exakt. Diese Stufe stellt beides gerade.
#
# Usage: sfx-post.sh <in-dir> <out-dir> [variante]
#   variante: a (default) oder b — welcher Suno-Entwurf uebernommen wird
#
# Steuerdatei <in-dir>/lengths.json (optional), zwei Schreibweisen:
#   {"boss-kill": 2.5}                          # Ziel-Laenge in Sekunden
#   {"channel-body": {"loop": true}}            # nahtlose Schleife — NICHT beschneiden!
#
# ONE-SHOTS (default):
#   1. Stille am Anfang und Ende entfernen (-50 dB) — kritisch fuer latenzfreien Tastendruck
#   2. Falls Ziel-Laenge gesetzt UND Datei laenger: auf Ziel kuerzen mit weicher Ausblendung
#      (exponentiell, max. 0,25 s) — kein harter Schnitt, klingt wie natuerlicher Abklang.
#      Nur KUERZEN, nie strecken.
#   3. Lautheit normalisieren (EBU R128, -16 LUFS)
#
# LOOPS ("loop": true):
#   NUR Lautheit normalisieren. Kein Trimmen, kein Fade — jeder Eingriff an den Raendern
#   zerstoert die Nahtstelle, die den Loop nahtlos macht. Laenge steuert das Spiel selbst,
#   indem es die Schleife so oft wiederholt, wie der Effekt dauert (10 s Kanalisierung,
#   Buff-Fenster, DoT-Brennen ...).
#
#   4. Ausgabe als OGG Vorbis (kleiner als MP3, im Browser latenzaermer dekodierbar)
set -eu

IN="${1:?Usage: sfx-post.sh <in-dir> <out-dir> [a|b]}"
OUT="${2:?out-dir fehlt}"
VAR="${3:-a}"
LEN_FILE="$IN/lengths.json"

# Zielwerte fuer Klaenge unter 400 ms, wo EBU R128 nicht greift (siehe unten).
SHORT_TARGET_RMS=-20.0
SHORT_PEAK_CEIL=-1.5
mkdir -p "$OUT"

# Gibt "<ziel-sekunden>|<loop 0/1>" zurueck, leer wenn kein Eintrag
config_for() {
  [ -f "$LEN_FILE" ] || return 0
  python3 -c "
import json, sys
try: d = json.load(open('$LEN_FILE'))
except Exception: sys.exit()
v = d.get('$1')
if v is None: sys.exit()
if isinstance(v, dict): print(f\"{v.get('target_s','')}|{1 if v.get('loop') else 0}\")
else: print(f'{v}|0')
" 2>/dev/null || true
}

n=0
for f in "$IN"/*-"$VAR".mp3; do
  [ -e "$f" ] || continue
  base=$(basename "$f"); name="${base%-$VAR.mp3}"
  cfg=$(config_for "$name")
  target="${cfg%%|*}"; is_loop="${cfg##*|}"
  [ "$cfg" = "" ] && { target=""; is_loop=0; }

  before=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")
  target_file="$OUT/$name.ogg"

  if [ "$is_loop" = "1" ]; then
    # Schleife: Raender unangetastet lassen, nur Pegel angleichen
    ffmpeg -v error -y -i "$f" -af "loudnorm=I=-16:TP=-1.5:LRA=11" -c:a libvorbis -q:a 5 "$target_file"
    note=" [Loop — Raender unangetastet]"
  else
    trimmed="$OUT/.$name.trim.wav"
    ffmpeg -v error -y -i "$f" -af \
      "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0:stop_periods=-1:stop_threshold=-50dB:stop_duration=0.05" \
      "$trimmed"
    mid=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$trimmed")
    filter="loudnorm=I=-16:TP=-1.5:LRA=11"
    note=""
    if [ -n "$target" ] && awk "BEGIN{exit !($mid > $target + 0.05)}"; then
      fade=$(awk "BEGIN{f=$target*0.2; if(f>0.25) f=0.25; if(f<0.05) f=0.05; printf \"%.3f\", f}")
      fstart=$(awk "BEGIN{printf \"%.3f\", $target - $fade}")
      filter="afade=t=out:st=$fstart:d=$fade:curve=exp,atrim=0:$target,$filter"
      note=" [gekuerzt auf ${target}s, ${fade}s Ausblendung]"
    fi
    # Kurze Klaenge: RMS statt loudnorm.
    #
    # EBU R128 misst in Bloecken von 400 ms. Bei kuerzerem Material hat
    # loudnorm nichts, woran es sich orientieren kann, und hebt wild an —
    # gemessen ueber elf kurze Klaenge eine Streuung von 14 dB (-11,4 bis
    # -25,7 dBFS RMS). Nebeneinander gehoert ist das ein Durcheinander.
    # Deshalb hier ein fester Zielwert ueber den Effektivwert, begrenzt durch
    # die Spitzenreserve. Kein Kompressor: Der Klang behaelt seine Form.
    out_len="$target"
    [ -z "$out_len" ] && out_len="$mid"
    if awk "BEGIN{exit !($out_len < 0.4)}"; then
      stats=$(ffmpeg -v info -i "$trimmed" -af astats=measure_perchannel=none -f null - 2>&1)
      rms=$(printf '%s\n' "$stats" | grep -oP 'RMS level dB:\s*\K-?[0-9.]+' | head -1)
      peak=$(printf '%s\n' "$stats" | grep -oP 'Peak level dB:\s*\K-?[0-9.]+' | head -1)
      if [ -n "$rms" ] && [ -n "$peak" ]; then
        gain=$(awk "BEGIN{g = $SHORT_TARGET_RMS - ($rms); h = $SHORT_PEAK_CEIL - ($peak); if (g > h) g = h; printf \"%.2f\", g}")
        filter="${filter%%loudnorm*}volume=${gain}dB"
        note="$note [kurz: Pegel ueber RMS, ${gain} dB]"
      fi
    fi
    ffmpeg -v error -y -i "$trimmed" -af "$filter" -c:a libvorbis -q:a 5 "$target_file"
    rm -f "$trimmed"
  fi

  after=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$target_file")
  printf "  %-22s %6.2fs -> %6.2fs%s\n" "$name.ogg" "$before" "$after" "$note"
  n=$((n + 1))
done
echo "FERTIG: $n Dateien in $OUT"
