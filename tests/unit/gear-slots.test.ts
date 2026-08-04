import { describe, expect, it } from 'vitest';
// Als Namensraum eingebunden, damit sich prüfen lässt, was das Modul NICHT
// mehr ausliefert — ein Einzel-Import einer entfernten Ausfuhr wäre schon ein
// Übersetzungsfehler und käme nie im Prüflauf an.
import * as slots from '../../src/gear/slots.js';
import {
  BASE_STAT_IDS,
  SLOT_ANCHOR,
  SLOT_CATALYST,
  SLOT_COUNT,
  SLOT_FOCUS,
  SLOT_GRIMOIRE,
  SLOT_NAMES,
  SLOT_PHYLACTERY,
  SLOT_PRISM,
  SLOT_RELIC,
  SLOT_SHARD,
  SLOT_SIGIL,
  SLOT_TALISMAN,
} from '../../src/gear/slots.js';
import { QUALITY_AFFIX_SLOTS, QUALITY_COMMON } from '../../src/gear/budget.js';

describe('Zehn Plätze (13.2)', () => {
  it('führt genau die zehn Namen aus dem Design', () => {
    expect(SLOT_COUNT).toBe(10);
    expect(SLOT_NAMES).toEqual([
      'focus',
      'catalyst',
      'sigil',
      'relic',
      'grimoire',
      'shard',
      'talisman',
      'phylactery',
      'anchor',
      'prism',
    ]);
  });

  it('gibt jedem Platz eine eigene Nummer', () => {
    const nummern = [
      SLOT_FOCUS,
      SLOT_CATALYST,
      SLOT_SIGIL,
      SLOT_RELIC,
      SLOT_GRIMOIRE,
      SLOT_SHARD,
      SLOT_TALISMAN,
      SLOT_PHYLACTERY,
      SLOT_ANCHOR,
      SLOT_PRISM,
    ];
    expect(new Set(nummern).size).toBe(SLOT_COUNT);
    expect(Math.min(...nummern)).toBe(0);
    expect(Math.max(...nummern)).toBe(SLOT_COUNT - 1);
  });
});

describe('Ein Platz ist nur ein Platz', () => {
  it('trägt keine platz-spezifische Wertezuordnung mehr', () => {
    // **Bis zum 2026-07-29 stand hier eine.** Focus und Catalyst immer
    // CATALYSIS, Sigil und Talisman immer PURITY/YIELD, und so fort. Sie
    // stand im Plan, sie stand im Design — angeordnet worden war sie nie.
    // Der Auftraggeber hat sie gestrichen: Plätze sind Plätze, wie in WoW
    // oder Diablo.
    //
    // Dieser Prüffall hält den Rückbau fest. Wer die Zuordnung
    // wiedereinführt, muss ihn anfassen — und damit die Frage bewusst
    // beantworten, statt sie beim Schreiben entstehen zu lassen.
    const ausgeliefert = Object.keys(slots).sort();
    for (const verschwunden of [
      'SLOT_SPECIAL',
      'SPECIAL_NAMES',
      'SPECIAL_UNDECIDED',
      'SPECIAL_CATALYSIS',
      'SPECIAL_PURITY_YIELD',
      'SPECIAL_DEFENSE',
      'SPECIAL_SPELL_AMP',
      'isSlotDecided',
      'undecidedSlots',
    ]) {
      expect(ausgeliefert, `${verschwunden} ist zurückgekehrt`).not.toContain(verschwunden);
    }
  });

  it('liefert nur Namen, Nummern und Grundwerte', () => {
    // Die Gegenrichtung: Was das Modul HAT, ist abschließend aufgezählt. Ein
    // neuer Export fällt hier auf, statt sich still einzuschleichen.
    expect(Object.keys(slots).sort()).toEqual([
      'BASE_STAT_IDS',
      'SLOT_ANCHOR',
      'SLOT_CATALYST',
      'SLOT_COUNT',
      'SLOT_FOCUS',
      'SLOT_GRIMOIRE',
      'SLOT_NAMES',
      'SLOT_PHYLACTERY',
      'SLOT_PRISM',
      'SLOT_RELIC',
      'SLOT_SHARD',
      'SLOT_SIGIL',
      'SLOT_TALISMAN',
    ]);
  });
});

describe('Grundwerte auf jedem Gegenstand (13.2)', () => {
  it('führt ESSENCE, VESSEL und WARD', () => {
    expect(BASE_STAT_IDS).toEqual(['essence', 'vessel', 'ward']);
  });

  it('kommen ZUSÄTZLICH zu den Affixen', () => {
    // Die Rechnung, an der es aufgefallen ist: Common bekommt genau EINEN
    // Affix, aber jeder Gegenstand trägt drei Grundwerte. Zögen beide aus
    // demselben Topf, ginge das nicht auf.
    expect(QUALITY_AFFIX_SLOTS[QUALITY_COMMON]).toBe(1);
    expect(BASE_STAT_IDS.length).toBeGreaterThan(QUALITY_AFFIX_SLOTS[QUALITY_COMMON] as number);
  });
});
