#!/bin/bash
# AC6: Was bewegt ein neuer Zauber in den bestehenden Aufzeichnungen?
#
# Kein BESTEHENDER Wert darf sich bewegen — ausser den drei Pruefsummen.
# Neue Zauber duerfen Eintraege HINZUFUEGEN; sie duerfen keinen alten aendern.
#
# ZWEI FALLEN, BEIDE AM EIGENEN LEIB GEMESSEN (2026-08-05):
#
#   1. Der Bezugspunkt.  Die erste Fassung verglich gegen `git show HEAD:`.
#      Nach einem Commit ist HEAD der eigene Stand — das Skript verglich die
#      Datei mit sich selbst und meldete ab da IMMER "OK". Gefunden von
#      Worker-1, weil ihm das OK zu gut vorkam.
#      Deshalb: merge-base, und der Bezugspunkt wird AUSGEDRUCKT. Ein
#      Vergleich, der nicht sagt wogegen, ist keine Messung.
#
#   2. Der Abbruch.  Zwei Aufzeichnungs-Formate liegen nebeneinander
#      (`runs[]` gegen flach). Ein jq-Filter, der eines voraussetzt, stirbt
#      beim anderen — und zwei leere Ausgaben vergleichen sich als gleich.
#      Deshalb bricht jeder jq-Fehler mit Exit 2 ab. LASS DAS DRIN.
set -uo pipefail
HIER="$(cd "$(dirname "$0")" && pwd)"
BASIS="${AC6_BASIS:-$(git merge-base origin/main HEAD)}"

if [ "$(git rev-parse "$BASIS")" = "$(git rev-parse HEAD)" ]; then
  echo "ABBRUCH: Bezugspunkt ist HEAD selbst — es gaebe nichts zu vergleichen."
  echo "         (Zweig ohne eigene Commits? Dann ist dieses Skript sinnlos.)"
  exit 2
fi
echo "Bezugspunkt: $(git rev-parse --short "$BASIS")  $(git log -1 --format=%s "$BASIS" | cut -c1-60)"

STRIP='walk(if type=="object" then del(.checksums,.sequenceChecksum,.finalChecksum) else . end)'
schuldig=0
for p in tests/parity/recordings/*.json; do
  git show "$BASIS:$p" 2>/dev/null | jq -S "$STRIP" > /work/tmp/ac6-a.json || {
    # Neue Datei im Zweig: es gibt nichts zu vergleichen, das ist kein Fehler.
    git cat-file -e "$BASIS:$p" 2>/dev/null || { echo "  neu im Zweig: $(basename "$p")"; continue; }
    echo "ABBRUCH lesen $p"; exit 2; }
  jq -S "$STRIP" "$p" > /work/tmp/ac6-b.json || { echo "ABBRUCH lesen $p"; exit 2; }
  bewegt=$(jq -rn --slurpfile a /work/tmp/ac6-a.json --slurpfile b /work/tmp/ac6-b.json \
           -f "$HIER/ac6-vergleich.jq") || { echo "ABBRUCH vergleichen $p"; exit 2; }
  [ -n "$bewegt" ] && { printf "%-26s %s\n" "$(basename "$p")" "$bewegt"; schuldig=1; }
done
[ $schuldig = 0 ] && echo "OK - kein bestehender Wert bewegt sich"
exit $schuldig
