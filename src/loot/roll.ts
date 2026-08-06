/**
 * Was ein Sieg fallen lässt (Issue #447, GAME-DESIGN 13.3).
 *
 * WAS BIS HIERHER FEHLTE
 * ----------------------
 * `src/loot/` hatte zehn exportierte Funktionen und **keine**, die einen
 * Gegenstand zusammensetzt: `weightsFor` liefert die Qualitäts-Gewichte,
 * `qualityOfRoll` den Wurf, `itemLevelOf` die Stufe, `mayDropAt` die Bindung.
 * Alle Bausteine lagen da, keiner rief den anderen.
 *
 * Dieselbe Lage wie bei `wornFrom` in #421a: Es fehlte nicht die Formel,
 * sondern das Zusammensetzen.
 *
 * DER BEUTE-STROM IST EIN EIGENER, UND DAS IST DER KERN
 * ----------------------------------------------------
 * `rng.ts` begründet es ausführlich: „Beute wird beim **Sieg** gewürfelt, nicht
 * im Kampf — sie kann nichts verschieben, und nichts kann sie verschieben."
 *
 * Ein Strom, der im Kampf nie zieht, hängte sonst über 7952 Takte 7952 Nullen
 * an die Zustands-Prüfsumme. Deshalb liegt er in `stream.ts` und nicht in
 * `RandomStreams` — `loot-stream.test.ts` bewacht das seit E6-2.
 *
 * **Diese Datei zieht deshalb ausschliesslich aus dem übergebenen Strom.** Wer
 * hier einen Kampfstrom hineinreicht, bricht die Bitgleichheit — still, weil
 * es nur bei bestimmten Zügen auffällt.
 *
 * DIE FALLRATE IST NICHT ENTSCHIEDEN
 * ----------------------------------
 * Keine der zehn Funktionen sagt, **ob** etwas fällt — nur, was. Die Häufigkeit
 * bestimmt, wie sich ein Sieg anfühlt, und gehört damit dem Auftraggeber. Sie
 * steht als `dropChancePerMille` in `data/items.json`, wie der Exponent in
 * `reward.json`: austauschbar ohne Umbau.
 */
import type { RandomStream } from '../core/rng.js';
import { ITEMS, type ItemDef } from '../gear/catalog.js';
import { DROP_CHANCE_PER_MILLE } from '../gear/catalog.js';
import { QUALITY_COUNT } from '../gear/budget.js';
import { itemLevelOf, mayDropAt, qualityOfRoll, weightsFor } from './rates.js';

/** Was ein Sieg über den Fund weiss, ohne den Kampf zu kennen. */
export interface DropContext {
  /** Instanz 1 bis 14 (`rates.ts`). */
  readonly instance: number;
  /** 0 normal, 1 heroisch, 2 mythisch. */
  readonly difficulty: number;
  /** Stufe im Endlos-Modus, 0 ausserhalb. */
  readonly endlessLevel: number;
  /** Welcher Boss gefallen ist — für bossgebundene Stücke. */
  readonly bossId: string;
}

/** Ein Fund: welcher Eintrag, auf welcher Stufe, in welcher Qualität. */
export interface Drop {
  readonly itemId: string;
  /** Aus `itemLevelOf`, nicht aus dem Katalog — der Eintrag ist die Vorlage. */
  readonly itemLevel: number;
  readonly quality: number;
}

/**
 * Welche Einträge hier überhaupt fallen dürfen.
 *
 * **Bossgebundene Stücke nur beim richtigen Boss** (`mayDropAt`). Heute trägt
 * kein Eintrag eine Bindung; die Prüfung steht trotzdem, weil sie sonst beim
 * ersten gebundenen Stück vergessen würde — und dann fiele es überall.
 */
export function candidatesFor(bossId: string, katalog: readonly ItemDef[] = ITEMS): ItemDef[] {
  return katalog.filter((item) => mayDropAt(item.boundToBoss, bossId));
}

/**
 * Was dieser Sieg fallen lässt, oder `null`.
 *
 * **Die Reihenfolge der Züge ist Teil der Zusicherung.** Sie steht hier fest,
 * damit derselbe Startwert dieselbe Beute ergibt — daran hängt, dass eine
 * aufgezeichnete Runde ihre Beute exakt wiederholt (`stream.ts`).
 *
 *   1. Fällt überhaupt etwas?
 *   2. Welche Qualität?
 *   3. Welcher Eintrag aus den zulässigen?
 *
 * **Ein Wurf, der nichts ergibt, zieht trotzdem einmal.** Wer die erste Ziehung
 * überspränge, wenn der Katalog leer ist, bekäme bei gleichem Startwert eine
 * andere Folge, sobald ein Eintrag dazukommt.
 */
export function rollLoot(
  stream: RandomStream,
  context: DropContext,
  katalog: readonly ItemDef[] = ITEMS,
): Drop | null {
  const faellt = Math.floor(stream.next() * 1000) < DROP_CHANCE_PER_MILLE;
  const gewichte = new Int32Array(QUALITY_COUNT);
  weightsFor(context.instance, context.difficulty, context.endlessLevel, gewichte);
  const quality = qualityOfRoll(gewichte, stream.next());
  const auswahl = candidatesFor(context.bossId, katalog);
  const gewaehlt = auswahl.length === 0 ? -1 : Math.floor(stream.next() * auswahl.length);

  if (!faellt || auswahl.length === 0) return null;
  const item = auswahl[Math.min(gewaehlt, auswahl.length - 1)];
  if (item === undefined) return null;

  return {
    itemId: item.id,
    // **Die Stufe kommt aus dem Fundort, nicht aus dem Eintrag** (13.3): Ein
    // Katalog-Eintrag ist die Vorlage, seine Stufe der Vorgabewert. Was in
    // Instanz 9 auf mythisch fällt, ist stärker als dasselbe Stück in Instanz 1.
    itemLevel: itemLevelOf(context.instance, context.difficulty, context.endlessLevel),
    quality,
  };
}
