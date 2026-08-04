/**
 * Affix-Budget und Qualitäten (GAME-DESIGN 13.3, Issue E6-1).
 *
 * > „**Kein Gegenstand wird von Hand balanciert** — ein neuer Gegenstand ist
 * > ein Eintrag mit Stufe, Qualität und Werte-Arten." (13.3)
 *
 * Das ist der ganze Zweck dieses Moduls: Aus **Stufe** und **Qualität** fällt
 * ein Budget, und das Budget verteilt sich auf die Werte-Plätze. Niemand trägt
 * je eine Wertzahl ein.
 *
 * | Qualität  | Werte-Plätze | Besonderheit                              |
 * | --------- | ------------ | ----------------------------------------- |
 * | Common    | 1            | —                                         |
 * | Rare      | 2            | —                                         |
 * | Epic      | 3            | —                                         |
 * | Legendary | 3            | **echter Sondereffekt** (13.3)            |
 *
 * # Die Zahlen sind ausdrücklich vorläufig
 *
 * 13.1b sagt es selbst: *„Die Zahlen stehen ausdrücklich noch nicht fest. Sie
 * werden nicht festgelegt, sondern **gemessen**."* Der Balance-Läufer
 * entscheidet sie in #118. Was hier steht, trägt die Prüffälle und sonst
 * nichts — die **Regel** ist endgültig, die **Zahl** nicht.
 *
 * # Warum durchweg ganzzahlig gerechnet wird
 *
 * Ein `Math.pow(1.05, stufe)` wäre der naheliegende Weg und wäre falsch: Spiel
 * und Simulator müssen bitgleich rechnen (18.4), und Potenzfunktionen sind
 * zwischen zwei Sprachen nicht bitgleich garantiert. Steht das Ergebnis dann
 * zufällig auf `.5`, kippt die Rundung auf einer Seite und auf der anderen
 * nicht — ein Fehler, der bei Stufe 47 auftritt und bei 46 und 48 nicht.
 *
 * Deshalb wächst das Budget als **ganzzahlige Rekursion**: `b → ⌊b · 21/20⌋`.
 * Das ist dieselbe Rechnung in jeder Sprache, die Ganzzahlen kennt, und
 * braucht keine Annahme über Fließkomma.
 */

/** Die Qualitäten in fester Reihenfolge — die Kennnummer ist Teil der Schnittstelle. */
export const QUALITY_COMMON = 0;
export const QUALITY_RARE = 1;
export const QUALITY_EPIC = 2;
export const QUALITY_LEGENDARY = 3;

export const QUALITY_COUNT = 4;

export const QUALITY_NAMES: readonly string[] = ['common', 'rare', 'epic', 'legendary'];

/**
 * Wie viele **Affixe** eine Qualität trägt (13.3).
 *
 * **Affixe, nicht Werte überhaupt.** Die Spalte heißt im Design
 * „Werte-Plätze", und genau daran ist eine Rechnung gescheitert: Common
 * bekommt einen davon, aber jeder Gegenstand trägt drei Grundwerte
 * (ESSENCE, VESSEL, WARD) — ein Platz kann keine drei tragen. Am 2026-07-29
 * geklärt: **Die Grundwerte kommen zusätzlich**, wie die Rüstung an einem
 * Diablo-Gegenstand. Die Zahl hier zählt nur, was obendrauf gewürfelt wird.
 *
 * Der Name `QUALITY_AFFIX_SLOTS` steht deshalb bewusst nicht so da wie im
 * Design — die dortige Benennung hat die Verwechslung überhaupt erst erzeugt.
 *
 * **Legendary steht in der Tabelle als „3+".** Das Pluszeichen ist in
 * derselben Zeile erklärt — „echter Sondereffekt, der die Rotation verändert".
 * Gelesen als: drei Affixe **und zusätzlich** ein Effekt, nicht als „drei oder
 * mehr Affixe". Die andere Lesart wäre ein vierter Affix und damit eine
 * stillschweigende Balance-Entscheidung.
 */
export const QUALITY_AFFIX_SLOTS: readonly number[] = [1, 2, 3, 3];

/** Welche Qualität einen Sondereffekt trägt (13.3). */
export const QUALITY_HAS_EFFECT: readonly boolean[] = [false, false, false, true];

