/**
 * Wie ein Wert am Gegenstand geschrieben wird (GAME-DESIGN 13.1b, Issue E6-1).
 *
 * > Wie in Diablo, wo `[+]%` und `[x]%` am Gegenstand stehen: **Multiplikative
 * > Werte bekommen eine eigene Schreibweise und Farbe.** Ohne diese
 * > Kennzeichnung kann ein Spieler nicht erkennen, warum ein Gegenstand viel
 * > besser ist als ein scheinbar gleichwertiger — und müsste es außerhalb des
 * > Spiels nachschlagen. (13.1b)
 *
 * # Warum das mehr ist als Kosmetik
 *
 * Zwei Gegenstände mit „+20 %" sind gleich stark. Zwei mit „×1,20" sind es
 * auch. Einer von jeder Sorte ist es **nicht** — und welcher gewinnt, hängt
 * davon ab, was der Spieler sonst trägt. Ohne sichtbaren Unterschied ist die
 * Entscheidung nicht bloß schwer, sondern **von der Oberfläche verborgen**.
 * Dieselbe Linie wie bei den Boss-Ankündigungen: Was man zum Spielen wissen
 * muss, steht im Spiel.
 *
 * # Zwei Merkmale, nicht eins
 *
 * Schreibweise **und** Farbe. Eine Farbe allein trägt nicht: Etwa jeder
 * zwölfte Mann sieht Rot und Grün nicht auseinander, und der Unterschied wäre
 * für ihn schlicht weg. Das Zeichen davor bleibt lesbar, auch wenn die Farbe
 * nichts sagt — dieselbe Überlegung wie bei der Hitze-Anzeige im HUD, die
 * ihren Zustand nicht allein über Farbe meldet.
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */

/** Ein Wert, der sich mit anderen seiner Art addiert. */
export const STAT_ADDITIVE = 0;
/** Ein Wert, der einen eigenen Faktor bildet. */
export const STAT_MULTIPLICATIVE = 1;

/** Die CSS-Klasse, die die Farbe trägt. */
export const CLASS_ADDITIVE = 'rota-stat-additive';
export const CLASS_MULTIPLICATIVE = 'rota-stat-multiplicative';

/** Ein Wert, wie er am Gegenstand steht. */
export interface StatLine {
  /** `ESSENCE`, `CATALYSIS`, … — englisch, wie alle Spieltexte. */
  readonly label: string;
  /** Der Anteil: 0,2 heißt 20 %. */
  readonly share: number;
  /** [`STAT_ADDITIVE`] oder [`STAT_MULTIPLICATIVE`]. */
  readonly kind: number;
}

/**
 * Die Schreibweise eines Werts.
 *
 * ```text
 *   additiv         +20%
 *   multiplikativ   x1.20
 * ```
 *
 * **Warum `x1.20` und nicht `x20%`:** Ein Multiplikator ist keine Zugabe,
 * sondern ein Faktor. „×20 %" läse sich als „ein Fünftel davon" — das
 * Gegenteil dessen, was gemeint ist. Der ausgeschriebene Faktor ist die
 * einzige Schreibweise, die man nicht falsch verstehen kann.
 */
export function formatStat(line: StatLine): string {
  if (line.kind === STAT_MULTIPLICATIVE) {
    return `x${(1 + line.share).toFixed(2)}`;
  }
  const percent = Math.round(line.share * 1000) / 10;
  const sign = percent < 0 ? '' : '+';
  return `${sign}${percent}%`;
}

/** Welche CSS-Klasse die Farbe eines Werts trägt. */
export function classOfStat(kind: number): string {
  return kind === STAT_MULTIPLICATIVE ? CLASS_MULTIPLICATIVE : CLASS_ADDITIVE;
}

/**
 * Sind zwei Werte in der Anzeige auseinanderzuhalten?
 *
 * Der Prüffall zu AC4 in einer Funktion — und zwar so, dass er **beide**
 * Merkmale verlangt. Ein Unterschied allein in der Farbe zählt nicht.
 */
export function isDistinguishable(a: StatLine, b: StatLine): boolean {
  if (a.kind === b.kind) return false;
  const gleicheSchreibweise = formatStat(a) === formatStat(b);
  const gleicheFarbe = classOfStat(a.kind) === classOfStat(b.kind);
  return !gleicheSchreibweise && !gleicheFarbe;
}
