#!/usr/bin/env bash
# Gegenprobe zu tools/ac6.sh (Issue #1035).
#
# Ein Wächter, der nur grün gelaufen ist, belegt nichts. Dieses Skript führt
# alle vier Fälle herbei, die ac6.sh unterscheiden muss, und prüft für jeden
# den Exit-Code UND die Ausgabe:
#
#   A  neue Aufzeichnung, gültig     -> "neu im Zweig", Exit 0
#   B  gesunder Lauf                 -> "OK", Exit 0
#   C  neue Aufzeichnung, KAPUTT     -> "ABBRUCH", Exit 2      <- der Fall aus #1035
#   D  bestehende Aufzeichnung kaputt-> "ABBRUCH", Exit 2
#
#   tools/ac6-gegenprobe.sh
#
# WARUM ES DIESES SKRIPT GIBT. Vor dem Fix erzeugten A und C **byteweise
# dieselbe Ausgabe** und denselben Exit-Code: Der `continue` für "neu im Zweig"
# sprang an der Stelle vorbei, an der jq die Datei angesehen hätte. ac6.sh
# konnte eine unbrauchbare neue Aufzeichnung nicht von einer gültigen
# unterscheiden und meldete für beide OK.
#
# C ist deshalb der einzige Fall, der ohne den Fix rot wird. A, B und D sind
# die Gegenrichtungen — ohne sie hätte man einen Wächter gebaut, der jede neue
# Datei blockiert, und es nicht gemerkt.
#
# DER URSPRÜNGLICHE BEFUND WAR EIN ANDERER UND WAR FALSCH: "der Abbruch bei
# Lesefehler ist unerreichbar". Er entstand in einer Shell ohne `pipefail` —
# ac6.sh setzt es in Zeile 20, und damit greift `git show … | jq || { … }`
# sehr wohl. Fall D hält das fest, damit der widerlegte Befund nicht
# wiederkehrt.
#
# Fasst echte Datendateien an und stellt sie danach wieder her, auch bei
# Abbruch.
set -uo pipefail
cd "$(dirname "$0")/.."

REC=tests/parity/recordings
VORLAGE="$REC/aether-fenster.json"
OPFER="$VORLAGE"                       # für Fall D
SICHERUNG=$(mktemp /work/tmp/ac6-gegenprobe-XXXXXX.json)
NEU_GUELTIG="$REC/zzz-gegenprobe-gueltig.json"
NEU_KAPUTT="$REC/zzz-gegenprobe-kaputt.json"

aufraeumen() {
  rm -f "$NEU_GUELTIG" "$NEU_KAPUTT"
  [ -s "$SICHERUNG" ] && cp "$SICHERUNG" "$OPFER"
  rm -f "$SICHERUNG"
}
trap aufraeumen EXIT

cp "$VORLAGE" "$SICHERUNG"

# Ein Bezugspunkt, der NICHT HEAD ist — sonst bricht ac6.sh vorher ab
# ("Bezugspunkt ist HEAD selbst"). Das ist kein Trick, sondern derselbe Weg,
# den ac6.sh selbst anbietet.
BASIS=$(git rev-parse HEAD~5 2>/dev/null) || {
  echo "ABBRUCH: kein HEAD~5 vorhanden — dieser Zweig ist zu flach fuer die Gegenprobe."
  exit 2
}

fehler=0

# fall <name> <soll-exit> <soll-text> — führt ac6.sh aus und prüft BEIDES.
# Der Text zählt mit, weil ein Exit-Code allein nicht sagt, WARUM.
fall() {
  local name="$1" soll_exit="$2" soll_text="$3" aus code
  aus=$(AC6_BASIS="$BASIS" bash tools/ac6.sh 2>&1)
  code=$?
  local ok_code=ja ok_text=ja
  [ "$code" = "$soll_exit" ] || ok_code=NEIN
  grep -q -- "$soll_text" <<<"$aus" || ok_text=NEIN
  if [ "$ok_code" = ja ] && [ "$ok_text" = ja ]; then
    printf "  [ok ] %-34s Exit %s, Text gefunden\n" "$name" "$code"
  else
    printf "  [ROT] %-34s Exit %s (Soll %s), Text '%s' %s\n" \
           "$name" "$code" "$soll_exit" "$soll_text" \
           "$([ "$ok_text" = ja ] && echo gefunden || echo FEHLT)"
    printf "        Ausgabe war:\n%s\n" "$(sed 's/^/          /' <<<"$aus")"
    fehler=1
  fi
}

echo "=== Gegenprobe zu ac6.sh, Bezugspunkt $(git rev-parse --short "$BASIS") ==="

fall "B gesunder Lauf" 0 "OK - kein bestehender Wert bewegt sich"

cp "$VORLAGE" "$NEU_GUELTIG"
fall "A neue Aufzeichnung, gueltig" 0 "neu im Zweig: $(basename "$NEU_GUELTIG")"
rm -f "$NEU_GUELTIG"

printf '{ das ist kein JSON' > "$NEU_KAPUTT"
fall "C neue Aufzeichnung, KAPUTT" 2 "ABBRUCH lesen"
rm -f "$NEU_KAPUTT"

printf '{ kaputt' > "$OPFER"
fall "D bestehende Aufzeichnung kaputt" 2 "ABBRUCH lesen"
cp "$SICHERUNG" "$OPFER"

echo
if [ "$(md5sum < "$OPFER")" = "$(md5sum < "$SICHERUNG")" ]; then
  echo "  Arbeitsbaum: $(basename "$OPFER") wiederhergestellt (md5 gleich)"
else
  echo "  ROT: $(basename "$OPFER") weicht von der Sicherung ab!"
  fehler=1
fi

echo
if [ "$fehler" = 0 ]; then
  echo "Gegenprobe gruen: alle vier Faelle unterscheidbar."
  echo "Ohne den Fix aus #1035 waere C ROT — mit derselben Ausgabe wie A."
  exit 0
fi
echo "GEGENPROBE ROT — siehe oben."
exit 1