/**
 * Das Budget einer Qualität auf Gegenstandsstufe 1. **Vorläufig (#118).**
 *
 * Muss mindestens 20 sein, sonst steht die Wachstums-Rekursion still:
 * `⌊b · 21/20⌋` ist für `b < 20` wieder `b`.
 */
const QUALITY_BASE_BUDGET: readonly number[] = [100, 140, 190, 190];

/** Zähler und Nenner des Stufen-Wachstums — 21/20 sind +5 % (6.4). **Vorläufig.** */
const GROWTH_NUMERATOR = 21;
const GROWTH_DENOMINATOR = 20;

/** Höchste Gegenstandsstufe, die gerechnet wird (6.4: 100 Spielerstufen). */
export const MAX_ITEM_LEVEL = 100;

/**
 * Das Werte-Budget eines Gegenstands.
 *
 * Gibt 0 zurück, wenn Stufe oder Qualität außerhalb liegen — ein Gegenstand
 * ohne Budget fällt in jedem Prüffall auf, ein stillschweigend geklemmter Wert
 * nicht.
 */
export function budgetOf(itemLevel: number, quality: number): number {
  if (!Number.isInteger(quality) || quality < 0 || quality >= QUALITY_COUNT) return 0;
  if (!Number.isInteger(itemLevel) || itemLevel < 1 || itemLevel > MAX_ITEM_LEVEL) return 0;
  let budget = QUALITY_BASE_BUDGET[quality] ?? 0;
  for (let level = 1; level < itemLevel; level++) {
    budget = Math.floor((budget * GROWTH_NUMERATOR) / GROWTH_DENOMINATOR);
  }
  return budget;
}

/**
 * Was der Werte-Platz `index` von `budget` bekommt — gleichmäßig verteilt,
 * Rest auf die vorderen Plätze.
 *
 * `⌊(budget + plätze − 1 − index) / plätze⌋` ist dieselbe Funktion wie das
 * naheliegende `⌊budget/plätze⌋ + (index < budget mod plätze ? 1 : 0)`. Mit
 * `budget = q·plätze + r` wird der Ausdruck zu `q + (index < r ? 1 : 0)` — die
 * geschlossene Form spart nur die Fallunterscheidung. **Über 720 036
 * Wertepaare gemessen: null Abweichung.** Der Hinweis steht hier, weil beim
 * Lesen leicht der Eindruck entsteht, die geschlossene Form leiste etwas
 * anderes. Sie tut es nicht.
 *
 * # Woran AC1 tatsächlich hängt
 *
 * **Nicht an dieser Formel, sondern am Wachstum.** Bei einem Budget-Zuwachs
 * von 1 kann gar keine Verteilung alle Plätze anheben — die Summe stiege
 * sonst um die Zahl der Plätze. „Durchweg höhere Werte" gilt genau dann, wenn
 * das Budget je Stufe um **mindestens so viel wächst wie es Plätze gibt**.
 *
 * Bei den jetzigen Zahlen ist der knappste Fall Common auf Stufe 1: Zuwachs 5
 * bei 1 Platz, also vier Schritte Luft. Die Zahlen sind aber vorläufig, und
 * ein flacheres Wachstum oder ein kleineres Grundbudget würde AC1 kippen —
 * ohne dass jemand die Verteilung angefasst hätte. Genau diese Bedingung
 * prüft `tests/unit/gear-budget.test.ts` deshalb ausdrücklich.
 */
export function shareOfSlot(budget: number, slots: number, index: number): number {
  if (slots <= 0 || index < 0 || index >= slots) return 0;
  return Math.floor((budget + slots - 1 - index) / slots);
}

/**
 * Das Budget auf die Affixe einer Qualität verteilen.
 *
 * Schreibt in ein übergebenes Feld, statt eines anzulegen (18.2). Gibt die
 * Zahl der beschriebenen Plätze zurück.
 */
export function distributeBudget(
  itemLevel: number,
  quality: number,
  out: Int32Array | number[],
): number {
  const slots = QUALITY_AFFIX_SLOTS[quality] ?? 0;
  const budget = budgetOf(itemLevel, quality);
  const count = Math.min(slots, out.length);
  for (let index = 0; index < count; index++) {
    out[index] = shareOfSlot(budget, slots, index);
  }
  return count;
}
