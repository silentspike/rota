/**
 * Die Punkte für einen Sieg (GAME-DESIGN 12.6; Issue #276).
 *
 * WAS 12.6 VERLANGT
 * -----------------
 * „Restlaufzeit beim Boss-Sieg, **überproportional** umgerechnet — die letzten
 * Sekunden sind unverhältnismäßig viel wert, damit Feinoptimierung sich lohnt."
 *
 * Überproportional heißt hier genau eines: die Kurve ist **streng konvex**. Aus
 * der Konvexität folgt die Prüfung, die das Kriterium trägt — zwei Siege mit
 * 10 s Unterschied zahlen mehr als doppelt so verschieden aus wie zwei mit
 * 5 s, und zwar für jede Ausgangs-Restzeit:
 *
 *     f(t+10) - f(t)  =  [f(t+10) - f(t+5)]  +  [f(t+5) - f(t)]
 *                         \_______________/      \____________/
 *                          bei konvexem f größer als der rechte Summand
 *
 * Eine lineare Formel trifft dabei exakt das Doppelte und fällt durch. Das ist
 * der Grund, warum `tests/unit/points.test.ts` das Verhältnis misst und nicht
 * einen Exponenten abfragt: Der Exponent ist austauschbar, die Konvexität nicht.
 *
 * DIE KURVE IST NICHT GEWÄHLT
 * ---------------------------
 * Issue #276 nennt sie ausdrücklich einen Blocker: „Quadratisch, kubisch,
 * exponentiell — alle drei erfüllen das Wort, und sie unterscheiden sich um
 * Größenordnungen." Der Exponent steht deshalb als benannter Wert in
 * `data/player/reward.json` und nicht als Zahl in diesem Rumpf. Diese Datei
 * rechnet mit ihm; sie entscheidet ihn nicht.
 *
 * ZWEI QUELLEN, EINE ENTSCHEIDUNG GEERBT
 * --------------------------------------
 * Bei einem `runaway`-Boss ist die Frist immer voll ausgeschöpft — Restzeit
 * ist dort konstant null und trennt keine zwei Siege. Was sie dort trennt, ist
 * der Ring-Rest; Issue #256 hat ihn als `heldPercent` in die Zusammenfassung
 * gelegt, und die Anzeige verzweigt seither an genau demselben Feld — in
 * `overlay.ts` steht `if (view.heldPercent === null)` vor der Zeile
 * „Remaining", mit dem Ring-Rest im anderen Zweig.
 *
 * **Die Formel erbt diese Verzweigung, sie trifft sie nicht neu.** Ein eigener
 * Rückschluss hier — etwa „Restzeit ist null, also runaway" — wäre eine zweite
 * Wahrheit über denselben Sachverhalt, und die beiden gingen irgendwann
 * auseinander.
 */
import { REWARD } from '../core/data.js';
import { applyRewardFalloff, rewardFalloffPerMille } from './reward-falloff.js';

/** Was ein Lauf beisteuert — der Ausschnitt der Zusammenfassung, der zählt. */
export interface PointsSource {
  readonly victory: boolean;
  /** Restlaufzeit beim Sieg. */
  readonly remainingMs: number;
  /** Ring-Rest, nur bei einem `runaway`-Boss gesetzt (Issue #256). */
  readonly heldPercent: number | null;
}

/** Die beiden Stufen, aus denen der Abstand entsteht. */
export interface PointsContext {
  readonly bossLevel: number;
  readonly playerLevel: number;
}

/** Die Rechnung in ihren Teilen — damit ein Prüffall sie einzeln fassen kann. */
export interface PointsBreakdown {
  /** Die Größe, aus der gerechnet wurde, in Sekunden-Einheiten. */
  readonly units: number;
  /** Was die Kurve daraus macht, vor dem Stufenabstand. */
  readonly base: number;
  /** Der Abstandsfaktor in Promille, 1000 heißt voller Ertrag. */
  readonly falloffPerMille: number;
  /** Was ausgezahlt wird. */
  readonly awarded: number;
}

/**
 * Die Einheit, aus der die Kurve rechnet.
 *
 * Sekunden bei einem gewöhnlichen Boss, umgerechnete Ring-Prozentpunkte bei
 * einem `runaway`. Eine Niederlage trägt nichts bei: 12.6 sagt „Restlaufzeit
 * beim Boss-**Sieg**".
 */
function unitsOf(source: PointsSource): number {
  if (!source.victory) return 0;
  if (source.heldPercent !== null) {
    return Math.max(0, source.heldPercent) * REWARD.heldPointsPerPercent;
  }
  return Math.max(0, source.remainingMs) / 1000;
}

/**
 * Was ein Lauf einbringt, in seine Teile zerlegt.
 *
 * Die Reihenfolge ist nicht beliebig: erst die Kurve, dann der Abstand. Wer
 * zuerst mindert und dann potenziert, verstärkt den Abstand mit dem Exponenten
 * und macht aus „ein Bruchteil bei 10 Stufen" ein Vielfaches davon.
 */
export function pointsBreakdown(source: PointsSource, context: PointsContext): PointsBreakdown {
  const units = unitsOf(source);
  const base = Math.round(REWARD.pointsPerSecond * Math.pow(units, REWARD.pointsExponent));
  return {
    units,
    base,
    // **Abgefragt, nicht zurückgerechnet.** `awarded / base` läge bei kleinen
    // Beträgen wegen der Ganzzahl-Division daneben, und der Prüffall zu den
    // beiden Währungen verglich dann Rundungsrauschen statt der Kurve.
    falloffPerMille: rewardFalloffPerMille(context.bossLevel, context.playerLevel),
    awarded: applyRewardFalloff(base, context.bossLevel, context.playerLevel),
  };
}

/** Was ein Lauf einbringt. */
export function pointsForRun(source: PointsSource, context: PointsContext): number {
  return pointsBreakdown(source, context).awarded;
}
