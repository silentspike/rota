/**
 * Die vier Töpfe (GAME-DESIGN 13.1b, Issue E6-1).
 *
 * **Das Prinzip in einem Satz: Innerhalb eines Topfes wird addiert, zwischen
 * den Töpfen multipliziert.**
 *
 * | Topf                        | Inhalt                                   | Verrechnung                    |
 * | --------------------------- | ---------------------------------------- | ------------------------------ |
 * | ESSENCE                     | alle Quellen des Grundwerts              | additiv, dann **ein** Faktor   |
 * | Sonderwerte                 | CATALYSIS, PURITY, YIELD                 | **je eigener** Faktor          |
 * | Aktive Verstärkungen        | Overdrive, Synergien, Zersetzungs-Stapel | additiv untereinander          |
 * | Ausrüstungs-Sondereffekte   | legendäre Effekte **und** Satz-Boni      | additiv untereinander          |
 *
 * # Warum vier Töpfe, aber sechs Faktoren
 *
 * Das ist keine abweichende Struktur, sondern die wörtliche Lesart der Zeile
 * „Sonderwerte — **je eigener** Faktor". Zwei Quellen desselben Sonderwerts
 * addieren sich; CATALYSIS und PURITY multiplizieren sich gegeneinander. Ein
 * gemeinsamer Sonderwert-Faktor wäre das Gegenteil dessen, was die Zeile sagt,
 * und würde die Absicht dahinter zerstören:
 *
 * > „Weil sie sich multiplizieren, will man alle drei mischen statt einen zu
 * > stapeln. Das ist der eigentliche Designwert — nicht ‚mehr Zahl', sondern
 * > **keine Monokultur**." (13.1b)
 *
 * Die drei Sonderwerte sind also drei Kammern eines Topfes, und jede Kammer
 * ist nach außen ein eigener Faktor. Gezählt wird hier deshalb in Faktoren.
 *
 * # Warum legendäre Effekte und Satz-Boni denselben Topf teilen
 *
 * Das ist die wichtigste Zeile der ganzen Tabelle, und sie ist aus Schaden
 * gelernt, nicht aus Geschmack:
 *
 * > In Diablo IV waren Kritschaden und Verwundbarkeit eigene multiplikative
 * > Töpfe. Sie wurden „disproportional stark", jeder Aufbau lief über sie, und
 * > Blizzard verschob sie in Saison 2 zurück in den additiven Topf. Diablo 3
 * > verlor dieselbe Schlacht bei den Sätzen: Die Sechser-Boni wurden über
 * > Jahre hochgezogen, bis legendäre Gegenstände **ohne** Satz-Bonus schlicht
 * > ignoriert wurden — von Spielern wie von Entwicklern.
 *
 * **Was multiplikativ ist, wird gejagt, und alles andere wird liegen
 * gelassen.** Ein Sechser-Satz mit 30 % und ein legendärer Effekt mit 20 %
 * ergeben deshalb 50 %, nicht 56 %.
 *
 * Der Unterschied wirkt klein und ist es nicht: Bei fünf solchen Quellen zu je
 * 20 % stehen 200 % gegen 249 %, bei zehn 300 % gegen 519 %. Multiplikativ
 * wächst der Vorteil mit jeder weiteren Quelle — und genau das macht aus einer
 * Wahl eine Pflicht.
 *
 * # Was hier NICHT steht
 *
 * **Wie das Ergebnis in die Schadensformel eintritt.** 13.1b legt fest, wie
 * die Töpfe untereinander rechnen, nicht an welcher Stelle von 6.3 der
 * Ausrüstungs-Topf multipliziert wird. Die Reihenfolge der Multiplikationen
 * ist laut 6.3 ausdrücklich eine Festlegung des Design-Dokuments und **darf
 * nicht im Code entschieden werden** — deshalb endet dieses Modul beim
 * Faktor und hängt ihn nirgends ein.
 *
 * **Die Zahlen.** Wie groß ein Beitrag ist, misst der Balance-Läufer (#118).
 * Hier steht nur, wie gerechnet wird.
 */

