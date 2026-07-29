/**
 * Die Verlustrechnung nach dem Kampf (GAME-DESIGN 15.2, Issue #81).
 *
 * **Was hier NICHT passiert: eine bessere Rotation ausrechnen.** Es gibt keine
 * Referenz-Reihenfolge, keine Kombinatorik, keinen Simulator im Browser
 * (GAME-DESIGN 21.2). Das Spiel sagt nie "die optimale Reihenfolge waere A-B-C
 * gewesen" — es sagt "du hast diese vier Dinge liegen gelassen, zusammen 30 %".
 * Das Herausfinden der Reihenfolge IST das Spiel; wer es dem Spieler abnimmt,
 * nimmt ihm den Kern.
 *
 * Jede der vier Verlustquellen ist eine Multiplikation aus Werten, die bereits
 * im Kampfprotokoll stehen:
 *
 * | Quelle              | Rechnung                                                     |
 * | ------------------- | ------------------------------------------------------------ |
 * | Verpasste Wirkungen | (Kampfzeit / Abklingzeit - gewirkt) x Schaden je Wirkung      |
 * | Leerlauf            | (verlorene Zeit / GCD) x Durchschnittsschaden je Aktion       |
 * | Verpasste Synergien | (verfuegbar - genutzt) x Synergie-Bonus                       |
 * | Verpasste Unterbr.  | Anzahl x Schaden des durchgekommenen Boss-Zaubers             |
 *
 * WORAUF SICH DIE PROZENTE BEZIEHEN
 * ---------------------------------
 * Auf das **Gesamtpotenzial** dieses Durchlaufs: erzielter Schaden plus die
 * Summe aller Verluste. Nicht auf ein theoretisches Optimum - ein solches gibt
 * es hier bewusst nicht. "9 %" heisst also: neun Prozent dessen, was dieser
 * Durchlauf mit denselben Zaubern haette erreichen koennen, blieben liegen.
 */

/** Der Beitrag eines einzelnen Zaubers zur Rechnung. */
export interface SpellRun {
  readonly spellId: string;
  /** Anzeigename, steht im Spiel — englisch (Design-Gesetz 5). */
  readonly name: string;
  readonly cooldownMs: number;
  readonly casts: number;
  /** Tatsaechlich erzielter Schaden dieses Zaubers, aus dem Protokoll. */
  readonly damage: number;
  /**
   * Was eine Wirkung laut Zauber-Daten austeilt - Grundschaden plus die Summe
   * seiner DoT-Ticks.
   *
   * Nur als Rueckfall, wenn der Zauber KEIN einziges Mal gewirkt wurde: Dann
   * gibt es keinen gemessenen Wert, und ohne ihn wuerde die Analyse behaupten,
   * ein nie benutzter Zauber koste nichts. Das waere die gefaehrlichste Luecke
   * ueberhaupt, denn genau dieser Fall ist der teuerste.
   */
  readonly nominalDamagePerCast: number;
}

export interface LossInput {
  readonly elapsedMs: number;
  /** Erzielter Gesamtschaden. */
  readonly damage: number;
  /** Zahl der Wirkungen insgesamt. */
  readonly casts: number;
  /** Zeit ohne Handlung, in ms - kommt aus der Kurzfassung, nicht neu gerechnet. */
  readonly lostTimeMs: number;
  readonly gcdMs: number;
  readonly spells: readonly SpellRun[];
  readonly synergyUsed: number;
  readonly synergyAvailable: number;
  /** Schaden, den eine genutzte Synergie im Schnitt zusaetzlich bringt. */
  readonly synergyBonusDamage: number;
  readonly interruptsMissed: number;
  /** Schaden eines durchgekommenen Boss-Zaubers. */
  readonly interruptDamage: number;
}

export type LossKind = 'missedCasts' | 'idle' | 'synergy' | 'interrupt';

export interface LossSource {
  readonly kind: LossKind;
  /** Kurzname der Quelle, englisch. */
  readonly label: string;
  /** Liegengelassener Schaden. */
  readonly damage: number;
  /** Anteil am Gesamtpotenzial, 0 bis 1. */
  readonly share: number;
  /** Ein Satz fuer den Spieler, englisch (GAME-DESIGN 15.2). */
  readonly detail: string;
}

/** Wie ein einzelner Zauber ausgenutzt wurde. */
export interface SpellEfficiency {
  readonly spellId: string;
  readonly name: string;
  readonly casts: number;
  /** Wie oft der Zauber in die Kampfzeit gepasst haette. */
  readonly possibleCasts: number;
  /** casts / possibleCasts, 0 bis 1. Die fuehrende Groesse. */
  readonly efficiency: number;
  readonly missedCasts: number;
  readonly damage: number;
  /** Anteil am Gesamtschaden, 0 bis 1. */
  readonly damageShare: number;
  /** Liegengelassener Schaden dieses Zaubers. */
  readonly lostDamage: number;
}

export interface LossReport {
  /** Alle Quellen mit Verlust ueber 0, groesste zuerst. */
  readonly sources: LossSource[];
  /** Die drei groessten - sie werden hervorgehoben (GAME-DESIGN 15.2). */
  readonly top: LossSource[];
  readonly totalLost: number;
  /** Erzielt plus verloren. Bezugsgroesse aller Prozente. */
  readonly potential: number;
  /** Anteil, der liegen blieb, 0 bis 1. */
  readonly totalShare: number;
  readonly perSpell: SpellEfficiency[];
}

/**
 * Wie oft ein Zauber in die Kampfzeit gepasst haette.
 *
 * GAME-DESIGN 15.2 rechnet vor: 12 s Abklingzeit, 240 s Kampf, 20 Wirkungen.
 * Also abgerundet und ohne den Wurf bei Sekunde 0 mitzuzaehlen - die Rechnung
 * folgt dem Dokument, nicht der eigenen Intuition.
 */
