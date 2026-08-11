/**
 * PYROCLASMs fünf Spender-Größen (Issue #547).
 *
 * # Der schwerste Einzelbefund der Vollmessung
 *
 * `tools/groessen-mutation.py`, alle 116 Paare am 2026-08-11 gefahren:
 *
 * ```text
 *   spender.bonusMultiplier  pyroclasm  2 -> 1       NUR FRISCHE
 *   spender.bonusThreshold   pyroclasm  15 -> 7      SCHWEIGT
 *   spender.consumesAll      pyroclasm  true -> false NUR FRISCHE
 *   spender.damagePerPoint   pyroclasm  12 -> 6      NUR FRISCHE
 *   spender.resource         pyroclasm  heat -> …    NUR FRISCHE
 * ```
 *
 * **Fünf von fünf, der komplette Spender-Block.** Einer davon schweigt
 * vollständig — man verschiebt die Bonus-Schwelle, und nicht einmal die
 * Vergleichsdatei färbt.
 *
 * PYROCLASM ist der große Entlader der CALCINATION: der Zauber, der die
 * gesammelte Hitze in Schaden umsetzt. Man konnte seinen Schaden je Punkt
 * halbieren, ohne dass ein Prüffall etwas sagte.
 *
 * # Warum ausgerechnet er ungedeckt war
 *
 * Sein Zwilling `pyre` trägt **dieselben fünf Werte** und ist mit 41 Fällen
 * gedeckt:
 *
 * ```text
 *   pyre       cooldownMs  5000   spender { heat, 12, 15, 2, consumesAll }
 *   pyroclasm  cooldownMs 45000   spender { heat, 12, 15, 2, consumesAll }
 * ```
 *
 * Dieselbe Klasse wie AETHER neben DISTIL (#760): Der Zwilling ist bewacht,
 * das Gegenstück nicht — und weil beide dieselben Zahlen tragen, sieht die
 * Abdeckung auf den ersten Blick vollständig aus.
 *
 * **Bitter dabei:** PYROCLASM ist genau der Träger, den `buff-groessen-540`
 * und `aether-buff-groessen` benutzen, weil seine 45-Sekunden-Abklingzeit
 * Platz für Wirkung lässt. Er misst andere Größen und war selbst ungedeckt.
 *
 * # Die Zahlen sind gemessen, nicht gesetzt
 *
 * ```text
 *   hitze= 5  schaden= 60  rest=0
 *   hitze=14  schaden=168  rest=0
 *   hitze=15  schaden=360  rest=0     <- die Schwelle
 *   hitze=20  schaden=480  rest=0
 * ```
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  return new Combat({ school: CALCINATION.value, boss, seed: 0 });
}

/** Wirft PYROCLASM bei gegebenem Hitzestand und meldet Schaden und Rest. */
function entladen(hitze: number): { schaden: number; rest: number } {
  const combat = kampf();
  combat.setResource(hitze);
  expect(combat.resource, `die Hitze wurde nicht auf ${hitze} gesetzt`).toBe(hitze);

  const index = alsSpellIndex(combat.indexOf('pyroclasm'));
  expect(index, 'PYROCLASM fehlt in der Schule').toBeGreaterThanOrEqual(0);
  combat.stepPassive(10);
  const ergebnis = combat.cast(index, 10);
  expect(ergebnis.result, 'PYROCLASM wurde abgewiesen').toBe(CAST_OK);
  return { schaden: ergebnis.damage, rest: combat.resource };
}

describe('PYROCLASM — Schaden je Hitzepunkt (spender.damagePerPoint)', () => {
  it('setzt jeden Punkt in zwölf Schaden um', () => {
    // Unterhalb der Schwelle, damit der Bonus die Rechnung nicht überdeckt.
    expect(entladen(5).schaden, 'fünf Punkte ergeben nicht 60').toBe(60);
    expect(entladen(10).schaden, 'zehn Punkte ergeben nicht 120').toBe(120);
  });

  it('skaliert linear — doppelte Hitze, doppelter Schaden', () => {
    // Ohne diese Zeile wäre auch eine feste Zahl statt einer Multiplikation
    // grün. Beide Stände liegen unter der Schwelle.
    expect(entladen(14).schaden).toBe(2 * entladen(7).schaden);
  });
});

