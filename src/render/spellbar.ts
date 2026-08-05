import { alsSpellIndex } from '../core/combat.js';
/**
 * Die Zauberleiste (GAME-DESIGN 9, 9.1, 9.4).
 *
 * **Die Leiste bildet die Tastatur nach:** obere Reihe, mittlere Reihe, untere
 * Reihe, Leertaste — links der Block ohne Umschalt, rechts der mit. Auge und
 * Finger lernen dieselbe Karte. Wer das Symbol oben links sieht, weiß ohne
 * Nachdenken, dass die Taste oben links liegt.
 *
 * **Gezeichnet wird nur, was belegt ist** (GAME-DESIGN 9.1: „nur freigeschaltete
 * Plätze zeigen. Am Anfang zwei Symbole statt 26 leerer Kästen"). Die Lage jedes
 * Platzes steht trotzdem von Anfang an fest — ein später dazukommender Zauber
 * verschiebt keinen bestehenden, sonst litte das Muskelgedächtnis.
 *
 * **Drei Zeitanzeigen, drei verschiedene Formen.** Sie dürfen nie verwechselbar
 * sein, weil sie ganz Verschiedenes bedeuten:
 *
 * | Was            | Form                                  | Richtung          |
 * | -------------- | ------------------------------------- | ----------------- |
 * | Abklingzeit    | dunkle Wischblende über dem Symbol    | im Uhrzeigersinn  |
 * | globale Warte. | heller Streifen an der Unterkante     | waagerecht        |
 * | Wirkungsdauer  | Balken an der rechten Kante mit Marken| von oben nach unten |
 */
import type { Combat } from '../core/combat.js';
import { NO_SPELL, type Session } from '../core/session.js';
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
import { drawFixedRight, measureDigitAdvance } from './numerals.js';

// ------------------------------------------------------------------ Geometrie

export const SLOT_SIZE = 104;
export const SLOT_GAP = 12;
export const SPACE_HEIGHT = 52;
export const BLOCK_WIDTH = 4 * SLOT_SIZE + 3 * SLOT_GAP;
export const BLOCK_HEIGHT = 3 * SLOT_SIZE + 3 * SLOT_GAP + SPACE_HEIGHT;
/** Unten links (GAME-DESIGN 9.1). */
export const BLOCK_TOP = 616;
export const BLOCK_LEFT: readonly number[] = [72, 72 + BLOCK_WIDTH + 56];
export const BAR_LEFT = BLOCK_LEFT[0] as number;
export const BAR_RIGHT = (BLOCK_LEFT[1] as number) + BLOCK_WIDTH;

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Lage eines Platzes in Entwurfs-Koordinaten.
 *
 * Schreibt in ein übergebenes Objekt, statt eines zu erzeugen — die Funktion
 * läuft im Bild-Pfad (GAME-DESIGN 18.2).
 */
export function slotRect(slot: number, out: Rect): void {
  const position = positionOfSlot(slot);
  const layer = Math.floor(slot / KEYS_PER_LAYER);
  const left = BLOCK_LEFT[layer] ?? 0;
  const isSpace = position === SPACE_POSITION;
  out.x = isSpace ? left : left + columnOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
  out.y = BLOCK_TOP + rowOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
  out.width = isSpace ? BLOCK_WIDTH : SLOT_SIZE;
  out.height = isSpace ? SPACE_HEIGHT : SLOT_SIZE;
}

// --------------------------------------------------------------------- Farben

const SLOT_FILL_READY = '#232330';
const SLOT_FILL_BUSY = '#15151c';
/** CALCINATION ist rot-orange (GAME-DESIGN 9.3). */
const SCHOOL_EDGE = '#7a4526';
const SCHOOL_EDGE_READY = '#c9702f';
const INK = '#e2e2ea';
const INK_DIM = '#6a6a78';
/** Die Wischblende. Dunkel und halbdurchsichtig, damit das Symbol sichtbar bleibt. */
const SWEEP_VEIL = 'rgba(6, 6, 10, 0.72)';
const SWEEP_EDGE = 'rgba(210, 150, 90, 0.55)';
const GCD_STRIPE = '#7fa6cc';
const DOT_BAR = '#d4762e';
const DOT_BAR_TRACK = 'rgba(255, 255, 255, 0.10)';
const DOT_TICK_MARK = 'rgba(255, 255, 255, 0.55)';
const FLASH_ACCEPT = '#efe7d4';
const FLASH_REJECT = '#c03a30';

const FONT_KEY = '21px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_NAME = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_COOLDOWN = '34px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_DOT = '18px ui-monospace, "DejaVu Sans Mono", monospace';
const BLOCK_TITLES: readonly string[] = ['DIRECT', 'SHIFT'];

/** Zwölf Uhr. Von dort läuft die Wischblende im Uhrzeigersinn weg. */
const SWEEP_START = -Math.PI / 2;
const TWO_PI = Math.PI * 2;

/** Höhe des Streifens für die globale Wartezeit. */
const GCD_STRIPE_HEIGHT = 7;
/** Breite des Balkens für die Wirkungsdauer an der rechten Kante. */
const DOT_BAR_WIDTH = 9;

