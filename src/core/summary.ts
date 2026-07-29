/**
 * Die Kurzfassung nach dem Kampf (GAME-DESIGN 15.1).
 *
 * Drei Kennzahlen, mehr nicht — die ausführliche Auswertung mit Effizienz-Tabelle
 * und Bestlauf-Vergleich gehört zu Epic E7:
 *
 * 1. **Leerlauf je Zauber** — wie viele Sekunden lag jeder Zauber ungenutzt
 *    bereit, nach Verlust sortiert
 * 2. **Verlorene Zeit gesamt** — Sekunden und Prozent ohne aktiven Zauber
 * 3. **Synergie-Ausbeute** — wie viel Prozent der möglichen Kombinationen
 *    genutzt wurden
 *
 * **„Ungenutzt bereit" heißt: gewirkt werden konnte, aber nichts gewirkt wurde.**
 * Ein Zauber, der bereit war, während ein anderer gewirkt wurde, zählt nicht —
 * das war eine Entscheidung, kein Leerlauf. Sonst bekäme der Spieler
 * Leerlauf angerechnet, gerade weil er etwas getan hat.
 */
import type { Combat } from './combat.js';
import { EVENT_CAST, EVENT_CRIT, EVENT_DAMAGE, EVENT_DOT_TICK, type CombatLog } from './log.js';

/** Leerlauf eines einzelnen Zaubers. */
export interface SpellIdle {
  readonly spellId: string;
  readonly idleMs: number;
  /** Anteil an der Kampfzeit, 0 bis 1. */
  readonly share: number;
  readonly casts: number;
}

export interface RunSummary {
  readonly outcome: string;
  readonly victory: boolean;
  /** Verstrichene Kampfzeit in ms. */
  readonly elapsedMs: number;
  /** Restzeit bis zum Überdruck in ms. Beim Sieg die Reserve. */
  readonly remainingMs: number;
  readonly damage: number;
  readonly bossHp: number;
  /** Fortschrittsmarke in Prozent (GAME-DESIGN 12.8). */
  readonly progressPercent: number;
  readonly casts: number;

  // --- die drei Kennzahlen ------------------------------------------------
  /** Nach Verlust sortiert, größter Leerlauf zuerst. */
  readonly idlePerSpell: SpellIdle[];
  readonly lostTimeMs: number;
  readonly lostTimeShare: number;
  /**
   * Genutzte von möglichen Synergien.
   *
   * In Schritt 1 gibt es **keine** Synergien: Alle drei Zauber gehören derselben
   * Schule, und schulübergreifende Fenster (GAME-DESIGN 5) entstehen erst mit
   * weiteren Schulen. `available` ist deshalb 0 und die Ausbeute unbestimmt —
   * hier eine Zahl zu erfinden hieße, dem Spieler etwas vorzurechnen.
   */
  readonly synergyUsed: number;
  readonly synergyAvailable: number;

  // --- Protokoll ----------------------------------------------------------
  readonly logEntries: number;
  readonly logCapacity: number;
  /** Muss 0 sein. Darüber ist der Puffer zu klein ausgelegt. */
  readonly logDropped: number;
  readonly logOverflowed: boolean;
  readonly eventCounts: Readonly<Record<string, number>>;
}

export interface SummaryInput {
  readonly combat: Combat;
  readonly log: CombatLog;
  readonly outcome: string;
  readonly victory: boolean;
  readonly elapsedMs: number;
  readonly remainingMs: number;
  readonly casts: number;
  /** Takte ohne Handlung, in ms — aus der Session. */
  readonly lostTimeMs: number;
  /** Leerlauf je Zauber in ms, nach Kennnummer. */
  readonly idlePerSpellMs: Float64Array;
  readonly synergyUsed: number;
  readonly synergyAvailable: number;
}

/**
 * Die Kurzfassung ausrechnen.
 *
 * Läuft **einmal nach dem Kampf**, nicht im Bild — hier darf Speicher entstehen.
 */
export function buildSummary(input: SummaryInput): RunSummary {
  const { combat, log, elapsedMs } = input;

  const idlePerSpell: SpellIdle[] = [];
  for (let i = 0; i < combat.spells.length; i++) {
    const spell = combat.spells[i];
    if (spell === undefined) continue;
    const idleMs = input.idlePerSpellMs[i] ?? 0;
    idlePerSpell.push({
      spellId: spell.id,
      idleMs,
      share: elapsedMs > 0 ? idleMs / elapsedMs : 0,
      casts: combat.castCount(i),
    });
  }
  // Nach Verlust sortiert (GAME-DESIGN 15.1). Bei Gleichstand nach Kennung,
  // damit die Reihenfolge zwischen zwei Läufen nicht springt.
  idlePerSpell.sort((a, b) => b.idleMs - a.idleMs || a.spellId.localeCompare(b.spellId));

  const eventCounts: Record<string, number> = {
    cast: log.countOf(EVENT_CAST),
    damage: log.countOf(EVENT_DAMAGE),
    crit: log.countOf(EVENT_CRIT),
    dotTick: log.countOf(EVENT_DOT_TICK),
  };

  return {
    outcome: input.outcome,
    victory: input.victory,
    elapsedMs,
    remainingMs: input.remainingMs,
    damage: combat.damageDealt,
    bossHp: combat.boss.hp,
    progressPercent:
      combat.boss.hp > 0 ? Math.min(100, (100 * combat.damageDealt) / combat.boss.hp) : 0,
    casts: input.casts,
    idlePerSpell,
    lostTimeMs: input.lostTimeMs,
    lostTimeShare: elapsedMs > 0 ? input.lostTimeMs / elapsedMs : 0,
    synergyUsed: input.synergyUsed,
    synergyAvailable: input.synergyAvailable,
    logEntries: log.length,
    logCapacity: log.capacity,
    logDropped: log.droppedCount,
    logOverflowed: log.hasOverflowed,
    eventCounts,
  };
}
