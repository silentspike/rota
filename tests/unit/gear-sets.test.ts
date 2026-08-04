import { describe, expect, it } from 'vitest';
import {
  BONUS_ROTATION,
  BONUS_VALUE,
  NO_SET,
  SET_COUNT,
  activeRotationEffects,
  countPieces,
  tierActive,
  tiersReached,
  valueShareOf,
  type SetDef,
} from '../../src/gear/sets.js';
import { POOL_GEAR_EFFECTS, ValuePools } from '../../src/gear/pools.js';
import { SLOT_COUNT } from '../../src/gear/slots.js';

/**
 * Zwei erfundene Sätze. **Sie werden nicht ausgeliefert.**
 *
 * Welche 18 Boni es gibt (sechs Sätze mal drei Stufen), ist Katalog-Inhalt
 * und steht vor E10 aus (GAME-DESIGN 22). Geprüft wird das SYSTEM, nicht ein
 * Inhalt, den es nicht gibt — dieselbe Lage wie beim Auswahlmodus (#102) und
 * bei den Töpfen (#114).
 */
const SATZ_A: SetDef = {
  id: 0,
  school: 'CALCINATION',
  tiers: [
    { kind: BONUS_VALUE, share: 0.1, effect: '' },
    { kind: BONUS_VALUE, share: 0.2, effect: '' },
    // Werte unten, Rotation oben (13.5).
    { kind: BONUS_ROTATION, share: 0, effect: 'test-a-sechser' },
  ],
};

const SATZ_B: SetDef = {
  id: 1,
  school: 'SUBLIMATION',
  tiers: [
    { kind: BONUS_VALUE, share: 0.05, effect: '' },
    { kind: BONUS_VALUE, share: 0.15, effect: '' },
    { kind: BONUS_ROTATION, share: 0, effect: 'test-b-sechser' },
  ],
};

const SAETZE = [SATZ_A, SATZ_B];

/** Eine Zusammenstellung: `[Satz auf Platz 0, Satz auf Platz 1, …]`. */
function teile(...saetze: number[]): number[] {
  const leiste = new Array<number>(SLOT_COUNT).fill(NO_SET);
  for (let i = 0; i < saetze.length && i < SLOT_COUNT; i++) leiste[i] = saetze[i] as number;
  return leiste;
}

function zaehlen(leiste: readonly number[]): Int32Array {
  const out = new Int32Array(SET_COUNT);
  countPieces(leiste, out);
  return out;
}

describe('Drei Bonus-Stufen bei zwei, vier und sechs Teilen (AC1)', () => {
  it('löst je Schwelle eine weitere Stufe aus', () => {
    expect(tiersReached(0)).toBe(0);
    expect(tiersReached(1)).toBe(0);
    expect(tiersReached(2)).toBe(1);
    expect(tiersReached(3)).toBe(1);
    expect(tiersReached(4)).toBe(2);
    expect(tiersReached(6)).toBe(3);
  });

  it('gibt bei fünf Teilen DASSELBE wie bei vier', () => {
    // Der Fall, den AC1 ausdrücklich nennt. Eine Prüfung auf `=== 4` wäre die
    // naheliegende Schreibweise und gäbe bei fünf Teilen NICHTS — der Spieler
    // verlöre einen Bonus, weil er ein Teil zu viel trägt.
    const mitVier = zaehlen(teile(0, 0, 0, 0));
    const mitFuenf = zaehlen(teile(0, 0, 0, 0, 0));

    expect(tiersReached(4)).toBe(tiersReached(5));
    expect(valueShareOf(SAETZE, mitFuenf)).toBe(valueShareOf(SAETZE, mitVier));
    expect(activeRotationEffects(SAETZE, mitFuenf)).toEqual(activeRotationEffects(SAETZE, mitVier));
  });

  it('gibt ohne Satz NICHTS — der Vergleichsfall', () => {
    // **Die Messlatte muss den Fall enthalten, in dem sich nichts ändern
    // darf.** Ohne diesen Lauf belegte der Rest nur, dass überhaupt etwas
    // passiert, nicht dass es am Satz liegt.
    const ohne = zaehlen(teile());
    expect(valueShareOf(SAETZE, ohne)).toBe(0);
    expect(activeRotationEffects(SAETZE, ohne)).toEqual([]);
  });

  it('wirkt stärker, je mehr Teile — geprüft als Ungleichung, nicht als Zahl', () => {
    // Eine Gleichheitsprüfung gegen einen Zahlenwert fängt den Nullfall
    // nicht: 0 === 0 ist grün. Die Ungleichung fängt ihn.
    const ohne = valueShareOf(SAETZE, zaehlen(teile()));
    const zwei = valueShareOf(SAETZE, zaehlen(teile(0, 0)));
    const vier = valueShareOf(SAETZE, zaehlen(teile(0, 0, 0, 0)));

    expect(zwei).toBeGreaterThan(ohne);
    expect(vier).toBeGreaterThan(zwei);
  });
});

