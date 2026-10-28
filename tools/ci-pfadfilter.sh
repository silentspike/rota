#!/bin/bash
# Welche geänderte Datei löst welche CI-Stufe aus? (Issue #1100, AC4)
#
# Die Muster werden AUS .github/workflows/ci.yml GELESEN, nicht abgetippt. Eine
# Abschrift kann nicht bemerken, dass sie veraltet ist — genau der Grund, aus
# dem tests/unit/ci-sim-filter.test.ts existiert.
#
#   bash tools/ci-pfadfilter.sh              # die Standardliste
#   bash tools/ci-pfadfilter.sh pfad/x.ts …  # eigene Pfade
#
# ACHTUNG: Dieses Skript fährt selbst keine Stufe — es fällt unter denselben
# Befund wie die 20 anderen unter tools/ (#1100). Wer es ändert, prüft es von
# Hand.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 9
CI=.github/workflows/ci.yml

SIM_MUSTER=$(grep -oE "match '[^']*'" "$CI" | head -1 | sed "s/^match '//; s/'$//")
DOC_MUSTER=$(grep -oE "grep -qvE '[^']*'" "$CI" | head -1 | sed "s/^grep -qvE '//; s/'$//")

# KONTROLLZEILE. Ein leeres Muster liefert lauter `false` und sähe aus wie ein
# Ergebnis: Jede Datei stünde dann als reine Dokumentation da. Beim ersten Bau
# dieses Skripts ist genau das passiert — der Regex für das Doku-Muster traf
# nicht, und die Tabelle war vollständig und vollständig falsch.
[ -n "$SIM_MUSTER" ] || { echo "FEHLER: sim-Muster nicht aus $CI gelesen"; exit 2; }
[ -n "$DOC_MUSTER" ] || { echo "FEHLER: doc-Muster nicht aus $CI gelesen"; exit 2; }

echo "sim-Muster aus $CI:"
echo "    $SIM_MUSTER"
echo "doc-Muster aus $CI:"
echo "    $DOC_MUSTER"
echo

match() { printf '%s\n' "$1" | grep -qE "$SIM_MUSTER" && echo true || echo false; }
nondoc() { printf '%s\n' "$1" | grep -qvE "$DOC_MUSTER" && echo true || echo false; }

STANDARD=(
  docs/GAME-DESIGN.md
  docs/groessen-abdeckung-messung.txt
  README.md
  src/core/combat.ts
  tests/unit/x.test.ts
  tests/parity/payloads.ts
  tests/parity/recordings/x.json
  sim/src/fight.rs
  sim/tests/x.rs
  sim/build.rs
  data/spells/calcination.json
  data/bosses/boss-1-1.json
  Cargo.toml
  rust-toolchain.toml
  .cargo-remote.toml
  tools/step1-check.py
  tools/groessen-hebel.py
  tools/rust-pin.py
  tools/wirkung-baubruch-gegenprobe.py
  .github/workflows/ci.yml
  .github/workflows/nightly.yml
  .github/workflows/meta.yml
  package.json
  vite.config.ts
  assets/sfx/x.mp3
)
DATEIEN=("$@")
[ ${#DATEIEN[@]} -eq 0 ] && DATEIEN=("${STANDARD[@]}")

printf '%-40s %-6s %-6s  %s\n' "geänderte Datei" "code" "sim" "zusätzliche Stufen"
printf '%-40s %-6s %-6s  %s\n' "$(printf '%.0s-' {1..40})" "------" "------" "------------------"
for f in "${DATEIEN[@]}"; do
  c=$(nondoc "$f")
  s=$(match "$f")
  zusatz=""
  [ "$c" = true ] && zusatz="typecheck unit build parity"
  [ "$s" = true ] && zusatz="$zusatz sim"
  [ -z "$zusatz" ] && zusatz="— (nur die immer laufenden)"
  printf '%-40s %-6s %-6s  %s\n' "$f" "$c" "$s" "$zusatz"
done

echo
echo "IMMER, ohne Filter: changes, lint, actionlint, secret-scan, ci-pass"
echo "  lint hat bewusst keinen Filter (Kommentar in $CI): 'npm run lint' fährt"
echo "  prettier über ALLE Dateien, auch Markdown."
