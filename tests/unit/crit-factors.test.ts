/**
 * Die Krit-Faktoren ziehen mit (Issue #196).
 *
 * DER FEHLER, DEN DAS VERHINDERT
 * Die Krit-Fassung wurde im Konstruktor angelegt und eingefroren -- moeglich,
 * "weil `worn` sich waehrend eines Kampfes nicht aendert". Das galt, solange
 * ALLE Faktoren bei Kampfbeginn feststanden. Sobald einer sich bewegt, rechnet
 * jeder kritische Treffer mit dem Stand von Sekunde null, und zwar STILL: Der
 * gewoehnliche Treffer waere richtig, der kritische falsch.
 *
 * WARUM DIE PRUEFFAELLE EINEN FAKTOR SELBST BEWEGEN
 * Ein Prueffall ueber `resistance` oder `worn` bliebe gruen -- beide stehen von
 * Kampfbeginn an fest. Es braucht einen, der sich WAEHREND des Laufs aendert,
 * und den gibt es in main noch nicht (er kommt mit #97). Deshalb setzt der
 * Prueffall ihn selbst, ueber genau den Weg, den eine Mechanik spaeter nimmt.
 *
 * KRITS SIND HIER DETERMINISTISCH: `critChance` gibt bei purity >= 1000 genau
 * 1 zurueck. Ohne das haenge der Nachweis am Zufallsstrom, und ein Prueffall,
 * der manchmal misst, misst nichts.
 */
import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { spellIndex } from '../helpers/spell-index.js';
import { COUNTDOWN_MS, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

const KINDLE = spellIndex('kindle');
/** Jeder Treffer kritet — critChance(worn) ist bei purity >= 1000 genau 1. */
const IMMER_KRIT = { ...NOTHING_WORN, purity: 1000 };

function neuerKampf(worn = IMMER_KRIT): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, worn });
}

/**
 * Einen einzelnen Zauber wirken und den Schaden dieses Treffers zurueckgeben.
 *
 * Ueber die Sitzung und nicht ueber Combat allein: So laeuft derselbe Weg wie
 * im Spiel, einschliesslich Countdown und Grundtakt.
 */
function einTreffer(combat: Combat, vorbereiten?: (c: Combat) => void): number {
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  session.bind(0, alsSpellIndex(KINDLE));
  session.beginCountdown(0);

  let tick = 0;
  let gedrueckt = false;
  const vorher = combat.damageDealt;
  for (let t = 0; t <= COUNTDOWN_MS + 3000; t += 10) {
    if (t === COUNTDOWN_MS && vorbereiten !== undefined) vorbereiten(combat);
    if (t >= COUNTDOWN_MS && !gedrueckt) {
      queue.push(0, tick, t, tick);
      gedrueckt = true;
    }
    session.tick(t);
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  return combat.damageDealt - vorher;
}

describe('AC2 — ein Faktor, der sich WAEHREND des Laufs bewegt', () => {
  it('wirkt sich auf den kritischen Treffer aus', () => {
    // Ohne Aenderung: der Grundwert.
    const grund = einTreffer(neuerKampf());
    expect(grund).toBeGreaterThan(0);

    // Mit einer Aenderung MITTEN im Kampf, ueber den Weg, den eine Mechanik
    // spaeter nimmt.
    const erhoeht = einTreffer(neuerKampf(), (c) => {
      c.setDamageFactor('buffs', 2);
    });

    expect(erhoeht).toBe(grund * 2);
  });

  it('faellt auf, WEIL der Treffer kritet — der Kern des Fehlers', () => {
    // Derselbe Vorgang ohne Krit. Er war schon vorher richtig; die Luecke lag
    // ausschliesslich im kritischen Pfad. Steht hier, damit sichtbar ist,
    // dass der Unterschied am KRIT hing und nicht an der Aenderung.
    const ohneKrit = einTreffer(neuerKampf(NOTHING_WORN), (c) => {
      c.setDamageFactor('buffs', 2);
    });
    const ohneKritOhneAenderung = einTreffer(neuerKampf(NOTHING_WORN));
    expect(ohneKrit).toBe(ohneKritOhneAenderung * 2);
  });

  it('bleibt gleich, wenn sich nichts aendert', () => {
    // Der Fall, in dem sich nichts aendern darf. Ohne ihn bestuende die
    // Pruefung auch fuer eine Rechnung, die den Schaden bei jedem Aufruf
    // verdoppelt.
    expect(einTreffer(neuerKampf())).toBe(einTreffer(neuerKampf()));
  });
});

describe('AC4 — ein vergessenes Nachziehen wird gefangen', () => {
  it('haelt die Krit-Fassung in allen Feldern ausser crit deckungsgleich', () => {
    const combat = neuerKampf();
    expect(combat.critFactorsMirrorBase).toBe(true);

    combat.setDamageFactor('buffs', 3);
    combat.setDamageFactor('talents', 1.5);
    combat.setDamageFactor('gearEffects', 0.8);
    expect(combat.critFactorsMirrorBase).toBe(true);
  });

  it('bleibt auch nach dem Zuruecksetzen deckungsgleich', () => {
    const combat = neuerKampf();
    combat.setDamageFactor('buffs', 3);
    combat.reset();
    expect(combat.critFactorsMirrorBase).toBe(true);
  });
});

describe('crit ist abgeleitet, nicht gesetzt', () => {
  it('steht in der Krit-Fassung und NICHT in der Basis', () => {
    // Waere crit ueber setDamageFactor erreichbar, koennte jemand die
    // Krit-Fassung ueber diesen Weg still von der Basis abkoppeln -- und die
    // Deckungsgleichheit oben wuerde es nicht merken, weil sie crit
    // ausnimmt.
    const combat = neuerKampf();
    const mitKrit = einTreffer(combat);
    const ohneKrit = einTreffer(neuerKampf(NOTHING_WORN));
    // 150 % Kritschaden ohne YIELD.
    expect(mitKrit).toBe(Math.round(ohneKrit * 1.5));
  });
});
