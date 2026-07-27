/**
 * Die Zauberleiste als Tastatur-Abbild (GAME-DESIGN 9.1) — Stand Issue E1-3.
 *
 * Die Leiste bildet die Tastatur nach: obere Reihe, mittlere Reihe, untere Reihe,
 * Leertaste. **Auge und Finger lernen dieselbe Karte.** Beide Ebenen sind dauerhaft
 * sichtbar, weil auch die selteneren Belegungen ihre Abklingzeit zeigen müssen.
 *
 * Was hier noch **nicht** steht: Zauber-Symbole, die Wischblende als rotierende
 * Maske, die eigene Darstellung der globalen Wartezeit und die Hitze-Anzeige mit
 * Markierung bei 15. Das ist Issue E1-4. Ebenfalls dort zu entscheiden, ob leere
 * Plätze überhaupt gezeichnet werden (GAME-DESIGN 9.1: „nur freigeschaltete Plätze
 * zeigen") — bis dahin bleiben sie als schwache Kästen stehen, damit die feste
 * Lage der Plätze sichtbar ist.
 *
 * Im Bild-Pfad wird nichts erzeugt: feste Zeichenketten, ziffernweise Zahlen,
 * keine Textvermessung.
 */
import { FIGHT_OVERPRESSURE, FIGHT_WON } from '../core/combat.js';
import { TICK_MS } from '../core/loop.js';
import { NEVER, NO_SPELL, type Session } from '../core/session.js';
import {
  KEYS_PER_LAYER,
  LAYER_COUNT,
  SPACE_POSITION,
  columnOfPosition,
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
const INK_FAINT = '#3a3a46';
const SLOT_FILL = '#15151c';
const SLOT_FILL_READY = '#20202b';
const SLOT_EDGE = '#2e2e3a';
const SLOT_EDGE_BOUND = '#4a4a5c';
const GCD_TRACK = '#1d1d27';
const GCD_FILL = '#6f86a8';
const HEAT_TRACK = '#1d1d27';
const HEAT_FILL = '#e08a3c';
const HEAT_MARK = '#efe7d4';
const BOSS_TRACK = '#1d1d27';
const BOSS_FILL = '#a8442f';
const FLASH_ACCEPT = '#efe7d4';
const FLASH_REJECT = '#c03a30';

const FONT_KEY = '20px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_NAME = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_COOLDOWN = '30px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';

const TITLE = 'ROTA — spells, cooldowns, heat (E1-3)';
const BLOCK_TITLES: readonly string[] = ['DIRECT', 'SHIFT'];
const LABEL_GCD = 'GCD';
const LABEL_HEAT = 'HEAT';
const LABEL_BOSS = 'THE FIRST CRUCIBLE';
const LABEL_TIME = 'time';
const LABEL_DAMAGE = 'damage';
const LABEL_CASTS = 'casts';
const LABEL_MISSED = 'misinputs';
const LABEL_STATE = 'state';
const STATE_NAMES: readonly string[] = ['fighting', 'defeated', 'overpressure'];

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
const BLOCK_TOP = 658;
const BLOCK_LEFT: readonly number[] = [90, 90 + BLOCK_WIDTH + 70];
const BAR_LEFT = 90;
const BAR_WIDTH = (BLOCK_LEFT[1] as number) + BLOCK_WIDTH - BAR_LEFT;
const BAR_HEIGHT = 16;
const GCD_TOP = 596;
const HEAT_TOP = 530;
const BOSS_TOP = 300;
const BOSS_HEIGHT = 26;
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
    // Die anzuzeigende Zeit läuft zwischen den Takten weiter: Ohne das ruckten
    // Abklingzeiten und Wartezeit bei 144 Hz sichtbar in Zehntel-Stufen.
    const shownMs = simTimeMs + alpha * TICK_MS;

    this.surface.beginFrame(BACKGROUND, LETTERBOX);
    ctx.textBaseline = 'alphabetic';
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    ctx.fillText(TITLE, BAR_LEFT, 180);

    this.drawBoss(ctx);
    this.drawHeat(ctx);
    this.drawGcd(ctx, shownMs);
    for (let layer = 0; layer < LAYER_COUNT; layer++) this.drawBlock(ctx, layer, shownMs);
    this.drawReadouts(ctx, simTimeMs);
  }

  /** Der Boss als Balken — der Reaktionsgefäß-Ring entsteht in E1-5. */
  private drawBoss(ctx: CanvasRenderingContext2D): void {
    const combat = this.session.combat;
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_BOSS, BAR_LEFT, BOSS_TOP - 14);

    ctx.fillStyle = BOSS_TRACK;
    ctx.fillRect(BAR_LEFT, BOSS_TOP, BAR_WIDTH, BOSS_HEIGHT);
    ctx.fillStyle = BOSS_FILL;
    ctx.fillRect(BAR_LEFT, BOSS_TOP, BAR_WIDTH * combat.bossHpFraction, BOSS_HEIGHT);

    ctx.fillStyle = INK;
    drawIntegerRight(
      ctx,
      combat.bossHpRemaining,
      BAR_LEFT + BAR_WIDTH,
      BOSS_TOP - 14,
      this.digitAdvance,
    );
  }

  /** Hitze mit Markierung an der Bonus-Schwelle (GAME-DESIGN 9, STEP1-SPEC 1.4). */
  private drawHeat(ctx: CanvasRenderingContext2D): void {
    const combat = this.session.combat;
    const resource = combat.school.resource;
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_HEAT, BAR_LEFT, HEAT_TOP - 14);

    ctx.fillStyle = HEAT_TRACK;
    ctx.fillRect(BAR_LEFT, HEAT_TOP, BAR_WIDTH, BAR_HEIGHT);
    ctx.fillStyle = HEAT_FILL;
    ctx.fillRect(BAR_LEFT, HEAT_TOP, BAR_WIDTH * (combat.resource / resource.cap), BAR_HEIGHT);

    // Die Markierung ist die wichtigste Linie des Bildes: Rechts davon verdoppelt
    // der Entlade-Zauber seinen Schaden.
    const markX = BAR_LEFT + BAR_WIDTH * (resource.highlightAt / resource.cap);
    ctx.fillStyle = HEAT_MARK;
    ctx.fillRect(markX - 1, HEAT_TOP - 6, 2, BAR_HEIGHT + 12);

    ctx.fillStyle = INK;
    drawIntegerRight(ctx, combat.resource, BAR_LEFT + BAR_WIDTH, HEAT_TOP - 14, this.digitAdvance);
  }

  private drawGcd(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const gcd = this.session.gcd;
    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_GCD, BAR_LEFT, GCD_TOP - 14);

    ctx.fillStyle = GCD_TRACK;
    ctx.fillRect(BAR_LEFT, GCD_TOP, BAR_WIDTH, BAR_HEIGHT);

    const remaining = gcd.remainingMs(shownMs);
    if (remaining > 0) {
      // Der Balken läuft ab, statt sich zu füllen: Was schrumpft, ist die
      // Wartezeit — die Richtung der Bewegung soll das sagen.
      ctx.fillStyle = GCD_FILL;
      ctx.fillRect(BAR_LEFT, GCD_TOP, (remaining / gcd.lengthMs) * BAR_WIDTH, BAR_HEIGHT);
    }
    ctx.fillStyle = INK;
    drawFixedRight(ctx, remaining / 1000, 2, BAR_LEFT + BAR_WIDTH, GCD_TOP - 14, this.digitAdvance);
  }

  private drawBlock(ctx: CanvasRenderingContext2D, layer: number, shownMs: number): void {
    const left = BLOCK_LEFT[layer] as number;
    const combat = this.session.combat;

    ctx.textAlign = 'left';
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK_FAINT;
    ctx.fillText(BLOCK_TITLES[layer] as string, left, BLOCK_TOP - 18);

    for (let position = 0; position < KEYS_PER_LAYER; position++) {
      const slot = slotOf(layer, position);
      const isSpace = position === SPACE_POSITION;
      const x = isSpace ? left : left + columnOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
      const y = BLOCK_TOP + rowOfPosition(position) * (SLOT_SIZE + SLOT_GAP);
      const width = isSpace ? BLOCK_WIDTH : SLOT_SIZE;
      const height = isSpace ? SPACE_HEIGHT : SLOT_SIZE;

      const spellIndex = this.session.spellOnSlot(slot);
      const bound = spellIndex !== NO_SPELL;
      const ready = bound && combat.isReady(spellIndex, shownMs);

      ctx.fillStyle = ready ? SLOT_FILL_READY : SLOT_FILL;
      ctx.fillRect(x, y, width, height);
      ctx.strokeStyle = bound ? SLOT_EDGE_BOUND : SLOT_EDGE;
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 1, y + 1, width - 2, height - 2);

      this.drawFlash(ctx, slot, shownMs, x, y, width, height);

      // Auf jedem Platz steht der Keybind (GAME-DESIGN 9.1) — auch auf leeren.
      ctx.font = FONT_KEY;
      ctx.textAlign = 'left';
      ctx.fillStyle = bound ? (ready ? INK : INK_DIM) : INK_FAINT;
      ctx.fillText(this.labels.label(position), x + 8, y + 24);

      if (!bound || isSpace) continue;

      const spell = combat.spellAt(spellIndex);
      if (spell === undefined) continue;
      ctx.textAlign = 'center';
      ctx.font = FONT_NAME;
      ctx.fillStyle = ready ? INK : INK_DIM;
      ctx.fillText(spell.name, x + width * 0.5, y + height - 14);

      const remaining = combat.cooldownRemainingMs(spellIndex, shownMs);
      if (remaining > 0) {
        ctx.font = FONT_COOLDOWN;
        ctx.fillStyle = INK;
        drawFixedRight(
          ctx,
          remaining / 1000,
          1,
          x + width * 0.5 + this.digitAdvance * 1.5,
          y + height * 0.5 + 8,
          this.digitAdvance,
        );
      }
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
    const combat = this.session.combat;
    ctx.font = FONT_TEXT;
    ctx.textAlign = 'left';
    ctx.fillStyle = INK_DIM;

    ctx.fillText(LABEL_TIME, READOUT_LEFT, 660);
    ctx.fillText(LABEL_DAMAGE, READOUT_LEFT, 704);
    ctx.fillText(LABEL_CASTS, READOUT_LEFT, 748);
    ctx.fillText(LABEL_MISSED, READOUT_LEFT, 792);
    ctx.fillText(LABEL_STATE, READOUT_LEFT, 836);

    ctx.fillStyle = INK;
    drawFixedRight(ctx, simTimeMs / 1000, 2, READOUT_RIGHT, 660, this.digitAdvance);
    drawIntegerRight(ctx, combat.damageDealt, READOUT_RIGHT, 704, this.digitAdvance);
    drawIntegerRight(ctx, this.session.acceptedCount, READOUT_RIGHT, 748, this.digitAdvance);
    drawIntegerRight(ctx, this.session.rejectedCount, READOUT_RIGHT, 792, this.digitAdvance);

    ctx.textAlign = 'right';
    const state = combat.fightState;
    ctx.fillStyle = state === FIGHT_WON || state === FIGHT_OVERPRESSURE ? HEAT_MARK : INK;
    ctx.fillText(STATE_NAMES[state] ?? '?', READOUT_RIGHT, 836);
    ctx.textAlign = 'left';
  }
}

/** Ausblendkurve: schnell weg, dann ausklingen. Kein Überschwingen. */
function falloff(t: number): number {
  const rest = 1 - t;
  return rest * rest;
}
