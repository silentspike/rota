#!/usr/bin/env bash
# Buendelt das Pruefblatt der Speicherung (Issue #83, AC1 und AC2).
#
# Es laedt src/analysis/ unveraendert und laeuft gegen den ECHTEN
# localStorage - die Unit-Tests pruefen gegen eine nachgebaute Abbildung, und
# die beweist nicht, dass es im Browser ebenso laeuft.
#
# Kein Eintrag in vite.config.ts: Das Blatt gehoert nicht zur Auslieferung
# (GAME-DESIGN 19.3).
set -euo pipefail
cd "$(dirname "$0")/.."
npx esbuild docs/save-check/preview.ts \
  --bundle --format=iife --target=es2022 \
  --outfile=docs/save-check/preview.js
echo "docs/save-check/preview.js gebaut"