export function possibleCasts(elapsedMs: number, cooldownMs: number): number {
  if (cooldownMs <= 0 || elapsedMs <= 0) return 0;
  return Math.floor(elapsedMs / cooldownMs);
}

/**
 * Was eine Wirkung dieses Zaubers wert war.
 *
 * Gemessen, solange es etwas zu messen gibt. Erst wenn der Zauber nie gewirkt
 * wurde, tritt der deklarierte Wert an seine Stelle.
 */
export function damagePerCast(spell: SpellRun): number {
  if (spell.casts > 0) return spell.damage / spell.casts;
  return spell.nominalDamagePerCast;
}

function plural(n: number, one: string, many: string): string {
  return n === 1 ? one : many;
}

function round(value: number): number {
  return Math.round(value);
}

/** Prozent fuer die Ausgabe an den Spieler - eine Nachkommastelle. */
function percent(share: number): string {
  return `${(share * 100).toFixed(1)} %`;
}

export function analyseLosses(input: LossInput): LossReport {
  const { elapsedMs, gcdMs } = input;

  // --- 1. Verpasste Wirkungen, je Zauber -----------------------------------
  const perSpell: SpellEfficiency[] = [];
  let missedCastsLoss = 0;
  let missedCastsCount = 0;
  for (const spell of input.spells) {
    const possible = possibleCasts(elapsedMs, spell.cooldownMs);
    const missed = Math.max(0, possible - spell.casts);
    const lost = missed * damagePerCast(spell);
    missedCastsLoss += lost;
    missedCastsCount += missed;
    perSpell.push({
      spellId: spell.spellId,
      name: spell.name,
      casts: spell.casts,
      possibleCasts: possible,
      // Mehr als 100 % ist moeglich - etwa durch einen Ruecksetzer
      // (GAME-DESIGN 4.1). Das wird nicht gekappt, sonst verschwaende die
      // Anzeige genau die Leistung, die sie zeigen soll.
      efficiency: possible > 0 ? spell.casts / possible : 0,
      missedCasts: missed,
      damage: spell.damage,
      damageShare: input.damage > 0 ? spell.damage / input.damage : 0,
      lostDamage: lost,
    });
  }
  // Fuehrende Groesse ist die Effizienz, nicht der Schadensanteil
  // (GAME-DESIGN 15.3): Der Schadensanteil ist bei jedem Durchlauf nahezu
  // gleich und misst die Zauber, nicht das Koennen.
  perSpell.sort((a, b) => a.efficiency - b.efficiency || a.spellId.localeCompare(b.spellId));

  // --- 2. Leerlauf ---------------------------------------------------------
  // Die verlorene Zeit kommt aus der Kurzfassung. Sie hier neu zu rechnen
  // waere eine zweite Wahrheit ueber denselben Sachverhalt.
  const wastedActions = gcdMs > 0 ? Math.floor(input.lostTimeMs / gcdMs) : 0;
  const averageDamagePerAction = input.casts > 0 ? input.damage / input.casts : 0;
  const idleLoss = wastedActions * averageDamagePerAction;

  // --- 3. Verpasste Synergien ---------------------------------------------
  const synergyMissed = Math.max(0, input.synergyAvailable - input.synergyUsed);
  const synergyLoss = synergyMissed * input.synergyBonusDamage;

  // --- 4. Verpasste Unterbrechungen ---------------------------------------
  const interruptLoss = Math.max(0, input.interruptsMissed) * input.interruptDamage;

  const totalLost = missedCastsLoss + idleLoss + synergyLoss + interruptLoss;
  const potential = input.damage + totalLost;
  const shareOf = (value: number): number => (potential > 0 ? value / potential : 0);

  const all: LossSource[] = [
    {
      kind: 'missedCasts',
      label: 'Missed casts',
      damage: missedCastsLoss,
      share: shareOf(missedCastsLoss),
      detail:
        `${missedCastsCount} ${plural(missedCastsCount, 'cast', 'casts')} missed — ` +
        `${round(missedCastsLoss).toLocaleString('en-US')} damage left on the table, ` +
        percent(shareOf(missedCastsLoss)),
    },
    {
      kind: 'idle',
      label: 'Idle time',
      damage: idleLoss,
      share: shareOf(idleLoss),
      detail:
        `${(input.lostTimeMs / 1000).toFixed(1)} s idle — ` +
        `${wastedActions} wasted ${plural(wastedActions, 'action', 'actions')}, ` +
        percent(shareOf(idleLoss)),
    },
    {
      kind: 'synergy',
      label: 'Missed synergies',
      damage: synergyLoss,
      share: shareOf(synergyLoss),
      detail:
        `${input.synergyUsed} of ${input.synergyAvailable} combinations used, ` +
        percent(shareOf(synergyLoss)),
    },
    {
      kind: 'interrupt',
      label: 'Missed interrupts',
      damage: interruptLoss,
      share: shareOf(interruptLoss),
      detail: `${input.interruptsMissed} missed, ` + percent(shareOf(interruptLoss)),
    },
  ];

  // Quellen ohne Verlust werden nicht gezeigt. In Schritt 1 gibt es weder
  // Synergien noch Unterbrechungen; dort eine Null auszuweisen hiesse, dem
  // Spieler eine Fehlerart vorzuhalten, die es noch gar nicht gibt.
  const sources = all
    .filter((source) => source.damage > 0)
    .sort((a, b) => b.damage - a.damage || a.kind.localeCompare(b.kind));

  return {
    sources,
    top: sources.slice(0, 3),
    totalLost,
    potential,
    totalShare: shareOf(totalLost),
    perSpell,
  };
}
