import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { recordings, replayRecording } from '../../src/devtools/recording.js';

/**
 * Eine Aufzeichnung mit ABWEICHENDEN Bindungen spielt im Nachlauf denselben
 * Kampf wie im Direktlauf — Issue #663.
 *
 * # Der Befund, den dieser Fall festnagelt
 *
 * `replayRecording` löste einen Platz über `spellForSlot` auf, und dessen
 * dritter Parameter hatte einen Default: `= STEP1_BINDINGS`. Lieferte die
 * Aufzeichnung keine Belegung, griff dieser Default — und spielte einen
 * anderen Kampf:
 *
 * ```text
 *   Platz 0   new-calcination meint searing    STEP1 liefert kindle
 *   Platz 1   new-calcination meint bellows    STEP1 liefert smoulder
 *
 *   Direktlauf                     1860 Schaden
 *   Nachlauf ohne Bindungen         660 Schaden
 * ```
 *
 * **Es fiel nichts um.** Keine Ausnahme, kein roter Fall — nur eine kleinere
 * Zahl, die plausibel aussah. Gefunden wurde es, weil eine Umstellung von
 * `combat.*` auf `result.*` (Issue #653) den Wert plötzlich in eine
 * Vergleichsdatei schrieb.
 *
 * # Warum die Zahl hart hier steht
 *
 * Läse dieser Fall den Sollwert aus derselben Quelle wie den Istwert, wäre er
 * gegen genau diesen Fehler blind: Beide Seiten kämen aus dem Nachlauf und
 * wären sich einig, auch wenn beide falsch sind. Die 1860 stammt aus dem
 * Direktlauf über `Combat` — einem Weg, der `spellForSlot` nie berührt.
 */

const TICK_MS = 10;

/** Der Kampf, wie ihn `payloads.ts` direkt über `Combat` fährt. */
function direktlauf(presses: readonly { tick: number; slot: number }[]): number {
  const boss = FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const searing = combat.indexOf('searing');
  const bellows = combat.indexOf('bellows');
  let pressIndex = 0;
  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    while (pressIndex < presses.length) {
      const press = presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      combat.cast(press.slot === 0 ? searing : bellows, timeMs);
      pressIndex++;
    }
  }
  return combat.damageDealt;
}

describe('#663 — eine abweichende Belegung überlebt den Nachlauf', () => {
  const aufzeichnung = recordings().get('new-calcination');

  /**
   * **Die Zusicherung vor der Messung.** Trägt die Aufzeichnung keine
   * Bindungen, misst der Fall darunter den alten Zustand und wäre grün mit
   * der Aussage „es stimmt" — obwohl beide Seiten dann denselben falschen
   * Kampf spielen.
   */
  it('die Aufzeichnung führt ihre Bindungen überhaupt mit', () => {
    expect(aufzeichnung, 'die Aufzeichnung new-calcination fehlt').toBeDefined();
    expect(
      aufzeichnung?.bindings,
      'ohne Bindungen fällt der Nachlauf auf STEP1_BINDINGS zurück — genau der Befund aus #663',
    ).toBeDefined();
  });

  it('und zwar die gemeinten: searing auf Platz 0, bellows auf Platz 1', () => {
    const nach = new Map(aufzeichnung?.bindings?.map((b) => [b.slot, b.spellId]));
    expect(nach.get(0), 'Platz 0 ist nicht searing — STEP1 bände hier kindle').toBe('searing');
    expect(nach.get(1), 'Platz 1 ist nicht bellows — STEP1 bände hier smoulder').toBe('bellows');
  });

  it('AC3 — der Nachlauf richtet dieselben 1860 an wie der Direktlauf', () => {
    const direkt = direktlauf(aufzeichnung?.presses ?? []);
    expect(direkt, 'der Direktlauf richtet nicht die 1860 an, gegen die hier gemessen wird').toBe(
      1860,
    );

    const nachlauf = replayRecording(
      aufzeichnung ??
        (() => {
          throw new Error('Aufzeichnung fehlt');
        })(),
      { boss: FIRST_CRUCIBLE.value },
    );
    expect(
      nachlauf.damage,
      'der Nachlauf spielt einen anderen Kampf als der Direktlauf — 660 war der Wert, ' +
        'als die Bindungen fehlten und STEP1_BINDINGS einsprang',
    ).toBe(direkt);
  });

  /**
   * AC4 — die Gegenprobe. Ohne Bindungen ist der Nachlauf heute nicht mehr
   * still falsch, sondern laut: `replayRecording` wirft. Vorher lief er
   * durch und lieferte 660.
   */
  it('AC4 — ohne Bindungen wirft der Nachlauf, statt still 660 zu liefern', () => {
    const ohne = { ...aufzeichnung, bindings: undefined } as unknown as Parameters<
      typeof replayRecording
    >[0];
    expect(
      () => replayRecording(ohne, { boss: FIRST_CRUCIBLE.value }),
      'ohne Bindungen läuft der Nachlauf durch — dann ist der stille Rückfall wieder da',
    ).toThrow(/keine bindings/);
  });
});
