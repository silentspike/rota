/**
 * Trainingsraum (Issue #84).
 *
 * Der Kern ist nicht die Puppe, sondern dass sie ein Boss IST — und dass die
 * Kennzahlen aus einer Stelle kommen statt hier neu gerechnet zu werden.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, PLAYER, type BossDef } from '../../src/core/data.js';
import { PressQueue } from '../../src/input/queue.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import type { RunSummary } from '../../src/core/summary.js';
import {
  buildDummy,
  DEFAULT_DUMMY,
  DUMMY_HP,
  DUMMY_OVERPRESSURE_MS,
  TrainingRoom,
  type DummySettings,
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

/**
 * Ein Uebungsdurchgang gegen eine gegebene Puppe.
 *
 * Es wird WIRKLICH gewirkt: Ein Durchlauf ohne Tastendruck erzeugt null
 * Schaden, und dann zeigt ein Vergleich zweier Widerstaende nichts — 0 ist
 * nicht kleiner als 0. Genau der Fall, vor dem WORKER-GUIDE § 5 warnt: Ist die
 * gemessene Groesse ueberall null, kann der Prueffall nichts belegen.
 */
function runFor(boss: BossDef): RunSummary {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  // **Über den Bezeichner, nicht über die Zahl** (Issue #369): Der Index steht
  // in jeder Aufzeichnung; eine 0 hier zeigt auf den Zauber, der heute vorn
  // steht, und morgen auf einen anderen.
  session.bind(0, combat.indexOf('kindle'));
  // Ohne beginCountdown bleibt die Sitzung in der Vorbereitung, und dann wird
  // nichts gewirkt — der Schaden waere ueberall null.
  session.beginCountdown(0);
  let tick = 0;
  const ENDE = COUNTDOWN_MS + 9000;
  for (let t = 0; t <= ENDE; t += 100) {
    // Nach dem Countdown alle 1,5 s dieselbe Taste (Grundtakt, 6.6).
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
    session.tick(t);
    tick++;
  }
  return session.summary(ENDE);
}

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
    // **Das ist die Vorbedingung, nicht die Wirkung** — und genau diese
    // Verwechslung hat Issue #167 durchgelassen: Die Zeitleiste war nicht
    // leer, trug aber falsche Feldnamen, und es entstand nie eine
    // Unterbrechungs-Gelegenheit. Der Nachweis steht weiter unten in
    // „AC1 (#167)"; dieser Fall bleibt nur als Vorbedingung stehen.
    expect(buildDummy({ ...DEFAULT_DUMMY, castsSpells: false }, 'X').timeline).toEqual([]);
    expect(
      buildDummy({ ...DEFAULT_DUMMY, castsSpells: true }, 'X').timeline.length,
    ).toBeGreaterThan(0);
  });

  it('der Widerstand veraendert den Schaden WIRKLICH, und nur er', () => {
    // Der Test darueber zeigt, dass der Wert gesetzt wird — nicht, dass er
    // wirkt. Ohne diesen hier haette auch eine Puppe bestanden, deren
    // Widerstand nirgends ankommt (WORKER-GUIDE § 5).
    const ohne = runFor(buildDummy({ ...DEFAULT_DUMMY, resistance: 1 }, 'CALCINATION'));
    const halb = runFor(buildDummy({ ...DEFAULT_DUMMY, resistance: 0.5 }, 'CALCINATION'));

    expect(halb.damage).toBeLessThan(ohne.damage);
    // Und zwar um den Faktor, nicht irgendwie: halber Widerstand, halber Schaden.
    expect(halb.damage).toBeCloseTo(ohne.damage * 0.5, 0);
  });

  it('ein neutraler Widerstand aendert NICHTS — die Gegenprobe', () => {
    // Der Fall, in dem sich nichts aendern DARF. Ohne ihn haette auch eine
    // Rechnung bestanden, in der ein anderer Faktor die Arbeit tut.
    const neutral = runFor(buildDummy({ ...DEFAULT_DUMMY, resistance: 1 }, 'CALCINATION'));
    const ohneEinstellung = runFor(buildDummy(DEFAULT_DUMMY, 'CALCINATION'));
    expect(neutral.damage).toBe(ohneEinstellung.damage);
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

/**
 * Ein Uebungsdurchgang, der lang genug fuer die Zeitleiste ist.
 *
 * `runFor` oben endet neun Sekunden nach dem Countdown — vor dem ersten
 * Puppen-Zauber bei 12 s. Ein Fall, der die Zeitleiste messen will, saehe dort
 * nie einen Eintrag und waere trotzdem gruen.
 *
 * Gibt den Kampf selbst zurueck, nicht die Zusammenfassung: Die gesuchten
 * Groessen (Spielerleben, Unterbrechungen) stehen an `Combat`.
 */
function uebeGegen(settings: DummySettings, dauerMs = 30_000): Combat {
  const queue = new PressQueue();
  const combat = new Combat({
    school: CALCINATION.value,
    boss: buildDummy(settings, 'CALCINATION'),
  });
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('kindle'));
  session.beginCountdown(0);
  let tick = 0;
  for (let t = 0; t <= COUNTDOWN_MS + dauerMs; t += 100) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
    session.tick(t);
    tick++;
  }
  return combat;
}

