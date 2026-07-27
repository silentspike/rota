import { SoundLibrary, type LoadReport } from './library.js';
import { Mixer } from './mixer.js';
import { AudioRandom } from './rng.js';
import { NO_VOICE, type Bus, type OneShotDef, type VoiceHandle } from './types.js';
import { DEFAULT_POOL_SIZES, VoicePool, type PoolSizes } from './voice-pool.js';

export interface AudioEngineOptions {
  readonly poolSizes?: PoolSizes;
  /** Startwert des Streuungs-Zufalls — gesetzt fuer wiederholbare Tests. */
  readonly seed?: number;
  /**
   * Wandelt den Pfad aus dem Manifest in eine abrufbare Adresse. Ohne Angabe
   * wird der Pfad unveraendert verwendet.
   */
  readonly resolveUrl?: (src: string) => string;
}

/** Standard-Ausblendung eines Koerpers, wenn der Klang nichts anderes sagt. */
const DEFAULT_FADE_OUT = 0.08;

/** Wie viele Tonhoehen zur Kontrolle vorgehalten werden (vorab angelegt). */
const RATE_HISTORY = 64;

/**
 * Die Klang-Ausgabe des Spiels (GAME-DESIGN 17.1).
 *
 * Aufgaben:
 * - latenzfreies Abspielen auf Tastendruck (vorab entschluesselte Puffer)
 * - fester Vorrat an Abspiel-Plaetzen statt Erzeugung je Wiedergabe
 * - Tonhoehen-Streuung von wenigen Cent gegen den Maschinengewehr-Effekt
 * - drei getrennte Lautstaerke-Wege
 * - dreiteilige Klaenge (Ansatz, Koerper in Schleife, Ausklang)
 *
 * Die Ausgabe ist vom Kampf entkoppelt: Sie liest nur, was passiert ist, und
 * beeinflusst keine Rechnung. Ein stummes Spiel rechnet identisch.
 */
export class AudioEngine {
  readonly library = new SoundLibrary();
  readonly mixer: Mixer;
  readonly pool: VoicePool;

  private readonly random: AudioRandom;
  private readonly resolveUrl: (src: string) => string;

  /** Kennzahlen fuer Messungen — keine davon wird im Abspielpfad erzeugt. */
  private plays = 0;
  private sourcesCreated = 0;
  private missing = 0;
  private readonly rateHistory = new Float64Array(RATE_HISTORY);
  private rateWrite = 0;

  constructor(
    readonly ctx: BaseAudioContext,
    options: AudioEngineOptions = {},
  ) {
    this.mixer = new Mixer(ctx);
    this.pool = new VoicePool(ctx, this.mixer, options.poolSizes ?? DEFAULT_POOL_SIZES);
    this.random = new AudioRandom(options.seed ?? 0x2f6e2b1);
    this.resolveUrl = options.resolveUrl ?? ((src) => src);
  }

  /**
   * Nach der ersten Nutzergeste aufrufen. Browser starten einen AudioContext
   * angehalten; ohne dieses Aufwecken bleibt alles stumm.
   */
  async unlock(): Promise<void> {
    const ctx = this.ctx as BaseAudioContext & { resume?: () => Promise<void> };
    if (ctx.state === 'suspended' && typeof ctx.resume === 'function') {
      await ctx.resume();
    }
  }

  async load(): Promise<LoadReport> {
    return this.library.loadAll(this.ctx, this.resolveUrl);
  }

  // ------------------------------------------------------------------ Regler

  setVolume(bus: Bus, value: number): void {
    this.mixer.setVolume(bus, value);
  }

  volume(bus: Bus): number {
    return this.mixer.volume(bus);
  }

  setMasterVolume(value: number): void {
    this.mixer.setMasterVolume(value);
  }

  // --------------------------------------------------------------- Abspielen

