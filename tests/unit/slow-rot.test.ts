/**
 * `Slow Rot` — Zersetzungs-Stapel verfallen nicht, solange ein DoT läuft
 * (13.5, Issue #480).
 *
 * Der dritte der sechs Rotations-Effekte. Er macht den Aufbau vor dem
 * Schadensfenster zur Kernentscheidung statt zum Wettlauf: Wer einen
 * Fäulnis-Zauber am Laufen hält, hält auch die Stapel.
 *
 * **Aufgeschoben, nicht aufgehoben** — die Verfallszeiten bleiben stehen. Endet
 * der letzte DoT, holt der nächste Durchlauf alles nach, was inzwischen fällig
 * wurde. Ein Zurücksetzen der Zeiten wäre eine Verlängerung und damit ein
 * zweiter Effekt; genau das prüft der letzte Fall.
 */

import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { activeRotationEffects, SETS, SET_COUNT } from '../../src/gear/sets.js';
import { SET_EFFECT_SLOW_ROT, SET_EFFECT_WHOLE_WORK } from '../../src/gear/sets.js';
import { spellIndex } from '../helpers/spell-index.js';

/** Der Satz mit `slow-rot` — die Kennung steht in `data/sets.json`. */
const SLOW_ROT_SET = SETS.find((s) => s.tiers.some((t) => t.effect === SET_EFFECT_SLOW_ROT))?.id;

function teile(set: number, n: number): Int32Array {
  const pieces = new Int32Array(SET_COUNT);
  pieces[set] = n;
  return pieces;
}

const BOSS = bossById('test-warden') as BossDef;

/**
 * Stapel aufbauen, dann einen DoT ohne Stapel-Nachschub laufen lassen.
 *
 * # Zwei Messwege, die ich verworfen habe — und warum
 *
 * **BLIGHT** legt Stapel, aber gemessen KEINEN DoT (`dotRemainingMs` bleibt 0).
 * Die Bedingung des Effekts war damit nie erfüllt; beide Läufe zeigten 0, was
 * wie „der Effekt greift nicht" aussah.
 *
 * **ROT** legt beides — einen DoT und je Tick einen Stapel. Damit füllt er
 * laufend nach, was verfällt, und beide Läufe zeigten 5. Der Aufbau vermengte
 * Nachschub und Verfall, also genau die zwei Wirkungen, die getrennt gehören.
 *
 * Hier baut ROT die Stapel auf und hört dann auf; SMOULDER hält den DoT am
 * Laufen, ohne selbst Stapel zu legen. Nur so misst der Fall den Verfall.
 */
function stapelNach(effekte: readonly string[], bisMs: number): number {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  combat.setSetEffects(effekte);
  combat.beginFight(0);
  const rot = alsSpellIndex(spellIndex('rot'));
  const smoulder = alsSpellIndex(spellIndex('smoulder'));

  // Aufbau: ein ROT legt über seine vier Sekunden mehrere Stapel.
  combat.cast(rot, 0);
  for (let t = 0; t <= 4000; t += 100) combat.stepPassive(t);

  // Danach nur noch SMOULDER — ein DoT, der keine Stapel legt. Alle 10 s
  // erneuert, damit er durchgehend läuft (Dauer 12 s).
  for (let t = 4000; t <= bisMs; t += 100) {
    if ((t - 4000) % 10_000 === 0) combat.cast(smoulder, t);
    combat.stepPassive(t);
  }
  return combat.decayStacks;
}

describe('AC1 — sechs Satzteile lösen ihn aus, fünf nicht', () => {
  it('die Schwelle greift, nicht die blosse Kennung', () => {
    expect(SLOW_ROT_SET).toBeDefined();
    const id = SLOW_ROT_SET as number;
    expect(activeRotationEffects(SETS, teile(id, 6))).toContain(SET_EFFECT_SLOW_ROT);
    expect(activeRotationEffects(SETS, teile(id, 5))).not.toContain(SET_EFFECT_SLOW_ROT);
  });
});

