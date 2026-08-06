/**
 * Verlauf ueber alle Durchlaeufe und die Waechter-Ausnahme (Issue #82).
 *
 * DER WICHTIGERE TEIL STEHT UNTEN, NICHT OBEN.
 * Dass der Schaden jetzt gespeichert werden DARF, ist die leichte Haelfte. Die
 * schwere ist, dass die Ausnahme NUR dort greift -- eine Ausnahme, die zu breit
 * faellt, hebt den Waechter auf, ohne dass es auffaellt. Ein Prueffall, der
 * bloss zeigt, dass runHistory durchgeht, bestuende auch fuer eine Ausnahme,
 * die schlicht /damage/i aus der Liste nimmt.
 */
import { describe, expect, it } from 'vitest';
import {
  findForbiddenFields,
  NAMED_EXCEPTIONS,
  SAVE_SCHEMA,
  type RunHistoryEntry,
  type SaveData,
} from '../../src/analysis/save-format.js';
import {
  appendRun,
  historyFor,
  historySegments,
  HISTORY_LIMIT,
} from '../../src/analysis/run-history.js';

function eintrag(over: Partial<RunHistoryEntry> = {}): RunHistoryEntry {
  return {
    bossId: 'first-crucible',
    difficulty: 1,
    at: 1_800_000_000_000,
    gameVersion: '0.1.0',
    victory: false,
    elapsedMs: 184_200,
    remainingPercent: 23,
    damage: 63_000,
    idleMs: 6400,
    missedMs: 3000,
    ...over,
  };
}

function stand(over: Partial<SaveData> = {}): SaveData {
  return {
    schema: SAVE_SCHEMA,
    gameVersion: '0.1.0',
    savedAt: 1_800_000_000_000,
    level: 42,
    experience: 128_400,
    talentPoints: {},
    items: [{ itemId: 'ember-band', upgradeSteps: 3, equippedSlot: 2 }],
    bindings: { '0': 'kindle' },
    bestRuns: [],
    defeatedBosses: [],
    runHistory: [eintrag()],
    progressMarks: [],
    ...over,
  };
}

// --------------------------------------------------------------------------
// Die Ausnahme greift — und NUR dort
// --------------------------------------------------------------------------

describe('Die benannte Waechter-Ausnahme', () => {
  it('laesst den Schaden am Verlaufs-Eintrag durch', () => {
    expect(findForbiddenFields(stand())).toEqual([]);
  });

  it('lehnt DENSELBEN Namen an jeder anderen Stelle weiter ab', () => {
    // Der eigentliche Nachweis. Waere die Ausnahme am NAMEN statt am PFAD
    // festgemacht, ginge jede dieser Stellen durch -- und der Waechter waere
    // aufgehoben, ohne dass eine Zeile danach aussieht.
    const stellen: readonly [string, SaveData][] = [
      [
        'auf einem Gegenstand',
        stand({
          items: [{ itemId: 'x', upgradeSteps: 1, equippedSlot: null, damage: 40 } as never],
        }),
      ],
      [
        'an einer Bestzeit',
        stand({
          bestRuns: [
            {
              bossId: 'a',
              difficulty: 1,
              timeMs: 1,
              gameVersion: '0.1.0',
              achievedAt: 1,
              damage: 40,
            } as never,
          ],
        }),
      ],
      [
        'an einer Fortschrittsmarke',
        stand({
          progressMarks: [
            {
              bossId: 'a',
              difficulty: 1,
              bestRemainingPercent: 5,
              attempts: 1,
              defeated: false,
              gameVersion: '0.1.0',
              at: 1,
              damage: 40,
            } as never,
          ],
        }),
      ],
      ['ganz oben im Stand', stand({ damage: 40 } as never)],
    ];

    for (const [wo, verdorben] of stellen) {
      expect(findForbiddenFields(verdorben), wo).toContain('damage');
    }
  });

  it('lehnt ein ANDERES verbotenes Feld am Verlaufs-Eintrag ab', () => {
    // Die Ausnahme gilt genau einem Namen. Sie darf nicht den ganzen Pfad
    // freistellen -- sonst waere runHistory ein Loch im Waechter, durch das
    // spaeter irgendein Werte-Bonus mitwandert.
    const verdorben = stand({
      runHistory: [{ ...eintrag(), spellPower: 3 } as never],
    });
    expect(findForbiddenFields(verdorben)).toContain('spellPower');
  });

  it('lehnt einen ABGEWANDELTEN Namen am erlaubten Pfad ab', () => {
    // "damage" ist freigestellt, "bonusDamage" nicht. Die Ausnahme vergleicht
    // den Namen genau und nicht mit einem Enthaeltsein.
    const verdorben = stand({
      runHistory: [{ ...eintrag(), bonusDamage: 40 } as never],
    });
    expect(findForbiddenFields(verdorben)).toContain('bonusDamage');
  });

  it('greift nicht auf einem gleich benannten Feld weiter unten', () => {
    // runHistory.0.detail.damage ist NICHT runHistory.0.damage. Ein Pfad, der
    // nur mit dem Anfang verglichen wuerde, liesse das durch.
    const verdorben = stand({
      runHistory: [{ ...eintrag(), detail: { damage: 40 } } as never],
    });
    expect(findForbiddenFields(verdorben)).toContain('damage');
  });

  it('traegt zu jeder Ausnahme eine Begruendung', () => {
    // Eine Ausnahme ohne Begruendung ist in einem Jahr eine Ausnahme, deren
    // Grund niemand mehr kennt -- und dann bleibt sie stehen, weil sich keiner
    // traut, sie zu entfernen.
    expect(NAMED_EXCEPTIONS.length).toBeGreaterThan(0);
    for (const ausnahme of NAMED_EXCEPTIONS) {
      expect(ausnahme.why.length, ausnahme.field).toBeGreaterThan(40);
    }
  });
});

