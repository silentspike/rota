/**
 * Der Kampf-Zustand: Eingabe, globale Wartezeit, Zauber.
 *
 * Hier treffen die Tastendrücke aus `src/input/` auf die Regeln aus `combat.ts`.
 * Zwei Regeln des Designs werden an dieser Stelle durchgesetzt:
 *
 * - Ein Druck, der nicht gewirkt werden kann, wird **verworfen**, nicht gemerkt
 *   (GAME-DESIGN 8.2 und 21.1). Der Spieler lernt sofort.
 * - Die Rückmeldung darauf ist rotes Aufblinken und ein dumpfer Ton.
 *
 * Der Ablauf je Takt folgt `tools/step1-check.py`: erst die laufenden Wirkungen,
 * dann die Prüfung auf das Kampfende, dann die Tastendrücke dieses Takts.
 */
import { CAST_OK, Combat, FIGHT_RUNNING, FIGHT_WON } from './combat.js';
import { buildSummary, type RunSummary } from './summary.js';
import {
  COUNTDOWN_MS,
  OUTCOME_NAMES,
  OUTCOME_NONE,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  PHASE_COUNTDOWN,
  PHASE_ENDED,
  PHASE_FIGHTING,
  PHASE_PREPARE,
} from './phases.js';
import { LatencyTrace } from './latency.js';
import { CombatLog } from './log.js';
import type { Simulation } from './loop.js';
import { TICK_MS } from './loop.js';
import { SLOT_COUNT } from '../input/keys.js';
import type { PressQueue } from '../input/queue.js';

/** Kein Zauber auf diesem Platz. */
export const NO_SPELL = -1;

/**
 * Gründe für eine verworfene Eingabe, die nicht aus `combat.ts` stammen.
 * Negativ, damit sie sich nie mit den `CAST_*`-Kennungen überschneiden.
 */
export const REJECT_EMPTY_SLOT = -1;
export const REJECT_FIGHT_OVER = -2;
export const REJECT_NO_TARGET = -3;

/** Nie gedrückt. */
export const NEVER = -1;

/**
 * Wohin die Rückmeldung geht, die nicht im Bild stattfindet — derzeit der Klang.
 * Der Kampf kennt weder Puffer noch Knoten; er sagt nur, was geschehen ist.
 */
export interface FeedbackSink {
  onPressAccepted(slot: number, spellIndex: number, timeMs: number): void;
  onPressRejected(slot: number, reason: number, timeMs: number): void;
}

export class Session implements Simulation {
  readonly latency = new LatencyTrace();