describe('AC2 — die Stapel bleiben stehen, solange ein DoT läuft', () => {
  it('mit dem Effekt überdauern sie ihre eigene Verfallszeit', () => {
    // Die Stapel haben eine kürzere Haltbarkeit als BLIGHT läuft. Ohne den
    // Effekt sind sie weg, mit ihm stehen sie.
    const mit = stapelNach([SET_EFFECT_SLOW_ROT], 20_000);
    const ohne = stapelNach([], 20_000);

    expect(mit).toBeGreaterThan(ohne);
    // ZUSICHERUNG: Ohne Effekt sind sie wirklich verfallen — sonst vergliche
    // der Fall zwei gleiche Zahlen und wäre aus dem falschen Grund grün.
    expect(ohne).toBe(0);
    expect(mit).toBeGreaterThan(0);
  });

  it('und ein fremder Effekt hält sie NICHT', () => {
    // Der Fall, der bei Ashen Regimen den Unterschied gemacht hat: Ohne ihn
    // wäre auch eine Rechnung grün, die auf jede beliebige Kennung reagiert.
    expect(stapelNach([SET_EFFECT_WHOLE_WORK], 20_000)).toBe(0);
    expect(stapelNach(['gibt-es-nicht'], 20_000)).toBe(0);
  });
});

describe('AC3 — aufgeschoben, nicht aufgehoben', () => {
  it('nach dem Ende des DoT verfallen sie doch', () => {
    // **Der Fall, der den Effekt von einer Verlängerung trennt.** Würden die
    // Verfallszeiten zurückgesetzt statt nur übersprungen, hielten die Stapel
    // auch nach dem letzten DoT weiter — aus „nicht verfallen, solange" würde
    // „nie verfallen".
    //
    // BLIGHT endet; danach ist die Bedingung weg, und der nächste Durchlauf
    // holt alles Fällige nach.
    const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
    combat.setSetEffects([SET_EFFECT_SLOW_ROT]);
    combat.beginFight(0);
    const rot = alsSpellIndex(spellIndex('rot'));
    const smoulder = alsSpellIndex(spellIndex('smoulder'));

    combat.cast(rot, 0);
    for (let t = 0; t <= 4000; t += 100) combat.stepPassive(t);
    // Ein SMOULDER hält den DoT zwölf Sekunden, also bis 16 000. Abgelesen wird
    // bei 15 000 — SOLANGE er läuft. Ohne den Effekt wären die Stapel hier
    // schon weg (gemessen: sie fallen bei 16 000, ihre Haltbarkeit endet
    // früher).
    combat.cast(smoulder, 4000);
    for (let t = 4000; t <= 15_000; t += 100) combat.stepPassive(t);
    const waehrendDot = combat.decayStacks;
    expect(combat.dotRemainingMs(smoulder, 15_000)).toBeGreaterThan(0);

    // Danach wird NICHT erneuert: Der DoT läuft aus, die Bedingung fällt weg.
    for (let t = 15_000; t <= 40_000; t += 100) combat.stepPassive(t);
    const danach = combat.decayStacks;
    expect(combat.dotRemainingMs(smoulder, 40_000)).toBe(0);

    expect(waehrendDot).toBeGreaterThan(0);
    expect(danach).toBe(0);
  });
});

describe('AC4 — Kreuzprobe: was der Effekt nicht anfasst', () => {
  it('die Zahl der Stapel beim Aufbau bleibt gleich', () => {
    // Der Effekt hält, er legt nicht. Legte er mit, wäre er doppelt so stark
    // wie beschrieben.
    const gleichNachDemWurf = (effekte: readonly string[]): number => {
      const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
      combat.setSetEffects(effekte);
      combat.beginFight(0);
      combat.cast(alsSpellIndex(spellIndex('blight')), 0);
      return combat.decayStacks;
    };
    expect(gleichNachDemWurf([SET_EFFECT_SLOW_ROT])).toBe(gleichNachDemWurf([]));
    expect(gleichNachDemWurf([])).toBeGreaterThan(0);
  });

  it('der Schaden eines Wurfs bleibt gleich', () => {
    const schaden = (effekte: readonly string[]): number => {
      const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
      combat.setSetEffects(effekte);
      combat.beginFight(0);
      combat.cast(alsSpellIndex(spellIndex('blight')), 0);
      return combat.damageDealt;
    };
    expect(schaden([SET_EFFECT_SLOW_ROT])).toBe(schaden([]));
  });

  it('die globale Wartezeit bleibt unberührt', () => {
    // Die Kreuzprobe gegen den Effekt, der als einziger dort eingreift.
    const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
    combat.setSetEffects([SET_EFFECT_SLOW_ROT]);
    expect(combat.gcd.lengthMs).toBe(1500);
  });
});
