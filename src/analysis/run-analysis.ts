/**
 * Ein Durchlauf, ausgewertet (Issue #81).
 *
 * Die Stelle, an der die Kurzfassung des Kampfes auf die Verlustquellen und die
 * Effizienz-Tabelle trifft. Sie **rechnet selbst nichts** über die Umrechnung
 * von Takten in Fenster hinaus — das tun die beiden Module. Sie verbindet nur.
 *
 * WARUM DIE ZAHLEN VON AUSSEN KOMMEN
 * ----------------------------------
 * `missedInterrupts` ist heute `null`: Das Spiel schreibt die Boss-Gelegenheiten
 * nicht ins Protokoll (Issue #151). Der Wert wird deshalb **hereingereicht**,
 * nicht hier ermittelt — sobald #151 steht, ist der Anschluss eine Zeile, und
 * bis dahin weist `LossReport.unavailable` die Lücke aus, statt sie als Null
 * auszugeben.
 */
import type { Combat } from '../core/combat.js';
import type { RunSummary } from '../core/summary.js';
import {
  buildEfficiencyTable,
  type EfficiencyTable,
  type SpellEfficiencyInput,
} from './efficiency.js';
import {
  dotTickSource,
  idleSource,
  interruptSource,
  missedCastSource,
  rankLosses,
  synergySource,
  type DotTickLoss,
  type LossReport,
} from './loss-sources.js';

/** Was über einen mitgeführten Zauber in die Auswertung eingeht. */
export interface SpellRunData extends SpellEfficiencyInput {
  /** Verfallene Ticks durch zu frühes Erneuern (`combat.lostDotTicks`). */
  readonly lostDotTicks: number;
  readonly damagePerTick: number;
}

export interface RunAnalysisInput {
  readonly summary: RunSummary;
  readonly gcdMs: number;
  /**
   * Nur die **tatsächlich mitgeführten** Zauber (AC3).
   *
   * Eine andere Zusammenstellung ergibt andere Zahlen, ohne dass eine Zeile
   * Code sich ändert — deshalb steht die Liste hier und nicht in einer Tabelle
   * im Modul.
   */
  readonly spells: readonly SpellRunData[];
  /** Schaden je Synergie-Fenster. In Schritt 1 ohne Wirkung, weil es keine gibt. */
  readonly synergyBonusDamage: number;
  /** Ungenutzte Unterbrechungs-Gelegenheiten. `null`, bis #151 sie liefert. */
  readonly missedInterrupts: number | null;
  /** Schaden eines durchgekommenen Boss-Zaubers. */
  readonly damagePerLandedBossCast: number;
  /**
   * Wie viel Fortschritt der Boss zurueckgedreht hat (Issue #218).
   *
   * Steht als eigenes Feld und nicht in `summary`: Es ist keine Kennzahl des
   * SPIELERS, sondern eine des Bosses — und es gehoert deshalb auch in der
   * Auswertung in eine eigene Zeile.
   */
  readonly bossHealed: number;
}

export interface RunAnalysis {
  readonly table: EfficiencyTable;
  readonly losses: LossReport;
  /** Leere Fenster mit bereitem Zauber — die Grundlage der verpassten Wirkungen. */
  readonly missedCastWindows: number;
  /** Leere Fenster ohne bereiten Zauber. Überschneidet sich mit dem obigen nicht. */
  readonly idleWindows: number;
}

/** Takte in Handlungsfenster umrechnen. */
function windows(ms: number, gcdMs: number): number {
  return gcdMs > 0 ? Math.round(ms / gcdMs) : 0;
}

export function analyzeRun(input: RunAnalysisInput): RunAnalysis {
  const { summary, gcdMs } = input;

  // Die beiden Hälften des Leerlaufs kommen getrennt aus der Sitzung. Sie hier
  // aus `lostTimeMs` aufzuteilen wäre nicht möglich: Die Summe weiß nicht mehr,
  // ob in einem Takt etwas bereit lag (AC7).
  //
  // ZUERST DIE ZAHL DER LEEREN FENSTER, DANN DIE AUFTEILUNG DARAUS.
  // Jede Hälfte einzeln zu runden ergibt mehr Fenster, als es leere gab:
  // gemessen 1000 ms + 1000 ms → 1 + 1 = 2 Fenster, obwohl 2000 ms bei 1,5 s
  // Grundtakt nur eines sind. Der Verlust stünde damit höher, als er war —
  // und genau das verbietet AC7. Die Aufteilung folgt dem gemessenen
  // Verhältnis, die Summe steht durch die Konstruktion fest.
  const totalIdleWindows = windows(summary.lostTimeMs, gcdMs);
  const missedCastWindows =
    summary.lostTimeMs > 0
      ? Math.round((totalIdleWindows * summary.lostTimeWithReadyMs) / summary.lostTimeMs)
      : 0;
  const idleWindows = totalIdleWindows - missedCastWindows;

  const averageDamagePerCast = summary.casts > 0 ? summary.damage / summary.casts : 0;

  const dotLosses: DotTickLoss[] = input.spells
    .filter((spell) => spell.lostDotTicks > 0)
    .map((spell) => ({
      spellId: spell.spellId,
      lostTicks: spell.lostDotTicks,
      damagePerTick: spell.damagePerTick,
    }));

  const losses = rankLosses(
    summary.damage,
    [
      missedCastSource(missedCastWindows, averageDamagePerCast),
      idleSource(idleWindows, gcdMs, averageDamagePerCast),
      synergySource(summary.synergyUsed, summary.synergyAvailable, input.synergyBonusDamage),
      dotTickSource(dotLosses),
      interruptSource(input.missedInterrupts, input.damagePerLandedBossCast),
    ],
    input.bossHealed,
  );

  const table = buildEfficiencyTable({
    elapsedMs: summary.elapsedMs,
    gcdMs,
    casts: summary.casts,
    damage: summary.damage,
    lostTimeNothingReadyMs: summary.lostTimeNothingReadyMs,
    synergyUsed: summary.synergyUsed,
    synergyAvailable: summary.synergyAvailable,
    spells: input.spells,
  });

  return { table, losses, missedCastWindows, idleWindows };
}

