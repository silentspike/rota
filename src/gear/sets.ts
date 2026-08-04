/**
 * Sätze (GAME-DESIGN 13.5, Issue E6-3).
 *
 * | Regel | Festlegung |
 * | --- | --- |
 * | **Nur im Endgame** | Während des Aufstiegs nur Einzelstücke |
 * | **Drei Bonus-Stufen** | zwei, vier und sechs Teile — der Sechser als Krönung |
 * | **Frei kombinierbar** | Satz-Teile passen auf beliebige Plätze; gezählt wird nur die **Anzahl** |
 * | **Werte unten, Rotation oben** | Werte-Boni auf den unteren Stufen, echte Rotations-Effekte auf den höheren |
 * | **Ein Satz je Schule** | „auch ungenutzte Zauber bekommen ihre Nische" |
 * | **Ein Topf mit den Legendären** | additiv untereinander (13.1b) |
 *
 * # Warum nur gezählt und nicht zugeordnet wird
 *
 * „Satz-Teile passen auf **beliebige** Plätze" ist die Regel, und sie steht
 * hier in der Bauform: Diese Datei kennt keine Plätze. Sie bekommt eine Liste
 * „welcher Satz sitzt auf Platz i" und zählt. Eine Zuordnung ließe sich gar
 * nicht ausdrücken.
 *
 * Das ist seit dem 2026-07-29 auch der Rest des Systems: Die
 * platz-spezifische Wertezuordnung wurde gestrichen, Plätze sind Behälter
 * (13.2).
 *
 * # Die Schwelle gilt „ab", nicht „genau"
 *
 * Fünf Teile geben denselben Bonus wie vier. Das klingt selbstverständlich und
 * ist es nicht: Eine Prüfung auf `=== 4` wäre die naheliegende Schreibweise
 * und würde bei fünf Teilen **nichts** geben — der Spieler verlöre einen
 * Bonus, weil er ein Teil zu viel trägt.
 *
 * # Was hier NICHT steht
 *
 * **Welche 18 Boni es gibt** (sechs Sätze mal drei Stufen). Das ist
 * Katalog-Inhalt und steht vor E10 aus (GAME-DESIGN 22, „Gegenstands-Katalog").
 * Diese Datei nimmt die Boni als **Daten** entgegen; die Prüffälle benutzen
 * erfundene Sätze, die nicht ausgeliefert werden.
 */

/** Kein Satz — ein gewöhnliches Stück. */
export const NO_SET = -1;

/**
 * Die drei Bonus-Stufen (13.5).
 *
 * **Nicht vorläufig.** Zwei, vier und sechs stehen im Design.
 */
export const SET_TIERS: readonly number[] = [2, 4, 6];

/** Wie viele Sätze es gibt — einer je Endgame-Instanz (Entscheidung 2026-07-29). */
export const SET_COUNT = 6;

/** Ein Werte-Bonus: ein Anteil, der in den Ausrüstungs-Topf fließt. */
export const BONUS_VALUE = 0;
/**
 * Ein **Rotations-Effekt**: verändert, wie die Rotation funktioniert.
 *
 * Er trägt keinen Anteil, sondern eine Kennung, auf die andere Mechanik
 * reagiert — wie ein legendärer Effekt. Welche es gibt, entscheidet der
 * Katalog.
 */
export const BONUS_ROTATION = 1;

/** Ein Bonus einer Stufe. */
export interface SetBonus {
  /** [`BONUS_VALUE`] oder [`BONUS_ROTATION`]. */
  readonly kind: number;
  /** Bei einem Werte-Bonus der Anteil (0,3 heißt „+30 %"), sonst 0. */
  readonly share: number;
  /** Bei einem Rotations-Effekt seine Kennung, sonst leer. */
  readonly effect: string;
}

/** Ein Satz: drei Stufen, eine Schule. */
export interface SetDef {
  readonly id: number;
  /** Auf welche Schule er zielt (13.5). */
  readonly school: string;
  /** Die Boni zu [`SET_TIERS`], in derselben Reihenfolge. */
  readonly tiers: readonly SetBonus[];
}

/**
 * Zählt, wie viele Teile je Satz getragen werden.
 *
 * `equipped[i]` ist der Satz auf Platz `i` oder [`NO_SET`]. Schreibt in ein
 * übergebenes Feld (18.2) und gibt zurück, wie viele Plätze überhaupt ein
 * Satz-Teil tragen.
 */
export function countPieces(equipped: readonly number[], out: Int32Array | number[]): number {
  for (let set = 0; set < out.length; set++) out[set] = 0;
  let getragen = 0;
  for (const set of equipped) {
    if (set === NO_SET) continue;
    if (!Number.isInteger(set) || set < 0 || set >= out.length) continue;
    out[set] = (out[set] ?? 0) + 1;
    getragen++;
  }
  return getragen;
}

/**
 * Wie viele Bonus-Stufen bei `pieces` Teilen ausgelöst sind.
 *
 * **Die Schwelle gilt „ab", nicht „genau"** — fünf Teile lösen die Stufen für
 * zwei und vier aus, nicht nur die für vier und schon gar nicht keine.
 */
export function tiersReached(pieces: number): number {
  let erreicht = 0;
  for (const schwelle of SET_TIERS) {
    if (pieces >= schwelle) erreicht++;
  }
  return erreicht;
}

/**
 * Ist die Stufe `tier` bei `pieces` Teilen ausgelöst?
 */
export function tierActive(pieces: number, tier: number): boolean {
  const schwelle = SET_TIERS[tier];
  return schwelle !== undefined && pieces >= schwelle;
}

/**
 * Der gesamte **Werte**-Anteil aller ausgelösten Satz-Boni.
 *
 * Er gehört in denselben Topf wie die legendären Effekte und ist dort
 * **additiv** (13.1b) — deshalb gibt diese Funktion eine Summe zurück und
 * keinen Faktor. Wer daraus einen Faktor macht, tut es an einer Stelle, für
 * alle Quellen gemeinsam.
 */
export function valueShareOf(sets: readonly SetDef[], pieces: Int32Array | number[]): number {
  let summe = 0;
  for (const satz of sets) {
    const getragen = pieces[satz.id] ?? 0;
    for (let tier = 0; tier < satz.tiers.length; tier++) {
      const bonus = satz.tiers[tier];
      if (bonus === undefined || bonus.kind !== BONUS_VALUE) continue;
      if (tierActive(getragen, tier)) summe += bonus.share;
    }
  }
  return summe;
}

/**
 * Die Kennungen aller ausgelösten **Rotations**-Effekte.
 *
 * In fester Reihenfolge — erst nach Satz, dann nach Stufe —, damit Spiel und
 * Simulator dieselbe Folge sehen (18.4).
 */
export function activeRotationEffects(
  sets: readonly SetDef[],
  pieces: Int32Array | number[],
): string[] {
  const effekte: string[] = [];
  for (const satz of sets) {
    const getragen = pieces[satz.id] ?? 0;
    for (let tier = 0; tier < satz.tiers.length; tier++) {
      const bonus = satz.tiers[tier];
      if (bonus === undefined || bonus.kind !== BONUS_ROTATION) continue;
      if (tierActive(getragen, tier)) effekte.push(bonus.effect);
    }
  }
  return effekte;
}
