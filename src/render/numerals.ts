/**
 * Zahlen zeichnen, ohne Speicher anzufordern (GAME-DESIGN 18.2).
 *
 * Das Problem: `String(wert)` erzeugt in JavaScript immer eine neue Zeichenkette.
 * Bei Restsekunden, Schadenszahlen und Prozentwerten wäre das je Bild eine
 * Speicheranforderung — genau das, was die Ruckelfreiheit kosten würde.
 *
 * Die Lösung: Die zehn Ziffern liegen als feste Zeichenketten vor und werden
 * einzeln gezeichnet. Damit kostet eine beliebig große Zahl null Anforderungen.
 * Voraussetzung ist eine Schrift mit gleich breiten Ziffern; das ist im HUD
 * ohnehin die richtige Wahl, weil sonst wackelnde Zahlen entstehen.
 */

const DIGITS: readonly string[] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
const MINUS = '-';
const DOT = '.';

/**
 * Zeichnet eine ganze Zahl rechtsbündig, ohne Speicher anzufordern.
 *
 * @param rightX Rechte Kante der Zahl in Entwurfs-Koordinaten.
 * @param advance Vorschub je Zeichen. Einmalig über `measureDigitAdvance` ermitteln.
 * @returns Die linke Kante der gezeichneten Zahl.
 */
export function drawIntegerRight(
  ctx: CanvasRenderingContext2D,
  value: number,
  rightX: number,
  y: number,
  advance: number,
): number {
  let v = Math.trunc(value);
  const negative = v < 0;
  if (negative) v = -v;
  let x = rightX - advance;
  if (v === 0) {
    ctx.fillText(DIGITS[0] as string, x, y);
    x -= advance;
  } else {
    while (v > 0) {
      ctx.fillText(DIGITS[v % 10] as string, x, y);
      v = (v / 10) | 0;
      x -= advance;
    }
  }
  if (negative) {
    ctx.fillText(MINUS, x, y);
    x -= advance;
  }
  return x + advance;
}

/**
 * Zeichnet eine Zahl mit fester Nachkommastellen-Zahl, rechtsbündig.
 *
 * @param decimals Nachkommastellen, 1 bis 3.
 */
export function drawFixedRight(
  ctx: CanvasRenderingContext2D,
  value: number,
  decimals: number,
  rightX: number,
  y: number,
  advance: number,
): number {
  let factor = 1;
  for (let i = 0; i < decimals; i++) factor *= 10;
  const scaled = Math.round(Math.abs(value) * factor);
  const whole = (scaled / factor) | 0;
  let fraction = scaled - whole * factor;

  let x = rightX - advance;
  for (let i = 0; i < decimals; i++) {
    ctx.fillText(DIGITS[fraction % 10] as string, x, y);
    fraction = (fraction / 10) | 0;
    x -= advance;
  }
  ctx.fillText(DOT, x, y);
  const left = drawIntegerRight(ctx, value < 0 ? -whole : whole, x, y, advance);
  return left;
}

/**
 * Ermittelt den Vorschub einer Ziffer für die aktuell gesetzte Schrift.
 *
 * Erzeugt Speicher (`measureText` liefert ein Objekt) — deshalb einmalig beim
 * Einrichten aufrufen, niemals im Bild-Pfad.
 */
export function measureDigitAdvance(ctx: CanvasRenderingContext2D): number {
  return ctx.measureText(DIGITS[0] as string).width;
}
