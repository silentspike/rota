#!/usr/bin/env bash
# Gegenprobe zum Abschnittsnummern-Wächter (Issue #1023).
#
# Der Wächter zählte Abschnittsnummern, die in einem ```-Block als BEISPIEL
# stehen, als echte Überschriften. Wer im Guide zeigen wollte, wie eine
# Überschrift aussieht, konnte sie nicht zitieren — der Wächter meldete eine
# Dublette.
#
#   tools/guide-abschnittsnummern-gegenprobe.sh
#
# VIER FÄLLE, und der vierte ist der, um den es hier eigentlich geht:
#
#   A  Beispiel im Codeblock       -> KEIN Alarm        (der Fehlalarm aus #1023)
#   B  echte Dublette daneben      -> Alarm             (die Gegenrichtung)
#   C  gesunder Lauf               -> Exit 0, 5 Dateien (findet er überhaupt noch was?)
#   D  ZEILENTREUE                 -> die gemeldete Zeile IST die echte
#
# WARUM D EXISTIERT. Der naheliegende Fix wäre, Codeblock-Zeilen zu LÖSCHEN.
# Der bestünde A, B und C — und würde alle Fundstellen um die Zahl der
# Codezeilen verschieben. Eine Fundstelle, die auf die falsche Zeile zeigt, ist
# schlimmer als keine, und keiner der ersten drei Fälle würde es merken.
#
# Deshalb leert der Filter die Zeilen, statt sie zu entfernen. D ist der
# einzige Fall, der eine schwächere Fassung von einer richtigen unterscheidet.
#
# DIE MELDETEXTE DES WÄCHTERS SIND EINE SCHNITTSTELLE, kein Beiwerk. Diese
# Gegenprobe prüft je Fall den Exit-Code UND den Text — ein Exit-Code allein
# sagt nicht, WARUM. Damit hängt sie an Formulierungen wie „Doppelt vergebene"
# und „Abschnittsnummern, keine doppelt": **Wer sie im Wächter umformuliert,
# bricht diese Datei.** Das ist gewollt (die Alternative wäre eine Gegenprobe,
# die zwei Fälle nicht trennt) — aber es gehört gewusst, und die ROT-Zeilen
# sagen es: „die GEGENPROBE passt nicht mehr" ist eine andere Diagnose als
# „der Wächter ist kaputt".
#
# Fasst docs/VERIFICATION.md an und stellt die Datei danach wieder her, auch
# bei Abbruch.
set -uo pipefail
cd "$(dirname "$0")/.."

WAECHTER=tools/guide-abschnittsnummern.sh
OPFER=docs/VERIFICATION.md
SICHERUNG=$(mktemp)

aufraeumen() {
  [ -s "$SICHERUNG" ] && cp "$SICHERUNG" "$OPFER"
  rm -f "$SICHERUNG"
}
trap aufraeumen EXIT

cp "$OPFER" "$SICHERUNG"

# Eine Nummer, die es in der Datei WIRKLICH gibt — daran hängt jeder Fall.
# Abgelesen, nicht eingetragen: Wer sie im Skript festschreibt, misst nach dem
# nächsten Umbau der Datei etwas anderes als er glaubt.
NUMMER=$(grep -oE '^#{2,4} [0-9]+\.[0-9]+ ' "$OPFER" | head -1 | awk '{print $2}')
ECHTE_ZEILE=$(grep -nE "^#{2,4} ${NUMMER//./\\.} " "$OPFER" | head -1 | cut -d: -f1)
[ -n "$NUMMER" ] && [ -n "$ECHTE_ZEILE" ] || {
  echo "ABBRUCH: keine Abschnittsnummer in $OPFER gefunden — die Gegenprobe kann nichts messen."
  exit 2
}

fehler=0
echo "=== Gegenprobe zum Abschnittsnummern-Waechter ==="
echo "    Bezug: $NUMMER in $OPFER, echte Zeile $ECHTE_ZEILE"
echo

melde() {  # melde <ok|ROT> <name> <text…>
  if [ "$1" = ok ]; then printf "  [ok ] %-38s %s\n" "$2" "$3"
  else printf "  [ROT] %-38s %s\n" "$2" "$3"; fehler=1; fi
}

