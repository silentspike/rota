/**
 * Zusammenbau des laufenden Spiels: Zeichenfläche, Logik-Takt, Zeichenschleife,
 * Eingabe, Kampf und Klang.
 *
 * Hier hängt zusammen, was sonst bewusst getrennt gehalten ist. Die Klasse bleibt
 * frei von Entwicklungs-Werkzeugen — die hängen sich von außen an
 * (`GameLoop.onFrameMeasured`) und fallen im Auslieferungsstand weg.
 */
import { AudioFeedback } from './audio-bridge.js';
import { Combat } from './combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById, type BossDef } from './data.js';
import { EV_BOSS_HIT, NO_KEY } from './events.js';
import { bindSpell, defaultBindings, sanitize, spellAtSlot } from '../progression/bindings.js';
import { SLOT_COUNT } from '../input/keys.js';
import { GameLoop } from './loop.js';
import { Session } from './session.js';
import { PressQueue } from '../input/queue.js';
import { InputRouter } from '../input/router.js';
import { KeyLabels } from '../input/layout.js';
import { Impact } from '../render/impact.js';
import { Overlay } from '../render/overlay.js';
import { PHASE_COUNTDOWN, PHASE_ENDED, PHASE_FIGHTING, PHASE_PREPARE } from './phases.js';
import { RunPersistence } from '../analysis/run-persistence.js';
import { recordRun, type RecordResult } from '../analysis/run-recorder.js';
import { markDisplayFor } from '../analysis/mark-display.js';
import type { MarkDisplay } from '../render/overlay.js';
import { PlayerHome } from '../progression/player-home.js';
import { statOverview } from '../progression/stat-overview.js';
import { levelBarFill, levelOverview } from '../progression/level-overview.js';
import { buildLevelOverview } from '../progression/level-overview-view.js';
import { buildStatOverview } from '../progression/stat-overview-view.js';
import { NOTHING_WORN } from '../gear/worn.js';
import { baseStats } from '../progression/base-stats.js';
import '../progression/stats.css';
import '../progression/levels.css';
import { HudRenderer } from '../render/hud.js';
import { Surface } from '../render/surface.js';

/**
 * Welcher Boss den Kampf traegt — normalerweise der erste Tiegel.
 *
 * # Der Startparameter `?boss=` ist ein Entwicklungs-Werkzeug (19.1)
 *
 * Die Werkzeugkiste nennt „Zustand setzen" mit vier Beispielen — Boss-Leben,
 * Abklingzeiten, Phase, Hitze —, und alle vier haben denselben Zweck: einen
 * Zustand herstellen, den zu erspielen unverhältnismäßig wäre. Einen Boss zu
 * wählen ist derselbe Zweck eine Ebene höher: „zeig mir den, der die Mechanik
 * überhaupt hat."
 *
 * Ohne ihn wäre die Energieleiste (#219) nicht prüfbar. Sechs der sieben Bosse
 * haben keine; der einzige mit einer ist im Spiel nicht erreichbar, solange die
 * Instanz-Auswahl (12.7) fehlt.
 *
 * # Warum ein Startparameter und kein Wechsel zur Laufzeit
 *
 * Ein `setBoss(id)` müsste `Combat` und `Session` **ersetzen** — beide sind in
 * `App` unveränderlich, und an der Session hängt der Beobachter für den
 * Spielstand. Ein verlorener Haken fiele erst auf, wenn ein Lauf nicht
 * gespeichert wird, also an einer Stelle, die mit dem Boss nichts zu tun hat.
 *
 * Hier entsteht der Kampf **von Anfang an** mit dem richtigen Boss. Es wird
 * nichts ersetzt, und der Haken wird zwei Zeilen später gesetzt wie immer.
 * Das ist außerdem der Weg, den 12.7 ohnehin nimmt: Der Boss wird **vor** dem
 * Kampf gewählt.
 *
 * # Ein Tippfehler scheitert sichtbar
 *
 * `?boss=tipfeler` startet **nicht** stillschweigend den Standard-Boss. Wer
 * dann misst, misst die falsche Sache und hält sie für die richtige — genau
 * die Klasse Fehler, gegen die dieses Werkzeug gebaut wird.
 */
