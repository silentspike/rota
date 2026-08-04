/**
 * Die Fortschrittsmarke (Issue #82, AC2 und AC3).
 *
 * Der Kern ist nicht, dass die Marke sinkt, sondern dass sie **getrennt**
 * gefuehrt wird und dass eine **Niederlage** sie bewegen kann. Ein Prueffall,
 * der nur einen Boss auf einer Stufe kennt, bestuende auch fuer eine Rechnung
 * ohne jede Trennung.
 */
import { describe, expect, it } from 'vitest';
import {
  attemptFrom,
  markFor,
  markKey,
  markSentence,
  recordAttempt,
  type Attempt,
  type ProgressMark,
} from '../../src/analysis/progress-mark.js';

const NORMAL = 1;
const MYTHISCH = 3;

function versuch(over: Partial<Attempt> = {}): Attempt {
  return {
    bossId: 'first-crucible',
    difficulty: NORMAL,
    remainingPercent: 50,
    victory: false,
    gameVersion: '0.1.0',
    at: 1_000,
    ...over,
  };
}

describe('AC2 — getrennt je Boss UND Schwierigkeitsstufe', () => {
  it('laesst ein Normal-Versuch die Marke auf Mythisch unberuehrt', () => {
    // Erst eine echte Marke auf Mythisch setzen, damit es etwas zu
    // verfaelschen GIBT. Gegen eine leere Liste zeigt der Fall nichts.
    let marks: readonly ProgressMark[] = recordAttempt(
      [],
      versuch({
        difficulty: MYTHISCH,
        remainingPercent: 23,
      }),
    ).marks;

    const vorher = markFor(marks, 'first-crucible', MYTHISCH);
    marks = recordAttempt(marks, versuch({ difficulty: NORMAL, remainingPercent: 2 })).marks;
    const nachher = markFor(marks, 'first-crucible', MYTHISCH);

    // Der Normal-Versuch war DEUTLICH besser (2 gegen 23). Flossen die Stufen
    // zusammen, stuende jetzt 2 auf Mythisch.
    expect(nachher?.bestRemainingPercent).toBe(23);
    expect(nachher).toEqual(vorher);
    // Und auf Normal steht er sehr wohl.
    expect(markFor(marks, 'first-crucible', NORMAL)?.bestRemainingPercent).toBe(2);
  });

  it('trennt auch zwei Bosse auf derselben Stufe', () => {
    let marks = recordAttempt([], versuch({ bossId: 'a', remainingPercent: 40 })).marks;
    marks = recordAttempt(marks, versuch({ bossId: 'b', remainingPercent: 5 })).marks;
    expect(markFor(marks, 'a', NORMAL)?.bestRemainingPercent).toBe(40);
    expect(markFor(marks, 'b', NORMAL)?.bestRemainingPercent).toBe(5);
    expect(marks).toHaveLength(2);
  });

  it('zaehlt Versuche je Stufe einzeln', () => {
    let marks = recordAttempt([], versuch({ difficulty: NORMAL })).marks;
    marks = recordAttempt(marks, versuch({ difficulty: NORMAL })).marks;
    marks = recordAttempt(marks, versuch({ difficulty: MYTHISCH })).marks;
    expect(markFor(marks, 'first-crucible', NORMAL)?.attempts).toBe(2);
    expect(markFor(marks, 'first-crucible', MYTHISCH)?.attempts).toBe(1);
  });

  it('bildet den Schluessel aus beiden Teilen', () => {
    // Wuerde er nur den Boss nehmen, waeren diese beiden gleich.
    expect(markKey('x', NORMAL)).not.toBe(markKey('x', MYTHISCH));
  });
});

describe('AC3 — eine Niederlage kann Fortschritt sein', () => {
  it('weist einen gescheiterten Versuch als Verbesserung aus', () => {
    const marks = recordAttempt([], versuch({ remainingPercent: 23 })).marks;
    const update = recordAttempt(marks, versuch({ remainingPercent: 19, victory: false }));

    expect(update.improved).toBe(true);
    expect(update.previousPercent).toBe(23);
    expect(update.gainedPercent).toBe(4);
    expect(markFor(update.marks, 'first-crucible', NORMAL)?.defeated).toBe(false);
  });

  it('sagt es dem Spieler in Worten', () => {
    const marks = recordAttempt([], versuch({ remainingPercent: 23 })).marks;
    const attempt = versuch({ remainingPercent: 19 });
    expect(markSentence(recordAttempt(marks, attempt), attempt)).toBe(
      'New best attempt: down to 19 % — 4 points further than before.',
    );
  });

  it('behauptet keinen Fortschritt, wo keiner war', () => {
    // Der Fall, in dem sich nichts aendern darf. Ohne ihn bestuende die
    // Pruefung auch fuer eine Rechnung, die JEDEN Versuch als Fortschritt
    // ausgibt -- und dann waere die Marke wertlos.
    const marks = recordAttempt([], versuch({ remainingPercent: 23 })).marks;
    const schlechter = recordAttempt(marks, versuch({ remainingPercent: 40 }));

    expect(schlechter.improved).toBe(false);
    expect(schlechter.gainedPercent).toBe(0);
    expect(markFor(schlechter.marks, 'first-crucible', NORMAL)?.bestRemainingPercent).toBe(23);
    // Der Versuch zaehlt trotzdem mit.
    expect(markFor(schlechter.marks, 'first-crucible', NORMAL)?.attempts).toBe(2);
  });

  it('nennt beim ersten Versuch keine Verbesserung', () => {
    // Es gibt nichts zu unterbieten. "Verbessert" waere hier eine Aussage
    // ueber einen Vergleich, den es nicht gab.
    const update = recordAttempt([], versuch({ remainingPercent: 60 }));
    expect(update.improved).toBe(false);
    expect(update.previousPercent).toBeNull();
    expect(markSentence(update, versuch({ remainingPercent: 60 }))).toBe(
      'Best attempt: down to 60 %.',
    );
  });
});

