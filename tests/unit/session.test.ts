import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS, GlobalCooldown } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';
import { NEVER, NO_SPELL, Session, type FeedbackSink } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

class RecordingFeedback implements FeedbackSink {
  readonly accepted: number[] = [];
  readonly rejected: number[] = [];

  onPressAccepted(_slot: number, _spellIndex: number, timeMs: number): void {
    this.accepted.push(timeMs);
  }

  onPressRejected(_slot: number, _reason: number, timeMs: number): void {
    this.rejected.push(timeMs);
  }
}

interface Harness {
  readonly queue: PressQueue;
  readonly combat: Combat;
  readonly session: Session;
  readonly feedback: RecordingFeedback;
  /** Rechnet `ticks` Takte ab `fromMs` — so, wie es die Schleife täte. */
  readonly run: (ticks: number, fromMs?: number) => void;
}

/** Q = KINDLE, W = SMOULDER, E = PYRE (siehe `src/core/loadout.ts`). */
function setup(): Harness {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const feedback = new RecordingFeedback();
  const session = new Session(queue, combat, feedback);
  session.bind(0, combat.indexOf('kindle'));
  session.bind(1, combat.indexOf('smoulder'));
  session.bind(2, combat.indexOf('pyre'));
  return {
    queue,
    combat,
    session,
    feedback,
    run: (ticks: number, fromMs = 0): void => {
      for (let i = 0; i < ticks; i++) session.tick(fromMs + i * TICK_MS);
    },
  };
}

describe('GlobalCooldown', () => {
  it('ist zu Beginn bereit', () => {
    const gcd = new GlobalCooldown();
    expect(gcd.isReady(0)).toBe(true);
    expect(gcd.remainingMs(0)).toBe(0);
    expect(gcd.lengthMs).toBe(GCD_MS);
  });

  it('sperrt exakt 1,5 Sekunden', () => {
    const gcd = new GlobalCooldown();
    gcd.trigger(1000);

    expect(gcd.isReady(1000)).toBe(false);
    expect(gcd.isReady(2490)).toBe(false);
    expect(gcd.remainingMs(2490)).toBe(10);
    expect(gcd.isReady(2500)).toBe(true);
    expect(gcd.remainingMs(2500)).toBe(0);
  });

  it('rechnet den Fortschritt für die Anzeige aus', () => {
    const gcd = new GlobalCooldown();
    gcd.trigger(0);

    expect(gcd.progress(0)).toBeCloseTo(0);
    expect(gcd.progress(750)).toBeCloseTo(0.5);
    expect(gcd.progress(1500)).toBe(1);
  });

  it('driftet über 100 Auslösungen nicht', () => {
    const gcd = new GlobalCooldown();
    let now = 0;
    for (let i = 0; i < 100; i++) {
      gcd.trigger(now);
      now = gcd.readyAt;
    }
    expect(now).toBe(100 * GCD_MS);
    expect(Number.isInteger(now)).toBe(true);
  });

  it('weist krumme Längen zurück', () => {
    const gcd = new GlobalCooldown();
    expect(() => gcd.setLength(1000.5)).toThrow(RangeError);
    expect(() => gcd.setLength(-1)).toThrow(RangeError);
  });
});

describe('Session — Eingabe während der Wartezeit (AC4 aus E1-2)', () => {
  it('nimmt den ersten Druck an und wirkt den Zauber', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.run(1);

    expect(h.session.acceptedCount).toBe(1);
    expect(h.session.rejectedCount).toBe(0);
    expect(h.combat.castCount(h.combat.indexOf('kindle'))).toBe(1);
    expect(h.combat.damageDealt).toBe(100);
  });

  it('verwirft einen Druck während der Wartezeit — er wird nicht nachgeholt', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0); // Takt 0: angenommen
    h.queue.push(1, 50, 0, 0); // 500 ms: mitten in der Wartezeit
    h.run(400); // vier Sekunden

    expect(h.session.acceptedCount).toBe(1);
    expect(h.session.rejectedCount).toBe(1);
    // Wäre der Druck vorgemerkt worden, stünde hier eine zweite Annahme.
    expect(h.feedback.accepted).toEqual([0]);
    expect(h.feedback.rejected).toEqual([500]);
    expect(h.queue.size).toBe(0);
    expect(h.combat.castCount(h.combat.indexOf('smoulder'))).toBe(0);
  });

  it('nimmt den nächsten Druck genau nach Ablauf der Wartezeit an', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.queue.push(1, 150, 0, 0); // genau bei 1500 ms
    h.run(200);

    expect(h.session.acceptedCount).toBe(2);
    expect(h.session.rejectedCount).toBe(0);
  });

  it('weist einen Druck einen Takt zu früh ab', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.queue.push(1, 149, 0, 0); // 1490 ms
    h.run(200);

    expect(h.session.acceptedCount).toBe(1);
    expect(h.session.rejectedCount).toBe(1);
  });

  it('verwirft auch eine ganze Serie, ohne einen davon nachzuholen', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    for (let tick = 1; tick < 60; tick++) h.queue.push(1, tick, 0, 0);
    h.run(600);

    expect(h.session.acceptedCount).toBe(1);
    expect(h.session.rejectedCount).toBe(59);
  });
});

