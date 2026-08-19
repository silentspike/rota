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

# **Abgelesen, nicht aufgezaehlt.** Die erste Fassung trug zwei Dateinamen fest
# im Skript — und deckte damit zwei von fuenf Dokumenten ab, die
# Abschnittsnummern tragen (suno-sfx.md, VERIFICATION.md und STEP1-SPEC.md
# fehlten). Ein sechstes Dokument haette der Waechter nie zu sehen bekommen,
# und er haette weiter brav "keine doppelt" gemeldet — ueber die zwei, die er
# kannte.
#
# Eine Liste dort, wo eine Ablesung hingehoert: Der Waechter waechst dann nicht
# mit dem Bestand, sondern muss gepflegt werden — und wer ihn pflegen muesste,
# weiss nicht, dass er es muesste.
DATEIEN=()
for kandidat in docs/*.md; do
  [ -f "$kandidat" ] || continue
  # Nur Dokumente mit Abschnittsnummern. Die uebrigen zu pruefen waere nicht
  # falsch, aber die Kontrollzeile unten wuerde sie als "Muster greift nicht"
  # melden — und ein Waechter, der bei jedem Lauf ueber harmlose Dateien
  # meckert, wird abgeschaltet.
  if grep -qE '^#{2,4} [0-9]+\.[0-9]+ ' "$kandidat"; then
    DATEIEN+=("$kandidat")
  fi
done

if [ "${#DATEIEN[@]}" -eq 0 ]; then
  echo "::error::Kein Dokument unter docs/ traegt Abschnittsnummern — das Muster greift nicht." >&2
  exit 1
fi

fehler=0

for datei in "${DATEIEN[@]}"; do
  [ -f "$datei" ] || continue

  # Alle Abschnittsnummern der Form "### <Ziffern>.<Ziffern> "
  nummern=$(grep -oE '^#{2,4} [0-9]+\.[0-9]+ ' "$datei" | awk '{print $2}' || true)

  # **Kontrollzeile, und sie ist der Grund, warum das Ergebnis etwas heisst.**
  # Findet das Muster gar nichts, meldet die Dublettenpruefung darunter
  # "keine Dubletten" — ein gruenes Ergebnis aus dem falschen Grund.
  #
  # DIE SCHWELLE WAR ZUERST 10 UND WAR FALSCH. Sie stammte aus der Zeit, als
  # das Skript nur den WORKER-GUIDE und GAME-DESIGN kannte (91 und 77
  # Nummern). Seit die Dateien abgelesen statt aufgezaehlt werden, sind auch
  # kleine dabei: STEP1-SPEC.md traegt 4, VERIFICATION.md 5. Die Schwelle
  # brach dort ab — MIT `continue`, also VOR der Dublettenpruefung.
  #
  # Gemessen: Eine kuenstlich eingebaute Dublette in VERIFICATION.md wurde
  # NICHT gemeldet. Der Waechter war rot, aber aus dem falschen Grund, und der
  # echte Befund fiel unter den Tisch. Die Schutzmassnahme verdeckte genau
  # das, wovor sie schuetzen sollte.
  #
  # DIE RICHTIGE SCHWELLE IST 1, und sie prueft jetzt etwas anderes: Die
  # Dateiauswahl oben nimmt nur Dateien MIT Treffern. Findet die Zaehlung hier
  # trotzdem keinen, sind die beiden Muster auseinandergelaufen — und das ist
  # ein Fehler im Skript selbst, kein Befund ueber das Dokument.
  anzahl=$(printf '%s\n' "$nummern" | grep -c . || true)
  if [ "$anzahl" -lt 1 ]; then
    echo "::error file=$datei::Die Auswahl fand Abschnittsnummern, die Zaehlung nicht." >&2
    echo "  Die beiden Muster im Skript sind auseinandergelaufen." >&2
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
