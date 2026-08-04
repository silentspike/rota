import { describe, expect, it } from 'vitest';
import {
  POOL_ACTIVE,
  POOL_CATALYSIS,
  POOL_COUNT,
  POOL_ESSENCE,
  POOL_GEAR_EFFECTS,
  POOL_NAMES,
  POOL_PURITY,
  POOL_YIELD,
  ValuePools,
} from '../../src/gear/pools.js';

describe('Innerhalb addieren, zwischen den Töpfen multiplizieren (AC2)', () => {
  it('addiert zwei ESSENCE-Quellen', () => {
    const pools = new ValuePools();
    pools.add(POOL_ESSENCE, 0.2);
    pools.add(POOL_ESSENCE, 0.3);
    // 20 % + 30 % = 50 %, nicht 56 %.
    expect(pools.factor(POOL_ESSENCE)).toBe(1.5);
  });

  it('multipliziert zwei verschiedene Sonderwerte', () => {
    const pools = new ValuePools();
    pools.add(POOL_CATALYSIS, 0.2);
    pools.add(POOL_PURITY, 0.3);
    expect(pools.factor(POOL_CATALYSIS)).toBe(1.2);
    expect(pools.factor(POOL_PURITY)).toBe(1.3);
    // 1,2 × 1,3 = 1,56 — und ausdrücklich NICHT 1,5.
    expect(pools.product()).toBeCloseTo(1.56, 12);
    expect(pools.product()).not.toBeCloseTo(1.5, 6);
  });

  it('zeigt beide Regeln im selben Lauf', () => {
    // Der Kern von AC2 ist der UNTERSCHIED. Ein Lauf, der nur eine der beiden
    // Regeln zeigt, wäre auch grün, wenn es die andere gar nicht gäbe.
    const pools = new ValuePools();
    pools.add(POOL_CATALYSIS, 0.2);
    pools.add(POOL_CATALYSIS, 0.3); // gleicher Wert  -> addiert
    pools.add(POOL_PURITY, 0.5); // anderer Wert   -> multipliziert

    expect(pools.factor(POOL_CATALYSIS)).toBe(1.5);
    expect(pools.factor(POOL_PURITY)).toBe(1.5);
    expect(pools.product()).toBe(2.25);
    // Wären alle drei Quellen additiv, stünde hier 2,0.
    expect(pools.product()).not.toBe(2.0);
  });
});

describe('Legendäre Effekte und Satz-Boni teilen einen Topf (AC3)', () => {
  it('addiert 30 % Satz-Bonus und 20 % legendären Effekt zu 50 %, nicht zu 56 %', () => {
    const pools = new ValuePools();
    pools.add(POOL_GEAR_EFFECTS, 0.3); // Sechser-Satz
    pools.add(POOL_GEAR_EFFECTS, 0.2); // legendärer Effekt

    expect(pools.factor(POOL_GEAR_EFFECTS)).toBe(1.5);
    // Der Wert, den ein eigener Topf je Quelle ergäbe:
    expect(pools.factor(POOL_GEAR_EFFECTS)).not.toBe(1.2 * 1.3);
  });

  it('zeigt BEIDE Fälle in EINEM Lauf', () => {
    // **Die Auflage zu diesem Issue, wörtlich:** „AC3 muss beide Fälle in
    // einem Lauf zeigen. Nur additiv oder nur multiplikativ wäre trivial
    // grün." Der Fall unten enthält deshalb gleichzeitig
    //   - zwei Quellen IM SELBEN Topf (Satz + legendär)  -> addieren
    //   - einen zweiten Topf daneben (CATALYSIS)         -> multipliziert
    const pools = new ValuePools();
    pools.add(POOL_GEAR_EFFECTS, 0.3);
    pools.add(POOL_GEAR_EFFECTS, 0.2);
    pools.add(POOL_CATALYSIS, 0.1);

    expect(pools.factor(POOL_GEAR_EFFECTS)).toBe(1.5); // additiv innen
    expect(pools.factor(POOL_CATALYSIS)).toBe(1.1);
    expect(pools.product()).toBeCloseTo(1.65, 12); // multiplikativ außen

    // Und der Gegenbeweis: Wären Satz und Legendär eigene Töpfe, käme
    // 1,2 × 1,3 × 1,1 = 1,716 heraus. Der Unterschied ist der ganze Punkt.
    expect(pools.product()).not.toBeCloseTo(1.716, 6);
  });

  it('wächst der Unterschied mit jeder weiteren Quelle', () => {
    // Warum die Regel überhaupt zählt: Bei einer Quelle ist der Unterschied
    // null, bei zehn ist er das Spiel. Genau deshalb wurde in Diablo 3 alles
    // ohne Satz-Bonus ignoriert.
    const additiv = new ValuePools();
    let multiplikativ = 1;
    for (let quelle = 0; quelle < 10; quelle++) {
      additiv.add(POOL_GEAR_EFFECTS, 0.2);
      multiplikativ *= 1.2;
    }
    expect(additiv.factor(POOL_GEAR_EFFECTS)).toBeCloseTo(3.0, 12);
    expect(multiplikativ).toBeCloseTo(6.19, 2);
    expect(multiplikativ).toBeGreaterThan(additiv.factor(POOL_GEAR_EFFECTS) * 2);
  });
});