describe('Frei kombinierbar (AC2)', () => {
  it('löst bei vier von A und zwei von B beide Sätze aus', () => {
    const gemischt = zaehlen(teile(0, 0, 0, 0, 1, 1));
    expect(gemischt[0]).toBe(4);
    expect(gemischt[1]).toBe(2);

    // A auf Stufe 1 und 2, B auf Stufe 1 — nicht mehr und nicht weniger.
    expect(tierActive(gemischt[0] as number, 0)).toBe(true);
    expect(tierActive(gemischt[0] as number, 1)).toBe(true);
    expect(tierActive(gemischt[0] as number, 2)).toBe(false);
    expect(tierActive(gemischt[1] as number, 0)).toBe(true);
    expect(tierActive(gemischt[1] as number, 1)).toBe(false);

    // 0,1 + 0,2 aus A, 0,05 aus B.
    expect(valueShareOf(SAETZE, gemischt)).toBeCloseTo(0.35, 12);
  });

  it('gibt gemischt mehr als jeder Satz allein — die Ungleichung', () => {
    const nurA = valueShareOf(SAETZE, zaehlen(teile(0, 0, 0, 0)));
    const nurB = valueShareOf(SAETZE, zaehlen(teile(1, 1)));
    const gemischt = valueShareOf(SAETZE, zaehlen(teile(0, 0, 0, 0, 1, 1)));

    expect(gemischt).toBeGreaterThan(nurA);
    expect(gemischt).toBeGreaterThan(nurB);
    expect(gemischt).toBeCloseTo(nurA + nurB, 12);
  });

  it('lässt sechs Teile eines Satzes den Rotations-Effekt auslösen', () => {
    const sechs = zaehlen(teile(0, 0, 0, 0, 0, 0));
    expect(activeRotationEffects(SAETZE, sechs)).toEqual(['test-a-sechser']);
    // Und bei fünf noch nicht — die Krönung ist die Krönung.
    const fuenf = zaehlen(teile(0, 0, 0, 0, 0));
    expect(activeRotationEffects(SAETZE, fuenf)).toEqual([]);
  });
});

