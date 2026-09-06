/**
 * Die Schadensformel (GAME-DESIGN 6.3).
 *
 * ```
 * Schaden = Grundschaden
 *         × ESSENCE-Faktor
 *         × Talent-Faktoren
 *         × Ausrüstungs-Sondereffekte
 *         × aktive Verstärkungen
 *         × Synergie-Faktoren
 *         × Widerstandsfaktor des Bosses
 *         × Krit
 * ```
 *
 * **Die Reihenfolge der Multiplikationen ist festgelegt und für Spiel und Simulator
 * identisch.** Fließkomma-Multiplikation ist nicht assoziativ: `(a × b) × c` und
 * `a × (b × c)` können sich im letzten Bit unterscheiden. Bei zehntausenden Treffern
 * je Balance-Lauf summiert sich das zu einer Abweichung, die niemand mehr zuordnen
 * kann. Deshalb steht die Reihenfolge hier an genau einer Stelle.
 *
 * In Schritt 1 sind alle Faktoren 1,0 und Krit ruht (STEP1-SPEC § 2) — die Struktur
 * steht trotzdem schon, weil sie nachträglich einzuziehen jede aufgezeichnete Runde
 * ungültig machen würde.
 */

/** Die Faktoren in der Reihenfolge, in der sie angewandt werden. */
export interface DamageFactors {
  /** Aus dem Wert ESSENCE (GAME-DESIGN 6.1). */
  readonly essence: number;
  /** Aus dem Talentbaum. */
  readonly talents: number;
  /**
   * Legendäre Effekte **und** Satz-Boni — Topf 4 aus GAME-DESIGN 13.1b.
   *
   * **Warum an dieser Stelle und nicht anderswo:** Keiner der bestehenden
   * Faktoren konnte ihn aufnehmen. `essence` machte ihn additiv mit ESSENCE;
   * `talents` additiv mit den Talenten — und 13.1 trennt beide Systeme
   * ausdrücklich („WIE die Rotation funktioniert" gegen „WIE STARK die Werte
   * sind"). `buffs` **ist** Topf 3, und zwischen Töpfen wird multipliziert.
   * Es blieb nur ein eigener Faktor.
   *
   * Die Position gruppiert die Formel: erst die Spieler-Faktoren (ESSENCE,
   * Talente, Ausrüstung), dann der Zustand, dann der Boss, zuletzt Krit.
   *
   * **Die Position ist eine Festlegung des Design-Dokuments, keine
   * Geschmacksfrage** — Fließkomma-Multiplikation ist nicht assoziativ, eine
   * andere Reihenfolge ergibt andere Bits und damit eine andere Prüfsumme
   * (6.3, 18.4).
   *
   * Heute steht er bei 1,0 und bewegt keine einzige Zahl. **Genau deshalb war
   * Vorsicht geboten:** Eine Änderung, die nichts bewegt, geht sonst unbemerkt
   * durch.
   */
  readonly gearEffects: number;
  /** Laufende Verstärkungen auf dem Spieler. */
  readonly buffs: number;
  /** Offene Synergie-Fenster (GAME-DESIGN 5). */
  readonly synergy: number;
  /** Widerstand des Bosses gegen die Schule (GAME-DESIGN 11.4). */
  readonly resistance: number;
  /**
   * Verwundbarkeit des Bosses — der siebte Faktor (GAME-DESIGN 6.3).
   *
   * Trägt alles, was den Boss für Schaden **aus jeder Quelle** empfänglicher
   * macht; heute sind das die Zersetzungs-Stapel der Putrefaktion (4.1).
   *
   * **Die Position ist eine Festlegung, keine Geschmacksfrage:**
   * `… × resistance × vulnerability × crit`. Fließkomma-Multiplikation ist
   * nicht assoziativ — eine andere Reihenfolge ergibt andere Bits und damit
   * eine andere Zustands-Prüfsumme. Sie steht direkt neben dem Widerstand,
   * weil beide Aussagen über den Boss sind und nicht über den Spieler.
   */
  readonly vulnerability: number;
  /** 1,0 ohne Krit, sonst der YIELD-Faktor. */
  readonly crit: number;
}

/** Alle Faktoren neutral — der Zustand auf Stufe 1 ohne Ausrüstung. */
export const NEUTRAL_FACTORS: DamageFactors = {
  essence: 1,
  talents: 1,
  gearEffects: 1,
  buffs: 1,
  synergy: 1,
  resistance: 1,
  vulnerability: 1,
  crit: 1,
};

/**
 * Gemeinsame Rundung mit `factors::round_js` im Simulator (#962).
 * Nächste ganze Zahl; bei gleichem Abstand Richtung +∞, also -1,5 → -1.
 * Negative Null bleibt erhalten. Diese Regel betrifft die Rundung, nicht den
 * erlaubten Wertebereich einer Schadensmenge; siehe docs/RUNDUNGSVERTRAG.md.
 */
export function roundLikeJavaScript(value: number): number {
  return Math.round(value);
}

/**
 * Schaden eines einzelnen Treffers.
 *
 * Gerundet wird **einmal am Ende**, nicht zwischendurch: Zwischenrundungen wären
 * eine zweite Stelle, an der Spiel und Simulator auseinanderlaufen könnten. Der
 * Rundungspunkt gehört damit zur gemeinsamen Schnittstelle.
 */
export function computeDamage(baseDamage: number, factors: DamageFactors): number {
  let value = baseDamage;
  value *= factors.essence;
  value *= factors.talents;
  value *= factors.gearEffects;
  value *= factors.buffs;
  value *= factors.synergy;
  value *= factors.resistance;
  value *= factors.vulnerability;
  value *= factors.crit;
  return roundLikeJavaScript(value);
}

/**
 * Schaden eines exponentiellen Stapels (GAME-DESIGN 11.2, Baustein 3; #216).
 *
 * > 1 Stapel = X, 2 = 3X, 3 = 9X.
 *
 * Also `baseDamage * factor^(stacks-1)` — bei einem Stapel steht X, nicht 3X.
 * Der Exponent ist um eins versetzt, und das ist keine Feinheit: Mit
 * `factor^stacks` begänne die Reihe bei 3X und der erste Stapel wäre schon so
 * teuer wie der zweite sein soll.
 *
 * **Ganzzahlig durchgerechnet**, in einer Schleife statt über `Math.pow`.
 * `Math.pow` liefert eine Fließkommazahl, und Rust rechnet daneben mit
 * `saturating_mul` über ganze Zahlen. Zwei Seiten, die einen großen Wert
 * verschieden runden oder verschieden überlaufen lassen, sind keine Parität
 * mehr (GAME-DESIGN 18.4).
 */
export function stackDamage(
  def: { readonly baseDamage: number; readonly factor: number },
  stacks: number,
): number {
  if (stacks <= 0) return 0;
  let value = def.baseDamage;
  for (let i = 1; i < stacks; i++) value *= def.factor;
  return value;
}
