import { describe, expect, it } from 'vitest';
import {
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
  BASE_STAT_IDS,
  SLOT_SPECIAL,
  SLOT_TALISMAN,
  SPECIAL_CATALYSIS,
  SPECIAL_DEFENSE,
  SPECIAL_PURITY_YIELD,
  SPECIAL_SPELL_AMP,
  SPECIAL_UNDECIDED,
  isSlotDecided,
  undecidedSlots,
} from '../../src/gear/slots.js';
import { QUALITY_STAT_SLOTS } from '../../src/gear/budget.js';

/** Die meisten Werte-Plätze, die eine Qualität trägt. */
const QUALITY_STAT_SLOTS_MAX = Math.max(...QUALITY_STAT_SLOTS);

describe('Zehn Plätze mit festem Charakter (13.2)', () => {
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
    expect(SLOT_SPECIAL.length).toBe(SLOT_COUNT);
  });

  it('ordnet die acht bestimmten Plätze wie 13.2 zu', () => {
    expect(SLOT_SPECIAL[SLOT_FOCUS]).toBe(SPECIAL_CATALYSIS);
    expect(SLOT_SPECIAL[SLOT_CATALYST]).toBe(SPECIAL_CATALYSIS);
    expect(SLOT_SPECIAL[SLOT_SIGIL]).toBe(SPECIAL_PURITY_YIELD);
    expect(SLOT_SPECIAL[SLOT_TALISMAN]).toBe(SPECIAL_PURITY_YIELD);
    expect(SLOT_SPECIAL[SLOT_ANCHOR]).toBe(SPECIAL_DEFENSE);
    expect(SLOT_SPECIAL[SLOT_PHYLACTERY]).toBe(SPECIAL_DEFENSE);
    expect(SLOT_SPECIAL[SLOT_GRIMOIRE]).toBe(SPECIAL_SPELL_AMP);
    expect(SLOT_SPECIAL[SLOT_PRISM]).toBe(SPECIAL_SPELL_AMP);
  });
});

describe('Grundwerte auf jedem Platz (13.2)', () => {
  it('führt ESSENCE, VESSEL und WARD', () => {
    expect(BASE_STAT_IDS).toEqual(['essence', 'vessel', 'ward']);
  });

  it('sagt NICHT, woher ihre Zahlen kommen', () => {
    // 13.3 verteilt das Budget „auf die Werte-Plätze" und gibt Common genau
    // einen. Ein Platz kann keine drei Grundwerte tragen — also sind sie
    // entweder zusätzlich zum Budget, oder „Werte-Plätze" meint etwas
    // anderes. Beides ist balance-relevant, keines steht im Dokument.
    // Dieser Prüffall hält fest, dass hier nichts geraten wurde.
    expect(BASE_STAT_IDS.length).toBe(3);
    expect(QUALITY_STAT_SLOTS_MAX).toBeLessThan(BASE_STAT_IDS.length + 1);
  });
});

describe('Zwei Plätze warten auf ihre Zuordnung', () => {
  it('nennt Relic und Shard offen, statt etwas zu erfinden', () => {
    // 13.2 zählt zehn Plätze auf und ordnet acht zu. Relic und Shard kommen
    // im ganzen Design-Dokument genau einmal vor — in der Namensliste.
    // Ein geratener Sonderwert wäre eine Design-Entscheidung im Code, und
    // zwar eine, die niemandem auffiele, weil sie plausibel aussähe.
    expect(SLOT_SPECIAL[SLOT_RELIC]).toBe(SPECIAL_UNDECIDED);
    expect(SLOT_SPECIAL[SLOT_SHARD]).toBe(SPECIAL_UNDECIDED);
    expect(undecidedSlots()).toEqual([SLOT_RELIC, SLOT_SHARD]);
  });

  it('meldet je Platz, ob er bestimmt ist', () => {
    expect(isSlotDecided(SLOT_FOCUS)).toBe(true);
    expect(isSlotDecided(SLOT_RELIC)).toBe(false);
    expect(isSlotDecided(SLOT_SHARD)).toBe(false);
    // Ein Platz außerhalb der zehn gilt nicht als bestimmt.
    expect(isSlotDecided(SLOT_COUNT)).toBe(false);
    expect(isSlotDecided(-1)).toBe(false);
  });

  it('lässt genau zwei offen — nicht mehr und nicht weniger', () => {
    // Die Stolperdraht-Zeile: Wer eine Zuordnung nachträgt, muss diesen
    // Prüffall anfassen und damit die Frage bewusst beantworten. Wer
    // versehentlich eine dritte Lücke einbaut, fällt hier auf.
    expect(undecidedSlots().length).toBe(2);
    const bestimmt = SLOT_SPECIAL.filter((s) => s !== SPECIAL_UNDECIDED);
    expect(bestimmt.length).toBe(8);
  });
});
