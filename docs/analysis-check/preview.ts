/**
 * Prüfseite für Issue #81 — sie zeigt die ECHTE Ansicht, nicht eine Nachbildung.
 *
 * Sie importiert `src/analysis/` unverändert; wäre die Tabelle hier noch einmal
 * gebaut, prüfte der Browser-Nachweis eine zweite Wahrheit statt der Sache.
 * Gebündelt mit `tools/analysis-preview.sh`.
 */
import { analyseLosses, type LossInput } from '../../src/analysis/losses.js';
import { buildAnalysisView } from '../../src/analysis/report-view.js';
import '../../src/analysis/report.css';

// Ein Durchlauf, wie er wirklich aussieht: vier Zauber, drei davon unterschiedlich
// gut ausgenutzt, dazu Leerlauf und verpasste Unterbrechungen.
const RUN: LossInput = {
  elapsedMs: 240_000,
  damage: 186_400,
  casts: 118,
  lostTimeMs: 6400,
  gcdMs: 1500,
  spells: [
    {
      spellId: 'kindle',
      name: 'Kindle',
      cooldownMs: 1500,
      casts: 92,
      damage: 92_000,
      nominalDamagePerCast: 1000,
    },
    {
      spellId: 'smoulder',
      name: 'Smoulder',
      cooldownMs: 12_000,
      casts: 15,
      damage: 63_000,
      nominalDamagePerCast: 4200,
    },
    {
      spellId: 'pyre',
      name: 'Pyre',
      cooldownMs: 30_000,
      casts: 4,
      damage: 24_000,
      nominalDamagePerCast: 6000,
    },
    {
      spellId: 'quench',
      name: 'Quench',
      cooldownMs: 45_000,
      casts: 1,
      damage: 7400,
      nominalDamagePerCast: 7400,
    },
  ],
  synergyUsed: 7,
  synergyAvailable: 10,
  synergyBonusDamage: 900,
  interruptsMissed: 2,
  interruptDamage: 7000,
};

const root = document.querySelector('#view');
if (root !== null) root.appendChild(buildAnalysisView(analyseLosses(RUN)));
