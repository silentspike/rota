/**
 * Stufe und Qualität EINES STÜCKS, nicht seiner Vorlage (Issue #480).
 *
 * # Warum es diese Datei gibt
 *
 * Zwei Achsen wurden erhoben und nie gelesen:
 *
 * - **Die Fundstufe.** `rollLoot` würfelt sie pro Fund und legt sie in `Drop`
 *   ab — der Kommentar dort sagt es wörtlich: „Aus `itemLevelOf`, nicht aus
 *   dem Katalog — der Eintrag ist die Vorlage." Verbucht wurde nur die Kennung.
 * - **Die Aufwertung.** `upgradeSteps` wurde gespeichert und als „+14"
 *   angezeigt, ging aber in keine Wertrechnung ein.
 *
 * Das Ergebnis war, dass jedes Stück derselben Kennung exakt gleich stark war:
 *
 * ```
 * catalysis 407, purity 406, yieldBonus 406, essence 67, vessel 67, ward 67
 * catalysis 407, purity 406, yieldBonus 406, essence 67, vessel 67, ward 67
 * ```
 *
 * Damit hatte die Ausrüstung keine Stärke-Achse, sondern nur eine Art-Achse —
 * und 13.4 baut ausdrücklich darauf, dass „alter Inhalt sich von selbst
 * entwertet".
 *
 * # Warum die Vorlage übergeben wird, statt sie hier nachzuschlagen
 *
 * Der naheliegende Weg wäre, `itemById` aufzurufen. Das ergäbe einen
 * Zirkelbezug: `catalog.ts` braucht diese Datei für `wornFrom`, und sie
 * bräuchte `catalog.ts` für die Vorlage. Zirkel zwischen ES-Modulen lösen sich
 * bei Funktionen meist auf, weil die Bindung erst beim Aufruf greift — „meist"
 * ist aber die falsche Grundlage: Er bricht dann abhängig von der
 * Ladereihenfolge, also je nach Einstiegspunkt und nur manchmal.
 *
 * Jeder Aufrufer, der Werte rechnet, hat die Vorlage ohnehin schon
 * nachgeschlagen — `wornFrom` etwa in `def`. Sie zu übergeben kostet nichts
 * und macht die Abhängigkeit sichtbar.
 *
 * Gegenstück im Simulator: `sim/src/gear.rs`.
 */

import { MAX_ITEM_LEVEL } from './budget.js';

/** Was aufgelöst werden kann — die Felder aus `OwnedItem`, ohne den Rest. */
export interface InstanceLike {
  readonly itemLevel?: number;
  readonly quality?: number;
  readonly upgradeSteps?: number;
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
 * Die Stufe dieses Fundes, oder die der Vorlage.
 *
 * **Der Rückfall ist die Vorlage und nicht 1.** Eine 1 wäre die naheliegende
 * Wahl und die falsche: Ein Spielstand aus einer früheren Fassung hat das Feld
 * nicht, und er verlöre beim ersten Laden den grössten Teil seiner Werte (bei
 * Stufe 40 den Faktor 6,7), ohne dass der Spieler etwas getan hat. Mit der
 * Vorlage rechnet ein alter Stand **genau wie vorher** — die neue Achse greift
 * erst für das, was danach gefunden wird. Ein Fassungswechsel darf Fortschritt
 * hinzufügen; er darf keinen wegnehmen.
 */
export function instanceLevelOf(item: InstanceLike, vorlage: number): number {
  return istGanzzahl(item.itemLevel) ? item.itemLevel : vorlage;
}

/** Die Qualität dieses Fundes, oder die der Vorlage. Sonst wie oben. */
export function instanceQualityOf(item: InstanceLike, vorlage: number): number {
  return istGanzzahl(item.quality) ? item.quality : vorlage;
}

/**
 * Die Stufe, mit der wirklich gerechnet wird: Fundstufe **plus Aufwertung**.
 *
 * > „Material und Punkte heben die GEGENSTANDSSTUFE an — höchstens bis zur
 * > Stufe, die im aktuellen Inhalt ohnehin fällt." (13.4)
 *
 * Die Aufwertung hebt also die vorhandene Stufe und steht nicht als zweite
 * Grösse daneben.
 *
 * # Warum geklemmt wird, und warum das nicht kosmetisch ist
 *
 * `basePermille` und `budgetOf` weisen jede Stufe über `MAX_ITEM_LEVEL` ab —
 * mit **0**, nicht mit dem Höchstwert. Ohne die Klemme machte die Aufwertung
 * ein Stück oberhalb schlagartig wertlos: Ein Fund auf Stufe 85 verlöre bei
 * der sechzehnten Aufwertung sämtliche Grundwerte, und zwar still, weil 0 eine
 * gültige Zahl ist. Das wäre derselbe unsichtbare Verlust, gegen den diese
 * Datei geschrieben wurde — nur diesmal beim Belohnen statt beim Finden.
 *
 * **Die Design-Obergrenze aus 13.4 ist eine andere und steht noch aus:** „bis
 * zur Stufe, die im aktuellen Inhalt ohnehin fällt" hängt davon ab, welcher
 * Inhalt freigeschaltet ist — eine Angabe, die weder der Besitz noch der
 * Katalog kennt. Die Klemme hier ist die Grenze der Rechnung, nicht die des
 * Spiels; sie ersetzt die andere nicht.
 */
export function effectiveLevelOf(item: InstanceLike, vorlage: number): number {
  const basis = instanceLevelOf(item, vorlage);
  // Eine unbekannte oder ungültige Vorlage bleibt es — sie durch Aufwertung
  // auf eine gültige Stufe zu heben, gäbe einem Stück Werte, das es nicht gibt.
  if (!istGanzzahl(basis) || basis <= 0) return 0;
  const schritte = istGanzzahl(item.upgradeSteps) && item.upgradeSteps > 0 ? item.upgradeSteps : 0;
  return Math.min(basis + schritte, MAX_ITEM_LEVEL);
}