function startBoss(): BossDef {
  if (!__ROTA_DEVTOOLS__) return FIRST_CRUCIBLE.value;
  const wunsch = new URLSearchParams(window.location.search).get('boss');
  if (wunsch === null || wunsch === '') return FIRST_CRUCIBLE.value;

  const gefunden = bossById(wunsch);
  if (gefunden === undefined) {
    throw new Error(
      `Unbekannter Boss "${wunsch}" im Startparameter. ` +
        `Ein Tippfehler darf nicht stillschweigend den Standard-Boss starten — ` +
        `sonst misst man die falsche Sache und hält sie für die richtige.`,
    );
  }
  console.info(`[rota] Boss aus dem Startparameter: ${wunsch}`);
  return gefunden;
}

export class App {
  readonly surface: Surface;
  readonly loop: GameLoop;
  readonly combat: Combat;
  readonly session: Session;
  readonly input: InputRouter;
  readonly labels = new KeyLabels();
  readonly audio = new AudioFeedback();
  readonly overlay: Overlay;
  readonly impact = new Impact();

  private readonly boundUiKey: (event: KeyboardEvent) => void;

  /** Die Übergabe vom Browser an den Logik-Takt. Öffentlich für die Werkzeuge. */
  readonly pressQueue = new PressQueue();

  /** Was beim Lesen der Spieldaten nicht stimmte. Leer heißt: alles in Ordnung. */
  readonly dataProblems: readonly string[];

  /**
   * Der Spielstand im Browser (Issue #179).
   *
   * Öffentlich, weil das Prüfblatt ihn liest — und weil ein Zustand, den man
   * nur über einen Kampf erreichen kann, nicht nachweisbar wäre.
   */
  readonly persistence: RunPersistence;

  /**
   * Was der letzte Kampf an Marke und Verlauf bewirkt hat, oder `null`.
   *
   * Daraus zeigt die Auswertung dem Spieler, dass er weiterkommt — bei einer
   * **Niederlage** ist das der ganze Zweck (GAME-DESIGN 15.1).
   */
  lastRecord: RecordResult | null = null;

  /**
   * Die Fortschrittsmarke, fertig zum Anzeigen (Issue #187).
   *
   * Hier gebildet und nicht in der Ansicht: Der Satz entsteht an EINER Stelle,
   * aus `markSentence`. Zwei Stellen, die aus denselben Zahlen einen Satz
   * machen, laufen beim naechsten Nachziehen auseinander.
   *
   * `null` heisst: nichts zeigen. Das ist der Fall beim allerersten Kampf
   * eines Spielstands und nach einem Sieg -- dort steht das Ergebnis, nicht
   * die Marke (Issue #187, AC3 und AC4).
   */
  lastMark: MarkDisplay | null = null;

  /**
   * Der Bildschirm zwischen den Kaempfen (GAME-DESIGN 16.1, Issue #188).
   *
   * Oeffentlich, weil das Pruefblatt ihn liest -- und weil ein Bildschirm, den
   * man nur ueber einen Knopf erreicht, ohne Browser nicht nachweisbar waere.
   */
  readonly home: PlayerHome;

