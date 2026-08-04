import { describe, expect, it } from 'vitest';
import {
  RandomStream,
  RandomStreams,
  STREAM_COUNT,
  STREAM_NAMES,
  seedForStream,
} from '../../src/core/rng.js';
import { LOOT_STREAM_INDEX, lootStreamFor } from '../../src/loot/stream.js';

describe('Der Beute-Strom liegt außerhalb der Kampf-Ströme', () => {
  it('taucht NICHT in den Kampf-Strömen auf', () => {
    // **Der Wächter, um den es geht.** Wer den Beute-Strom später in
    // RandomStreams einträgt — aus dem naheliegenden Reflex, alle
    // Zufallsquellen an einer Stelle zu haben —, verschiebt stillschweigend
    // JEDE aufgezeichnete Prüfsumme: 23 463 Zeilen in fünf Vergleichsdateien,
    // gemessen am 2026-08-04.
    //
    // Er soll das SAGEN, nicht nur verhindern. Deshalb steht die Begründung
    // in der Meldung und nicht nur in einem Kommentar.
    const hinweis =
      'Der Beute-Strom gehört NICHT in RandomStreams (Entscheidung vom 2026-08-04, ' +
      'GAME-DESIGN 20.2). Beute wird beim SIEG gewürfelt, nicht im Kampf — sie kann ' +
      'nichts verschieben. Ihn hier einzutragen hängt jedem Takt eine Null an die ' +
      'Zustands-Prüfsumme und verschiebt alle aufgezeichneten Läufe. Er steht in ' +
      'src/loot/stream.ts.';

    expect(STREAM_COUNT, hinweis).toBe(3);
    expect(STREAM_NAMES, hinweis).toEqual(['crit', 'dotTick', 'heal']);
    expect(STREAM_NAMES, hinweis).not.toContain('loot');
  });

  it('hängt keine Ziehung an die Kampf-Prüfsumme', () => {
    // Die Wirkung, nicht nur die Zahl: Ein Beute-Wurf darf drawCounts() nicht
    // verändern, sonst wandert er doch in die Zustands-Prüfsumme.
    const streams = new RandomStreams(0x1234);
    const vorher = streams.drawCounts();

    const beute = lootStreamFor(0x1234);
    for (let i = 0; i < 100; i++) beute.next();

    expect(streams.drawCounts()).toEqual(vorher);
    expect(streams.totalDraws).toBe(0);
    expect(beute.drawCount).toBe(100);
  });
});

describe('Dieselbe Ableitung wie die Kampf-Ströme', () => {
  it('ist genau der Strom, der in RandomStreams den Index 3 bekäme', () => {
    // **Die erste Auflage, und sie ist mehr als Kosmetik.** Wer die
    // Entscheidung eines Tages umdreht und den Strom doch einreiht, bekommt
    // bitgleich dieselben Zahlen — keine aufgezeichnete Beute ändert sich.
    const seed = 0xc0ffee;
    const beute = lootStreamFor(seed);
    const alsIndex3 = new RandomStream(seedForStream(seed, LOOT_STREAM_INDEX));

    expect(beute.seed).toBe(alsIndex3.seed);
    for (let i = 0; i < 50; i++) {
      expect(beute.next()).toBe(alsIndex3.next());
    }
  });

  it('unterscheidet sich von jedem Kampf-Strom', () => {
    // Sonst zöge die Beute dieselbe Folge wie der Krit — der Fehler, vor dem
    // getrennte Ströme überhaupt schützen.
    const seed = 0xc0ffee;
    const beute = lootStreamFor(seed);
    const streams = new RandomStreams(seed);
    for (let i = 0; i < STREAM_COUNT; i++) {
      expect(beute.seed).not.toBe(streams.stream(i).seed);
    }
  });

  it('gibt für denselben Kampf dieselbe Beute-Folge', () => {
    // Eine aufgezeichnete Runde muss auch ihre Beute wiederholen. Ohne das
    // wäre der getrennte Strom tatsächlich der schlechtere Weg.
    const ersteRunde = lootStreamFor(0xabcd);
    const zweiteRunde = lootStreamFor(0xabcd);
    for (let i = 0; i < 20; i++) {
      expect(zweiteRunde.next()).toBe(ersteRunde.next());
    }
  });

  it('gibt für verschiedene Kämpfe verschiedene Folgen', () => {
    const eine = lootStreamFor(1);
    const andere = lootStreamFor(2);
    expect(andere.next()).not.toBe(eine.next());
  });
});
