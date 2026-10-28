/**
 * Der Schutz-Weg der PUTREFACTION wirkt auf den Schutz je Stapel — und auf
 * NICHTS sonst (Issue #419).
 *
 * # Der Befund
 *
 * Commit 347395fd hat in `src/core/combat.ts` an 27 Stellen eine `1` durch
 * `talentDecayWardFactor(this.talents)` ersetzt. Gewollt war genau eine: der
 * Schutz je Zersetzungs-Stapel in `hitPlayer`. Unter den übrigen standen neben
 * 22 Kommentarzeilen vier Klemmen auf 1:
 *
 * ```text
 *   setPlayerLevel      Math.max(1, Math.trunc(level))
 *   FIXATION-Preis      Math.max(1, Math.round(control.cost / Faktor))
 *   Heilung über Zeit   Math.max(1, Math.round(heal.overMs / abstand))
 *   WARD-Minderung      Math.min(1, Math.min(Deckel, roh))
 * ```
 *
 * Ohne den Schutz-Weg ist der Faktor genau 1,0 — jede Prüfung ohne ihn sah
 * dieselben Zahlen wie vorher, und keine Prüfung trug ihn. Ab einem Rang ist er
 * 1,12, bei fünf Rängen 1,6.
 *
 * # Was jeder Fall prüft
 *
 * Dieselbe Rechnung mit und ohne den Schutz-Weg. Gleich bleiben muss sie, weil
 * keine der drei Stellen mit einem Zersetzungs-Stapel zu tun hat.
 *
 * # Warum die vierte Stelle keinen Fall hat
 *
 * Die WARD-Minderung klemmt unter dem Deckel aus `data/player/base.json`
 * (750 Promille). Der liegt unter 1, und der Schutz-Faktor ist nie kleiner als
 * 1 — `Math.min` mit ihm liefert deshalb immer denselben Wert wie mit 1. Ein
 * Fall dafür könnte nicht rot werden, und ein Fall, der nicht scheitern kann,
 * belegt nichts.
 *
 * Den Abgleich mit dem Simulator für die Stelle, die mit den heutigen Daten
 * erreichbar ist (LATCH unter fünf Rängen `fixation-cost`), führt
 * `sim/tests/parity_kontrolle_preis_talent.rs` gegen die Aufzeichnung
 * `kontrolle-preis-talent`.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type SchoolDef } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Ein bis fünf Ränge — der Faktor ist 1,12 bis 1,6. */
const SCHUTZ_RAENGE = [1, 2, 3, 4, 5] as const;

/** Der Schutz-Weg samt dem Stamm, den er verlangt (`requires`). */
function mitSchutzWeg(raenge: number, basis: Record<string, number> = {}): Record<string, number> {
  return { ...basis, 'putrefaction-trunk': 1, 'putrefaction-duration': raenge };
}

describe('der FIXATION-Preis', () => {
  /**
   * Fünf Ränge `fixation-cost` teilen LATCHs 2 Punkte durch 1,4 — gerundet 1,
   * also genau AUF der Untergrenze. Nur dort kann sie etwas ändern.
   */
  const KOSTEN_WEG = { 'fixation-trunk': 1, 'fixation-cost': 5 };

  /** BIND, dann zweimal LATCH — der Vorrat nach jedem der drei Drücke. */
  function runde(talents: Record<string, number>): {
    vorrat: number[];
    ausgegeben: number;
    gescheitert: number;
  } {
    const boss = bossById('test-warden');
    if (boss === undefined) throw new Error('data/bosses/test-warden.json fehlt.');
    const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
    combat.setTalents(talents);
    const latch = alsSpellIndex(combat.indexOf('latch'));
    const bind = alsSpellIndex(combat.indexOf('bind'));
    // Dieselbe feste Folge wie in der Aufzeichnung `kontrolle-preis-talent`:
    // LATCHs Abklingzeit ist 10 000 ms, der zweite Druck trifft also auf den
    // Rest des ersten. Warum der erste erst bei 4500 ms kommt, steht dort.
    const plan = new Map([
      [0, bind],
      [4500, latch],
      [16_500, latch],
    ]);
    const vorrat: number[] = [];
    for (let t = 0; t <= 17_000; t += TICK_MS) {
      combat.stepPassive(t);
      const zauber = plan.get(t);
      if (zauber === undefined) continue;
      expect(combat.cast(zauber, t).result, `der Druck bei ${String(t)} ms`).toBe(CAST_OK);
      vorrat.push(combat.control);
    }
    return { vorrat, ausgegeben: combat.controlSpent, gescheitert: combat.controlFailed };
  }

  it('die Runde liegt auf der Untergrenze: zwei LATCH aus zwei Punkten', () => {
    // **Die Zusicherung vor dem Vergleich.** Läge der Preis über 1, sähe ein
    // Vergleich mit und ohne Schutz-Weg gleich aus, ohne die Klemme je
    // berührt zu haben.
    expect(runde(KOSTEN_WEG)).toEqual({ vorrat: [2, 1, 0], ausgegeben: 2, gescheitert: 0 });
    // Und ohne den Kosten-Weg kostet LATCH 2 — der zweite scheitert. Erst
    // dieser Unterschied zeigt, dass die Runde den geteilten Preis überhaupt
    // sieht.
    expect(runde({ 'fixation-trunk': 1 }).gescheitert).toBe(1);
  });

  it.each(SCHUTZ_RAENGE)('kostet mit %i Rängen im Schutz-Weg denselben einen Punkt', (raenge) => {
    expect(runde(mitSchutzWeg(raenge, KOSTEN_WEG))).toEqual(runde(KOSTEN_WEG));
  });
});