/** Alle ESSENCE-Quellen. */
export const POOL_ESSENCE = 0;
/** CATALYSIS — erste Kammer des Sonderwert-Topfes. */
export const POOL_CATALYSIS = 1;
/** PURITY — zweite Kammer. */
export const POOL_PURITY = 2;
/** YIELD — dritte Kammer. */
export const POOL_YIELD = 3;
/** Aktive Verstärkungen: Overdrive, Synergien, Zersetzungs-Stapel. */
export const POOL_ACTIVE = 4;
/** Legendäre Effekte **und** Satz-Boni — gemeinsam. */
export const POOL_GEAR_EFFECTS = 5;

/** Wie viele Faktoren die vier Töpfe nach außen ergeben. */
export const POOL_COUNT = 6;

/**
 * Die Namen in der Reihenfolge der Kennnummern.
 *
 * Sie sind Teil der gemeinsamen Schnittstelle: Der Simulator führt dieselbe
 * Liste, und `tests/parity/recordings/gear.json` hält sie fest.
 */
export const POOL_NAMES: readonly string[] = [
  'essence',
  'catalysis',
  'purity',
  'yield',
  'active',
  'gearEffects',
];

/**
 * Welche Töpfe **einen** Faktor ergeben, obwohl sie mehrere Kammern haben.
 *
 * Nur zur Erklärung in der Oberfläche und im Prüflauf — gerechnet wird über
 * die sechs Kennnummern.
 */
export const POOLS_OF_SPECIAL_STATS: readonly number[] = [POOL_CATALYSIS, POOL_PURITY, POOL_YIELD];

/**
 * Die Beiträge zu den Töpfen.
 *
 * Ein Beitrag ist ein **Anteil**, keine Punktzahl: 0,3 heißt „+30 %". Die
 * Umrechnung von Wert-Punkten in Anteile gehört nicht hierher, sondern zu den
 * Gegenständen — sonst stünden vorläufige Zahlen mitten in einer Regel, die
 * nicht vorläufig ist.
 *
 * Vorab angelegt, damit im Takt nichts entsteht (GAME-DESIGN 18.2). Ein
 * Ausrüstungswechsel ist zwar kein Bild-Pfad, aber die Rechnung läuft auch im
 * Balance-Läufer über zehntausende Zusammenstellungen.
 */
export class ValuePools {
  private readonly sums = new Float64Array(POOL_COUNT);

  /**
   * Einen Beitrag hinzufügen.
   *
   * Gibt `false` zurück, wenn es den Topf nicht gibt — still zu verwerfen
   * wäre der Fehler, den man erst drei Wochen später am falschen Schaden
   * bemerkt.
   */
  add(pool: number, share: number): boolean {
    if (!Number.isInteger(pool) || pool < 0 || pool >= POOL_COUNT) return false;
    this.sums[pool] = (this.sums[pool] ?? 0) + share;
    return true;
  }

  /** Die Summe der Beiträge eines Topfes — ohne die Eins. */
  sum(pool: number): number {
    return this.sums[pool] ?? 0;
  }

  /**
   * Der Faktor eines Topfes: **eins plus** die Summe seiner Beiträge.
   *
   * Die Eins steht hier und nicht bei den Beiträgen, weil sonst jeder
   * zusätzliche Beitrag den Faktor um eine ganze Eins anheben würde — aus
   * zwei Quellen zu je 20 % würden 2,4 statt 1,4.
   */
  factor(pool: number): number {
    return 1 + (this.sums[pool] ?? 0);
  }

  /**
   * Alle sechs Faktoren miteinander multipliziert.
   *
   * **Die Reihenfolge ist die der Kennnummern und liegt fest.**
   * Fließkomma-Multiplikation ist nicht assoziativ: `(a × b) × c` und
   * `a × (b × c)` können sich im letzten Bit unterscheiden. Über zehntausende
   * Bewertungen im Balance-Läufer summiert sich das zu einer Abweichung, die
   * niemand mehr zuordnen kann — dieselbe Begründung wie bei der
   * Schadensformel (6.3, 18.4).
   */
  product(): number {
    let value = 1;
    for (let pool = 0; pool < POOL_COUNT; pool++) {
      value *= this.factor(pool);
    }
    return value;
  }

  /** Alles auf null — bei einem Ausrüstungswechsel. */
  clear(): void {
    this.sums.fill(0);
  }
}
