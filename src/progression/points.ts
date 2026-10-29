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
import { pointsPermilleOf } from '../core/difficulty.js';
import { applyRewardFalloff, rewardFalloffPerMille } from './reward-falloff.js';

/** Was ein Lauf beisteuert — der Ausschnitt der Zusammenfassung, der zählt. */
export interface PointsSource {
  readonly victory: boolean;
  /** Restlaufzeit beim Sieg. */
  readonly remainingMs: number;
  /** Ring-Rest, nur bei einem `runaway`-Boss gesetzt (Issue #256). */
  readonly heldPercent: number | null;
}

/** Die beiden Stufen, aus denen der Abstand entsteht — und die Schwierigkeit. */
export interface PointsContext {
  readonly bossLevel: number;
  readonly playerLevel: number;
  /**
   * Die Schwierigkeitsstufe des Kampfes, 0 bis 2 (Issue #1177).
   *
   * Masterplan A.4: „Normal / Heroisch / Mythisch … **mehr Punkte**“. Fehlt
   * sie, ist es Normal — ein Aufrufer, der keine Stufe kennt, rechnet wie bis
   * #1177.
   */
  readonly difficulty?: number;
}

/** Die Rechnung in ihren Teilen — damit ein Prüffall sie einzeln fassen kann. */
export interface PointsBreakdown {
  /** Die Größe, aus der gerechnet wurde, in Sekunden-Einheiten. */
  readonly units: number;
  /** Was die Kurve daraus macht, vor dem Stufenabstand. */
  readonly base: number;
  /**
   * Was die Kurve aus dem bisherigen Bestwert macht, oder 0 (Issue #377).
   *
   * Der Abzug bei der Differenz-Auszahlung. 0 heißt: kein brauchbarer
   * Bestwert — dann ist die Differenz der volle Wert, ohne dass es dafür einen
   * Sonderfall bräuchte.
   */
  readonly previousBase: number;
  /** Ob nur der Rest-Ertrag fürs Wiederholen gezahlt wurde (Issue #377). */
  readonly repeatOnly: boolean;
  /** Der Abstandsfaktor in Promille, 1000 heißt voller Ertrag. */
  readonly falloffPerMille: number;
  /** Der Faktor der Schwierigkeitsstufe in Promille, 1000 auf Normal (#1177). */
  readonly difficultyPerMille: number;
  /** Was ausgezahlt wird. */
  readonly awarded: number;
}

/**
 * Die Einheit, aus der die Kurve rechnet.
 *
 * Sekunden bei einem gewöhnlichen Boss, umgerechnete Ring-Prozentpunkte bei
 * einem `runaway`. **Eine Funktion für den laufenden und den gespeicherten
 * Wert**: Beide sind dieselbe Größe, und zwei Umrechnungen wären zwei
 * Gelegenheiten, sie verschieden zu machen.
 */
function unitsFrom(istRing: boolean, wert: number): number {
  if (istRing) return Math.max(0, wert) * REWARD.heldPointsPerPercent;
  return Math.max(0, wert) / 1000;
}

/**
 * Die Einheit dieses Laufs.
 *
 * Eine Niederlage trägt nichts bei: 12.6 sagt „Restlaufzeit beim
 * Boss-**Sieg**".
 */
function unitsOf(source: PointsSource): number {
  if (!source.victory) return 0;
  const istRing = source.heldPercent !== null;
  return unitsFrom(istRing, istRing ? (source.heldPercent ?? 0) : source.remainingMs);
}

/** Was die Kurve aus einer Einheit macht. */
function curve(units: number): number {
  return Math.round(REWARD.pointsPerSecond * Math.pow(units, REWARD.pointsExponent));
}

/**
 * Was ein Lauf einbringt, in seine Teile zerlegt.
 *
 * ZWEI REIHENFOLGEN, BEIDE NICHT BELIEBIG
 * ---------------------------------------
 * **Erst die Kurve, dann der Abstand.** Wer zuerst mindert und dann
 * potenziert, verstärkt den Abstand mit dem Exponenten und macht aus „ein
 * Bruchteil bei 10 Stufen" ein Vielfaches davon.
 *
 * **Die Differenz auf den ungeminderten Werten.** Der Abstandsfaktor kann
 * sich zwischen zwei Läufen ändern, weil der Spieler aufsteigt. Auf
 * geminderten Werten zu subtrahieren hiesse, einen Aufstieg als
 * Verschlechterung zu verbuchen.
 *
 * @param previousBest Der bisherige Bestwert als ROHWERT (ms bzw. Prozent),
 *   oder `null`. Nicht die Punkte daraus: Der Spielstand hält, was erreicht
 *   wurde, nicht was gerechnet wurde (GAME-DESIGN 18.5).
 */
export function pointsBreakdown(
  source: PointsSource,
  context: PointsContext,
  previousBest: number | null = null,
): PointsBreakdown {
  const units = unitsOf(source);
  const base = curve(units);

  // **Ohne Bestwert ist der Abzug 0**, also die Differenz der volle Wert. Der
  // erste Sieg braucht damit keinen Sonderfall — und ein Sonderfall wäre die
  // Stelle, an der später „erster Lauf" und „Lauf ohne brauchbaren Eintrag"
  // auseinanderliefen.
  const previousBase =
    previousBest === null || !source.victory
      ? 0
      : curve(unitsFrom(source.heldPercent !== null, previousBest));
  const differenz = Math.max(0, base - previousBase);

  // **Rest-Ertrag fürs blosse Wiederholen** (GAME-DESIGN 12.6). Anteilig am
  // eigenen Lauf, nicht als fester Betrag: Ein guter Wiederholungslauf, der die
  // Marke knapp verfehlt, soll mehr abwerfen als ein schwacher.
  const repeatOnly = source.victory && differenz === 0;
  const roh = repeatOnly ? Math.trunc((base * REWARD.repeatPerMille) / 1000) : differenz;

  // **Die Schwierigkeit zuletzt, nach dem Abstand** (Issue #1177). Beide sind
  // Faktoren auf derselben Zahl, also wäre die Reihenfolge für reelle Zahlen
  // gleichgültig — für ganze nicht: Erst abzurunden und dann zu vervielfachen
  // vervielfachte den Rundungsfehler mit. So rundet jeder Faktor einmal, in
  // derselben Folge wie `data/difficulty.json` sie beschreibt.
  //
  // **Auf die Differenz, nicht auf die Kurve davor.** Der Bestwert, gegen den
  // `previousBest` rechnet, gehört zur selben Stufe (`usableBest` sucht je Boss
  // UND Stufe) — die Differenz ist also schon eine heroische, und der Faktor
  // macht aus ihr heroische Punkte.
  const difficultyPerMille = pointsPermilleOf(context.difficulty ?? 0);
  const gemindert = applyRewardFalloff(roh, context.bossLevel, context.playerLevel);

  return {
    units,
    base,
    previousBase,
    repeatOnly,
    // **Abgefragt, nicht zurückgerechnet.** `awarded / base` läge bei kleinen
    // Beträgen wegen der Ganzzahl-Division daneben, und der Prüffall zu den
    // beiden Währungen verglich dann Rundungsrauschen statt der Kurve.
    falloffPerMille: rewardFalloffPerMille(context.bossLevel, context.playerLevel),
    difficultyPerMille,
    awarded: Math.trunc((gemindert * difficultyPerMille) / 1000),
  };
}

/** Was ein Lauf einbringt. */
export function pointsForRun(
  source: PointsSource,
  context: PointsContext,
  previousBest: number | null = null,
): number {
  return pointsBreakdown(source, context, previousBest).awarded;
}
