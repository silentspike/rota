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
import { CALCINATION, FIRST_CRUCIBLE } from './data.js';
import { STEP1_BINDINGS, spellSoundId } from './loadout.js';
import { GameLoop } from './loop.js';
import { Session } from './session.js';
import { PressQueue } from '../input/queue.js';
import { InputRouter } from '../input/router.js';
import { KeyLabels } from '../input/layout.js';
import { Overlay } from '../render/overlay.js';
import { PHASE_COUNTDOWN, PHASE_ENDED, PHASE_FIGHTING, PHASE_PREPARE } from './phases.js';
import { HudRenderer } from '../render/hud.js';
import { Surface } from '../render/surface.js';

export class App {
  readonly surface: Surface;
  readonly loop: GameLoop;
  readonly combat: Combat;
  readonly session: Session;
  readonly input: InputRouter;
  readonly labels = new KeyLabels();
  readonly audio = new AudioFeedback();
  readonly overlay: Overlay;

  private readonly boundUiKey: (event: KeyboardEvent) => void;

  /** Die Übergabe vom Browser an den Logik-Takt. Öffentlich für die Werkzeuge. */
  readonly pressQueue = new PressQueue();

  /** Was beim Lesen der Spieldaten nicht stimmte. Leer heißt: alles in Ordnung. */
  readonly dataProblems: readonly string[];

  constructor(parent: HTMLElement) {
    this.dataProblems = [...CALCINATION.problems, ...FIRST_CRUCIBLE.problems];
    this.surface = new Surface(parent);
    this.combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    this.session = new Session(this.pressQueue, this.combat, this.audio);

    for (const binding of STEP1_BINDINGS) {
      const index = this.combat.indexOf(binding.spellId);
      if (index >= 0) this.session.bind(binding.slot, index);
    }
    this.audio.setSpellSounds(this.combat.spells.map((spell) => spellSoundId(spell.id)));

    this.overlay = new Overlay(parent, {
      onReady: () => this.ready(),
      onResume: () => this.resume(),
      onWipe: () => this.wipe(),
      onRestart: () => this.restart(),
      onBackToMenu: () => this.backToMenu(),
    });

    const renderer = new HudRenderer(this.surface, this.session, this.labels, this.overlay);
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
  ready(): boolean {
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
    this.loop.reset();
  }

  private handleUiKey(event: KeyboardEvent): void {
    if (event.ctrlKey || event.altKey || event.metaKey) return;
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
