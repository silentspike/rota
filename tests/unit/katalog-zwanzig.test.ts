/**
 * Die zwanzig Gegenstände aus `docs/AUSRUESTUNGS-RASTER.md` (Issue #421b).
 *
 * # Was hier geprüft wird und was nicht
 *
 * Geprüft wird, dass die zwanzig **tun, was das Raster von ihnen behauptet**:
 * jeder Platz belegt, zwei Ausrichtungen je Platz, und — das ist der Kern —
 * dass die beiden Ausrichtungen im Kampf wirklich auseinandergehen.
 *
 * **Nicht** geprüft wird, ob sie ausgewogen sind. Siehe den Warnkasten bei
 * `SCHADEN_MIX` unten; das misst der Balance-Läufer.
 *
 * # Warum die Erwartungen abgeschrieben sind
 *
 * `ERWARTETE_PLAETZE` und die beiden Affix-Listen leiten sich **nicht** aus
 * `data/items.json` her. Täten sie das, wäre jeder Fall grün, egal was
 * drinsteht — er prüfte seine eigene Erwartung gegen sich selbst. Dieselbe
 * Begründung steht in `tests/unit/talentbaeume.test.ts` und in
 * `src/progression/unlocks.ts`.
 */
import { describe, expect, it } from 'vitest';

import { ITEMS, itemById, wornFrom } from '../../src/gear/catalog.js';
import { SLOT_COUNT, SLOT_NAMES } from '../../src/gear/slots.js';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, bossById, FIRST_CRUCIBLE } from '../../src/core/data.js';

/** Die zehn Plätze aus GAME-DESIGN 13.2, in der Reihenfolge des Designs. */
const ERWARTETE_PLAETZE = [
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
] as const;

/** Die Mischung aus 13.1b — drei Sonderwerte. */
const MIX = ['catalysis', 'purity', 'yield'];
/** Ein Schadensfaktor, zwei Sonderwerte. */
const DRIVE = ['essence', 'purity', 'catalysis'];

/** Die zwanzig ohne den Prüf-Gegenstand. */
const ZWANZIG = ITEMS.filter((item) => !item.id.startsWith('test-'));

describe('Die zwanzig decken alle zehn Plätze', () => {
  it('führt genau zwanzig Einträge neben dem Prüf-Gegenstand', () => {
    expect(ZWANZIG).toHaveLength(20);
  });

  it('belegt jeden der zehn Plätze mit genau zwei', () => {
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const drauf = ZWANZIG.filter((item) => item.slot === slot);
      expect(drauf.map((i) => i.id).sort(), `Platz ${slot} (${SLOT_NAMES[slot]})`).toEqual([
        `${ERWARTETE_PLAETZE[slot]}-drive`,
        `${ERWARTETE_PLAETZE[slot]}-mix`,
      ]);
    }
  });

  it('trägt englische Namen — Spieltexte sind englisch (Design-Gesetz 5)', () => {
    for (const item of ZWANZIG) {
      expect(item.name, `${item.id} hat keinen Namen`).not.toBe('');
      expect(item.name, `${item.id} heißt wie seine Kennung`).not.toBe(item.id);
      // Echte Umlaute gehören in Doku und Kommentare, nicht in Spieltexte.
      expect(item.name, `${item.id} trägt einen Umlaut im Spieltext`).toMatch(/^[A-Za-z ]+$/);
    }
  });
});

describe('Die zwei Ausrichtungen — als Liste festgenagelt, nicht als Zahl', () => {
  it('`-mix` trägt überall dieselben drei Sonderwerte', () => {
    for (const item of ZWANZIG.filter((i) => i.id.endsWith('-mix'))) {
      expect(item.affixKinds, item.id).toEqual(MIX);
    }
  });

  it('`-drive` tauscht YIELD gegen den Schadensfaktor ESSENCE', () => {
    for (const item of ZWANZIG.filter((i) => i.id.endsWith('-drive'))) {
      expect(item.affixKinds, item.id).toEqual(DRIVE);
    }
  });

  it('KEIN Eintrag nennt `gearEffects` — entschieden am 2026-08-10', () => {
    // **Es gibt keine Sperre im Code dafür.** `gearEffects` ist eine gültige
    // Affix-Art, weil sie ein Feld füllt; die Entscheidung, sie nicht zu
    // benutzen, ist eine Design-Entscheidung und lebt nur hier.
    //
    // Der Grund steht in 13.1b: „Was multiplikativ ist, wird gejagt — und
    // alles andere wird liegen gelassen." Ein multiplikativer Faktor über
    // gewöhnliche Beute entwertet jeden anderen Affix.
    for (const item of ITEMS) {
      expect(item.affixKinds, `${item.id} bricht die Entscheidung von 13.1b`).not.toContain(
        'gearEffects',
      );
    }
  });

  it('alle zwanzig auf Stufe 40 und Qualität 2 — kein Platz ist heimlich stärker', () => {
    // Verschiedene Werte hier wären eine versteckte Balance-Aussage. Beide
    // werden beim Fall ohnehin neu gewürfelt (`itemLevelOf`, `qualityOfRoll`).
    for (const item of ZWANZIG) {
      expect(item.itemLevel, item.id).toBe(40);
      // Qualität 2 ist die niedrigste mit drei Affix-Plätzen; darunter wären
      // drei `affixKinds` mehr Arten als Plätze, und der Katalog lüde nicht.
      expect(item.quality, item.id).toBe(2);
    }
  });
});

