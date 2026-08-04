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
 * # Die Wirkorte, und einer der ausdrücklich fehlt
 *
 * | Wirkort                | Untergrenze |
 * | ---------------------- | ----------- |
 * | **Alle** Abklingzeiten | ein Takt    |
 * | DoT-Tick-Abstand       | ein Takt    |
 * | ~~Globale Wartezeit~~  | —           |
 *
 * **Die globale Wartezeit gehört NICHT dazu.** 6.2 legt seit dem 2026-07-29
 * fest: Der GCD sinkt ausschließlich über den **Talentbaum**, nie über
 * Ausrüstung. Kürzere Abklingzeiten sind eine Planungsaufgabe, ein kürzerer GCD
 * ein Reaktionstest — und die Reaktionszeit wird nicht bewertet (21.2).
 *
 * 6.7 hatte sie beim ersten Schreiben versehentlich als Wirkort geführt; die
 * Fassung vom 2026-08-04 streicht sie wieder. Diese Datei hat den Wirkort nie
 * gehabt.
 *
 * # Was hier NICHT steht: der Deckel
 *
 * **Der Tempo-Deckel wird gemessen, nicht gesetzt** (6.7). Die Frage lautet,
 * bei welchem Wert Tempo aufhört, sich gegen Kritchance und Kritschaden zu
 * lohnen — das beantwortet der Balance-Läufer, kein Überlegen.
 *
 * > Eine Ableitung stand dort und trug nicht: „50 %, folgt aus
 * > `1500 / (1 + t) = 1000` — die Untergrenze aus 6.6". Beide Bezugspunkte
 * > waren falsch. 6.6 enthält keine Untergrenze (sie steht in 6.4, als
 * > **„ca. 1,0 s"** und für den **Talentbaum**), und Ausrüstungs-Tempo berührt
 * > die Wartezeit gar nicht.
 * >
 * > **Merksatz aus 6.7:** Eine Zahl mit Begründungs-Anschein ist schlimmer als
 * > eine offen gesetzte — sie sieht geprüft aus. Wer sie liest, sucht die
 * > Quelle nicht nach.
 *
 * Solange die Messung aussteht, ist Tempo hier **ungedeckelt**. Die einzige
 * Grenze ist die Untergrenze von einem Takt je Zeit, und die ist eine
 * Rechenregel, keine Balance-Entscheidung.
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
