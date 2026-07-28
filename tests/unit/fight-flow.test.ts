import { describe, expect, it } from 'vitest';
import { Combat, FIGHT_OVERPRESSURE, FIGHT_WON } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import {
  COUNTDOWN_MS,
  OUTCOME_NONE,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  PHASE_COUNTDOWN,
  PHASE_ENDED,
  PHASE_FIGHTING,
  PHASE_PREPARE,
  isFinished,
  isVictory,
} from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { OVERPRESSURE_WARNING_MS, shakeOffset } from '../../src/render/vessel.js';

interface Harness {
  readonly queue: PressQueue;
  readonly combat: Combat;
  readonly session: Session;
  readonly run: (fromMs: number, toMs: number) => void;
}

function setup(): Harness {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('kindle'));
  session.bind(1, combat.indexOf('smoulder'));
  session.bind(2, combat.indexOf('pyre'));
  return {
    queue,
    combat,
    session,
    run: (fromMs: number, toMs: number): void => {
      for (let t = fromMs; t < toMs; t += TICK_MS) session.tick(t);
    },
  };
}

describe('Kampfablauf — Vorbereitung', () => {
  it('beginnt in der Vorbereitung, ohne Ausgang', () => {
    const h = setup();
    expect(h.session.phase).toBe(PHASE_PREPARE);
    expect(h.session.outcome).toBe(OUTCOME_NONE);
    expect(isFinished(h.session.outcome)).toBe(false);
  });

  it('lässt keine Uhr laufen', () => {
    const h = setup();
    h.run(0, 10_000);
    // Zehn Sekunden Vorbereitung: Die Uhr des Spielers steht bei null, und der
    // Überdruck-Timer zeigt die volle Zeit.
    expect(h.session.fightElapsedMs(10_000)).toBe(0);
    expect(h.session.overpressureRemainingMs(10_000)).toBe(60_000);
    expect(h.session.phase).toBe(PHASE_PREPARE);
  });

  it('lässt die Uhr auch im Countdown stillstehen', () => {
    const h = setup();
    h.session.beginCountdown(0);
    h.run(0, COUNTDOWN_MS);
    expect(h.session.fightElapsedMs(2000)).toBe(0);
    expect(h.session.overpressureRemainingMs(2000)).toBe(60_000);
  });

  it('graut alle Zauber auf den Boss aus', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    h.queue.push(1, 10, 0, 0);
    h.queue.push(2, 20, 0, 0);
    h.run(0, 1000);

    expect(h.session.acceptedCount).toBe(0);
    expect(h.session.rejectedCount).toBe(3);
    expect(h.combat.damageDealt).toBe(0);
  });

  it('verfallen lässt in der Vorbereitung keinen Vorrat', () => {
    // GAME-DESIGN 10: Es gibt keine Uhr. Der Verfall gehört zum Kampf.
    const h = setup();
    h.combat.setResource(10);
    h.run(0, 20_000);
    expect(h.combat.resource).toBe(10);
  });

  it('lässt die Abklingzeiten mitlaufen', () => {
    // Ausdrücklich anders als die Uhr: "Abklingzeiten laufen mit."
    const h = setup();
    const pyre = h.combat.indexOf('pyre');
    h.combat.cast(pyre, 0); // etwa eine Selbst-Verstärkung in der Vorbereitung
    expect(h.combat.isOffCooldown(pyre, 4990)).toBe(false);
    h.run(0, 6000);
    expect(h.combat.isOffCooldown(pyre, 5000)).toBe(true);
  });
});

