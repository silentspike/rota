#!/usr/bin/env bash
# Buendelt die Pruefseite der Auswertung (Issue #81).
#
# Sie laedt src/analysis/ unveraendert - deshalb ein Buendel statt einer
# nachgebauten Seite. Kein Eintrag in vite.config.ts noetig: Die Seite gehoert
# nicht zur Auslieferung (GAME-DESIGN 19.3).
set -euo pipefail
cd "$(dirname "$0")/.."
npx esbuild docs/analysis-check/preview.ts \
  --bundle --format=iife --target=es2022 \
  --loader:.css=css \
  --outfile=docs/analysis-check/preview.js
echo "docs/analysis-check/preview.js gebaut"
