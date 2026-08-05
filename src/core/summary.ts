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
  /**
   * Was einen Sieg vom anderen trennt (Issue #256).
   *
   * **Der Fortschritt sagt, WIE WEIT man kam; diese Zahl, WIE GUT.** Bei
   * einem gewöhnlichen Boss sind alle Siege 100 % Fortschritt, und getrennt
   * werden sie durch die Zeit — wer schneller war, war besser.
   *
   * Bei einem Boss mit `runaway` ist die Zeit fest: Der Sieg IST das Ablaufen
   * der Frist, die Reserve ist immer 0,00 s und sagt nichts. Dort trennt der
   * Ring-Rest — wer mehr Reaktion übrig hat, hat besser gehalten.
   *
   * | | Fortschritt | trennt die Siege |
   * | --- | --- | --- |
   * | gewöhnlich | Schaden am Ring | Restzeit |
   * | mit `runaway` | überstandene Frist | **Ring-Rest** |
   *
   * Dass die beiden Größen die Plätze tauschen, ist die Umkehrung selbst und
   * keine Ausnahme daneben.
   *
   * `null` bei einem gewöhnlichen Boss: Dort trennt die Restzeit, die schon
   * dasteht. Eine zweite Zahl wäre der „zweite Satz Anzeigen", den die Auflage
   * verbietet.
   */
  readonly heldPercent: number | null;
  readonly damage: number;
  readonly bossHp: number;
  /**
   * Fortschrittsmarke in Prozent (GAME-DESIGN 12.8; Issue #256).
   *
   * **Größer ist besser, bei jedem Boss** — nur die gemessene Größe wechselt:
   *
   * | | Fortschritt | 100 % heißt |
   * | --- | --- | --- |
   * | gewöhnlich | Schaden am Ring | der Boss ist gefallen |
   * | mit `runaway` | überstandene Frist | die Frist ist überstanden |
   *
   * Beide Male sagt die Zahl dasselbe: **wie weit man kam.** Ein Spieler, der
   * „85 %" liest, weiß bei beiden Boss-Arten, was er weiß — er muss keine
   * zweite Lesart lernen.
   *
   * Das war vor #256 nicht so: Bei einem `runaway`-Boss zählte der Schaden am
   * Ring, und der ist dort der Weg zur Niederlage. Ein verlorener Lauf stand
   * bei 100 %, ein gewonnener bei 84 % — die Anzeige feierte, was den Lauf
   * beendet hatte.
   *
   * **Die Gegenrichtung wurde geprüft und verworfen:** Der Ring-Rest als
   * Fortschritt hätte den Sieg bei 16 % stehen lassen, also als schlechtestes
   * Ergebnis der Anzeige. Er trennt stattdessen die SIEGE untereinander —
   * dort, wo bei einem gewöhnlichen Boss die Restzeit steht.
   */
  readonly progressPercent: number;
  readonly casts: number;

  // --- die drei Kennzahlen ------------------------------------------------
  /** Nach Verlust sortiert, größter Leerlauf zuerst. */
  readonly idlePerSpell: SpellIdle[];
  readonly lostTimeMs: number;
  readonly lostTimeShare: number;
  /**
   * Der Leerlauf, getrennt nach seiner Ursache (Issue #81, AC7).
   *
   * Die beiden Zahlen ergeben zusammen immer `lostTimeMs` und ueberschneiden
   * sich in keinem Takt — sie entstehen aus einem Wenn-Sonst in `Session.tick`.
   * Aus `lostTimeMs` allein waeren sie nicht mehr herstellbar.
   */
  readonly lostTimeWithReadyMs: number;
  readonly lostTimeNothingReadyMs: number;
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

/**
 * Wie weit der Lauf kam, in Prozent (Issue #256).
 *
 * **Eine Entscheidung, an drei Stellen angewandt.** `progress-mark.ts` und
 * `run-compare.ts` rechnen mit diesem Wert weiter und nehmen beide an, dass
 * größer besser ist. Solange diese Funktion das einhält, stimmen sie mit —
 * und niemand muss dreimal getrennt nachdenken.
 *
 * Bei einem Boss mit `runaway` ist es die überstandene Frist, sonst der
 * Schaden am Ring. Beide Male: 100 % heißt, das Ziel ist erreicht.
 */
export function progressOf(combat: Combat, elapsedMs: number): number {
  if (combat.boss.runaway !== null) {
    const frist = combat.boss.overpressureMs;
    // Ein Boss ohne Frist hätte kein Ziel — dann ist jeder Augenblick gleich
    // weit, und 0 ist ehrlicher als eine Division durch null.
    if (frist <= 0) return 0;
    return Math.min(100, (100 * Math.max(elapsedMs, 0)) / frist);
  }
  if (combat.boss.hp <= 0) return 0;
  return Math.min(100, (100 * combat.damageDealt) / combat.boss.hp);
}

/**
 * Wie viel Ring am Ende noch stand, in Prozent (Issue #256).
 *
 * Nur bei einem Boss mit `runaway`; sonst `null`. Siehe `heldPercent`.
 */
export function heldOf(combat: Combat): number | null {
  if (combat.boss.runaway === null || combat.boss.hp <= 0) return null;
  const gehalten = combat.boss.hp - combat.damageDealt;
  return Math.min(100, Math.max(0, (100 * gehalten) / combat.boss.hp));
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
  /** Die beiden Haelften des Leerlaufs (Issue #81, AC7). Summe = lostTimeMs. */
  readonly lostTimeWithReadyMs: number;
  readonly lostTimeNothingReadyMs: number;
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
    heldPercent: heldOf(combat),
    damage: combat.damageDealt,
    bossHp: combat.boss.hp,
    progressPercent: progressOf(combat, elapsedMs),
    casts: input.casts,
    idlePerSpell,
    lostTimeMs: input.lostTimeMs,
    lostTimeWithReadyMs: input.lostTimeWithReadyMs,
    lostTimeNothingReadyMs: input.lostTimeNothingReadyMs,
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