describe('Der Sieg', () => {
  it('setzt die Marke auf 0 und merkt sich den Sieg', () => {
    const update = recordAttempt([], versuch({ remainingPercent: 0, victory: true }));
    const mark = markFor(update.marks, 'first-crucible', NORMAL);
    expect(mark?.bestRemainingPercent).toBe(0);
    expect(mark?.defeated).toBe(true);
    expect(markSentence(update, versuch({ remainingPercent: 0, victory: true }))).toBe('Defeated.');
  });

  it('wird von einer spaeteren Niederlage nicht zurueckgenommen', () => {
    let marks = recordAttempt([], versuch({ remainingPercent: 0, victory: true })).marks;
    marks = recordAttempt(marks, versuch({ remainingPercent: 80, victory: false })).marks;
    const mark = markFor(marks, 'first-crucible', NORMAL);
    expect(mark?.defeated).toBe(true);
    expect(mark?.bestRemainingPercent).toBe(0);
  });
});

describe('Fassung und Zeitpunkt gehoeren zur Marke', () => {
  it('bleiben stehen, wenn der neue Versuch schlechter war', () => {
    // Sie sagen, WANN und WORUNTER die Bestleistung entstand -- nicht, wann
    // zuletzt gespielt wurde. Ueberschriebe sie jeder Versuch, stuende an der
    // Marke die Fassung eines Laufs, der sie nie erreicht hat.
    const marks = recordAttempt(
      [],
      versuch({
        remainingPercent: 23,
        gameVersion: '0.1.0',
        at: 1000,
      }),
    ).marks;
    const spaeter = recordAttempt(
      marks,
      versuch({
        remainingPercent: 40,
        gameVersion: '0.2.0',
        at: 9000,
      }),
    ).marks;

    const mark = markFor(spaeter, 'first-crucible', NORMAL);
    expect(mark?.gameVersion).toBe('0.1.0');
    expect(mark?.at).toBe(1000);
  });

  it('ziehen mit, wenn die Marke faellt', () => {
    const marks = recordAttempt(
      [],
      versuch({
        remainingPercent: 23,
        gameVersion: '0.1.0',
        at: 1000,
      }),
    ).marks;
    const besser = recordAttempt(
      marks,
      versuch({
        remainingPercent: 5,
        gameVersion: '0.2.0',
        at: 9000,
      }),
    ).marks;

    const mark = markFor(besser, 'first-crucible', NORMAL);
    expect(mark?.gameVersion).toBe('0.2.0');
    expect(mark?.at).toBe(9000);
  });
});

describe('Die uebergebene Liste bleibt unangetastet', () => {
  it('gibt eine neue Liste zurueck', () => {
    // Der Aufrufer entscheidet, ob er ablegt. Ein halb geschriebener
    // Spielstand darf keine Marke verfaelschen.
    const marks = recordAttempt([], versuch({ remainingPercent: 50 })).marks;
    const kopie = JSON.stringify(marks);
    recordAttempt(marks, versuch({ remainingPercent: 5 }));
    expect(JSON.stringify(marks)).toBe(kopie);
  });
});

describe('Die Umrechnung aus der Kurzfassung', () => {
  const KONTEXT = {
    bossId: 'first-crucible',
    difficulty: NORMAL,
    gameVersion: '0.1.0',
    at: 1000,
  };

  it('dreht den Fortschritt in verbliebene Gesundheit um', () => {
    // 77 % geschafft heisst 23 % uebrig. Genau die Zahl aus dem Design:
    // "Reaction down to 23 %".
    expect(attemptFrom({ progressPercent: 77, victory: false }, KONTEXT).remainingPercent).toBe(23);
  });

  it('macht aus einem Sieg eine Null', () => {
    const versuchAusSieg = attemptFrom({ progressPercent: 100, victory: true }, KONTEXT);
    expect(versuchAusSieg.remainingPercent).toBe(0);
    expect(versuchAusSieg.victory).toBe(true);
  });

  it('bleibt bei Ueberschlag nicht negativ', () => {
    // Mehr Schaden als Leben gibt es rechnerisch; eine negative verbliebene
    // Gesundheit gibt es nicht.
    expect(attemptFrom({ progressPercent: 130, victory: true }, KONTEXT).remainingPercent).toBe(0);
  });

  it('kehrt die Marke NICHT um — der Fall, den die Funktion verhindert', () => {
    // Der eigentliche Grund fuer attemptFrom. Wer progressPercent direkt als
    // remainingPercent einsetzt, dreht die ganze Marke um: Der schlechtere
    // Durchlauf gaelte als Verbesserung.
    const schwach = { progressPercent: 20, victory: false };
    const stark = { progressPercent: 80, victory: false };

    let marks = recordAttempt([], attemptFrom(schwach, KONTEXT)).marks;
    const richtig = recordAttempt(marks, attemptFrom(stark, KONTEXT));
    expect(richtig.improved).toBe(true);
    expect(markFor(richtig.marks, 'first-crucible', NORMAL)?.bestRemainingPercent).toBe(20);

    // Und so saehe es aus, wenn jemand die Umrechnung vergisst -- hier
    // absichtlich falsch gebaut, damit der Unterschied im Prueffall steht.
    marks = recordAttempt([], { ...KONTEXT, remainingPercent: 20, victory: false }).marks;
    const verdreht = recordAttempt(marks, {
      ...KONTEXT,
      remainingPercent: 80,
      victory: false,
    });
    expect(verdreht.improved).toBe(false);
  });
});
