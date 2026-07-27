/**
 * Fester Logik-Takt, davon getrennte Zeichenschleife (GAME-DESIGN 18.2 und 18.4).
 *
 * Der Kern des Determinismus: Die Simulation läuft ausschließlich in Schritten von
 * `TICK_MS` und rechnet mit **ganzzahliger** Zeit in Millisekunden. Die Bildrate des
 * Geräts beeinflusst nur, wie oft gezeichnet wird — nie, was gerechnet wird. Auf
 * 144 Hz gelten dieselben Regeln wie auf 60 Hz, sonst wären Bestzeiten geräteabhängig.
 *
 * Zwischen zwei Takten interpoliert das Zeichnen über `alpha`, damit Bewegungen auch
 * bei 144 Hz flüssig aussehen, obwohl die Logik nur 100-mal je Sekunde rechnet.
 *
 * Im laufenden Bild wird **nichts** erzeugt (GAME-DESIGN 18.2): Der Bild-Rückruf ist
 * einmalig im Konstruktor gebunden, alle Zwischenwerte sind Zahlen.
 */

/** Logik-Takt in Millisekunden. Ganzzahlig, damit nichts driften kann. */
export const TICK_MS = 10;

/**
 * Obergrenze für nachgeholte Takte je Bild.
 *
 * Ohne sie schaukelt sich ein Rückstand auf: Viele nachzuholende Takte machen das
 * Bild noch langsamer, was noch mehr Takte erzeugt. 25 Takte sind 250 ms — mehr
 * Rückstand als das entsteht nur, wenn der Reiter im Hintergrund lag oder das Gerät
 * hängen blieb. Dann ist es richtig, die Zeitschuld zu verwerfen statt sie abzuarbeiten.
 */
export const MAX_CATCHUP_TICKS = 25;

/** Was je Logik-Takt gerechnet wird. */
export interface Simulation {
  /**
   * Ein Logik-Takt.
   *
   * @param timeMs Verstrichene Kampfzeit **vor** diesem Takt, immer ein ganzzahliges
   *   Vielfaches von `TICK_MS`. Der erste Takt bekommt 0 — dieselbe Zählweise wie
   *   `tools/step1-check.py`, damit Spiel und Referenz taktweise vergleichbar bleiben.
   */
  tick(timeMs: number): void;
}

/** Was je Bild gezeichnet wird. */
export interface Renderer {
  /**
   * @param simTimeMs Ganzzahlige Kampfzeit, bis zu der gerechnet wurde.
   * @param alpha Anteil des noch nicht gerechneten Takts, in [0, 1). Die anzuzeigende
   *   Zeit ist `simTimeMs + alpha * TICK_MS`.
   */
  render(simTimeMs: number, alpha: number): void;
}

/** Zeitquelle. Als Parameter, damit Tests ohne Browser eine eigene Uhr stellen können. */
export type NowFn = () => number;

/** Bild-Anforderung. Im Browser `requestAnimationFrame`, im Test ein eigener Treiber. */
export type RequestFrameFn = (callback: (timestampMs: number) => void) => number;

/** Gegenstück zu `RequestFrameFn`. */
export type CancelFrameFn = (handle: number) => void;

export interface GameLoopOptions {
  readonly simulation: Simulation;
  readonly renderer: Renderer;
  readonly requestFrame: RequestFrameFn;
  readonly cancelFrame: CancelFrameFn;
  readonly now: NowFn;
}

/**
 * Messwert-Rückruf. Bewusst mit einzelnen Zahlen statt einem Objekt — ein Objekt je
 * Bild wäre genau die Speicheranforderung, die GAME-DESIGN 18.2 ausschließt.
 *
 * @param frameWorkMs Zeit, die dieses Bild in Takt und Zeichnen verbracht hat.
 * @param frameIntervalMs Abstand zum vorherigen Bild.
 * @param ticksThisFrame Wie viele Logik-Takte in diesem Bild gerechnet wurden.
 */
export type FrameMeasurementFn = (
  frameWorkMs: number,
  frameIntervalMs: number,
  ticksThisFrame: number,
) => void;

/**
 * Die Schleife selbst.
 *
 * Lebensweg: `start()` → beliebig viele Bilder → `stop()`. `reset()` setzt die
 * Simulationszeit auf 0 zurück, ohne die Schleife anzuhalten.
 */
export class GameLoop {
  /**
   * Messwert-Empfänger. Bleibt im Auslieferungsstand `null` — die
   * Entwicklungs-Werkzeuge setzen ihn, und nur sie (GAME-DESIGN 19.3).
   */
  onFrameMeasured: FrameMeasurementFn | null = null;

  private readonly simulation: Simulation;
  private readonly renderer: Renderer;
  private readonly requestFrame: RequestFrameFn;
  private readonly cancelFrame: CancelFrameFn;
  private readonly now: NowFn;

