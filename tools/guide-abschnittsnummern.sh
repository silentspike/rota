#!/usr/bin/env bash
# Faengt doppelt vergebene Abschnittsnummern in den Dokumenten.
#
# # Warum es das gibt
#
# Am 2026-08-19 haben zwei Sitzungen unabhaengig voneinander Abschnitte an den
# WORKER-GUIDE angehaengt. Beide zweigten von demselben Stand ab, beide
# vergaben 8.103 aufwaerts — der eine 8.103 bis 8.106, der andere 8.103 bis
# 8.105. Aufgefallen ist es nur, weil git beim Merge einen Konflikt meldete:
# Beide Bloecke endeten am selben Dateiende.
#
# HAETTEN SIE AN VERSCHIEDENEN STELLEN GESTANDEN, waere der Merge glatt
# durchgelaufen, und in derselben Datei staenden zwei verschiedene 8.103. Kein
# Werkzeug haette es gemeldet: prettier prueft die Form, eslint den Code,
# niemand die Nummern.
#
# # Was er prueft
#
# Jede Nummer in einer Abschnittsueberschrift (`### 8.103 …`) darf je Datei
# genau einmal vorkommen. Mehr ist ein Fehler, auch wenn beide Abschnitte fuer
# sich richtig sind.
#
# # Was er NICHT prueft
#
# Luecken. Eine fehlende 8.42 ist kein Fehler — Abschnitte werden geloescht,
# und eine Umnummerierung waere teurer als die Luecke. Geprueft wird nur, dass
# keine Nummer zweimal vergeben ist.
set -euo pipefail

cd "$(dirname "$0")/.."

DATEIEN=(docs/WORKER-GUIDE.md docs/GAME-DESIGN.md)
fehler=0

for datei in "${DATEIEN[@]}"; do
  [ -f "$datei" ] || continue

  # Alle Abschnittsnummern der Form "### <Ziffern>.<Ziffern> "
  nummern=$(grep -oE '^#{2,4} [0-9]+\.[0-9]+ ' "$datei" | awk '{print $2}' || true)

  # **Kontrollzeile, und sie ist der Grund, warum das Ergebnis etwas heisst.**
  # Findet das Muster gar nichts, meldet die Dublettenpruefung darunter
  # "keine Dubletten" — ein gruenes Ergebnis aus dem falschen Grund. Genau die
  # halbe Kontrolle, die im Guide unter 8.103 steht.
  anzahl=$(printf '%s\n' "$nummern" | grep -c . || true)
  if [ "$anzahl" -lt 10 ]; then
    echo "::error file=$datei::Nur $anzahl Abschnittsnummern gefunden — das Muster greift nicht." >&2
    echo "  Erwartet werden Ueberschriften der Form '### 8.103 Titel'." >&2
    fehler=1
    continue
  fi

  doppelte=$(printf '%s\n' "$nummern" | sort | uniq -d || true)
  if [ -n "$doppelte" ]; then
    echo "::error file=$datei::Doppelt vergebene Abschnittsnummern:" >&2
    while read -r n; do
      [ -z "$n" ] && continue
      echo "  $n kommt mehrfach vor:" >&2
      grep -nE "^#{2,4} ${n//./\\.} " "$datei" | sed 's/^/    /' >&2
    done <<<"$doppelte"
    echo "  Zwei Sitzungen haben dieselbe Nummer vergeben. Die spaetere zieht" >&2
    echo "  weiter, der Inhalt bleibt." >&2
    fehler=1
  else
    echo "$datei: $anzahl Abschnittsnummern, keine doppelt."
  fi
done

exit "$fehler"
