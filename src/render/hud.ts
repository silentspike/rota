/**
 * Die Zauberleiste als Tastatur-Abbild (GAME-DESIGN 9.1) — Stand Issue E1-2.
 *
 * Die Leiste bildet die Tastatur nach: obere Reihe, mittlere Reihe, untere Reihe,
 * Leertaste. **Auge und Finger lernen dieselbe Karte.** Beide Ebenen sind dauerhaft
 * sichtbar, weil auch die selteneren Belegungen ihre Abklingzeit zeigen müssen.
 *
 * Was hier noch **nicht** steht: Zauber-Symbole, Wischblende, Restsekunden und die
 * eigene Darstellung der globalen Wartezeit in ihrer endgültigen Form. Das ist
 * Issue E1-4. Gezeichnet wird derzeit das Gerüst plus die beiden Rückmeldungen,
 * die zu E1-2 gehören: Aufleuchten bei angenommener und rotes Blinken bei
 * verworfener Eingabe (GAME-DESIGN 8.2).
 *
 * Im Bild-Pfad wird nichts erzeugt: feste Zeichenketten, ziffernweise Zahlen,
 * keine Textvermessung.
 */
import { TICK_MS } from '../core/loop.js';
import type { Session } from '../core/session.js';
import { NEVER } from '../core/session.js';
import {
  KEYS_PER_LAYER,
  LAYER_COUNT,
  SPACE_POSITION,
  columnOfPosition,
  positionOfSlot,
  rowOfPosition,
  slotOf,
} from '../input/keys.js';
import type { KeyLabels } from '../input/layout.js';
import { DESIGN_WIDTH, Surface } from './surface.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';

const BACKGROUND = '#0b0b0f';
const LETTERBOX = '#000000';
const INK = '#cfcfd8';
const INK_DIM = '#61616e';
const SLOT_FILL = '#15151c';
const SLOT_FILL_READY = '#1d1d27';
const SLOT_EDGE = '#2e2e3a';
const GCD_TRACK = '#1d1d27';
const GCD_FILL = '#6f86a8';
const FLASH_ACCEPT = '#efe7d4';
const FLASH_REJECT = '#c03a30';

const FONT_LABEL = '30px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';

const TITLE = 'ROTA — key bindings (E1-2)';
const BLOCK_TITLES: readonly string[] = ['DIRECT', 'SHIFT'];
const LABEL_GCD = 'GCD';
const LABEL_ACCEPTED = 'accepted';
const LABEL_REJECTED = 'rejected';
const LABEL_TIME = 'time';
const LABEL_LAYOUT = 'labels';

/**
 * Bewegungen nach den vier Größen aus GAME-DESIGN 9.4: Dauer, Kurve,
 * Überschwingen, Verzögerung. Beide Blitze laufen ohne Überschwingen und ohne
 * Verzögerung — eine Rückmeldung auf einen Tastendruck darf nicht nachfedern.
 */
const ACCEPT_FLASH_MS = 140;
const REJECT_FLASH_MS = 220;

const SLOT_SIZE = 92;
const SLOT_GAP = 10;
const SPACE_HEIGHT = 46;
const BLOCK_WIDTH = 4 * SLOT_SIZE + 3 * SLOT_GAP;
/** Drei Buchstabenreihen, dann die Leertaste. */
const BLOCK_HEIGHT = 3 * SLOT_SIZE + 3 * SLOT_GAP + SPACE_HEIGHT;
const BLOCK_TOP = 658;
const BLOCK_LEFT: readonly number[] = [90, 90 + BLOCK_WIDTH + 70];
const GCD_LEFT = 90;
const GCD_TOP = 566;
const GCD_WIDTH = (BLOCK_LEFT[1] as number) + BLOCK_WIDTH - GCD_LEFT;
const GCD_HEIGHT = 16;
const READOUT_RIGHT = DESIGN_WIDTH - 90;
const READOUT_LEFT = 1180;

export class HudRenderer {
  private digitAdvance = 16;

  constructor(
    private readonly surface: Surface,
    private readonly session: Session,
    private readonly labels: KeyLabels,
  ) {
    const ctx = surface.ctx;
    ctx.font = FONT_TEXT;
    this.digitAdvance = measureDigitAdvance(ctx);
  }

  render(simTimeMs: number, alpha: number): void {
    const ctx = this.surface.ctx;
    // Die anzuzeigende Zeit läuft zwischen den Takten weiter: Ohne das ruckte die
    // Wartezeit-Anzeige bei 144 Hz sichtbar in Zehntel-Sekunden-Stufen.
    const shownMs = simTimeMs + alpha * TICK_MS;

    this.surface.beginFrame(BACKGROUND, LETTERBOX);
    ctx.textBaseline = 'alphabetic';
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    ctx.fillText(TITLE, GCD_LEFT, 180);

    this.drawGcd(ctx, shownMs);
    for (let layer = 0; layer < LAYER_COUNT; layer++) this.drawBlock(ctx, layer, shownMs);
    this.drawReadouts(ctx, simTimeMs);
  }

  private drawGcd(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const gcd = this.session.gcd;
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    ctx.fillText(LABEL_GCD, GCD_LEFT, GCD_TOP - 14);

    ctx.fillStyle = GCD_TRACK;
    ctx.fillRect(GCD_LEFT, GCD_TOP, GCD_WIDTH, GCD_HEIGHT);

    const remaining = gcd.remainingMs(shownMs);
    if (remaining > 0) {
      // Der Balken läuft ab, statt sich zu füllen: Was schrumpft, ist die
      // Wartezeit — die Richtung der Bewegung soll das sagen.
      const width = (remaining / gcd.lengthMs) * GCD_WIDTH;
      ctx.fillStyle = GCD_FILL;
      ctx.fillRect(GCD_LEFT, GCD_TOP, width, GCD_HEIGHT);
    }
    ctx.textAlign = 'left';
    drawFixedRight(ctx, remaining / 1000, 2, GCD_LEFT + GCD_WIDTH, GCD_TOP - 14, this.digitAdvance);
  }

