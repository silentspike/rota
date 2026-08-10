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

/**
 * Was der Spieler trägt: die drei Grundwerte aus 6.1, die drei Sonderwerte aus
 * 6.2 und Topf 4 aus 13.1b.
 */
export interface WornValues {
  /** CATALYSIS in Promille. */
  readonly catalysis: number;
  /** PURITY in Promille. */
  readonly purity: number;
  /** YIELD in Promille, **zusätzlich** zur Basis aus `CRIT_BASE_PERMILLE`. */
  readonly yieldBonus: number;
  /**
   * ESSENCE in Promille — **Topf 1**, nicht Topf 4 (Issue #282).
   *
   * Die Leitung nach Topf 4 (`gearEffects`) liegt schon, und sie zu benutzen
   * wäre eine gesparte Zeile. Sie wäre auch falsch: GAME-DESIGN 13.1b trennt
   * die Töpfe ausdrücklich — innerhalb additiv, **zwischen** ihnen
   * multiplikativ. Wer ESSENCE dort einhängt, macht aus zwei Töpfen einen.
   */
  readonly essence: number;
  /**
   * VESSEL in Promille — Lebenspunkte, **zusätzlich** zur Stufen-Basis.
   *
   * Additiv, nicht ersetzend: Wer die Stufen-Basis ersetzt, macht einen
   * Spieler auf Stufe 100 ohne Ausrüstung schwächer als einen auf Stufe 1 mit
   * einem Fund.
   */
  readonly vessel: number;
  /** WARD in Promille — **zusätzlich** zu `PLAYER.wardReduction`. */
  readonly ward: number;
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

/** Nichts getragen — alle sieben Werte null. */
export const NOTHING_WORN: WornValues = {
  catalysis: 0,
  purity: 0,
  yieldBonus: 0,
  essence: 0,
  vessel: 0,
  ward: 0,
  gearEffects: 0,
};

/**
 * Welches Feld eine Affix-Art füllt — oder `null`, wenn es keines gibt.
 *
 * # Warum diese Funktion existiert
 *
 * `POOL_NAMES` (`pools.ts`) führt die **Töpfe** aus GAME-DESIGN 13.1b, und
 * `WornValues` führt die **Werte**, die der Kampf liest. Das ist fast, aber
 * nicht ganz dasselbe: `active` ist ein Topf ohne Wert, weil aktive
 * Verstärkungen im Kampf entstehen (Overdrive, Synergien, Zersetzungs-Stapel)
 * und nicht auf einem Gegenstand stehen.
 *
 * Bis #421b hat `wornFrom` die Art **direkt** als Schlüssel benutzt. Ein
 * Eintrag mit `active` lud dann fehlerfrei, bestand jede Prüfung — und legte
 * seinen Anteil in ein Feld, das niemand liest. Gemessen am 2026-08-10 an
 * einem Epic der Stufe 40:
 *
 * ```text
 * {… "gearEffects":407, "active":406}
 *                        ^^^^^^^^^^^^ kennt der Kampf nicht
 * ```
 *
 * **406 von 1219 Promille verfielen, ohne dass irgendwo etwas auffiel.** Der
 * Gegenstand sah vollwertig aus und war ein Drittel schwächer.
 *
 * # Die Abbildung steht hier und nicht in `catalog.ts`
 *
 * Sie wird an zwei Stellen gebraucht — beim Prüfen des Katalogs und beim
 * Rechnen der Werte. Stünde sie zweimal, könnte die eine Stelle eine Art
 * erlauben, welche die andere nicht kennt; genau diese Lücke hat den stillen
 * Verlust erzeugt.
 *
 * `yield` heißt im Wertesatz `yieldBonus`, weil `yield` in JavaScript ein
 * reserviertes Wort ist.
 */
export function feldFuerAffixArt(kind: string): keyof WornValues | null {
  const feld = kind === 'yield' ? 'yieldBonus' : kind;
  return feld in NOTHING_WORN ? (feld as keyof WornValues) : null;
}

/** Ob überhaupt etwas getragen wird. */
export function isNothingWorn(worn: WornValues): boolean {
  return (
    worn.catalysis === 0 &&
    worn.purity === 0 &&
    worn.yieldBonus === 0 &&
    worn.essence === 0 &&
    worn.vessel === 0 &&
    worn.ward === 0 &&
    worn.gearEffects === 0
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
 * Der Faktor von **Topf 1** aus ESSENCE (Issue #282).
 *
 * Dieselbe Regel wie `gearEffectFactor` — eins plus die Summe — aber ein
 * **eigener** Faktor. Zwischen den Töpfen wird multipliziert (13.1b); wären
 * beide derselbe, wäre die Rechnung additiv und die Abwägung aus 13.5
 * gekippt.
 *
 * Ohne getragene Ausrüstung ist es exakt 1,0, und 1,0 ist in IEEE754 die
 * exakte Eins: Die Multiplikation bewegt dann keine einzige Zahl.
 */
export function essenceFactor(worn: WornValues): number {
  return 1 + worn.essence / 1000;
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

/**
 * Die Lebenspunkte aus Stufe **und** Ausrüstung (Issue #282, AC3).
 *
 * # Warum eine eigene Funktion und keine Zeile im Konstruktor
 *
 * Beide Seiten müssen dieselbe Rundung treffen. Steht die Rechnung an einer
 * Stelle, ist sie eine Zeile zum Vergleichen; steht sie zweimal im Aufbau,
 * muss man sie suchen.
 *
 * **Ganzzahlig gerundet, nicht in Fließkomma gehalten:** Lebenspunkte sind
 * ganze Zahlen, und ein Rest, der erst später abgeschnitten wird, ist ein
 * Rest, den die andere Seite anders abschneiden könnte.
 */
export function vesselWithGear(base: number, wornPermille: number): number {
  if (wornPermille === 0) return base;
  return Math.trunc((base * (1000 + wornPermille)) / 1000);
}
