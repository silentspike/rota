/**
 * Welcher Zauber auf welchem Platz liegt.
 *
 * **Die Belegung ist frei wählbar** (GAME-DESIGN 8.1) — was hier steht, ist die
 * Anfangsbelegung, mehr nicht. Die Oberfläche zum Umbelegen ist ausdrücklich noch
 * nicht entworfen (GAME-DESIGN 22).
 *
 * **Angenommen, nicht vorgegeben:** Die Zuordnung der drei Zauber aus Schritt 1 zu
 * Q, W und E steht in keinem Dokument. Gewählt ist die obere Reihe von links nach
 * rechts in der Reihenfolge, in der die Zauber eingeführt werden (STEP1-SPEC § 7:
 * KINDLE und SMOULDER auf Stufe 1, PYRE auf Stufe 2). Die Plätze liegen damit von
 * Anfang an fest, sodass ein später dazukommender Zauber keinen bestehenden
 * verschiebt — das ist die Auflage aus GAME-DESIGN 9.1.
 */

/** Ein Platz mit dem Zauber, der darauf liegt. */
export interface Binding {
  /** Belegung 0 bis 25, siehe `src/input/keys.ts`. */
  readonly slot: number;
  readonly spellId: string;
}

export const STEP1_BINDINGS: readonly Binding[] = [
  { slot: 0, spellId: 'kindle' }, // obere Reihe, erste Taste
  { slot: 1, spellId: 'smoulder' }, // obere Reihe, zweite Taste
  { slot: 2, spellId: 'pyre' }, // obere Reihe, dritte Taste
];

/** Kennung des Klangs eines Zaubers im Verzeichnis von Epic E8. */
export function spellSoundId(spellId: string): string {
  return `spell-${spellId}`;
}
