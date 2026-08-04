/**
 * Material und Aufwertung (GAME-DESIGN 13.6, Issue E6-4).
 *
 * | Regel | Festlegung |
 * | --- | --- |
 * | **Überflüssige Beute → Material** | automatisch, kein Inventar, kein Sortieren |
 * | **Aufwertung** | hebt die Gegenstandsstufe, **höchstens bis zur Stufe, die im aktuellen Inhalt ohnehin fällt** |
 * | **Stückgebunden** | hängt am einzelnen Gegenstand, nicht am Platz |
 *
 * # Die Obergrenze ist die eigentliche Regel
 *
 * > „Grinden macht nie stärker als der Inhalt hergibt." (13.6)
 *
 * Sie ist **hart**, nicht teuer: Die Aufwertung bleibt an der Obergrenze
 * **stehen**, egal wie viel Material daliegt. Eine bloß steigende Kostenkurve
 * wäre etwas anderes — sie machte den Grind zäh, aber nicht endlich, und die
 * Regel aus 13.6 wäre gebrochen, ohne dass es jemand bemerkte.
 *
 * Deshalb gibt [`upgradeItem`] die erreichte Stufe zurück **und** wie viel
 * Material tatsächlich verbraucht wurde. An der Obergrenze ist der Verbrauch
 * null: Wer dort aufwertet, verliert nichts.
 *
 * # Der bekannte Nachteil, den das Design selbst benennt
 *
 * > „Jeder bessere Fund entwertet die Investition — das kann das Ausprobieren
 * > bremsen. **Im Auge behalten.**" (13.6)
 *
 * Das ist keine Frage der Umsetzung, sondern eine bewusste Folge der
 * Stückbindung. Sie steht hier, damit sie beim Lesen nicht wie ein Versehen
 * aussieht.
 *
 * # Was hier vorläufig ist
 *
 * **Alle Zahlen** — was ein Stück je Qualität an Material bringt und was eine
 * Stufe kostet. Der Balance-Läufer entscheidet sie (#118). **Nicht**
 * vorläufig: dass höhere Qualität mehr bringt, und dass die Obergrenze hält.
 */

import { QUALITY_COUNT } from '../gear/budget.js';
import { itemLevelOf } from './rates.js';

/**
 * Was ein nicht behaltenes Stück je Qualität an Material bringt.
 * **Vorläufig (#118)** — nicht vorläufig ist, dass es mit der Qualität steigt.
 */
export const MATERIAL_PER_QUALITY: readonly number[] = [1, 3, 9, 27];

/**
 * Was eine Stufe Aufwertung kostet. **Vorläufig (#118).**
 *
 * Ganzzahlig, damit Spiel und Simulator bitgleich rechnen (18.4).
 */
export const MATERIAL_PER_LEVEL = 4;

/**
 * Das Material, das ein nicht behaltener Gegenstand einbringt.
 *
 * „Nicht behaltene Gegenstände werden **automatisch** umgewandelt. Kein
 * Inventar, kein Sortieren." — deshalb gibt es hier keine Entscheidung zu
 * treffen, nur eine Umrechnung.
 */
export function materialFor(quality: number): number {
  if (!Number.isInteger(quality) || quality < 0 || quality >= QUALITY_COUNT) return 0;
  return MATERIAL_PER_QUALITY[quality] ?? 0;
}

/** Was eine Aufwertung ergeben hat. */
export interface UpgradeResult {
  /** Die erreichte Gegenstandsstufe. */
  readonly level: number;
  /** Wie viel Material tatsächlich verbraucht wurde. */
  readonly spent: number;
  /** Ob die Obergrenze des Inhalts die Aufwertung angehalten hat. */
  readonly cappedByContent: boolean;
}

/**
 * Wertet ein Stück auf, so weit Material und **Inhalt** es zulassen.
 *
 * `instance`, `difficulty` und `endlessLevel` beschreiben den **aktuell
 * gespielten** Inhalt — aus ihm folgt die Obergrenze (13.6, über
 * [`itemLevelOf`]). Nicht aus dem Boss, bei dem das Stück gefallen ist: Wer
 * inzwischen weiter ist, darf ein altes Stück nachziehen.
 *
 * **Die Obergrenze hält auch bei beliebig viel Material.** Das ist die Regel,
 * nicht eine Kostenfrage.
 */
export function upgradeItem(
  itemLevel: number,
  material: number,
  instance: number,
  difficulty: number,
  endlessLevel: number,
): UpgradeResult {
  const grenze = itemLevelOf(instance, difficulty, endlessLevel);
  const start = Number.isInteger(itemLevel) && itemLevel > 0 ? itemLevel : 0;
  const vorrat = Number.isInteger(material) && material > 0 ? material : 0;

  if (start >= grenze) {
    // Schon am oder über dem Deckel — es wird nichts verbraucht. Wer hier
    // aufwertet, verliert nichts; er kommt nur nicht weiter.
    return { level: start, spent: 0, cappedByContent: true };
  }

  const bezahlbar = Math.floor(vorrat / MATERIAL_PER_LEVEL);
  const moeglich = grenze - start;
  const stufen = Math.min(bezahlbar, moeglich);

  return {
    level: start + stufen,
    spent: stufen * MATERIAL_PER_LEVEL,
    cappedByContent: stufen === moeglich,
  };
}

/**
 * Die Obergrenze, bis zu der im aktuellen Inhalt aufgewertet werden kann.
 *
 * Eigene Funktion, damit die Anzeige sie nennen kann, ohne eine Aufwertung
 * durchzurechnen — „bis 42" ist eine Auskunft, kein Vorgang.
 */
export function upgradeCapOf(instance: number, difficulty: number, endlessLevel: number): number {
  return itemLevelOf(instance, difficulty, endlessLevel);
}
