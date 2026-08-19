#!/usr/bin/env bash
#
# STAND 2026-08-19, 11:08 — AUSGEFÜHRT UND GEGENGEPROBT.
#
# Gefahren gegen dieselben Binaries und dasselbe Log wie die letzte Messung der
# alten Fassung, damit eine Abweichung dem Umbau zuzuordnen wäre und nicht
# einem veränderten Bestand:
#
#     206 von 206 Suiten, 1038 bestanden, 6 ignoriert, 0 rot, Exit 0
#     — identisch mit dem vorab festgelegten Stand.
#
# Und die neue Zusicherung „ohne Ergebniszeile" schlägt nachweislich an: Ein Log
# mit dem regulären Beispiel `wertungsmenge-981` (druckt eine Tabelle, Exit 0,
# keine Testzeile) ergibt „Ohne Ergebniszeile: 1" und Exit 1. Die alte Fassung
# hätte dasselbe Ziel als null bestandene Tests aufaddiert und weitergezählt.
#
# Führt die von `cargo remote -- test --no-run` gebauten Testbinaries LOKAL aus.
#
# Der Weg stammt aus der Klarstellung des Auftraggebers (Nachricht 8258):
# rustbuild baut, die Zielmaschine führt aus. Ein fertiges Binary zu starten
# ist kein cargo-Aufruf.
#
# Aufruf:  bash tools/tests-lokal.sh <norun-log> [worktree-wurzel]
#
# Ohne zweites Argument wird die Wurzel aus dem Aufrufort abgeleitet — das
# Skript liegt im Repository, also gilt sein eigenes Verzeichnis. Bis zum
# 2026-08-19 war der Pfad fest auf e9 verdrahtet und die Datei lag in
# /work/tmp: Wer sie brauchte, musste sie kopieren und anpassen, und die
# Fassung mit den teuer gelernten Korrekturen darin war nur einem zugaenglich.
set -u
LOG=${1:?Log von 'cargo remote -c -- test --no-run' angeben}
WURZEL=${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}
[ -d "$WURZEL/sim" ] || { echo "Keine sim/ unter $WURZEL — falsche Wurzel?"; exit 1; }
echo "Wurzel: $WURZEL"
cd "$WURZEL" || exit 1

# Die Pfade stehen im Log als "Executable src/lib.rs (target/debug/deps/name-hash)".
#
# **`deps/` UND `examples/`.** Bis zum 2026-08-19 stand hier nur `deps/`, und
# damit fielen bei `--all-targets` still 31 Ziele heraus — darunter zwei mit
# zusammen 17 Prueffaellen (`messumgebung`, `trace_stabilitaet`), die dieses
# Skript nie ausgefuehrt hat. Aufgefallen ist es an einer Differenz in fremden
# Zahlen: 202 Ziele beim Orchestrator gegen 171 bei mir, 171 + 31 = 202.
#
# Die Meldung "171 von 171 Suiten" war dabei jedes Mal wahr — sie zaehlte nur
# eine Menge, die das Skript selbst zugeschnitten hatte.
mapfile -t BINARIES < <(tr '\r' '\n' < "$LOG" \
  | grep -oE '\(target/debug/(deps|examples)/[^)]+\)' \
  | tr -d '()' \
  | sort -u)

echo "Aus dem Log gelesen: ${#BINARIES[@]} Testbinaries"
if [ "${#BINARIES[@]}" -eq 0 ]; then
  echo "KEIN BINARY GEFUNDEN — der Lauf hat nicht stattgefunden."
  exit 1
fi

gesamt=0
ignoriert=0
rote=0
stumm=0
fehlend=0
suiten=0
ROT_LISTE=""