describe('AC1 (#167) — castsSpells erzeugt WIRKLICH Gelegenheiten zum Unterbrechen', () => {
  it('mit Zaubern gibt es mindestens eine, ohne keine', () => {
    // **Gemessen wird die Wirkung im Kampf**, nicht die Laenge einer Liste.
    // Vor #167 war die Liste gefuellt und die Zahl hier trotzdem 0: Die
    // Eintraege trugen `kind` statt `mechanic`, also lief `Combat` in den
    // Schlag-Zweig und begann nie einen unterbrechbaren Zauber.
    const mit = uebeGegen({ ...DEFAULT_DUMMY, castsSpells: true });
    const ohne = uebeGegen({ ...DEFAULT_DUMMY, castsSpells: false });

    expect(mit.interruptsMissed + mit.interruptsLanded).toBeGreaterThan(0);
    // Und der Fall, in dem sich nichts aendern DARF.
    expect(ohne.interruptsMissed + ohne.interruptsLanded).toBe(0);
  });

  it('die Zeitleiste traegt die Feldnamen, die Combat liest', () => {
    // Der Fall, der die Fehlerklasse dauerhaft faengt — auch wenn jemand die
    // Typ-Zusicherung wieder einbaut. `mechanic` ist das Feld, dessen Fehlen
    // den ganzen Schaden angerichtet hat.
    const timeline = buildDummy(
      { ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true },
      'X',
    ).timeline;
    expect(timeline.length).toBeGreaterThan(0);
    for (const eintrag of timeline) {
      expect(['interruptibleCast', 'strike']).toContain(eintrag.mechanic);
      expect(typeof eintrag.name).toBe('string');
      expect(Number.isFinite(eintrag.damage)).toBe(true);
      expect(Number.isFinite(eintrag.castTimeMs)).toBe(true);
    }
    // Und sie steht aufsteigend — Combat laeuft mit einem Zeiger vorwaerts
    // durch und braeche beim ersten nicht faelligen Eintrag ab.
    for (let i = 1; i < timeline.length; i++) {
      expect(timeline[i]!.atMs).toBeGreaterThanOrEqual(timeline[i - 1]!.atMs);
    }
  });
});

describe('AC2 (#167) — dealsDamage senkt das Spielerleben, sonst bleibt es unberuehrt', () => {
  it('beide Faelle im selben Prueffall', () => {
    // `dealsDamage` wurde vor #167 NIRGENDS gelesen. Ein Fall, der nur den
    // eingeschalteten Zustand prueft, koennte gruen sein, weil eine andere
    // Quelle Schaden macht — deshalb stehen beide Richtungen hier zusammen.
    const mit = uebeGegen({ ...DEFAULT_DUMMY, dealsDamage: true });
    const ohne = uebeGegen({ ...DEFAULT_DUMMY, dealsDamage: false });

    expect(mit.playerHp).toBeLessThan(PLAYER.vessel);
    expect(ohne.playerHp).toBe(PLAYER.vessel);
    expect(mit.damageTaken).toBeGreaterThan(0);
    expect(ohne.damageTaken).toBe(0);
  });

  it('gilt auch, waehrend die Puppe zaubert', () => {
    // Die Zusage von `dealsDamage` haengt nicht an der zweiten Einstellung:
    // „macht keinen Schaden" muss auch dann gelten, wenn zugleich gezaubert
    // wird — sonst waere die Einstellung an eine andere gekoppelt, und der
    // Spieler koennte das Zeitfenster nicht ohne Druck ueben.
    const zaubernd = uebeGegen({ ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: false });
    expect(zaubernd.interruptsMissed + zaubernd.interruptsLanded).toBeGreaterThan(0);
    expect(zaubernd.playerHp).toBe(PLAYER.vessel);
  });
});

describe('AC3 (#167) — das Spielerleben ist in keinem Zustand NaN', () => {
  it('ueber alle vier Kombinationen der beiden Schalter', () => {
    // **`Number.isFinite`, nicht `toBeGreaterThan(0)`.** Jeder Vergleich mit
    // `NaN` ist falsch, `toBeGreaterThan` meldet dann aber nur „nicht
    // groesser" — wer das liest, sucht an der falschen Stelle. Genau dieser
    // Zustand lag vor: `hitPlayer(undefined)` machte das Leben zu `NaN`.
    for (const dealsDamage of [false, true]) {
      for (const castsSpells of [false, true]) {
        const combat = uebeGegen({ ...DEFAULT_DUMMY, dealsDamage, castsSpells });
        const lage = `dealsDamage=${dealsDamage} castsSpells=${castsSpells}`;
        expect(Number.isFinite(combat.playerHp), `${lage}: playerHp ist ${combat.playerHp}`).toBe(
          true,
        );
        expect(
          Number.isFinite(combat.damageTaken),
          `${lage}: damageTaken ist ${combat.damageTaken}`,
        ).toBe(true);
      }
    }
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
