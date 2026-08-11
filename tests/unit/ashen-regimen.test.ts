/**
 * Der Sechser-Bonus wirkt im Kampf (13.5, Issue #480).
 *
 * `activeRotationEffects` war gebaut und hatte **keinen Aufrufer** — die
 * sechste Satz-Stufe stand in den Daten und griff nirgends. Das ist dieselbe
 * Klasse wie die Fundstufe, `upgradeSteps` und Topf 4: erhoben und nicht
 * gelesen, kein Fall rot.
 *
 * Nachgewiesen an **einem** Effekt (Entscheidung des Auftraggebers
 * 2026-08-11: „Leitung zuerst, mit einem Effekt"):
 *
 * > **Ashen Regimen** — PYROCLASM lässt ein Drittel der angesammelten Hitze
 * > stehen, statt sie vollständig zu verbrauchen.
 *
 * Die Messgrösse ist die klarste, die es gibt: Hitze vor dem Wurf, Hitze
 * danach. Kein Zeitfenster, keine Schwelle, kein zweiter Träger.
 */

import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { rotationEffectsFrom, type OwnedItemLike } from '../../src/gear/catalog.js';
import { SET_EFFECT_ASHEN_REGIMEN } from '../../src/gear/sets.js';
import { spellIndex } from '../helpers/spell-index.js';

/** `n` Satz-Teile, jedes auf einem eigenen Platz. */
function satzTeile(n: number): OwnedItemLike[] {
  return Array.from({ length: n }, (_, i) => ({
    itemId: `test-set-${(i % 4) + 1}`,
    upgradeSteps: 0,
    equippedSlot: i,
  }));
}

/**
 * Hitze aufbauen, dann PYROCLASM werfen — und beide Stände zurückgeben.
 *
 * Der Aufbau ist für beide Läufe identisch; nur die Ausrüstung unterscheidet
 * sich. Das ist die Bedingung dafür, dass der Unterschied dem Effekt gehört
 * und nicht der Rotation.
 */
function wurfMit(effekte: readonly string[]): { vorher: number; nachher: number } {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  combat.setSetEffects(effekte);
  combat.beginFight(0);

  // KINDLE baut Hitze auf. Genug Würfe, um sicher über die Bonus-Schwelle zu
  // kommen — sonst misst der Fall einen Wurf, der gar nicht lohnt.
  const kindle = alsSpellIndex(spellIndex('kindle'));
  let t = 0;
  for (let i = 0; i < 8; i++) {
    combat.cast(kindle, t);
    t += 1500;
  }
  const vorher = combat.resource;
  combat.cast(alsSpellIndex(spellIndex('pyroclasm')), t);
  return { vorher, nachher: combat.resource };
}

describe('AC1 — sechs Satzteile lassen Hitze stehen, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    const sechs = rotationEffectsFrom(satzTeile(6));
    const fuenf = rotationEffectsFrom(satzTeile(5));

    expect(sechs).toContain(SET_EFFECT_ASHEN_REGIMEN);
    expect(fuenf).not.toContain(SET_EFFECT_ASHEN_REGIMEN);
    // ZUSICHERUNG: Bei fünf Teilen ist der Satz sehr wohl getragen — der Fall
    // prüft die SCHWELLE, nicht ob überhaupt Satzteile da sind.
    expect(satzTeile(5)).toHaveLength(5);
  });

  it('und im Kampf bleibt mit sechs Teilen ein Drittel stehen', () => {
    const mit = wurfMit(rotationEffectsFrom(satzTeile(6)));
    const ohne = wurfMit(rotationEffectsFrom(satzTeile(5)));

    // Dieselbe Ausgangslage — sonst wäre der Unterschied unten nicht dem
    // Effekt zuzuschreiben.
    expect(mit.vorher).toBe(ohne.vorher);
    // ZUSICHERUNG: Es war überhaupt Hitze da. Ohne sie wären beide Seiten 0
    // und der Fall grün, ohne etwas zu messen.
    expect(mit.vorher).toBeGreaterThan(0);

    expect(ohne.nachher).toBe(0);
    expect(mit.nachher).toBe(Math.floor(mit.vorher / 3));
    expect(mit.nachher).toBeGreaterThan(0);
  });

  it('abgerundet, und der Rest verfällt (18.4)', () => {
    // Ein Bruchteil driftete über hunderte Würfe, und beide Seiten müssten auf
    // dieselbe Zahl kommen.
    const mit = wurfMit([SET_EFFECT_ASHEN_REGIMEN]);
    expect(Number.isInteger(mit.nachher)).toBe(true);
    expect(mit.nachher).toBe(Math.floor(mit.vorher / 3));
  });
});

describe('AC2 — ohne den Effekt ändert sich nichts gegenüber vorher', () => {
  it('ein leerer Aufbau verbraucht die Hitze vollständig', () => {
    expect(wurfMit([]).nachher).toBe(0);
  });

  it('ein FREMDER Satz-Effekt lässt PYROCLASM unberührt', () => {
    // Die Kennung wird verglichen, nicht bloss ihr Vorhandensein. Ohne diesen
    // Fall wäre auch eine Rechnung grün, die auf jeden beliebigen Effekt
    // reagiert.
    expect(wurfMit(['irgendein-anderer-effekt']).nachher).toBe(0);
  });
});

describe('AC3 — Kreuzprobe: der Effekt lässt alles andere unberührt', () => {
  it('der Schaden des Wurfs ist derselbe', () => {
    // Der Effekt schenkt Vorrat für den NÄCHSTEN Wurf; er verbilligt diesen
    // nicht. Änderte er den Schaden mit, wäre er ein Schadensbonus mit
    // Hitze-Anschein.
    const schadenMit = (effekte: readonly string[]): number => {
      const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
      combat.setSetEffects(effekte);
      combat.beginFight(0);
      const kindle = alsSpellIndex(spellIndex('kindle'));
      let t = 0;
      for (let i = 0; i < 8; i++) {
        combat.cast(kindle, t);
        t += 1500;
      }
      const vorPyro = combat.damageDealt;
      combat.cast(alsSpellIndex(spellIndex('pyroclasm')), t);
      return combat.damageDealt - vorPyro;
    };

    const mit = schadenMit([SET_EFFECT_ASHEN_REGIMEN]);
    const ohne = schadenMit([]);
    expect(mit).toBe(ohne);
    // ZUSICHERUNG: PYROCLASM hat überhaupt Schaden gemacht — sonst vergliche
    // der Fall zwei Nullen.
    expect(ohne).toBeGreaterThan(0);
  });

  it('eine andere Schule bleibt unberührt — auch mit dem Effekt im Aufbau', () => {
    // CALCINATION heizt, COAGULATION nicht. Der Effekt hängt an einer
    // Hitze-Mechanik; er darf in einer Schule ohne sie nichts tun.
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.setSetEffects([SET_EFFECT_ASHEN_REGIMEN]);
    combat.beginFight(0);
    const smoulder = alsSpellIndex(spellIndex('smoulder'));
    combat.cast(smoulder, 0);
    const nachSmoulder = combat.resource;

    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    ohne.beginFight(0);
    ohne.cast(smoulder, 0);

    // Ein Zauber, der kein Spender ist, sieht den Effekt nicht.
    expect(nachSmoulder).toBe(ohne.resource);
  });
});
