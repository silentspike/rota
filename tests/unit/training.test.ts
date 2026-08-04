/**
 * Trainingsraum (Issue #84).
 *
 * Der Kern ist nicht die Puppe, sondern dass sie ein Boss IST — und dass die
 * Kennzahlen aus einer Stelle kommen statt hier neu gerechnet zu werden.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import {
  buildDummy,
  DEFAULT_DUMMY,
  DUMMY_HP,
  DUMMY_OVERPRESSURE_MS,
  TrainingRoom,
} from '../../src/analysis/training.js';

describe('Die Puppe — unbegrenzt, ohne Zeitlimit', () => {
  it('haelt laenger, als ein Uebungsdurchgang dauert', () => {
    const dummy = buildDummy(DEFAULT_DUMMY, 'CALCINATION');
    expect(dummy.hp).toBe(DUMMY_HP);
    expect(dummy.overpressureMs).toBe(DUMMY_OVERPRESSURE_MS);
    // Nicht Infinity: Der Wert geht in Pruefsumme und Anzeige ein.
    expect(Number.isFinite(dummy.hp)).toBe(true);
    expect(Number.isFinite(dummy.overpressureMs)).toBe(true);
  });

  it('ist ein gewoehnlicher Boss, kein Sonderfall', () => {
    const dummy = buildDummy(DEFAULT_DUMMY, 'CALCINATION');
    for (const feld of ['schema', 'id', 'name', 'instance', 'hp', 'resistances', 'phases']) {
      expect(dummy, feld).toHaveProperty(feld);
    }
  });
});

describe('AC1 — die drei Einstellungen wirken', () => {
  it('Widerstand landet dort, wo die Schadensformel ihn liest', () => {
    // Combat liest boss.resistances[school] (combat.ts:197).
    const dummy = buildDummy({ ...DEFAULT_DUMMY, resistance: 0.5 }, 'CALCINATION');
    expect(dummy.resistances['CALCINATION']).toBe(0.5);
  });

  it('trifft nur die eigene Schule, nicht pauschal', () => {
    const dummy = buildDummy({ ...DEFAULT_DUMMY, resistance: 0.5 }, 'CALCINATION');
    expect(dummy.resistances['FIXATION']).toBeUndefined();
  });

  it('ohne Zauber keine Zeitleiste, mit Zauber eine', () => {
    expect(buildDummy({ ...DEFAULT_DUMMY, castsSpells: false }, 'X').timeline).toEqual([]);
    expect(
      buildDummy({ ...DEFAULT_DUMMY, castsSpells: true }, 'X').timeline.length,
    ).toBeGreaterThan(0);
  });

  it('wirken in Kombination', () => {
    const dummy = buildDummy(
      { resistance: 0.25, dealsDamage: true, castsSpells: true },
      'PUTREFACTION',
    );
    expect(dummy.resistances['PUTREFACTION']).toBe(0.25);
    expect(dummy.timeline.length).toBeGreaterThan(0);
  });
});

describe('AC3 — Wechsel mitten im Durchgang wird abgelehnt', () => {
  it('laesst die Zusammenstellung zwischen Durchgaengen zu', () => {
    const raum = new TrainingRoom();
    expect(raum.setLoadout(['kindle', 'pyre']).accepted).toBe(true);
    expect(raum.loadout).toEqual(['kindle', 'pyre']);
  });

  it('lehnt sie waehrend des Uebens ab, mit Begruendung', () => {
    const raum = new TrainingRoom();
    raum.setLoadout(['kindle']);
    raum.start();
    const result = raum.setLoadout(['smoulder']);
    expect(result.accepted).toBe(false);
    expect(result.reason).toContain('between runs');
    // Und die alte bleibt stehen, statt halb ueberschrieben zu werden.
    expect(raum.loadout).toEqual(['kindle']);
  });

  it('lehnt auch die Puppen-Einstellungen waehrend des Uebens ab', () => {
    // Ein mitten im Durchgang eingeschalteter Widerstand machte die erste
    // Haelfte unvergleichbar mit der zweiten.
    const raum = new TrainingRoom();
    raum.start();
    const result = raum.setSettings({ ...DEFAULT_DUMMY, resistance: 0.5 });
    expect(result.accepted).toBe(false);
    expect(raum.settings.resistance).toBe(1);
  });

  it('laesst beides nach dem Abbrechen wieder zu', () => {
    const raum = new TrainingRoom();
    raum.start();
    raum.stop();
    expect(raum.setLoadout(['pyre']).accepted).toBe(true);
    expect(raum.setSettings({ ...DEFAULT_DUMMY, resistance: 0.5 }).accepted).toBe(true);
  });
});

describe('Jederzeit abbrechbar', () => {
  it('bricht ohne Bedingung ab', () => {
    const raum = new TrainingRoom();
    raum.start();
    expect(raum.isRunning).toBe(true);
    raum.stop();
    expect(raum.isRunning).toBe(false);
  });

  it('vertraegt ein Abbrechen ohne laufenden Durchgang', () => {
    const raum = new TrainingRoom();
    raum.stop();
    expect(raum.phase).toBe('idle');
  });

  it('startet nicht zweimal', () => {
    const raum = new TrainingRoom();
    raum.start();
    expect(raum.start().accepted).toBe(false);
  });
});

describe('AC4 — dieselbe Auswertung wie nach einem Boss-Kampf', () => {
  /**
   * Zwei Durchlaeufe, einer gegen die Puppe und einer gegen den echten Boss,
   * beide durch dieselbe Session und dieselbe buildSummary.
   *
   * Ein frueherer Prueffall verglich nur die Struktur zweier BossDef. Daraus
   * FOLGT zwar, dass die Auswertung dieselbe ist — aber gemessen war nur die
   * Voraussetzung, nicht die Folgerung. Dieser hier misst die Folgerung.
   */
  function runFor(boss: BossDef): RunSummary {
    const queue = new PressQueue();
    const session = new Session(queue, new Combat({ school: CALCINATION.value, boss }));
    // Kein start() — die Sitzung laeuft ueber tick(), und die Vorbereitung
    // laeuft mit. Nur so lange takten, wie der Vergleich braucht.
    for (let t = 0; t <= 6000; t += 100) session.tick(t);
    return session.summary(6000);
  }

  it('erzeugt dieselben Felder wie ein Boss-Kampf, nicht nur dieselbe Puppe', () => {
    const training = runFor(buildDummy(DEFAULT_DUMMY, 'CALCINATION'));
    const kampf = runFor(FIRST_CRUCIBLE.value);

    expect(Object.keys(training).sort()).toEqual(Object.keys(kampf).sort());
  });

  it('liefert die drei Kennzahlen in beiden Faellen', () => {
    const training = runFor(buildDummy(DEFAULT_DUMMY, 'CALCINATION'));
    for (const feld of [
      'idlePerSpell',
      'lostTimeMs',
      'lostTimeShare',
      'synergyUsed',
      'synergyAvailable',
    ]) {
      expect(training, feld).toHaveProperty(feld);
    }
  });

  it('unterscheidet sich in den WERTEN, nicht in der Form', () => {
    // Die Puppe haelt laenger — das ist der Unterschied, den es geben soll.
    const training = runFor(buildDummy(DEFAULT_DUMMY, 'CALCINATION'));
    const kampf = runFor(FIRST_CRUCIBLE.value);
    expect(training.bossHp).toBeGreaterThan(kampf.bossHp);
  });
});
