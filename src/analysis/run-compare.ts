/**
 * Vergleich mit dem Bestlauf (GAME-DESIGN 15.3, Issue #82).
 *
 * ZWEI KURVEN ÜBEREINANDER, NICHT ZWEI KURVEN NEBENEINANDER
 * ---------------------------------------------------------
 * Und beide auf derselben **absoluten** Zeitachse. Der naheliegende Weg wäre,
 * jede Kurve auf ihre eigene Länge zu strecken — dann liegen Anfang und Ende
 * übereinander und die Formen ließen sich schön vergleichen. Genau das wäre
 * falsch: Der kürzere Lauf war schneller, und das ist die halbe Aussage. Eine
 * gestreckte Kurve versteckt sie.
 *
 * Deshalb endet die kürzere Kurve früher, sichtbar.
 *
 * WAS EINE DIFFERENZ BEDEUTET, HÄNGT AN DER KENNZAHL
 * --------------------------------------------------
 * Bei der Kampfzeit ist weniger besser, beim Schaden mehr. Ein Vorzeichen
 * allein sagt deshalb nichts — jede Kennzahl trägt mit, in welche Richtung
 * „besser" zeigt, und die Ansicht liest es dort ab, statt es zu wissen.
 *
 * WAS HIER NIE PASSIERT
 * ---------------------
 * Eine Empfehlung. Der Vergleich zeigt, wo die beiden Läufe auseinandergehen —
 * nie, was zu tun gewesen wäre (GAME-DESIGN 21.2).
 */
import type { CombatLog } from '../core/log.js';
import { EVENT_CRIT, EVENT_DAMAGE, EVENT_DOT_TICK } from '../core/log.js';

/** Ein abgetasteter Verlauf. */
export interface RunCurve {
  /** Abstand zweier Punkte in ms. */
  readonly stepMs: number;
  /**
   * Aufsummierter Wert je Stützstelle, beginnend bei 0 ms.
   *
   * Aufsummiert und nicht je Abschnitt: Der Spieler will sehen, wo er
   * vorne lag, nicht wo einzelne Sekunden stärker waren.
   */
  readonly points: readonly number[];
  readonly durationMs: number;
}

/** Die Protokoll-Arten, die Schaden am Boss bedeuten. */
const DAMAGE_KINDS: readonly number[] = [EVENT_DAMAGE, EVENT_DOT_TICK, EVENT_CRIT];

/**
 * Den Schadensverlauf aus dem Kampfprotokoll abtasten.
 *
 * `stepMs` bestimmt die Auflösung. Eine Sekunde reicht für einen lesbaren
 * Verlauf und hält die Zahl der Punkte auch bei langen Kämpfen klein — das ist
 * die Größe, die später je Durchlauf abgelegt wird, und 240 Zahlen je Lauf sind
 * etwas anderes als 10 000 Protokoll-Einträge.
 */
export function sampleDamageCurve(log: CombatLog, durationMs: number, stepMs = 1000): RunCurve {
  const steps = durationMs > 0 && stepMs > 0 ? Math.floor(durationMs / stepMs) + 1 : 1;
  const points = new Array<number>(steps).fill(0);

  for (let i = 0; i < log.length; i++) {
    if (!DAMAGE_KINDS.includes(log.typeAt(i))) continue;
    const bucket = Math.floor(log.timeAt(i) / stepMs);
    // Einträge nach dem Ende — etwa ein Tick im letzten Takt — fallen auf die
    // letzte Stützstelle statt unter den Tisch.
    const at = Math.min(bucket, steps - 1);
    points[at] = (points[at] ?? 0) + log.valueAt(i);
  }

  // Aus den Abschnitten die Summe machen.
  for (let i = 1; i < points.length; i++) points[i] = (points[i] ?? 0) + (points[i - 1] ?? 0);

  return { stepMs, points, durationMs };
}

/** In welche Richtung „besser" zeigt. */
export type Direction = 'higher' | 'lower';

/**
 * Was ein Lauf mitbringen muss, um verglichen zu werden (Issue #82).
 *
 * # Warum nicht einfach `RunSummary`
 *
 * Die hat 29 Felder, der Vergleich liest sieben. Verlangte `compareRuns` die
 * ganze Zusammenfassung, müsste der Spielstand sie speichern — 22 Felder, die
 * niemand liest, darunter die einzigen Sammlungen (`idlePerSpell`,
 * `eventCounts`). Ein `BestRun` trägt deshalb genau diese sieben.
 *
 * **`RunSummary` erfüllt diesen Typ von selbst.** TypeScript vergleicht
 * strukturell: Jede vorhandene Übergabe einer Zusammenfassung bleibt gültig,
 * ohne eine einzige Zeile beim Aufrufer. Der Typ verengt, was verlangt wird —
 * er verlangt nichts Neues.
 */
