#!/usr/bin/env bash
# Gegenprobe zu tools/fenster-warnung.sh (Issue #1049, Fertig-Kriterium 3).
#
#     EXIT 0    alle Richtungen wie erwartet
#     EXIT 1    eine Richtung ist GESCHEITERT — ein echter Befund
#     EXIT 2    die Pruefung hat NICHT STATTGEFUNDEN (Vorbedingung fehlt)
#
# Die Trennung von 1 und 2 ist nicht kosmetisch: Am 2026-08-22 meldete ein
# Mutationslauf Exit 1 mit „eigene Rolle nicht ermittelbar" — die Gegenprobe
# war abgebrochen, bevor sie irgendetwas prüfte, und „rot war da". Als AC-Beleg
# hätte das durchgehen können. Gefangen hat es nur, wer den Fehlertext las
# statt den Code zu zählen.
#
# Prüft alle Zustände an einem WEGWERF-Schalterstand, in derselben Ablesung:
#
#     FALL                              ERGEBNIS   VARIIERT
#     fremdes Fenster gehalten          warnt      inhaber
#     eigenes Fenster gehalten          schweigt   eigene-rolle
#     Fenster frei                      schweigt   inhaber
#     fremdes Fenster, aber abgegeben   schweigt   abgabe
#     Umgebung ohne rota-fenster (CI)   schweigt   umgebung
#     kurzer PATH, aber Werkzeug da     warnt      umgebung
#     Standdatei fehlt ganz             unlesbar   stand-vorhanden
#     Standdatei da, aber kaputt        unlesbar   stand-lesbar
#     sehr langer Zweck                 warnt      zweck-laenge
#
# # Die Spalte VARIIERT ist der eigentliche Gegenstand (#1081, AC6)
#
# Sie nennt, welche EIGENSCHAFT eine Richtung ändert — und am Ende der Ausgabe
# steht, welche der bekannten Eigenschaften GAR KEINE Richtung ändert.
#
# In einer Sitzung sind dreimal grüne Richtungen an derselben Sache
# vorbeigelaufen: eine erfundene PID (der /proc-Zweig wurde nie betreten), ein
# relativer Aufruf (alle Beispielzeilen trugen absolute Pfade), eine Umgebung
# ohne das Werkzeug (die Prüfung läuft nur dort, wo es existiert). Jedes Mal
# war die ZAHL der grünen Richtungen unauffällig. Es fehlte keine Richtung —
# es fehlte eine Eigenschaft, die keine Richtung variiert hat.
#
# **Eine fehlende Eigenschaft färbt den Lauf NICHT rot**, und das ist Absicht:
# Sie ist kein Fehler des geprüften Codes, sondern eine Lücke in dieser Datei.
# Wer eine neue Eigenschaft einträgt, bevor er die Richtung dazu baut, soll
# nicht gezwungen sein, beides in einem Zug zu tun. Der Block steht dafür am
# Ende und nennt die Folge im Klartext.
#
# # Warum alle und nicht nur der erste
#
# Ein Melder, der bei jedem Aufruf anschlägt, erfüllt die erste Richtung
# ebenfalls — und ist wertlos. Am 2026-08-22 ist ein Zwei-Leerzeichen-Detektor
# genau daran gescheitert: 1328 Treffer auf 1416 Nachrichten. Die Zahl, die
# zählt, ist nicht die Trefferzahl, sondern das Verhältnis von Anschlagen zu
# Schweigen.
#
# # DER ECHTE SCHALTERSTAND WIRD NICHT ANGEFASST — und zwar gar nicht (#1081)
#
# Die erste Fassung schrieb in `~/.claude/rota-fenster.json`, sicherte ihn
# vorher und schrieb ihn im `trap` zurück. Der Orchestrator hat gemessen, was
# das kostet: Während seiner Abnahme stand dort `Worker-1, seit 07:35:21` —
# ein Lauf hätte diesen Eintrag viermal durch einen erfundenen ersetzt.
#
# Die Rücksicherung schützt gegen das LIEGENBLEIBEN, nicht gegen das ABLESEN
# währenddessen. Wer in einem dieser vier Momente `rota-fenster status`
# gefahren hätte, hätte ein fremdes Fenster gesehen — genau der Vorfall vom
# 2026-08-19, den der Dateikopf der Warnung als Mahnung zitiert. Dazu: bei
# SIGKILL läuft kein `trap`, und `/work/tmp` teilen sich alle Sitzungen.
#
# Jetzt setzt die Probe `ROTA_FENSTER_STAND` auf eine eigene Datei. Sicherung,
# `trap` und der geteilte Pfad sind ERSATZLOS entfallen: Es gibt nichts mehr,
# was schiefgehen könnte, statt einer Absicherung, die in drei Fällen versagt.