describe('Satz-Boni und legendäre Effekte teilen einen Topf (AC3)', () => {
  it('addiert sie untereinander, nicht multiplikativ', () => {
    // 13.1b: Ein Sechser-Satz mit 30 % und ein legendärer Effekt mit 20 %
    // ergeben 50 %, nicht 56 %.
    const pools = new ValuePools();
    const satzAnteil = valueShareOf(SAETZE, zaehlen(teile(0, 0, 0, 0)));
    expect(satzAnteil).toBeCloseTo(0.3, 12);

    pools.add(POOL_GEAR_EFFECTS, satzAnteil);
    pools.add(POOL_GEAR_EFFECTS, 0.2); // legendärer Effekt

    expect(pools.factor(POOL_GEAR_EFFECTS)).toBeCloseTo(1.5, 12);
    expect(pools.factor(POOL_GEAR_EFFECTS)).not.toBeCloseTo(1.3 * 1.2, 6);
  });

  it('zeigt beide Fälle in EINEM Lauf', () => {
    // Ohne den Vergleichsfall ohne Satz belegte das nur, dass die Zahl
    // irgendwoher kommt.
    const mitSatz = new ValuePools();
    mitSatz.add(POOL_GEAR_EFFECTS, valueShareOf(SAETZE, zaehlen(teile(0, 0, 0, 0))));
    mitSatz.add(POOL_GEAR_EFFECTS, 0.2);

    const ohneSatz = new ValuePools();
    ohneSatz.add(POOL_GEAR_EFFECTS, valueShareOf(SAETZE, zaehlen(teile())));
    ohneSatz.add(POOL_GEAR_EFFECTS, 0.2);

    expect(mitSatz.factor(POOL_GEAR_EFFECTS)).toBeGreaterThan(ohneSatz.factor(POOL_GEAR_EFFECTS));
    expect(ohneSatz.factor(POOL_GEAR_EFFECTS)).toBeCloseTo(1.2, 12);
  });
});

describe('Der Platz spielt keine Rolle (AC4)', () => {
  it('zählt dieselben Teile auf beliebigen Plätzen gleich', () => {
    const vorne = zaehlen(teile(0, 0, 0, 0));
    const hinten = new Int32Array(SET_COUNT);
    const leiste = new Array<number>(SLOT_COUNT).fill(NO_SET);
    leiste[9] = 0;
    leiste[7] = 0;
    leiste[4] = 0;
    leiste[2] = 0;
    countPieces(leiste, hinten);

    expect(Array.from(hinten)).toEqual(Array.from(vorne));
    expect(valueShareOf(SAETZE, hinten)).toBe(valueShareOf(SAETZE, vorne));
  });

  it('kennt gar keine Plätze — die Regel in der Bauform', () => {
    // `countPieces` bekommt nur „welcher Satz sitzt wo" und zählt. Eine
    // platz-abhängige Regel ließe sich hier nicht einmal ausdrücken.
    const gedreht = zaehlen([0, NO_SET, 0, NO_SET, 0, NO_SET, 0, NO_SET, NO_SET, NO_SET]);
    expect(gedreht[0]).toBe(4);
  });

  it('zählt ein Stück ohne Satz nicht mit', () => {
    const leiste = teile(0, 0, NO_SET, NO_SET);
    expect(countPieces(leiste, new Int32Array(SET_COUNT))).toBe(2);
  });

  it('verwirft eine unbekannte Satz-Kennung, statt daneben zu schreiben', () => {
    const out = new Int32Array(SET_COUNT);
    expect(countPieces([SET_COUNT, -5, 1.5, 0], out)).toBe(1);
    expect(out[0]).toBe(1);
  });
});

describe('Was hier NICHT festgelegt ist', () => {
  it('nimmt die Boni als Daten entgegen', () => {
    // Welche 18 Boni es gibt, ist Katalog-Inhalt (GAME-DESIGN 22). Dieser
    // Prüffall hält fest, dass das System ohne sie vollständig ist: Ein
    // anderer Satz mit anderen Zahlen läuft durch dieselbe Rechnung.
    const anderer: SetDef = {
      id: 2,
      school: 'PUTREFACTION',
      tiers: [
        { kind: BONUS_VALUE, share: 0.99, effect: '' },
        { kind: BONUS_ROTATION, share: 0, effect: 'irgendwas' },
        { kind: BONUS_VALUE, share: 0.01, effect: '' },
      ],
    };
    const pieces = zaehlen(teile(2, 2, 2, 2));
    expect(valueShareOf([anderer], pieces)).toBeCloseTo(0.99, 12);
    expect(activeRotationEffects([anderer], pieces)).toEqual(['irgendwas']);
  });
});
