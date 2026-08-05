/**
 * Das umgekehrte Ziel — die Wertung (GAME-DESIGN 11.2, Baustein 6; Issue #247).
 *
 * **Etappen 1 und 2 von dreien.** Die Wertung steht (wie der Lauf ausgeht) und
 * der Ring sinkt von selbst. Was fehlt, ist Etappe 3: Schutz und Heilung
 * drehen ihn zurück — und damit der einzige Weg, einen solchen Kampf zu
 * gewinnen.
 *
 * **Deshalb ist der Boss zwischen den Etappen unbesiegbar**, und das ist kein
 * Versehen: 60 je Sekunde über 20 s sind 1200 gegen 1000 Lebenspunkte. Der
 * Prüffall zu AC2 sagt das ausdrücklich, statt einen Sieg zu behaupten, den es
 * gerade nicht gibt.
 *
 * WARUM ZWEI BEDINGUNGEN UND NICHT EINE
 * -------------------------------------
 * | | Ring auf 0 | Zeit abgelaufen |
 * | --- | --- | --- |
 * | gewöhnlich | Sieg | Niederlage |
 * | `runaway` | Niederlage | **Sieg** |
 *
 * Kehrte sich nur die erste um, wäre es kein umgekehrtes ZIEL, sondern ein
 * zusätzlicher Verlustweg: Der Spieler hätte weiter nichts zu gewinnen.
 *
 * WAS SIE NICHT BELEGEN: dass ein Kampf mit diesem Feld überhaupt zu gewinnen
 * ist. Das kann erst Etappe 3, und bis dahin steht es hier als offene Aussage
 * statt als grüner Haken.
 */
import { describe, expect, it } from 'vitest';
import { Combat, FIGHT_OVERPRESSURE, FIGHT_RUNNING, FIGHT_WON } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const RUNAWAY = bossById('test-runaway');
if (RUNAWAY === undefined) throw new Error('data/bosses/test-runaway.json fehlt.');

/**
 * Bis `bisMs` laufen lassen, dabei `spellId` wirken, sooft es geht.
 *
 * `checkDefeat` gehört dazu: Ohne den Aufruf bliebe der Zustand auf
 * `FIGHT_RUNNING`, und jeder Fall unten wäre grün, ohne etwas zu messen.
 */
function laufen(combat: Combat, bisMs: number, spellId?: string): void {
  const index = spellId === undefined ? -1 : combat.indexOf(spellId);
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.checkDefeat(t)) return;
    if (index >= 0 && combat.gcd.isReady(t) && combat.isReady(index, t)) combat.cast(index, t);
  }
}

function neuerKampf(boss = RUNAWAY): Combat {
  return new Combat({ school: CALCINATION.value, boss: boss as never });
}

/**
 * Derselbe Boss, nur ohne das Feld.
 *
 * **Nicht der Erste Schmelztiegel als Gegenfall.** Mein erster Entwurf nahm
 * ihn, und beide Gegenfälle scheiterten: Er hat 4240 Lebenspunkte und 60 s
 * Frist — KINDLE allein schafft in dieser Zeit nur rund 4000 Schaden, der Lauf
 * endete also weder als Sieg noch überhaupt. Ein Gegenfall, der aus einem
 * ZWEITEN Grund anders ausgeht, belegt nichts über den ersten.
 *
 * Hier unterscheidet genau ein Feld die beiden Läufe, sonst nichts.
 */
const OHNE_FELD = { ...RUNAWAY, runaway: null };

/**
 * Derselbe Boss mit einer Reaktion, die kaum davonläuft.
 *
 * Sie senkt den Ring um 1 je Sekunde — in der 20-s-Frist also um 20 von 1000.
 * Damit lässt sich der SIEG bei laufendem Tick belegen, ohne auf Etappe 3 zu
 * warten. Zugleich zeigt der Vergleich mit dem echten Datensatz, woran es
 * hängt: an der Rate, nicht an der Mechanik.
 */
const LANGSAM = { ...RUNAWAY, runaway: { perStepMs: 1000, amount: 1 } };

/**
 * Derselbe Boss mit einem Unverwundbarkeits-Fenster (#218) obendrauf.
 *
 * Kein Boss trägt heute beides. Der Prüffall gibt es ihm trotzdem, weil der
 * Code entscheiden MUSS, was im Fenster mit der Reaktion geschieht — und eine
 * Antwort, die niemand geprüft hat, ist geraten.
 */
const MIT_FENSTER = { ...RUNAWAY, race: { atMs: 3000, windowMs: 6000, threshold: 999999 } };