/** Rückmeldung auf den Tastendruck (GAME-DESIGN 8.2, 9.4). */
const ACCEPT_FLASH_MS = 140;
const REJECT_FLASH_MS = 220;

export class SpellBar {
  private digitAdvance = 16;
  private readonly rect: Rect = { x: 0, y: 0, width: 0, height: 0 };

  constructor(
    private readonly session: Session,
    private readonly labels: KeyLabels,
  ) {}

  /** Einmalig nach dem Setzen der Schrift aufrufen — `measureText` erzeugt Speicher. */
  measure(ctx: CanvasRenderingContext2D): void {
    ctx.font = FONT_COOLDOWN;
    this.digitAdvance = measureDigitAdvance(ctx);
  }

  draw(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const combat = this.session.combat;
    for (let layer = 0; layer < LAYER_COUNT; layer++) {
      let drewAny = false;
      for (let position = 0; position < KEYS_PER_LAYER; position++) {
        const slot = slotOf(layer, position);
        const spellIndex = this.session.spellOnSlot(slot);
        if (spellIndex === NO_SPELL) continue;
        slotRect(slot, this.rect);
        this.drawSlot(ctx, combat, slot, spellIndex, position, shownMs);
        drewAny = true;
      }
      if (drewAny) {
        ctx.font = FONT_NAME;
        ctx.textAlign = 'left';
        ctx.fillStyle = INK_DIM;
        ctx.fillText(BLOCK_TITLES[layer] as string, BLOCK_LEFT[layer] as number, BLOCK_TOP - 18);
      }
    }
  }

  private drawSlot(
    ctx: CanvasRenderingContext2D,
    combat: Combat,
    slot: number,
    spellIndex: number,
    position: number,
    shownMs: number,
  ): void {
    const { x, y, width, height } = this.rect;
    const spell = combat.spellAt(alsSpellIndex(spellIndex));
    if (spell === undefined) return;

    const cooldownLeft = combat.cooldownRemainingMs(spellIndex, shownMs);
    const gcdLeft = combat.gcd.remainingMs(shownMs);
    const ready = cooldownLeft === 0 && gcdLeft === 0;

    // Grundfläche. Der Bereit-Zustand hängt an der Helligkeit, nicht an der Farbe
    // (GAME-DESIGN 21.2: die spielkritische Information darf nicht allein an der
    // Farbe hängen).
    ctx.fillStyle = ready ? SLOT_FILL_READY : SLOT_FILL_BUSY;
    ctx.fillRect(x, y, width, height);

    // Beschriftung und Name liegen UNTER der Wischblende: Sie sollen mit
    // verdunkelt werden, sonst wirkt der Platz halb bereit.
    ctx.font = FONT_KEY;
    ctx.textAlign = 'left';
    ctx.fillStyle = ready ? INK : INK_DIM;
    ctx.fillText(this.labelFor(position), x + 9, y + 26);

    if (height >= SLOT_SIZE) {
      ctx.font = FONT_NAME;
      ctx.textAlign = 'center';
      ctx.fillStyle = ready ? INK : INK_DIM;
      ctx.fillText(spell.name, x + width * 0.5, y + height - 13);
    }

    if (cooldownLeft > 0 && spell.cooldownMs > 0) {
      drawSweep(ctx, x, y, width, height, cooldownLeft / spell.cooldownMs);
    }

    // Rand nach der Blende, damit er nicht mit verdunkelt wird.
    ctx.strokeStyle = ready ? SCHOOL_EDGE_READY : SCHOOL_EDGE;
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 1, y + 1, width - 2, height - 2);

    if (cooldownLeft > 0) {
      ctx.font = FONT_COOLDOWN;
      ctx.textAlign = 'center';
      ctx.fillStyle = INK;
      drawFixedRight(
        ctx,
        cooldownLeft / 1000,
        1,
        x + width * 0.5 + this.digitAdvance * 1.5,
        y + height * 0.5 + 10,
        this.digitAdvance,
      );
    }

    // Globale Wartezeit: waagerechter Streifen an der Unterkante. Er läuft von
    // rechts nach links leer und ist damit von der Blende nicht zu verwechseln.
    if (gcdLeft > 0) {
      const share = gcdLeft / combat.gcd.lengthMs;
      ctx.fillStyle = GCD_STRIPE;
      ctx.fillRect(
        x + 2,
        y + height - GCD_STRIPE_HEIGHT - 2,
        (width - 4) * share,
        GCD_STRIPE_HEIGHT,
      );
    }

