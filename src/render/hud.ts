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
import type { MarkDisplay, Overlay } from './overlay.js';
import type { RunSummary } from '../core/summary.js';
import type { Impact } from './impact.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS, drawVessel } from './vessel.js';
import { BAR_LEFT, BAR_RIGHT, BLOCK_TOP, SpellBar } from './spellbar.js';
import { DESIGN_WIDTH, Surface } from './surface.js';
import {
  loadoutFingerprint,
  mechanicsOfLoadout,
  visibleIndicators,
  type LoadoutSource,
} from '../combat/schools/indicators.js';
import { SLOT_COUNT } from '../input/keys.js';

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
/**
 * Die Rückmeldung, dass der Auswahlmodus scharf steht (Issue E3-3c).
 *
 * Englisch wie alle Spieltexte (Design-Gesetz 5). Sie sagt, was zu tun ist,
 * nicht was geschehen ist: Der Modus wartet auf eine Entscheidung, und ohne
 * diesen Satz wüsste der Spieler nicht, dass sein nächster Druck etwas
 * anderes bedeutet als sonst.
 */
const LABEL_RESET_ARMED = 'CHOOSE A SPELL TO RESET';
const LABEL_BOSS = 'THE FIRST CRUCIBLE';
const LABEL_PERCENT = 'REACTION';
const LABEL_OVERPRESSURE = 'OVERPRESSURE IN';
// Spieltexte englisch (Design-Gesetz 5).
const LABEL_DOOM = 'DOOM';
const LABEL_DOOM_CHARGES = 'CLEANSE';
/** Ab diesem Anteil der Schwelle warnt die Farbe. Vorlaeufig, wird gemessen. */
const THRESHOLD_WARN_AT = 0.7;
const THRESHOLD_WIDTH = 190;
const THRESHOLD_HEIGHT = 10;
const THRESHOLD_TRACK = 'rgba(255, 255, 255, 0.12)';
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
  /** Für welchen Ausgang die Kurzfassung schon gerechnet wurde. */
  private lastSummaryOutcome = -1;
  /** Fingerabdruck der Zusammenstellung, für den die Anzeigen gelten. */
  private indicatorFingerprint = -1;
  /** Die sichtbaren Schul-Anzeigen. Wird nur bei Änderung neu gerechnet. */
  private visibleCache: readonly string[] = [];
  /**
   * Woraus die Anzeigen-Auswahl liest — **ein einziges Objekt**, beim Aufbau
   * angelegt. Es je Bild zu bauen wäre Speicher im Bild.
   */
  private readonly mechanicSource: LoadoutSource;
  private readonly spellBar: SpellBar;
  /** Ein einziges, wiederverwendetes Objekt — im Bild wird nichts erzeugt. */
  private readonly vesselView: {
    hpFraction: number;
    overpressureRemainingMs: number;
    shownMs: number;
    finished: boolean;
    flash: number;
  } = { hpFraction: 1, overpressureRemainingMs: 0, shownMs: 0, finished: false, flash: 0 };

  /** Ergebnis-Werte der Auswertung. Wiederverwendet, damit nichts entsteht. */
  private readonly resultView: {
    outcome: number;
    elapsedMs: number;
    remainingMs: number;
    damage: number;
    bossHp: number;
    casts: number;
    summary: RunSummary | null;
    mark: MarkDisplay | null;
  } = {
    outcome: 0,
    elapsedMs: 0,
    remainingMs: 0,
    damage: 0,
    bossHp: 0,
    casts: 0,
    summary: null,
    mark: null,
  };

  constructor(
    private readonly surface: Surface,
    private readonly session: Session,
    labels: KeyLabels,
    private readonly overlay: Overlay,
    private readonly impact: Impact,
    /**
     * Woher die Fortschrittsmarke kommt (Issue #187).
     *
     * Als Quelle und nicht als Wert: Die Anzeige fragt beim Wechsel in die
     * Auswertung, statt dass jemand sie hineinreicht und dabei vergisst, sie
     * nachzuziehen. Dasselbe Muster wie `MetricsSource` im Trainingsraum --
     * und die Vorgabe `null` haelt jeden bestehenden Aufruf gueltig.
     */
    private readonly markSource: () => MarkDisplay | null = () => null,
  ) {
    this.mechanicSource = {
      get resourceId(): string {
        return session.combat.resourceId;
      },
      slotCount: SLOT_COUNT,
      spellOnSlot: (slot) => session.spellOnSlot(slot),
      gainOfSpell: (spell, id) => session.combat.gainOfSpell(spell, id),
      gainOfTick: (spell, id) => session.combat.gainOfTick(spell, id),
    };
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
    this.impact.measure(ctx);
  }

  render(simTimeMs: number, alpha: number): void {
    const ctx = this.surface.ctx;
    // Die anzuzeigende Zeit läuft zwischen den Takten weiter: Ohne das ruckten
    // Wischblende und Wartezeit bei 144 Hz sichtbar in Zehntel-Stufen.
    const shownMs = simTimeMs + alpha * TICK_MS;

    // Das Bild-Ruckeln verschiebt die ganze Fläche (GAME-DESIGN 9).
    this.surface.beginFrame(BACKGROUND, LETTERBOX, this.impact.shakeOffset(shownMs), 0);
    ctx.textBaseline = 'alphabetic';

    this.drawVesselAndTimer(ctx, shownMs);
    this.impact.draw(ctx, shownMs);
    this.drawHeat(ctx);
    this.drawResetPrompt(ctx);
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
    // Die Kurzfassung wird nur beim Wechsel in die Auswertung gerechnet, nicht
    // je Bild: Sie erzeugt Speicher und darf das auch (GAME-DESIGN 15.1).
    view.summary =
      this.session.phase === PHASE_ENDED && this.session.outcome !== this.lastSummaryOutcome
        ? this.session.summary(shownMs)
        : view.summary;
    // Die Marke wird beim selben Wechsel geholt wie die Kurzfassung -- nicht
    // je Bild. Sie aendert sich waehrend der Auswertung nicht.
    if (this.session.phase === PHASE_ENDED && this.session.outcome !== this.lastSummaryOutcome) {
      view.mark = this.markSource();
    }
    if (this.session.phase === PHASE_ENDED) this.lastSummaryOutcome = this.session.outcome;
    else this.lastSummaryOutcome = -1;
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
    this.vesselView.flash = this.impact.flashStrength(shownMs);
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

    this.drawDoom(ctx);

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
   * Die Rückmeldung des Auswahlmodus (Issue E3-3c).
   *
   * Sie steht über der Leiste, weil der Blick beim Wählen dort ist — und sie
   * verschwindet mit dem nächsten Druck, gleich wie er ausgeht.
   */
  private drawResetPrompt(ctx: CanvasRenderingContext2D): void {
    if (!this.session.isResetArmed) return;
    ctx.textAlign = 'left';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = HEAT_FILL_BONUS;
    ctx.fillText(LABEL_RESET_ARMED, BAR_LEFT, BLOCK_TOP - 18);
  }

  /**
   * Eine Leiste für „ein Wert steigt auf eine Schwelle zu" (Issue #216).
   *
   * DIE FORM IST NICHT FÜR EINEN BAUSTEIN GEBAUT
   * --------------------------------------------
   * Von den neun Bausteinen aus 11.2 haben mindestens zwei dieselbe Gestalt:
   * der tödliche Zähler (Baustein 2) und die Energieleiste (Baustein 1). Beide
   * steigen von selbst auf einen Punkt zu, an dem etwas geschieht — was sie
   * unterscheidet, ist die Folge, nicht die Anzeige.
   *
   * Zwei verschiedene Darstellungen für dieselbe Sache wären eine Regel mehr,
   * die der Spieler lernen muss; 11.5 verlangt dieselben Kategorien überall.
   * Deshalb nimmt diese Funktion Beschriftung, Stand und Schwelle entgegen und
   * kennt keinen der beiden Bausteine.
   *
   * **Die Energieleiste ist hier nicht angeschlossen** (Issue #219). Sie hätte
   * einen zweiten Aufruf gebraucht, und der gehört in das Issue, das sie
   * sichtbar macht — nicht in dieses.
   *
   * Die Zahl steht neben dem Balken, nicht nur im Balken: Bei einer Schwelle,
   * die tötet, ist „wie weit noch" eine Rechnung und keine Schätzung.
   *
   * @param zusatz Was rechts neben der Zahl steht, oder `null`. Beim Zähler
   *   sind es die verbleibenden Anwendungen — ohne sie ist er eine Zahl, die
   *   steigt, und mit ihnen eine Entscheidung.
   */
  private drawThresholdBar(
    ctx: CanvasRenderingContext2D,
    top: number,
    label: string,
    stand: number,
    schwelle: number,
    zusatz: { text: string; dringend: boolean } | null,
  ): void {
    const anteil = schwelle > 0 ? Math.min(1, stand / schwelle) : 0;

    ctx.textAlign = 'left';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(label, TIMER_LEFT, top);

    ctx.fillStyle = THRESHOLD_TRACK;
    ctx.fillRect(TIMER_LEFT, top + 10, THRESHOLD_WIDTH, THRESHOLD_HEIGHT);
    // Die Warnfarbe kommt aus dem STAND, nicht aus der Zeit: Wer früh viel
    // abbekommen hat, ist früh in Gefahr.
    ctx.fillStyle = anteil >= THRESHOLD_WARN_AT ? HEAT_MARK : INK;
    ctx.fillRect(TIMER_LEFT, top + 10, THRESHOLD_WIDTH * anteil, THRESHOLD_HEIGHT);

    const zeileY = top + 10 + THRESHOLD_HEIGHT + 20;
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    // `drawIntegerRight` und nicht `drawFixedRight` mit null Nachkommastellen:
    // Letzteres zeichnet den Punkt trotzdem, und der Zähler stand als „60." da.
    // Im Bild gesehen, nicht im Code.
    drawIntegerRight(ctx, stand, TIMER_LEFT + THRESHOLD_WIDTH + 44, zeileY, this.digitAdvance);

    if (zusatz === null) return;
    ctx.font = FONT_SMALL;
    ctx.fillStyle = zusatz.dringend ? HEAT_MARK : INK_DIM;
    ctx.fillText(zusatz.text, TIMER_LEFT + THRESHOLD_WIDTH + 56, zeileY);
  }

  /**
   * Der Zähler, der bei seiner Schwelle tötet (11.2, Baustein 2; Issue #216).
   *
   * WARUM ER ÜBERHAUPT DASTEHEN MUSS
   * --------------------------------
   * 11.2 sagt es selbst: „muss gut angekündigt sein — wer ihn verpasst, ist
   * meist sofort tot." Eine tödliche Schwelle ohne Anzeige ist die Sorte
   * Schwierigkeit, die 11.7 ausdrücklich verwirft: Information vorzuenthalten
   * ist die billigste Form davon.
   *
   * ER ERSCHEINT NUR, WENN ES IHN GIBT
   * ----------------------------------
   * Kein Boss trägt den Baustein heute. Ein leerer Balken für alle anderen
   * wäre eine Anzeige, die nie etwas anzeigt — und der Blick lernt, sie zu
   * überspringen. Genau dann steht sie umsonst da, wenn sie einmal zählt.
   */
  private drawDoom(ctx: CanvasRenderingContext2D): void {
    const doom = this.session.combat.boss.doom;
    if (doom === null) return;
    const ladungen = this.session.combat.doomCharges;
    this.drawThresholdBar(
      ctx,
      TIMER_TOP + 78,
      LABEL_DOOM,
      this.session.combat.doom,
      doom.threshold,
      { text: `${LABEL_DOOM_CHARGES} ${ladungen}`, dringend: ladungen === 0 },
    );
  }

  /**
   * Welche Schul-Anzeigen erscheinen (Issue E3-5).
   *
   * **Die Liste wird nur neu gerechnet, wenn sich die Zusammenstellung
   * ändert.** Sie zu erzeugen kostet Speicher, und im Bild darf nichts
   * entstehen (GAME-DESIGN 18.2) — der Fingerabdruck davor ist eine Zahl und
   * kostet nichts. Ein Umbelegen wirkt trotzdem sofort.
   */
  private visibleMechanics(): readonly string[] {
    const fingerprint = loadoutFingerprint(this.mechanicSource);
    if (fingerprint !== this.indicatorFingerprint) {
      this.indicatorFingerprint = fingerprint;
      this.visibleCache = visibleIndicators(mechanicsOfLoadout(this.mechanicSource));
    }
    return this.visibleCache;
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
    // Nur die aktiv genutzten Mechaniken (GAME-DESIGN 5, Issue E3-5). Wer
    // keine Kalzination mitführt, sieht keine Hitze-Anzeige — eine Zahl, die
    // sich nie bewegt, ist schlimmer als keine.
    if (!this.visibleMechanics().includes(combat.resourceId)) return;
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
