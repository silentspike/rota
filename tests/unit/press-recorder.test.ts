/**
 * Der Aufzeichnungs-Helfer (Issue #351).
 *
 * WARUM DIESE DATEI ÜBERHAUPT ETWAS PRÜFEN KANN
 * ---------------------------------------------
 * Der Fehlerfall ist im Bestand **nicht erreichbar**: `misplays.json` drückt
 * die Plätze 0, 1 und 2, und alle drei sind gebunden. Die Rückfallebene `?? 0`
 * hat dort nie gegriffen — sie war toter Code, bis jemand einen ungebundenen
 * Zauber aufzeichnet.
 *
 * Eine Gegenprobe, die nur den Bestand fährt, bliebe deshalb grün und belegte
 * nichts. Sie könnte „fällt nicht mehr still zurück" nicht von „fällt weiterhin
 * zurück, aber die Werte passen zufällig" unterscheiden.
 *
 * Deshalb löst diese Datei den Fall **absichtlich aus** — und weist mit einer
 * nachgebauten Rückfallebene nach, was ohne die Prüfung passiert wäre.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { STEP1_BINDINGS } from '../../src/core/loadout.js';
import { pressRecorder, slotFor, slotMap } from '../../tests/parity/press-recorder.js';
import type { RecordedPress } from '../../src/devtools/recording.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('AC1 — ein ungebundener Zauber bricht ab, statt auf Platz 0 zu landen', () => {
  it('wirft, und die Meldung nennt den Zauber', () => {
    const combat = kampf();
    const presses: RecordedPress[] = [];
    const press = pressRecorder(combat, STEP1_BINDINGS, presses);

    // CANKER liegt in STEP1_BINDINGS auf keinem Platz — genau der Fall, den
    // Worker-2 aufzeichnen wollte, als er die Stelle fand.
    expect(() => {
      press(combat.indexOf('canker'), 0, 0);
    }).toThrow(/canker/);
    // Und nichts ist mitgeschrieben worden.
    expect(presses).toHaveLength(0);
  });

  it('die Meldung nennt auch die Leiste — sonst zeigt sie auf die Folge, nicht die Ursache', () => {
    const combat = kampf();
    const press = pressRecorder(combat, STEP1_BINDINGS, []);
    let text = '';
    try {
      press(combat.indexOf('fester'), 0, 0);
    } catch (fehler) {
      text = fehler instanceof Error ? fehler.message : String(fehler);
    }
    expect(text).toContain('0:kindle');
    expect(text).toContain('3:quench');
  });

  it('GEGENPROBE: ein gebundener Zauber läuft sauber durch', () => {
    // Ohne diese Zeile wäre nicht zu sehen, ob der Helfer überhaupt je etwas
    // durchlässt — einer, der immer wirft, bestünde die Prüfung darüber auch.
    const combat = kampf();
    const presses: RecordedPress[] = [];
    const press = pressRecorder(combat, STEP1_BINDINGS, presses);

    press(combat.indexOf('kindle'), 0, 0);
    press(combat.indexOf('quench'), 5, 500);
    expect(presses).toEqual([
      { tick: 0, slot: 0 },
      { tick: 5, slot: 3 },
    ]);
  });

  it('schreibt den Platz aus DIESER Leiste, nicht aus STEP1_BINDINGS', () => {
    // Der stille Rückfall eine Ebene höher: 19 der 30 Aufzeichnungen bringen
    // seit #318 ihre eigene Leiste mit. Ein Helfer, der die Vorgabe benutzte,
    // schriebe für dieselbe Taste einen anderen Platz, als der Simulator liest.
    const combat = kampf();
    const presses: RecordedPress[] = [];
    const eigene = [
      { slot: 7, spellId: 'canker' },
      { slot: 2, spellId: 'kindle' },
    ];
    const press = pressRecorder(combat, eigene, presses);

    press(combat.indexOf('canker'), 0, 0);
    press(combat.indexOf('kindle'), 1, 100);
    expect(presses).toEqual([
      { tick: 0, slot: 7 },
      { tick: 1, slot: 2 },
    ]);
  });

  it('der Platz wird VOR dem Wirken bestimmt', () => {
    // Wirkte der Helfer zuerst und bräche dann ab, hätte der Kampf einen
    // Zauber gesehen, den die Aufzeichnung nicht kennt — ein Aufrufer, der den
    // Fehler fängt, schriebe ab da eine Datei, die still um einen Druck
    // versetzt ist.
    const combat = kampf();
    const press = pressRecorder(combat, STEP1_BINDINGS, []);
    const vorher = combat.damageDealt;
    expect(() => {
      press(combat.indexOf('canker'), 0, 0);
    }).toThrow();
    expect(combat.damageDealt).toBe(vorher);
  });
});

describe('AC3 — die Gegenprobe: ohne die Prüfung wäre es still Platz 0', () => {
  it('nachgebaut, was vorher dastand', () => {
    // **Die Fehlerart, nicht irgendein Fehler.** Nachgebaut wird genau die
    // Zeile, die bis zu diesem Issue in `payloads.ts` stand:
    //
    //     presses.push({ tick, slot: slotOf.get(spell) ?? 0 });
    //
    // Sie liefert für CANKER eine 0 — und 0 ist in STEP1_BINDINGS KINDLE. Die
    // Datei hiesse CANKER und der Simulator läse KINDLE.
    const combat = kampf();
    const slots = slotMap(combat, STEP1_BINDINGS);
    const canker = combat.indexOf('canker');

    const mitRueckfall = slots.get(canker) ?? 0;
    expect(mitRueckfall).toBe(0);
    // Und Platz 0 trägt einen ANDEREN Zauber — das ist der ganze Schaden.
    expect(STEP1_BINDINGS.find((b) => b.slot === 0)?.spellId).toBe('kindle');
    expect(combat.spells[canker]?.id).toBe('canker');

    // Mit der Prüfung kommt statt der Zahl ein Abbruch.
    expect(() => slotFor(slots, canker, combat)).toThrow();
  });

  it('und der Rückfall wäre von einem echten Wert nicht zu unterscheiden', () => {
    // Der Grund, warum ein Abbruch nötig ist und keine Warnung genügt: Die
    // zurückgegebene 0 sieht aus wie die 0, die KINDLE verdient hat.
    const combat = kampf();
    const slots = slotMap(combat, STEP1_BINDINGS);
    const echterWert = slotFor(slots, combat.indexOf('kindle'), combat);
    const rueckfall = slots.get(combat.indexOf('canker')) ?? 0;
    expect(rueckfall).toBe(echterWert);
  });
});

describe('Der Helfer selbst', () => {
  it('kennt jeden Zauber der übergebenen Leiste', () => {
    const combat = kampf();
    const slots = slotMap(combat, STEP1_BINDINGS);
    expect(slots.size).toBe(STEP1_BINDINGS.length);
    for (const binding of STEP1_BINDINGS) {
      expect(slotFor(slots, combat.indexOf(binding.spellId), combat)).toBe(binding.slot);
    }
  });

  it('wirft auch bei einem Index, den es gar nicht gibt', () => {
    const combat = kampf();
    const slots = slotMap(combat, STEP1_BINDINGS);
    expect(() => slotFor(slots, alsSpellIndex(999), combat)).toThrow(/999/);
  });
});
