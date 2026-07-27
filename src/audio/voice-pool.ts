import type { Mixer } from './mixer.js';
import { BUSES, NO_VOICE, type Bus, type VoiceHandle } from './types.js';

/**
 * Ein vorab angelegter Abspiel-Platz.
 *
 * Der Lautstaerke-Knoten (`gain`) wird **einmal** erzeugt und fuer alle
 * kuenftigen Wiedergaben dieses Platzes wiederverwendet — er haengt fest am
 * Bus-Eingang und wird nie umgesteckt.
 *
 * Der `AudioBufferSourceNode` ist der einzige Knoten, der je Wiedergabe neu
 * entstehen muss: Die Web-Audio-Spezifikation erklaert ihn ausdruecklich fuer
 * einmal verwendbar ("can only be played once"), `start()` ein zweites Mal
 * loest einen `InvalidStateError` aus. Diese eine Erzeugung ist damit nicht
 * vermeidbar; alles andere im Abspielpfad ist es (GAME-DESIGN 18.2).
 */
export interface VoiceSlot {
  readonly index: number;
  readonly bus: Bus;
  readonly gain: GainNode;
  /** Vorab gebundener Ende-Handler — kein neuer Abschluss je Wiedergabe. */
  readonly onEnded: () => void;
  source: AudioBufferSourceNode | null;
  /** Klang, der auf diesem Platz laeuft — nur eine Referenz, keine Erzeugung. */
  defId: string | null;
  active: boolean;
  /** Zaehlt bei jeder Freigabe hoch und entwertet alte Bezeichner. */
  generation: number;
  /** Zeitpunkt in der Uhr des Kontexts, an dem der Platz frei wird. */
  endsAt: number;
  /** Laeuft dieser Platz in einer Schleife (endet also nicht von selbst)? */
  looping: boolean;
}

/** Wie viele Stimmen je Weg gleichzeitig klingen koennen. */
export type PoolSizes = Readonly<Record<Bus, number>>;

export const DEFAULT_POOL_SIZES: PoolSizes = {
  // Zauber, Treffer, Boss: hier faellt am meisten gleichzeitig an.
  sfx: 24,
  // Musik-Schichten je Instanz (GAME-DESIGN 17.1) plus Reserve fuer Uebergaenge.
  music: 8,
  ui: 8,
};

/** Index-Anteil des Bezeichners; der Rest ist die Generation. */
const INDEX_BITS = 8;
const INDEX_MASK = (1 << INDEX_BITS) - 1;

/**
 * Fester Vorrat an Abspiel-Plaetzen (GAME-DESIGN 18.2: vorab anlegen und
 * wiederverwenden, nie im laufenden Bild etwas erzeugen).
 */
export class VoicePool {
  private readonly slots: VoiceSlot[] = [];
  private readonly ranges = new Map<Bus, { from: number; to: number }>();
  private stolen = 0;

  constructor(ctx: BaseAudioContext, mixer: Mixer, sizes: PoolSizes = DEFAULT_POOL_SIZES) {
    for (const bus of BUSES) {
      const size = sizes[bus];
      const from = this.slots.length;
      for (let i = 0; i < size; i++) {
        const gain = ctx.createGain();
        gain.gain.value = 1;
        gain.connect(mixer.input(bus));
        const slot: VoiceSlot = {
          index: this.slots.length,
          bus,
          gain,
          onEnded: () => this.release(slot),
          source: null,
          defId: null,
          active: false,
          generation: 0,
          endsAt: 0,
          looping: false,
        };
        this.slots.push(slot);
      }
      this.ranges.set(bus, { from, to: this.slots.length });
      if (this.slots.length > INDEX_MASK + 1) {
        throw new Error(`Zu viele Stimmen: ${this.slots.length} > ${INDEX_MASK + 1}`);
      }
    }
  }

  /**
   * Freien Platz des Busses holen. Ist keiner frei, wird der Platz genommen,
   * der als naechstes ohnehin ausgelaufen waere — Schleifen zuletzt, weil sie
   * gar nicht von selbst enden.
   */
  acquire(bus: Bus): VoiceSlot {
    const range = this.ranges.get(bus);
    if (!range) throw new Error(`Unbekannter Bus: ${bus}`);

    let victim: VoiceSlot | null = null;
    for (let i = range.from; i < range.to; i++) {
      const slot = this.slots[i];
      if (!slot) continue;
      if (!slot.active) return slot;
      if (victim === null || slot.endsAt < victim.endsAt) victim = slot;
    }

    // Alle belegt: den am fruehesten endenden Platz raeumen.
    if (!victim) throw new Error(`Bus ohne Stimmen: ${bus}`);
    this.stolen++;
    this.release(victim);
    return victim;
  }

  /** Platz freigeben und alle darauf zeigenden Bezeichner entwerten. */
  release(slot: VoiceSlot): void {
    const source = slot.source;
    if (source) {
      slot.source = null;
      source.onended = null;
      try {
        source.stop();
      } catch {
        // Bereits beendet — die Spezifikation erlaubt stop() nach dem Ende,
        // aber nicht vor start(). Beides ist hier unkritisch.
      }
      source.disconnect();
    }
    if (slot.active) slot.generation = (slot.generation + 1) & 0x7fffff;
    slot.active = false;
    slot.defId = null;
    slot.looping = false;
    slot.endsAt = 0;
  }

  handleOf(slot: VoiceSlot): VoiceHandle {
    return slot.index | (slot.generation << INDEX_BITS);
  }

  /** Platz zu einem Bezeichner — oder `null`, wenn die Stimme laengst vorbei ist. */
  resolve(handle: VoiceHandle): VoiceSlot | null {
    if (handle === NO_VOICE) return null;
    const index = handle & INDEX_MASK;
    const slot = this.slots[index];
    if (!slot || !slot.active) return null;
    if (slot.generation !== handle >>> INDEX_BITS) return null;
    return slot;
  }

  /** Alles anhalten — etwa beim Verlassen des Kampfes. */
  releaseAll(): void {
    for (const slot of this.slots) this.release(slot);
  }

  activeCount(bus?: Bus): number {
    let count = 0;
    for (const slot of this.slots) {
      if (!slot.active) continue;
      if (bus && slot.bus !== bus) continue;
      count++;
    }
    return count;
  }

  size(bus?: Bus): number {
    if (!bus) return this.slots.length;
    const range = this.ranges.get(bus);
    return range ? range.to - range.from : 0;
  }

  /** Wie oft ein belegter Platz geraeumt werden musste — Hinweis auf zu kleinen Vorrat. */
  stolenCount(): number {
    return this.stolen;
  }
}
