import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';

/**
 * Die Zauber-Reihenfolge ist eine Schnittstelle (Issue #369).
 *
 * # Warum sie eine ist und nicht ein Detail
 *
 * Der Index eines Zaubers steht in **jeder** aufgezeichneten Runde — als Zahl,
 * nicht als Name. Eine Aufzeichnung, die `slot 0 → spell 3` sagt, meint den
 * Zauber, der beim Aufzeichnen an Position 3 stand. Verschiebt sich die Liste,
 * zeigt dieselbe Zahl auf einen anderen Zauber.
 *
 * # Der gefährliche Teil: Ein falscher Index ist nicht rot
 *
 * Er ist **grün und misst etwas anderes**. Bei `interrupt-losses` fiel es nur
 * auf, weil an Index 3 ein Hitze-Zauber landete, der gar keine Unterbrechung
 * platziert. Wäre dort ein anderer Unterbrecher gelandet, hätte niemand etwas
 * gemerkt — der Fall wäre weiter grün gewesen und hätte den falschen Zauber
 * geprüft.
 *
 * # Die Reihenfolge folgt keiner ableitbaren Ordnung
 *
 * Nicht alphabetisch, nicht nach Schule, nicht nach Stufe: `quench` aus
 * `common.json` steht vor `crust` aus `coagulation.json`. Sie ist in `data.ts`
 * von Hand zusammengesetzt, und genau deshalb kann kein Leser sie sich
 * herleiten — er muss sie nachschlagen oder sie bricht ihm.
 */

/**
 * Der festgeschriebene Stand.
 *
 * **Diese Liste ist Absicht, keine Bequemlichkeit.** Sie ist der einzige Ort im
 * Projekt, an dem eine Zauber-Reihenfolge ausgeschrieben steht — überall sonst
 * gilt „über den Bezeichner auflösen, nie über die Zahl". Hier ist die Zahl der
 * Gegenstand der Prüfung.
 *
 * Wer einen Zauber ergänzt, hängt ihn **hinten** an (GAME-DESIGN 18.7). Wer
 * ihn woanders einfügt, verschiebt jede Aufzeichnung — und muss diese Liste
 * ändern, was der Wächter zu einer bewussten Entscheidung macht statt zu einem
 * Versehen.
 */
const FESTGESCHRIEBEN = [
  'kindle',
  'smoulder',
  'pyre',
  'quench',
  'crust',
  'salve',
  'temper',
  'bind',
  'stall',
  'stay',
  'anchor',
  'vapour',
  'reflux',
  'canker',
  'fester',
  'test-cast',
] as const;

describe('AC1 — die Zauber-Reihenfolge ist festgeschrieben', () => {
  it('nennt bei einer Abweichung die Namen und beide Positionen', () => {
    const heute = CALCINATION.value.spells.map((spell) => spell.id);

    // **Erst die Verschiebungen einzeln, dann die ganze Liste.** Ein blanker
    // `toEqual` meldete „Arrays sind verschieden" und ließe den Leser die
    // Stelle suchen; bei sechzehn Einträgen ist das eine Minute Arbeit für
    // etwas, das die Prüfung schon weiß.
    const verschoben = FESTGESCHRIEBEN.map((id, alt) => ({ id, alt, neu: heute.indexOf(id) }))
      .filter((e) => e.alt !== e.neu)
      .map((e) => `${e.id}: ${e.alt} → ${e.neu === -1 ? 'nicht mehr da' : e.neu}`);

    expect(
      verschoben,
      'Die Zauber-Reihenfolge hat sich verschoben. Sie steht als ZAHL in jeder ' +
        'Aufzeichnung unter tests/parity/recordings/ — dieselbe Zahl zeigt jetzt ' +
        'auf einen anderen Zauber.\n' +
        'Neue Zauber gehören ans ENDE der Liste (GAME-DESIGN 18.7). Ist das ' +
        'geschehen und die Verschiebung gewollt, gehören die Aufzeichnungen neu ' +
        'erzeugt und diese Liste nachgezogen.',
    ).toEqual([]);

    // Und der Gegenfall zur Verschiebung: ein Zauber, der dazugekommen ist.
    expect(heute.slice(0, FESTGESCHRIEBEN.length)).toEqual([...FESTGESCHRIEBEN]);
  });

  it('ein neuer Zauber steht hinten, nicht dazwischen', () => {
    // **Der Fall, der die Regel trägt.** Hinten anhängen ist erlaubt und
    // ändert keine bestehende Zahl; dazwischen einfügen verschiebt alles
    // danach.
    const heute = CALCINATION.value.spells.map((spell) => spell.id);
    expect(heute.length).toBeGreaterThanOrEqual(FESTGESCHRIEBEN.length);
    for (let i = 0; i < FESTGESCHRIEBEN.length; i++) {
      expect(heute[i], `Position ${i} trägt nicht mehr ${FESTGESCHRIEBEN[i]}`).toBe(
        FESTGESCHRIEBEN[i],
      );
    }
  });
});