export interface ComparableRun {
  readonly victory: boolean;
  readonly elapsedMs: number;
  readonly damage: number;
  readonly progressPercent: number;
  readonly casts: number;
  readonly lostTimeWithReadyMs: number;
  readonly lostTimeNothingReadyMs: number;
}

export interface ComparedMetric {
  readonly key: string;
  /** Beschriftung im Spiel — englisch (Design-Gesetz 5). */
  readonly label: string;
  readonly current: number;
  /** Der Wert des Bestlaufs, oder `null`, wenn es keinen gibt. */
  readonly best: number | null;
  /** `current - best`, oder `null` ohne Bestlauf. */
  readonly delta: number | null;
  readonly better: Direction;
  /**
   * Ob dieser Lauf in dieser Kennzahl besser war.
   *
   * `null` heißt **unbekannt**, nicht „nein": Ohne Bestlauf gibt es nichts zu
   * vergleichen, und ein `false` sähe aus wie ein Rückschritt.
   */
  readonly improved: boolean | null;
}

/** Die Kennzahlen des Vergleichs, in der Reihenfolge der Anzeige. */
const METRICS: readonly {
  key: string;
  label: string;
  better: Direction;
  of: (run: ComparableRun) => number;
  /**
   * Ob die Kennzahl nur zwischen zwei SIEGEN etwas aussagt.
   *
   * Die Kampfzeit ist so eine: Wer nach 22 s stirbt, war nicht schneller als
   * wer nach 30 s gewonnen hat -- er war frueher fertig. Ohne diese
   * Unterscheidung meldet der Vergleich eine Niederlage als Fortschritt.
   */
  victoryOnly?: boolean;
}[] = [
  // Fortschritt zuerst: Er ist die Kennzahl, die auch bei einer Niederlage
  // etwas aussagt (Issue #82, Fortschrittsmarke).
  { key: 'progress', label: 'PROGRESS', better: 'higher', of: (r) => r.progressPercent },
  { key: 'time', label: 'FIGHT TIME', better: 'lower', of: (r) => r.elapsedMs, victoryOnly: true },
  { key: 'damage', label: 'DAMAGE', better: 'higher', of: (r) => r.damage },
  { key: 'casts', label: 'CASTS', better: 'higher', of: (r) => r.casts },
  { key: 'idle', label: 'IDLE', better: 'lower', of: (r) => r.lostTimeNothingReadyMs },
  { key: 'missed', label: 'MISSED', better: 'lower', of: (r) => r.lostTimeWithReadyMs },
];

/**
 * Zwei Läufe gegeneinander.
 *
 * `best` darf `null` sein — beim allerersten Durchlauf gibt es keinen
 * Bestlauf, und der Vergleich sagt das, statt gegen Nullen zu rechnen.
 */
export function compareRuns(current: ComparableRun, best: ComparableRun | null): ComparedMetric[] {
  return METRICS.map((metric): ComparedMetric => {
    const currentValue = metric.of(current);
    if (best === null) {
      return {
        key: metric.key,
        label: metric.label,
        current: currentValue,
        best: null,
        delta: null,
        better: metric.better,
        improved: null,
      };
    }
    const bestValue = metric.of(best);
    const delta = currentValue - bestValue;
    // Eine Kennzahl, die nur zwischen Siegen gilt, bleibt sonst UNBEKANNT.
    // Der Wert und die Differenz stehen trotzdem da -- verschwiegen wuerde
    // sonst eine Zahl, die der Spieler im Bild sieht.
    const vergleichbar = metric.victoryOnly !== true || (current.victory && best.victory);
    return {
      key: metric.key,
      label: metric.label,
      current: currentValue,
      best: bestValue,
      delta,
      better: metric.better,
      // Gleichstand ist keine Verbesserung. Ihn als eine auszugeben hiesse,
      // dem Spieler Fortschritt zu melden, wo er auf der Stelle trat.
      improved: !vergleichbar
        ? null
        : delta === 0
          ? false
          : metric.better === 'higher'
            ? delta > 0
            : delta < 0,
    };
  });
}
