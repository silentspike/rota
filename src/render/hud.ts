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
import { heldOf, progressOf } from '../core/summary.js';
import { analyzeRun, interruptDataFor, runDataFor } from '../analysis/run-analysis.js';
import type { LossReport } from '../analysis/loss-sources.js';
import type { Combat } from '../core/combat.js';
import { GCD_MS } from '../core/gcd.js';
import { TICK_MS } from '../core/loop.js';
import type { Session } from '../core/session.js';
import type { KeyLabels } from '../input/layout.js';
import { PHASE_COUNTDOWN, PHASE_ENDED, PHASE_FIGHTING, PHASE_NAMES } from '../core/phases.js';
import type { MarkDisplay, Overlay } from './overlay.js';
import type { RunSummary } from '../core/summary.js';
import type { Impact } from './impact.js';
import { drawFixedRight, drawIntegerRight, measureDigitAdvance } from './numerals.js';
import { MAX_DECAY_STACKS } from '../core/combat.js';
import { VESSEL_CENTER_X, VESSEL_CENTER_Y, VESSEL_RADIUS, drawVessel } from './vessel.js';
import { BAR_LEFT, BAR_RIGHT, BLOCK_TOP, SpellBar } from './spellbar.js';
import { DESIGN_WIDTH, Surface } from './surface.js';
import {
  feedsMechanic,
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
/**
 * Der Zauberbalken (Issue #304). Ein anderer Farbton als die Hitze und als der
 * Wartezeit-Streifen: Drei Anzeigen in derselben Farbe wären drei Aussagen, die
 * wie eine aussehen — und die spielkritische Information darf ohnehin nicht
 * allein an der Farbe hängen (GAME-DESIGN 21.2), weshalb Ort und Wuchsrichtung
 * den Unterschied tragen.
 */
const CAST_BAR_FILL = '#8fb4d8';
/**
 * Der Boss-Balken (Issue #327). Warm gegen das kühle Blau des Spieler-Balkens
 * — aber die Farbe ist die dritte Unterscheidung, nicht die erste: Ort (Ring
 * gegen Leiste) und Richtung (ablaufend gegen zulaufend) tragen die Aussage
 * schon allein (GAME-DESIGN 21.2).
 */
const BOSS_BAR_FILL = '#c0603a';
const HEAT_FILL_BONUS = '#f0a24a';
const HEAT_MARK = '#efe7d4';

const FONT_TEXT = '24px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_SMALL = '19px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_VESSEL = '58px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_TIMER = '46px ui-monospace, "DejaVu Sans Mono", monospace';
const FONT_COUNTDOWN = '140px ui-monospace, "DejaVu Sans Mono", monospace';

const LABEL_HEAT = 'HEAT';
/** Die zwei Mechaniken, die seit #283 danebenstehen (GAME-DESIGN 5). */
const LABEL_CONTROL = 'CONTROL';
const LABEL_DECAY = 'DECAY';
/**
 * Die Rückmeldung, dass der Auswahlmodus scharf steht (Issue E3-3c).
 *
 * Englisch wie alle Spieltexte (Design-Gesetz 5). Sie sagt, was zu tun ist,
 * nicht was geschehen ist: Der Modus wartet auf eine Entscheidung, und ohne
 * diesen Satz wüsste der Spieler nicht, dass sein nächster Druck etwas
 * anderes bedeutet als sonst.
 */
const LABEL_RESET_ARMED = 'CHOOSE A SPELL TO RESET';

const LABEL_PERCENT = 'REACTION';
const LABEL_OVERPRESSURE = 'OVERPRESSURE IN';
// Spieltexte englisch (Design-Gesetz 5).
const LABEL_DOOM = 'DOOM';
const LABEL_ENERGY = 'ENERGY';
const LABEL_DOOM_CHARGES = 'CLEANSE';
// Spieltexte englisch (Design-Gesetz 5).
const LABEL_RACE = 'IMMUNE — BREAK THE SEAL';
const LABEL_RACE_HINT = 'no damage lands';
/** Ab diesem Anteil der Schwelle warnt die Farbe. Vorlaeufig, wird gemessen. */
const THRESHOLD_WARN_AT = 0.7;
const THRESHOLD_WIDTH = 190;
const THRESHOLD_HEIGHT = 10;
/**
 * Abstand zweier gestapelter Schwellen-Leisten.
 *
 * Abgeleitet aus der Höhe einer Leiste, nicht daneben gesetzt: 10 für die
 * Beschriftung, die Leiste selbst, 20 bis zur Zahlenzeile und 24 Luft.
 * Ändert jemand `THRESHOLD_HEIGHT`, wandert der Abstand mit.
 */
const THRESHOLD_STACK_STEP = 10 + THRESHOLD_HEIGHT + 20 + 24;
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
/**
 * Die Zahlen stehen ÜBER dem Hitze-Balken, nicht darunter (Issue #283).
 *
 * Darunter läge die Zauberleiste; 5 verlangt „direkt über der Zauberleiste",
 * und drei Anzeigen brauchen dort zwei Zeilen. Die obere trägt die Zahlen,
 * weil sie schmaler sind als der Balken.
 */
const MECHANIC_TOP = HEAT_TOP - 54;
/**
 * Der Zauberbalken (GAME-DESIGN 7.1, Issue #304).
 *
 * ÜBER der Zahlenzeile und links vom Ziel: Das Band zwischen dem unteren Rand
 * des Rings (Entwurfs-y 522 bei x 774 bis 1146) und der Mechanik-Zeile ist die
 * einzige Fläche in Blickrichtung, die frei ist — nachgemessen an einem
 * Screenshot und nicht geschätzt. Die Breite endet deshalb vor dem Ring.
 *
 * **Die VIERTE Zeitanzeige**, und sie muss von den drei in `spellbar.ts` klar
 * unterscheidbar sein (Abklingzeit als Wischblende, Wartezeit als Streifen,
 * Wirkungsdauer als Kantenbalken). Sie ist die einzige, die WÄCHST statt zu
 * schrumpfen, sie trägt einen Namen, und sie steht an einem anderen Ort.
 */
const CAST_BAR_TOP = MECHANIC_TOP - 92;
/**
 * Der Balken des BOSS-Zaubers (GAME-DESIGN 11.2 Baustein 9, Issue #327).
 *
 * **Im Ring, unter der Beschriftung.** Er gehört zum ZIEL, nicht zur Leiste —
 * das ist der Unterschied zum Balken des Spielers, und er ist der Grund für
 * den Ort: Der eine zeigt, was ICH tue und abwarte; der andere, was der BOSS
 * tut und wogegen ich etwas tun kann.
 *
 * Die Fläche ist nachgemessen und nicht geraten: Unter dem Ring liegt der
 * Hitze-Balken (72 bis 1032 bei y 550), links davon die Mechanik-Zeile, unten
 * links der Spieler-Balken. Der Ring-Innenraum ist die einzige freie Fläche,
 * die zugleich am Ziel liegt.
 *
 * **Zwei Zeitanzeigen, zwei Gestalten** (GAME-DESIGN 21.2, die spielkritische
 * Aussage hängt nicht an der Farbe): Der Spieler-Balken ist breit und liegt
 * unten links, dieser ist schmal und liegt in der Mitte des Bildes.
 */
const BOSS_BAR_WIDTH = 208;
const BOSS_BAR_HEIGHT = 20;
/**
 * **Nachgemessen und einmal korrigiert.** Mein erster Entwurf setzte den
 * Balken auf +74; im Bild klebte die Beschriftung dann unter „REACTION" (bei
 * +48) und die zwei lasen sich als ein zweizeiliger Block. Jetzt liegt die
 * Zeile bei +76 und der Balken darunter — der Ring-Innenraum reicht bis etwa
 * +150, es ist also Platz da, den der erste Entwurf nur nicht genutzt hat.
 */
const BOSS_BAR_TOP = VESSEL_CENTER_Y + 84;
const BOSS_BAR_LEFT = VESSEL_CENTER_X - BOSS_BAR_WIDTH * 0.5;
const CAST_BAR_HEIGHT = 34;
const CAST_BAR_WIDTH = 560;
const MECHANIC_GAP = 150;
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
    heldPercent: number | null;
    analysis: LossReport | null;
    damage: number;
    progressPercent: number;
    casts: number;
    summary: RunSummary | null;
    mark: MarkDisplay | null;
  } = {
    outcome: 0,
    elapsedMs: 0,
    remainingMs: 0,
    heldPercent: null,
    analysis: null,
    damage: 0,
    progressPercent: 0,
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
      slotCount: SLOT_COUNT,
      spellOnSlot: (slot) => session.spellOnSlot(slot),
      // **Dieselbe Entscheidung wie in der Zusammenstellungs-Ansicht** — sie
      // steht in `feedsMechanic` und wird hier nur angewandt. Vorher lagen
      // hier zwei Abfragen, die nur den Vorrat der Schule kannten; mit
      // `control` und `decay` wären es vier geworden (Issue #270).
      feedsMechanic: (spell, mechanicId) => {
        const def = session.combat.spellAt(spell);
        return def !== undefined && feedsMechanic(def, mechanicId);
      },
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
    this.drawMechanicNumbers(ctx);
    this.drawBossCastBar(ctx, shownMs);
    this.drawCastBar(ctx, shownMs);
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

  /**
   * Die Fehleranalyse eines beendeten Laufs (Issue #280).
   *
   * **824 Zeilen lagen fertig da und liefen im Spiel nie.** `analyzeRun`,
   * `rankLosses` und `buildLossView` hatten genau einen Aufrufer: das
   * Prüfblatt aus #81. Der Spieler sah unter „View Analysis" eine Rohliste je
   * Zauber und sonst nichts — während GAME-DESIGN 15.3 die drei größten
   * Verlustquellen mit Prozentanteil als Kern der Ansicht nennt.
   *
   * Die Eingaben kommen aus dem Kampf, nicht aus der Kurzfassung: `runDataFor`
   * braucht die tatsächlich belegten Plätze, `interruptDataFor` die
   * Zeitleiste des Bosses.
   */
  private analysisOf(combat: Combat, summary: RunSummary | null): LossReport | null {
    if (summary === null) return null;
    const belegte: number[] = [];
    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const spell = this.session.spellOnSlot(slot);
      if (spell >= 0) belegte.push(spell);
    }
    const interrupts = interruptDataFor(combat);
    return analyzeRun({
      summary,
      gcdMs: GCD_MS,
      spells: runDataFor(combat, summary, belegte),
      synergyBonusDamage: 0,
      missedInterrupts: interrupts.missed,
      damagePerLandedBossCast: interrupts.damagePerLandedCast,
      // Was der Boss zurückgedreht hat — aus dem Kampf, nicht aus der
      // Kurzfassung: Es ist keine Kennzahl des Spielers (Issue #218).
      bossHealed: combat.bossHealed,
    }).losses;
  }

  /** Sichtbarkeit der HTML-Ebene nachziehen. Fasst nur bei Phasenwechsel an. */
  private syncOverlay(shownMs: number): void {
    const combat = this.session.combat;
    const view = this.resultView;
    view.outcome = this.session.outcome;
    view.elapsedMs = this.session.fightElapsedMs(shownMs);
    view.remainingMs = this.session.overpressureRemainingMs(shownMs);
    // Bei einem Boss mit umgekehrtem Ziel trennt der Ring-Rest die Siege, nicht
    // die Reserve — die ist dort immer null (Issue #256).
    view.heldPercent = heldOf(combat);
    view.damage = combat.damageDealt;
    // **Fertig gerechnet statt als Zutat.** Die Anzeige bekam bis zum
    // 2026-08-05 `bossHp` und rechnete daraus selbst — eine zweite Stelle für
    // die Entscheidung aus #256, die im Browser 100 % zeigte, wo die
    // Kurzfassung 85 % meldete.
    view.progressPercent = progressOf(combat, view.elapsedMs);
    view.casts = this.session.acceptedCount;
    // Die Kurzfassung wird nur beim Wechsel in die Auswertung gerechnet, nicht
    // je Bild: Sie erzeugt Speicher und darf das auch (GAME-DESIGN 15.1).
    view.summary =
      this.session.phase === PHASE_ENDED && this.session.outcome !== this.lastSummaryOutcome
        ? this.session.summary(shownMs)
        : view.summary;
    // **NACH der Kurzfassung, nicht davor** — und das war ein Befund im
    // Browser: Meine erste Fassung stand oben bei `heldPercent`, also bevor
    // `view.summary` gesetzt war. `analysisOf` bekam `null` und gab `null`
    // zurück; die Rangliste erschien nie, während alle Prüffälle grün waren.
    //
    // **Die Fehleranalyse wird HIER gerechnet, nicht im Overlay** (Issue #280).
    //
    // Sie braucht den `Combat` (für `runDataFor` und `interruptDataFor`), das
    // Overlay hat nur den `ResultView`. Ihm den Kampf zu geben hieße, dass es
    // irgendwann selbst rechnet — und das ist genau die zweite Stelle, die
    // #256 dort entfernt hat. Sie hat an einem Tag zwei Befunde gekostet.
    //
    // Einmal beim Wechsel, nicht je Bild: Die Rechnung erzeugt Speicher und
    // darf das auch (GAME-DESIGN 18.2), aber nicht sechzig Mal in der
    // Sekunde. Dieselbe Regel wie bei der Kurzfassung.
    view.analysis =
      this.session.phase === PHASE_ENDED && this.session.outcome !== this.lastSummaryOutcome
        ? this.analysisOf(combat, view.summary)
        : view.analysis;
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
    // **Der Name kommt aus dem Boss, nicht aus einer Konstante.**
    //
    // Hier stand bis #229 eine Konstante `LABEL_BOSS` mit dem Namen des
    // ersten Tiegels, und dieselbe Zeichenkette ein zweites Mal in
    // `overlay.ts`. Solange nur ein Boss spielbar war, stimmte sie zufällig —
    // mit dem Startparameter aus #219 stand über dem Testboss der falsche
    // Name.
    //
    // **Der Name wird hier bewusst nicht ausgeschrieben.** Ein Kommentar, der
    // ihn zitiert, ist selbst eine Abschrift — und `boss-name-source.test.ts`
    // hat genau diese zwei Kommentare gefangen, als sie noch dastanden.
    //
    // Das ist die gefährlichere Sorte Handabschrift: Sie erzeugt keinen
    // Pflegeaufwand, sondern führt eine MESSUNG in die Irre. Wer den Boss
    // wechselt und den vertrauten Namen sieht, hält den Wechsel für
    // gescheitert — oder, schlimmer, hält einen echten Fehlschlag für
    // gelungen.
    //
    // Großbuchstaben hier statt in den Daten: Sie sind eine Eigenschaft der
    // ANZEIGE (9.3, dieselbe Schreibweise wie alle Beschriftungen), nicht des
    // Bosses. Ein Datensatz, der seinen Namen in Versalien führte, wäre in
    // jeder anderen Verwendung falsch.
    ctx.fillText(
      this.session.combat.boss.name.toUpperCase(),
      VESSEL_CENTER_X,
      VESSEL_CENTER_Y - VESSEL_RADIUS - 26,
    );
    ctx.textAlign = 'left';

    // **Die Leisten stapeln sich, statt feste Plätze zu haben.**
    //
    // Heute trägt kein Boss beide (`test-vessel` hat die Energieleiste, keiner
    // den Zähler), und mit festen Plätzen fiele das nicht auf — bis der erste
    // Boss beides hat und zwei Leisten übereinander stünden. Der Stapel kostet
    // eine Zeile und kann nicht schiefgehen.
    let leistenTop = TIMER_TOP + 78;
    leistenTop = this.drawDoom(ctx, leistenTop);
    leistenTop = this.drawRace(ctx, leistenTop);
    this.drawEnergy(ctx, leistenTop);

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
   * Der laufende Zauber des BOSSES (GAME-DESIGN 11.2 Baustein 9, Issue #327).
   *
   * # Warum es ihn bis heute nicht gab
   *
   * Das Unterbrechen ist seit #110 gebaut: Die Zeitleiste startet den Zauber,
   * die Eskalation verkürzt sein Fenster (#240), FIXATION verlängert es
   * (#100), QUENCH bricht ihn ab, und beide Größen gehen in die Prüfsumme ein.
   * Nur las sie niemand außer der Prüfsumme — gemessen: zwei Fundstellen von
   * `combat.casting` im ganzen Quelltext, beide in `checksum.ts`.
   *
   * **Das verletzt eine Design-Anforderung**, nicht nur eine Bequemlichkeit.
   * GAME-DESIGN 7.1 verlangt für den Abbruch „vorher **erkennbare**
   * Boss-Angriffe", und 21.2 nimmt die Reaktionszeit ausdrücklich aus der
   * Bewertung: Ein Fenster, das man nicht sieht, ist ein Reaktionstest auf ein
   * unsichtbares Ereignis.
   *
   * # Er läuft ab, der des Spielers läuft zu
   *
   * Beim eigenen Zauber wartet man auf das Ende — der Balken füllt sich. Hier
   * ist das Ende die Gefahr: Der Balken **leert** sich, und was übrig ist, ist
   * die Zeit zum Handeln. Dieselbe Regel wie bei den drei Zeitanzeigen der
   * Zauberleiste, wo ablaufende Zeit ablaufend gezeigt wird.
   */
  private drawBossCastBar(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const combat = this.session.combat;
    if (combat.casting < 0) return;
    const name = combat.castingName;
    if (name === null) return;

    // **Kampfzeit, nicht Uhrzeit.** `castLandsAtMs` und `castingStartedAtMs`
    // zählen ab Kampfbeginn; `shownMs` ist die Uhr des Spiels. Wer die zwei
    // mischt, bekommt einen Balken, der um die Vorbereitungszeit versetzt ist.
    const jetzt = this.session.fightElapsedMs(shownMs);
    const anfang = combat.castingStartedAtMs;
    const ende = combat.castLandsAtMs;
    const dauer = ende - anfang;
    const restMs = ende - jetzt;
    // Der Anteil, der NOCH bleibt. Bei einer Verschiebung durch STALL wächst
    // `ende` mitten im Lauf — der Balken wird dadurch wieder länger, und genau
    // das soll man sehen.
    const anteil = dauer <= 0 ? 0 : restMs <= 0 ? 0 : restMs >= dauer ? 1 : restMs / dauer;

    // **Dieselbe Anordnung wie beim Balken des Spielers**: Name links, Restzeit
    // rechts, beides ÜBER dem Balken. Zwei Zeitanzeigen, die verschiedene
    // Dinge zeigen, sollen sich in Ort und Richtung unterscheiden — nicht in
    // der Art, wie man sie liest.
    ctx.textAlign = 'left';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(name.toUpperCase(), BOSS_BAR_LEFT, BOSS_BAR_TOP - 8);

    // Die Restzeit ist die Zahl, nach der gehandelt wird — „reicht meine
    // Abklingzeit noch?"
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    drawFixedRight(
      ctx,
      (restMs > 0 ? restMs : 0) / 1000,
      2,
      BOSS_BAR_LEFT + BOSS_BAR_WIDTH,
      BOSS_BAR_TOP - 8,
      this.digitAdvance,
    );

    ctx.fillStyle = THRESHOLD_TRACK;
    ctx.fillRect(BOSS_BAR_LEFT, BOSS_BAR_TOP, BOSS_BAR_WIDTH, BOSS_BAR_HEIGHT);
    ctx.fillStyle = BOSS_BAR_FILL;
    ctx.fillRect(BOSS_BAR_LEFT, BOSS_BAR_TOP, BOSS_BAR_WIDTH * anteil, BOSS_BAR_HEIGHT);
  }

  /**
   * Der laufende Zauber des Spielers (GAME-DESIGN 7.1, Issue #304).
   *
   * > **Blockiert.** Während der Balken läuft, kann nichts anderes gewirkt
   * > werden. […] Erzeugt eine echte Verpflichtung: „Kann ich mir das gerade
   * > leisten?"
   *
   * Diese Verpflichtung braucht ein Bild. Ohne den Balken wüsste der Spieler
   * nicht, wie lange er noch gebunden ist — und die Sperre wirkte wie ein
   * Fehler des Spiels statt wie eine Entscheidung, die er getroffen hat.
   *
   * **Er wächst nach rechts**, im Gegensatz zu allen anderen Zeitanzeigen im
   * Spiel: Abklingzeit, Wartezeit und Wirkungsdauer laufen ab, dieser läuft
   * zu. Das ist der Unterschied zwischen „noch nicht wieder" und „gleich".
   *
   * Er verschwindet mit dem Ende — auch beim Abbruch, und zwar sofort. Ein
   * Balken, der nach einem abgebrochenen Zauber stehen bliebe, behauptete eine
   * Wirkung, die nicht mehr kommt.
   */
  private drawCastBar(ctx: CanvasRenderingContext2D, shownMs: number): void {
    const combat = this.session.combat;
    const index = combat.playerCasting;
    if (index < 0) return;
    const spell = combat.spellAt(index);
    if (spell === undefined) return;

    const anteil = combat.playerCastProgress(shownMs);
    const restMs = combat.playerCastEndsAtMs - shownMs;

    ctx.textAlign = 'left';
    ctx.font = FONT_SMALL;
    ctx.fillStyle = INK_DIM;
    ctx.fillText(spell.name.toUpperCase(), BAR_LEFT, CAST_BAR_TOP - 10);

    ctx.fillStyle = THRESHOLD_TRACK;
    ctx.fillRect(BAR_LEFT, CAST_BAR_TOP, CAST_BAR_WIDTH, CAST_BAR_HEIGHT);
    ctx.fillStyle = CAST_BAR_FILL;
    ctx.fillRect(BAR_LEFT, CAST_BAR_TOP, CAST_BAR_WIDTH * anteil, CAST_BAR_HEIGHT);

    // Die Restzeit in Sekunden, rechtsbündig am Balkenende. Zwei Nachkommastellen
    // wie bei der Frist oben links — dieselbe Größe, dieselbe Schreibweise.
    ctx.font = FONT_TEXT;
    ctx.fillStyle = INK;
    drawFixedRight(
      ctx,
      (restMs > 0 ? restMs : 0) / 1000,
      2,
      BAR_LEFT + CAST_BAR_WIDTH,
      CAST_BAR_TOP - 10,
      this.digitAdvance,
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
  private drawDoom(ctx: CanvasRenderingContext2D, top: number): number {
    const doom = this.session.combat.boss.doom;
    if (doom === null) return top;
    const ladungen = this.session.combat.doomCharges;
    this.drawThresholdBar(ctx, top, LABEL_DOOM, this.session.combat.doom, doom.threshold, {
      text: `${LABEL_DOOM_CHARGES} ${ladungen}`,
      dringend: ladungen === 0,
    });
    return top + THRESHOLD_STACK_STEP;
  }

  /**
   * Das Unverwundbarkeits-Fenster (11.2, Baustein 4; Issue #218).
   *
   * **Der Spieler muss sehen, dass er gegen eine Wand schlägt.** Sonst hält er
   * die ausbleibende Wirkung für einen Rechenfehler — und Information
   * vorzuenthalten ist die billigste Form von Schwierigkeit (11.7).
   *
   * Es nutzt **dieselbe Schwellen-Leiste** wie Zähler und Energieleiste. Der
   * geladene Zähler ist „ein Wert steigt auf eine Schwelle zu", genau wie
   * jene; zwei Darstellungen für dieselbe Sache wären eine Regel mehr, die der
   * Spieler lernen muss (11.5).
   *
   * Sie erscheint **nur, solange das Fenster offen ist** — anders als die
   * beiden anderen, die den ganzen Kampf über stehen. Danach hat sie nichts
   * mehr zu sagen, und der Stapel rückt wieder zusammen.
   */
  private drawRace(ctx: CanvasRenderingContext2D, top: number): number {
    const race = this.session.combat.boss.race;
    if (race === null || !this.session.combat.raceOpen) return top;
    this.drawThresholdBar(ctx, top, LABEL_RACE, this.session.combat.raceProgress, race.threshold, {
      text: LABEL_RACE_HINT,
      dringend: true,
    });
    return top + THRESHOLD_STACK_STEP;
  }

  /**
   * Die Energieleiste des Bosses (11.2, Baustein 1; Issue #219).
   *
   * DIE ZWEITE UHR, DIE MAN SEHEN MUSS
   * ----------------------------------
   * 11.2 nennt sie „unabhängig vom Schaden des Spielers" — sie läuft nach
   * ihrem eigenen Takt, und wer schneller zuschlägt, verschiebt sie nicht.
   * Genau deshalb ist sie eine Planungsgröße: Man weiß, wann etwas kommt, und
   * richtet die Rotation danach aus. Ohne Anzeige bleibt davon nur Überraschung
   * übrig, und 11.7 verwirft ausdrücklich, Information vorzuenthalten.
   *
   * SIE ERSCHEINT NUR, WENN ES SIE GIBT
   * -----------------------------------
   * **Sechs von sieben Bossen haben keine.** Ein leerer Balken bei allen
   * anderen wäre schlimmer als keiner: Der Blick lernt, ihn zu überspringen —
   * und übersieht ihn dann bei dem einen Boss, bei dem er zählt.
   *
   * DIESELBE FORM WIE DER ZÄHLER, UND ZWAR ABSICHTLICH
   * --------------------------------------------------
   * Beide sind „ein Wert steigt auf eine Schwelle zu". Was sie unterscheidet,
   * ist die Folge, nicht die Anzeige (11.5: dieselben Kategorien überall).
   * `drawThresholdBar` kennt keinen der beiden Bausteine, und dieser Aufruf
   * fügt ihr nichts hinzu.
   *
   * **Kein Zusatz rechts der Zahl.** Beim Zähler stehen dort die verbleibenden
   * Anwendungen, weil sie eine Entscheidung tragen. Die Energieleiste hat
   * nichts Vergleichbares — ein leeres Feld wäre eine Frage, auf die es keine
   * Antwort gibt.
   */
  private drawEnergy(ctx: CanvasRenderingContext2D, top: number): number {
    const energy = this.session.combat.boss.energy;
    if (energy === null) return top;
    this.drawThresholdBar(
      ctx,
      top,
      LABEL_ENERGY,
      this.session.combat.bossEnergy,
      energy.threshold,
      null,
    );
    return top + THRESHOLD_STACK_STEP;
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

  /**
   * Kontrolle und Zersetzung — als ZAHLEN, nicht als Balken (Issue #283).
   *
   * DIE FORM FOLGT DER HERKUNFT DER GRENZE
   * --------------------------------------
   * | Herkunft | Form | hier |
   * | --- | --- | --- |
   * | in den Daten festgelegt | Balken, er darf sich daran binden | Hitze |
   * | im Code, „wird gemessen" | Zahl mit Nenner | Zersetzung |
   * | keine | Zahl allein | Kontrolle |
   *
   * **Zwei unabhängige Gründe gegen drei Balken**, beide gemessen:
   *
   * 1. Der Wertebereich. `control` hat keine Obergrenze — gemessen 82 nach
   *    einer Minute BIND auf Abklingzeit. Ein Balken müsste eine erfinden, und
   *    jede Wahl wäre entweder zu klein oder willkürlich.
   * 2. Der Verfall, und der ist der stärkere. `control` fällt NIE (60 s
   *    gemessen, unverändert), `decay` verfällt nach 12 s je Stapel, die Hitze
   *    1 je 2000 ms. **Ein Balken, der nie fällt, sieht kaputt aus.**
   *
   * `MAX_DECAY_STACKS` trägt den Kommentar „Wird gemessen": Die Fünf ist ein
   * Platzhalter des Balance-Läufers. Kästchen wären fünf Kästchen, bis die
   * Balance läuft — dann sieben, und die Anordnung müsste mitwachsen. „3 / 5"
   * wird zu „3 / 7" ohne eine Zeile Änderung.
   *
   * Sie erscheinen nach derselben Regel wie die Hitze: nur die aktiv
   * genutzten (GAME-DESIGN 5).
   */
  private drawMechanicNumbers(ctx: CanvasRenderingContext2D): void {
    const combat = this.session.combat;
    const sichtbar = this.visibleMechanics();
    ctx.textAlign = 'left';

    let x = BAR_LEFT;
    const zeichne = (label: string, wert: string): void => {
      ctx.font = FONT_SMALL;
      ctx.fillStyle = INK_DIM;
      ctx.fillText(label, x, MECHANIC_TOP - 10);
      ctx.font = FONT_TEXT;
      ctx.fillStyle = INK;
      ctx.fillText(wert, x, MECHANIC_TOP + 14);
      x += MECHANIC_GAP;
    };

    if (sichtbar.includes('control')) zeichne(LABEL_CONTROL, String(combat.control));
    if (sichtbar.includes('decay')) {
      zeichne(LABEL_DECAY, `${combat.decayStacks} / ${MAX_DECAY_STACKS}`);
    }
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
