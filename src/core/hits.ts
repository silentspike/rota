/**
 * Treffer als Ereignis (GAME-DESIGN 9 „Wucht", 18.3).
 *
 * Der Kampf meldet jeden Treffer; wer darauf reagiert, entscheidet selbst. Die
 * Anzeige lässt eine Schadenszahl aufsteigen und den Ring aufblitzen, der Klang
 * spielt einen Aufschlag. **Keiner der Empfänger darf zurückwirken** — ein
 * stummes, unsichtbares Spiel rechnet identisch (GAME-DESIGN 18.5).
 *
 * Die Meldung besteht aus einzelnen Zahlen statt aus einem Objekt: Bei zwanzig
 * Treffern je Sekunde wären zwanzig Objekte je Sekunde genau die Speicherlast,
 * die GAME-DESIGN 18.2 ausschließt.
 */

/** Woher der Treffer kam. */
export const HIT_DIRECT = 0;
/** Ein Tick eines Effekts über Zeit — kleiner, leiser, unauffälliger. */
export const HIT_DOT = 1;

/**
 * Ab wann ein Schlag „groß" ist: Anteil der Lebenspunkte des Bosses, den ein
 * einzelner Treffer wegnimmt.
 *
 * **Bewusst relativ, nicht absolut.** Eine feste Grenze in Schadenspunkten wäre
 * auf Stufe 1 richtig und auf Stufe 100 sinnlos — dort trifft jeder Füller
 * fünfstellig. Fünf Prozent bedeuten beim Testboss 210 Schaden: KINDLE (100) und
 * ein SMOULDER-Tick (40) lösen nichts aus, PYRE mit Bonus (ab 360) sehr wohl.
 * Genau der Zauber, der sich wuchtig anfühlen soll.
 */
export const BIG_HIT_HP_SHARE = 0.05;

/**
 * Empfänger von Treffern.
 *
 * @param damage Zugefügter Schaden.
 * @param kind `HIT_DIRECT` oder `HIT_DOT`.
 * @param isCrit Kritischer Treffer. In Schritt 1 immer `false` (PURITY = 0),
 *   die Anzeige ist trotzdem darauf vorbereitet.
 * @param big Nimmt der Treffer einen spürbaren Teil der Lebenspunkte weg?
 * @param timeMs Kampfzeit des Treffers.
 */
export type HitFn = (
  damage: number,
  kind: number,
  isCrit: boolean,
  big: boolean,
  timeMs: number,
) => void;