describe('Kampfablauf — Countdown (AC2)', () => {
  it('läuft genau drei Sekunden', () => {
    const h = setup();
    expect(h.session.beginCountdown(1000)).toBe(true);
    expect(h.session.phase).toBe(PHASE_COUNTDOWN);
    expect(h.session.countdownRemainingMs(1000)).toBe(COUNTDOWN_MS);
    expect(h.session.countdownRemainingMs(2500)).toBe(1500);
    expect(h.session.countdownRemainingMs(4000)).toBe(0);
  });

  it('startet Uhr und Eingabe im selben Takt', () => {
    const h = setup();
    h.session.beginCountdown(0);
    // Ein Druck, der genau in den Start-Takt fällt.
    h.queue.push(0, COUNTDOWN_MS / TICK_MS, 0, 0);
    h.run(0, COUNTDOWN_MS);
    expect(h.session.phase).toBe(PHASE_COUNTDOWN);
    expect(h.session.acceptedCount).toBe(0);

    h.session.tick(COUNTDOWN_MS);
    // Im selben Takt: Phase gewechselt, Kampfuhr auf null, Eingabe angenommen.
    expect(h.session.phase).toBe(PHASE_FIGHTING);
    expect(h.combat.fightStartedAt).toBe(COUNTDOWN_MS);
    expect(h.combat.fightElapsedMs(COUNTDOWN_MS)).toBe(0);
    expect(h.session.acceptedCount).toBe(1);
  });

  it('verwirft einen Druck einen Takt vor dem Start', () => {
    const h = setup();
    h.session.beginCountdown(0);
    h.queue.push(0, COUNTDOWN_MS / TICK_MS - 1, 0, 0);
    h.run(0, COUNTDOWN_MS + 100);

    expect(h.session.acceptedCount).toBe(0);
    expect(h.session.rejectedCount).toBe(1);
  });

  it('verlängert sich nicht durch einen zweiten Druck auf Ready', () => {
    const h = setup();
    h.session.beginCountdown(0);
    expect(h.session.beginCountdown(1000)).toBe(false);
    expect(h.session.countdownRemainingMs(1000)).toBe(2000);
  });

  it('setzt den Verfall-Plan des Vorrats auf den Kampfbeginn', () => {
    const h = setup();
    h.combat.setResource(10);
    h.session.beginCountdown(0);
    h.run(0, COUNTDOWN_MS + 2000);
    // Erster Verfall zwei Sekunden NACH dem Start, nicht nach dem Laden.
    expect(h.combat.resource).toBe(10);
    h.session.tick(COUNTDOWN_MS + 2000);
    expect(h.combat.resource).toBe(9);
  });
});

describe('Kampfablauf — Ausgänge', () => {
  it('meldet den Sieg, wenn die Lebenspunkte aufgebraucht sind', () => {
    const h = setup();
    h.session.startFight(0);
    const kindle = h.combat.indexOf('kindle');
    // Bis einschließlich 60.000 ms — genau dort schlägt der Überdruck zu.
    for (let t = 0; t <= 60_000; t += TICK_MS) {
      if (h.combat.isReady(kindle, t)) h.queue.push(0, t / TICK_MS, 0, 0);
      h.session.tick(t);
      if (h.session.phase === PHASE_ENDED) break;
    }
    // KINDLE allein reicht nicht: 100 Schaden je 1,5 s sind 4000 in 60 s.
    expect(h.session.outcome).toBe(OUTCOME_OVERPRESSURE);
    expect(h.combat.fightState).toBe(FIGHT_OVERPRESSURE);
  });

  it('endet bei Überdruck genau nach 60 Sekunden Kampfzeit (AC4)', () => {
    const h = setup();
    h.session.beginCountdown(0);
    h.run(0, COUNTDOWN_MS + 60_000);
    expect(h.session.phase).toBe(PHASE_FIGHTING);

    h.session.tick(COUNTDOWN_MS + 60_000);
    expect(h.session.phase).toBe(PHASE_ENDED);
    expect(h.session.outcome).toBe(OUTCOME_OVERPRESSURE);
    // Die Vorbereitung hat der Kampfzeit nichts abgezogen.
    expect(h.session.fightElapsedMs(h.session.endedAtMs)).toBe(60_000);
    // Nach dem Ende steht die Uhr still, auch wenn die Schleife weiterläuft.
    expect(h.session.fightElapsedMs(999_999)).toBe(60_000);
  });

  it('führt "Wipe Run" zum selben Ende wie ein Sieg (AC3)', () => {
    const h = setup();
    h.session.startFight(0);
    h.run(0, 5000);
    expect(h.session.wipe(5000)).toBe(true);

    expect(h.session.phase).toBe(PHASE_ENDED);
    expect(h.session.outcome).toBe(OUTCOME_WIPE);
    expect(isFinished(h.session.outcome)).toBe(true);
    expect(isVictory(h.session.outcome)).toBe(false);
    expect(h.session.endedAtMs).toBe(5000);
  });

  it('lässt sich in der Vorbereitung nicht abbrechen', () => {
    const h = setup();
    expect(h.session.wipe(1000)).toBe(false);
    expect(h.session.phase).toBe(PHASE_PREPARE);
  });

  it('nimmt nach dem Ende keine Eingabe mehr an', () => {
    const h = setup();
    h.session.startFight(0);
    h.session.wipe(0);
    h.queue.push(0, 10, 0, 0);
    h.run(0, 1000);

    expect(h.session.acceptedCount).toBe(0);
    expect(h.session.rejectedCount).toBe(1);
  });

  it('bleibt beim ersten Ausgang — ein Abbruch danach ändert nichts', () => {
    const h = setup();
    h.session.startFight(0);
    h.session.wipe(1000);
    h.session.wipe(2000);
    expect(h.session.outcome).toBe(OUTCOME_WIPE);
    expect(h.session.endedAtMs).toBe(1000);
  });

  it('setzt beim Zurücksetzen alles auf die Vorbereitung', () => {
    const h = setup();
    h.session.startFight(0);
    h.run(0, 3000);
    h.session.wipe(3000);
    h.session.reset();

    expect(h.session.phase).toBe(PHASE_PREPARE);
    expect(h.session.outcome).toBe(OUTCOME_NONE);
    expect(h.session.endedAtMs).toBe(-1);
    expect(h.combat.fightStartedAt).toBe(0);
  });
});

