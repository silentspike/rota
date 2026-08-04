/**
 * Tempo (CATALYSIS) wirkt auf **Zeiten**, nicht auf den Schaden
 * (GAME-DESIGN 6.7, Issue #177).
 *
 * Das Gegenstück zu `sim/src/haste.rs` — Zeile für Zeile dieselbe Rechnung.
 *
 * > Diablo 4 formuliert es genau: _„Attack speed does not enter the damage
 * > formula, but rather it multiplies how often you get to use it."_
 *
 * ```text
 * neue_zeit = auf_takt_aufrunden(basis_zeit × 1000 / (1000 + tempo_promille))
 * ```
 *
 * # Warum Promille und nicht Fließkomma
 *
 * Die Determinismus-Regel aus 18.4 verbietet Fließkomma-**Zeit**. Ein
 * Tempo-Faktor von 1,17 auf eine Abklingzeit von 8 s ergibt 6,837606… s — eine
 * Zahl, die zwei Sprachen nicht garantiert gleich runden. Ganzzahlig in
 * Promille gerechnet und einmal auf den Logik-Takt gerastert, ist das Ergebnis
 * auf beiden Seiten dieselbe ganze Zahl.
 *
 * JavaScript kennt keine Ganzzahl-Division: `Math.floor` steht deshalb dort,
 * wo Rust `/` auf `u64` rechnet. Beide Seiten bleiben weit unter
 * `Number.MAX_SAFE_INTEGER` (9,007e15) — die längste Abklingzeit aus 6.4 sind
 * 180 000 ms, mal 1000 also 1,8e8.
 *
 * # Was hier NICHT steht: der Deckel
 *
 * 6.7 nennt für CATALYSIS einen Deckel von 50 %, abgeleitet aus
 * `1500 / (1 + t) = 1000` — „die Untergrenze aus 6.6".
 *
 * **6.6 enthält keine Untergrenze.** Nachgezählt am 2026-08-04: Die 1,0 s
 * stehen an genau einer Stelle im Design, nämlich in 6.4, und dort als „per
 * **Talent** senkbar bis **ca.** 1,0 s". Aus einem Ungefähr für einen anderen
 * Mechanismus lässt sich kein Deckel ableiten, und ein `HASTE_CAP = 500` wäre
 * genau die Handabschrift, vor der die Auflage zu Issue #177 warnt.
 *
 * Dieselbe Stelle trägt den zweiten offenen Punkt: 6.7 führt die **globale
 * Wartezeit** als Wirkort, 6.2 schließt sie ausdrücklich aus („Der GCD sinkt
 * ausschließlich über den Talentbaum, nie über Ausrüstung", Festlegung vom
 * 2026-07-29). Beides ist gemeldet; bis zur Entscheidung wirkt Tempo hier auf
 * **Abklingzeiten und DoT-Tick-Abstände** — die Wirkorte, in denen 6.2 und 6.7
 * übereinstimmen.
 */

import { TICK_MS, snapUp } from './loop.js';

/** Der Nenner der Promille-Rechnung. Kein Balance-Wert, sondern die Einheit. */
const PER_MILLE = 1000;

/**
 * Eine Zeit unter dem Einfluss von Tempo.
 *
 * `permille` ist der **Zuwachs**: 0 heißt kein Tempo, 300 heißt „+30 %".
 *
 * # Die drei Sonderfälle, und warum jeder einzeln zählt
 *
 * | Fall           | Ergebnis  | Grund                                        |
 * | -------------- | --------- | -------------------------------------------- |
 * | `baseMs === 0` | `0`       | keine Zeit bleibt keine Zeit — nicht ein Takt |
 * | `permille === 0` | `baseMs`| **ohne Tempo ändert sich nichts** (AC7)      |
 * | Ergebnis `< 10`| `TICK_MS` | ein Takt ist die kleinste Zeit, die es gibt   |
 *
 * Der mittlere Fall ist nicht bloß eine Abkürzung. Ohne ihn liefe jede
 * bestehende Zeit durch `snapUp`, und **falls** eine davon nicht auf dem
 * Takt-Raster läge, verschöbe sie sich — ein Umbau, der ohne getragene
 * Ausrüstung etwas verändert. Genau das schließt AC7 aus.
 */
export function hastedMs(baseMs: number, permille: number): number {
  if (baseMs === 0) return 0;
  if (permille === 0) return baseMs;
  const scaled = Math.floor((baseMs * PER_MILLE) / (PER_MILLE + permille));
  const snapped = snapUp(scaled);
  return snapped < TICK_MS ? TICK_MS : snapped;
}

/**
 * Wie viele Ticks ein Effekt über Zeit bei diesem Abstand abgibt.
 *
 * > **DoTs: die Dauer bleibt, die Ticks werden mehr** (6.7).
 * >
 * > `tick_zahl = dauer / tick_abstand` — ganzzahlig, Rest verfällt.
 *
 * **Kein anteiliger letzter Tick.** WoW zahlt ihn aus, wenn der Rest mindestens
 * 0,1 s beträgt; bei uns wäre das eine Kommazahl im **Schaden**, also genau
 * dort, wo Spiel und Simulator bitgleich sein müssen.
 */
export function tickCount(durationMs: number, tickIntervalMs: number): number {
  if (tickIntervalMs === 0) return 0;
  return Math.floor(durationMs / tickIntervalMs);
}
