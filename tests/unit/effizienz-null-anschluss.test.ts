/**
 * Der Satz entsteht aus einem echten Kampf, nicht aus erfundenen Zahlen (#806).
 *
 * # Warum diese Datei neben der anderen steht
 *
 * `effizienz-erklaert-die-null.test.ts` prüft die Rechnung und die Ansicht mit
 * gesetzten Werten. Das ist richtig und reicht nicht: Zwischen `Combat` und der
 * Tabelle liegt `runDataFor`, und genau dort geht so etwas verloren. Ein Feld,
 * das niemand füllt, sieht in beiden Richtungen aus wie eines, das immer 0 ist.
 *
 * Dieser Fall spielt deshalb den Fehler nach — SMOULDER auf Abklingzeit
 * gedrückt, wie ein Anfänger es tut — und liest den Satz am Ende der Kette.
 */

import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { buildEfficiencyTable } from '../../src/analysis/efficiency.js';
import { runDataFor } from '../../src/analysis/run-analysis.js';

/**
 * Spielt 30 Sekunden und gibt die Tabellen-Zeile für SMOULDER.
 *
 * `aufAbklingzeit` ist der Unterschied zwischen dem Anfänger und dem Spieler,
 * der 7.2 verstanden hat — sonst ist alles gleich.
 */
function zeileFuerSmoulder(aufAbklingzeit: boolean) {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const smoulder = combat.indexOf('smoulder');
  const kindle = combat.indexOf('kindle');
  for (let t = 0; t <= 30_000; t += 100) {
    combat.stepPassive(t);
    if (!combat.gcd.isReady(t)) continue;
    const erneuern = aufAbklingzeit
      ? combat.isOffCooldown(alsSpellIndex(smoulder), t)
      : combat.isOffCooldown(alsSpellIndex(smoulder), t) &&
        combat.dotRemainingMs(smoulder, t) < 1500;
    if (erneuern) combat.cast(alsSpellIndex(smoulder), t);
    else if (combat.isOffCooldown(alsSpellIndex(kindle), t)) combat.cast(alsSpellIndex(kindle), t);
  }

  const spells = runDataFor(
    combat,
    {
      // Nur die zwei Felder, die `runDataFor` liest — der Rest der Zusammen-
      // fassung spielt für diese Frage keine Rolle.
      idlePerSpell: [],
    } as never,
    [smoulder, kindle],
  );
  const tabelle = buildEfficiencyTable({
    elapsedMs: 30_000,
    gcdMs: 1500,
    casts: spells.reduce((s, x) => s + x.casts, 0),
    damage: combat.damageDealt,
    lostTimeNothingReadyMs: 0,
    synergyUsed: 0,
    synergyAvailable: 0,
    spells,
  });
  return {
    zeile: tabelle.rows.find((r) => r.spellId === 'smoulder'),
    schaden: combat.damageDealt,
  };
}

describe('Der Weg vom Kampf bis zum Satz', () => {
  it('auf Abklingzeit gedrückt: null Schaden, und die Tabelle sagt warum', () => {
    const { zeile, schaden } = zeileFuerSmoulder(true);
    // **Die Zusicherung: der Fehler ist wirklich passiert.** Ohne sie prüfte
    // der Fall einen Satz zu einer Lage, die es gar nicht gab.
    expect(schaden, 'der Lauf teilt Schaden aus — dann ist es nicht der Fall aus #806').toBe(0);
    expect(zeile?.casts, 'SMOULDER wurde gar nicht gewirkt').toBeGreaterThan(0);
    expect(zeile?.damage).toBe(0);

    expect(zeile?.note, 'die Tabelle lässt die Null unerklärt').toBe(
      'renewed before its first tick — 100 ticks lost',
    );
  });

  it('richtig gespielt: Schaden da, und kein Satz nötig', () => {
    const { zeile, schaden } = zeileFuerSmoulder(false);
    expect(schaden, 'der gute Lauf teilt keinen Schaden aus — der Vergleich misst nichts').toBe(
      2400,
    );
    expect(zeile?.note, 'ein Satz, wo nichts zu erklären ist').toBeNull();
  });
});
