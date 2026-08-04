/**
 * Der automatische Vergleich (GAME-DESIGN 13.6, Issue E6-4).
 *
 * > Jeder Fund zeigt sofort, ob er besser ist als das Getragene — mit
 * > ungefährer Schadensangabe aus den **Skalierungsfaktoren des
 * > Balance-Läufers**. Die Bewertung ist eine **Näherung** und kann bei
 * > ungewöhnlichen Zusammenstellungen danebenliegen.
 *
 * # Warum die Näherung als solche dastehen muss
 *
 * Die Auflage zum Issue sagt es: *„Ein Spieler, der sie für exakt hält, trifft
 * falsche Entscheidungen."* Eine Zahl ohne Kennzeichnung liest sich wie eine
 * Messung. Deshalb trägt jede Angabe ein `~` **und** einen Zusatz in Worten —
 * zwei Merkmale, wie bei der Unterscheidung additiv/multiplikativ (#114):
 * Ein Zeichen allein wird übersehen, ein Wort allein überlesen.
 *
 * # Was hier NICHT steht
 *
 * **Die Skalierungsfaktoren selbst.** Sie kommen aus dem Balance-Läufer
 * (#118) und stehen noch aus. Diese Datei nimmt sie entgegen; die Prüffälle
 * benutzen erfundene.
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */

/** Wie ein Fund gegen das Getragene abschneidet. */
export const COMPARE_BETTER = 1;
export const COMPARE_EQUAL = 0;
export const COMPARE_WORSE = -1;

/** Das Ergebnis eines Vergleichs. */
export interface Comparison {
  /** [`COMPARE_BETTER`], [`COMPARE_EQUAL`] oder [`COMPARE_WORSE`]. */
  readonly verdict: number;
  /** Der geschätzte Unterschied im Schaden, in Prozent. */
  readonly damagePercent: number;
  /** Die Angabe, wie sie am Fund steht — **immer** mit Näherungszeichen. */
  readonly text: string;
  /** Der Zusatz, der die Näherung in Worten benennt. */
  readonly caveat: string;
}

/**
 * Der Zusatz, der jede Angabe begleitet.
 *
 * Englisch (Design-Gesetz 5) und **nie leer** — eine Bewertung ohne ihn wäre
 * genau die Zahl, die ein Spieler für exakt hält.
 */
export const APPROXIMATION_CAVEAT = 'estimate — may be off for unusual builds';

/**
 * Vergleicht einen Fund mit dem Getragenen.
 *
 * `estimatedDamage` ist der geschätzte Schaden pro Sekunde, den der
 * Balance-Läufer aus seinen Skalierungsfaktoren errechnet — für den Fund und
 * für das getragene Stück.
 */
export function compareItems(estimatedWithFind: number, estimatedWithWorn: number): Comparison {
  const gueltig =
    Number.isFinite(estimatedWithFind) &&
    Number.isFinite(estimatedWithWorn) &&
    estimatedWithWorn > 0;
  if (!gueltig) {
    return {
      verdict: COMPARE_EQUAL,
      damagePercent: 0,
      text: '~0%',
      caveat: APPROXIMATION_CAVEAT,
    };
  }

  const anteil = (estimatedWithFind - estimatedWithWorn) / estimatedWithWorn;
  const prozent = Math.round(anteil * 1000) / 10;
  const verdict = prozent > 0 ? COMPARE_BETTER : prozent < 0 ? COMPARE_WORSE : COMPARE_EQUAL;
  // Das Näherungszeichen steht IMMER da — auch bei null, auch bei einer
  // glatten Zahl. Eine Ausnahme wäre die Stelle, an der jemand die Angabe für
  // exakt hielte.
  const vorzeichen = prozent > 0 ? '+' : '';
  return {
    verdict,
    damagePercent: prozent,
    text: `~${vorzeichen}${prozent}%`,
    caveat: APPROXIMATION_CAVEAT,
  };
}

/** Trägt eine Angabe beide Merkmale der Näherung — Zeichen und Wort? */
export function isMarkedAsApproximation(comparison: Comparison): boolean {
  return comparison.text.startsWith('~') && comparison.caveat.length > 0;
}
