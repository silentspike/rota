/**
 * Musik in Schichten, die auf den Kampf reagieren (GAME-DESIGN 17.1).
 *
 * Ein Grundklang je Instanz, dazu zuschaltbare Schichten — Spannung, Endspurt,
 * Phasenwechsel, Ueberdruck. Deutlich haltbarer als eine feste Schleife, die
 * bei hunderten Wiederholungen zur Qual wird.
 *
 * **Der Kern des Verfahrens: Alle Schichten laufen von Anfang an mit, hoerbar
 * ist nur, was aufgeblendet wurde.** Zuschalten heisst also nicht "starten",
 * sondern "lauter werden".
 *
 * Warum nicht bei Bedarf starten? Zwei Gruende, beide entscheidend:
 *
 * 1. **Gleichlauf.** Schichten, die zu verschiedenen Zeiten anfangen, stehen
 *    zueinander verschoben. Die Verschiebung waere zufaellig und bei jedem
 *    Versuch anders — aus Musik wuerde Matsch.
 * 2. **Kein Knacken.** Ein Puffer, der mitten im Signal einsetzt, springt.
 *    Eine Blende von 0 auf 1 springt nicht.
 *
 * Der Preis sind ein paar dauerhaft laufende Stimmen. Der Vorrat hat sie
 * (`DEFAULT_POOL_SIZES.music`), und sie kosten nichts, solange sie stumm sind.
 *
 * **Taktsynchron:** Uebergaenge beginnen nicht sofort, sondern an der naechsten
 * Taktgrenze. Sonst faellt eine neue Schicht mitten in einen Schlag.
 */
import type { AudioEngine } from './engine.js';
import { NO_VOICE, type VoiceHandle } from './types.js';

export interface MusicLayer {
  /** Kennung des Klangs im Verzeichnis. */
  readonly id: string;
  /** Pegel im eingeschalteten Zustand. */
  readonly gain?: number;
  /** Von Anfang an hoerbar — der Grundklang. */
  readonly base?: boolean;
}

export interface MusicOptions {
  /**
   * Laenge eines Takts in Sekunden. Uebergaenge beginnen an einem Vielfachen
   * davon, gerechnet ab dem Start der Musik.
   */
  readonly barSeconds?: number;
  /** Dauer einer Blende in Sekunden. */
  readonly fadeSeconds?: number;
}

interface LayerState {
  readonly layer: MusicLayer;
  handle: VoiceHandle;
  enabled: boolean;
  /** Zeitpunkt, an dem die letzte angeforderte Blende endet. */
  settledAt: number;
}

const DEFAULT_BAR_SECONDS = 2;
const DEFAULT_FADE_SECONDS = 1.5;

export class MusicDirector {
  private readonly states = new Map<string, LayerState>();
  private readonly barSeconds: number;
  private readonly fadeSeconds: number;
  private startedAt = 0;
  private running = false;

  constructor(
    private readonly engine: AudioEngine,
    options: MusicOptions = {},
  ) {
    this.barSeconds = options.barSeconds ?? DEFAULT_BAR_SECONDS;
    this.fadeSeconds = options.fadeSeconds ?? DEFAULT_FADE_SECONDS;
  }

  /**
   * Alle Schichten starten. Der Grundklang klingt sofort, alles andere laeuft
   * stumm mit.
   *
   * @returns die Schichten, die nicht starten konnten (fehlende Datei)
   */
  start(layers: readonly MusicLayer[]): string[] {
    this.stop();
    const failed: string[] = [];
    this.startedAt = this.engine.ctx.currentTime;

    for (const layer of layers) {
      const target = layer.base ? (layer.gain ?? 1) : 0;
      const handle = this.engine.startSustained(layer.id, target);
      if (handle === NO_VOICE) {
        failed.push(layer.id);
        continue;
      }
      this.states.set(layer.id, {
        layer,
        handle,
        enabled: Boolean(layer.base),
        settledAt: this.startedAt,
      });
    }
    this.running = this.states.size > 0;
    return failed;
  }

  /** Naechste Taktgrenze ab jetzt (oder ab dem Zeitpunkt, den man vorgibt). */
  nextBar(from = this.engine.ctx.currentTime): number {
    if (!this.running) return from;
    const elapsed = from - this.startedAt;
    const bars = Math.ceil(elapsed / this.barSeconds + 1e-9);
    return this.startedAt + bars * this.barSeconds;
  }

  /**
   * Eine Schicht zuschalten.
   *
   * @param immediate Blende sofort statt an der naechsten Taktgrenze. Fuer
   *   Ereignisse, die nicht warten koennen — etwa den Ueberdruck.
   */
  enable(id: string, immediate = false): boolean {
    return this.set(id, true, immediate);
  }

  /** Eine Schicht abschalten. */
  disable(id: string, immediate = false): boolean {
    return this.set(id, false, immediate);
  }

  private set(id: string, enabled: boolean, immediate: boolean): boolean {
    const state = this.states.get(id);
    if (!state) return false;
    if (state.enabled === enabled) return true;

    const at = immediate ? this.engine.ctx.currentTime : this.nextBar();
    const target = enabled ? (state.layer.gain ?? 1) : 0;
    if (!this.engine.fadeVoice(state.handle, target, at, this.fadeSeconds)) return false;

    state.enabled = enabled;
    state.settledAt = at + this.fadeSeconds;
    return true;
  }

  isEnabled(id: string): boolean {
    return this.states.get(id)?.enabled ?? false;
  }

  /** Welche Schichten gerade hoerbar sind. */
  activeLayers(): string[] {
    return [...this.states.values()].filter((s) => s.enabled).map((s) => s.layer.id);
  }

  /** Welche Schichten mitlaufen, hoerbar oder nicht. */
  runningLayers(): string[] {
    return [...this.states.keys()];
  }

  /** Laeuft die Musik ueberhaupt? */
  isRunning(): boolean {
    return this.running && this.states.size > 0;
  }

  /** Alles ausblenden und danach anhalten. */
  fadeOutAll(seconds = this.fadeSeconds): void {
    const at = this.engine.ctx.currentTime;
    for (const state of this.states.values()) {
      this.engine.fadeVoice(state.handle, 0, at, seconds);
      state.enabled = false;
    }
  }

  /** Sofort anhalten. */
  stop(): void {
    for (const state of this.states.values()) this.engine.stop(state.handle);
    this.states.clear();
    this.running = false;
  }
}