  /** Einmalig gebunden — sonst entstünde je Bild eine neue Funktion. */
  private readonly boundFrame: (timestampMs: number) => void;

  /** Verstrichene Kampfzeit in ganzen Millisekunden. Immer `ticksRun * TICK_MS`. */
  private simTimeMs = 0;
  private ticksRun = 0;
  private ticksDropped = 0;
  private framesRendered = 0;

  /** Noch nicht in Takte umgesetzte Realzeit. Der einzige Fließkomma-Wert. */
  private carryMs = 0;
  private lastTimestampMs = 0;
  private hasLastTimestamp = false;

  private running = false;
  private frameHandle = 0;

  constructor(options: GameLoopOptions) {
    this.simulation = options.simulation;
    this.renderer = options.renderer;
    this.requestFrame = options.requestFrame;
    this.cancelFrame = options.cancelFrame;
    this.now = options.now;
    this.boundFrame = (timestampMs: number): void => {
      this.onFrame(timestampMs);
    };
  }

  get isRunning(): boolean {
    return this.running;
  }

  /** Ganzzahlige Kampfzeit, bis zu der gerechnet wurde. */
  get timeMs(): number {
    return this.simTimeMs;
  }

  get tickCount(): number {
    return this.ticksRun;
  }

  /** Takte, die wegen `MAX_CATCHUP_TICKS` verworfen wurden. Im Normalbetrieb 0. */
  get droppedTickCount(): number {
    return this.ticksDropped;
  }

  get frameCount(): number {
    return this.framesRendered;
  }

  /** Anteil des noch nicht gerechneten Takts, in [0, 1). */
  get alpha(): number {
    return this.carryMs / TICK_MS;
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    this.hasLastTimestamp = false;
    this.frameHandle = this.requestFrame(this.boundFrame);
  }

  stop(): void {
    if (!this.running) return;
    this.running = false;
    this.cancelFrame(this.frameHandle);
  }

  /** Setzt die Kampfzeit auf 0. Zähler und Messwerte bleiben stehen. */
  reset(): void {
    this.simTimeMs = 0;
    this.ticksRun = 0;
    this.carryMs = 0;
    this.hasLastTimestamp = false;
  }

  /**
   * Rechnet `count` Takte sofort, ohne auf ein Bild zu warten.
   *
   * Für die Entwicklungs-Werkzeuge (Einzelschritt, GAME-DESIGN 19.1) und für Tests,
   * die eine Runde ohne Browser durchrechnen.
   */
  advanceTicks(count: number): void {
    for (let i = 0; i < count; i++) {
      this.simulation.tick(this.simTimeMs);
      this.simTimeMs += TICK_MS;
      this.ticksRun++;
    }
  }

  private onFrame(timestampMs: number): void {
    if (!this.running) return;
    this.frameHandle = this.requestFrame(this.boundFrame);

    // Die Uhr wird nur abgefragt, wenn jemand zuhört: im Auslieferungsstand
    // kostet die Messung dadurch nicht einmal den Aufruf.
    const measured = this.onFrameMeasured !== null;
    const workStartMs = measured ? this.now() : 0;

    let intervalMs = 0;
    if (this.hasLastTimestamp) {
      intervalMs = timestampMs - this.lastTimestampMs;
      // Ein rückwärts laufender Zeitstempel wäre ein Fehler der Zeitquelle; ihn
      // hier abzufangen ist billiger als überall mit negativer Zeit zu rechnen.
      if (intervalMs < 0) intervalMs = 0;
      this.carryMs += intervalMs;
    } else {
      this.hasLastTimestamp = true;
    }
    this.lastTimestampMs = timestampMs;

    let ticksThisFrame = 0;
    while (this.carryMs >= TICK_MS && ticksThisFrame < MAX_CATCHUP_TICKS) {
      this.simulation.tick(this.simTimeMs);
      this.simTimeMs += TICK_MS;
      this.carryMs -= TICK_MS;
      this.ticksRun++;
      ticksThisFrame++;
    }

    // Rückstand über der Obergrenze wird verworfen statt abgearbeitet.
    if (this.carryMs >= TICK_MS) {
      const dropped = Math.floor(this.carryMs / TICK_MS);
      this.ticksDropped += dropped;
      this.carryMs -= dropped * TICK_MS;
    }

    this.renderer.render(this.simTimeMs, this.carryMs / TICK_MS);
    this.framesRendered++;

    if (measured) {
      // Der Aufruf ist hier nachweislich nicht null; TypeScript sieht das über die
      // Zuweisung oben nicht, weil das Feld zwischenzeitlich geändert werden könnte.
      const probe = this.onFrameMeasured;
      if (probe !== null) probe(this.now() - workStartMs, intervalMs, ticksThisFrame);
    }
  }
}
