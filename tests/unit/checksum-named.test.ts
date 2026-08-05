/**
 * Der Name geht nicht in die Prüfsumme ein (Issue #236, AC5).
 *
 * DIESER PRÜFFALL TRÄGT DEN GANZEN ANSATZ
 * ---------------------------------------
 * `mixNamedInt` und `mixNamedFloat` geben den Feldnamen mit, damit der
 * Gleichlauf-Wächter sagen kann, WO zwei Seiten auseinanderlaufen — statt nur
 * „Prüfsumme stimmt nicht". In #97 hat diese Auskunft eine Stunde Suche
 * gekostet.
 *
 * **Der Name darf dabei nichts verändern.** Ginge er in die Summe ein, wäre
 * jede eingecheckte Vergleichsdatei im Projekt falsch, die Referenz 4240
 * würde sich bewegen, und der Gleichlauf mit dem Simulator wäre gebrochen —
 * für einen Wächter, der ein Problem melden soll, das es dann erst gäbe.
 *
 * Deshalb steht dieser Fall zuerst und nicht zuletzt: Fällt er, ist der ganze
 * Ansatz tot.
 */
import { describe, expect, it } from 'vitest';
import {
  mixFloat,
  mixInt,
  mixNamedFloat,
  mixNamedInt,
  recordFieldOrder,
} from '../../src/core/checksum.js';

const START = 0x811c9dc5;

describe('Der Name verändert die Prüfsumme nicht (AC5)', () => {
  it('mischt ganze Zahlen wie zuvor', () => {
    let ohne = START;
    let mit = START;
    for (const wert of [0, 1, -1, 255, 256, 4240, 40_510, -999_999]) {
      ohne = mixInt(ohne, wert);
      mit = mixNamedInt(mit, 'ein beliebiger name', wert);
    }
    expect(mit).toBe(ohne);
  });

  it('mischt Fließkommazahlen wie zuvor', () => {
    let ohne = START;
    let mit = START;
    for (const wert of [0, 1.5, -0.001, 9893.4, 1e-9, Number.MAX_SAFE_INTEGER]) {
      ohne = mixFloat(ohne, wert);
      mit = mixNamedFloat(mit, 'ein anderer name', wert);
    }
    expect(mit).toBe(ohne);
  });

  it('ergibt dasselbe, egal wie der Name lautet', () => {
    // **Der schärfere Fall.** Die beiden oben zeigen, dass EIN Name nichts
    // ändert. Dieser zeigt, dass es auf den Namen überhaupt nicht ankommt —
    // sonst könnte ein Aufruf ihn versehentlich mitmischen und die Prüfung
    // bliebe grün, solange alle Namen gleich lang sind.
    const kurz = mixNamedInt(START, 'a', 4240);
    const lang = mixNamedInt(START, 'ein sehr viel längerer feldname', 4240);
    const leer = mixNamedInt(START, '', 4240);
    expect(lang).toBe(kurz);
    expect(leer).toBe(kurz);
    expect(kurz).toBe(mixInt(START, 4240));
  });
});

describe('Die Namensfolge wird zur Laufzeit aufgezeichnet', () => {
  it('sammelt die Namen in der Reihenfolge ihrer Mischung', () => {
    // **Der Endwert wird mitgeprüft, nicht weggeworfen.** Die erste Fassung
    // sammelte nur die Namen ein und liess die Prüfsumme liegen — eslint hat
    // das als ungenutzte Variable gemeldet, und die Meldung hatte recht: Ein
    // Aufzeichner, der die Reihenfolge richtig meldet und dabei die Zahl
    // verändert, wäre hier durchgekommen. Beide Wege müssen auf dieselbe Zahl
    // führen, sonst ist jede eingecheckte Vergleichsdatei falsch.
    let mitNamen = START;
    const folge = recordFieldOrder(() => {
      mitNamen = mixNamedInt(mitNamen, 'erstes', 1);
      mitNamen = mixNamedFloat(mitNamen, 'zweites', 2);
      mitNamen = mixNamedInt(mitNamen, 'drittes', 3);
    });
    expect(folge).toEqual(['erstes', 'zweites', 'drittes']);
    expect(mitNamen).toBe(mixInt(mixFloat(mixInt(START, 1), 2), 3));
  });

  it('zeichnet eine Schleife so oft auf, wie sie läuft', () => {
    // **Der Grund, warum zur Laufzeit gesammelt wird und nicht aus dem
    // Quelltext.** Eine Quelltext-Stelle ist nicht ein Wert: Diese eine Zeile
    // erzeugt drei Einträge. Wer Zeilen zählt, vergleicht etwas anderes als
    // wer Werte zählt — und bekommt eine Abweichung gemeldet, die keine ist.
    let mitNamen = START;
    const folge = recordFieldOrder(() => {
      for (let i = 0; i < 3; i++) mitNamen = mixNamedInt(mitNamen, 'in der schleife', i);
    });
    expect(mitNamen).toBe(mixInt(mixInt(mixInt(START, 0), 1), 2));
    expect(folge).toEqual(['in der schleife', 'in der schleife', 'in der schleife']);
  });

  it('zeichnet außerhalb der Aufzeichnung nichts auf', () => {
    // Im laufenden Spiel darf im Takt nichts entstehen (18.2). Ohne diese
    // Zeile könnte die Liste unbemerkt mitwachsen — ein Speicherleck, das
    // niemand sieht, weil die Prüfsummen weiterhin stimmen.
    mixNamedInt(START, 'außerhalb', 1);
    const folge = recordFieldOrder(() => {
      mixNamedInt(START, 'innerhalb', 1);
    });
    expect(folge).toEqual(['innerhalb']);
  });

  it('hört auch dann auf aufzuzeichnen, wenn der Lauf wirft', () => {
    // Sonst bliebe die Aufzeichnung nach einem gescheiterten Prüffall an, und
    // der nächste Lauf im Spiel würde stillschweigend Speicher erzeugen.
    expect(() =>
      recordFieldOrder(() => {
        throw new Error('absichtlich');
      }),
    ).toThrow('absichtlich');

    const danach = recordFieldOrder(() => {
      mixNamedInt(START, 'sauber', 1);
    });
    expect(danach).toEqual(['sauber']);
  });
});
