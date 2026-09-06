/**
 * Der Beute-Zufallsstrom (GAME-DESIGN 13.4 und 20.2, Issue E6-2).
 *
 * **Er liegt bewusst AUSSERHALB der Kampf-Ströme** (`src/core/rng.ts`) — das
 * ist eine Festlegung des Auftraggebers vom 2026-08-04, keine Nachlässigkeit.
 *
 * # Warum nicht als vierter Strom in `RandomStreams`
 *
 * Die Versuchung ist groß: „alle Zufallsquellen an einer Stelle" klingt
 * aufgeräumt, und der erste Entwurf dieses Issues hat es genau so gebaut.
 *
 * Die Regel in 20.2 lautet: *„Krit, DoT-Ticks und Heilung brauchen je einen
 * eigenen Strom — **sonst verschiebt eine zusätzliche Ziehung alles
 * Nachfolgende**."* Der zweite Halbsatz ist der Schutzzweck, und er greift für
 * Ströme, die im **selben Ablauf** ziehen.
 *
 * **Beute wird beim Sieg gewürfelt, nicht im Kampf.** Sie kann nichts
 * verschieben, und nichts kann sie verschieben. Die Regel schützt hier vor
 * einer Gefahr, die es nicht gibt.
 *
 * Die Kehrseite ist gemessen, nicht geschätzt: Ein vierter Strom hängt in
 * einem Kampf über 7952 Takte **7952 Nullen** an die Zustands-Prüfsumme, für
 * einen Strom, der dort nie zieht — und verschiebt dabei 23 463 Zeilen in fünf
 * Vergleichsdateien. Länger wird sie, schärfer nicht. Die Prüfsumme soll den
 * **Kampf**-Zustand beschreiben; Beute ist keiner.
 *
 * # Der Einwand, der zur Auflage wurde
 *
 * Zwei Orte für Zufall statt einem heißt: Jemand vergisst später einen Strom,
 * oder — wahrscheinlicher — jemand räumt auf und trägt diesen hier in
 * `RandomStreams` ein. Das würde **stillschweigend jede aufgezeichnete
 * Prüfsumme verschieben**.
 *
 * Dagegen stehen drei Dinge, alle drei hier:
 *
 * | Auflage | Wo sie eingelöst ist |
 * | --- | --- |
 * | dieselbe Ableitung aus dem Kampf-Startwert | [`lootStreamFor`] ruft `seedForStream` auf — dieselbe Funktion, die `RandomStreams` benutzt |
 * | ein Wächter gegen die Rückkehr in `drawCounts()` | `tests/unit/loot-stream.test.ts` |
 * | die Begründung im Code, nicht nur im Design | dieser Kopf |
 *
 * Die erste ist mehr als Kosmetik: Dieser Strom **ist** der, der in
 * `RandomStreams` den Index 3 bekäme. Wer ihn eines Tages doch dorthin
 * verschiebt, bekommt bitgleich dieselben Zahlen — die Entscheidung ist damit
 * umkehrbar, ohne dass eine einzige aufgezeichnete Beute sich ändert.
 */

import { RandomStream, seedForStream } from '../core/rng.js';

/**
 * Der Index, den der Beute-Strom in `RandomStreams` hätte.
 *
 * **Nicht dort eingetragen** — siehe Kopf. Er steht hier, damit die Ableitung
 * dieselbe ist und bleibt.
 */
export const LOOT_STREAM_INDEX = 3;

/**
 * Der Beute-Strom eines Kampfes.
 *
 * Aus dem aufgezeichneten Startwert abgeleitet, damit der Fund exakt
 * nachgerechnet werden kann, ohne die Kampf-Zufallsströme zu verändern.
 */
export function lootStreamFor(fightSeed: number): RandomStream {
  return new RandomStream(seedForStream(fightSeed, LOOT_STREAM_INDEX));
}

/**
 * Ein neuer Beute-Startwert je begonnenem Spieler-Versuch. Er wird im Verlauf
 * gespeichert und berührt keinen Kampfstrom. Auch nach dem Neuladen beginnt
 * damit nicht wieder dieselbe Fundfolge bei null.
 */
export function newLootSeed(): number {
  return globalThis.crypto.getRandomValues(new Uint32Array(1))[0] ?? 0;
}
