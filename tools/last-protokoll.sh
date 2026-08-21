#!/usr/bin/env bash
# Schreibt während einer langen Messung mit, welche FREMDE Last lief.
#
# Warum das ein eigenes Werkzeug ist und nicht `rota-fenster status` genügt:
#
#   1. Der Fensterschalter kennt nur Rust-Muster (target/debug/deps, rota-sim,
#      cargo-remote-fmt). Am 2026-08-22 liefen vier vitest-Prozesse aus einem
#      fremden Worktree mit über 600 % CPU durch eine Messung, und er meldete
#      "0 fremde Prozesse" — nicht falsch abgelesen, sondern blind für die
#      halbe Werkzeugkette.
#
#   2. Ein Blick zu Beginn und am Ende sagt nichts über die vier Stunden
#      dazwischen. Fremde Last, die um 02:00 anfängt und um 03:00 aufhört,
#      ist in beiden Ablesungen unsichtbar.
#
# Aufruf:  bash tools/last-protokoll.sh <ausgabedatei> [sekunden]
#          (im Hintergrund starten, am Ende mit kill beenden)

set -uo pipefail

ZIEL=${1:?Ausgabedatei angeben}
TAKT=${2:-30}
MEIN_BAUM=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# Die eigene Prozesskette. Ohne sie zählt das Werkzeug SICH SELBST als Last:
# Ein pgrep-Aufruf steht in seiner eigenen Ausgabe, und er hat denselben cwd
# wie die Messung, die er beobachtet. Gemessen am 2026-08-22 — die Prüfung
# meldete einen laufenden Prozess, den `ps -p` nicht mehr kannte, weil er der
# Fragende selbst war.
EIGEN="$$"
p=$PPID
while [ "$p" -gt 1 ]; do
  EIGEN="$EIGEN $p"
  p=$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ')
  [ -z "$p" ] && break
done

ist_eigen() {
  for e in $EIGEN; do [ "$1" = "$e" ] && return 0; done
  return 1
}

{
  echo "# Lastprotokoll, Takt ${TAKT}s, mein Baum: $MEIN_BAUM"
  echo "# Zeit | load1 | fremde Prozesse (pid cpu% cwd cmd)"
} > "$ZIEL"

while true; do
  jetzt=$(date '+%H:%M:%S')
  load=$(awk '{print $1}' /proc/loadavg)
  zeilen=""
  # Breit suchen, nicht nach einer Musterliste: node, npx und vitest fehlten
  # der bestehenden Liste, und was dort fehlt, wird nie gemeldet.
  while read -r pid cpu cmd; do
    [ -z "${pid:-}" ] && continue
    ist_eigen "$pid" && continue
    cwd=$(readlink "/proc/$pid/cwd" 2>/dev/null) || continue
    case "$cwd" in
      "$MEIN_BAUM"*) continue ;;   # meine eigene Messung, keine Fremdlast
    esac
    # Unter 20 % ist Rauschen (Editoren, Sprachserver, Shell-Prompts).
    awk -v c="$cpu" 'BEGIN{exit !(c+0 >= 20)}' || continue
    zeilen="${zeilen}${pid} ${cpu}% ${cwd} ${cmd:0:40}; "
  done < <(ps -eo pid=,pcpu=,comm= --sort=-pcpu 2>/dev/null | head -40)

  if [ -n "$zeilen" ]; then
    echo "$jetzt | $load | $zeilen" >> "$ZIEL"
  else
    echo "$jetzt | $load | -" >> "$ZIEL"
  fi
  sleep "$TAKT"
done
