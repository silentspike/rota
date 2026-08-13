import { describe, expect, it } from 'vitest';
import { TALENTS } from '../../src/core/data.js';

/**
 * Der AC3-Wächter im Simulator rechnet mit fest verdrahteten Promille —
 * diese Datei hält sie an den Daten fest (Issue #419).
 *
 * # Warum es diese Datei braucht
 *
 * `sim/tests/ac3_wege_gewinnen.rs` belegt, dass jeder CALCINATION-Weg in einer
 * spielbaren Rotation gewinnt. Er setzt die Talent-Größen dafür als **Zahlen**:
 * `talent_resource_permille = 500` und so weiter. Der Simulator kennt
 * `talents.json` nicht — bewusst, aus demselben Grund wie bei `decay-talent`.
 *
 * **Gemessen, und deshalb steht diese Datei hier:** Setzt man
 * `calcination-hold` in den Daten auf 0 Promille, bleibt der Rust-Wächter
 * GRÜN. Er prüft die Kampf-Logik, nicht die Datenlage — ein Weg könnte in den
 * Daten sterben, während der Beleg für ihn weiterläuft.
 *
 * Die Gegenprobe dazu ist gefahren: Nimmt man dem Kampf den Laufzeit-Faktor,
 * werden dort zwei von drei Fällen rot. Der Wächter ist also nicht blind —
 * ihm fehlte nur die andere Hälfte, und die steht jetzt hier.
 */

/** Was der Rust-Wächter je Weg einsetzt: fünf Ränge mal Promille je Rang. */
const ERWARTET = [
  { id: 'calcination-build', feld: 'resourceGainPerRankPerMille', jeRang: 100 },
  { id: 'calcination-release', feld: 'damagePerRankPerMille', jeRang: 25 },
  { id: 'calcination-hold', feld: 'dotDurationPerRankPerMille', jeRang: 100 },
  { id: 'global-release', feld: 'spenderDamagePerRankPerMille', jeRang: 100 },
  { id: 'global-build', feld: 'resourceGainPerRankPerMille', jeRang: 60 },
  { id: 'global-hold', feld: 'dotDurationPerRankPerMille', jeRang: 60 },
] as const;

describe('Die Zahlen des AC3-Wächters stehen so in den Daten', () => {
  it.each(ERWARTET)('$id trägt $jeRang Promille in $feld', ({ id, feld, jeRang }) => {
    const talent = TALENTS.find((t) => t.id === id);
    expect(talent, `${id} gibt es nicht mehr`).toBeDefined();
    expect(
      (talent as unknown as Record<string, number>)[feld],
      `${id}.${feld} hat sich geändert. Der Beleg in sim/tests/ac3_wege_gewinnen.rs ` +
        `rechnet weiter mit ${jeRang} je Rang und bliebe grün, obwohl er eine Lage ` +
        `belegt, die es nicht mehr gibt. Beide Stellen nachziehen.`,
    ).toBe(jeRang);
  });

  it('und fünf Ränge sind überall die Obergrenze', () => {
    // Der Wächter setzt „fünf Ränge" als 5 × jeRang. Wäre `maxRanks` kleiner,
    // rechnete er mit einer Verteilung, die kein Spieler haben kann.
    for (const { id } of ERWARTET) {
      const talent = TALENTS.find((t) => t.id === id);
      expect(talent?.maxRanks, `${id} hat nicht mehr fünf Ränge`).toBe(5);
    }
  });
});