  /**
   * Einen One-Shot abspielen — der heisse Pfad.
   *
   * Erzeugt genau ein Objekt: den `AudioBufferSourceNode`, den die Web-Audio-
   * Spezifikation je Wiedergabe verlangt (er ist einmal verwendbar). Alles
   * andere — Lautstaerke-Knoten, Ende-Handler, Platzverwaltung — ist vorab
   * angelegt und wird wiederverwendet (GAME-DESIGN 18.2).
   *
   * @param when Startzeitpunkt in der Uhr des Kontexts. 0 = sofort.
   */
  play(id: string, when = 0): VoiceHandle {
    const def = this.library.def(id);
    if (!def || def.kind !== 'oneshot') {
      this.missing++;
      return NO_VOICE;
    }
    return this.startOneShot(def, when);
  }

  private startOneShot(def: OneShotDef, when: number): VoiceHandle {
    const buffer = this.library.buffer(def.src);
    if (!buffer) {
      this.missing++;
      return NO_VOICE;
    }

    const now = this.ctx.currentTime;
    const at = when > now ? when : now;
    const rate = this.rateFor(def.pitchJitterCents ?? 0);
    const slot = this.pool.acquire(def.bus);

    const source = this.ctx.createBufferSource();
    this.sourcesCreated++;
    source.buffer = buffer;
    source.playbackRate.value = rate;
    source.connect(slot.gain);
    source.onended = slot.onEnded;

    setGainNow(slot.gain.gain, def.gain ?? 1, now);
    source.start(at);

    slot.source = source;
    slot.defId = def.id;
    slot.active = true;
    slot.looping = false;
    slot.endsAt = at + buffer.duration / rate;
    this.plays++;
    return this.pool.handleOf(slot);
  }

  // --------------------------------------------------- Dreiteilige Klaenge

  /**
   * Einen dreiteiligen Klang starten: Ansatz einmal, Koerper in Schleife.
   *
   * Der Bezeichner zeigt auf den Koerper — er ist der Teil, der laeuft, bis
   * der Spieleffekt endet oder abgebrochen wird.
   */
  startSustained(id: string): VoiceHandle {
    const def = this.library.def(id);
    if (!def || def.kind !== 'sustained') {
      this.missing++;
      return NO_VOICE;
    }
    const body = this.library.buffer(def.bodySrc);
    if (!body) {
      this.missing++;
      return NO_VOICE;
    }

    const now = this.ctx.currentTime;
    const target = def.gain ?? 1;
    const rate = this.rateFor(def.pitchJitterCents ?? 0);

    if (def.attackSrc) {
      const attack = this.library.buffer(def.attackSrc);
      if (attack) this.startRaw(def.bus, attack, def.id, target, 1, now);
    }

    const slot = this.pool.acquire(def.bus);
    const source = this.ctx.createBufferSource();
    this.sourcesCreated++;
    source.buffer = body;
    source.playbackRate.value = rate;
    source.loop = true;
    source.loopStart = def.loopStart ?? 0;
    // 0 bedeutet in der Spezifikation "bis zum Ende des Puffers".
    source.loopEnd = def.loopEnd ?? 0;
    source.connect(slot.gain);
    source.onended = slot.onEnded;

    const fadeIn = def.fadeInSeconds ?? 0;
    const gain = slot.gain.gain;
    gain.cancelScheduledValues(now);
    if (fadeIn > 0) {
      gain.setValueAtTime(0, now);
      gain.linearRampToValueAtTime(target, now + fadeIn);
    } else {
      gain.setValueAtTime(target, now);
    }
    source.start(now);

    slot.source = source;
    slot.defId = def.id;
    slot.active = true;
    slot.looping = true;
    // Eine Schleife endet nicht von selbst: Sie ist beim Stimmen-Raeumen der
    // letzte Kandidat.
    slot.endsAt = Number.POSITIVE_INFINITY;
    this.plays++;
    return this.pool.handleOf(slot);
  }

