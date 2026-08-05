#!/bin/bash
# AC6 (neu): Kein BESTEHENDER Wert bewegt sich - ausser den drei Pruefsummen.
#
# Aufruf aus der Wurzel des Arbeitsbaums:   tools/ac6.sh
#
# WARUM ES IHN GIBT (Issue #268)
# Ein neuer Zauber verschiebt JEDE Pruefsumme - die Schleifen in
# `combatChecksum` laufen ueber alle Zauber, drei mehr sind drei Durchlaeufe
# mehr. "Jede bestehende Pruefsumme unveraendert" ist damit kein erfuellbares
# Kriterium, sobald ein Zauber dazukommt.
#
# Was man WIRKLICH wissen will: Hat der neue Zauber einen bestehenden KAMPF
# veraendert? Dieses Skript beantwortet genau das - es blendet die drei
# Pruefsummen aus und vergleicht alles uebrige.
#
# Erwartet bewegen sich nur `playerCasting` (ein Zauber-INDEX) und
# `fieldOrder` (Positionen in der Namensliste): data.ts sortiert die
# Zauberdateien nach DATEINAMEN, und calcination.json ist die erste - jeder
# Kalzinations-Zauber schiebt alle anderen nach hinten. Meldet das Skript ein
# ANDERES Feld, hat ein neuer Zauber einen bestehenden Kampf veraendert.
# Neue Zauber duerfen Eintraege HINZUFUEGEN; sie duerfen keinen alten aendern.
# JEDER jq-FEHLER IST EIN ABBRUCH, KEIN STILLES GRUEN - und das ist der
# wichtigste Teil. Zwei Aufzeichnungs-Formate liegen nebeneinander:
# fixation.json hat `runs[]`, cast-time.json ist flach. Ein Filter, der nur
# eines kennt, stuerzt beim anderen ab - und dann verglichen sich zwei LEERE
# Ausgaben als gleich. So sind einmal 15 Dateien als "unveraendert" gemeldet
# worden, waehrend der Befehl in Wahrheit abgestuerzt war.
#
# Wer dieses Skript aendert: LASS DEN ABBRUCH DRIN. Ein Pruefbefehl, der bei
# einem Fehler gruen meldet, ist schlimmer als keiner.
STRIP='walk(if type=="object" then del(.checksums,.sequenceChecksum,.finalChecksum) else . end)'
schuldig=0
for p in tests/parity/recordings/*.json; do
  git show HEAD:"$p" | jq -S "$STRIP" > /work/tmp/ac6-a.json || { echo "ABBRUCH lesen $p"; exit 2; }
  jq -S "$STRIP" "$p"                 > /work/tmp/ac6-b.json || { echo "ABBRUCH lesen $p"; exit 2; }
  bewegt=$(jq -rn --slurpfile a /work/tmp/ac6-a.json --slurpfile b /work/tmp/ac6-b.json \
           -f "$(dirname "$0")/ac6-vergleich.jq") || { echo "ABBRUCH vergleichen $p"; exit 2; }
  [ -n "$bewegt" ] && { printf "%-26s %s\n" "$(basename "$p")" "$bewegt"; schuldig=1; }
done
[ $schuldig = 0 ] && echo "OK - kein bestehender Wert bewegt sich"
exit $schuldig
