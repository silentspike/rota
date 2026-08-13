/**
 * Zwei Sätze dürfen nicht dieselbe Kennung tragen (13.5, #480).
 *
 * # Woher dieser Prüffall kommt
 *
 * Nicht aus einer Überlegung, sondern aus einem Schaden. Am 2026-08-13 wurden
 * drei Satz-Effekte in drei Zweigen gebaut, jeder einzeln von `main`. Jeder
 * suchte „die nächste freie Kennung", und alle drei sahen denselben Stand —
 * also vergaben alle drei dieselbe. Bemerkt wurde es erst, als der zweite
 * Zweig `DIRTY` stand, und beim Auflösen des Konflikts stand ein Satz kurz
 * ZWEIMAL in der Datei, ohne dass irgendein Lauf rot wurde.
 *
 * Eine Kennung, die aus dem Bestand abgeleitet wird, ist nur so lange
 * eindeutig, wie ein einziger Zweig sie ableitet. Genau deshalb gehört die
 * Prüfung in den Parser und nicht in die Sorgfalt des Bearbeitenden.
 *
 * # Warum eine Dublette still bliebe
 *
 * `countPieces` legt die getragenen Teile in einem Feld ab, dessen Index die
 * Kennung ist. Zwei Sätze mit derselben Kennung zählen auf dasselbe Feld:
 * Ihre Teile addieren sich, und bei sechs GEMISCHTEN Teilen lösen BEIDE
 * Sechser-Boni aus — der Spieler bekäme zwei Rotations-Effekte für einen
 * halben Satz. Keine Ausnahme, kein falscher Wert, nur ein Aufbau, der doppelt
 * so stark ist wie beschrieben.
 */

import { describe, expect, it } from 'vitest';
import {
  activeRotationEffects,
  countPieces,
  parseSets,
  SETS,
  SET_COUNT,
} from '../../src/gear/sets.js';

/** Ein gültiger Satz-Rohling; `id` und `effect` setzt der jeweilige Fall. */
function roh(id: number, name: string, effect: string) {
  return {
    id,
    school: 'CALCINATION',
    name,
    tiers: [
      { kind: 0, share: 0.1, effect: '' },
      { kind: 0, share: 0.2, effect: '' },
      { kind: 1, share: 0, effect },
    ],
  };
}

describe('AC — der Parser weist zwei gleiche Kennungen ab', () => {
  it('wirft, und nennt BEIDE Sätze beim Namen', () => {
    const doppelt = [roh(1, 'Erster Satz', 'erster'), roh(1, 'Zweiter Satz', 'zweiter')];
    expect(() => parseSets(doppelt)).toThrow(/Kennung 1 kommt zweimal vor/);
    // Der Name muss in der Meldung stehen: Wer sie liest, sucht sonst in einer
    // Datei mit sechs gleich aussehenden Blöcken nach der Stelle.
    expect(() => parseSets(doppelt)).toThrow(/Erster Satz/);
    expect(() => parseSets(doppelt)).toThrow(/Zweiter Satz/);
  });

  it('auch wenn die Dublette nicht direkt daneben steht', () => {
    // So sah der echte Fall aus: Zwischen den beiden lag ein unbeteiligter
    // Satz. Eine Prüfung, die nur den Vorgänger vergleicht, hätte ihn
    // durchgelassen.
    const verstreut = [roh(0, 'A', 'a'), roh(2, 'B', 'b'), roh(1, 'C', 'c'), roh(2, 'D', 'd')];
    expect(() => parseSets(verstreut)).toThrow(/Kennung 2 kommt zweimal vor/);
    expect(() => parseSets(verstreut)).toThrow(/"B" und "D"/);
  });

  it('ZUSICHERUNG: derselbe Aufbau OHNE Dublette geht durch', () => {
    // Ohne diesen Fall bewiese der obige nur, dass `parseSets` irgendetwas
    // ablehnt — nicht, dass es die Dublette ist.
    const sauber = [roh(0, 'A', 'a'), roh(1, 'B', 'b'), roh(2, 'C', 'c')];
    expect(parseSets(sauber)).toHaveLength(3);
    expect(parseSets(sauber).map((s) => s.id)).toEqual([0, 1, 2]);
  });
});

describe('AC — der Schaden, den die Dublette anrichten würde', () => {
  it('sechs GEMISCHTE Teile lösten beide Sechser-Boni aus', () => {
    // Nicht der Parser, sondern die Wirkung dahinter: Wären die zwei Sätze
    // durchgekommen, hätten drei Teile des einen und drei des anderen für
    // BEIDE höchsten Stufen gereicht. Dieser Fall zeigt, was der Wächter
    // verhindert — er baut die Dublette am Parser vorbei nach.
    const zwei = [
      { id: 1, school: 'CALCINATION', tiers: parseSets([roh(1, 'X', 'effekt-eins')])[0]!.tiers },
      { id: 1, school: 'CALCINATION', tiers: parseSets([roh(1, 'Y', 'effekt-zwei')])[0]!.tiers },
    ];
    const getragen = new Int32Array(SET_COUNT);
    getragen[1] = 6;

    const effekte = activeRotationEffects(zwei, getragen);
    expect(effekte).toContain('effekt-eins');
    expect(effekte).toContain('effekt-zwei');
    // Sechs Teile, zwei Sechser-Effekte. Das ist der Schaden.
    expect(effekte).toHaveLength(2);
  });

  it('und die Teile beider Sätze landeten in DEMSELBEN Feld', () => {
    // `countPieces` ist die Stelle, an der sie sich vermischen: Der Index IST
    // die Kennung. Drei Teile von Satz X und drei von Satz Y stünden hier
    // ununterscheidbar als „sechs Teile von Satz 1".
    const gezaehlt = new Int32Array(SET_COUNT);
    const getragen = countPieces([1, 1, 1, 1, 1, 1], gezaehlt);
    expect(gezaehlt[1]).toBe(6);
    expect(getragen).toBe(6);
  });
});

describe('AC — der ausgelieferte Bestand ist frei davon', () => {
  it('jede Kennung in data/sets.json kommt genau einmal vor', () => {
    const ids = SETS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('und die Kennungen sind fortlaufend ohne Lücke', () => {
    // Eine Lücke wäre kein Fehler, aber ein Hinweis auf einen verlorenen Satz
    // — und sie macht „die nächste freie Kennung" mehrdeutig, was genau der
    // Auslöser dieser ganzen Sache war.
    const ids = [...SETS.map((s) => s.id)].sort((a, b) => a - b);
    expect(ids).toEqual(ids.map((_, i) => i));
  });

  it('ZUSICHERUNG: es sind mehr als zwei Sätze, sonst prüft das nichts', () => {
    expect(SETS.length).toBeGreaterThan(2);
  });
});
