import { describe, expect, it } from 'vitest';
import { CALCINATION, LEVELS } from '../../src/core/data.js';

/**
 * Welche Zauber werden in einem Paritäts-Weg **gewirkt** (Issue #340)?
 *
 * # Der Befund, aus dem dieses Issue entstand
 *
 * Von fünfzehn ausgelieferten Zaubern wurden sieben in **keiner** Aufzeichnung
 * und in keiner Payload je gedrückt. Für sie gilt die Bitgleichheit zwischen
 * Spiel und Simulator als angenommen, nicht als geprüft.
 *
 * # Gedrückt, nicht gebunden — und das ist der Kern
 *
 * Die erste Zählung dazu ging über die **Namen** in den Dateien und kam auf
 * acht abgedeckte Zauber. Das war falsch: Ein Name steht dort auch, wenn der
 * Zauber nur auf der Leiste liegt und nie gedrückt wird. `worn-values.json`
 * führt „canker" genau einmal — in der Zauberliste.
 *
 * Dieser Wächter löst deshalb jeden `press` über die `bindings` seiner
 * Aufzeichnung auf. Ein Zauber zählt erst, wenn eine Taste auf seinen Platz
 * fällt.
 *
 * # Die Ausnahme steht in den DATEN, nicht hier
 *
 * `test-cast` trägt `unlockLevel: 101` gegen `maxLevel: 100` — er ist damit
 * kein ausgelieferter Zauber, sondern ein Prüf-Zauber. Der Wächter fragt die
 * Stufengrenze ab und kennt seinen Namen nicht. Wer einen zweiten Prüf-Zauber
 * anlegt, muss diese Datei nicht anfassen.
 */

const RECORDINGS = import.meta.glob('/tests/parity/recordings/*.json', {
  import: 'default',
  eager: true,
}) as Record<string, Record<string, unknown>>;

interface Press {
  readonly slot: number;
}
interface Binding {
  readonly slot: number;
  readonly spellId: string;
}

/** Alle Zauber, die das Spiel je ausliefert — ohne die Prüf-Zauber. */
function ausgelieferteZauber(): string[] {
  return CALCINATION.value.spells
    .filter((spell) => spell.unlockLevel <= LEVELS.maxLevel)
    .map((spell) => spell.id);
}

/** Wie oft jeder Zauber in den Aufzeichnungen **gedrückt** wird. */
function druecke(
  dateien: Record<string, Record<string, unknown>> = RECORDINGS,
): Map<string, number> {
  const zaehler = new Map<string, number>();
  for (const datei of Object.values(dateien)) {
    const laeufe = Array.isArray(datei['runs'])
      ? (datei['runs'] as Record<string, unknown>[])
      : [datei];
    for (const lauf of laeufe) {
      const bindings = (lauf['bindings'] ?? datei['bindings']) as Binding[] | undefined;
      const presses = (lauf['presses'] ?? datei['presses']) as Press[] | undefined;
      if (bindings === undefined || presses === undefined) continue;
      const slotZuZauber = new Map(bindings.map((b) => [b.slot, b.spellId]));
      for (const press of presses) {
        const id = slotZuZauber.get(press.slot);
        // **Kein Rückfall auf Platz 0.** Ein Druck auf einen unbelegten Platz
        // ist eine Fehleingabe und zählt für niemanden (Issue #351).
        if (id === undefined) continue;
        zaehler.set(id, (zaehler.get(id) ?? 0) + 1);
      }
    }
  }
  return zaehler;
}

describe('AC1 — jeder ausgelieferte Zauber wird in einem Paritäts-Weg gewirkt', () => {
  it('findet die Aufzeichnungen überhaupt', () => {
    // **Ohne diese Zusicherung liefe der Wächter auch dann durch, wenn er
    // nichts lädt** — und eine Schleife über null Dateien ist grün. Genau die
    // Klasse, die bei #318 einen Prüffall wertlos gemacht hat.
    expect(Object.keys(RECORDINGS).length).toBeGreaterThan(10);
    expect(druecke().size).toBeGreaterThan(1);
  });

  it('nennt die Namen der ungedeckten Zauber, nicht nur eine Zahl', () => {
    const gedrueckt = druecke();
    const ohne = ausgelieferteZauber().filter((id) => (gedrueckt.get(id) ?? 0) === 0);

    expect(
      ohne,
      `diese Zauber werden in KEINER Aufzeichnung gewirkt — für sie gilt die ` +
        `Bitgleichheit als angenommen, nicht als geprüft:\n  ${ohne.join(', ')}\n` +
        `gedrückt wird: ${[...gedrueckt.entries()].map(([id, n]) => `${id} ${n}`).join(', ')}`,
    ).toEqual([]);
  });
});

describe('AC4 — der Wächter unterscheidet Bindung von Wirkung', () => {
  it('ein Zauber, der nur gebunden ist, zählt nicht', () => {
    // **Gemessen an `druecke()` selbst, nicht an einer Kopie davon.**
    //
    // Hier stand vorher eine nachgebaute Zählschleife — sie prüfte, dass die
    // drei Zeilen darüber richtig zählen, und die taten es. Über die Funktion,
    // die der Wächter wirklich benutzt, sagte sie nichts: Eine Sabotage in
    // `druecke()` (Bindungen mitzählen — genau der Fehler, aus dem dieses
    // Issue entstand) ließ alle vier Fälle grün.
    //
    // Das ist WORKER-GUIDE 8.18: Wer die Zahl nachrechnet, prüft seine eigene
    // Kopie gegen das Original.
    const konstruiert = {
      '/erfunden.json': {
        bindings: [
          { slot: 0, spellId: 'kindle' },
          { slot: 1, spellId: 'canker' },
        ],
        presses: [{ slot: 0 }, { slot: 0 }],
      },
    };

    const gezaehlt = druecke(konstruiert);

    expect(gezaehlt.get('kindle'), 'KINDLE wurde zweimal gedrückt').toBe(2);
    expect(
      gezaehlt.get('canker'),
      'CANKER liegt auf Platz 1 und wird nie gedrückt — er darf nicht zählen',
    ).toBeUndefined();
  });

  it('und ein Druck auf einen unbelegten Platz zählt für niemanden', () => {
    // Die zweite Hälfte derselben Trennung: `?? 0` würde ihn auf Platz 0
    // schieben und dem dortigen Zauber zuschlagen (Issue #351).
    const gezaehlt = druecke({
      '/erfunden.json': {
        bindings: [{ slot: 0, spellId: 'kindle' }],
        presses: [{ slot: 0 }, { slot: 7 }],
      },
    });

    expect(gezaehlt.get('kindle')).toBe(1);
    expect([...gezaehlt.keys()], 'ein Druck ins Leere hat jemandem gezählt').toEqual(['kindle']);
  });
});

describe('AC6 — die Ausnahme für Prüf-Zauber steht in den Daten', () => {
  it('test-cast ist ausgenommen, ohne dass sein Name hier steht', () => {
    const ausgeliefert = ausgelieferteZauber();
    expect(ausgeliefert).not.toContain('test-cast');
    // **Und der Grund ist die Stufengrenze, nicht der Name.** Wer einen
    // zweiten Prüf-Zauber anlegt, muss diese Datei nicht anfassen.
    const pruefzauber = CALCINATION.value.spells.filter(
      (spell) => spell.unlockLevel > LEVELS.maxLevel,
    );
    expect(pruefzauber.map((s) => s.id)).toContain('test-cast');
    expect(ausgeliefert.length).toBe(CALCINATION.value.spells.length - pruefzauber.length);
  });
});
