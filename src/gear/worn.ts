/**
 * Was der Spieler **trägt** — die drei Sonderwerte aus GAME-DESIGN 6.2,
 * ganzzahlig (Issue #177).
 *
 * Das Gegenstück zu `WornValues` in `sim/src/gear.rs`.
 *
 * # Warum Promille und nicht die Töpfe aus `pools.ts`
 *
 * `ValuePools` führt Anteile als Fließkommazahl, weil Schadensfaktoren
 * Verhältnisse sind. CATALYSIS wirkt aber auf **Zeiten**, und 18.4 verbietet
 * Fließkomma-Zeit. Ein Wert, der als Bruch ankommt und für die Zeitrechnung
 * gerundet werden müsste, wäre genau die Stelle, an der Spiel und Simulator um
 * ein Bit auseinanderlaufen können.
 *
 * 6.7 nimmt die Entscheidung vorweg: **„Ganzzahlig in Promille gerechnet."**
 * Die Werte kommen deshalb gar nicht erst als Bruch herein.
 *
 * | Feld         | Bedeutung                                  | 1000 heißt |
 * | ------------ | ------------------------------------------ | ---------- |
 * | `catalysis`  | Tempo — verkürzt Zeiten                    | +100 %     |
 * | `purity`     | Kritchance                                 | 100 %      |
 * | `yieldBonus` | Kritschaden **über der Basis** von 150 %   | +100 pp    |
 *
 * # Ohne getragene Ausrüstung sind alle drei null
 *
 * Das ist die Zusicherung aus AC7 in der Bauform: `NOTHING_WORN` ist der
 * Zustand, in dem sich nichts ändern darf — keine Zeit, kein Wurf, keine
 * Prüfsumme.
 */

/** Was der Spieler trägt: die drei Sonderwerte aus 6.2 und Topf 4 aus 13.1b. */
export interface WornValues {
  /** CATALYSIS in Promille. */
  readonly catalysis: number;
  /** PURITY in Promille. */
  readonly purity: number;
  /** YIELD in Promille, **zusätzlich** zur Basis aus `CRIT_BASE_PERMILLE`. */
  readonly yieldBonus: number;
  /**
   * **Topf 4** in Promille: Satz-Boni **und** legendäre Effekte zusammen
   * (13.1b).
   *
   * # Warum dieses Feld erst nach #177 kam
   *
   * #177 hat die drei Sonderwerte aus 6.2 verdrahtet. Topf 4 blieb liegen,
   * obwohl der Befund im Issue ihn ausdrücklich nannte — keine einzige AC hat
   * ihn aufgeführt. Aufgefallen ist es bei #118, wo „bester Satz-Aufbau gegen
   * besten Affix-Aufbau" gemessen werden soll und die Satz-Seite gar nicht im
   * Kampf ankam.
   *
   * # Eine Verdrahtung, zwei Quellen
   *
   * 13.1b legt Satz-Boni und legendäre Effekte in **denselben** Topf, wo sie
   * untereinander addiert werden. Nach außen ist es ein Faktor.
   */
  readonly gearEffects: number;
}

/**
 * Der Kritschaden ohne jeden YIELD-Beitrag: **150 %** (GAME-DESIGN 6.7).
 *
 * Steht hier und nicht in `damage.ts`, weil es eine Eigenschaft des Wertes ist
 * und nicht der Formel — `factors.crit` nimmt das Ergebnis entgegen.
 */
export const CRIT_BASE_PERMILLE = 1500;

/** Nichts getragen — alle vier Werte null. */
export const NOTHING_WORN: WornValues = {
  catalysis: 0,
  purity: 0,
  yieldBonus: 0,
  gearEffects: 0,
};

/** Ob überhaupt etwas getragen wird. */
export function isNothingWorn(worn: WornValues): boolean {
  return (
    worn.catalysis === 0 && worn.purity === 0 && worn.yieldBonus === 0 && worn.gearEffects === 0
  );
}

/**
 * Der Faktor von Topf 4 für die Schadensformel.
 *
 * **Eins plus die Summe** — dieselbe Regel wie in `ValuePools.factor`, nur
 * ganzzahlig hereingegeben. Ohne getragene Ausrüstung ist es exakt 1,0, und 1,0
 * ist in IEEE754 die exakte Eins: Der Faktor bewegt dann keine einzige Zahl.
 */
export function gearEffectFactor(worn: WornValues): number {
  return 1 + worn.gearEffects / 1000;
}

/**
 * Die Kritchance als Wahrscheinlichkeit in `[0, 1]`.
 *
 * **Der Deckel bei 100 % ist keine Balance-Entscheidung, sondern eine
 * Rechenregel:** Eine Wahrscheinlichkeit über eins gibt es nicht, und
 * `RandomStream.chance` würde sie ohnehin als sicheren Treffer behandeln. 6.7
 * hält denselben Deckel fest — „jeder Treffer kritet; darüber hinaus
 * wirkungslos".
 */
export function critChance(worn: WornValues): number {
  if (worn.purity >= 1000) return 1;
  return worn.purity / 1000;
}

/**
 * Der Faktor, mit dem ein kritischer Treffer multipliziert wird.
 *
 * Basis 150 %, plus YIELD. **Ohne YIELD ist es genau 1,5** — und weil 1,5 in
 * IEEE754 exakt darstellbar ist, ist der Wert auf beiden Seiten bitgleich, ohne
 * dass es einer Zusicherung bedarf.
 */
export function critFactor(worn: WornValues): number {
  return (CRIT_BASE_PERMILLE + worn.yieldBonus) / 1000;
}
