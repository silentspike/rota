/**
 * Die drei Wege von CALCINATION bewegen drei VERSCHIEDENE Größen (Issue #419).
 *
 * # Warum das der Kern des Issues ist
 *
 * Bis hierher trugen alle drei Wege `damagePerRankPerMille`. Der
 * Kopfkommentar von `data/player/talents.json` nannte das eine
 * „Zwischenstufe": Die Namen hielten die Absicht aus GAME-DESIGN 12.5 fest,
 * die Wirkung war einheitlich.
 *
 * **Die Folge war messbar und zwingend:** Wenn drei Wege dieselbe Größe
 * bewegen und sich nur in der Zahl unterscheiden, ist der höchste immer die
 * beste Wahl. Bei CALCINATION (15/25/15) waren damit zwei von drei Wegen tot —
 * nicht zufällig, sondern per Konstruktion. AC3 („kein Weg ist tot") war so
 * nicht erfüllbar.
 *
 * ```text
 *              Hitze   Brand-Ticks   Schaden
 *   ohne         3          6          100
 *   build 5      5          6          100     <- nur die Hitze
 *   release 5    3          6          113     <- nur der Schaden
 *   hold 5       3          9          100     <- nur die Laufzeit
 * ```
 *
 * # Die Kreuzprobe ist die eigentliche Aussage
 *
 * Jeder Fall unten prüft nicht nur, DASS sein Weg wirkt, sondern dass er die
 * beiden **fremden** Größen unberührt lässt. Ohne diese Hälfte fiele ein
 * vertauschtes Ziel nicht auf: Der Faktor wirkte, nur woanders — und drei
 * Einzelproben wären alle grün.
 *
 * Dieselbe Bauart wie `buff-groessen-540.test.ts`, aus demselben Grund.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Volle fünf Ränge — der Ausbau, den die Zahlen unten voraussetzen. */
const VOLL = 5;

function kampf(talente: Record<string, number>): Combat {
  // `test-crucible-trial` teilt nicht aus und hat 999999 Leben: Der Lauf endet
  // nicht vorzeitig, und nichts stört die gemessenen Größen.
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  // **VOR `beginFight`.** Ab Zeit 0 ist die Verteilung gesperrt (#425) — ein
  // `setTalents` danach wirkt nicht, und der Fall wäre grün ohne Talente.
  combat.setTalents(talente);
  combat.beginFight(0);
  return combat;
}

/** Weg „build": Wie viel Hitze KINDLE einbringt. */
function hitze(talente: Record<string, number>): number {
  const combat = kampf(talente);
  const vorher = combat.resource;
  const index = alsSpellIndex(combat.indexOf('kindle'));
  expect(combat.cast(index, 0).result, 'KINDLE wurde abgewiesen').toBe(CAST_OK);
  return combat.resource - vorher;
}

/** Weg „hold": Wie oft SMOULDERs Brand tickt, bevor er abläuft. */
function brandTicks(talente: Record<string, number>): number {
  const combat = kampf(talente);
  const index = alsSpellIndex(combat.indexOf('smoulder'));
  expect(combat.cast(index, 0).result, 'SMOULDER wurde abgewiesen').toBe(CAST_OK);
  // Weit über die verlängerte Laufzeit hinaus: Ein zu kurzes Fenster zählte
  // die Ticks des Fensters statt die des Brandes.
  for (let t = TICK_MS; t <= 30_000; t += TICK_MS) combat.stepPassive(t);
  return combat.dotTickCount(combat.indexOf('smoulder'));
}

/** Weg „release": Der Schaden eines Wirkens. */
function schaden(talente: Record<string, number>): number {
  const combat = kampf(talente);
  const index = alsSpellIndex(combat.indexOf('kindle'));
  const ergebnis = combat.cast(index, 0);
  expect(ergebnis.result, 'KINDLE wurde abgewiesen').toBe(CAST_OK);
  return ergebnis.damage;
}

describe('Ohne Talente — die Messlatte', () => {
  it('steht auf den Werten der Datensätze', () => {
    // Ohne diese drei Zahlen sagen die Vergleiche unten nichts: Sie sind der
    // Bezug, gegen den „mehr" gemessen wird.
    expect(hitze({}), 'KINDLE bringt nicht 3 Hitze').toBe(3);
    expect(brandTicks({}), 'SMOULDER tickt nicht 6-mal').toBe(6);
    expect(schaden({}), 'KINDLE richtet nicht 100 an').toBe(100);
  });
});

describe('build — der Weg auf den Ressourcen-Gewinn', () => {
  const TALENT = { 'calcination-build': VOLL };

  it('erhöht die Hitze', () => {
    // 100 Promille je Rang, fünf Ränge: Faktor 1,5 auf 3 ergibt 4,5 → 5.
    expect(hitze(TALENT)).toBe(5);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    expect(brandTicks(TALENT), 'build wirkt auf die Laufzeit').toBe(6);
    expect(schaden(TALENT), 'build wirkt auf den Schaden').toBe(100);
  });
});

describe('release — der Weg auf den Schaden', () => {
  const TALENT = { 'calcination-release': VOLL };

  it('erhöht den Schaden', () => {
    // 25 Promille je Rang, fünf Ränge: Faktor 1,125 auf 100 ergibt 113.
    expect(schaden(TALENT)).toBe(113);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    expect(hitze(TALENT), 'release wirkt auf den Hitze-Gewinn').toBe(3);
    expect(brandTicks(TALENT), 'release wirkt auf die Laufzeit').toBe(6);
  });
});

describe('hold — der Weg auf die Laufzeit', () => {
  const TALENT = { 'calcination-hold': VOLL };

  it('verlängert den Brand — und das heißt MEHR Ticks, nicht langsamere', () => {
    // 100 Promille je Rang, fünf Ränge: Faktor 1,5 auf 12000 ms ergibt 18000.
    // Bei 2000 ms Abstand sind das 9 statt 6 Ticks.
    expect(brandTicks(TALENT)).toBe(9);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    expect(hitze(TALENT), 'hold wirkt auf den Hitze-Gewinn').toBe(3);
    expect(schaden(TALENT), 'hold wirkt auf den Schaden').toBe(100);
  });
});

describe('Damit ist AC3 überhaupt erst eine Frage', () => {
  it('bewegt jeder Weg eine ANDERE Größe als die beiden anderen', () => {
    // **Die Aussage in einer Zeile.** Solange alle drei denselben Topf
    // füllten, war der stärkste immer die beste Wahl — „kein Weg ist tot"
    // war nicht offen, sondern unerfüllbar. Jetzt hängt es von der Rotation
    // ab, und genau das soll der Balance-Läufer beantworten.
    const gemessen = [
      [hitze({ 'calcination-build': VOLL }), 5],
      [schaden({ 'calcination-release': VOLL }), 113],
      [brandTicks({ 'calcination-hold': VOLL }), 9],
    ];
    for (const [ist, soll] of gemessen) expect(ist).toBe(soll);

    // Und die Gegenrichtung: Keine der drei Größen bewegt sich von selbst.
    expect([hitze({}), schaden({}), brandTicks({})]).toEqual([3, 100, 6]);
  });
});
