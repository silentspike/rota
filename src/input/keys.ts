/**
 * Die Tastenplätze und ihre physische Anordnung (GAME-DESIGN 8.1).
 *
 * **Warum `KeyboardEvent.code` und nicht `key`:** `code` benennt die **Stelle** auf
 * der Tastatur, `key` das Zeichen, das dort steht. Die Stelle links unter `A` heißt
 * überall `KeyZ` — ein englischer Spieler liest dort `Z`, ein deutscher `Y`. Beide
 * drücken dieselbe Taste unter demselben Finger, und genau das will das Design.
 *
 * Deshalb steht in `SLOT_CODES` unten `KeyZ`, obwohl GAME-DESIGN 9.1 die Reihe als
 * „Y X C V" beschreibt: Das Dokument nennt die Beschriftung, die der Auftraggeber
 * auf seiner deutschen Tastatur sieht. Die Kennung derselben Taste ist `KeyZ`.
 *
 *      Q  W  E  R          13 Stellen
 *      A  S  D  F      ×   2 Ebenen (ohne und mit Umschalt)
 *      Y  X  C  V      =   26 Belegungen
 *         Leertaste
 */

/** Tastenstellen je Ebene. */
export const KEYS_PER_LAYER = 13;

/** Ebenen: ohne Umschalt und mit Umschalt. */
export const LAYER_COUNT = 2;
export const LAYER_DIRECT = 0;
export const LAYER_SHIFT = 1;

/** Alle Belegungen zusammen. */
export const SLOT_COUNT = KEYS_PER_LAYER * LAYER_COUNT;

/** Rückgabewert, wenn eine Taste nicht belegt ist. */
export const NO_SLOT = -1;

/**
 * Die dreizehn Stellen in der Reihenfolge, in der sie auf der Leiste liegen:
 * obere Reihe, mittlere Reihe, untere Reihe, Leertaste.
 */
export const SLOT_CODES: readonly string[] = [
  'KeyQ',
  'KeyW',
  'KeyE',
  'KeyR',
  'KeyA',
  'KeyS',
  'KeyD',
  'KeyF',
  'KeyZ',
  'KeyX',
  'KeyC',
  'KeyV',
  'Space',
];

/** Die Stelle der Leertaste. Sie bekommt in der Anzeige eine eigene Zeile. */
export const SPACE_POSITION = 12;

/** Zeilen der Anordnung: 0 bis 2 sind die Buchstabenreihen, 3 ist die Leertaste. */
export const ROW_COUNT = 4;

const CODE_TO_POSITION = new Map<string, number>();
for (let i = 0; i < SLOT_CODES.length; i++) {
  CODE_TO_POSITION.set(SLOT_CODES[i] as string, i);
}

/** Stelle einer Tastenkennung, oder `NO_SLOT`. */
export function positionForCode(code: string): number {
  return CODE_TO_POSITION.get(code) ?? NO_SLOT;
}

/**
 * Belegung für eine Tastenkennung und den Zustand der Umschalttaste.
 *
 * @returns Platz von 0 bis 25, oder `NO_SLOT`, wenn die Taste nicht belegt ist.
 */
export function slotFor(code: string, shift: boolean): number {
  const position = positionForCode(code);
  if (position === NO_SLOT) return NO_SLOT;
  return (shift ? LAYER_SHIFT : LAYER_DIRECT) * KEYS_PER_LAYER + position;
}

/** Platz aus Ebene und Stelle. */
export function slotOf(layer: number, position: number): number {
  return layer * KEYS_PER_LAYER + position;
}

export function layerOfSlot(slot: number): number {
  return Math.floor(slot / KEYS_PER_LAYER);
}

export function positionOfSlot(slot: number): number {
  return slot % KEYS_PER_LAYER;
}

export function codeOfSlot(slot: number): string {
  return SLOT_CODES[positionOfSlot(slot)] as string;
}

/** Zeile einer Stelle: 0 bis 2 für die Buchstabenreihen, 3 für die Leertaste. */
export function rowOfPosition(position: number): number {
  return position === SPACE_POSITION ? 3 : Math.floor(position / 4);
}

/** Spalte einer Stelle innerhalb ihrer Zeile. Die Leertaste steht in Spalte 0. */
export function columnOfPosition(position: number): number {
  return position === SPACE_POSITION ? 0 : position % 4;
}

/** Ist der Platz eine gültige Belegung? */
export function isSlot(slot: number): boolean {
  return Number.isInteger(slot) && slot >= 0 && slot < SLOT_COUNT;
}