// --------------------------------------------------------------------------
// Der Fassungsvermerk und der Schnitt
// --------------------------------------------------------------------------

describe('Der Schnitt beim Fassungswechsel', () => {
  it('fuehrt zwei Fassungen als zwei Abschnitte', () => {
    const verlauf = [
      eintrag({ gameVersion: '0.1.0', at: 1 }),
      eintrag({ gameVersion: '0.1.0', at: 2 }),
      eintrag({ gameVersion: '0.2.0', at: 3 }),
    ];
    const abschnitte = historySegments(verlauf);

    expect(abschnitte).toHaveLength(2);
    expect(abschnitte[0]?.gameVersion).toBe('0.1.0');
    expect(abschnitte[0]?.entries).toHaveLength(2);
    expect(abschnitte[1]?.gameVersion).toBe('0.2.0');
    expect(abschnitte[1]?.startIndex).toBe(2);
  });

  it('laesst eine einzige Fassung ungeschnitten', () => {
    // Der Fall, in dem sich nichts aendern darf. Ohne ihn bestuende die
    // Pruefung auch fuer eine Zerlegung, die JEDEN Eintrag einzeln nimmt --
    // und dann waere der Verlauf keine Linie mehr.
    const verlauf = [eintrag({ at: 1 }), eintrag({ at: 2 }), eintrag({ at: 3 })];
    const abschnitte = historySegments(verlauf);
    expect(abschnitte).toHaveLength(1);
    expect(abschnitte[0]?.entries).toHaveLength(3);
  });

  it('schneidet an jedem WECHSEL, nicht nach Fassung gruppiert', () => {
    // Wer eine aeltere Fassung noch einmal spielt, hat drei Abschnitte.
    // Gruppieren wuerde die Reihenfolge zerreissen, und die Reihenfolge ist
    // der ganze Punkt eines Verlaufs.
    const verlauf = [
      eintrag({ gameVersion: '0.1.0', at: 1 }),
      eintrag({ gameVersion: '0.2.0', at: 2 }),
      eintrag({ gameVersion: '0.1.0', at: 3 }),
    ];
    const abschnitte = historySegments(verlauf);
    expect(abschnitte).toHaveLength(3);
    expect(abschnitte.map((a) => a.gameVersion)).toEqual(['0.1.0', '0.2.0', '0.1.0']);
  });

  it('kommt mit einem leeren Verlauf zurecht', () => {
    expect(historySegments([])).toEqual([]);
  });
});

// --------------------------------------------------------------------------
// Anhaengen und Obergrenze
// --------------------------------------------------------------------------

describe('Der Verlauf waechst und laeuft nicht ueber', () => {
  it('haengt hinten an und laesst die uebergebene Liste in Ruhe', () => {
    const vorher = [eintrag({ at: 1 })];
    const kopie = JSON.stringify(vorher);
    const nachher = appendRun(vorher, eintrag({ at: 2 }));
    expect(nachher).toHaveLength(2);
    expect(nachher[1]?.at).toBe(2);
    expect(JSON.stringify(vorher)).toBe(kopie);
  });

  it('wirft bei Ueberlauf die AELTESTEN weg, nicht die neuesten', () => {
    let verlauf: readonly RunHistoryEntry[] = [];
    for (let i = 0; i < 7; i++) verlauf = appendRun(verlauf, eintrag({ at: i }), 5);

    expect(verlauf).toHaveLength(5);
    // Die beiden aeltesten sind weg, der neueste ist da.
    expect(verlauf[0]?.at).toBe(2);
    expect(verlauf.at(-1)?.at).toBe(6);
  });

  it('haelt weit mehr als die 50 Durchlaeufe aus AC4', () => {
    expect(HISTORY_LIMIT).toBeGreaterThan(50);
  });

  it('filtert je Boss UND Stufe', () => {
    const verlauf = [
      eintrag({ bossId: 'a', difficulty: 1 }),
      eintrag({ bossId: 'a', difficulty: 3 }),
      eintrag({ bossId: 'b', difficulty: 1 }),
    ];
    expect(historyFor(verlauf, 'a', 1)).toHaveLength(1);
    expect(historyFor(verlauf, 'a', 3)).toHaveLength(1);
  });
});