  constructor(parent: HTMLElement) {
    const boss = startBoss();
    this.dataProblems = [...CALCINATION.problems, ...FIRST_CRUCIBLE.problems];
    this.surface = new Surface(parent);
    this.combat = new Combat({ school: CALCINATION.value, boss });
    this.session = new Session(this.pressQueue, this.combat, this.audio);

    // Erste Verwendung des Spielstands ueberhaupt: Rotation, Pruefsummen und
    // Wiederherstellung stehen seit #83 bereit und liefen nie.
    this.persistence = new RunPersistence(__ROTA_VERSION__, () => Date.now());

    // Am Kampfende festhalten, was der Versuch eingebracht hat. Der Beobachter
    // greift nicht ein -- ein Spiel ohne Spielstand rechnet identisch
    // (GAME-DESIGN 18.5), es merkt sich nur nichts.
    this.session.onFinish = (summary, outcome): void => {
      this.lastRecord = recordRun(this.persistence.state, summary, outcome, {
        bossId: this.combat.boss.id,
        // Schritt 1 kennt eine Stufe. Die Zahl steht hier und nicht als
        // Vorgabe im Rechner: Sobald es mehrere gibt, ist es diese Zeile.
        difficulty: 1,
        // Die Stufe des BOSSES, aus seinem Datensatz — nicht die des
        // Spielers und nicht die Schwierigkeit (Issue #248).
        bossLevel: this.combat.boss.level,
        gameVersion: __ROTA_VERSION__,
        at: Date.now(),
      });
      // Ein misslungenes Ablegen ist ein Verlust, aber kein Grund, den Ablauf
      // anzuhalten -- der Rueckgabewert wird bewusst nicht geprueft.
      if (this.lastRecord.counted) this.persistence.persist(this.lastRecord);
      this.lastMark = markDisplayFor(this.lastRecord, summary, this.combat.boss.id);
    };

    this.applyBindings();
    this.audio.bindSpells(this.combat.spells.map((spell) => spell.id));

    this.overlay = new Overlay(parent, this.combat.boss.name, {
      onReady: () => this.ready(),
      onResume: () => this.resume(),
      onWipe: () => this.wipe(),
      onRestart: () => this.restart(),
      onBackToMenu: () => this.backToMenu(),
      onOpenHome: () => {
        // **Vor dem Zeigen neu rechnen.** Der Bildschirm entsteht einmal beim
        // Start; ohne diesen Aufruf stünde dort, was damals galt. Im Browser
        // gesehen: nach einem gewonnenen Kampf weiterhin „0 / 200".
        this.refreshHome();
        this.home.show();
      },
    });

    // Das Player Home. Es fasst den Kampf NICHT an -- kein Anhalten, kein
    // Zuruecksetzen, kein Zugriff auf die Sitzung. Es zeigt und verbirgt sich.
    this.home = new PlayerHome(() => {
      // Zurueck heisst zurueck zum Bildschirm davor. Das Overlay steht
      // ohnehin dort, wo es stand -- es wurde nie angefasst.
    });
    // **Der Aufstieg steht vor den Werten.** Er ist das, was sich zwischen
    // zwei Besuchen ändert; die Grundwerte folgen daraus (Issue #248, AC6).
    this.home.addSection({
      id: 'levels',
      title: 'Progression',
      content: buildLevelOverview(
        levelOverview({
          level: this.persistence.state.level,
          xp: this.persistence.state.experience,
        }),
        levelBarFill({
          level: this.persistence.state.level,
          xp: this.persistence.state.experience,
        }),
      ),
    });
    this.home.addSection({
      id: 'stats',
      title: 'Stats',
      // Die Werte, wie sie heute gelten. Ohne Ausruestung sind es die
      // Grundwerte -- ein neuer Spielstand hat nichts angelegt (AC7).
      content: buildStatOverview(statOverview(baseStats(), NOTHING_WORN)),
    });
    parent.appendChild(this.home.root);

    // Treffer gehen an die Anzeige und an den Klang. Beide lesen nur mit — ein
    // stummes, unsichtbares Spiel rechnet identisch (GAME-DESIGN 18.5).
    this.combat.onHit = (damage, kind, isCrit, big, timeMs): void => {
      this.impact.onHit(damage, kind, isCrit, big, timeMs);
      // **Jeder Treffer klingt** — so beschreibt das Verzeichnis den Klang:
      // „der häufigste Klang im Spiel überhaupt, jeder Zauber und jeder
      // DoT-Tick löst ihn aus. Deshalb sehr kurz, trocken und mit Streuung."
      // Gegen den Maschinengewehr-Effekt steht die Tonhöhen-Streuung, nicht das
      // Weglassen (GAME-DESIGN 17.1). In E1-6 war das noch offen und deshalb auf
      // die großen Treffer beschränkt; E8 hat es entschieden.
      this.audio.onEvent(EV_BOSS_HIT, NO_KEY, timeMs);
    };

    const renderer = new HudRenderer(
      this.surface,
      this.session,
      this.labels,
      this.overlay,
      this.impact,
      () => this.lastMark,
    );
    this.loop = new GameLoop({
      simulation: this.session,
      renderer,
      requestFrame: (callback) => window.requestAnimationFrame(callback),
      cancelFrame: (handle) => {
        window.cancelAnimationFrame(handle);
      },
      now: () => performance.now(),
    });
    this.input = new InputRouter({
      queue: this.pressQueue,
      // Der Druck wirkt im nächsten Takt, der gerechnet wird. Damit ist er im
      // nächsten Bild sichtbar — die kürzeste Strecke, die das Zeitgerüst zulässt.
      nextTickIndex: () => this.loop.tickCount,
      frameIndex: () => this.loop.frameCount,
      onFirstInput: () => {
        this.audio.unlock();
      },
    });

    // Die Bedienung des Ablaufs läuft über eigene Tasten, nicht über die
    // Zauberleiste: Esc öffnet das Menü, Eingabe startet den Kampf. Beide sind
    // bewusst keine Belegungen (siehe `src/input/keys.ts`).
    this.boundUiKey = (event: KeyboardEvent): void => {
      this.handleUiKey(event);
    };
  }