  /** Kampfzeit des letzten angenommenen Drucks je Belegung, sonst `NEVER`. */
  readonly acceptedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);
  /** Kampfzeit des letzten verworfenen Drucks je Belegung, sonst `NEVER`. */
  readonly rejectedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);

  /** Belegung → Zauber. `NO_SPELL`, solange der Platz leer ist. */
  private readonly bindings = new Int32Array(SLOT_COUNT).fill(NO_SPELL);

  /** Das Kampfprotokoll (GAME-DESIGN 18.2). */
  readonly log = new CombatLog();

  /**
   * Leerlauf je Zauber in ms — wie lange er ungenutzt bereit lag.
   *
   * Gezählt wird nur in Takten, in denen **gar nichts** gewirkt wurde. Ein
   * Zauber, der bereit war, während ein anderer gewirkt wurde, war eine
   * Entscheidung, kein Leerlauf.
   */
  readonly idlePerSpellMs: Float64Array;

  /** Liegt der Zauber auf mindestens einem Platz? */
  private readonly boundSpells: Uint8Array;

  private accepted = 0;
  private rejected = 0;
  private lastSlot = -1;
  private timeMs = 0;
  private idleTicks = 0;
  private phaseValue = PHASE_PREPARE;
  private outcomeValue = OUTCOME_NONE;
  private countdownEndsAtMs = 0;
  private endedAt = -1;

  constructor(
    private readonly queue: PressQueue,
    readonly combat: Combat,
    private readonly feedback: FeedbackSink | null = null,
  ) {
    this.idlePerSpellMs = new Float64Array(combat.spells.length);
    this.boundSpells = new Uint8Array(combat.spells.length);
    combat.log = this.log;
  }

  /** Abkürzung: Die globale Wartezeit gehört dem Kampf. */
  get gcd(): Combat['gcd'] {
    return this.combat.gcd;
  }

  get acceptedCount(): number {
    return this.accepted;
  }

  get rejectedCount(): number {
    return this.rejected;
  }

  /** Zuletzt gedrückte Belegung, oder -1. */
  get lastPressedSlot(): number {
    return this.lastSlot;
  }

  /** Kampfzeit, bis zu der gerechnet wurde. */
  get nowMs(): number {
    return this.timeMs;
  }

  /**
   * Takte, in denen gewirkt werden konnte, aber nichts gewirkt wurde.
   *
   * Grundlage der Kennzahl „verlorene Zeit" aus GAME-DESIGN 15.1; ausgewertet
   * wird sie in E1-7.
   */
  get idleMs(): number {
    return this.idleTicks * TICK_MS;
  }

  bind(slot: number, spellIndex: number): void {
    if (slot < 0 || slot >= SLOT_COUNT) return;
    this.bindings[slot] = spellIndex;
    if (spellIndex >= 0 && spellIndex < this.boundSpells.length) {
      this.boundSpells[spellIndex] = 1;
    }
  }

  spellOnSlot(slot: number): number {
    return this.bindings[slot] ?? NO_SPELL;
  }

  /** Plätze mit einem Zauber darauf — die einzigen, die die Leiste zeigt. */
  boundSlotCount(): number {
    let count = 0;
    for (let i = 0; i < SLOT_COUNT; i++) {
      if ((this.bindings[i] ?? NO_SPELL) !== NO_SPELL) count++;
    }
    return count;
  }

  tick(timeMs: number): void {
    this.timeMs = timeMs;

    // 0. Countdown. Der Übergang geschieht VOR allem anderen in diesem Takt:
    //    Uhr und Eingabe beginnen damit im selben Takt, nicht einen später.
    if (this.phaseValue === PHASE_COUNTDOWN && timeMs >= this.countdownEndsAtMs) {
      this.startFight(timeMs);
    }

    const fighting = this.phaseValue === PHASE_FIGHTING;

    // 1. laufende Wirkungen — nur im Kampf. In der Vorbereitung laufen allein die
    //    Abklingzeiten mit (GAME-DESIGN 10), es gibt keine Uhr und keinen Verfall.
    if (fighting) this.combat.stepPassive(timeMs);

    // 2. Kampfende
    if (fighting && this.combat.checkDefeat(timeMs)) {
      this.finish(
        this.combat.fightState === FIGHT_WON ? OUTCOME_VICTORY : OUTCOME_OVERPRESSURE,
        timeMs,
      );
    }

    // 3. Tastendrücke dieses Takts. Alles abarbeiten, was diesem Takt oder einem
    //    früheren zugewiesen war — dadurch kann der Puffer nie zur Warteschlange
    //    werden.
    const tickIndex = timeMs / TICK_MS;
    let casted = false;
    while (this.queue.size > 0 && this.queue.frontTick() <= tickIndex) {
      const slot = this.queue.frontSlot();
      const stampMs = this.queue.frontStamp();
      const frameIndex = this.queue.frontFrame();
      this.queue.shift();
      if (this.apply(slot, timeMs, stampMs, frameIndex)) casted = true;
    }

    if (this.phaseValue === PHASE_FIGHTING && !casted && this.combat.gcd.isReady(timeMs)) {
      this.idleTicks++;
      // Welcher Zauber lag in diesem Takt ungenutzt bereit? Nur belegte zählen —
      // ein Zauber, der nicht auf der Leiste liegt, kann nicht versäumt werden.
      for (let i = 0; i < this.boundSpells.length; i++) {
        if (this.boundSpells[i] === 1 && this.combat.isOffCooldown(i, timeMs)) {
          this.idlePerSpellMs[i] = (this.idlePerSpellMs[i] ?? 0) + TICK_MS;
        }
      }
    }
  }

  // ------------------------------------------------------------ Kampfablauf

  get phase(): number {
    return this.phaseValue;
  }

  get outcome(): number {
    return this.outcomeValue;
  }

  /** Restzeit des Countdowns in ms, sonst 0. */
  countdownRemainingMs(timeMs: number): number {
    if (this.phaseValue !== PHASE_COUNTDOWN) return 0;
    const left = this.countdownEndsAtMs - timeMs;
    return left > 0 ? left : 0;
  }

  /** Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf endete. */
  get endedAtMs(): number {
    return this.endedAt;
  }

  /**
   * Die Uhr, die der Spieler sieht.
   *
   * **Null in der Vorbereitung und im Countdown** (GAME-DESIGN 10: „Keine Uhr"),
   * und nach dem Ende stehengeblieben — die Auswertung zeigt die Zeit des
   * Kampfes, nicht die seit dem Laden der Seite. `Combat` kennt diesen
   * Unterschied nicht; er gehört zum Ablauf und damit hierher.
   */
  fightElapsedMs(timeMs: number): number {
    if (this.phaseValue === PHASE_PREPARE || this.phaseValue === PHASE_COUNTDOWN) return 0;
    return this.combat.fightElapsedMs(this.endedAt >= 0 ? this.endedAt : timeMs);
  }

  /** Restzeit bis zum Überdruck, aus Sicht des Spielers. */
  overpressureRemainingMs(timeMs: number): number {
    const left = this.combat.boss.overpressureMs - this.fightElapsedMs(timeMs);
    return left > 0 ? left : 0;
  }

  /**
   * „Ready" gedrückt: Der Countdown beginnt.
   *
   * Nur aus der Vorbereitung heraus — ein zweiter Druck während des Countdowns
   * darf ihn nicht verlängern.
   */
  beginCountdown(timeMs: number): boolean {
    if (this.phaseValue !== PHASE_PREPARE) return false;
    this.phaseValue = PHASE_COUNTDOWN;
    this.countdownEndsAtMs = timeMs + COUNTDOWN_MS;
    return true;
  }

  /**
   * Abbruch durch den Spieler (GAME-DESIGN 10: „Wipe Run").
   *
   * Gilt als Ende und führt zur **selben** Auswertung wie ein Sieg — nur mit
   * anderem Ausgang. Keine Strafe.
   */
  wipe(timeMs: number): boolean {
    if (this.phaseValue !== PHASE_FIGHTING && this.phaseValue !== PHASE_COUNTDOWN) return false;
    this.finish(OUTCOME_WIPE, timeMs);
    return true;
  }

  /**
   * Die Kurzfassung des Laufs (GAME-DESIGN 15.1).
   *
   * Wird **nach** dem Kampf gerufen, nicht im Bild — hier darf Speicher entstehen.
   */
  summary(timeMs: number): RunSummary {
    return buildSummary({
      combat: this.combat,
      log: this.log,
      outcome: OUTCOME_NAMES[this.outcomeValue] ?? '?',
      victory: this.outcomeValue === OUTCOME_VICTORY,
      elapsedMs: this.fightElapsedMs(timeMs),
      remainingMs: this.overpressureRemainingMs(timeMs),
      casts: this.accepted,
      lostTimeMs: this.idleMs,
      idlePerSpellMs: this.idlePerSpellMs,
      // In Schritt 1 gibt es keine Synergien: eine Schule, keine
      // schulübergreifenden Fenster (GAME-DESIGN 5). Beide Zahlen sind 0, und
      // die Ausbeute bleibt damit unbestimmt statt erfunden.
      synergyUsed: 0,
      synergyAvailable: 0,
    });
  }

  /**
   * Den Kampf in diesem Takt beginnen lassen.
   *
   * Derselbe Weg, den der Countdown an seinem Ende nimmt — Uhr und Eingabe gehen
   * dadurch zwangsläufig im selben Takt auf. Für die Entwicklungs-Werkzeuge
   * zusätzlich direkt aufrufbar (GAME-DESIGN 19.1: Zustand setzen).
   */
  startFight(timeMs: number): void {
    if (this.phaseValue === PHASE_ENDED) return;
    this.phaseValue = PHASE_FIGHTING;
    this.countdownEndsAtMs = timeMs;
    this.combat.beginFight(timeMs);
  }

  private finish(outcome: number, timeMs: number): void {
    if (this.phaseValue === PHASE_ENDED) return;
    this.phaseValue = PHASE_ENDED;
    this.outcomeValue = outcome;
    this.endedAt = timeMs;
  }

  /** @returns true, wenn tatsächlich gewirkt wurde. */
  private apply(slot: number, timeMs: number, stampMs: number, frameIndex: number): boolean {
    this.lastSlot = slot;
    const spellIndex = this.bindings[slot] ?? NO_SPELL;

    let accepted = false;
    let reason = CAST_OK;
    if (this.phaseValue !== PHASE_FIGHTING) {
      // In der Vorbereitung ist kein Ziel vorhanden: Alle Zauber auf den Boss
      // sind ausgegraut (GAME-DESIGN 10). In Schritt 1 sind das alle drei.
      reason = REJECT_NO_TARGET;
    } else if (spellIndex === NO_SPELL || this.combat.fightState !== FIGHT_RUNNING) {
      // Ein leerer Platz ist eine Fehleingabe wie jede andere.
      reason = spellIndex === NO_SPELL ? REJECT_EMPTY_SLOT : REJECT_FIGHT_OVER;
    } else {
      const outcome = this.combat.cast(spellIndex, timeMs);
      reason = outcome.result;
      accepted = outcome.result === CAST_OK;
    }

    if (accepted) {
      this.acceptedAtMs[slot] = timeMs;
      this.accepted++;
      if (this.feedback !== null) this.feedback.onPressAccepted(slot, spellIndex, timeMs);
    } else {
      this.rejectedAtMs[slot] = timeMs;
      this.rejected++;
      if (this.feedback !== null) this.feedback.onPressRejected(slot, reason, timeMs);
    }
    // Beide Ausgänge sind im nächsten Bild sichtbar: der eine als Aufleuchten,
    // der andere als rotes Blinken. Beide zählen daher für die Verzögerung.
    this.latency.push(stampMs, frameIndex);
    return accepted;
  }

  reset(): void {
    this.combat.reset();
    this.acceptedAtMs.fill(NEVER);
    this.rejectedAtMs.fill(NEVER);
    this.latency.clear();
    this.queue.clear();
    this.accepted = 0;
    this.rejected = 0;
    this.lastSlot = -1;
    this.timeMs = 0;
    this.idleTicks = 0;
    this.phaseValue = PHASE_PREPARE;
    this.outcomeValue = OUTCOME_NONE;
    this.countdownEndsAtMs = 0;
    this.endedAt = -1;
    this.log.clear();
    this.idlePerSpellMs.fill(0);
  }
}