describe('PYROCLASM — die Bonus-Schwelle (spender.bonusThreshold)', () => {
  it('greift AB fünfzehn, nicht darunter — an der Grenze gemessen', () => {
    // **Die Grenze, nicht zwei ferne Punkte.** Bei 14 gilt der Bonus nicht,
    // bei 15 schon; wer nur 5 und 20 prüfte, ließe jede Schwelle zwischen 6
    // und 20 durchgehen.
    //
    // Diese Größe SCHWEIGT auf `main` — nicht einmal die Vergleichsdatei
    // färbt, wenn man sie von 15 auf 7 setzt.
    expect(entladen(14).schaden, 'bei 14 greift der Bonus schon').toBe(168);
    expect(entladen(15).schaden, 'bei 15 greift der Bonus nicht').toBe(360);
  });
});

describe('PYROCLASM — der Bonus-Faktor (spender.bonusMultiplier)', () => {
  it('verdoppelt oberhalb der Schwelle', () => {
    // 15 Punkte × 12 = 180 ohne Bonus, 360 mit. Der Faktor steht damit
    // festgenagelt, nicht nur seine Richtung.
    const mitBonus = entladen(15).schaden;
    expect(mitBonus).toBe(2 * 15 * 12);
    // Und die Gegenprobe: knapp darunter ist es der einfache Satz.
    expect(entladen(14).schaden).toBe(14 * 12);
  });
});

describe('PYROCLASM — verbraucht ALLES (spender.consumesAll)', () => {
  it('lässt keinen Punkt übrig, egal wie viel anlag', () => {
    expect(entladen(5).rest, 'nach dem Entladen bleibt Hitze übrig').toBe(0);
    expect(entladen(20).rest, 'nach dem Entladen bleibt Hitze übrig').toBe(0);
  });

  it('und die Zusicherung dazu: vorher lag wirklich etwas an', () => {
    // Ohne sie wäre „rest === 0" auch bei einem Kampf grün, in dem nie Hitze
    // vorhanden war — die Prüfung misst dann ihren eigenen Ausgangszustand.
    const combat = kampf();
    combat.setResource(20);
    expect(combat.resource).toBe(20);
  });
});

describe('PYROCLASM — zieht aus der HITZE (spender.resource)', () => {
  it('senkt die Hitze und keine andere Größe', () => {
    // `spender.resource` ist eine Zeichenkette; ein falscher Wert macht den
    // Zauber wirkungslos, statt ihn laut scheitern zu lassen. Gemessen wird
    // deshalb, dass genau diese Größe fällt.
    const combat = kampf();
    combat.setResource(12);
    const vorher = combat.resource;
    const index = alsSpellIndex(combat.indexOf('pyroclasm'));
    combat.stepPassive(10);
    const ergebnis = combat.cast(index, 10);

    expect(ergebnis.result).toBe(CAST_OK);
    expect(vorher, 'ohne Hitze sagt der Vergleich nichts').toBeGreaterThan(0);
    expect(combat.resource, 'die Hitze wurde nicht verbraucht').toBeLessThan(vorher);
    expect(ergebnis.damage, 'ohne Hitze kein Entladeschaden').toBe(vorher * 12);
  });
});

describe('Der Zwilling PYRE trägt dieselben Werte — das war der blinde Fleck', () => {
  it('entlädt bei gleichem Stand gleich viel', () => {
    // **Warum das hier steht:** `pyre` ist mit 41 Fällen gedeckt, PYROCLASM
    // war es mit keinem. Weil beide dieselben fünf Werte tragen, sah die
    // Abdeckung vollständig aus — dieselbe Klasse wie AETHER neben DISTIL.
    //
    // Diese Zeile hält fest, dass sie WIRKLICH gleich rechnen. Läuft einer
    // der beiden künftig auseinander, ist das eine Entscheidung und kein
    // Versehen.
    const combat = kampf();
    combat.setResource(15);
    const index = alsSpellIndex(combat.indexOf('pyre'));
    combat.stepPassive(10);
    const pyre = combat.cast(index, 10);
    expect(pyre.result).toBe(CAST_OK);
    expect(pyre.damage, 'PYRE und PYROCLASM entladen verschieden').toBe(entladen(15).schaden);
  });
});