set -u

HIER="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WARNUNG="$HIER/fenster-warnung.sh"

# Eigenes Verzeichnis JE LAUF. Ohne die PID hätte ich das geteilte /work/tmp
# nur an eine andere Stelle verschoben — zwei gleichzeitige Läufe schrieben
# wieder dieselbe Datei.
WEGWERF="/work/tmp/e9-1049-probe-$$"
mkdir -p "$WEGWERF"
PROBESTAND="$WEGWERF/rota-fenster.json"
trap 'rm -rf "$WEGWERF"' EXIT

# **Exit 2 heißt „hat nicht stattgefunden", Exit 1 heißt „gescheitert".**
#
# Beides war bis hierher Exit 1, und der Orchestrator ist am 2026-08-22 genau
# daran hängengeblieben: Sein Mutationslauf meldete Exit 1 mit „eigene Rolle
# nicht ermittelbar" — er stand in /work/tmp, und `rota-ipc` leitet die Rolle
# aus dem Verzeichnis ab. Die Gegenprobe war abgebrochen, BEVOR sie etwas
# prüft. Das hätte als bestandenes AC4 durchgehen können: „rot war da".
#
# Gefangen hat es nur, dass er den Fehlertext gelesen hat statt den Exit-Code
# zu zählen. Wer automatisiert prüft, sieht den Unterschied nicht — und das
# ist derselbe Unterschied wie zwischen einem fehlenden und einem grünen
# Messwert.
if ! command -v rota-ipc > /dev/null 2>&1; then
  echo "NICHT DURCHGEFUEHRT (Exit 2): rota-ipc ist nicht auffindbar." >&2
  echo "  Ohne die Rollenauskunft laesst sich 'eigenes' nicht von 'fremdem'" >&2
  echo "  Fenster trennen. Das ist KEIN Fehlschlag der Warnung — es ist eine" >&2
  echo "  Pruefung, die nicht stattgefunden hat." >&2
  exit 2
fi

MEINE_ROLLE="$(rota-ipc wer 2>/dev/null | awk '/^Rolle:/ {print $2; exit}')"
if [ -z "$MEINE_ROLLE" ]; then
  echo "NICHT DURCHGEFUEHRT (Exit 2): eigene Rolle nicht ermittelbar." >&2
  echo "  Haeufigste Ursache: falsches Arbeitsverzeichnis — rota-ipc leitet" >&2
  echo "  die Rolle daraus ab. Aufruf aus $(pwd)." >&2
  echo "  Ohne sie wuerde die Probe 'eigenes Fenster' und 'fremdes Fenster'" >&2
  echo "  gleich behandeln und trotzdem gruen melden." >&2
  exit 2
fi

setze_stand() {
  python3 - "$PROBESTAND" "$1" "$2" <<'PYTHON'
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

# Die Rolle, die NICHT meine ist — für den Fremdfall.
if [ "$MEINE_ROLLE" = "Worker-1" ]; then FREMD=Worker-2; else FREMD=Worker-1; fi

# ═════════════════════════════════════════════════════════════════════════
#  WELCHE EIGENSCHAFTEN GIBT ES ÜBERHAUPT ZU VARIIEREN (#1081, AC6)
# ═════════════════════════════════════════════════════════════════════════
#
# **Diese Liste stammt NICHT aus den Richtungen unten.** Käme sie von dort,
# enthielte sie per Bauart nur, was ohnehin geprüft wird — und meldete immer
# „nichts fehlt". Dieselbe Konstruktion wie `package.json` gegen `MUSTER` in
# #1059: eine zweite Quelle, die von der ersten nichts weiß.
#
# Die zweite Quelle sind hier die EINGÄNGE der geprüften Datei, abgelesen
# statt aufgezählt:
#
#     grep '\$\{\?[A-Z_]'       ROTA_FENSTER_STAND, HOME, PATH
#     grep 'stand.get'          inhaber, abgabe, zweck
#     grep -E '^\s*(if|elif) '  sechs Verzweigungen
#     der try/except um json.load
#
# In einer Sitzung sind DREIMAL grüne Richtungen an derselben Sache
# vorbeigelaufen: die erfundene PID 992, der relative Aufruf, die Umgebung
# ohne `rota-fenster`. Jedes Mal war die ZAHL der grünen Richtungen
# unauffällig. Es fehlte keine Richtung — es fehlte eine EIGENSCHAFT, die
# keine Richtung variiert hat.
EIGENSCHAFTEN="inhaber abgabe eigene-rolle umgebung stand-vorhanden stand-lesbar zweck-laenge"
VARIIERT=""

fehler=0
pruefe() {   # name  erwartung(warnt|still|unlesbar)  ausgabe  eigenschaft
  local name="$1" erwartet="$2" ausgabe="$3" eigenschaft="${4:-KEINE}"
  local hat="still"
  case "$ausgabe" in
    *MESSFENSTER*) hat="warnt";;
    *"nicht lesbar"*|*"unlesbar"*) hat="unlesbar";;
  esac
  VARIIERT="$VARIIERT $eigenschaft"
  if [ "$hat" = "$erwartet" ]; then
    printf "  %-36s %-8s  %-14s ok\n" "$name" "$hat" "$eigenschaft"
  else
    printf "  %-36s %-8s  %-14s FALSCH (erwartet %s)\n" \
           "$name" "$hat" "$eigenschaft" "$erwartet"
    fehler=1
  fi
}