describe('AC1 — der Ring auf null ist die NIEDERLAGE', () => {
  it('endet an der ausgerechneten Stelle, und die Zahl stand vor der Messung fest', () => {
    // **Zwei Quellen senken den Ring, und beide gehören in die Rechnung:**
    // der Tick mit 60 je Sekunde und KINDLE mit 100 je GCD (1500 ms). Bei
    // 7500 ms stehen 420 + 600 = 1020 gegen 1000 Lebenspunkte.
    //
    // In Etappe 1 stand hier 13500 — nur der Spielerschaden. Der Prüffall war
    // richtig und ist es geblieben; was sich geändert hat, ist die Mechanik
    // unter ihm. Deshalb wird die Zahl hier gerechnet und nicht abgeschrieben.
    let spieler = 0;
    let tick = 0;
    let leerBeiMs = -1;
    for (let t = 0; t < 20000; t += TICK_MS) {
      spieler = (Math.floor(t / 1500) + 1) * 100;
      tick = Math.floor(t / RUNAWAY.runaway!.perStepMs) * RUNAWAY.runaway!.amount;
      if (spieler + tick >= RUNAWAY.hp) {
        leerBeiMs = t;
        break;
      }
    }
    expect(leerBeiMs).toBe(7500);
    // Einen Takt später wird es bemerkt: `checkDefeat` steht vor dem Wirken.
    const erwartetMs = leerBeiMs + TICK_MS;

    const combat = neuerKampf();
    laufen(combat, 20000, 'kindle');

    expect(combat.damageDealt).toBeGreaterThanOrEqual(RUNAWAY.hp);
    expect(combat.fightState, 'Ring auf 0 darf hier NICHT der Sieg sein').toBe(FIGHT_OVERPRESSURE);
    expect(combat.endedAt).toBe(erwartetMs);
  });

  it('SCHADEN MACHT ES SCHLIMMER — das ist der ganze Baustein', () => {
    // **Der Fall, um den es in 11.2 geht:** „Statt Schaden zu machen." Zwei
    // Läufe gegen denselben Boss, der Unterschied ist genau eine Sache — im
    // zweiten drückt der Spieler Tasten. Er verliert dadurch früher.
    const still = neuerKampf();
    const fleissig = neuerKampf();
    laufen(still, 25000);
    laufen(fleissig, 25000, 'kindle');

    expect(still.fightState).toBe(FIGHT_OVERPRESSURE);
    expect(fleissig.fightState).toBe(FIGHT_OVERPRESSURE);
    expect(fleissig.endedAt, 'wer Schaden macht, verliert früher').toBeLessThan(still.endedAt);
    expect(still.endedAt).toBe(17000);
    expect(fleissig.endedAt).toBe(7510);
  });

  it('derselbe Lauf gegen einen gewöhnlichen Boss ist der SIEG', () => {
    // **Der Gegenfall, ohne den der erste nichts belegt.** Wenn dieselbe
    // Handlung bei einem Boss ohne das Feld gewinnt und mit ihm verliert, dann
    // liegt es am Feld und nicht an der Rotation.
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 20000, 'kindle');
    expect(combat.fightState).toBe(FIGHT_WON);
    expect(combat.endedAt, 'ohne Tick braucht der Spieler länger').toBe(13500 + TICK_MS);
  });
});

describe('AC2 — die abgelaufene Zeit ist der SIEG', () => {
  it('gewinnt, wenn der Ring die Frist übersteht', () => {
    // **Mit laufendem Tick, aber kleiner Rate.** Ein Lauf gegen den echten
    // Datensatz kann das gerade nicht zeigen: Dort ist der Ring vor der Frist
    // leer, und ohne Etappe 3 hat der Spieler nichts dagegen. Die Wertung
    // hängt aber nicht an der Rate — genau das belegt dieser Fall.
    const combat = neuerKampf(LANGSAM);
    laufen(combat, 25000);

    expect(combat.damageDealt, 'der Tick läuft, er reicht nur nicht').toBe(20);
    expect(combat.fightState, 'die Frist zu überstehen ist hier der Sieg').toBe(FIGHT_WON);
    expect(combat.endedAt).toBe(RUNAWAY.overpressureMs);
  });

  it('derselbe Lauf gegen einen gewöhnlichen Boss ist die NIEDERLAGE', () => {
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 25000);
    expect(combat.fightState).toBe(FIGHT_OVERPRESSURE);
    expect(combat.endedAt).toBe(RUNAWAY.overpressureMs);
  });
});

