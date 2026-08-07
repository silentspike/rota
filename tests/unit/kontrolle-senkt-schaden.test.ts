/**
 * Die Fixierung greift messbar in den Kampf ein (Issue #576, AC3).
 *
 * # Warum dieser Fall neben der Aufzeichnung steht
 *
 * `kontrolle-wirkt.json` und ihr Rust-Gegenstück prüfen, dass **beide Seiten
 * dasselbe rechnen**. Sie prüfen nicht, dass die Zahlen etwas aussagen: Wären
 * alle drei Läufe bei 560, wäre die Parität genauso grün.
 *
 * Der Frische-Fall daneben prüft `toEqual(payload())` — auch er bliebe grün,
 * wenn die Wirkung verschwände, denn er vergleicht die Datei mit dem Spiel und
 * beide bewegten sich gemeinsam.
 *
 * Hier steht die Aussage selbst: **STALL senkt den erlittenen Schaden, STAY
 * nicht.**
 *
 * # Der Befund, aus dem der Fall entstand
 *
 * Zehn von zehn Fixations-Zaubern wurden in den Aufzeichnungen gewirkt und
 * bewegten **keine** der fünf Messgrößen. Das sah nach einer Mechanik aus, die
 * nicht ankommt — sie kommt an, es fehlte die Lage.
 *
 * `delayStrike` (STAY, LATCH) bewegt `damageTaken` bauartbedingt nie: Es
 * verschiebt den nächsten Zeitleisten-Eintrag und setzt danach zurück, die
 * folgenden holen auf. Der Treffer kommt später, aber er kommt. Was wirkt, ist
 * `slowCast` plus Unterbrechen — der Zauber trifft dann gar nicht.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const BOSS_ID = 'boss-1-4';
/** Der Zauber landet ohne STALL bei 32500 ms; ein Druck genau dort geht ins Leere. */
const QUENCH_BEI_MS = 32_500;
const TAKTE = 4000;

type Ergebnis = {
  damageTaken: number;
  unterbrochen: number;
  verpasst: number;
  bindWuerfe: number;
  quenchWuerfe: number;
};

/**
 * Ein Durchgang mit oder ohne Kontroll-Zauber.
 *
 * **QUENCH wird in jedem Lauf zur selben Kampfzeit gedrückt.** Der erste
 * Entwurf drückte ihn relativ zum laufenden Cast — dann wurde er ohne STALL
 * gar nicht geworfen, weil das Fenster schon zu war, und die Läufe
 * unterschieden sich in zwei Dingen statt in einem.
 */
function lauf(kontrollZauber: string | null): Ergebnis {
  const boss = bossById(BOSS_ID);
  if (boss === undefined) throw new Error(`Der Boss ${BOSS_ID} fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const quench = alsSpellIndex(combat.indexOf('quench'));
  const kontrolle =
    kontrollZauber === null ? alsSpellIndex(-1) : alsSpellIndex(combat.indexOf(kontrollZauber));

  let castBegann = -1;
  let kontrolleGeworfen = false;
  let quenchGeworfen = false;

  for (let tick = 0; tick < TAKTE; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) break;
    if (combat.casting !== -1 && castBegann < 0) castBegann = timeMs;
    if (combat.casting === -1 && castBegann >= 0) castBegann = -1;

    if (
      kontrolle >= 0 &&
      castBegann >= 0 &&
      !kontrolleGeworfen &&
      combat.control >= 4 &&
      combat.isReady(kontrolle, timeMs) &&
      combat.cast(kontrolle, timeMs).result === CAST_OK
    ) {
      kontrolleGeworfen = true;
    } else if (
      !quenchGeworfen &&
      timeMs >= QUENCH_BEI_MS &&
      combat.isReady(quench, timeMs) &&
      combat.cast(quench, timeMs).result === CAST_OK
    ) {
      quenchGeworfen = true;
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }
  }

  return {
    damageTaken: combat.damageTaken,
    unterbrochen: combat.interruptsLanded,
    verpasst: combat.interruptsMissed,
    bindWuerfe: combat.castCount(bind),
    quenchWuerfe: combat.castCount(quench),
  };
}

describe('Die Kontrolle senkt den erlittenen Schaden', () => {
  it('alle drei Läufe drücken QUENCH genau einmal', () => {
    // **Die Zusicherung vor der Messung.** Würde QUENCH nur im STALL-Lauf
    // geworfen, wäre die Differenz echt und der Nachweis trotzdem angreifbar:
    // Die Läufe unterschieden sich in zwei Dingen.
    for (const k of [null, 'stay', 'stall']) {
      expect(lauf(k).quenchWuerfe, `Lauf ${k ?? 'ohne'} drückt QUENCH nicht einmal`).toBe(1);
    }
  });

  it('ohne Kontrolle kommt der Zauber an', () => {
    const ohne = lauf(null);
    expect(ohne.damageTaken).toBe(560);
    expect(ohne.unterbrochen).toBe(0);
    expect(ohne.verpasst).toBe(1);
  });

  it('mit STALL trifft er gar nicht', () => {
    const mit = lauf('stall');
    expect(mit.damageTaken).toBe(0);
    expect(mit.unterbrochen).toBe(1);
    expect(mit.verpasst).toBe(0);
  });

  it('STAY trägt dieselbe Last und ändert trotzdem nichts', () => {
    // **Die Kontrolle.** STALL verdrängt einen BIND-Wurf; ein Vergleich gegen
    // „gar kein Kontroll-Zauber" unterschiede sich in der Wirkung UND einem
    // GCD. STAY kostet Punkte, löst denselben GCD aus, wirkt aber auf die
    // Zeitleiste statt auf den laufenden Cast.
    const ohne = lauf(null);
    const kontrolle = lauf('stay');
    const wirkung = lauf('stall');

    expect(kontrolle.damageTaken, 'STAY senkt den Schaden — dann ist er keine Kontrolle').toBe(
      ohne.damageTaken,
    );
    expect(kontrolle.unterbrochen).toBe(0);

    // Gleiche Last, gemessen statt behauptet.
    expect(kontrolle.bindWuerfe, 'STAY und STALL verdrängen verschieden viele BINDs').toBe(
      wirkung.bindWuerfe,
    );
    // Und beide verdrängen wirklich etwas — sonst wäre die Zeile darüber leer.
    expect(ohne.bindWuerfe).toBeGreaterThan(kontrolle.bindWuerfe);
  });

  it('und die Null ist die Struktur des Bosses, kein Randwert', () => {
    // `Sealing Chant` ist der EINZIGE Zeitleisten-Eintrag von boss-1-4. Es gibt
    // kein „ein Teil kommt durch" — der Zauber trifft ganz oder gar nicht.
    // Käme ein zweiter Eintrag dazu, wäre die 0 oben ein Zufall statt einer
    // Aussage, und dieser Fall meldet es.
    const boss = bossById(BOSS_ID);
    if (boss === undefined) throw new Error(`Der Boss ${BOSS_ID} fehlt.`);
    expect(boss.timeline).toHaveLength(1);
    expect(boss.timeline[0]?.mechanic).toBe('interruptibleCast');
    expect(boss.timeline[0]?.castTimeMs).toBe(2500);
  });
});
