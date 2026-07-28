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
import { TICK_MS } from '../core/loop.js';
import type { Session } from '../core/session.js';
import type { KeyLabels } from '../input/layout.js';
import { PHASE_COUNTDOWN, PHASE_ENDED, PHASE_FIGHTING, PHASE_NAMES } from '../core/phases.js';
import type { Overlay } from './overlay.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS, drawVessel } from './vessel.js';
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

const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_SMALL = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_VESSEL = '58px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TIMER = '46px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_COUNTDOWN = '140px ui-monospace, "DejaVu Sans Mono", monospace';

const LABEL_HEAT = 'HEAT';
const LABEL_BOSS = 'THE FIRST CRUCIBLE';
const LABEL_PERCENT = 'REACTION';
const LABEL_OVERPRESSURE = 'OVERPRESSURE IN';
const LABEL_TIME = 'time';
const LABEL_DAMAGE = 'damage';
const LABEL_CASTS = 'casts';
const LABEL_MISSED = 'misinputs';
const LABEL_STATE = 'state';

const BAR_WIDTH = BAR_RIGHT - BAR_LEFT;
/** Direkt über der Zauberleiste (GAME-DESIGN 9.1). */
const HEAT_HEIGHT = 26;
const HEAT_TOP = BLOCK_TOP - 66;
/** Oben links (GAME-DESIGN 9.1). */
const TIMER_LEFT = 72;
const TIMER_TOP = 96;
const READOUT_RIGHT = DESIGN_WIDTH - 90;
const READOUT_LEFT = 1180;

export class HudRenderer {
  private digitAdvance = 16;
  private digitAdvanceVessel = 32;
  private digitAdvanceTimer = 26;
  private digitAdvanceCountdown = 84;
  private readonly spellBar: SpellBar;
  /** Ein einziges, wiederverwendetes Objekt — im Bild wird nichts erzeugt. */
  private readonly vesselView: {
    hpFraction: number;
    overpressureRemainingMs: number;
    shownMs: number;
    finished: boolean;
  } = { hpFraction: 1, overpressureRemainingMs: 0, shownMs: 0, finished: false };

  /** Ergebnis-Werte der Auswertung. Wiederverwendet, damit nichts entsteht. */
  private readonly resultView = {
    outcome: 0,
    elapsedMs: 0,
    remainingMs: 0,
    damage: 0,
    bossHp: 0,
    casts: 0,
  };

  constructor(
    private readonly surface: Surface,
    private readonly session: Session,
    labels: KeyLabels,
    private readonly overlay: Overlay,
  ) {
    this.spellBar = new SpellBar(session, labels);
    const ctx = surface.ctx;
    ctx.font = FONT_TEXT;
    this.digitAdvance = measureDigitAdvance(ctx);
    ctx.font = FONT_VESSEL;
    this.digitAdvanceVessel = measureDigitAdvance(ctx);
    ctx.font = FONT_TIMER;
    this.digitAdvanceTimer = measureDigitAdvance(ctx);
    ctx.font = FONT_COUNTDOWN;
    this.digitAdvanceCountdown = measureDigitAdvance(ctx);
    this.spellBar.measure(ctx);
  }

  render(simTimeMs: number, alpha: number): void {
    const ctx = this.surface.ctx;
    // Die anzuzeigende Zeit läuft zwischen den Takten weiter: Ohne das ruckten
    // Wischblende und Wartezeit bei 144 Hz sichtbar in Zehntel-Stufen.
    const shownMs = simTimeMs + alpha * TICK_MS;

    this.surface.beginFrame(BACKGROUND, LETTERBOX);
    ctx.textBaseline = 'alphabetic';

    this.drawVesselAndTimer(ctx, shownMs);
    this.drawHeat(ctx);
    this.spellBar.draw(ctx, shownMs);
    this.drawReadouts(ctx, shownMs);
    this.drawCountdown(ctx, shownMs);
    this.syncOverlay(shownMs);
  }

  /**
   * Der Countdown vor dem Kampf (GAME-DESIGN 10).
   *
   * Auf der Zeichenfläche, nicht im HTML: Er gehört zum Kampf, nicht zum Menü,
   * und er läuft in der Zeit des Logik-Takts.
   */
  private drawCountdown(ctx: CanvasRenderingContext2D, shownMs: number): void {
    if (this.session.phase !== PHASE_COUNTDOWN) return;
    const remaining = this.session.countdownRemainingMs(shownMs);
    // Aufgerundet: Bei 2,4 s Restzeit steht 3, wie bei jedem Countdown.
    const shown = Math.ceil(remaining / 1000);
    ctx.textAlign = 'center';
    ctx.font = FONT_COUNTDOWN;
    ctx.fillStyle = INK;
    drawIntegerRight(
      ctx,
      shown,
      VESSEL_CENTER_X + this.digitAdvanceCountdown * 0.5,
      VESSEL_CENTER_Y + 40,
      this.digitAdvanceCountdown,
    );
    ctx.textAlign = 'left';
  }