  /**
   * Einen dreiteiligen Klang beenden: Koerper ausblenden und stoppen, danach
   * den Ausklang einmal abspielen.
   *
   * Ohne Ausblendung knackt es — ein Schleifen-Koerper wird mitten im Signal
   * abgeschnitten und der Sprung auf null ist hoerbar.
   */
  stopSustained(handle: VoiceHandle): void {
    const slot = this.pool.resolve(handle);
    if (!slot) return;

    const def = slot.defId ? this.library.def(slot.defId) : undefined;
    const sustained = def && def.kind === 'sustained' ? def : undefined;
    const fadeOut = sustained?.fadeOutSeconds ?? DEFAULT_FADE_OUT;
    const now = this.ctx.currentTime;

    const gain = slot.gain.gain;
    gain.cancelScheduledValues(now);
    gain.setValueAtTime(gain.value, now);
    gain.linearRampToValueAtTime(0, now + fadeOut);
    // Der Ende-Handler gibt den Platz frei, sobald die Quelle wirklich steht.
    slot.source?.stop(now + fadeOut);
    slot.endsAt = now + fadeOut;
    slot.looping = false;

    if (sustained?.releaseSrc) {
      const release = this.library.buffer(sustained.releaseSrc);
      if (release) this.startRaw(sustained.bus, release, sustained.id, sustained.gain ?? 1, 1, now);
    }
  }

  /** Laeuft die Stimme zu diesem Bezeichner noch? */
  isPlaying(handle: VoiceHandle): boolean {
    return this.pool.resolve(handle) !== null;
  }

  /** Eine einzelne Stimme sofort beenden — ohne Ausklang. */
  stop(handle: VoiceHandle): void {
    const slot = this.pool.resolve(handle);
    if (slot) this.pool.release(slot);
  }

  stopAll(): void {
    this.pool.releaseAll();
  }

  // ----------------------------------------------------------------- Innerei

  /** Gemeinsamer Kern fuer Ansatz und Ausklang. */
  private startRaw(
    bus: Bus,
    buffer: AudioBuffer,
    defId: string,
    gainValue: number,
    rate: number,
    now: number,
  ): void {
    const slot = this.pool.acquire(bus);
    const source = this.ctx.createBufferSource();
    this.sourcesCreated++;
    source.buffer = buffer;
    source.playbackRate.value = rate;
    source.connect(slot.gain);
    source.onended = slot.onEnded;
    setGainNow(slot.gain.gain, gainValue, now);
    source.start(now);
    slot.source = source;
    slot.defId = defId;
    slot.active = true;
    slot.looping = false;
    slot.endsAt = now + buffer.duration / rate;
    this.plays++;
  }

  /**
   * Abspielgeschwindigkeit aus der Streuung in Cent.
   *
   * 100 Cent sind ein Halbton; wenige Cent hoert man als Lebendigkeit, nicht
   * als Verstimmung. Bei tonalen Klaengen steht im Datensatz eine kleine oder
   * gar keine Streuung (GAME-DESIGN 17.1).
   */
  private rateFor(cents: number): number {
    const rate = cents === 0 ? 1 : Math.pow(2, this.random.centered(cents) / 1200);
    this.rateHistory[this.rateWrite] = rate;
    this.rateWrite = (this.rateWrite + 1) % RATE_HISTORY;
    return rate;
  }

  // ------------------------------------------------------------- Messgroessen

  /**
   * Momentaufnahme fuer Messungen und Entwicklungs-Werkzeuge.
   *
   * Wird ausserhalb des Abspielpfads gerufen; dass hier ein Objekt entsteht,
   * ist unkritisch.
   */
  metrics(): {
    plays: number;
    sourcesCreated: number;
    missing: number;
    activeVoices: number;
    stolenVoices: number;
    poolSize: number;
  } {
    return {
      plays: this.plays,
      sourcesCreated: this.sourcesCreated,
      missing: this.missing,
      activeVoices: this.pool.activeCount(),
      stolenVoices: this.pool.stolenCount(),
      poolSize: this.pool.size(),
    };
  }

  /** Die zuletzt verwendeten Abspielgeschwindigkeiten, aelteste zuerst. */
  recentRates(count = RATE_HISTORY): number[] {
    const n = Math.min(count, RATE_HISTORY, this.plays);
    const out: number[] = [];
    for (let i = n; i > 0; i--) {
      const index = (this.rateWrite - i + RATE_HISTORY) % RATE_HISTORY;
      out.push(this.rateHistory[index] ?? 1);
    }
    return out;
  }
}

/** Lautstaerke eines Platzes setzen und laufende Automation verwerfen. */
function setGainNow(param: AudioParam, value: number, now: number): void {
  param.cancelScheduledValues(now);
  param.setValueAtTime(value, now);
}
