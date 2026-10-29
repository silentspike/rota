// @vitest-environment jsdom

/**
 * Alle vierzehn Instanzen mit Namen und Thema (Issue #1177, Abschnitt 1).
 *
 * # Was festliegt und was Arbeitsstand ist
 *
 * **Fest** (Masterplan „Umfang V1", Entscheidung 29.10.): 14 Instanzen, acht
 * Aufstiegs-Instanzen à vier Bosse und sechs Endgame-Instanzen à sechs.
 * Prozess-Schwerpunkt erst ab Instanz 5 (f-instanz-prozess-schwerpunkt),
 * Instanz 5 = COAGULATION (f-instanz5-schwerpunkt-schule), die achte führt alle
 * fünf zusammen.
 *
 * **Arbeitsstand** (Freigabe 28.10.): jeder Name, jedes Thema und der
 * Schwerpunkt ab Instanz 6. Gekennzeichnet über `workingName`, damit der
 * Auftraggeber sie ändern kann — dieser Fall hält fest, dass das Kennzeichen
 * an jeder Instanz steht.
 *
 * # Die leere Instanz bricht nichts
 *
 * Acht Instanzen haben noch keinen Boss. Sie stehen trotzdem in den Daten,
 * damit die Boss-Pakete sie vorfinden. Gemessen wird an den drei Lesern:
 * Freischaltung, wählbare Bosse und Boss-Wahl.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES, INSTANCES, instanceById } from '../../src/core/data.js';
import {
  buildBossSelect,
  freigeschalteteInstanzen,
  waehlbareBosse,
} from '../../src/progression/boss-select.js';

const OHNE_PROLOG = INSTANCES.filter((i) => i.id !== 'prologue');

describe('Umfang V1 — vierzehn Instanzen, 68 Boss-Plätze', () => {
  it('der Prolog und die Instanzen 1 bis 14, in dieser Reihenfolge', () => {
    expect(INSTANCES.map((i) => i.id)).toEqual([
      'prologue',
      ...Array.from({ length: 14 }, (_, k) => `instance-${k + 1}`),
    ]);
  });

  it('acht Aufstiegs-Instanzen à vier, sechs Endgame-Instanzen à sechs', () => {
    const aufstieg = OHNE_PROLOG.filter((i) => i.stage === 'ascent');
    const endgame = OHNE_PROLOG.filter((i) => i.stage === 'endgame');
    expect(aufstieg.map((i) => i.plannedBosses)).toEqual([4, 4, 4, 4, 4, 4, 4, 4]);
    expect(endgame.map((i) => i.plannedBosses)).toEqual([6, 6, 6, 6, 6, 6]);
    // Und zusammen die 68 aus dem Plan.
    expect(OHNE_PROLOG.reduce((summe, i) => summe + i.plannedBosses, 0)).toBe(68);
  });

  it('die Beute-Nummer läuft von 1 bis 14 mit der Instanz', () => {
    // `number` ist die Nummer aus src/loot/rates.ts (INSTANCE_COUNT = 14).
    expect(OHNE_PROLOG.map((i) => i.number)).toEqual(Array.from({ length: 14 }, (_, k) => k + 1));
  });

  it('jede Instanz verlangt die vorige — eine Kette ohne Lücke', () => {
    for (let k = 2; k <= 14; k++) {
      expect(instanceById(`instance-${k}`)?.requires).toBe(`instance-${k - 1}`);
    }
    expect(instanceById('instance-1')?.requires).toBeNull();
  });
});

describe('Namen und Themen — Arbeitsstand, gekennzeichnet', () => {
  it('jede Instanz trägt einen Namen statt eines Platzhalters', () => {
    // „Instance One" bis „Instance Five" standen bis #1177 in den Daten.
    for (const instanz of INSTANCES) {
      expect(instanz.name, instanz.id).not.toMatch(/^Instance\b|^Prologue$/);
      expect(instanz.name.length, instanz.id).toBeGreaterThan(0);
    }
  });

  it('und ein Thema, englisch', () => {
    for (const instanz of INSTANCES) {
      expect(instanz.theme.length, `${instanz.id}: kein Thema`).toBeGreaterThan(0);
      expect(instanz.theme, instanz.id).not.toMatch(/[äöüßÄÖÜ„“]/);
    }
  });

  it('alle Namen sind als Arbeitsnamen gekennzeichnet', () => {
    // Fällt, sobald der Auftraggeber einen Namen festlegt und das Kennzeichen
    // entfernt — dann gehört der Fall angepasst, und das ist sein Zweck.
    expect(INSTANCES.filter((i) => !i.workingName).map((i) => i.id)).toEqual([]);
  });

  it('keine zwei Instanzen heißen gleich', () => {
    expect(new Set(INSTANCES.map((i) => i.name)).size).toBe(INSTANCES.length);
  });
});

describe('Der Prozess-Schwerpunkt — erst ab Instanz 5', () => {
  it('Prolog und Instanz 1 bis 4 bleiben schwerpunktfrei', () => {
    for (const id of ['prologue', 'instance-1', 'instance-2', 'instance-3', 'instance-4']) {
      expect(instanceById(id)?.focus, id).toBeNull();
    }
  });

  it('Instanz 5 ist COAGULATION, die achte und die letzte führen alle fünf zusammen', () => {
    expect(instanceById('instance-5')?.focus).toBe('COAGULATION');
    expect(instanceById('instance-8')?.focus).toBe('ALL');
    expect(instanceById('instance-14')?.focus).toBe('ALL');
  });

  it('ab Instanz 5 hat jede einen, und jede Schule kommt im Endgame vor', () => {
    const ab5 = OHNE_PROLOG.filter((i) => i.number >= 5);
    for (const instanz of ab5) expect(instanz.focus, instanz.id).not.toBeNull();
    const endgame = new Set(OHNE_PROLOG.filter((i) => i.stage === 'endgame').map((i) => i.focus));
    for (const schule of [
      'CALCINATION',
      'COAGULATION',
      'FIXATION',
      'PUTREFACTION',
      'SUBLIMATION',
    ]) {
      expect(endgame, schule).toContain(schule);
    }
  });
});

describe('Eine Instanz ohne Boss bricht nichts', () => {
  // **Alle Bosse bis einschliesslich Instanz 5** — erst dann ist Instanz 5
  // selbst offen UND abgeschlossen. Nur ihre vier reichten nicht: Ohne die
  // davor bliebe sie gesperrt, und der Fall prüfte eine Kette, die nie anfängt.
  const instanz5 = [...BOSSES.values()]
    .map((b) => b.value)
    .filter((b) => /^(prologue|instance-[1-5])$/.test(b.instance))
    .map((b) => b.id);

  it('ZUSICHERUNG: instance-6 hat heute keinen Boss', () => {
    // Sonst prüfte der Fall darunter eine gefüllte Instanz und sagte über
    // die leere nichts.
    const drin = [...BOSSES.values()].filter((b) => b.value.instance === 'instance-6');
    expect(drin).toEqual([]);
  });

  it('nach Instanz 5 ist instance-6 offen, aber ohne wählbaren Boss', () => {
    expect(freigeschalteteInstanzen(instanz5, BOSSES)).toContain('instance-6');
    const waehlbar = waehlbareBosse(BOSSES, instanz5);
    expect(waehlbar.filter((b) => b.instance === 'instance-6')).toEqual([]);
    // Und die gebauten bleiben wählbar.
    expect(waehlbar.some((b) => b.id === 'boss-5-1')).toBe(true);
  });

  it('eine leere Instanz öffnet die nächste NICHT', () => {
    // `abgeschlossen` zählt eine Instanz ohne Bosse nicht als abgeschlossen —
    // sonst öffnete instance-6 lautlos instance-7 und alles danach.
    expect(freigeschalteteInstanzen(instanz5, BOSSES)).not.toContain('instance-7');
  });

  it('die Boss-Wahl zeigt keine leere Gruppe', () => {
    const wurzel = buildBossSelect(waehlbareBosse(BOSSES, instanz5), '', () => undefined);
    expect(wurzel.querySelector('[data-instance="instance-6"]')).toBeNull();
    expect(wurzel.querySelector('[data-instance="instance-5"]')).not.toBeNull();
  });
});

describe('Die Boss-Wahl zeigt Name und Thema', () => {
  it('unter dem Namen steht das Thema der Instanz', () => {
    const wurzel = buildBossSelect(waehlbareBosse(BOSSES, []), '', () => undefined);
    const gruppe = wurzel.querySelector('[data-instance="instance-1"]');
    const instanz = instanceById('instance-1');
    expect(gruppe?.querySelector('.boss-select-group-title')?.textContent).toBe(instanz?.name);
    expect(gruppe?.querySelector('.boss-select-group-theme')?.textContent).toBe(instanz?.theme);
  });
});
