/**
 * Welche Schul-Anzeigen über der Leiste erscheinen (GAME-DESIGN 5, Issue E3-5).
 *
 * > **Nur die aktiv genutzten**, direkt über der Zauberleiste. Wer keine
 * > CALCINATION mitführt, sieht keine Hitze-Anzeige. Nie mehr als drei bis vier
 * > gleichzeitig, und sie sitzen dort, wo der Blick ohnehin ist.
 *
 * Zwei Regeln, und beide sind Auswahl, nicht Zeichnung — deshalb stehen sie
 * hier und nicht in `src/render/`:
 *
 * 1. **Aktiv genutzt heißt: ein Zauber auf der Leiste zahlt darauf ein.** Nicht
 *    „die Schule existiert", nicht „der Spieler hat sie freigeschaltet". Eine
 *    Anzeige ohne zugehörigen Zauber wäre eine Zahl, die sich nie bewegt.
 * 2. **Höchstens vier.** Darüber hinaus wird abgeschnitten, nicht verkleinert:
 *    Fünf Anzeigen nebeneinander sind bei jeder Größe fünf Dinge, auf die der
 *    Blick fallen kann, und das Design nennt die Grenze in Anzeigen, nicht in
 *    Pixeln.
 *
 * # Welche vier, wenn es mehr sind
 *
 * Nach der **Zahl der einzahlenden Zauber**, bei Gleichstand nach der Kennung.
 * Begründung: Die Mechanik, auf die vier Zauber einzahlen, bewegt sich öfter
 * als die mit einem — sie ist die, die der Spieler im Blick behalten muss. Bei
 * Gleichstand entscheidet die Kennung, damit die Reihenfolge zwischen zwei
 * Bildern nicht springt.
 *
 * Das Design nennt keine Auswahlregel für den Fall; es nennt nur die Grenze.
 * Diese hier ist Umsetzung und in einer Zeile zu ändern.
 */

/** Nie mehr als so viele Anzeigen gleichzeitig (GAME-DESIGN 5). */
export const MAX_INDICATORS = 4;

/** Eine Mechanik, die eine Anzeige haben könnte. */
export interface MechanicInfo {
  /** Kennung des Vorrats, z. B. `heat`. */
  readonly id: string;
  /** Wie viele Zauber der Leiste darauf einzahlen. */
  readonly contributors: number;
}

/**
 * Welche Anzeigen erscheinen, in ihrer Reihenfolge.
 *
 * Läuft beim Zusammenstellen der Leiste, nicht im Bild — hier darf eine Liste
 * entstehen. Das Bild liest nur das Ergebnis.
 */
export function visibleIndicators(mechanics: readonly MechanicInfo[]): string[] {
  const genutzt = mechanics.filter((mechanic) => mechanic.contributors > 0);
  genutzt.sort((a, b) => b.contributors - a.contributors || a.id.localeCompare(b.id));
  return genutzt.slice(0, MAX_INDICATORS).map((mechanic) => mechanic.id);
}

/**
 * Die Mechaniken, die eine Zusammenstellung tatsächlich nutzt.
 *
 * **Gezählt wird über die Leiste, nicht über die Schule.** Ein Zauber zahlt
 * ein, wenn er beim Wirken oder bei einem Tick etwas beisteuert — steht kein
 * solcher Zauber auf der Leiste, gibt es nichts anzuzeigen.
 */
export function mechanicsOfLoadout(source: LoadoutSource): MechanicInfo[] {
  const id = source.resourceId;
  let contributors = 0;
  for (let slot = 0; slot < source.slotCount; slot++) {
    const spell = source.spellOnSlot(slot);
    if (spell < 0) continue;
    if (source.gainOfSpell(spell, id) > 0 || source.gainOfTick(spell, id) > 0) contributors++;
  }
  return [{ id, contributors }];
}

/**
 * Ein Fingerabdruck der Zusammenstellung — **ohne Speicheranforderung**.
 *
 * Die Auswahl selbst erzeugt eine Liste und darf deshalb nicht je Bild laufen
 * (GAME-DESIGN 18.2). Diese Zahl ändert sich genau dann, wenn sich die Auswahl
 * ändern könnte; wer sie mitführt, rechnet die Liste nur bei Bedarf neu.
 */
export function loadoutFingerprint(source: LoadoutSource): number {
  const id = source.resourceId;
  let fingerprint = 0;
  for (let slot = 0; slot < source.slotCount; slot++) {
    const spell = source.spellOnSlot(slot);
    if (spell < 0) continue;
    if (source.gainOfSpell(spell, id) > 0 || source.gainOfTick(spell, id) > 0) {
      // Der Platz geht mit ein, nicht nur die Anzahl: Ein Umbelegen, das die
      // Zahl gleich lässt, soll trotzdem auffallen.
      fingerprint = fingerprint * 31 + slot + 1;
    }
  }
  return fingerprint;
}

/** Woraus die Auswahl liest. Nur Abfragen — sie ändert nichts. */
export interface LoadoutSource {
  readonly resourceId: string;
  readonly slotCount: number;
  spellOnSlot(slot: number): number;
  gainOfSpell(spellIndex: number, resourceId: string): number;
  gainOfTick(spellIndex: number, resourceId: string): number;
}

/**
 * Wie viele Anzeigen weggelassen wurden.
 *
 * **Stillschweigend abzuschneiden wäre der Fehler.** Wer eine Mechanik
 * mitführt, deren Anzeige nicht erscheint, soll das nachlesen können — im
 * Werkzeug, nicht im Bild.
 */
export function hiddenIndicatorCount(mechanics: readonly MechanicInfo[]): number {
  const genutzt = mechanics.filter((mechanic) => mechanic.contributors > 0).length;
  return genutzt > MAX_INDICATORS ? genutzt - MAX_INDICATORS : 0;
}
