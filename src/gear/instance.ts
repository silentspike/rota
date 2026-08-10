/**
 * Stufe und Qualität EINES FUNDES, nicht seiner Vorlage (Issue #480).
 *
 * # Warum es diese Datei gibt
 *
 * `rollLoot` würfelt beides pro Fund und legt es in `Drop` ab — der Kommentar
 * dort sagt es wörtlich: „Aus `itemLevelOf`, nicht aus dem Katalog — der
 * Eintrag ist die Vorlage." Verbucht wurde davon nur die Kennung. Jeder Fund
 * rechnete deshalb mit den Zahlen der Vorlage, und zwei Stücke derselben
 * Kennung waren **exakt** gleich stark, egal woher sie kamen:
 *
 * ```
 * catalysis 407, purity 406, yieldBonus 406, essence 67, vessel 67, ward 67
 * catalysis 407, purity 406, yieldBonus 406, essence 67, vessel 67, ward 67
 * ```
 *
 * Damit war die Fortschritts-Achse der Ausrüstung wirkungslos — und 13.4 baut
 * ausdrücklich darauf auf, dass „alter Inhalt sich von selbst entwertet".
 *
 * # Warum die Auflösung EINE benannte Stelle ist
 *
 * Weil die Felder fehlen dürfen (alte Spielstände, siehe `OwnedItem`), muss
 * jede Lesestelle sonst selbst entscheiden, was ein fehlender Wert bedeutet.
 * Drei Lesestellen, drei Antworten — und die Abweichung fällt niemandem auf,
 * weil alle drei plausible Zahlen liefern. Hier wird die Frage einmal
 * beantwortet.
 *
 * **Der Rückfall ist die Vorlage, nicht ein Festwert.** Eine 1 wäre die
 * naheliegende Wahl und die falsche: Ein alter Spielstand verlöre beim ersten
 * Laden den grössten Teil seiner Werte (bei Stufe 40 den Faktor 6,7), ohne dass
 * der Spieler etwas getan hat. Mit der Vorlage rechnet ein alter Stand nach
 * dem Wechsel **genau wie vorher** — die neue Achse greift erst für das, was
 * danach gefunden wird. Ein Fassungswechsel darf Fortschritt hinzufügen; er
 * darf keinen wegnehmen.
 *
 * Gegenstück im Simulator: `sim/src/gear.rs`.
 */

import { itemById } from './catalog.js';

/** Was aufgelöst werden kann — die Felder aus `OwnedItem`, ohne den Rest. */
export interface InstanceLike {
  readonly itemId: string;
  readonly itemLevel?: number;
  readonly quality?: number;
}

/**
 * Ob ein gespeicherter Wert überhaupt als Zahl taugt.
 *
 * Ein `NaN` oder ein Bruch käme aus einem von Hand bearbeiteten Spielstand und
 * ginge sonst in `basePermille` ein, das ihn stillschweigend auf 0 abbildet —
 * ein Stück ohne Werte, das aussieht wie ein Stück mit Werten.
 */
function istGanzzahl(wert: unknown): wert is number {
  return typeof wert === 'number' && Number.isInteger(wert);
}

/**
 * Die Stufe dieses Fundes, oder die seiner Vorlage.
 *
 * Gibt 0 zurück, wenn die Kennung nicht im Katalog steht — dieselbe Antwort,
 * die `wornFrom` heute schon durch Überspringen gibt. `basePermille` weist eine
 * 0 ohnehin ab, statt mit ihr zu rechnen.
 */
export function instanceLevelOf(item: InstanceLike): number {
  if (istGanzzahl(item.itemLevel)) return item.itemLevel;
  return itemById(item.itemId)?.itemLevel ?? 0;
}

/** Die Qualität dieses Fundes, oder die seiner Vorlage. Sonst wie oben. */
export function instanceQualityOf(item: InstanceLike): number {
  if (istGanzzahl(item.quality)) return item.quality;
  return itemById(item.itemId)?.quality ?? 0;
}
