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
# CI startet es deshalb über gegenproben-ci.py in einer isolierten Kopie.
set -uo pipefail
cd "$(dirname "$0")/.."

# **Warnt, wenn ein ANDERER gerade das Messfenster hält** (#1049). Blockiert
# nicht; Grenzen der Warnung stehen in der Datei selbst.
bash tools/fenster-warnung.sh || true

# **Zwei Dateien, nicht eine** — und das ist der Kern der Sache:
#
#   „vorne"  = an den Anfang der ERSTEN Quelle  (calcination.json)
#   „hinten" = hinter die letzte akzeptierte Kennung (Quelle wird gelesen)
#
# Meine erste Fassung fügte beides in calcination.json ein und meldete den
# erlaubten Fall als Verstoß. Zu Recht: Diese Datei steht in der Mitte der
# Gesamtliste (calcination, common, coagulation, fixation, sublimation,
# putrefaction, test-cast). Ein Zauber am Ende von calcination.json ist in der
# GESAMTLISTE ein Einschub und verschiebt alles danach.
#
# **Was daraus folgte, gilt seit dem 2026-08-06 nicht mehr.** Die Regel „neue
# Zauber ans Ende der Gesamtliste" ist zurückgenommen — sie sparte nachweislich
# keinen Neu-Export (23 bewegte Aufzeichnungen in beiden Fällen) und kostete den
# fachlich richtigen Ort. Einzelheiten im Kopf von `spell-order.test.ts`.
#
# Dieses Skript prüft deshalb jetzt fünf Fälle statt vier. Der fünfte ist der
# neue: Nach dem Nachziehen von `spell-order.json` muss der Wächter wieder grün
# sein. Ohne ihn belegte Fall D nur, dass der Wächter meckert — nicht, dass man
# weiterkommt.
ERSTE=data/spells/calcination.json
LETZTE=$(python3 tools/reihenfolge-einfuegen.py --letzte-datei) || exit 2
S1=$(mktemp)
S2=$(mktemp)
S3=$(mktemp)
cp "$ERSTE" "$S1"
cp "$LETZTE" "$S2"
cp tests/unit/spell-order.json "$S3"
trap 'cp "$S1" "$ERSTE"; cp "$S2" "$LETZTE"; cp "$S3" tests/unit/spell-order.json; rm -f "$S1" "$S2" "$S3"' EXIT

einfuegen() { # $1 = vorne|hinten
  python3 tools/reihenfolge-einfuegen.py "$1"
}

waechter() {
  # **Kein `grep -q` in der Pipe.** Es beendet die Leitung beim ersten Treffer,
  # vitest stirbt an SIGPIPE, und mit `pipefail` ist der Status dann ungleich
  # null — die Messung hätte IMMER „rot" gemeldet, auch unverändert.
  # Genau dafür steht der unveränderte Fall unten: Er hat es gefangen.
  local ausgabe code
  ausgabe=$(npx --no-install vitest run tests/unit/spell-order.test.ts 2>&1)
  code=$?
  if ! printf '%s' "$ausgabe" | grep -qE "Tests +[0-9]+ (passed|failed)"; then
    printf '%s\n' "$ausgabe" >&2
    echo 'FEHLER: keine Test-Ergebniszeile'
  elif [ "$code" = 0 ] && printf '%s' "$ausgabe" | grep -qE "Tests +[0-9]+ passed"; then
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

# **Der Fall, dessen BEDEUTUNG sich geändert hat** (Issues #408 → 2026-08-06):
# ans Ende der eigenen Schuldatei. Er ist weiterhin rot — die Reihenfolge
# verschiebt sich ja tatsächlich —, aber er ist kein Verstoß mehr.
#
# Er ist der NORMALE Fall für einen neuen Zauber. Gemessen wurde: Ein Zauber
# ganz hinten bewegt genauso viele Aufzeichnungen wie einer in der Mitte
# (23 gegen 23), weil die Prüfsumme über alle Zauber läuft. Die alte Regel
# sparte nichts und kostete den fachlich richtigen Ort.
#
# Was der Wächter hier leistet, ist eine **Meldung**: Zieh `spell-order.json`
# nach und exportiere — im selben Commit. Fall E unten belegt, dass danach
# grün ist.
echo -n "  ans Ende der EIGENEN Datei   -> "
einfuegen eigene-datei
D=$(waechter)
echo "$D"
cp "$S1" "$ERSTE"

# **Der Weg heraus** — und ohne ihn belegte Fall D nur, dass der Wächter meckert,
# nicht dass man weiterkommt. Derselbe Einschub, aber mit nachgezogener Liste.
echo -n "  ... und Liste nachgezogen    -> "
einfuegen eigene-datei
python3 -c "
import json
neu = [s['id'] for s in json.load(open('data/spells/calcination.json'))['spells']]
alt = json.load(open('tests/unit/spell-order.json'))
# Die Gesamtliste beginnt mit calcination; der Rest bleibt, wie er war.
json.dump(neu + alt[len(neu) - 1:], open('tests/unit/spell-order.json', 'w'), indent=2)
"
E=$(waechter)
echo "$E"
cp "$S1" "$ERSTE"
cp "$S3" tests/unit/spell-order.json

echo
if [ "$A" = gruen ] && [ "$B" = rot ] && [ "$C" = gruen ] && [ "$D" = rot ] && [ "$E" = gruen ]; then
  echo "OK — der Waechter merkt jede Verschiebung, laesst das reine Anhaengen"
  echo "     durch, und die nachgezogene Liste macht ihn wieder gruen."
  exit 0
fi
echo "FEHLGESCHLAGEN — erwartet: gruen / rot / gruen / rot / gruen,"
echo "                 gemessen: $A / $B / $C / $D / $E"
echo
echo "  Ist der ERSTE Fall rot, misst nicht der Waechter, sondern dieses Skript"
echo "  falsch — vor jeder anderen Deutung hier nachsehen."
echo "  Ist der ZWEITE Fall gruen, prueft der Waechter nicht, was er behauptet."
echo "  Ist der DRITTE Fall rot, ist der Waechter zu streng: Anfuegen ans Ende der"
echo "  LETZTEN Datei MUSS ohne Nachziehen durchgehen."
echo "  Ist der VIERTE Fall gruen, meldet der Waechter eine Verschiebung nicht mehr"
echo "  — dann faellt eine UNBEABSICHTIGTE Umsortierung durch."
echo "  Ist der FUENFTE Fall rot, gibt es keinen Weg heraus: Dann waere ein neuer"
echo "  Zauber in seiner Schuldatei dauerhaft blockiert, und die Ruecknahme der"
echo "  Regel vom 2026-08-06 haette nichts gebracht."
exit 1
