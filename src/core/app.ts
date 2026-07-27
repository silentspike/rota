/**
 * Zusammenbau des laufenden Spiels: Zeichenfläche, Logik-Takt, Zeichenschleife.
 *
 * Hier hängt zusammen, was in `loop.ts` bewusst getrennt gehalten ist. Die Klasse
 * bleibt frei von Entwicklungs-Werkzeugen — die hängen sich von außen an
 * (`GameLoop.onFrameMeasured`) und fallen im Auslieferungsstand weg.
 */
import { GameLoop, type Simulation } from './loop.js';
import { ScaffoldRenderer } from '../render/scaffold.js';
import { Surface } from '../render/surface.js';

/**
 * Platzhalter-Simulation für E1-1.
 *
 * E1-1 baut ausdrücklich **nur das Zeitgerüst** — Zauber, HUD und Boss stehen
 * außerhalb seines Umfangs. Der Kampf entsteht in E1-3 und ersetzt diese Klasse.
 */
class IdleSimulation implements Simulation {
  tick(_timeMs: number): void {
    // absichtlich leer, siehe Klassenkommentar
  }
}

export class App {
  readonly surface: Surface;
  readonly loop: GameLoop;

  constructor(parent: HTMLElement) {
    this.surface = new Surface(parent);
    const renderer = new ScaffoldRenderer(this.surface);
    this.loop = new GameLoop({
      simulation: new IdleSimulation(),
      renderer,
      requestFrame: (callback) => window.requestAnimationFrame(callback),
      cancelFrame: (handle) => {
        window.cancelAnimationFrame(handle);
      },
      now: () => performance.now(),
    });
  }

  start(): void {
    this.loop.start();
  }

  stop(): void {
    this.loop.stop();
  }
}