describe('die Stufe', () => {
  function stufeNach(level: number, talents?: Record<string, number>): number {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    if (talents !== undefined) combat.setTalents(talents);
    combat.setPlayerLevel(level);
    return combat.level;
  }

  it('klemmt ohne Talente auf 1 und folgt sonst der Eingabe', () => {
    // Die Gegenprobe zu den Fällen darunter: Die Klemme gibt es, und der
    // Setzer wirkt — sonst bewiese „bleibt 1" nur, dass nichts geschieht.
    expect(stufeNach(0)).toBe(1);
    expect(stufeNach(7)).toBe(7);
  });

  it.each(SCHUTZ_RAENGE)('klemmt mit %i Rängen im Schutz-Weg weiter auf 1', (raenge) => {
    expect(stufeNach(0, mitSchutzWeg(raenge))).toBe(1);
    expect(stufeNach(1, mitSchutzWeg(raenge))).toBe(1);
  });
});

describe('die Heilung über Zeit', () => {
  /**
   * POULTICE mit EINEM Schritt statt sechs.
   *
   * **Ein gebauter Zauber, und das ist Absicht.** Ausgeliefert ist heute nur
   * POULTICE mit 6000 über 1000 ms, also sechs Schritten — die Untergrenze
   * von einem Schritt erreicht er nie. Erreichbar wird sie erst mit einem
   * Zauber, dessen Dauer gleich seinem Abstand ist; genau den baut diese
   * Funktion. Dass der Fall die Klemme wirklich berührt, belegt die
   * Zusicherung im ersten Fall unten: ein Schritt, danach nichts mehr.
   */
  function einSchrittSchule(): SchoolDef {
    const basis = CALCINATION.value;
    return {
      ...basis,
      spells: basis.spells.map((spell) =>
        spell.id === 'poultice' && spell.heal !== undefined
          ? { ...spell, heal: { ...spell.heal, overMs: spell.heal.tickIntervalMs ?? TICK_MS } }
          : spell,
      ),
    };
  }

  /** Was nach einem und nach zwei Abständen angekommen ist. */
  function geheilt(talents?: Record<string, number>): { nachEinem: number; nachZwei: number } {
    // **Ein Boss, der austeilt** — dieselbe Wahl wie in `poultice.test.ts`:
    // Ohne fehlendes Leben verpufft jeder Schritt, und ein Vergleich zweier
    // Nullen sähe aus wie Gleichheit.
    const boss = bossById('test-onslaught');
    if (boss === undefined) throw new Error('data/bosses/test-onslaught.json fehlt.');
    const combat = new Combat({ school: einSchrittSchule(), boss, seed: 0 });
    if (talents !== undefined) combat.setTalents(talents);
    combat.beginFight(0);
    // Zwei Treffer abwarten: Nach einem fehlen 1600, weniger als der Betrag.
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    expect(combat.damageTaken - combat.healed, 'zu wenig fehlt').toBeGreaterThan(1800);
    const poultice = alsSpellIndex(combat.indexOf('poultice'));
    expect(combat.cast(poultice, 12_200).result, 'POULTICE wurde abgewiesen').toBe(CAST_OK);
    for (let t = 12_200 + TICK_MS; t <= 13_200; t += TICK_MS) combat.stepPassive(t);
    const nachEinem = combat.healed;
    for (let t = 13_200 + TICK_MS; t <= 14_200; t += TICK_MS) combat.stepPassive(t);
    return { nachEinem, nachZwei: combat.healed };
  }

  it('der gebaute Zauber heilt in EINEM Schritt seinen ganzen Betrag', () => {
    expect(geheilt()).toEqual({ nachEinem: 1800, nachZwei: 1800 });
  });

  it.each(SCHUTZ_RAENGE)('und mit %i Rängen im Schutz-Weg genauso', (raenge) => {
    expect(geheilt(mitSchutzWeg(raenge))).toEqual(geheilt());
  });
});