# --- C: gesunder Lauf -------------------------------------------------------
#
# C zählt bewusst NUR die unauffällige Zeilenform, anders als Fall A. Das ist
# kein Versehen: C verlangt zusätzlich `code = 0`. Gäbe es hier eine echte
# Dublette, wäre der Code 1 und C schon deshalb rot — die alarmierende
# Zeilenform kann in einem bestandenen Fall C gar nicht vorkommen.
#
# Und die ROT-Zeile nennt beide Werte, nicht eine vermutete Ursache: Der Leser
# sieht, welche der zwei Bedingungen gerissen ist.
aus=$(bash "$WAECHTER" 2>&1); code=$?
dateien=$(grep -c 'Abschnittsnummern, keine doppelt' <<<"$aus" || true)
if [ "$code" = 0 ] && [ "$dateien" -ge 5 ]; then
  melde ok "C gesunder Lauf" "Exit 0, $dateien Dateien geprueft"
else
  melde ROT "C gesunder Lauf" "Exit $code, nur $dateien Dateien (Soll: 0 und >=5)"
  printf "%s\n" "$aus" | sed 's/^/        /'
fi

# --- A: Beispiel im Codeblock darf NICHT ausloesen ---------------------------
#
# **Exit 0 allein genuegt hier nicht.** "Kein Alarm" und "gar nichts gemessen"
# sehen von aussen gleich aus: Ein Waechter, der keine Datei findet, alarmiert
# auch nicht. Deshalb zaehlt A die geprueften Dateien mit — erst dann heisst
# Exit 0 wirklich "kein Fehlalarm".
#
# Gefunden vom Orchestrator mit einer dritten Mutation (Filter leert ALLES,
# nicht nur Codebloecke): A wurde rot und meldete "Fehlalarm auf Beispieltext".
# Es gab keinen Fehlalarm — es gab keine Messung. Die ROT-Zeile nennt jetzt,
# WAS gemessen wurde, statt eine Ursache zu behaupten.
{
  printf '\n<!-- gegenprobe-1023 -->\n\n```text\n'
  printf '### %s Beispieltext, keine Ueberschrift\n' "$NUMMER"
  printf '```\n'
} >> "$OPFER"
aus=$(bash "$WAECHTER" 2>&1); code=$?
# ZWEI Zeilenformen zaehlen als "geprueft": die unauffaellige Meldung UND der
# Alarm. Ein erster Versuch zaehlte nur "keine doppelt" — unter der Mutation
# "Filter raus" meldete A dann "nur 4 Dateien, nichts gemessen", obwohl sehr
# wohl gemessen worden war: Die fuenfte Datei war die alarmierende. Die
# Diagnose waere falsch gewesen, und zwar in die bequeme Richtung.
geprueft=$(grep -cE 'Abschnittsnummern, keine doppelt|Doppelt vergebene' <<<"$aus" || true)
if [ "$code" = 0 ] && [ "$geprueft" -ge 5 ]; then
  melde ok "A Beispiel im Codeblock" "kein Alarm, Exit 0, $geprueft Dateien geprueft"
elif [ "$geprueft" -lt 5 ]; then
  melde ROT "A Beispiel im Codeblock" "Exit $code, aber nur $geprueft Dateien geprueft — nichts gemessen, nicht 'kein Alarm'"
  printf "%s\n" "$aus" | head -3 | sed 's/^/        /'
else
  melde ROT "A Beispiel im Codeblock" "Exit $code bei $geprueft geprueften Dateien — Fehlalarm auf Beispieltext"
  grep -A3 'Doppelt' <<<"$aus" | sed 's/^/        /'
fi

# --- B und D: echte Dublette daneben, und die Zeile muss stimmen -------------
printf '\n### %s Eine echte Dublette, ausserhalb jedes Codeblocks\n' "$NUMMER" >> "$OPFER"
DUBLETTEN_ZEILE=$(grep -nE "^#{2,4} ${NUMMER//./\\.} Eine echte" "$OPFER" | cut -d: -f1)
aus=$(bash "$WAECHTER" 2>&1); code=$?