  // ------------------------------------------------------------ Kampfablauf

  /** „Ready" — der Countdown beginnt (GAME-DESIGN 10). */
  /**
   * Die Belegung aus dem Spielstand auf die Sitzung übertragen (Issue #264).
   *
   * **Bis hierher stand sie fest** (`STEP1_BINDINGS`), seit Schritt 1. Die
   * Plätze wuchsen mit der Stufe (#248), belegen konnte sie niemand.
   *
   * Ein leerer Spielstand bekommt die Startbelegung — und zwar **nicht
   * gespeichert**, sondern gerechnet: Solange der Spieler nichts geändert hat,
   * gibt es nichts zu speichern. Erst seine Entscheidung wird abgelegt.
   *
   * `sanitize` läuft davor: Ein Stand kann Einträge tragen, die heute nicht
   * mehr gelten — aus einer Fassung mit anderen Freischaltstufen etwa. Sie
   * stillschweigend zu übernehmen hieße, dass der Kampf einen Zauber wirkt,
   * den der Spieler gar nicht haben dürfte.
   *
   * **Jeder Platz wird gesetzt, auch die leeren.** Wer nur die belegten
   * überträgt, lässt einen geräumten Platz auf seinem alten Zauber stehen —
   * und genau das ist der Fall, der still bricht: Der Spieler merkt es erst im
   * Kampf und hält es für einen Fehler in seiner Rotation.
   */
  private applyBindings(): void {
    const stand = this.persistence.state;
    const gespeichert = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const belegung =
      Object.keys(gespeichert).length > 0
        ? gespeichert
        : defaultBindings(stand.level, this.combat.spells);

    for (let slot = 0; slot < SLOT_COUNT; slot++) {
      const spellId = spellAtSlot(belegung, slot);
      const index = spellId === null ? -1 : this.combat.indexOf(spellId);
      this.session.bind(slot, index);
    }
  }

  /**
   * Einen Zauber auf einen Platz legen (Issue #264, AC1 und AC5).
   *
   * **Drei Dinge in einer Reihenfolge, und die Reihenfolge zählt:**
   *
   * 1. Die Belegung ändern — abgelehnt wird sie, wenn Zauber oder Platz nicht
   *    frei sind; dann geschieht gar nichts.
   * 2. Ablegen, damit sie den Neustart überlebt (AC1).
   * 3. **Auf die Sitzung übertragen.** Ohne diesen Schritt wirkt der Kampf
   *    weiter den alten Zauber — der Fall, der still bricht: Der Spieler
   *    merkt es erst im Kampf und hält es für einen Fehler in seiner
   *    Rotation.
   *
   * @returns ob die Belegung wirklich geändert wurde
   */
  bindSpellToSlot(spellId: string, slot: number): boolean {
    const stand = this.persistence.state;
    const vorher = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const gefuellt =
      Object.keys(vorher).length > 0 ? vorher : defaultBindings(stand.level, this.combat.spells);
    const nachher = bindSpell(gefuellt, spellId, slot, stand.level, this.combat.spells);
    if (nachher === gefuellt) return false;

    this.persistence.persistBindings(nachher);
    this.applyBindings();
    return true;
  }

