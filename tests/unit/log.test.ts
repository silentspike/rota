import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import {
  CombatLog,
  EVENT_CAST,
  EVENT_DAMAGE,
  EVENT_DOT_TICK,
  EVENT_KINDS_WRITTEN_BY_GAME,
  EVENT_NAMES,
  LOG_CAPACITY,
} from '../../src/core/log.js';
import { PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

function makeCombat(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('Kampfprotokoll — Ringpuffer (AC1)', () => {
  it('ist für einen Zehn-Minuten-Kampf ausgelegt', () => {
    const log = new CombatLog();
    expect(log.capacity).toBe(LOG_CAPACITY);
    expect(LOG_CAPACITY).toBe(10_000);
    // 14 Byte je Eintrag: Int32 + Uint8 + Int8 + Float64.
    expect(log.byteLength).toBe(10_000 * 14);
    expect(log.byteLength).toBeLessThan(200_000);
  });

  it('behält bei genau 10.000 Einträgen jedes Ereignis', () => {
    const log = new CombatLog();
    for (let i = 0; i < LOG_CAPACITY; i++) log.push(i * 10, EVENT_DAMAGE, i % 3, i + 1);

    expect(log.length).toBe(LOG_CAPACITY);
    expect(log.droppedCount).toBe(0);
    expect(log.hasOverflowed).toBe(false);
    // Jeder einzelne Eintrag, nicht nur der erste und letzte.
    for (let i = 0; i < LOG_CAPACITY; i++) {
      expect(log.timeAt(i)).toBe(i * 10);
      expect(log.spellAt(i)).toBe(i % 3);
      expect(log.valueAt(i)).toBe(i + 1);
      expect(log.typeAt(i)).toBe(EVENT_DAMAGE);
    }
  });

  it('gibt 10.000 Einträge vollständig als Liste aus', () => {
    const log = new CombatLog();
    for (let i = 0; i < LOG_CAPACITY; i++) log.push(i, EVENT_CAST, 0, i);
    const list = log.toArray();

    expect(list.length).toBe(LOG_CAPACITY);
    expect(list[0]?.timeMs).toBe(0);
    expect(list[LOG_CAPACITY - 1]?.timeMs).toBe(LOG_CAPACITY - 1);
    expect(list.every((e) => e.type === 'cast')).toBe(true);
  });
});

describe('Kampfprotokoll — Überlauf (AC4)', () => {
  it('meldet einen Überlauf, statt still zu verwerfen', () => {
    const log = new CombatLog(100);
    for (let i = 0; i < 250; i++) log.push(i, EVENT_DAMAGE, 0, i);

    expect(log.length).toBe(100);
    expect(log.totalWritten).toBe(250);
    expect(log.droppedCount).toBe(150);
    expect(log.hasOverflowed).toBe(true);
  });

  it('behält nach dem Überlauf die jüngsten Einträge in der Reihenfolge', () => {
    const log = new CombatLog(5);
    for (let i = 0; i < 12; i++) log.push(i, EVENT_DAMAGE, 0, i);

    expect(log.toArray().map((e) => e.timeMs)).toEqual([7, 8, 9, 10, 11]);
  });

  it('meldet keinen Überlauf, solange Platz ist', () => {
    const log = new CombatLog(100);
    for (let i = 0; i < 100; i++) log.push(i, EVENT_DAMAGE, 0, i);
    expect(log.hasOverflowed).toBe(false);
    expect(log.droppedCount).toBe(0);

    log.push(100, EVENT_DAMAGE, 0, 100);
    expect(log.hasOverflowed).toBe(true);
    expect(log.droppedCount).toBe(1);
  });

  it('weist unsinnige Größen zurück', () => {
    expect(() => new CombatLog(0)).toThrow(RangeError);
    expect(() => new CombatLog(2.5)).toThrow(RangeError);
  });
});

describe('Kampfprotokoll — was der Kampf hineinschreibt', () => {
  it('schreibt Wirkung und Schaden getrennt', () => {
    const combat = makeCombat();
    const log = new CombatLog();
    combat.log = log;
    combat.cast(combat.indexOf('kindle'), 0);

    // Ein Schaden-Ereignis und ein Wirkungs-Ereignis, in dieser Reihenfolge.
    expect(log.length).toBe(2);
    expect(log.typeAt(0)).toBe(EVENT_DAMAGE);
    expect(log.valueAt(0)).toBe(100);
    expect(log.typeAt(1)).toBe(EVENT_CAST);
  });

  it('schreibt jeden DoT-Tick', () => {
    const combat = makeCombat();
    const log = new CombatLog();
    combat.log = log;
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    for (let t = TICK_MS; t <= 12_000; t += TICK_MS) combat.stepPassive(t);

    expect(log.countOf(EVENT_DOT_TICK)).toBe(6);
    expect(log.sumOf(EVENT_DOT_TICK)).toBe(240);
  });

  it('schreibt in Kampfzeit, nicht in der durchlaufenden Uhr', () => {
    const combat = makeCombat();
    const log = new CombatLog();
    combat.log = log;
    combat.beginFight(5000); // Kampf beginnt erst nach fünf Sekunden
    combat.cast(combat.indexOf('kindle'), 7000);

    // 7000 auf der durchlaufenden Uhr sind 2000 Kampfzeit.
    expect(log.timeAt(0)).toBe(2000);
  });

  it('kostet den Kampf nichts, wenn niemand mitschreibt', () => {
    const combat = makeCombat();
    combat.log = null;
    expect(() => combat.cast(combat.indexOf('kindle'), 0)).not.toThrow();
  });

  it('kennt für jede Art einen Namen', () => {
    // Acht seit E3-6, zehn seit #96: Der Nummernraum ist mit dem Simulator
    // geteilt, und die Namen stehen auch für die Arten, die nur er schreibt.
    // Wer ein fremdes Protokoll liest, soll denselben Namen sehen statt ein
    // Fragezeichen.
    // Sechzehn seit #79: Der Simulator führt inzwischen Spieler-Leben (#110)
    // und die drei Wege der Koagulation (#79). Die Namen stehen auch für die
    // Arten, die diese Seite nie schreibt — wer ein fremdes Protokoll liest,
    // soll denselben Namen sehen statt ein Fragezeichen.
    expect(EVENT_NAMES.length).toBe(16);
    expect(EVENT_NAMES[EVENT_CAST]).toBe('cast');
    expect(EVENT_NAMES[EVENT_DOT_TICK]).toBe('dotTick');
    expect(EVENT_NAMES[4]).toBe('spend');
    expect(EVENT_NAMES[5]).toBe('kill');
    // Die beiden aus #96 — Boss-Energieleiste und Boss-Zeitleiste. Die Namen
    // sagen ausdrücklich `Triggered`/`Fired`, weil beide **Vorgänge** sind und
    // keine Zustände: Der Stand der Leiste heißt `Combat.bossEnergy`, der
    // Zeiger `Combat.timelineIndex`. Genau diese beiden wurden beim ersten
    // Lesen verwechselt.
    expect(EVENT_NAMES[8]).toBe('bossEnergyTriggered');
    expect(EVENT_NAMES[9]).toBe('bossTimelineFired');
    // Die drei aus #79 — auch sie Vorgänge, nicht Zustände.
    expect(EVENT_NAMES[13]).toBe('shieldAbsorb');
    expect(EVENT_NAMES[14]).toBe('healed');
    expect(EVENT_NAMES[15]).toBe('convertedToShield');
  });

  it('schreibt nur die Arten, die dem Spiel gehören', () => {
    // Die Gegenprobe zur Aufteilung: Was der Simulator führt, taucht hier nie
    // auf. Eine Zahl, die beide Seiten schreiben, aber verschieden meinen,
    // war genau der Fehler, den E3-6 behoben hat.
    expect(EVENT_KINDS_WRITTEN_BY_GAME).not.toContain(4);
    expect(EVENT_KINDS_WRITTEN_BY_GAME).not.toContain(5);
    expect(EVENT_KINDS_WRITTEN_BY_GAME).toEqual([0, 1, 2, 3, 6, 7]);
  });
});

describe('Kurzfassung — die drei Kennzahlen (AC3)', () => {
  function setup(): { session: Session; combat: Combat; queue: PressQueue } {
    const queue = new PressQueue();
    const combat = makeCombat();
    const session = new Session(queue, combat);
    session.bind(0, combat.indexOf('kindle'));
    session.bind(1, combat.indexOf('smoulder'));
    session.bind(2, combat.indexOf('pyre'));
    session.startFight(0);
    return { session, combat, queue };
  }

  it('rechnet die verlorene Zeit wie von Hand', () => {
    // Handrechnung: 100 Takte, im ersten wird gewirkt, danach sperrt die
    // Wartezeit 150 Takte. Also 0 Takte Leerlauf in den ersten 100.
    const a = setup();
    a.queue.push(0, 0, 0, 0);
    for (let i = 0; i < 100; i++) a.session.tick(i * TICK_MS);
    expect(a.session.summary(1000).lostTimeMs).toBe(0);

    // Ohne jede Eingabe sind alle 100 Takte Leerlauf: 100 × 10 ms = 1000 ms.
    const b = setup();
    for (let i = 0; i < 100; i++) b.session.tick(i * TICK_MS);
    const summary = b.session.summary(1000);
    expect(summary.lostTimeMs).toBe(1000);
    expect(summary.lostTimeShare).toBeCloseTo(1);
  });

  it('rechnet den Leerlauf je Zauber wie von Hand', () => {
    // 300 Takte (3 s) ohne jede Eingabe: Alle drei Zauber liegen die ganze Zeit
    // bereit, also je 3000 ms Leerlauf.
    const h = setup();
    for (let i = 0; i < 300; i++) h.session.tick(i * TICK_MS);
    const summary = h.session.summary(3000);

    // Vier Zeilen seit #110, aber nur drei mit Leerlauf: QUENCH ist in diesem
    // Aufbau nicht auf die Leiste gelegt, und was nicht belegt ist, kann nicht
    // versäumt werden.
    //
    // **Im Spiel liegt er auf Platz 3** (`STEP1_BINDINGS`), und dort zeigt sich
    // die eigentliche Schwäche dieser Liste: Ein Unterbrecher liegt fast immer
    // bereit, verliert aber keine Zeit — er wartet auf eine Gelegenheit. Die
    // Liste würde ihn trotzdem als Verlustquelle führen.
    //
    // **Das bleibt vorerst so, und zwar mit Absicht.** Das Problem ist
    // allgemeiner als der Unterbrecher: Jeder situative Zauber trifft es — ein
    // Schutzschild, der bereitliegt, weil kein Schlag kam, ist auch kein
    // verschenkter Leerlauf. Die richtige Rechnung ist nicht „hat er
    // stillgelegen", sondern „gab es eine Gelegenheit, die er ungenutzt ließ".
    // Das steht als Kriterium in Issue #81 (E7) und gehört dorthin.
    expect(summary.idlePerSpell.length).toBe(11);
    const belegt = summary.idlePerSpell.filter((e) =>
      ['kindle', 'smoulder', 'pyre'].includes(e.spellId),
    );
    expect(belegt.length).toBe(3);
    for (const entry of belegt) {
      expect(entry.idleMs).toBe(3000);
      expect(entry.share).toBeCloseTo(1);
    }
    // Der Unbelegte steht mit null da — nicht mit Leerlauf.
    expect(summary.idlePerSpell.find((e) => e.spellId === 'quench')?.idleMs).toBe(0);
  });

  it('zählt einen Zauber auf Abklingzeit nicht als Leerlauf', () => {
    // PYRE bei 20 Hitze wirken: 5 s Abklingzeit. In den folgenden 300 Takten
    // liegt PYRE NICHT bereit, KINDLE und SMOULDER schon (ab dem Ende der
    // Wartezeit bei 150 Takten).
    const h = setup();
    h.combat.setResource(20);
    h.queue.push(2, 0, 0, 0);
    for (let i = 0; i < 300; i++) h.session.tick(i * TICK_MS);
    const summary = h.session.summary(3000);
    const by = new Map(summary.idlePerSpell.map((e) => [e.spellId, e.idleMs]));

    // Wartezeit endet bei 1500 ms; ab da sind 150 Takte übrig = 1500 ms.
    expect(by.get('kindle')).toBe(1500);
    expect(by.get('smoulder')).toBe(1500);
    expect(by.get('pyre')).toBe(0); // 5 s Abklingzeit, nie wieder bereit
  });

  it('sortiert nach Verlust, größter zuerst', () => {
    const h = setup();
    h.combat.setResource(20);
    h.queue.push(2, 0, 0, 0);
    for (let i = 0; i < 300; i++) h.session.tick(i * TICK_MS);
    const list = h.session.summary(3000).idlePerSpell;

    for (let i = 1; i < list.length; i++) {
      expect((list[i - 1] as { idleMs: number }).idleMs).toBeGreaterThanOrEqual(
        (list[i] as { idleMs: number }).idleMs,
      );
    }
    // PYRE und QUENCH stehen beide bei null — PYRE, weil es auf Abklingzeit
    // liegt, QUENCH, weil es hier nicht belegt ist. Bei Gleichstand entscheidet
    // die Kennung, damit die Reihenfolge zwischen zwei Läufen nicht springt.
    // PYRE liegt auf Abklingzeit, die vier anderen sind hier nicht belegt —
    // alle fuenf stehen deshalb bei null, und bei Gleichstand entscheidet die
    // Kennung, damit die Reihenfolge zwischen zwei Laeufen nicht springt.
    const hinten = list.slice(-9).map((e) => e.spellId);
    expect(hinten).toEqual([
      'anchor',
      'bind',
      'crust',
      'pyre',
      'quench',
      'salve',
      'stall',
      'stay',
      'temper',
    ]);
    expect(list[list.length - 1]?.idleMs).toBe(0);
  });

  it('erfindet keine Synergie-Ausbeute, wo es keine Synergien gibt', () => {
    // GAME-DESIGN 5: Synergien sind schulübergreifend. Schritt 1 hat eine
    // Schule — also null mögliche Kombinationen, und keine Prozentzahl.
    const h = setup();
    for (let i = 0; i < 50; i++) h.session.tick(i * TICK_MS);
    const summary = h.session.summary(500);

    expect(summary.synergyAvailable).toBe(0);
    expect(summary.synergyUsed).toBe(0);
  });

  it('trägt Ausgang, Zeiten und Fortschrittsmarke', () => {
    const h = setup();
    h.combat.setResource(20);
    h.queue.push(2, 0, 0, 0);
    for (let i = 0; i < 10; i++) h.session.tick(i * TICK_MS);
    h.session.wipe(100);
    const summary = h.session.summary(100);

    expect(h.session.phase).toBe(PHASE_ENDED);
    expect(summary.outcome).toBe('wipe');
    expect(summary.victory).toBe(false);
    expect(summary.elapsedMs).toBe(100);
    expect(summary.remainingMs).toBe(59_900);
    expect(summary.damage).toBe(480);
    expect(summary.progressPercent).toBeCloseTo((100 * 480) / 4200);
    expect(summary.casts).toBe(1);
  });

  it('meldet den Zustand des Protokolls mit', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    for (let i = 0; i < 200; i++) h.session.tick(i * TICK_MS);
    const summary = h.session.summary(2000);

    expect(summary.logCapacity).toBe(LOG_CAPACITY);
    expect(summary.logOverflowed).toBe(false);
    expect(summary.logDropped).toBe(0);
    expect(summary.eventCounts['cast']).toBe(1);
    expect(summary.eventCounts['damage']).toBe(1);
  });

  it('setzt Protokoll und Leerlauf beim Zurücksetzen zurück', () => {
    const h = setup();
    h.queue.push(0, 0, 0, 0);
    for (let i = 0; i < 200; i++) h.session.tick(i * TICK_MS);
    h.session.reset();

    expect(h.session.log.length).toBe(0);
    expect(h.session.idleMs).toBe(0);
    // Vier Einträge: Das ist die Rohmessung je Zauber, und die führt QUENCH
    // mit (immer 0, weil er nicht mitgezählt wird). Erst die aufbereitete
    // Liste `idlePerSpell` lässt ihn weg.
    expect(Array.from(h.session.idlePerSpellMs)).toEqual([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  });
});
