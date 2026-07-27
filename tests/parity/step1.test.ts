/**
 * Gemeinsamer Prüffall: Spiel gegen `tools/step1-check.py` (GAME-DESIGN 18.4).
 *
 * **Das ist der erste harte Prüfstein von Epic E1.** Dieselbe Prioritätenliste muss
 * auf beiden Seiten denselben Schaden ergeben — bis auf den letzten Punkt. Weicht
 * eine Zahl ab, stimmt eine Regel nicht, und zwar bevor darauf aufgebaut wird.
 *
 * Die Sollwerte stehen in `docs/STEP1-SPEC.md` § 4 und stammen aus dem Lauf der
 * Referenz. Sie sind hier bewusst als Zahlen ausgeschrieben statt zur Laufzeit
 * ermittelt: Ein Prüffall, der seinen Sollwert aus dem Prüfling zieht, prüft nichts.
 *
 * Läuft im CI als eigener Job `parity`, sobald `data/`, `src/core/` oder `sim/`
 * berührt werden.
 */
import { describe, expect, it } from 'vitest';
import { runReference } from '../../src/devtools/reference-run.js';

/** Aus `docs/STEP1-SPEC.md` § 4 — dem Lauf von `python3 tools/step1-check.py`. */
const REFERENCE = {
  good: {
    damage: 4236,
    killedAtMs: 44_000,
    remainingMs: 16_000,
    casts: { kindle: 23, smoulder: 4, pyre: 3 },
    pyreDamage: 1176,
    smoulderDamage: 760,
    lostDotTicks: 0,
    resourceOverflow: 0,
    idleMs: 0,
  },
  bad: {
    damage: 4044,
    killedAtMs: -1,
    casts: { kindle: 24, smoulder: 8, pyre: 8 },
    pyreDamage: 684,
    smoulderDamage: 960,
    lostDotTicks: 14,
    resourceOverflow: 0,
    idleMs: 0,
  },
  /** Vergleich über die vollen 60 s, ohne Abbruch beim Sieg. */
  full: {
    good: 5992,
    bad: 4044,
    dotErrorOnly: 5368,
    pyreErrorOnly: 4316,
    spreadPercent: 48.2,
  },
} as const;

describe('Prüffall Schritt 1 — gute Rotation (AC1)', () => {
  const run = runReference('good');

  it('ergibt denselben Schaden wie die Referenz', () => {
    expect(run.damage).toBe(REFERENCE.good.damage);
  });

  it('legt den Boss zur selben Zeit mit derselben Reserve', () => {
    expect(run.killedAtMs).toBe(REFERENCE.good.killedAtMs);
    expect(run.remainingMs).toBe(REFERENCE.good.remainingMs);
  });

  it('wirkt jeden Zauber gleich oft', () => {
    expect(run.casts).toEqual(REFERENCE.good.casts);
  });

  it('verteilt den Schaden gleich auf die Zauber', () => {
    expect(run.damageBySpell['pyre']).toBe(REFERENCE.good.pyreDamage);
    expect(run.damageBySpell['smoulder']).toBe(REFERENCE.good.smoulderDamage);
    // Der Rest ist KINDLE: 23 Wirkungen à 100.
    expect(run.damageBySpell['kindle']).toBe(2300);
  });

  it('verschenkt weder DoT-Ticks noch Hitze noch Zeit', () => {
    expect(run.lostDotTicks).toBe(REFERENCE.good.lostDotTicks);
    expect(run.resourceOverflow).toBe(REFERENCE.good.resourceOverflow);
    expect(run.idleMs).toBe(REFERENCE.good.idleMs);
  });

  it('zieht aus keinem Zufallsstrom', () => {
    // STEP1-SPEC § 2: kein Krit in Schritt 1 — die Rechnung ist exakt, nicht
    // gemittelt. Eine einzige Ziehung würde das stillschweigend aufheben.
    expect(run.randomDraws).toBe(0);
  });
});

describe('Prüffall Schritt 1 — schlechte Rotation (AC1)', () => {
  const run = runReference('bad');

  it('ergibt denselben Schaden wie die Referenz', () => {
    expect(run.damage).toBe(REFERENCE.bad.damage);
  });

  it('legt den Boss nicht', () => {
    expect(run.killedAtMs).toBe(REFERENCE.bad.killedAtMs);
    expect(run.bossHpFraction).toBeGreaterThan(0);
    // STEP1-SPEC § 3: knapp scheitern, damit der Versuch etwas einbringt.
    expect((100 * run.damage) / 4200).toBeCloseTo(96.3, 1);
  });

  it('wirkt jeden Zauber gleich oft', () => {
    expect(run.casts).toEqual(REFERENCE.bad.casts);
  });

  it('verliert genau 14 DoT-Ticks durch zu frühes Erneuern', () => {
    expect(run.lostDotTicks).toBe(REFERENCE.bad.lostDotTicks);
  });

  it('verteilt den Schaden gleich auf die Zauber', () => {
    expect(run.damageBySpell['pyre']).toBe(REFERENCE.bad.pyreDamage);
    expect(run.damageBySpell['smoulder']).toBe(REFERENCE.bad.smoulderDamage);
    expect(run.damageBySpell['kindle']).toBe(2400);
  });
});

describe('Prüffall Schritt 1 — Vergleich über die vollen 60 s', () => {
  const options = { stopOnKill: false } as const;
  const good = runReference('good', options);
  const bad = runReference('bad', options);
  const dotOnly = runReference('dot-error', options);
  const pyreOnly = runReference('pyre-error', options);

  it('trifft alle vier Werte der Referenz', () => {
    expect(good.damage).toBe(REFERENCE.full.good);
    expect(bad.damage).toBe(REFERENCE.full.bad);
    expect(dotOnly.damage).toBe(REFERENCE.full.dotErrorOnly);
    expect(pyreOnly.damage).toBe(REFERENCE.full.pyreErrorOnly);
  });

  it('erfüllt das harte Abnahme-Kriterium aus GAME-DESIGN 20.3', () => {
    const spread = (100 * (good.damage - bad.damage)) / bad.damage;
    expect(spread).toBeCloseTo(REFERENCE.full.spreadPercent, 1);
    expect(spread).toBeGreaterThanOrEqual(10);
  });

  it('lässt auch jeden Einzelfehler über zehn Prozent kosten', () => {
    // Wer nur einen der beiden Punkte versteht, sieht bereits einen messbaren
    // Sprung — die Lernkurve aus STEP1-SPEC § 4.
    const dotLoss = (100 * (good.damage - dotOnly.damage)) / good.damage;
    const pyreLoss = (100 * (good.damage - pyreOnly.damage)) / good.damage;
    expect(dotLoss).toBeGreaterThan(10);
    expect(pyreLoss).toBeGreaterThan(10);
  });
});

describe('Prüffall Schritt 1 — Wiederholbarkeit', () => {
  it('liefert bei zwei Läufen dasselbe Ergebnis', () => {
    // Determinismus ist Design-Gesetz 6. Zwei Läufe müssen byteweise gleich sein.
    const a = runReference('good');
    const b = runReference('good');
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });
});
