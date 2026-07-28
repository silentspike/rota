/**
 * Das Reaktionsgefäß (GAME-DESIGN 9).
 *
 * **Ein großer Kreisring in der Bildmitte ist Lebensbalken und Boss-Darstellung
 * zugleich.** Es gibt keine Kreatur und kein Artwork (Design-Gesetz 4) — darin
 * läuft eine Umwandlung außer Kontrolle, und der Ring zeigt, wie weit sie ist.
 *
 * Der Ring hat zwei Zustände nebeneinander:
 *
 * - **glühender Rest** — was noch reagiert, im Rot-Orange der Kalzination
 * - **auskristallisierter Teil** — was schon verbraucht ist: matt, kalt, mit
 *   Bruchkanten statt glattem Bogen
 *
 * Beim Überdruck färbt sich der Ring rot und vibriert (GAME-DESIGN 9). Die
 * Vibration ist eine Sinusfunktion der Zeit, **kein Zufall**: Zwei Durchläufe
 * müssen dasselbe Bild ergeben (Design-Gesetz 6).
 */

/** Mitte des Rings in Entwurfs-Koordinaten. */
export const VESSEL_CENTER_X = 960;
export const VESSEL_CENTER_Y = 336;
export const VESSEL_RADIUS = 186;
export const VESSEL_THICKNESS = 34;

/** Zwölf Uhr. Von dort füllt sich der Ring im Uhrzeigersinn. */
const RING_START = -Math.PI / 2;
const TWO_PI = Math.PI * 2;

const GLOW_OUTER = '#f2a049';
const GLOW_INNER = '#c0472a';
const CRYSTAL = '#2b2b36';
const CRYSTAL_EDGE = '#3d3d4c';
const OVERPRESSURE = '#d8362a';
const INK = '#e6e6ee';
const INK_DIM = '#75758a';

/** Ab dieser Restzeit beginnt der Ring zu warnen. */
export const OVERPRESSURE_WARNING_MS = 10_000;
/** Größte Auslenkung der Vibration in Entwurfs-Pixeln. */
const SHAKE_MAX_PX = 5;
/** Schwingungen je Sekunde. */
const SHAKE_HZ = 14;

/** Wie viele Bruchkanten der verbrauchte Teil bekommt. */
const CRYSTAL_SEGMENTS = 36;

export interface VesselView {
  /** Anteil der verbleibenden Lebenspunkte, 0 bis 1. */
  readonly hpFraction: number;
  /** Restzeit bis zum Überdruck in ms. */
  readonly overpressureRemainingMs: number;
  /** Anzeigezeit in ms — treibt die Vibration. */
  readonly shownMs: number;
  /** Ist der Kampf vorbei? Dann steht der Ring still. */
  readonly finished: boolean;
}

/**
 * Auslenkung der Vibration zu einem Zeitpunkt.
 *
 * Getrennt herausgezogen, damit sie ohne Zeichenfläche prüfbar ist: Sie muss bei
 * gleicher Zeit denselben Wert liefern und außerhalb der Warnzeit exakt null sein.
 */
export function shakeOffset(overpressureRemainingMs: number, shownMs: number): number {
  if (overpressureRemainingMs <= 0 || overpressureRemainingMs >= OVERPRESSURE_WARNING_MS) return 0;
  // Je näher der Überdruck, desto stärker — quadratisch, damit die letzten
  // Sekunden deutlich herausstechen und die davor ruhig bleiben.
  const urgency = 1 - overpressureRemainingMs / OVERPRESSURE_WARNING_MS;
  return Math.sin((shownMs / 1000) * SHAKE_HZ * TWO_PI) * SHAKE_MAX_PX * urgency * urgency;
}

/** Farbe des glühenden Teils, zum Überdruck hin ins Rote gezogen. */
function glowColor(overpressureRemainingMs: number): string {
  if (overpressureRemainingMs <= 0 || overpressureRemainingMs >= OVERPRESSURE_WARNING_MS) {
    return GLOW_OUTER;
  }
  return OVERPRESSURE;
}

export function drawVessel(ctx: CanvasRenderingContext2D, view: VesselView): void {
  const shake = view.finished ? 0 : shakeOffset(view.overpressureRemainingMs, view.shownMs);
  const cx = VESSEL_CENTER_X + shake;
  const cy = VESSEL_CENTER_Y;
  const outer = VESSEL_RADIUS;
  const inner = VESSEL_RADIUS - VESSEL_THICKNESS;
  const share = view.hpFraction < 0 ? 0 : view.hpFraction > 1 ? 1 : view.hpFraction;

  // Der verbrauchte Teil: auskristallisiert. Erst der volle Ring in matt, dann
  // die Bruchkanten — so entsteht der Eindruck von erstarrter Masse, ohne dass
  // eine Textur nötig wäre.
  ctx.beginPath();
  ctx.arc(cx, cy, outer, 0, TWO_PI);
  ctx.arc(cx, cy, inner, 0, TWO_PI, true);
  ctx.fillStyle = CRYSTAL;
  ctx.fill('evenodd');

  ctx.strokeStyle = CRYSTAL_EDGE;
  ctx.lineWidth = 1.5;
  for (let i = 0; i < CRYSTAL_SEGMENTS; i++) {
    const angle = RING_START + (i / CRYSTAL_SEGMENTS) * TWO_PI;
    // Nur im verbrauchten Teil, also hinter dem glühenden Bogen.
    if (i / CRYSTAL_SEGMENTS < share) continue;
    ctx.beginPath();
    ctx.moveTo(cx + Math.cos(angle) * inner, cy + Math.sin(angle) * inner);
    ctx.lineTo(cx + Math.cos(angle) * outer, cy + Math.sin(angle) * outer);
    ctx.stroke();
  }

  // Der glühende Rest.
  if (share > 0) {
    const end = RING_START + share * TWO_PI;
    ctx.beginPath();
    ctx.arc(cx, cy, outer, RING_START, end);
    ctx.arc(cx, cy, inner, end, RING_START, true);
    ctx.closePath();
    ctx.fillStyle = glowColor(view.overpressureRemainingMs);
    ctx.fill();

    // Innenkante dunkler: Der Ring bekommt Tiefe, ohne dass ein Verlauf nötig
    // wäre — Verläufe erzeugen je Bild ein Objekt (GAME-DESIGN 18.2).
    ctx.beginPath();
    ctx.arc(cx, cy, inner + 5, RING_START, end);
    ctx.strokeStyle = GLOW_INNER;
    ctx.lineWidth = 8;
    ctx.stroke();
  }

  // Äußere Fassung.
  ctx.beginPath();
  ctx.arc(cx, cy, outer, 0, TWO_PI);
  ctx.strokeStyle = CRYSTAL_EDGE;
  ctx.lineWidth = 2;
  ctx.stroke();
}

/** Farben und Maße, die das HUD für Beschriftung und Zahlen braucht. */
export const VESSEL_INK = INK;
export const VESSEL_INK_DIM = INK_DIM;
