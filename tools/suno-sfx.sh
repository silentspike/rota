#!/usr/bin/env bash
# suno-sfx.sh — Sound-Generierung via Suno Sounds (UI-getrieben, ohne Koordinaten/Menüs)
#
# Voraussetzung: laufende playwright-cli Session "suno", eingeloggt auf suno.com/create,
#                Sounds-Tab aktiv (Dropdown "Sounds" neben Simple/Advanced).
#
# Usage: suno-sfx.sh "<prompt>" <basename> [outdir]
#   Erzeugt EINE Generierung (= 2 Entwürfe), lädt BEIDE als mp3:
#   <outdir>/<basename>-a.mp3 und <basename>-b.mp3
#
# Ablauf: Snapshot-Diff der /song/{id}-Links im Workspace → CDN-Direktdownload.
# Kein pipefail: head/grep-Pipes erzeugen sonst SIGPIPE-Abbrüche (Exit 141).
set -eu

SESSION=suno
PW_DIR=/work/.playwright-cli
PROMPT="${1:?Usage: suno-sfx.sh \"<prompt>\" <basename> [outdir]}"
NAME="${2:?basename fehlt}"
OUTDIR="${3:-/work/rota/assets/sfx/raw}"
mkdir -p "$OUTDIR"

latest_yml() {
  # neueste Snapshot-Datei ohne SIGPIPE-anfällige ls|head-Pipe
  local f
  f=$(cd "$PW_DIR" && ls -t page-*.yml 2>/dev/null | sed -n '1p')
  echo "$PW_DIR/$f"
}

snap_ids() {
  playwright-cli -s=$SESSION snapshot >/dev/null 2>&1 || true
  grep -oE '/song/[0-9a-f-]{36}' "$(latest_yml)" 2>/dev/null | sed 's|/song/||' | awk '!seen[$0]++' || true
}

echo "[1/4] Workspace-Stand erfassen..."
before=$(snap_ids)

# Refs IMMER frisch aus dem aktuellen Snapshot ziehen (Refs/Attribute können sich ändern)
YML=$(latest_yml)
TB=$(grep -oP 'textbox "Describe the sound you want".*?\[ref=\K[^\]]+' "$YML" | sed -n '1p')
CB=$(grep -oP 'button "Create song".*?\[ref=\K[^\]]+' "$YML" | sed -n '1p')
if [ -z "${TB:-}" ] || [ -z "${CB:-}" ]; then
  echo "FEHLER: Textbox/Create-Button nicht gefunden — ist suno.com/create mit Sounds-Tab offen?"
  exit 1
fi

echo "[2/4] Prompt eintragen und Create klicken..."
playwright-cli -s=$SESSION fill "$TB" "$PROMPT" >/dev/null 2>&1
playwright-cli -s=$SESSION click "$CB" >/dev/null 2>&1

echo "[3/4] Auf 2 neue Entwürfe warten..."
new=""
for i in $(seq 1 36); do
  sleep 5
  after=$(snap_ids)
  new=$(comm -13 <(echo "$before" | sort) <(echo "$after" | sort))
  n=$(echo "$new" | grep -c . || true)
  if [ "$n" -ge 2 ]; then break; fi
  echo "  ... noch $n/2 (Versuch $i/36)"
done
n=$(echo "$new" | grep -c . || true)
if [ "$n" -lt 2 ]; then
  echo "FEHLER: Nach 3 min keine 2 neuen Entwürfe. Captcha/Kredit-Problem? Screenshot: playwright-cli -s=suno screenshot"
  exit 1
fi

echo "[4/4] Download via CDN..."
suffix=a
fail=0
for id in $(echo "$new" | sed -n '1,2p'); do
  out="$OUTDIR/$NAME-$suffix.mp3"
  ok=0
  for j in $(seq 1 12); do
    code=$(curl -sL -o "$out" -w "%{http_code}" "https://cdn1.suno.ai/$id.mp3")
    if [ "$code" = "200" ]; then ok=1; break; fi
    sleep 10
  done
  if [ "$ok" != "1" ]; then
    echo "FEHLER: Download $id fehlgeschlagen (letzter HTTP-Code: $code)"
    fail=1
  else
    dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$out")
    echo "  $out  (id=$id, ${dur}s)"
  fi
  suffix=b
done
if [ "$fail" = "1" ]; then exit 1; fi
echo "FERTIG: $NAME"
