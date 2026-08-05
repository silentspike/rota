import { describe, expect, it } from 'vitest';

/**
 * Keine Platzangabe entsteht aus einem Vergleich mit einem Zauber (Issue #405).
 *
 * # Warum drei Muster und nicht eines
 *
 * Die elf Stellen trugen ihre Zahl in drei Schreibweisen, und ein `grep` nach
 * `slot: <Ziffer>` fand nur die erste:
 *
 * ```
 * presses.push({ tick, slot: 3 })                       acht Stellen
 * presses.push({ tick, slot: gewaehlt === quench ? 1 : 0 })   eine
 * const platz = gewaehlt === kindle ? 0 : … ? 1 : 3     zwei
 * ```
 *
 * Das Kriterium ist deshalb nicht die Schreibweise, sondern: **Entsteht der
 * Platz aus einem Vergleich mit einem Zauber?**
 *
 * # Eine Suche, die nichts findet, belegt erst etwas, wenn sie etwas finden kann
 *
 * Der zweite Fall setzt die drei Formen testweise wieder ein und prüft, dass
 * jede gemeldet wird. Ohne ihn wäre der erste grün, sobald jemand das Muster
 * kaputtmacht.
 */

const QUELLE = import.meta.glob('/tests/parity/payloads.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/** Die drei Formen, in denen eine Platzangabe von Hand entstehen kann. */
const MUSTER = [/presses\.push\(\{\s*tick,\s*slot:\s*\d/, /slot:\s*\w+ ===/, /const platz\s*=/];

function quelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length < 1000) {
    throw new Error('payloads.ts nicht geladen — die Prüfung würde nichts messen');
  }
  return text;
}

function treffer(text: string): string[] {
  return text
    .split('\n')
    .map((zeile, i) => ({ zeile: zeile.trim(), nummer: i + 1 }))
    .filter(({ zeile }) => MUSTER.some((m) => m.test(zeile)))
    .map(({ zeile, nummer }) => `${nummer}: ${zeile.slice(0, 60)}`);
}

describe('AC1 — kein Platz entsteht aus einem Vergleich mit einem Zauber', () => {
  it('findet keine der drei Formen', () => {
    expect(treffer(quelle()), 'diese Stellen setzen den Platz von Hand').toEqual([]);
  });

  it('und würde jede finden, wenn sie wiederkäme', () => {
    // **Ohne diesen Fall belegt der erste nichts.** Eine Suche, die nichts
    // findet, kann ebenso gut kaputt sein — das ist die Klasse, die uns heute
    // mehrfach begegnet ist.
    const wieder = [
      'presses.push({ tick, slot: 3 });',
      'presses.push({ tick, slot: gewaehlt === quench ? 1 : 0 });',
      'const platz = gewaehlt === kindle ? 0 : 3;',
    ];
    for (const zeile of wieder) {
      expect(treffer(zeile), `nicht gemeldet: ${zeile}`).toHaveLength(1);
    }
  });
});
