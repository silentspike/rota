import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import { bindProblem, bindSpell, sanitize, type Bindings } from '../../src/progression/bindings.js';

/**
 * Ein Zauber liegt auf höchstens einem Platz (GAME-DESIGN 715, Issue #287 AC3).
 *
 * # Warum das eine Regel ist und keine Aufräumarbeit
 *
 * Zwei Plätze mit demselben Zauber sind **keine zwei Einzahler**. Der zweite
 * gibt nur einen zweiten Knopf für dieselbe Quelle — die Vorschau nach
 * GAME-DESIGN 16 („wie viele Zauber je Schul-Mechanik einzahlen") behauptete
 * dann eine Vielfalt, die es nicht gibt, und beantwortete genau die Frage
 * falsch, für die sie da ist.
 *
 * # Zwei Hälften, und die zweite wird gern vergessen
 *
 * Die Sperre in der Auswahl verhindert **neue** Doppelbelegungen. Es gibt aber
 * Spielstände, die unter der alten Regel entstanden sind — für die braucht es
 * die Bereinigung beim Laden. Ein Nachweis, der nur die Auswahl prüft, lässt
 * sie durch.
 */

const SPELLS = CALCINATION.value.spells;
const STUFE = 20;

describe('AC3, erste Hälfte — die Sperre beim Legen', () => {
  it('derselbe Zauber auf einem zweiten Platz wird abgewiesen', () => {
    const gelegt: Bindings = { '0': 'kindle' };
    expect(bindProblem('kindle', 1, STUFE, SPELLS, gelegt)).toBe('already-bound');
  });

  it('auf SEINEM Platz ist er kein Problem', () => {
    // **Der Fall, der eine naive Sperre kaputtmacht.** Wer einen Zauber
    // dorthin legt, wo er schon liegt, ändert nichts. Das als Fehler zu melden
    // hieße, dem Spieler eine Sperre zu zeigen, die keine ist — und die
    // Auswahlliste ließe die aktuelle Wahl aus ihren eigenen Einträgen fallen.
    const gelegt: Bindings = { '0': 'kindle' };
    expect(bindProblem('kindle', 0, STUFE, SPELLS, gelegt)).toBeNull();
  });

  it('ein anderer Zauber auf einen freien Platz geht weiter', () => {
    // **Der Gegenfall.** Ohne ihn bliebe offen, ob die Sperre alles abweist.
    const gelegt: Bindings = { '0': 'kindle' };
    expect(bindProblem('smoulder', 1, STUFE, SPELLS, gelegt)).toBeNull();
  });

  it('bindSpell gibt die unveränderte Belegung zurück', () => {
    // Die Bauart aus #248: Geht es nicht, kommt die alte Belegung zurück —
    // eine halb geänderte wäre schlimmer als gar keine.
    const gelegt: Bindings = { '0': 'kindle', '1': 'smoulder' };
    expect(bindSpell(gelegt, 'kindle', 2, STUFE, SPELLS)).toEqual(gelegt);
    // Und der erlaubte Fall verändert wirklich etwas — sonst prüfte die Zeile
    // darüber nur, dass bindSpell gar nichts tut.
    expect(bindSpell(gelegt, 'pyre', 2, STUFE, SPELLS)).toEqual({
      '0': 'kindle',
      '1': 'smoulder',
      '2': 'pyre',
    });
  });
});

describe('AC3, zweite Hälfte — die Bereinigung beim Laden', () => {
  it('ein alter Spielstand mit Doppelbelegung verliert den zweiten Platz', () => {
    // **Genau der Zustand aus dem Befund**: In #283 lag BIND auf zwei Plätzen.
    // Solche Stände existieren, weil sie unter der alten Regel entstanden sind.
    const alt: Bindings = { '0': 'kindle', '1': 'kindle', '2': 'smoulder' };
    const heil = sanitize(alt, STUFE, SPELLS);

    expect(heil['0'], 'der kleinere Platz behält den Zauber').toBe('kindle');
    expect(heil['1'], 'der zweite Platz ist nicht geräumt').toBeUndefined();
    expect(heil['2'], 'ein unbeteiligter Platz wurde mitgeräumt').toBe('smoulder');
  });

  it('und die Bereinigung ist wiederholbar', () => {
    // **Sonst hinge das Ergebnis an der Reihenfolge**, in der die Plätze
    // ankommen. `sanitize` läuft deshalb aufsteigend nach Platz und nicht in
    // der Reihenfolge der Schlüssel — die ist eine Eigenschaft der Laufzeit
    // und keine Zusage.
    const alt: Bindings = { '2': 'kindle', '0': 'kindle', '1': 'smoulder' };
    const einmal = sanitize(alt, STUFE, SPELLS);
    expect(einmal['0']).toBe('kindle');
    expect(einmal['2']).toBeUndefined();
    expect(sanitize(einmal, STUFE, SPELLS)).toEqual(einmal);
  });

  it('ein sauberer Spielstand bleibt unangetastet', () => {
    // **Der Gegenfall zur Bereinigung selbst.** Ohne ihn behöbe sie den Befund
    // auch dann, wenn sie einfach Plätze räumte — dieselbe Falle wie bei #306,
    // wo die Reparatur die Aufschlüsselung hätte leeren können.
    const sauber: Bindings = { '0': 'kindle', '1': 'smoulder', '2': 'pyre' };
    expect(sanitize(sauber, STUFE, SPELLS)).toEqual(sauber);
  });

  it('drei Plätze mit demselben Zauber lassen genau einen übrig', () => {
    const alt: Bindings = { '0': 'kindle', '1': 'kindle', '2': 'kindle' };
    const heil = sanitize(alt, STUFE, SPELLS);
    expect(Object.values(heil)).toEqual(['kindle']);
  });
});