describe('AC3 — der Ring sinkt von selbst, ohne dass jemand etwas tut', () => {
  it('kippt an der gerechneten Stelle, und die Zahl stand vorher fest', () => {
    // 60 je Sekunde gegen 1000: Der siebzehnte Schritt bringt 1020 und damit
    // den Ring auf null. **Gerechnet, bevor gemessen wurde.**
    const def = RUNAWAY.runaway!;
    const schritte = Math.ceil(RUNAWAY.hp / def.amount);
    const erwartetMs = schritte * def.perStepMs;
    expect(erwartetMs).toBe(17000);

    const combat = neuerKampf();
    laufen(combat, 25000);

    expect(combat.fightState, 'ohne Eingriff ist es die Niederlage').toBe(FIGHT_OVERPRESSURE);
    expect(combat.endedAt).toBe(erwartetMs);
    expect(combat.damageDealt).toBe(schritte * def.amount);
  });

  it('läuft in gleichmäßigen Schritten, nicht in einem Sprung', () => {
    // Ein Tick, der den Ring auf einmal leerte, erfüllte den Fall oben
    // ebenfalls. Gemessen wird deshalb der Verlauf.
    //
    // **`t < 5000`, nicht `t <= 5000`** — dieselbe Grenze wie die Wiedergabe
    // (`for tick < ticks`) und wie der Prüffall im Simulator. Mit `<=` maß
    // dieser Fall einen Schritt mehr als sein Gegenstück in Rust, und der
    // Unterschied sah einen Moment lang wie eine Abweichung zwischen den
    // Seiten aus.
    const combat = neuerKampf();
    const stand: number[] = [];
    combat.beginFight(0);
    for (let t = 0; t < 5000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.checkDefeat(t)) break;
      if (t % 1000 === 0) stand.push(combat.damageDealt);
    }
    expect(stand).toEqual([0, 60, 120, 180, 240]);
    expect(combat.damageDealt, 'dieselbe Zahl wie in sim/tests/parity_runaway.rs').toBe(240);
  });

  it('ein Boss OHNE das Feld bewegt sich nicht von selbst', () => {
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 10000);
    expect(combat.damageDealt).toBe(0);
  });
});

describe('AC3b — das Unverwundbarkeits-Fenster hält die Reaktion NICHT auf', () => {
  it('läuft im Fenster weiter und zählt dort nicht als verlorener Schaden', () => {
    // **Die Entscheidung, die der Code treffen muss.** Das Fenster (#218)
    // schützt den Boss vor SCHADEN; eine Reaktion, die von selbst kippt, ist
    // keiner. Liefe sie in den Fenster-Zweig, meldete die Fehleranalyse dem
    // Spieler außerdem Schaden, den er nie gemacht hat (15.3).
    const combat = neuerKampf(MIT_FENSTER);
    combat.beginFight(0);
    for (let t = 0; t <= 8000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.checkDefeat(t)) break;
    }
    // Das Fenster steht von 3 s bis 9 s offen — bei 8 s ist es also offen.
    expect(combat.raceOpen, 'sonst prüft der Fall das Fenster gar nicht').toBe(true);
    expect(combat.damageDealt, 'acht Schritte à 60 sind durchgelaufen').toBe(480);
    expect(combat.damageLostToRace, 'sie ist kein Schaden des Spielers').toBe(0);
  });
});

describe('Der Wächter: der KAMPF wertet um, nicht die Daten', () => {
  it('kennt vor dem Ende keinen der beiden Ausgänge', () => {
    // Ohne diese Zusicherung wäre nicht belegt, dass die Wertung überhaupt an
    // einer Bedingung hängt — ein Kampf, der sofort endet, erfüllte die Fälle
    // oben ebenfalls.
    const combat = neuerKampf();
    combat.beginFight(0);
    for (let t = 0; t <= 5000; t += TICK_MS) {
      combat.stepPassive(t);
      combat.checkDefeat(t);
    }
    expect(combat.fightState).toBe(FIGHT_RUNNING);
  });

  it('unterscheidet die beiden Bosse an EINEM Feld, nicht an ihrer Kennung', () => {
    // Beide Läufe benutzen denselben Datensatz; genau ein Feld unterscheidet
    // sie. Wer die Umkehrung an der Boss-Kennung festmachte, käme hier durch —
    // deshalb steht das Feld selbst in der Zusicherung.
    expect(RUNAWAY.runaway).not.toBeNull();
    expect(OHNE_FELD.runaway).toBeNull();
    expect(RUNAWAY.runaway?.perStepMs).toBe(1000);
    expect(RUNAWAY.runaway?.amount).toBe(60);
  });
});

describe('AC7-Vorgriff — ein Boss ohne das Feld bleibt unberührt', () => {
  it('trägt es nicht, bewegt sich nicht von selbst und wertet wie bisher', () => {
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 20000, 'kindle');
    expect(combat.damageDealt).toBe(RUNAWAY.hp);
    expect(combat.fightState).toBe(FIGHT_WON);
  });
});
