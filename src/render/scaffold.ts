/**
 * Vorläufige Zeichenfläche für Issue E1-1.
 *
 * Sie zeigt ausschließlich, dass Logik-Takt und Bild getrennt laufen: Zwei Marken
 * wandern über dieselbe Spur, die obere folgt der interpolierten Zeit, die untere
 * springt nur im Logik-Takt. Bei 144 Hz muss die obere sichtbar flüssiger laufen,
 * während beide Spuren zur selben Zeit am selben Ende ankommen.
 *
 * Das eigentliche HUD entsteht in E1-4 und E1-5 und ersetzt diese Datei.
 *
 * Im Bild-Pfad wird nichts erzeugt: Alle Texte sind feste Zeichenketten, alle
 * Zahlen werden ziffernweise über `numerals.ts` gezeichnet.
 */
import { TICK_MS } from '../core/loop.js';
import { DESIGN_HEIGHT, DESIGN_WIDTH, Surface } from './surface.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';

const BACKGROUND = '#0b0b0f';
const LETTERBOX = '#000000';
const TRACK = '#23232e';
const INK = '#cfcfd8';
const INTERPOLATED = '#e08a3c';
const TICK_ONLY = '#4a6f8a';

const FONT = '28px ui-monospace, "DejaVu Sans Mono", monospace';
const LABEL_TITLE = 'ROTA — core loop scaffold (E1-1)';
const LABEL_INTERPOLATED = 'interpolated';
const LABEL_TICK = 'tick only';
const LABEL_TIME = 'time';
const LABEL_TICKS = 'ticks';
const LABEL_FRAMES = 'frames';

/** Umlaufdauer der Marken. Zwei Sekunden sind lang genug, um Sprünge zu sehen. */
const SWEEP_MS = 2000;

const TRACK_LEFT = 260;
const TRACK_RIGHT = DESIGN_WIDTH - 260;
const TRACK_WIDTH = TRACK_RIGHT - TRACK_LEFT;
const TRACK_Y_INTERPOLATED = 430;
const TRACK_Y_TICK = 610;
const TRACK_THICKNESS = 10;
const MARKER_WIDTH = 16;
const MARKER_HEIGHT = 72;

export class ScaffoldRenderer {
  private digitAdvance = 16;
  private frames = 0;

  constructor(private readonly surface: Surface) {
    const ctx = surface.ctx;
    ctx.font = FONT;
    this.digitAdvance = measureDigitAdvance(ctx);
  }

  render(simTimeMs: number, alpha: number): void {
    const ctx = this.surface.ctx;
    this.frames++;
    this.surface.beginFrame(BACKGROUND, LETTERBOX);

    ctx.font = FONT;
    ctx.textBaseline = 'alphabetic';
    ctx.fillStyle = INK;
    ctx.fillText(LABEL_TITLE, TRACK_LEFT, 180);

    // Spuren
    ctx.fillStyle = TRACK;
    ctx.fillRect(TRACK_LEFT, TRACK_Y_INTERPOLATED, TRACK_WIDTH, TRACK_THICKNESS);
    ctx.fillRect(TRACK_LEFT, TRACK_Y_TICK, TRACK_WIDTH, TRACK_THICKNESS);

    ctx.fillStyle = INK;
    ctx.fillText(LABEL_INTERPOLATED, TRACK_LEFT, TRACK_Y_INTERPOLATED - 40);
    ctx.fillText(LABEL_TICK, TRACK_LEFT, TRACK_Y_TICK - 40);

    // Obere Marke: interpolierte Zeit, also inklusive des angebrochenen Takts.
    const shownMs = simTimeMs + alpha * TICK_MS;
    ctx.fillStyle = INTERPOLATED;
    ctx.fillRect(
      TRACK_LEFT + sweepOffset(shownMs),
      TRACK_Y_INTERPOLATED + TRACK_THICKNESS * 0.5 - MARKER_HEIGHT * 0.5,
      MARKER_WIDTH,
      MARKER_HEIGHT,
    );

    // Untere Marke: nur volle Takte.
    ctx.fillStyle = TICK_ONLY;
    ctx.fillRect(
      TRACK_LEFT + sweepOffset(simTimeMs),
      TRACK_Y_TICK + TRACK_THICKNESS * 0.5 - MARKER_HEIGHT * 0.5,
      MARKER_WIDTH,
      MARKER_HEIGHT,
    );

    // Ablesewerte
    ctx.fillStyle = INK;
    const readoutRight = TRACK_RIGHT;
    ctx.fillText(LABEL_TIME, TRACK_LEFT, DESIGN_HEIGHT - 260);
    drawFixedRight(ctx, simTimeMs / 1000, 2, readoutRight, DESIGN_HEIGHT - 260, this.digitAdvance);
    ctx.fillText(LABEL_TICKS, TRACK_LEFT, DESIGN_HEIGHT - 200);
    drawIntegerRight(
      ctx,
      simTimeMs / TICK_MS,
      readoutRight,
      DESIGN_HEIGHT - 200,
      this.digitAdvance,
    );
    ctx.fillText(LABEL_FRAMES, TRACK_LEFT, DESIGN_HEIGHT - 140);
    drawIntegerRight(ctx, this.frames, readoutRight, DESIGN_HEIGHT - 140, this.digitAdvance);
  }
}

/** Dreiecksbewegung: hin und zurück, damit die Marke nie unstetig springt. */
function sweepOffset(timeMs: number): number {
  const phase = (timeMs % (SWEEP_MS * 2)) / SWEEP_MS;
  const forward = phase <= 1 ? phase : 2 - phase;
  return forward * (TRACK_WIDTH - MARKER_WIDTH);
}