echo "Eigene Rolle: $MEINE_ROLLE   Fremdrolle fuer die Probe: $FREMD"
echo "Wegwerf-Stand: $PROBESTAND"
echo

setze_stand "$FREMD" ""
A="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
setze_stand "$MEINE_ROLLE" ""
B="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
setze_stand "" ""
C="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
setze_stand "$FREMD" "22.08. 01:00:00"
D="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"

# Umgebung ohne das Werkzeug — dort gibt es kein Messfenster, also ist jede
# Meldung Rauschen. Der erste Entwurf hat hier bei JEDEM CI-Lauf angeschlagen.
setze_stand "$FREMD" ""
E="$(PATH=/usr/bin:/bin ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"

# Trägerzeile dazu: derselbe kurze PATH, aber mit Werkzeug — muss warnen.
# Sonst belegt das Schweigen oben nur, dass der kurze PATH etwas anderes
# kaputtmacht.
BINDIR="$(dirname "$(command -v rota-fenster)")"
F="$(PATH=/usr/bin:/bin:$BINDIR ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"

# **Sehr langer Zweck** — die Warnung kürzt ab 150 Zeichen. Der Zweig war bis
# zu AC6 ungeprüft: Keine Richtung variierte die Länge, also hätte ein Fehler
# beim Kürzen (Absturz, abgeschnittene Steuerzeichen) niemanden gestört.
LANG="$(python3 -c "print('X' * 400)")"
python3 - "$PROBESTAND" "$FREMD" "$LANG" <<'PYTHON'
import json, sys
json.dump({"inhaber": sys.argv[2], "zweck": sys.argv[3],
           "seit": "22.08. 00:00:00", "abgabe": None, "kennung": "x"},
          open(sys.argv[1], "w"))
PYTHON
I="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
if ! printf '%s' "$I" | grep -q '\.\.\.'; then
  echo "  FEHLER: ein 400-Zeichen-Zweck wurde NICHT gekuerzt — die Warnung" >&2
  echo "    sprengt dann jede Zeilenbreite und wird ueberlesen." >&2
  fehler=1
fi

# **Standdatei da, aber kaputt.** Der `except ValueError`-Zweig in der Warnung
# war ungeprüft. Er ist nicht dasselbe wie „Datei fehlt": Dort weiß man, dass
# nichts da ist — hier ist etwas da und unbrauchbar, was leichter für „leer"
# gehalten wird.
printf 'das ist kein JSON {{{' > "$PROBESTAND"
H="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
H_EXIT=$?
if [ "$H_EXIT" -ne 0 ]; then
  echo "  FEHLER: kaputte Standdatei endete mit Exit $H_EXIT — die Warnung" >&2
  echo "    darf NIE blockieren (Kriterium 5)." >&2
  fehler=1
fi

# Standdatei fehlt ganz (#1081, AC5). MUSS „nicht lesbar" melden und NICHT
# schweigen: Ein fehlender Messwert ist kein grüner Messwert.
rm -f "$PROBESTAND"
G="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
G_EXIT=$?

