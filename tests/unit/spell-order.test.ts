import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import ERWARTET from './spell-order.json' with { type: 'json' };

/**
 * Die Zauberreihenfolge darf **wachsen, aber nicht sich ändern** (Issue #402).
 *
 * # Warum das überhaupt jemanden interessiert
 *
 * Die Reihenfolge in `SPELLS` bestimmt die Indizes, und die Indizes stehen in
 * **jeder aufgezeichneten Runde** — in `tests/parity/recordings/` genauso wie
 * in einem gespeicherten Spielstand. Wird ein Zauber vorne eingefügt, bedeutet
 * jede alte Aufzeichnung etwas anderes als zuvor: dieselbe Datei, andere
 * Zauber, **keine Fehlermeldung**.
 *
 * Bis heute hielten drei Kommentare (#201, #97, #304) diese Regel zusammen.
 * Ein Kommentar hält niemanden auf.
 *
 * # Warum ein Präfix-Vergleich und keine Gleichheit
 *
 * Gleichheit wäre zu streng: Sie bräche auch dann, wenn hinten ein Zauber
 * dazukommt — und genau das ist der erlaubte Weg. Der Wächter prüft deshalb,
 * dass die heutige Liste mit der eingecheckten **beginnt**. Wachsen ja,
 * umsortieren nein.
 *
 * # Warum die erwartete Liste in einer eigenen Datei steht
 *
 * Weil sie sonst aus derselben Quelle käme, die sie bewacht. Eine Liste, die
 * sich beim Laufen selbst herstellt, ist immer grün — sie belegte nur, dass
 * `map` funktioniert. Die JSON ist eingecheckt und ändert sich nur, wenn
 * jemand sie bewusst anfasst.
 */
describe('Zauberreihenfolge (Issue #402)', () => {
  const heute = CALCINATION.value.spells.map((s) => s.id);

  it('beginnt unverändert — kein Zauber wurde vorne oder in der Mitte eingefügt', () => {
    // Position für Position, nicht als Ganzes: Die Meldung soll sagen, WO es
    // auseinanderläuft und WELCHE zwei Namen dort stehen. `toEqual` über zwei
    // Listen nennt bei achtzehn Einträgen nur „nicht gleich".
    for (let i = 0; i < ERWARTET.length; i++) {
      expect(
        heute[i],
        `Position ${i}: erwartet "${ERWARTET[i]}", gefunden "${heute[i] ?? '(fehlt)'}". ` +
          `Ein Zauber wurde eingefügt, entfernt oder umsortiert — das deutet JEDE ` +
          `bestehende Aufzeichnung still um. Neue Zauber gehören ans ENDE.`,
      ).toBe(ERWARTET[i]);
    }
  });

  it('darf hinten gewachsen sein, aber nicht geschrumpft', () => {
    expect(
      heute.length,
      `Die Liste ist kürzer geworden (${heute.length} statt ${ERWARTET.length}). ` +
        `Ein entfernter Zauber verschiebt alle Indizes dahinter.`,
    ).toBeGreaterThanOrEqual(ERWARTET.length);
  });

  it('enthält keinen Namen doppelt', () => {
    // Sonst zeigten zwei Indizes auf denselben Zauber, und der Präfix-Vergleich
    // oben bliebe grün, während `indexOf` etwas anderes liefert als gedacht.
    expect(new Set(heute).size, `Doppelte Zauber-ID in SPELLS`).toBe(heute.length);
  });
});