/**
 * Was ein fester Lauf mit diesem Gegenstand an Schaden ergibt.
 *
 * Zwanzigmal KINDLE im Abstand einer Wartezeit, Startwert 0, gegen
 * `test-crucible-trial` (teilt nicht aus, 999999 Leben — nichts stört die
 * gemessene Größe).
 */
function schadenMit(itemId: string): number {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({
    school: CALCINATION.value,
    boss,
    seed: 0,
    worn: wornFrom([{ itemId, upgradeSteps: 0, equippedSlot: itemById(itemId)?.slot ?? 0 }]),
  });
  combat.beginFight(1);
  let zeit = 10;
  for (let i = 0; i < 20; i++) {
    combat.stepPassive(zeit);
    combat.cast(alsSpellIndex(combat.indexOf('kindle')), zeit);
    zeit += 1500;
  }
  combat.stepPassive(zeit);
  return combat.damageDealt;
}

/**
 * Die beiden Ausrichtungen an den **gebauten** Einträgen, nicht an Entwürfen.
 *
 * > ### ⚠ Diese Zahlen sagen NICHTS über Balance
 * >
 * > Der Lauf wirkt KINDLE im **festen** Takt von 1500 ms, und KINDLEs
 * > Abklingzeit ist genau 1500 ms. CATALYSIS verkürzt Abklingzeiten; wer
 * > trotzdem 1500 ms wartet, verschenkt jede Verkürzung. **Der Aufbau
 * > benachteiligt damit strukturell `-mix`**, die tempolastige Ausrichtung —
 * > ausgerechnet die mit der niedrigeren Zahl.
 * >
 * > Dieselbe Falle steht seit #547 in `buff-groessen-540.test.ts`: DISTIL
 * > wurde an KINDLE gemessen, dessen Abklingzeit schon am Boden war.
 * >
 * > Geprüft wird hier deshalb **nur der Unterschied**, nicht seine Richtung
 * > als Wertung. Was die Ausrichtungen taugen, misst der Balance-Läufer.
 */
const SCHADEN_MIX = 2812;
const SCHADEN_DRIVE = 3458;

describe('Die Ausrichtungen gehen im Kampf wirklich auseinander', () => {
  it('trifft die Zahlen aus dem Raster — an den gebauten Einträgen gemessen', () => {
    expect(schadenMit('focus-mix')).toBe(SCHADEN_MIX);
    expect(schadenMit('focus-drive')).toBe(SCHADEN_DRIVE);
  });

  it('und der Unterschied ist die AUSWAHL, nicht der Platz', () => {
    // Zehn Plätze, dieselbe Ausrichtung, dasselbe Ergebnis: Der Platz ist ein
    // Behälter (13.2), er trägt keine eigene Stärke. Liefe ein Platz aus der
    // Reihe, wäre die platz-spezifische Zuordnung zurück, die am 2026-07-29
    // ausdrücklich gestrichen wurde.
    for (const platz of ERWARTETE_PLAETZE) {
      expect(schadenMit(`${platz}-mix`), `${platz}-mix`).toBe(SCHADEN_MIX);
      expect(schadenMit(`${platz}-drive`), `${platz}-drive`).toBe(SCHADEN_DRIVE);
    }
  });

  it('GEGENPROBE: ohne Ausrüstung ist es weniger als mit', () => {
    // Ohne diese Zeile wären die Zahlen oben zwei Konstanten ohne Bezug — sie
    // belegten, dass sich die Ausrichtungen unterscheiden, aber nicht, dass
    // die Ausrüstung überhaupt etwas tut.
    const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
    const nackt = new Combat({ school: CALCINATION.value, boss, seed: 0 });
    nackt.beginFight(1);
    let zeit = 10;
    for (let i = 0; i < 20; i++) {
      nackt.stepPassive(zeit);
      nackt.cast(alsSpellIndex(nackt.indexOf('kindle')), zeit);
      zeit += 1500;
    }
    nackt.stepPassive(zeit);
    expect(nackt.damageDealt).toBeLessThan(SCHADEN_MIX);
  });
});