printf "  %-36s %-8s  %-14s\n" "FALL" "ERGEBNIS" "VARIIERT"
pruefe "fremdes Fenster, keine Abgabe" warnt "$A" inhaber
pruefe "eigenes Fenster" still "$B" eigene-rolle
pruefe "niemand haelt es" still "$C" inhaber
pruefe "fremdes Fenster, aber abgegeben" still "$D" abgabe
pruefe "Umgebung ohne rota-fenster (CI)" still "$E" umgebung
pruefe "kurzer PATH, aber Werkzeug da" warnt "$F" umgebung
pruefe "Standdatei fehlt ganz" unlesbar "$G" stand-vorhanden
pruefe "Standdatei da, aber kaputtes JSON" unlesbar "$H" stand-lesbar
pruefe "sehr langer Zweck wird gekuerzt" warnt "$I" zweck-laenge

if [ "$G_EXIT" -ne 0 ]; then
  echo "  FEHLER: der Fall 'Datei fehlt' endete mit Exit $G_EXIT — die" >&2
  echo "    Warnung darf NIE blockieren (Kriterium 5)." >&2
  fehler=1
fi

# --- Trägerzeile über alles ----------------------------------------------
#
# Ohne sie hiesse „fünfmal still" auch dann alles in Ordnung, wenn das Skript
# gar nichts ausgibt — etwa weil sein Pfad falsch ist. Dann wären fünf von
# sieben Richtungen grün.
if ! printf '%s' "$A" | grep -q "MESSFENSTER"; then
  echo
  echo "FEHLGESCHLAGEN: der EINE Fall, der warnen muss, hat geschwiegen." >&2
  echo "  Damit belegen die stillen Faelle NICHTS — ein Skript, das nie" >&2
  echo "  etwas sagt, besteht sie ebenfalls." >&2
  fehler=1
fi

# ═════════════════════════════════════════════════════════════════════════
#  WAS NICHT VARIIERT WURDE — die Auflage, die den Rest erst tragfähig macht
# ═════════════════════════════════════════════════════════════════════════
#
# Eine Liste, die nur aufzählt was DA ist, findet die fehlende Spalte nicht.
# Deshalb wird hier die Differenz gebildet: alle bekannten Eigenschaften minus
# die, die eine Richtung tatsächlich variiert hat.
#
# Das ist dieselbe Unterscheidung wie Anwesenheit gegen Wirksamkeit. Eine
# Richtung, die es gibt, sagt nichts über eine Eigenschaft, die sie nicht
# anfasst.
echo
FEHLEND=""
for e in $EIGENSCHAFTEN; do
  case " $VARIIERT " in
    *" $e "*) ;;
    *) FEHLEND="$FEHLEND $e";;
  esac
done
if [ -n "$FEHLEND" ]; then
  echo "  NICHT VARIIERT:$FEHLEND"
  echo
  echo "  Diese Eigenschaften liest die Warnung, aber keine Richtung aendert"
  echo "  sie. Ein Fehler darin faellt hier NICHT auf — die Zahl der gruenen"
  echo "  Richtungen bleibt dabei unauffaellig. Genau so sind am 2026-08-22"
  echo "  drei Luecken durchgerutscht (erfundene PID, relativer Aufruf,"
  echo "  Umgebung ohne Werkzeug)."
  echo
else
  echo "  Alle $(printf '%s' "$EIGENSCHAFTEN" | wc -w) bekannten Eigenschaften"
  echo "  werden von mindestens einer Richtung variiert."
fi

# **Und die Gegenprobe zur Gegenprobe**: Die Liste oben taugt nur, wenn sie
# eine Luecke auch MELDEN kann. Waere `EIGENSCHAFTEN` versehentlich leer oder
# identisch mit `VARIIERT`, saehe sie genauso aus wie ein vollstaendiger Lauf.
if [ -z "$(printf '%s' "$EIGENSCHAFTEN" | tr -d ' ')" ]; then
  echo "NICHT DURCHGEFUEHRT (Exit 2): die Eigenschaftsliste ist LEER — dann" >&2
  echo "  meldet der Abschnitt oben per Bauart nie eine Luecke." >&2
  exit 2
fi

if [ "$fehler" -ne 0 ]; then
  echo "GEGENPROBE ROT." >&2
  exit 1
fi
echo "BESTANDEN: warnt in drei Faellen, schweigt in vier, meldet in zwei"
echo "  'nicht lesbar'. Die vier stillen sind die wichtigeren — ein Melder,"
echo "  der immer anschlaegt, bestuende die drei ersten ebenfalls."
