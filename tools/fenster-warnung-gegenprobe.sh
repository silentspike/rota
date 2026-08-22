#!/usr/bin/env bash
# Gegenprobe zu tools/fenster-warnung.sh (Issue #1049, Fertig-Kriterium 3).
#
# Prüft alle Zustände an einem WEGWERF-Schalterstand, in derselben Ablesung:
#
#     fremdes Fenster gehalten          ->  warnt
#     eigenes Fenster gehalten          ->  schweigt
#     Fenster frei                      ->  schweigt
#     fremdes Fenster, aber abgegeben   ->  schweigt
#     Umgebung ohne rota-fenster (CI)   ->  schweigt
#     Standdatei fehlt ganz             ->  meldet „nicht lesbar", nicht „frei"
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

MEINE_ROLLE="$(rota-ipc wer 2>/dev/null | awk '/^Rolle:/ {print $2; exit}')"
if [ -z "$MEINE_ROLLE" ]; then
  echo "FEHLGESCHLAGEN: eigene Rolle nicht ermittelbar. Ohne sie kann die" >&2
  echo "  Probe 'eigenes Fenster' nicht von 'fremdes Fenster' trennen —" >&2
  echo "  sie wuerde dann zwei Richtungen gleich behandeln und trotzdem" >&2
  echo "  gruen melden." >&2
  exit 1
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

fehler=0
pruefe() {   # name  erwartung(warnt|still|unlesbar)  ausgabe
  local name="$1" erwartet="$2" ausgabe="$3"
  local hat="still"
  case "$ausgabe" in
    *MESSFENSTER*) hat="warnt";;
    *"nicht lesbar"*) hat="unlesbar";;
  esac
  if [ "$hat" = "$erwartet" ]; then
    printf "  %-36s %-8s  ok\n" "$name" "$hat"
  else
    printf "  %-36s %-8s  FALSCH (erwartet %s)\n" "$name" "$hat" "$erwartet"
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

# Standdatei fehlt ganz (#1081, AC5). MUSS „nicht lesbar" melden und NICHT
# schweigen: Ein fehlender Messwert ist kein grüner Messwert.
rm -f "$PROBESTAND"
G="$(ROTA_FENSTER_STAND="$PROBESTAND" bash "$WARNUNG" 2>&1)"
G_EXIT=$?

pruefe "fremdes Fenster, keine Abgabe" warnt "$A"
pruefe "eigenes Fenster" still "$B"
pruefe "niemand haelt es" still "$C"
pruefe "fremdes Fenster, aber abgegeben" still "$D"
pruefe "Umgebung ohne rota-fenster (CI)" still "$E"
pruefe "kurzer PATH, aber Werkzeug da" warnt "$F"
pruefe "Standdatei fehlt ganz" unlesbar "$G"

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

echo
if [ "$fehler" -ne 0 ]; then
  echo "GEGENPROBE ROT." >&2
  exit 1
fi
echo "BESTANDEN: warnt in zwei Faellen, schweigt in vier, meldet in einem"
echo "  'nicht lesbar'. Die vier stillen sind die wichtigeren — ein Melder,"
echo "  der immer anschlaegt, bestuende die zwei ersten ebenfalls."
