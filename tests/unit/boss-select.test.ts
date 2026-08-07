/**
 * Die Boss-Wahl zeigt die spielbaren Bosse und nur die (Issue #613).
 *
 * # Warum der Filter eine Positivliste ist
 *
 * Von den 23 Datensätzen sind 14 Prüfwerkzeug — und ihre Instanz heißt
 * uneinheitlich mal `testing`, mal `test`. Eine Ausschlussliste
 * (`!== 'testing'`) hätte die zweite Gruppe durchgelassen, und der Spieler
 * fände „The Racer" zwischen den echten Bossen. Der Fall unten misst genau
 * das: Er sucht nach beiden Schreibweisen, nicht nach einer.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES } from '../../src/core/data.js';
import { waehlbareBosse } from '../../src/progression/boss-select.js';

const waehlbar = waehlbareBosse(BOSSES);
const ids = waehlbar.map((b) => b.id);

describe('die Auswahl enthält die acht gebauten Bosse und den Prolog', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // Läse `BOSSES` leer — umbenannter Ordner, kaputtes Glob —, wäre alles
    // darunter trivial erfüllt.
    expect(BOSSES.size, 'keine Bossdatensätze gefunden').toBeGreaterThan(20);
  });

  it('alle acht Instanz-Bosse stehen darin', () => {
    for (const id of [
      'boss-1-1',
      'boss-1-2',
      'boss-1-3',
      'boss-1-4',
      'boss-2-1',
      'boss-2-2',
      'boss-2-3',
      'boss-2-4',
    ]) {
      expect(ids, `${id} fehlt in der Auswahl`).toContain(id);
    }
  });

  it('der Prolog steht darin — er war bis heute der einzige erreichbare', () => {
    expect(ids).toContain('first-crucible');
  });

  it('und sonst nichts: genau neun', () => {
    expect(ids).toHaveLength(9);
  });
});

describe('kein Prüf-Boss rutscht durch — in BEIDEN Schreibweisen', () => {
  it('weder die `testing`- noch die `test`-Gruppe erscheint', () => {
    const durchgerutscht = waehlbar.filter(
      (b) => b.instance === 'testing' || b.instance === 'test',
    );
    expect(durchgerutscht.map((b) => b.id)).toEqual([]);
  });

  it('die Zusicherung dazu: es GIBT beide Schreibweisen im Bestand', () => {
    // Ohne diesen Fall bewiese der obige nichts — er wäre auch dann grün, wenn
    // es gar keine Prüf-Bosse gäbe. Und er wird rot, sobald jemand die
    // Schreibweisen vereinheitlicht: Dann gehört der Filter nachgezogen.
    const alle = [...BOSSES.values()].map((e) => e.value.instance);
    expect(alle, 'keine `testing`-Gruppe mehr — Filter prüfen').toContain('testing');
    expect(alle, 'keine `test`-Gruppe mehr — Filter prüfen').toContain('test');
  });

  it('namentlich: The Racer und The Metronome sind nicht wählbar', () => {
    expect(ids).not.toContain('test-racer');
    expect(ids).not.toContain('test-metronome');
  });
});