    this.drawDot(ctx, combat, spellIndex, shownMs);
    this.drawFlash(ctx, slot, shownMs);
  }

  /**
   * Die Restlaufzeit des Effekts über Zeit — laut GAME-DESIGN 7.2 die wichtigste
   * Anzeige neben den Abklingzeiten.
   *
   * Der Balken beginnt bei voller Höhe und läuft nach unten leer. Die Marken
   * zeigen, wo die Ticks liegen; die unterste fällt mit dem Ablauf zusammen —
   * sechs Ticks bei 12 s und 2 s Abstand. Anzeige und Wirkung müssen hier
   * zusammenfallen, sonst verspricht der Balken Zeit, die nichts mehr bewirkt.
   */
  private drawDot(
    ctx: CanvasRenderingContext2D,
    combat: Combat,
    spellIndex: number,
    shownMs: number,
  ): void {
    const spell = combat.spellAt(alsSpellIndex(spellIndex));
    const dot = spell?.dot;
    if (dot === undefined) return;
    const { x, y, width, height } = this.rect;
    const barX = x + width - DOT_BAR_WIDTH - 3;
    const barY = y + 3;
    const barHeight = height - 6;

    ctx.fillStyle = DOT_BAR_TRACK;
    ctx.fillRect(barX, barY, DOT_BAR_WIDTH, barHeight);

    const remaining = combat.dotRemainingMs(alsSpellIndex(spellIndex), shownMs);
    if (remaining > 0) {
      const share = remaining / dot.durationMs;
      ctx.fillStyle = DOT_BAR;
      ctx.fillRect(barX, barY, DOT_BAR_WIDTH, barHeight * share);
    }

    // Tick-Marken. Der letzte Tick liegt am Ablauf, also ganz unten.
    const ticks = Math.round(dot.durationMs / dot.tickIntervalMs);
    for (let i = 1; i <= ticks; i++) {
      const atMs = dot.durationMs - i * dot.tickIntervalMs;
      const markY = barY + barHeight * (atMs / dot.durationMs);
      ctx.fillStyle = DOT_TICK_MARK;
      ctx.fillRect(barX, markY - 1, DOT_BAR_WIDTH, 2);
    }

    if (remaining > 0) {
      ctx.font = FONT_DOT;
      ctx.textAlign = 'right';
      ctx.fillStyle = INK;
      drawFixedRight(ctx, remaining / 1000, 1, barX - 4, y + 26, this.digitAdvance * 0.55);
    }
  }

  private drawFlash(ctx: CanvasRenderingContext2D, slot: number, shownMs: number): void {
    const { x, y, width, height } = this.rect;
    const acceptedAt = this.session.acceptedAtMs[slot] ?? -1;
    const rejectedAt = this.session.rejectedAtMs[slot] ?? -1;
    const acceptAge = acceptedAt < 0 ? Number.POSITIVE_INFINITY : shownMs - acceptedAt;
    const rejectAge = rejectedAt < 0 ? Number.POSITIVE_INFINITY : shownMs - rejectedAt;

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
      ctx.globalAlpha = falloff(acceptAge / ACCEPT_FLASH_MS) * 0.8;
      ctx.fillStyle = FLASH_ACCEPT;
      ctx.fillRect(x, y, width, height);
      ctx.globalAlpha = 1;
    }
  }

  private labelFor(position: number): string {
    return this.labels.label(position);
  }
}

/**
 * Die Wischblende: eine rotierende Maske, Winkel 0 bis 360° (GAME-DESIGN 9.4).
 *
 * Der dunkle Schleier wandert wie ein Uhrzeiger weg. Bei voller Abklingzeit
 * bedeckt er den ganzen Platz, am Ende verschwindet er vollständig — er bleibt
 * nicht bei 359° stehen, weil der Aufrufer bei Restzeit 0 gar nicht mehr zeichnet.
 *
 * @param remainingShare Restanteil der Abklingzeit, 0 bis 1.
 */
export function drawSweep(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  remainingShare: number,
): void {
  const share = remainingShare > 1 ? 1 : remainingShare < 0 ? 0 : remainingShare;
  if (share <= 0) return;
  const centerX = x + width * 0.5;
  const centerY = y + height * 0.5;
  // Halbe Diagonale: So reicht der Keil sicher bis in die Ecken.
  const radius = Math.sqrt(width * width + height * height) * 0.5;
  const handAngle = SWEEP_START + (1 - share) * TWO_PI;

  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, width, height);
  ctx.clip();

  ctx.beginPath();
  ctx.moveTo(centerX, centerY);
  ctx.arc(centerX, centerY, radius, handAngle, SWEEP_START + TWO_PI);
  ctx.closePath();
  ctx.fillStyle = SWEEP_VEIL;
  ctx.fill();

  // Die Kante des Schleiers macht die Bewegung ablesbar, auch wenn nur noch ein
  // schmaler Keil übrig ist.
  ctx.beginPath();
  ctx.moveTo(centerX, centerY);
  ctx.lineTo(centerX + Math.cos(handAngle) * radius, centerY + Math.sin(handAngle) * radius);
  ctx.strokeStyle = SWEEP_EDGE;
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.restore();
}

/** Der Winkel, bei dem die Kante der Blende gerade steht. Für Tests und Werkzeuge. */
export function sweepHandAngle(remainingShare: number): number {
  return SWEEP_START + (1 - remainingShare) * TWO_PI;
}

/** Ausblendkurve: schnell weg, dann ausklingen. Kein Überschwingen. */
function falloff(t: number): number {
  const rest = 1 - t;
  return rest * rest;
}
