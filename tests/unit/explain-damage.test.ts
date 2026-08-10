/**
 * `explainDamage` zeigt die echten Faktoren, nicht acht Einsen (Issue #421b).
 *
 * # Was hier bewacht wird und warum es keinen Wächter hatte
 *
 * Bis zum 2026-08-10 stand in `explainDamage` eine Liste aus **Konstanten**:
 * fünf von sechs Einträgen fest auf `1`, nur `resistance` gelesen. Ein
 * Werkzeug, das nur von Menschen aufgerufen wird, hatte keinen einzigen
 * Prüffall — und war deshalb ein Jahr lang falsch, ohne dass es auffiel.
 *
 * **Was das gekostet hat:** Bei der Messung der Ausrüstungs-Kette im Browser
 * zeigte es mit angelegtem Gegenstand `essence: 1`, während derselbe Wurf von
 * 100 auf 221 Schaden stieg. Das sah nach dem Befund „die Ausrüstung kommt im
 * Spiel nicht an" aus — der wäre falsch gewesen. Gefangen hat es nur, dass
 * `setItems` daneben `essence: 474` zurückgab: zwei Auskünfte desselben
 * Programms, die sich widersprachen.
 *
 * # Die Zahlen sind gemessen, nicht gesetzt
 *
 * `focus-drive` auf Stufe 40 trägt `essence 474` — im Browser abgelesen und
 * hier an derselben Kette nachgerechnet. Der Faktor ist `1 + 474/1000`.
 */
import { describe, expect, it } from 'vitest';

import { damageFactorList } from '../../src/devtools/index.js';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { wornFrom } from '../../src/gear/catalog.js';
import { NEUTRAL_FACTORS } from '../../src/core/damage.js';

/** Ein Kampf mit dem angegebenen Stück auf seinem Platz. */
function kampfMit(itemId: string, upgradeSteps = 0): Combat {
  return new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    worn: wornFrom([{ itemId, upgradeSteps, equippedSlot: 0 }]),
  });
}

function faktor(combat: Combat, name: string): number {
  const eintrag = damageFactorList(combat.damageFactors).find((f) => f.name === name);
  expect(eintrag, `der Aufriss kennt den Faktor "${name}" nicht`).toBeDefined();
  return eintrag?.value ?? Number.NaN;
}

describe('Der Aufriss liest die echten Faktoren', () => {
  it('zeigt mit angelegtem Gegenstand einen essence-Faktor ÜBER 1', () => {
    // **Der Rückfall-Wächter.** Wer hier wieder `{ name: 'essence', value: 1 }`
    // einträgt, macht diese Zeile rot. Ohne sie stünde in einem Monat wieder
    // eine Konstante da, und niemand merkte es — das Werkzeug wird von
    // Menschen gelesen, nicht von Tests.
    const combat = kampfMit('focus-drive');
    expect(combat.wornValues.essence, 'focus-drive trägt kein ESSENCE').toBe(474);
    expect(faktor(combat, 'essence')).toBeCloseTo(1.474, 10);
  });

  it('GEGENPROBE: ohne Gegenstand steht dort 1 — und zwar zu Recht', () => {
    // Ohne diese Zeile wäre ein Aufriss, der IMMER etwas über 1 zeigt,
    // ebenfalls grün. Die Eins ist hier kein Platzhalter, sondern das
    // Ergebnis: `essenceFactor(NOTHING_WORN)` ist exakt 1,0.
    const nackt = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    expect(nackt.wornValues.essence).toBe(0);
    expect(faktor(nackt, 'essence')).toBe(1);
  });

  it('folgt der Aufwertung — mehr Stufen, größerer Faktor', () => {
    // Im Browser gemessen: upgradeSteps 10 hebt ESSENCE von 474 auf 769.
    const ohne = kampfMit('focus-drive', 0);
    const mit = kampfMit('focus-drive', 10);
    expect(mit.wornValues.essence).toBe(769);
    expect(faktor(mit, 'essence')).toBeGreaterThan(faktor(ohne, 'essence'));
  });

  it('unterscheidet die beiden Ausrichtungen', () => {
    // `-mix` trägt kein ESSENCE als Affix, nur den Grundwert jedes
    // Gegenstands. `-drive` tauscht YIELD dagegen ein.
    expect(faktor(kampfMit('focus-mix'), 'essence')).toBeLessThan(
      faktor(kampfMit('focus-drive'), 'essence'),
    );
  });
});

describe('Der Aufriss ist vollständig', () => {
  it('führt ALLE acht Faktoren, nicht sechs', () => {
    // **Die alte Liste ließ `gearEffects` und `vulnerability` aus.** Sie war
    // nicht nur falsch, sondern unvollständig: `computeDamage` multipliziert
    // acht, und wer sechs zeigt, erklärt einen anderen Schaden als den
    // gefallenen.
    //
    // Als Liste festgenagelt und nicht als Zahl: „acht von acht" bliebe grün,
    // wenn zwei vertauscht würden.
    expect(damageFactorList(NEUTRAL_FACTORS).map((f) => f.name)).toEqual([
      'essence',
      'talents',
      'gearEffects',
      'buffs',
      'synergy',
      'resistance',
      'vulnerability',
      'crit',
    ]);
  });

  it('führt sie in der Reihenfolge, in der sie angewandt werden', () => {
    // Die Reihenfolge ist laut GAME-DESIGN 6.3 eine Festlegung des
    // Design-Dokuments — Fließkomma-Multiplikation ist nicht assoziativ. Ein
    // Aufriss in anderer Reihenfolge erklärte eine andere Rechnung.
    const namen = damageFactorList(NEUTRAL_FACTORS).map((f) => f.name);
    expect(namen.indexOf('essence')).toBeLessThan(namen.indexOf('buffs'));
    expect(namen.indexOf('buffs')).toBeLessThan(namen.indexOf('resistance'));
    expect(namen.indexOf('resistance')).toBeLessThan(namen.indexOf('crit'));
  });

  it('gibt bei neutralen Faktoren überall 1 — die Nullmessung', () => {
    for (const eintrag of damageFactorList(NEUTRAL_FACTORS)) {
      expect(eintrag.value, `${eintrag.name} ist neutral nicht 1`).toBe(1);
    }
  });
});