for b in "${BINARIES[@]}"; do
  if [ ! -x "$b" ]; then
    fehlend=$((fehlend + 1))
    echo "  FEHLT: $b"
    continue
  fi
  # **Aus sim/ heraus, nicht aus der Wurzel.** `cargo test` setzt das
  # Arbeitsverzeichnis auf das Paket; eine eigene Schleife tut das nicht, und
  # jeder Test mit einem Pfad wie "../data/spells" trifft dann ins Leere.
  # Gemessen: fünf Suiten scheiterten allein daran.
  # **stdout und stderr GETRENNT, nicht zusammengeführt** (2026-08-19).
  #
  # Bis heute stand hier `2>&1`, und die Zählung darunter lief über beide
  # Ströme. Das ist ein aggregierender Filter auf einem Strom mit
  # Fehlermeldungen darin: Eine Störung wird dort nicht verschluckt, sondern
  # **mitgezählt** — und eine falsche Zahl sieht nach einer Messung aus,
  # während eine fehlende Zeile nur fehlt.
  #
  # Getrennt, nicht weggeworfen: In stderr stehen die Panics, also der Grund,
  # warum ein Ziel rot war. Zwei Ströme, zwei Zwecke — der eine wird gezählt,
  # der andere gelesen.
  aus=$(mktemp /work/tmp/testlauf-XXXXXX.out)
  err=$(mktemp /work/tmp/testlauf-XXXXXX.err)
  (cd sim && "../$b" >"$aus" 2>"$err")
  code=$?
  suiten=$((suiten + 1))
  # "test result: ok. 12 passed; 0 failed; 3 ignored; ..."
  #
  # **`[a-zA-Z]+\.` statt `ok\.`** — sonst verschluckt das Muster die
  # BESTANDENEN Tests einer ROTEN Suite. Solange alles grün ist, fällt das
  # nicht auf; es wird erst falsch, wenn etwas rot ist, also genau dann, wenn
  # man die Zahl braucht.
  n=$(grep -oE 'test result: [a-zA-Z]+\. [0-9]+ passed' "$aus" | grep -oE '[0-9]+' | head -1)
  f=$(grep -oE '[0-9]+ failed' "$aus" | grep -oE '[0-9]+' | head -1)
  # Ignorierte gehören in den Bericht: Ein übersprungener Test ist weder
  # bestanden noch durchgefallen, und ihn unter „bestanden" zu verbuchen wäre
  # dieselbe Klasse Fehler wie ein zu enges grep-Muster.
  i=$(grep -oE '[0-9]+ ignored' "$aus" | grep -oE '[0-9]+' | head -1)
  gesamt=$((gesamt + ${n:-0}))
  ignoriert=$((ignoriert + ${i:-0}))
  # Eine Suite, die lief und KEINE Ergebniszeile hinterliess, ist kein grüner
  # Lauf — sie ist ein Lauf, der nicht stattgefunden hat. Ohne diese Zeile
  # verschwindet so ein Ziel lautlos in einer Summe über zweihundert.
  if [ -z "$n" ]; then
    stumm=$((stumm + 1))
    echo "  OHNE ERGEBNISZEILE: $b (Exit $code)"
    head -2 "$err" | sed 's/^/      /'
  fi
  if [ "$code" -ne 0 ]; then
    rote=$((rote + 1))
    ROT_LISTE="$ROT_LISTE\n  $b (Exit $code, ${f:-?} rot)"
    # Die gescheiterten Fälle stehen auf stdout, die Panics auf stderr —
    # beide gehören in den Bericht, deshalb beide Dateien.
    grep -E "^test .*FAILED" "$aus" | head -3 | sed 's/^/      /'
    grep -E "panicked at" "$err" | head -2 | sed 's/^/      /'
  fi
  rm -f "$aus" "$err"
done

echo
echo "========================================================================"
echo "  Suiten ausgefuehrt:   $suiten von ${#BINARIES[@]}"
echo "  Tests bestanden:      $gesamt"
echo "  Tests ignoriert:      $ignoriert"
echo "  Suiten mit Fehlern:   $rote"
echo "  Ohne Ergebniszeile:   $stumm"
echo "  Binaries fehlten:     $fehlend"
if [ "$rote" -gt 0 ]; then
  echo -e "  ROT:$ROT_LISTE"
fi
if [ "$stumm" -gt 0 ]; then
  echo "  ACHTUNG: $stumm Suite(n) ohne Ergebniszeile — nicht gelaufen ist KEIN PASS."
fi
echo "========================================================================"

# Ein Lauf ohne Tests ist ein Fehlschlag, kein grüner Lauf.
if [ "$gesamt" -eq 0 ]; then
  echo "NULL TESTS AUSGEFUEHRT — das gilt als FEHLGESCHLAGEN."
  exit 1
fi
[ "$rote" -eq 0 ] && [ "$fehlend" -eq 0 ]