describe('Die Topf-Struktur selbst', () => {
  it('führt sechs Faktoren aus vier Töpfen', () => {
    expect(POOL_COUNT).toBe(6);
    expect(POOL_NAMES.length).toBe(POOL_COUNT);
    // Die drei Sonderwerte sind drei Kammern EINES Topfes, nach außen aber
    // drei Faktoren — das ist die Zeile „je eigener Faktor" aus 13.1b.
    expect(new Set([POOL_CATALYSIS, POOL_PURITY, POOL_YIELD]).size).toBe(3);
  });

  it('ist ohne Beiträge durchweg neutral', () => {
    const pools = new ValuePools();
    for (let pool = 0; pool < POOL_COUNT; pool++) {
      expect(pools.factor(pool)).toBe(1);
    }
    expect(pools.product()).toBe(1);
  });

  it('meldet einen unbekannten Topf, statt still zu verwerfen', () => {
    const pools = new ValuePools();
    expect(pools.add(POOL_COUNT, 0.5)).toBe(false);
    expect(pools.add(-1, 0.5)).toBe(false);
    expect(pools.add(1.5, 0.5)).toBe(false);
    expect(pools.add(POOL_ESSENCE, 0.5)).toBe(true);
    expect(pools.product()).toBe(1.5);
  });

  it('multipliziert immer in derselben Reihenfolge', () => {
    // Fließkomma ist nicht assoziativ. Dieselben Beiträge in anderer
    // Reihenfolge eingetragen müssen bitgleich dasselbe Produkt ergeben,
    // sonst laufen Spiel und Simulator auseinander (18.4).
    const vorwaerts = new ValuePools();
    const rueckwaerts = new ValuePools();
    const beitraege = [0.13, 0.27, 0.07, 0.41, 0.19, 0.33];
    for (let pool = 0; pool < POOL_COUNT; pool++) {
      vorwaerts.add(pool, beitraege[pool] ?? 0);
    }
    for (let pool = POOL_COUNT - 1; pool >= 0; pool--) {
      rueckwaerts.add(pool, beitraege[pool] ?? 0);
    }
    expect(rueckwaerts.product()).toBe(vorwaerts.product());
  });

  it('setzt beim Ausrüstungswechsel alles zurück', () => {
    const pools = new ValuePools();
    pools.add(POOL_ACTIVE, 0.4);
    pools.add(POOL_GEAR_EFFECTS, 0.6);
    pools.clear();
    expect(pools.product()).toBe(1);
  });
});
