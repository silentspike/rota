/**
 * Das HUD (GAME-DESIGN 9) — Stand Issue E1-4.
 *
 * **Das Spiel IST das HUD** (Design-Gesetz 4). Es gibt keine Spielfigur und kein
 * Kreatur-Artwork; die Gestaltungsarbeit fließt in Zauberleiste, Abklingzeiten,
 * Zahlen und Balken.
 *
 * Anordnung nach GAME-DESIGN 9.1:
 *
 * - **Zauberleiste unten links** — siehe `spellbar.ts`
 * - **Schul-Mechanik-Anzeige direkt darüber** — die Hitze, mit Markierung bei 15
 * - **Instrumente rechts** — Zeit, Schaden, Wirkungen
 * - **Boss in der Bildmitte** — hier noch als Balken; der Reaktionsgefäß-Ring
 *   entsteht in E1-5
 *
 * Ein Zeichenvorgang je Bild (GAME-DESIGN 18.2): `Surface.beginFrame` läuft genau
 * einmal, danach wird nur noch in dieselbe Fläche gezeichnet.
 */
import { FIGHT_OVERPRESSURE, FIGHT_WON } from '../core/combat.js';
import { TICK_MS } from '../core/loop.js';
import type { Session } from '../core/session.js';
import type { KeyLabels } from '../input/layout.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { BAR_LEFT, BAR_RIGHT, BLOCK_TOP, SpellBar } from './spellbar.js';
import { DESIGN_WIDTH, Surface } from './surface.js';

const BACKGROUND = '#0b0b0f';
const LETTERBOX = '#000000';
const INK = '#d5d5de';
const INK_DIM = '#61616e';
const HEAT_TRACK = '#1b1b24';
/** CALCINATION ist rot-orange (GAME-DESIGN 9.3). */
const HEAT_FILL = '#d4762e';
const HEAT_FILL_BONUS = '#f0a24a';
const HEAT_MARK = '#efe7d4';
const BOSS_TRACK = '#1b1b24';
const BOSS_FILL = '#a8442f';

const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_SMALL = '19px ui-monospace, "DejaVu Sans Mono", monospace';

const LABEL_HEAT = 'HEAT';
const LABEL_BOSS = 'THE FIRST CRUCIBLE';
const LABEL_TIME = 'time';
const LABEL_DAMAGE = 'damage';
const LABEL_CASTS = 'casts';
const LABEL_MISSED = 'misinputs';
const LABEL_STATE = 'state';
const STATE_NAMES: readonly string[] = ['fighting', 'defeated', 'overpressure'];

const BAR_WIDTH = BAR_RIGHT - BAR_LEFT;
/** Direkt über der Zauberleiste (GAME-DESIGN 9.1). */
const HEAT_HEIGHT = 26;
const HEAT_TOP = BLOCK_TOP - 66;
const BOSS_TOP = 300;
const BOSS_HEIGHT = 26;
const READOUT_RIGHT = DESIGN_WIDTH - 90;
const READOUT_LEFT = 1180;

export class HudRenderer {
  private digitAdvance = 16;
  private readonly spellBar: SpellBar;

  constructor(
    private readonly surface: Surface,
    private readonly session: Session,
    labels: KeyLabels,
  ) {
    this.spellBar = new SpellBar(session, labels);
    const ctx = surface.ctx;
    ctx.font = FONT_TEXT;
    this.digitAdvance = measureDigitAdvance(ctx);
    this.spellBar.measure(ctx);
  }

  render(simTimeMs: number, alpha: number): void {
    const ctx = this.surface.ctx;
    // Die anzuzeigende Zeit läuft zwischen den Takten weiter: Ohne das ruckten
    // Wischblende und Wartezeit bei 144 Hz sichtbar in Zehntel-Stufen.
    const shownMs = simTimeMs + alpha * TICK_MS;

    this.surface.beginFrame(BACKGROUND, LETTERBOX);
    ctx.textBaseline = 'alphabetic';

    this.drawBoss(ctx);
    this.drawHeat(ctx);
    this.spellBar.draw(ctx, shownMs);
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
    drawIntegerRight(ctx, combat.bossHpRemaining, BAR_RIGHT, BOSS_TOP - 14, this.digitAdvance);
  }

  /**
   * Die Hitze mit Markierung an der Bonus-Schwelle (GAME-DESIGN 9, STEP1-SPEC 1.4).
   *
   * Die Markierung bei 15 ist die wichtigste Linie des Bildes: Rechts davon
   * verdoppelt der Entlade-Zauber seinen Schaden. Damit sie nicht allein als Farbe
   * wirkt, wechselt der Balken jenseits davon zusätzlich die Helligkeit.
   */
  private drawHeat(ctx: CanvasRenderingContext2D): void {
    const combat = this.session.combat;
    const resource = combat.school.resource;
    ctx.textAlign = 'left';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_HEAT, BAR_LEFT, HEAT_TOP - 10);

    ctx.fillStyle = HEAT_TRACK;
    ctx.fillRect(BAR_LEFT, HEAT_TOP, BAR_WIDTH, HEAT_HEIGHT);

    const share = combat.resource / resource.cap;
    const bonus = combat.resource >= resource.highlightAt;
    ctx.fillStyle = bonus ? HEAT_FILL_BONUS : HEAT_FILL;
    ctx.fillRect(BAR_LEFT, HEAT_TOP, BAR_WIDTH * share, HEAT_HEIGHT);

    const markX = BAR_LEFT + BAR_WIDTH * (resource.highlightAt / resource.cap);
    ctx.fillStyle = HEAT_MARK;
    ctx.fillRect(markX - 1.5, HEAT_TOP - 7, 3, HEAT_HEIGHT + 14);

    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    drawIntegerRight(ctx, combat.resource, BAR_RIGHT, HEAT_TOP - 10, this.digitAdvance);
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
