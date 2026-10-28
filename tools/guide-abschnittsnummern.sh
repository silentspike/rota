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

# **Codebloecke zaehlen nicht mit.** Eine Zeile wie `### 8.110 …` innerhalb
# eines ```-Blocks ist BEISPIELTEXT, keine Ueberschrift. Ohne diesen Filter
# meldete der Waechter sie als Dublette — und verbot damit eine legitime
# Schreibweise: Wer im Guide zeigen will, wie eine Ueberschrift aussieht, kann
# sie nicht zitieren (#1023).
#
# GELEERT, NICHT GELOESCHT. Die Zeilen bleiben als leere Zeilen stehen, damit
# `grep -n` weiter die ECHTEN Zeilennummern der Datei meldet. Ein Filter, der
# Zeilen entfernt, haette die Fundstellen-Ausgabe um die Zahl der Codezeilen
# verschoben — und eine Fundstelle, die auf die falsche Zeile zeigt, ist
# schlimmer als keine.
ohne_codebloecke() {
  awk '
    /^[[:space:]]*(```|~~~)/ { drin = !drin; print ""; next }
    drin                     { print ""; next }
                             { print }
  ' "$1"
}

# **Abgelesen, nicht aufgezaehlt.** Die erste Fassung trug zwei Dateinamen fest
# im Skript — und deckte damit zwei von fuenf Dokumenten ab, die
# Abschnittsnummern tragen (suno-sfx.md, VERIFICATION.md und STEP1-SPEC.md
# fehlten). Ein sechstes Dokument haette der Waechter nie zu sehen bekommen,
# und er haette weiter brav "keine doppelt" gemeldet — ueber die zwei, die er
# kannte. (Stand von #1023. suno-sfx.md liegt seit 2026-10-28 im privaten
# Werkzeug-Repo silentspike/rota-werkzeuge, die Ablesung fand sie danach von
# selbst nicht mehr.)
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
  # **Kein `grep -q` hinter dem Filter.** `-q` beendet nach dem ersten Treffer,
  # awk bekommt SIGPIPE und endet mit 141 — und `set -o pipefail` macht daraus
  # eine rote Pipe. Die Datei fiele dann aus der Auswahl, OBWOHL sie Nummern
  # traegt.
  #
  # Gemessen beim Bau von #1023: Mit `-q` blieb von fuenf Dokumenten genau
  # eines uebrig (STEP1-SPEC.md, 4 Nummern) — das kleinste. Bei ihm war awk
  # fertig, bevor grep abbrach. Die vier groesseren verschwanden lautlos, und
  # der Waechter meldete Exit 0.
  treffer=$(ohne_codebloecke "$kandidat" | grep -cE '^#{2,4} [0-9]+\.[0-9]+ ' || true)
  if [ "$treffer" -gt 0 ]; then
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
  nummern=$(ohne_codebloecke "$datei" | grep -oE '^#{2,4} [0-9]+\.[0-9]+ ' | awk '{print $2}' || true)

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
      ohne_codebloecke "$datei" | grep -nE "^#{2,4} ${n//./\\.} " | sed 's/^/    /' >&2
    done <<<"$doppelte"
    echo "  Zwei Sitzungen haben dieselbe Nummer vergeben. Die spaetere zieht" >&2
    echo "  weiter, der Inhalt bleibt." >&2
    fehler=1
  else
    echo "$datei: $anzahl Abschnittsnummern, keine doppelt."
  fi
done

exit "$fehler"