describe('Session — globale Wartezeit blockiert alle Zauber (AC5)', () => {
  it('sperrt nach einem Zauber auch alle anderen', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0); // KINDLE
    h.queue.push(1, 10, 0, 0); // SMOULDER, 100 ms später
    h.queue.push(2, 20, 0, 0); // PYRE, 200 ms später
    h.run(140); // bis 1390 ms, also innerhalb der Wartezeit

    expect(h.session.acceptedCount).toBe(1);
    expect(h.session.rejectedCount).toBe(2);
    for (const id of ['kindle', 'smoulder', 'pyre']) {
      const index = h.combat.indexOf(id);
      expect(h.combat.castCount(index)).toBe(id === 'kindle' ? 1 : 0);
    }
  });

  it('gibt alle Zauber im selben Takt wieder frei', () => {
    const h = setup();
    const kindle = h.combat.indexOf('kindle');
    const smoulder = h.combat.indexOf('smoulder');
    const pyre = h.combat.indexOf('pyre');
    h.combat.cast(kindle, 0);

    // Beide anderen sind selbst nicht auf Abklingzeit — nur die Wartezeit sperrt.
    expect(h.combat.isOffCooldown(smoulder, 0)).toBe(true);
    expect(h.combat.isOffCooldown(pyre, 0)).toBe(true);
    expect(h.combat.isReady(smoulder, 1490)).toBe(false);
    expect(h.combat.isReady(pyre, 1490)).toBe(false);
    expect(h.combat.isReady(smoulder, 1500)).toBe(true);
    expect(h.combat.isReady(pyre, 1500)).toBe(true);
  });
});

describe('Session — Zustand für die Anzeige', () => {
  it('merkt sich, wann eine Belegung zuletzt angenommen oder verworfen wurde', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.queue.push(1, 20, 0, 0);
    h.run(100);

    expect(h.session.acceptedAtMs[0]).toBe(0);
    expect(h.session.rejectedAtMs[1]).toBe(200);
    expect(h.session.acceptedAtMs[1]).toBe(NEVER);
    expect(h.session.lastPressedSlot).toBe(1);
  });

  it('behandelt einen leeren Platz als Fehleingabe', () => {
    const h = setup();
    h.queue.push(9, 0, 0, 0); // nichts darauf gelegt
    h.run(5);

    expect(h.session.spellOnSlot(9)).toBe(NO_SPELL);
    expect(h.session.acceptedCount).toBe(0);
    expect(h.session.rejectedCount).toBe(1);
    expect(h.combat.damageDealt).toBe(0);
  });

  it('kennt die Zahl der belegten Plätze', () => {
    const h = setup();
    expect(h.session.boundSlotCount()).toBe(3);
  });

  it('legt für jeden sichtbaren Ausgang eine Messspur ab', () => {
    const h = setup();
    h.queue.push(0, 0, 111.5, 4);
    h.queue.push(1, 10, 222.5, 9);
    h.run(100);

    expect(h.session.latency.size).toBe(2);
    expect(h.session.latency.frontStamp()).toBe(111.5);
    expect(h.session.latency.frontFrame()).toBe(4);
    h.session.latency.shift();
    expect(h.session.latency.frontStamp()).toBe(222.5);
  });

  it('zählt Takte ohne Handlung als Leerlauf', () => {
    const h = setup();
    h.run(10); // zehn Takte, nichts gedrückt
    expect(h.session.idleMs).toBe(100);

    const h2 = setup();
    h2.queue.push(0, 0, 0, 0);
    h2.run(10); // erster Takt wirkt, die neun übrigen liegen in der Wartezeit
    expect(h2.session.idleMs).toBe(0);
  });

  it('setzt sich vollständig zurück', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.run(10);
    h.session.reset();

    expect(h.session.acceptedCount).toBe(0);
    expect(h.session.rejectedCount).toBe(0);
    expect(h.session.lastPressedSlot).toBe(-1);
    expect(h.session.acceptedAtMs[0]).toBe(NEVER);
    expect(h.session.gcd.isReady(0)).toBe(true);
    expect(h.session.latency.size).toBe(0);
    expect(h.combat.damageDealt).toBe(0);
    expect(h.session.idleMs).toBe(0);
  });
});

describe('Session — Puffer wird nie zur Warteschlange', () => {
  it('leert alles, was dem laufenden Takt oder früher zugewiesen war', () => {
    const h = setup();
    for (let i = 0; i < 10; i++) h.queue.push(i % 3, 0, 0, 0);
    h.session.tick(0);

    expect(h.queue.size).toBe(0);
    expect(h.session.acceptedCount + h.session.rejectedCount).toBe(10);
  });

  it('lässt einen Druck aus einem späteren Takt liegen, bis dieser Takt kommt', () => {
    const h = setup();
    h.queue.push(0, 5, 0, 0);
    h.run(5);
    expect(h.queue.size).toBe(1);
    expect(h.session.acceptedCount).toBe(0);

    h.session.tick(50);
    expect(h.queue.size).toBe(0);
    expect(h.session.acceptedCount).toBe(1);
  });
});