  /** Sichtbarkeit der HTML-Ebene nachziehen. Fasst nur bei Phasenwechsel an. */
  private syncOverlay(shownMs: number): void {
    const combat = this.session.combat;
    const view = this.resultView;
    view.outcome = this.session.outcome;
    view.elapsedMs = this.session.fightElapsedMs(shownMs);
    view.remainingMs = this.session.overpressureRemainingMs(shownMs);
    view.damage = combat.damageDealt;
    view.bossHp = combat.boss.hp;
    view.casts = this.session.acceptedCount;
    this.overlay.update(this.session.phase, this.session.outcome, view);
  }

  /**
   * Das Reaktionsgefäß und der Überdruck-Timer (GAME-DESIGN 9, 9.1).
   *
   * Der Ring steht in der Bildmitte und ist Lebensbalken und Boss-Darstellung
   * zugleich; der Timer sitzt oben links.
   */
  private drawVesselAndTimer(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const combat = this.session.combat;
    const phase = this.session.phase;
    const fighting = phase === PHASE_FIGHTING;
    // Die Restzeit kommt aus dem Ablauf, nicht aus dem Kampf: volle Zeit in der
    // Vorbereitung, laufend im Kampf, stehend nach dem Ende. Die Auswertung und
    // die Anzeige müssen dieselbe Zahl nennen.
    const remaining = this.session.overpressureRemainingMs(shownMs);

    this.vesselView.hpFraction = combat.bossHpFraction;
    this.vesselView.overpressureRemainingMs = fighting ? remaining : Number.POSITIVE_INFINITY;
    this.vesselView.shownMs = shownMs;
    this.vesselView.finished = phase === PHASE_ENDED;
    drawVessel(ctx, this.vesselView);

    // Prozentzahl in der Mitte des Rings.
    ctx.textAlign = 'center';
    ctx.font = FONT_VESSEL;
    ctx.fillStyle = INK;
    const percent = combat.bossHpFraction * 100;
    drawFixedRight(
      ctx,
      percent,
      1,
      VESSEL_CENTER_X + this.digitAdvanceVessel * 2.6,
      VESSEL_CENTER_Y + 16,
      this.digitAdvanceVessel,
    );
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_PERCENT, VESSEL_CENTER_X, VESSEL_CENTER_Y + 48);
    ctx.fillText(LABEL_BOSS, VESSEL_CENTER_X, VESSEL_CENTER_Y - VESSEL_RADIUS - 26);
    ctx.textAlign = 'left';

    // Überdruck-Timer oben links.
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(LABEL_OVERPRESSURE, TIMER_LEFT, TIMER_TOP);
    ctx.font = FONT_TIMER;
    ctx.fillStyle = fighting && remaining < 10_000 ? HEAT_MARK : INK;
    drawFixedRight(
      ctx,
      remaining / 1000,
      2,
      TIMER_LEFT + 190,
      TIMER_TOP + 44,
      this.digitAdvanceTimer,
    );
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

  private drawReadouts(ctx: CanvasRenderingContext2D, shownMs: number): void {
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
    // Die Uhr des Spielers, nicht die der Zeichenschleife: null in der
    // Vorbereitung, stehend nach dem Ende.
    drawFixedRight(
      ctx,
      this.session.fightElapsedMs(shownMs) / 1000,
      2,
      READOUT_RIGHT,
      660,
      this.digitAdvance,
    );
    drawIntegerRight(ctx, combat.damageDealt, READOUT_RIGHT, 704, this.digitAdvance);
    drawIntegerRight(ctx, this.session.acceptedCount, READOUT_RIGHT, 748, this.digitAdvance);
    drawIntegerRight(ctx, this.session.rejectedCount, READOUT_RIGHT, 792, this.digitAdvance);

    // Der Zustand ist die PHASE des Ablaufs, nicht der interne Kampfzustand —
    // das ist die Auskunft, die zum Bild passt.
    ctx.textAlign = 'right';
    const phase = this.session.phase;
    ctx.fillStyle = phase === PHASE_ENDED ? HEAT_MARK : INK;
    ctx.fillText(PHASE_NAMES[phase] ?? '?', READOUT_RIGHT, 836);
    ctx.textAlign = 'left';
  }
}