  private drawBlock(ctx: CanvasRenderingContext2D, layer: number, shownMs: number): void {
    const left = BLOCK_LEFT[layer] as number;
    const ready = this.session.gcd.isReady(shownMs);

    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(BLOCK_TITLES[layer] as string, left, BLOCK_TOP - 18);

    for (let position = 0; position < KEYS_PER_LAYER; position++) {
      const slot = slotOf(layer, position);
      const row = rowOfPosition(position);
      const isSpace = position === SPACE_POSITION;
      const x = isSpace ? left : left + columnOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
      const y = BLOCK_TOP + row * (SLOT_SIZE + SLOT_GAP);
      const width = isSpace ? BLOCK_WIDTH : SLOT_SIZE;
      const height = isSpace ? SPACE_HEIGHT : SLOT_SIZE;

      ctx.fillStyle = ready ? SLOT_FILL_READY : SLOT_FILL;
      ctx.fillRect(x, y, width, height);
      ctx.strokeStyle = SLOT_EDGE;
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 1, y + 1, width - 2, height - 2);

      this.drawFlash(ctx, slot, shownMs, x, y, width, height);

      ctx.fillStyle = ready ? INK : INK_DIM;
      ctx.font = FONT_LABEL;
      ctx.textAlign = 'center';
      ctx.fillText(this.labels.label(position), x + width * 0.5, y + height * 0.5 + 11);
    }
  }

  /** Aufleuchten bei angenommener, rotes Blinken bei verworfener Eingabe. */
  private drawFlash(
    ctx: CanvasRenderingContext2D,
    slot: number,
    shownMs: number,
    x: number,
    y: number,
    width: number,
    height: number,
  ): void {
    const acceptedAt = this.session.acceptedAtMs[slot] as number;
    const rejectedAt = this.session.rejectedAtMs[slot] as number;
    const acceptAge = acceptedAt === NEVER ? Number.POSITIVE_INFINITY : shownMs - acceptedAt;
    const rejectAge = rejectedAt === NEVER ? Number.POSITIVE_INFINITY : shownMs - rejectedAt;

    // Der jüngere der beiden Blitze gewinnt: Wer zweimal drückt, sieht die
    // zweite Rückmeldung, nicht die erste.
    if (rejectAge <= acceptAge) {
      if (rejectAge >= 0 && rejectAge < REJECT_FLASH_MS) {
        ctx.globalAlpha = falloff(rejectAge / REJECT_FLASH_MS);
        ctx.fillStyle = FLASH_REJECT;
        ctx.fillRect(x, y, width, height);
        ctx.globalAlpha = 1;
      }
      return;
    }
    if (acceptAge >= 0 && acceptAge < ACCEPT_FLASH_MS) {
      ctx.globalAlpha = falloff(acceptAge / ACCEPT_FLASH_MS) * 0.85;
      ctx.fillStyle = FLASH_ACCEPT;
      ctx.fillRect(x, y, width, height);
      ctx.globalAlpha = 1;
    }
  }

  private drawReadouts(ctx: CanvasRenderingContext2D, simTimeMs: number): void {
    ctx.font = FONT_TEXT;
    ctx.textAlign = 'left';
    ctx.fillStyle = INK_DIM;

    ctx.fillText(LABEL_TIME, READOUT_LEFT, 700);
    ctx.fillText(LABEL_ACCEPTED, READOUT_LEFT, 744);
    ctx.fillText(LABEL_REJECTED, READOUT_LEFT, 788);
    ctx.fillText(LABEL_LAYOUT, READOUT_LEFT, 832);

    ctx.fillStyle = INK;
    drawFixedRight(ctx, simTimeMs / 1000, 2, READOUT_RIGHT, 700, this.digitAdvance);
    drawIntegerRight(ctx, this.session.acceptedCount, READOUT_RIGHT, 744, this.digitAdvance);
    drawIntegerRight(ctx, this.session.rejectedCount, READOUT_RIGHT, 788, this.digitAdvance);
    ctx.textAlign = 'right';
    ctx.fillText(this.labels.source, READOUT_RIGHT, 832);
    ctx.textAlign = 'left';
  }
}

/** Ausblendkurve: schnell weg, dann ausklingen. Kein Überschwingen. */
function falloff(t: number): number {
  const rest = 1 - t;
  return rest * rest;
}

/** Nur für Tests: die Höhe eines Blocks in Entwurfs-Koordinaten. */
export const BLOCK_GEOMETRY = {
  slotSize: SLOT_SIZE,
  gap: SLOT_GAP,
  width: BLOCK_WIDTH,
  height: BLOCK_HEIGHT,
  top: BLOCK_TOP,
  left: BLOCK_LEFT,
} as const;

export function slotRect(
  slot: number,
  out: { x: number; y: number; width: number; height: number },
): void {
  const position = positionOfSlot(slot);
  const layer = Math.floor(slot / KEYS_PER_LAYER);
  const left = BLOCK_LEFT[layer] as number;
  const isSpace = position === SPACE_POSITION;
  out.x = isSpace ? left : left + columnOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
  out.y = BLOCK_TOP + rowOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
  out.width = isSpace ? BLOCK_WIDTH : SLOT_SIZE;
  out.height = isSpace ? SPACE_HEIGHT : SLOT_SIZE;
}
