import type { SoundDef } from './types.js';

/** Was beim Laden schiefging — je Datei ein Eintrag. */
export interface LoadFailure {
  readonly src: string;
  readonly reason: string;
}

export interface LoadReport {
  readonly loaded: number;
  readonly bytes: number;
  readonly seconds: number;
  readonly failures: readonly LoadFailure[];
}

/**
 * Alle Klaenge, vorab entschluesselt (GAME-DESIGN 17.1: latenzfrei auf
 * Tastendruck).
 *
 * **Nie im Moment des Abspielens dekodieren.** `decodeAudioData` laeuft zwar
 * ausserhalb des Haupt-Fadens, braucht aber Zehntelsekunden — auf den
 * Tastendruck hin waere das eine hoerbare Verzoegerung. Deshalb: einmal beim
 * Laden alles in `AudioBuffer` verwandeln und behalten.
 */
export class SoundLibrary {
  private readonly defs = new Map<string, SoundDef>();
  private readonly buffers = new Map<string, AudioBuffer>();

  register(defs: readonly SoundDef[]): void {
    for (const def of defs) this.defs.set(def.id, def);
  }

  def(id: string): SoundDef | undefined {
    return this.defs.get(id);
  }

  ids(): string[] {
    return [...this.defs.keys()];
  }

  buffer(src: string): AudioBuffer | undefined {
    return this.buffers.get(src);
  }

  /** Fertigen Puffer einsetzen — fuer Tests und synthetisch erzeugte Klaenge. */
  put(src: string, buffer: AudioBuffer): void {
    this.buffers.set(src, buffer);
  }

  /** Alle Dateien, die die angemeldeten Klaenge brauchen, ohne Doppelungen. */
  sources(): string[] {
    const out: string[] = [];
    for (const def of this.defs.values()) {
      if (def.kind === 'oneshot') {
        out.push(def.src);
      } else {
        if (def.attackSrc) out.push(def.attackSrc);
        out.push(def.bodySrc);
        if (def.releaseSrc) out.push(def.releaseSrc);
      }
    }
    return [...new Set(out)];
  }

  /**
   * Alles laden und entschluesseln. Eine fehlende Datei bricht den Ladevorgang
   * nicht ab — sie wird gemeldet, und das Spiel startet stumm statt gar nicht.
   */
  async loadAll(
    ctx: BaseAudioContext,
    resolve: (src: string) => string = (src) => src,
  ): Promise<LoadReport> {
    const failures: LoadFailure[] = [];
    let bytes = 0;
    let seconds = 0;
    let loaded = 0;

    const pending = this.sources()
      .filter((src) => !this.buffers.has(src))
      .map(async (src) => {
        try {
          const response = await fetch(resolve(src));
          if (!response.ok) {
            failures.push({ src, reason: `HTTP ${response.status}` });
            return;
          }
          const raw = await response.arrayBuffer();
          const buffer = await ctx.decodeAudioData(raw.slice(0));
          this.buffers.set(src, buffer);
          loaded++;
          bytes += raw.byteLength;
          seconds += buffer.duration;
        } catch (error) {
          failures.push({ src, reason: String(error) });
        }
      });

    await Promise.all(pending);
    return { loaded, bytes, seconds, failures };
  }
}
