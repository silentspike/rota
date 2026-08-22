#!/usr/bin/env bash
# Gegenprobe zu tools/fenster-warnung.sh (Issue #1049, Fertig-Kriterium 3).
#
# Prüft ALLE DREI Zustände an einem ECHTEN Schalterstand, in derselben
# Ablesung:
#
#     fremdes Fenster gehalten   ->  warnt
#     eigenes Fenster gehalten   ->  schweigt
#     Fenster frei               ->  schweigt
#
# # Warum alle drei und nicht nur der erste
#
# Ein Melder, der bei jedem Aufruf anschlägt, erfüllt die erste Richtung
# ebenfalls — und ist wertlos. Am 2026-08-22 ist ein Zwei-Leerzeichen-Detektor
# genau daran gescheitert: 1328 Treffer auf 1416 Nachrichten. Die Zahl, die
# zählt, ist deshalb nicht die Trefferzahl, sondern das Verhältnis von
# Anschlagen zu Schweigen.
#
# # Der echte Schalterstand wird NICHT angefasst
#
# Er wird gesichert, ersetzt und im `trap` zurückgeschrieben — auch bei
# Abbruch. Ein Prüfwerkzeug, das den Zustand hinterlässt, den es vorgibt zu
# prüfen, ist schlimmer als keines: Am 2026-08-19 haben zwei Sitzungen
# gegeneinander gemessen, weil ein Fenster fälschlich als frei galt.

set -u

HIER="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WARNUNG="$HIER/fenster-warnung.sh"
STAND="${HOME}/.claude/rota-fenster.json"
SICHERUNG="/work/tmp/e9-1049-fensterstand-sicherung.json"

MEINE_ROLLE="$(rota-ipc wer 2>/dev/null | awk '/^Rolle:/ {print $2; exit}')"
if [ -z "$MEINE_ROLLE" ]; then
  echo "FEHLGESCHLAGEN: eigene Rolle nicht ermittelbar. Ohne sie kann die" >&2
  echo "  Probe 'eigenes Fenster' nicht von 'fremdes Fenster' trennen —" >&2
  echo "  sie wuerde dann zwei Richtungen gleich behandeln und trotzdem" >&2
  echo "  gruen melden." >&2
  exit 1
fi

# --- Sicherung, mit Rueckgabe auch bei Abbruch ---------------------------
if [ -f "$STAND" ]; then
  cp "$STAND" "$SICHERUNG"
  HATTE_STAND=ja
else
  HATTE_STAND=nein
fi

zuruecksetzen() {
  if [ "$HATTE_STAND" = ja ]; then
    cp "$SICHERUNG" "$STAND"
  else
    rm -f "$STAND"
  fi
  rm -f "$SICHERUNG"
}
trap zuruecksetzen EXIT INT TERM

setze_stand() {
  python3 - "$STAND" "$1" "$2" <<'PYTHON'
import json, sys
pfad, inhaber, abgabe = sys.argv[1], sys.argv[2], sys.argv[3]
json.dump({
    "inhaber": inhaber or None,
    "zweck": "Gegenprobe zu #1049 — kein echter Lauf",
    "seit": "22.08. 00:00:00",
    "abgabe": abgabe or None,
    "kennung": "session-gegenprobe",
}, open(pfad, "w"), indent=2, ensure_ascii=False)
PYTHON
}

# Die Rolle, die NICHT meine ist — fuer den Fremdfall.
if [ "$MEINE_ROLLE" = "Worker-1" ]; then FREMD=Worker-2; else FREMD=Worker-1; fi

fehler=0
pruefe() {   # name  erwartung(warnt|still)  ausgabe
  local name="$1" erwartet="$2" ausgabe="$3"
  local hat="still"
  case "$ausgabe" in *MESSFENSTER*) hat="warnt";; esac
  if [ "$hat" = "$erwartet" ]; then
    printf "  %-34s %-6s  ok\n" "$name" "$hat"
  else
    printf "  %-34s %-6s  FALSCH (erwartet %s)\n" "$name" "$hat" "$erwartet"
    fehler=1
  fi
}

echo "Eigene Rolle: $MEINE_ROLLE   Fremdrolle fuer die Probe: $FREMD"
echo

setze_stand "$FREMD" ""
A="$(bash "$WARNUNG" 2>&1)"
setze_stand "$MEINE_ROLLE" ""
B="$(bash "$WARNUNG" 2>&1)"
setze_stand "" ""
C="$(bash "$WARNUNG" 2>&1)"
setze_stand "$FREMD" "22.08. 01:00:00"
D="$(bash "$WARNUNG" 2>&1)"

pruefe "fremdes Fenster, keine Abgabe" warnt "$A"
pruefe "eigenes Fenster" still "$B"
pruefe "niemand haelt es" still "$C"
pruefe "fremdes Fenster, aber abgegeben" still "$D"

# --- Traegerzeile --------------------------------------------------------
#
# Ohne sie hiesse "drei Mal still" auch dann alles in Ordnung, wenn das
# Skript gar nichts ausgibt — etwa weil sein Pfad falsch ist. Genau dann
# waeren drei von vier Richtungen gruen.
if ! printf '%s' "$A" | grep -q "MESSFENSTER"; then
  echo
  echo "FEHLGESCHLAGEN: der EINE Fall, der warnen muss, hat geschwiegen." >&2
  echo "  Damit belegen die drei stillen Faelle NICHTS — ein Skript, das" >&2
  echo "  nie etwas sagt, besteht sie ebenfalls." >&2
  fehler=1
fi

echo
if [ "$fehler" -ne 0 ]; then
  echo "GEGENPROBE ROT." >&2
  exit 1
fi
echo "BESTANDEN: warnt bei fremdem Fenster, schweigt in den drei anderen Faellen."
