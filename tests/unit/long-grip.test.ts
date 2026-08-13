/**
 * `The Long Grip` — der erste Kontroll-Ausgeber nach jedem Phasenwechsel
 * kostet nichts (13.5, Issue #480).
 *
 * Der fünfte der sechs Rotations-Effekte (FIXATION-Sechser). Er belohnt, den
 * Moment zu treffen, statt Punkte zu horten.
 *
 * # „Kostet nichts" heisst: wird nicht abgezogen, nicht: braucht nichts
 *
 * `combat.ts` klemmt den Preis ausdrücklich auf mindestens 1, mit der
 * Begründung: „Ein Ausgeber, der nichts kostet, wäre keiner, und
 * `controlValue >= 0` wäre bei Preis 0 immer wahr."
 *
 * Der Effekt hebt deshalb den **Abzug** auf, nicht die **Voraussetzung** — man
 * braucht die Punkte weiterhin. Ein Effekt, der die Prüfung übersprünge,
 * umginge die Klemmung, statt sie zu berücksichtigen. Die Auslegung steht hier,
 * damit ihr widersprochen werden kann.
 */

import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { activeRotationEffects, SETS, SET_COUNT } from '../../src/gear/sets.js';
import { SET_EFFECT_LONG_GRIP, SET_EFFECT_WHOLE_WORK } from '../../src/gear/sets.js';
import { spellIndex } from '../helpers/spell-index.js';

const GRIP_SET = SETS.find((s) => s.tiers.some((t) => t.effect === SET_EFFECT_LONG_GRIP))?.id;

function teile(set: number, n: number): Int32Array {
  const pieces = new Int32Array(SET_COUNT);
  pieces[set] = n;
  return pieces;
}

/** `test-phase` wechselt bei 65 % — ein Wechsel, sauber ansteuerbar. */
const BOSS = bossById('test-phase') as BossDef;

/**
 * Punkte aufbauen, Phase wechseln lassen, dann einen Ausgeber werfen.
 *
 * Gibt den Punktestand vor und nach dem Ausgeber zurück, dazu die Zahl der
 * kostenlosen Ausgaben.
 */
function nachPhasenwechsel(effekte: readonly string[]) {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  combat.setSetEffects(effekte);
  combat.beginFight(0);

  const bind = alsSpellIndex(spellIndex('bind'));
  const stall = alsSpellIndex(spellIndex('stall'));

  // Punkte aufbauen und den Boss unter die Phasenschwelle bringen.
  let t = 0;
  for (let i = 0; i < 12; i++) {
    combat.cast(bind, t);
    t += 1500;
    combat.stepPassive(t);
  }
  const phaseVorher = combat.bossPhase;

  const vorher = combat.control;
  combat.cast(stall, t);
  return {
    phase: phaseVorher,
    vorher,
    nachher: combat.control,
    umsonst: combat.freeControlSpendCount,
  };
}

describe('AC1 — sechs Satzteile lösen ihn aus, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    expect(GRIP_SET).toBeDefined();
    const id = GRIP_SET as number;
    expect(activeRotationEffects(SETS, teile(id, 6))).toContain(SET_EFFECT_LONG_GRIP);
    expect(activeRotationEffects(SETS, teile(id, 5))).not.toContain(SET_EFFECT_LONG_GRIP);
  });
});

describe('AC2 — der erste Ausgeber nach dem Wechsel kostet nichts', () => {
  it('mit Zahlen: die Punkte bleiben stehen', () => {
    const mit = nachPhasenwechsel([SET_EFFECT_LONG_GRIP]);
    const ohne = nachPhasenwechsel([]);

    // ZUSICHERUNG: Es gab überhaupt einen Phasenwechsel — sonst prüft der Fall
    // einen Effekt, dessen Bedingung nie eintrat.
    expect(mit.phase).toBeGreaterThan(0);
    // Und Punkte waren da.
    expect(mit.vorher).toBeGreaterThan(0);

    expect(ohne.nachher).toBeLessThan(ohne.vorher);
    expect(mit.nachher).toBe(mit.vorher);
    expect(mit.umsonst).toBe(1);
    expect(ohne.umsonst).toBe(0);
  });

  it('nur der ERSTE — der zweite kostet wieder', () => {
    // Ein Ja/Nein, keine Zahl: Der Merker ist nach dem ersten Ausgeber weg.
    const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
    combat.setSetEffects([SET_EFFECT_LONG_GRIP]);
    combat.beginFight(0);
    const bind = alsSpellIndex(spellIndex('bind'));
    const stall = alsSpellIndex(spellIndex('stall'));

    let t = 0;
    for (let i = 0; i < 12; i++) {
      combat.cast(bind, t);
      t += 1500;
      combat.stepPassive(t);
    }
    expect(combat.bossPhase).toBeGreaterThan(0);

    const vorErstem = combat.control;
    combat.cast(stall, t);
    const nachErstem = combat.control;
    expect(nachErstem).toBe(vorErstem);

    // Punkte nachfüllen und ein zweites Mal ausgeben.
    for (let i = 0; i < 8; i++) {
      t += 1500;
      combat.cast(bind, t);
      combat.stepPassive(t);
    }
    const vorZweitem = combat.control;
    t += 1500;
    combat.cast(stall, t);
    expect(combat.control).toBeLessThan(vorZweitem);
    expect(combat.freeControlSpendCount).toBe(1);
  });
});

describe('AC3 — ein fremder Effekt löst ihn nicht aus', () => {
  it('The Whole Work im Aufbau macht nichts umsonst', () => {
    const fremd = nachPhasenwechsel([SET_EFFECT_WHOLE_WORK]);
    expect(fremd.umsonst).toBe(0);
    expect(fremd.nachher).toBeLessThan(fremd.vorher);
  });

  it('und eine erfundene Kennung ebenso wenig', () => {
    const fremd = nachPhasenwechsel(['gibt-es-nicht']);
    expect(fremd.umsonst).toBe(0);
    expect(fremd.nachher).toBeLessThan(fremd.vorher);
  });
});

describe('AC4 — die Voraussetzung bleibt: ohne Punkte geht nichts', () => {
  it('ein Ausgeber ohne genug Punkte scheitert auch mit dem Effekt', () => {
    // **Der Fall, der die Auslegung festnagelt.** „Kostet nichts" hebt den
    // Abzug auf, nicht die Prüfung. Übersprünge der Effekt sie, liesse sich
    // der Ausgeber mit null Punkten wirken — und die Klemmung auf Preis 1
    // wäre umgangen statt berücksichtigt.
    const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
    combat.setSetEffects([SET_EFFECT_LONG_GRIP]);
    combat.beginFight(0);
    // Kein Aufbau: null Punkte, aber auch kein Phasenwechsel.
    expect(combat.control).toBe(0);
    combat.cast(alsSpellIndex(spellIndex('stall')), 0);
    expect(combat.controlFailed).toBeGreaterThan(0);
    expect(combat.freeControlSpendCount).toBe(0);
  });
});