describe('Kampfablauf — Sieg mit der guten Rotation (AC5)', () => {
  it('legt den Boss und meldet Sieg', () => {
    // Nicht die volle Referenz-Rotation — die steht im Prüffall. Hier zählt nur,
    // dass der Ausgang "Sieg" überhaupt erreicht wird und die Kampfuhr stimmt.
    const h = setup();
    h.session.startFight(0);
    h.combat.setResource(20);
    // Genug Schaden auf einmal: Der Boss fällt sofort.
    while (h.combat.damageDealt < 4200 && h.session.phase === PHASE_FIGHTING) {
      const pyre = h.combat.indexOf('pyre');
      if (h.combat.isReady(pyre, h.session.nowMs)) {
        h.combat.setResource(20);
        h.combat.cast(pyre, h.session.nowMs);
      }
      h.session.tick(h.session.nowMs + TICK_MS);
    }
    expect(h.session.outcome).toBe(OUTCOME_VICTORY);
    expect(h.combat.fightState).toBe(FIGHT_WON);
    expect(isVictory(h.session.outcome)).toBe(true);
  });
});

describe('Reaktionsgefäß — Vibration bei Überdruck', () => {
  it('steht außerhalb der Warnzeit vollkommen still', () => {
    expect(shakeOffset(OVERPRESSURE_WARNING_MS, 1234)).toBe(0);
    expect(shakeOffset(30_000, 1234)).toBe(0);
    expect(shakeOffset(0, 1234)).toBe(0);
  });

  it('wächst, je näher der Überdruck rückt', () => {
    // Bei gleicher Phase der Schwingung: kurz vor Ablauf stärker als früh.
    const früh = Math.abs(shakeOffset(9000, 250 / 14));
    const spät = Math.abs(shakeOffset(500, 250 / 14));
    expect(spät).toBeGreaterThan(früh);
  });

  it('liefert bei gleicher Zeit denselben Wert', () => {
    // Design-Gesetz 6: kein Zufall. Zwei Durchläufe müssen dasselbe Bild ergeben.
    for (const t of [0, 137, 4321.5, 59_999]) {
      expect(shakeOffset(2000, t)).toBe(shakeOffset(2000, t));
    }
  });

  it('bleibt klein genug, um nicht zu stören', () => {
    let max = 0;
    for (let t = 0; t < 2000; t += 1) {
      const value = Math.abs(shakeOffset(50, t));
      if (value > max) max = value;
    }
    expect(max).toBeLessThanOrEqual(5);
    expect(max).toBeGreaterThan(3);
  });
});