  /**
   * Die Aufstiegs-Anzeige neu rechnen (Issue #248, AC6).
   *
   * **Gerechnet, nicht gespeichert — auch auf dem Bildschirm.** Der Spielstand
   * trägt Stufe und Erfahrung; alles andere entsteht hier bei jedem Öffnen
   * neu. Ein einmal gebauter Bereich zeigte sonst, was beim Start galt.
   */
  private refreshHome(): void {
    const stand = { level: this.persistence.state.level, xp: this.persistence.state.experience };
    this.home.replaceSection(
      'levels',
      buildLevelOverview(levelOverview(stand), levelBarFill(stand)),
    );
  }

  ready(): boolean {
    // „Ready" ist eine Nutzergeste, und zwar die erste des Kampfes. Erst danach
    // gibt der Browser den Klang-Kontext frei — geschähe das erst beim ersten
    // Zauber, bliebe der Countdown stumm.
    this.audio.unlock();
    return this.session.beginCountdown(this.loop.timeMs);
  }

  /** Esc-Menü öffnen oder schließen. Nur während des Kampfes sinnvoll. */
  togglePause(): void {
    const phase = this.session.phase;
    if (phase !== PHASE_FIGHTING && phase !== PHASE_COUNTDOWN) return;
    this.overlay.setPaused(!this.overlay.isPaused);
  }

  resume(): void {
    this.overlay.setPaused(false);
  }

  /** „Wipe Run" — gilt als Ende und führt zur selben Auswertung wie ein Sieg. */
  wipe(): boolean {
    const wiped = this.session.wipe(this.loop.timeMs);
    if (wiped) this.overlay.setPaused(false);
    return wiped;
  }

  /** Neuer Versuch: zurück in die Vorbereitung und sofort bereit machen. */
  restart(): void {
    this.backToMenu();
    this.ready();
  }

  /**
   * Zurück in die Vorbereitung.
   *
   * Ein Menü im eigentlichen Sinn gibt es noch nicht — es entsteht mit einem
   * späteren Epic. Der Knopf trägt den Namen aus GAME-DESIGN 10 und führt
   * dorthin, wo der Spieler wieder anfangen kann.
   */
  backToMenu(): void {
    this.overlay.setPaused(false);
    this.session.reset();
    this.impact.reset();
    this.loop.reset();
  }

  private handleUiKey(event: KeyboardEvent): void {
    if (event.ctrlKey || event.altKey || event.metaKey) return;
    // **Der Beute-Moment ist vollständig überspringbar** (GAME-DESIGN 13.6),
    // auch der des Legendary. Er wird ZUERST behandelt: Wer die Taste drückt,
    // will weiter — und ein Moment, der eine Eingabe schluckt, wäre beim
    // zweiten Mal eine Zumutung.
    if (event.code === 'Space' && this.overlay.skipLoot()) {
      event.preventDefault();
      return;
    }
    if (event.code === 'Escape') {
      event.preventDefault();
      this.togglePause();
      return;
    }
    if (event.code === 'Enter' || event.code === 'NumpadEnter') {
      event.preventDefault();
      if (this.session.phase === PHASE_PREPARE) this.ready();
      else if (this.session.phase === PHASE_ENDED) this.restart();
    }
  }

  start(): void {
    if (this.dataProblems.length > 0) {
      // Nicht abbrechen: Ein einzelner kaputter Datensatz darf nicht das ganze
      // Spiel verhindern — aber sichtbar sein muss es.
      console.error('[rota] Spieldaten fehlerhaft:', this.dataProblems);
    }
    this.input.attach();
    window.addEventListener('keydown', this.boundUiKey);
    this.loop.start();
    // Beides läuft nebenher: Das Spiel ist sofort spielbar, die Beschriftung
    // rückt nach, sobald der Browser Auskunft gibt, und der Klang, sobald geladen.
    void this.labels.refresh();
    void this.audio.init();
  }

  stop(): void {
    this.loop.stop();
    this.input.detach();
    window.removeEventListener('keydown', this.boundUiKey);
  }
}