# **Zwei Bedingungen, ZWEI Diagnosen.** Ein gemeinsamer ELSE-Zweig für
# `code = 1 && Text gefunden` behauptet die Ursache, die er für
# wahrscheinlicher hält — dasselbe wie in Fall A, nur eine Ebene höher.
#
# Gemessen vom Orchestrator mit einer vierten Mutation, die NUR die Wortwahl
# des Wächters änderte ("Doppelt" -> "Mehrfach"): B meldete "Exit 1, kein
# Alarm — der Wächter findet nichts mehr". Beides falsch. Der Wächter hatte
# alarmiert (Exit 1 stand in derselben Zeile) und die Dublette gefunden, nur
# mit anderem Wortlaut. Die richtige Diagnose ist: die GEGENPROBE passt nicht
# mehr, nicht der Wächter.
if [ "$code" != 1 ]; then
  melde ROT "B echte Dublette" "kein Alarm, Exit $code — der Waechter findet die Dublette nicht"
elif grep -q 'Muster greift nicht' <<<"$aus"; then
  # DRITTER Zweig, beim Bauen des zweiten gefunden: Exit 1 heisst nicht
  # zwangslaeufig "alarmiert". Der Waechter hat eine Traegerzeile, die
  # ebenfalls 1 liefert — dann hat er gar keine Datei angesehen, und die
  # Gegenprobe ist voellig in Ordnung. Ohne diesen Zweig behauptete B genau
  # das Gegenteil.
  melde ROT "B echte Dublette" "Exit 1, aber der Waechter fand keine Datei — kein Alarm, sondern seine Traegerzeile"
  grep -m1 '::error' <<<"$aus" | sed 's/^/        tatsaechlich: /'
elif ! grep -q 'Doppelt vergebene' <<<"$aus"; then
  melde ROT "B echte Dublette" "Alarm ausgeloest (Exit 1), aber die erwartete Meldung fehlt — die GEGENPROBE passt nicht mehr"
  grep -m1 '::error' <<<"$aus" | sed 's/^/        tatsaechlich: /'
else
  melde ok "B echte Dublette" "Alarm, Exit 1"
fi

# D: Die gemeldeten Zeilennummern muessen die ECHTEN sein. Ein Filter, der
# Zeilen loescht statt sie zu leeren, verschiebt sie — und faellt nur hier auf.
gemeldet=$(grep -oE "^    [0-9]+:#" <<<"$aus" | tr -d ' :#' | tr '\n' ' ')
if grep -qE "(^| )$ECHTE_ZEILE( |$)" <<<"$gemeldet" \
   && grep -qE "(^| )$DUBLETTEN_ZEILE( |$)" <<<"$gemeldet"; then
  melde ok "D Zeilentreue" "meldet $ECHTE_ZEILE und $DUBLETTEN_ZEILE"
else
  melde ROT "D Zeilentreue" "meldet '$gemeldet', erwartet $ECHTE_ZEILE und $DUBLETTEN_ZEILE"
fi

# Und die Beispielzeile darf NICHT dabei sein.
#
# **Dieselbe Voraussetzung wie D:** "X ist nicht dabei" ist trivial erfuellt,
# wenn gar nichts dabei ist. Ohne die Pruefung auf nicht-leere Fundstellen war
# diese Zeile unter der dritten Mutation des Orchestrators gruen — waehrend die
# Fundstellen leer waren und die vier Zeilen darueber rot.
BEISPIEL_ZEILE=$(grep -nE "^#{2,4} ${NUMMER//./\\.} Beispieltext" "$OPFER" | cut -d: -f1)
if [ -z "${gemeldet// /}" ]; then
  melde ROT "D Beispielzeile ausgeschlossen" "keine Fundstellen — die Pruefung waere leer erfuellt"
elif grep -qE "(^| )$BEISPIEL_ZEILE( |$)" <<<"$gemeldet"; then
  melde ROT "D Beispielzeile ausgeschlossen" "Zeile $BEISPIEL_ZEILE steht in den Fundstellen"
else
  melde ok "D Beispielzeile ausgeschlossen" "Zeile $BEISPIEL_ZEILE fehlt unter $gemeldet"
fi

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
  echo "Gegenprobe gruen: Fehlalarm weg, echte Dublette bleibt, Zeilennummern stimmen."
  exit 0
fi
echo "GEGENPROBE ROT — siehe oben."
exit 1
