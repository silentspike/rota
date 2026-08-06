/**
 * Die Eingabe für den Gegenstands-Vergleich (GAME-DESIGN 13.6, Issue #455).
 *
 * `compare.ts` war fertig und hatte keine Eingabe: Es nimmt zwei DPS-Schätzungen
 * entgegen, und die Quelle dafür gab es nicht — der einzige Aufrufer reichte
 * `compareItems(1234, 1000)`, zwei erfundene Zahlen. Diese Datei ist die Quelle.
 *
 * # Sie rechnet mit der ECHTEN Formel, nicht mit einer Näherungsformel
 *
 * Die Auflage lautete, die Schätzung dürfe nicht aus einer *im Spielcode
 * gesetzten Formel* kommen. Gemeint war: **keine erfundene Näherung** — eine
 * Abkürzung wie „ESSENCE zählt doppelt, CATALYSIS halb" wäre eine
 * Design-Entscheidung über Zahlen, getarnt als Rechnung.
 *
 * Hier steht das Gegenteil davon. Jeder Faktor wird über **dieselbe Funktion**
 * geholt, die auch `Combat` benutzt (`essenceFactor`, `gearEffectFactor`,
 * `critChance`, `critFactor` aus `src/gear/worn.ts`). Es gibt keine zweite
 * Quelle, die abweichen könnte — ändert jemand die Wertformel, ändert sich
 * diese Schätzung mit.
 *
 * # WAS DIESE SCHÄTZUNG SIEHT — UND WAS NICHT
 *
 * Das gehört an die Rechenstelle und nicht in eine Nachricht, weil der nächste
 * Leser sie sonst für vollständig hält.
 *
 * | Wert | Wirkung | gesehen |
 * | --- | --- | --- |
 * | `essence` | Schadensfaktor, Topf 1 | **ja** |
 * | `gearEffects` | Schadensfaktor, Topf 4 | **ja** |
 * | `purity` | Kritchance | **ja** |
 * | `yieldBonus` | Kritschaden | **ja** |
 * | `catalysis` | Tempo — verkürzt **Zeiten** | **NEIN** |
 *
 * **Ein Tempo-Stück wird deshalb systematisch UNTERSCHÄTZT.** Die Wertformel
 * sieht, was ein Gegenstand je **Wirkung** an Schaden bringt; sie sieht nicht,
 * dass CATALYSIS im selben Kampf **mehr Wirkungen** ermöglicht. Das ist eine
 * Rotations-Wirkung, und dafür gibt es den Balance-Läufer — eine Schätzung, die
 * ohne Kampf auskommen soll, kann sie nicht haben.
 *
 * Genau das meint der Plan mit *„die Bewertung ist eine Näherung und kann bei
 * ungewöhnlichen Zusammenstellungen danebenliegen"* (A.5). Der Fall ist nicht
 * hypothetisch: Ein reines Tempo-Stück ist die ungewöhnliche Zusammenstellung.
 *
 * Deshalb trägt jede Anzeige aus `compare.ts` ein `~` **und** einen Zusatz in
 * Worten. Die Kennzeichnung ist keine Höflichkeit, sondern die Stelle, an der
 * diese Grenze für den Spieler sichtbar wird.
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */
import { compareItems, type Comparison } from './compare.js';
import { itemById, wornFrom, type OwnedItemLike } from '../gear/catalog.js';
import {
  critChance,
  critFactor,
  essenceFactor,
  gearEffectFactor,
  type WornValues,
} from '../gear/worn.js';

/**
 * Der geschätzte Schaden **je Wirkung**, als Vielfaches des Grundschadens.
 *
 * Ohne getragene Ausrüstung ist das Ergebnis exakt 1,0: Alle drei Faktoren sind
 * dann die exakte Eins, und `critChance` ist null — die Multiplikation bewegt
 * keine einzige Zahl. Dieselbe Zusicherung wie bei `NOTHING_WORN`.
 *
 * # Warum der Krit-Term so und nicht als `1 + p · (k − 1)` dasteht
 *
 * Beide Ausdrücke sind mathematisch dasselbe. Der hier geschriebene bildet
 * aber ab, was im Kampf wirklich geschieht: `Combat` hält zwei Faktorensätze,
 * und bei einem Krit tritt `critFactor` an die Stelle der neutralen Eins
 * (`factorsOnCrit = { ...factors, crit: critFactor(worn) }`). Der
 * Erwartungswert ist die gewichtete Summe **dieser beiden Fälle** — wer die
 * Zeile liest, findet die Rechenstelle wieder.
 */
export function estimateDamagePerHit(worn: WornValues): number {
  const p = critChance(worn);
  // Die Faktoren einzeln und in der Reihenfolge der Schadensformel (6.3).
  // Zwischen den Töpfen wird multipliziert (13.1b) — sie hier zu addieren
  // machte aus zwei Töpfen einen und kippte die Abwägung aus 13.5.
  const werte = essenceFactor(worn) * gearEffectFactor(worn);
  return werte * ((1 - p) * 1 + p * critFactor(worn));
}

/**
 * Um wie viel eine Ausrüstung besser ist als eine andere — als Anteil.
 *
 * Das Ergebnis geht so in `compareItems`; getrennt gehalten, damit der
 * Vergleich prüfbar bleibt, ohne die Anzeige zu bauen.
 */
export function estimatedGain(withFind: WornValues, withWorn: WornValues): number {
  const vorher = estimateDamagePerHit(withWorn);
  if (!(vorher > 0)) return 0;
  return (estimateDamagePerHit(withFind) - vorher) / vorher;
}

/**
 * Wie ein Stück im Beutel gegen das abschneidet, was gerade auf seinem Platz
 * liegt.
 *
 * # Warum das hier steht und nicht in der Ansicht
 *
 * Die Ansicht ist DOM und wird im Browser geprüft; diese Rechnung ist es
 * nicht. Läge sie dort, wäre der Kern von AC2 — dasselbe Stück, andere
 * Zusammenstellung, anderes Urteil — nur noch von Hand am Bild zu prüfen.
 *
 * # Der Tausch ist ein echter Tausch
 *
 * Nicht „Werte des Fundes gegen Werte des Getragenen": Ein Fund **verdrängt**
 * das Stück auf seinem Platz, und die Werte wirken multiplikativ über alle
 * getragenen Stücke zusammen (13.1b). Deshalb wird die **ganze** Ausrüstung
 * einmal mit und einmal ohne den Tausch gerechnet.
 *
 * Genau daran hängt AC2: Was ein Stück wert ist, folgt aus dem, was sonst
 * getragen wird — nicht aus dem Stück allein.
 */
export function compareBagItem(
  items: readonly OwnedItemLike[],
  bagIndex: number,
): Comparison | undefined {
  const fund = items[bagIndex];
  if (fund === undefined || fund.equippedSlot !== null) return undefined;
  const def = itemById(fund.itemId);
  if (def === undefined) return undefined;

  const nachTausch = items.map((besitz, i) => {
    // Der Fund geht auf seinen Platz …
    if (i === bagIndex) return { ...besitz, equippedSlot: def.slot };
    // … und was dort lag, geht in den Beutel.
    if (besitz.equippedSlot === def.slot) return { ...besitz, equippedSlot: null };
    return besitz;
  });

  return compareItems(
    estimateDamagePerHit(wornFrom(nachTausch)),
    estimateDamagePerHit(wornFrom(items)),
  );
}
