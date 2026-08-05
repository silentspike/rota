#!/usr/bin/env bash
# Gegenprobe zum Zauberreihenfolge-Wächter (Issue #402, AC3).
#
# Ein Wächter, der nur grün gelaufen ist, belegt nichts: Er sieht genauso aus
# wie einer, der gar nichts prüft. Dieses Skript bricht absichtlich, was er
# schützt, und zeigt, dass er es merkt — und dass er den ERLAUBTEN Fall
# durchlässt.
#
#   tools/reihenfolge-gegenprobe.sh
#
# Es fasst echte Datendateien an und stellt sie danach wieder her, auch wenn es
# unterwegs abbricht.
set -uo pipefail
cd "$(dirname "$0")/.."

# **Zwei Dateien, nicht eine** — und das ist der Kern der Sache:
#
#   „vorne"  = an den Anfang der ERSTEN Quelle  (calcination.json)
#   „hinten" = an das Ende der LETZTEN Quelle   (test-cast.json)
#
# Meine erste Fassung fügte beides in calcination.json ein und meldete den
# erlaubten Fall als Verstoß. Zu Recht: Diese Datei steht in der Mitte der
# Gesamtliste (calcination, common, coagulation, fixation, sublimation,
# putrefaction, test-cast). Ein Zauber am Ende von calcination.json ist in der
# GESAMTLISTE ein Einschub und verschiebt alles danach.
#
# Damit ist die Regel „neue Zauber ans Ende" mehrdeutig, solange nicht dabei
# steht, ans Ende WOVON. Die Gegenprobe hat das gefangen, bevor jemand danach
# gehandelt hat.
ERSTE=data/spells/calcination.json
LETZTE=data/spells/test-cast.json
S1=$(mktemp /work/tmp/erste-XXXXXX.json)
S2=$(mktemp /work/tmp/letzte-XXXXXX.json)
cp "$ERSTE" "$S1"
cp "$LETZTE" "$S2"
trap 'cp "$S1" "$ERSTE"; cp "$S2" "$LETZTE"; rm -f "$S1" "$S2"' EXIT

einfuegen() { # $1 = vorne|hinten
  python3 tools/reihenfolge-einfuegen.py "$1"
}

waechter() {
  # **Kein `grep -q` in der Pipe.** Es beendet die Leitung beim ersten Treffer,
  # vitest stirbt an SIGPIPE, und mit `pipefail` ist der Status dann ungleich
  # null — die Messung hätte IMMER „rot" gemeldet, auch unverändert.
  # Genau dafür steht der unveränderte Fall unten: Er hat es gefangen.
  local ausgabe
  ausgabe=$(npx vitest run tests/unit/spell-order.test.ts 2>&1)
  if printf '%s' "$ausgabe" | grep -qE "Tests +[0-9]+ passed"; then
    echo gruen
  else
    echo rot
  fi
}

echo "== Gegenprobe Zauberreihenfolge (Issue #402) =="

echo -n "  unveraendert                 -> "
A=$(waechter)
echo "$A"

echo -n "  Zauber VORNE eingefuegt      -> "
einfuegen vorne
B=$(waechter)
echo "$B"
cp "$S1" "$ERSTE"

echo -n "  Zauber HINTEN angefuegt      -> "
einfuegen hinten
C=$(waechter)
echo "$C"
cp "$S2" "$LETZTE"

# **Der lehrreiche Fall** (Issue #408): ans Ende der EIGENEN Schuldatei. Das
# ist die naheliegende Lesart von „hinten" — und sie ist falsch, weil nach
# calcination.json noch sechs Dateien kommen. Er MUSS rot sein.
echo -n "  ans Ende der EIGENEN Datei   -> "
einfuegen eigene-datei
D=$(waechter)
echo "$D"
cp "$S1" "$ERSTE"

echo
if [ "$A" = gruen ] && [ "$B" = rot ] && [ "$C" = gruen ] && [ "$D" = rot ]; then
  echo "OK — der Waechter merkt beide Einschuebe und laesst nur das echte"
  echo "     Anfuegen ans Ende der LETZTEN Datei durch."
  exit 0
fi
echo "FEHLGESCHLAGEN — erwartet: gruen / rot / gruen / rot, gemessen: $A / $B / $C / $D"
echo
echo "  Ist der ERSTE Fall rot, misst nicht der Waechter, sondern dieses Skript"
echo "  falsch — vor jeder anderen Deutung hier nachsehen."
echo "  Ist der MITTLERE Fall gruen, prueft der Waechter nicht, was er behauptet."
echo "  Ist der DRITTE Fall rot, ist der Waechter zu streng: Anfuegen ans Ende der"
echo "  LETZTEN Datei MUSS erlaubt bleiben, sonst kann kein Zauber dazukommen."
echo "  Ist der VIERTE Fall gruen, ist die Regel aus #408 verletzt: Ans Ende der"
echo "  eigenen Schuldatei ist ein Einschub, weil danach sechs Dateien folgen."
exit 1
