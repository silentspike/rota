/**
 * Das umgekehrte Ziel — die Wertung (GAME-DESIGN 11.2, Baustein 6; Issue #247).
 *
 * **Etappe 1 von dreien.** Hier steht nur, wie der Lauf AUSGEHT; dass der Ring
 * von selbst sinkt, kommt in Etappe 2, und Schutz und Heilung drehen ihn in
 * Etappe 3 zurück. Der Schnitt ist so gewählt, dass jede Etappe für sich
 * prüfbar ist: Die Wertung lässt sich mit gewöhnlichen Schadenszaubern
 * belegen — der Spieler senkt den Ring dann selbst.
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
 * WAS DIESE PRÜFFÄLLE NICHT BELEGEN und was sie deshalb ausdrücklich sagen:
 * Sie messen die Wertung, nicht die Mechanik. Ein Boss, der den Ring nicht von
 * selbst senkt, ist mit ihnen vereinbar — genau deshalb steht Etappe 2 noch
 * aus.
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

describe('AC1 — der Ring auf null ist die NIEDERLAGE', () => {
  it('endet an der ausgerechneten Stelle, und die Zahl stand vor der Messung fest', () => {
    // KINDLE macht 100 je GCD (1500 ms), der Ring hat 1000: Die zehnte
    // Wirkung fällt bei 13500 ms. **Gerechnet, bevor gemessen wurde** — sonst
    // beschreibt der Prüffall nur, was ohnehin herauskam.
    const proWirkung = 100;
    const letzteWirkungMs = (RUNAWAY.hp / proWirkung - 1) * 1500;
    expect(letzteWirkungMs).toBe(13500);
    // **Einen Takt später wird es bemerkt**, nicht im selben. `checkDefeat`
    // steht VOR dem Wirken — der Treffer, der den Ring leert, wird also erst in
    // der Prüfung des nächsten Takts gesehen. Gemessen 13510 gegen gerechnete
    // 13500; die Rechnung war richtig, meine Erwartung an die Reihenfolge nicht.
    // Der Simulator hält dafür eigens einen Weckruf vor (`fight.rs`).
    const erwartetMs = letzteWirkungMs + TICK_MS;

    const combat = neuerKampf();
    laufen(combat, 20000, 'kindle');

    expect(combat.damageDealt).toBeGreaterThanOrEqual(RUNAWAY.hp);
    expect(combat.fightState, 'Ring auf 0 darf hier NICHT der Sieg sein').toBe(FIGHT_OVERPRESSURE);
    expect(combat.endedAt).toBe(erwartetMs);
  });

  it('derselbe Lauf gegen einen gewöhnlichen Boss ist der SIEG', () => {
    // **Der Gegenfall, ohne den der erste nichts belegt.** Wenn dieselbe
    // Handlung bei einem Boss ohne das Feld gewinnt und mit ihm verliert, dann
    // liegt es am Feld und nicht an der Rotation.
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 20000, 'kindle');
    expect(combat.fightState).toBe(FIGHT_WON);
    expect(combat.endedAt, 'zur selben Zeit — nur die Wertung dreht sich').toBe(13500 + TICK_MS);
  });
});

describe('AC2 — die abgelaufene Zeit ist der SIEG', () => {
  it('gewinnt, wenn der Ring die Frist übersteht', () => {
    const combat = neuerKampf();
    laufen(combat, 25000);

    expect(combat.damageDealt, 'ohne Zauber sinkt der Ring in Etappe 1 nicht').toBe(0);
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
  it('trägt es nicht und wertet wie bisher', () => {
    const combat = neuerKampf(OHNE_FELD);
    laufen(combat, 20000, 'kindle');
    expect(combat.damageDealt).toBeGreaterThan(0);
    expect(combat.fightState).toBe(FIGHT_WON);
  });
});