/**
 * Die Zeilen fuer die **tatsaechlich mitgefuehrten** Zauber zusammenstellen (AC3).
 *
 * `boundSpellIndices` kommt aus der Zauberleiste, nicht aus der Schule: Wer
 * KINDLE nicht auf der Leiste hat, kann ihn nicht versaeumen, und er hat in
 * seiner Auswertung nichts verloren. Eine andere Zusammenstellung ergibt damit
 * andere Zahlen, ohne dass sich eine Zeile Code aendert.
 */
export function runDataFor(
  combat: Combat,
  summary: RunSummary,
  boundSpellIndices: readonly number[],
): SpellRunData[] {
  const idleById = new Map(summary.idlePerSpell.map((entry) => [entry.spellId, entry.idleMs]));
  const rows: SpellRunData[] = [];
  for (const index of boundSpellIndices) {
    const spell = combat.spells[index];
    if (spell === undefined) continue;
    rows.push({
      spellId: spell.id,
      casts: combat.castCount(index),
      idleMs: idleById.get(spell.id) ?? 0,
      damage: combat.damageOf(index),
      lostDotTicks: combat.lostDotTicks(index),
      damagePerTick: spell.dot?.tickDamage ?? 0,
      triggersGcd: spell.triggersGcd,
    });
  }
  return rows;
}

/**
 * Die Zahlen fuer die verpassten Unterbrechungen, aus dem Kampf gelesen.
 *
 * SEIT ISSUE #151 IST DAS MOEGLICH. Bis dahin schrieb nur der Simulator die
 * Boss-Ereignisse, und die Fehleranalyse laeuft im Browser -- ein Protokoll,
 * in dem die Groesse ueberall fehlt, kann eine Gelegenheit nicht belegen.
 *
 * GEZAEHLT WIRD DIE GELEGENHEIT, NICHT DIE BEREITSCHAFT (GAME-DESIGN 15.2).
 * `combat.interruptsMissed` steigt genau dann, wenn ein unterbrechbarer
 * Boss-Zauber seine Wirkzeit ueberstanden hat und durchgekommen ist. Ein
 * Unterbrecher, der bereitlag, weil es nichts zu unterbrechen gab, taucht
 * darin nicht auf -- und genau das ist der Fall, den die Regel verhindern soll
 * (AC5).
 */
export interface InterruptData {
  /** Ungenutzte Gelegenheiten. */
  readonly missed: number;
  /** Schaden eines durchgekommenen Boss-Zaubers. */
  readonly damagePerLandedCast: number;
  /**
   * Ob die unterbrechbaren Zauber dieses Bosses verschieden viel Schaden
   * machen.
   *
   * Dann ist `damagePerLandedCast` ein Mittelwert und die Rechnung
   * "Anzahl x Schaden" (GAME-DESIGN 15.2) eine Naeherung. Steht hier, damit
   * niemand die Zahl fuer genauer haelt, als sie ist.
   */
  readonly averaged: boolean;
}

export function interruptDataFor(combat: Combat): InterruptData {
  const unterbrechbare = combat.boss.timeline.filter(
    (eintrag) => eintrag.mechanic === 'interruptibleCast',
  );
  if (unterbrechbare.length === 0) {
    // Kein unterbrechbarer Zauber im ganzen Kampf. `0` und nicht `null`: Das
    // ist eine MESSUNG ("es gab nichts zu verpassen") und keine fehlende
    // Datengrundlage.
    return { missed: 0, damagePerLandedCast: 0, averaged: false };
  }
  const werte = unterbrechbare.map((eintrag) => eintrag.damage);
  const summe = werte.reduce((a, b) => a + b, 0);
  return {
    missed: combat.interruptsMissed,
    damagePerLandedCast: summe / werte.length,
    averaged: new Set(werte).size > 1,
  };
}
