/**
 * Messungen an der Klang-Ausgabe — Entwicklungs-Werkzeug (GAME-DESIGN 19.1).
 *
 * Alles hier rechnet auf dem **fertig gemischten Signal**, gerendert in einem
 * `OfflineAudioContext`. Das ist der einzige ehrliche Weg: Ob eine Schleife
 * nahtlos laeuft oder ein Regler wirklich auf null geht, entscheidet das
 * Signal — nicht der Quelltext, der es angefordert hat.
 *
 * Claude kann nicht hoeren; diese Funktionen sind der Ersatz dafuer.
 */
import { AudioEngine } from './engine.js';
import { makeClick, makeSeamlessTone } from './dev-tones.js';
import { MusicDirector } from './music.js';
import type { OneShotDef, SustainedDef } from './types.js';

const SAMPLE_RATE = 48000;

// ---------------------------------------------------------------- Signalmasse

export function rms(data: Float32Array, from = 0, to = data.length): number {
  let sum = 0;
  let count = 0;
  for (let i = from; i < to; i++) {
    const v = data[i] ?? 0;
    sum += v * v;
    count++;
  }
  return count === 0 ? 0 : Math.sqrt(sum / count);
}

export function peak(data: Float32Array, from = 0, to = data.length): number {
  let max = 0;
  for (let i = from; i < to; i++) {
    const v = Math.abs(data[i] ?? 0);
    if (v > max) max = v;
  }
  return max;
}

/** Groesster Sprung zwischen zwei aufeinanderfolgenden Abtastwerten. */
export function maxStep(data: Float32Array, from = 1, to = data.length): number {
  let max = 0;
  for (let i = Math.max(1, from); i < to; i++) {
    const step = Math.abs((data[i] ?? 0) - (data[i - 1] ?? 0));
    if (step > max) max = step;
  }
  return max;
}

// ------------------------------------------------------------ Schleifen-Naht

export interface SeamReport {
  /** Wie viele Umlaeufe im gerenderten Signal liegen. */
  readonly rounds: number;
  /** Laenge eines Umlaufs in Abtastwerten. */
  readonly loopSamples: number;
  /** Groesster Sprung an den Nahtstellen. */
  readonly seamMaxStep: number;
  /** Groesster Sprung im uebrigen Signal — der natuerliche Massstab. */
  readonly signalMaxStep: number;
  /** Sprung an jeder einzelnen Naht. */
  readonly seamSteps: readonly number[];
  /** Groesste Abweichung zwischen erstem und spaeterem Umlauf (Drift). */
  readonly roundDrift: number;
}

/**
 * Eine Schleife ueber mehrere Umlaeufe rendern und die Nahtstellen vermessen.
 *
 * Bestanden heisst: Der Sprung an der Naht ist nicht groesser als der groesste
 * Sprung, den das Signal ohnehin macht. Dann gibt es dort keine Stufe, die man
 * als Knacken hoeren wuerde.
 */
export async function measureLoopSeam(rounds = 20): Promise<SeamReport> {
  const probe = new OfflineAudioContext(1, SAMPLE_RATE, SAMPLE_RATE);
  const tone = makeSeamlessTone(probe, 220, 22, 0.5);
  const loopSamples = tone.length;
  const total = loopSamples * (rounds + 1);

  const ctx = new OfflineAudioContext(1, total, SAMPLE_RATE);
  const engine = new AudioEngine(ctx);
  const def: SustainedDef = {
    kind: 'sustained',
    id: 'measure-loop',
    bus: 'music',
    bodySrc: 'measure://tone',
    pitchJitterCents: 0,
  };
  engine.library.register([def]);
  engine.library.put('measure://tone', makeSeamlessTone(ctx, 220, 22, 0.5));
  engine.startSustained('measure-loop');

  const rendered = await ctx.startRendering();
  const data = rendered.getChannelData(0);

  const seamSteps: number[] = [];
  for (let round = 1; round <= rounds; round++) {
    const at = round * loopSamples;
    if (at >= data.length) break;
    seamSteps.push(Math.abs((data[at] ?? 0) - (data[at - 1] ?? 0)));
  }

  // Groesster Sprung ohne die Nahtstellen: der Massstab, gegen den die Naht
  // gemessen wird.
  let signalMax = 0;
  for (let i = 1; i < loopSamples; i++) {
    const step = Math.abs((data[i] ?? 0) - (data[i - 1] ?? 0));
    if (step > signalMax) signalMax = step;
  }

  // Drift: Weicht der zwanzigste Umlauf vom ersten ab, laeuft die Schleife
  // nicht exakt rund.
  let drift = 0;
  const lastRound = Math.min(rounds, Math.floor(data.length / loopSamples) - 1);
  for (let i = 0; i < loopSamples; i++) {
    const a = data[i] ?? 0;
    const b = data[lastRound * loopSamples + i] ?? 0;
    const diff = Math.abs(a - b);
    if (diff > drift) drift = diff;
  }

  return {
    rounds: seamSteps.length,
    loopSamples,
    seamMaxStep: seamSteps.length === 0 ? 0 : Math.max(...seamSteps),
    signalMaxStep: signalMax,
    seamSteps,
    roundDrift: drift,
  };
}

// ------------------------------------------------------------- Regler-Wirkung

export interface BusMixReport {
  readonly musicOnly: number;
  readonly sfxOnly: number;
  readonly both: number;
  readonly musicMutedSfxAudible: number;
  readonly musicMutedTotal: number;
}

/**
 * Die drei Regler einzeln nachweisen.
 *
 * Gemessen wird der Effektivwert (RMS) des fertigen Signals — je einmal mit
 * Musik allein, Effekt allein, beiden zusammen und mit stummgeschalteter Musik.
 */
export async function measureBusIsolation(seconds = 1): Promise<BusMixReport> {
  return {
    // Nur die Musik klingt, beide Regler offen.
    musicOnly: await renderMix(seconds, 1, 1, { music: true, sfx: false }),
    // Nur der Effekt klingt, beide Regler offen.
    sfxOnly: await renderMix(seconds, 1, 1, { music: false, sfx: true }),
    both: await renderMix(seconds, 1, 1, { music: true, sfx: true }),
    // Musik-Regler auf null, beide Klaenge laufen: der Effekt muss bleiben.
    musicMutedSfxAudible: await renderMix(seconds, 0, 1, { music: true, sfx: true }),
    // Musik-Regler auf null, nur die Musik laeuft: es muss still sein.
    musicMutedTotal: await renderMix(seconds, 0, 1, { music: true, sfx: false }),
  };
}

/**
 * @param musicVolume Reglerstellung Musik
 * @param sfxVolume   Reglerstellung Effekte
 * @param play        Welche Klaenge ueberhaupt gestartet werden
 */
async function renderMix(
  seconds: number,
  musicVolume: number,
  sfxVolume: number,
  play: { music: boolean; sfx: boolean },
): Promise<number> {
  const ctx = new OfflineAudioContext(1, Math.round(seconds * SAMPLE_RATE), SAMPLE_RATE);
  const engine = new AudioEngine(ctx);
  const music: SustainedDef = {
    kind: 'sustained',
    id: 'measure-music',
    bus: 'music',
    bodySrc: 'measure://tone',
    pitchJitterCents: 0,
  };
  const hit: OneShotDef = {
    kind: 'oneshot',
    id: 'measure-hit',
    bus: 'sfx',
    src: 'measure://click',
    pitchJitterCents: 0,
  };
  engine.library.register([music, hit]);
  engine.library.put('measure://tone', makeSeamlessTone(ctx, 220, 22, 0.5));
  engine.library.put('measure://click', makeClick(ctx, seconds * 0.9, 660, 0.7));

  engine.setVolume('music', musicVolume);
  engine.setVolume('sfx', sfxVolume);
  if (play.sfx) engine.play('measure-hit');
  if (play.music) engine.startSustained('measure-music');

  const rendered = await ctx.startRendering();
  // Erst nach der Reglerrampe messen (20 ms), sonst zaehlt der Uebergang mit.
  return rms(rendered.getChannelData(0), Math.round(0.05 * SAMPLE_RATE));
}

// ------------------------------------------------------- Tonhoehen-Streuung

export interface PitchReport {
  readonly plays: number;
  readonly unique: number;
  readonly min: number;
  readonly max: number;
  readonly spreadPercent: number;
  readonly maxCents: number;
}

/** Wiederholungen desselben Klangs vermessen (GAME-DESIGN 17.1). */
export function measurePitchSpread(engine: AudioEngine, id: string, count = 20): PitchReport {
  for (let i = 0; i < count; i++) engine.play(id);
  const rates = engine.recentRates(count);
  const min = Math.min(...rates);
  const max = Math.max(...rates);
  return {
    plays: rates.length,
    unique: new Set(rates).size,
    min,
    max,
    spreadPercent: (max / min - 1) * 100,
    maxCents: Math.max(...rates.map((r) => Math.abs(1200 * Math.log2(r)))),
  };
}

// ------------------------------------------------------------ Musik-Schichten

export interface LayerSwitchReport {
  /** Groesster Sprung im Signal waehrend der Umblendung. */
  readonly transitionMaxStep: number;
  /** Groesster Sprung im uebrigen Signal — der Massstab. */
  readonly steadyMaxStep: number;
  /** Effektivwert vor dem Zuschalten. */
  readonly rmsBefore: number;
  /** Effektivwert danach. */
  readonly rmsAfter: number;
  /** Zeitpunkt, an dem die Blende begann (Sekunden). */
  readonly switchedAt: number;
}

/**
 * Eine Schicht zuschalten und das fertige Signal vermessen.
 *
 * Zwei Fragen auf einmal: Wird die Schicht ueberhaupt hoerbar (Effektivwert
 * vorher gegen nachher), und knackt es dabei (Sprung im Signal gegen den
 * Sprung, den das Signal ohnehin macht)?
 */
export async function measureLayerSwitch(seconds = 8): Promise<LayerSwitchReport> {
  const total = Math.round(seconds * SAMPLE_RATE);
  const ctx = new OfflineAudioContext(1, total, SAMPLE_RATE);
  const engine = new AudioEngine(ctx);

  const base: SustainedDef = {
    kind: 'sustained',
    id: 'measure-base',
    bus: 'music',
    bodySrc: 'measure://bed',
    pitchJitterCents: 0,
  };
  const extra: SustainedDef = {
    kind: 'sustained',
    id: 'measure-extra',
    bus: 'music',
    bodySrc: 'measure://layer',
    pitchJitterCents: 0,
  };
  engine.library.register([base, extra]);
  engine.library.put('measure://bed', makeSeamlessTone(ctx, 110, 11, 0.4));
  engine.library.put('measure://layer', makeSeamlessTone(ctx, 275, 22, 0.4));

  const music = new MusicDirector(engine, { barSeconds: 2, fadeSeconds: 1 });
  music.start([{ id: 'measure-base', base: true }, { id: 'measure-extra' }]);
  // Der Offline-Kontext steht bei 0, bis gerendert wird: Die Blende wird also
  // auf die naechste Taktgrenze nach dem Start terminiert und liegt damit
  // vorhersagbar im Signal.
  music.enable('measure-extra');
  const scheduled = music.nextBar(0);

  const rendered = await ctx.startRendering();
  const data = rendered.getChannelData(0);

  const from = Math.round((scheduled - 0.05) * SAMPLE_RATE);
  const to = Math.round((scheduled + 1.05) * SAMPLE_RATE);
  return {
    transitionMaxStep: maxStep(data, Math.max(1, from), Math.min(data.length, to)),
    steadyMaxStep: maxStep(data, Math.round(2.5 * SAMPLE_RATE), Math.round(3.5 * SAMPLE_RATE)),
    rmsBefore: rms(data, Math.round(0.5 * SAMPLE_RATE), Math.round(1.5 * SAMPLE_RATE)),
    rmsAfter: rms(data, data.length - Math.round(1.5 * SAMPLE_RATE), data.length),
    switchedAt: scheduled,
  };
}

export interface LayerSyncReport {
  readonly rounds: number;
  readonly loopSamples: number;
  /**
   * Groesste Abweichung zwischen der Summe zweier gleicher Schichten und dem
   * doppelten Einzelsignal. Null heisst: Sie laufen exakt gleich.
   */
  readonly maxDeviation: number;
  readonly peak: number;
}

/**
 * Gleichlauf zweier Schichten ueber viele Umlaeufe.
 *
 * Verfahren: Zwei Schichten mit **demselben** Puffer laufen lassen. Sind sie
 * gleichauf, ist die Summe exakt das doppelte Einzelsignal. Jede Verschiebung
 * — und jeder Drift, der sich ueber zwanzig Umlaeufe aufbaut — zeigt sich
 * sofort als Abweichung davon.
 */
export async function measureLayerSync(rounds = 20): Promise<LayerSyncReport> {
  const probe = new OfflineAudioContext(1, SAMPLE_RATE, SAMPLE_RATE);
  const loopSamples = makeSeamlessTone(probe, 220, 22, 0.4).length;
  const total = loopSamples * (rounds + 1);

  const render = async (layers: number): Promise<Float32Array> => {
    const ctx = new OfflineAudioContext(1, total, SAMPLE_RATE);
    const engine = new AudioEngine(ctx);
    const defs: SustainedDef[] = [];
    for (let i = 0; i < layers; i++) {
      defs.push({
        kind: 'sustained',
        id: `measure-sync-${i}`,
        bus: 'music',
        bodySrc: 'measure://sync',
        pitchJitterCents: 0,
      });
    }
    engine.library.register(defs);
    engine.library.put('measure://sync', makeSeamlessTone(ctx, 220, 22, 0.4));
    const music = new MusicDirector(engine, { barSeconds: 1, fadeSeconds: 0.01 });
    music.start(defs.map((def) => ({ id: def.id, base: true, gain: 1 })));
    const out = await ctx.startRendering();
    return out.getChannelData(0).slice();
  };

  const single = await render(1);
  const doubled = await render(2);

  let deviation = 0;
  let peak = 0;
  for (let i = 0; i < single.length; i++) {
    const expected = (single[i] ?? 0) * 2;
    const actual = doubled[i] ?? 0;
    const diff = Math.abs(actual - expected);
    if (diff > deviation) deviation = diff;
    if (Math.abs(actual) > peak) peak = Math.abs(actual);
  }

  return { rounds, loopSamples, maxDeviation: deviation, peak };
}

/**
 * Wirkt der Musik-Regler auf alle Schichten? Rendert dieselbe Aufstellung
 * einmal offen und einmal mit dem Regler auf null.
 */
export async function measureMusicVolume(seconds = 4): Promise<{ open: number; muted: number }> {
  const render = async (volume: number): Promise<number> => {
    const ctx = new OfflineAudioContext(1, Math.round(seconds * SAMPLE_RATE), SAMPLE_RATE);
    const engine = new AudioEngine(ctx);
    const defs: SustainedDef[] = ['a', 'b', 'c'].map((suffix) => ({
      kind: 'sustained',
      id: `measure-vol-${suffix}`,
      bus: 'music',
      bodySrc: 'measure://vol',
      pitchJitterCents: 0,
    }));
    engine.library.register(defs);
    engine.library.put('measure://vol', makeSeamlessTone(ctx, 220, 22, 0.4));
    engine.setVolume('music', volume);
    const music = new MusicDirector(engine, { barSeconds: 1, fadeSeconds: 0.01 });
    music.start(defs.map((def) => ({ id: def.id, base: true })));
    const out = await ctx.startRendering();
    return rms(out.getChannelData(0), Math.round(0.5 * SAMPLE_RATE));
  };
  return { open: await render(1), muted: await render(0) };
}

// --------------------------------------------------------- Aufraeum-Last

export interface FramePressureReport {
  readonly plays: number;
  readonly frames: number;
  readonly seconds: number;
  readonly maxFrameMs: number;
  readonly p99FrameMs: number;
  readonly framesOver16_7: number;
  readonly meanFrameMs: number;
}

/**
 * Wiedergaben ueber mehrere Sekunden verteilen und dabei die Bildzeiten messen.
 *
 * Warum dieser Umweg: `performance.memory` ist in Chrome grob gerastert — im
 * Kopflos-Betrieb aendert es sich selbst dann nicht, wenn tausend Objekte
 * wirklich erzeugt werden (nachgemessen). Ein Heap-Wert waere hier also kein
 * Nachweis, sondern eine Behauptung mit Zahl.
 *
 * Was zaehlt, ist ohnehin die Wirkung: Aufraeum-Last zeigt sich als Ruckeln
 * (GAME-DESIGN 18.2). Bleiben alle Bilder unter 16,7 ms, waehrend im Takt
 * Klaenge starten, entsteht keine spuerbare Last.
 */
export function measureFramePressure(
  play: () => void,
  seconds = 6,
  playsPerSecond = 20,
): Promise<FramePressureReport> {
  return new Promise((resolve) => {
    const frameTimes: number[] = [];
    const start = performance.now();
    let last = start;
    let plays = 0;
    let nextPlay = start;
    const interval = 1000 / playsPerSecond;

    const step = (): void => {
      const now = performance.now();
      frameTimes.push(now - last);
      last = now;
      // playsPerSecond <= 0 ergibt den Leerlauf-Vergleich: dieselbe Messung
      // ohne jede Wiedergabe. Ohne diesen Massstab sagt eine Bildzeit nichts.
      while (playsPerSecond > 0 && nextPlay <= now) {
        play();
        plays++;
        nextPlay += interval;
      }
      if (now - start < seconds * 1000) {
        requestAnimationFrame(step);
        return;
      }
      // Das erste Bild enthaelt den Anlauf und wird verworfen.
      const times = frameTimes.slice(1).sort((a, b) => a - b);
      const sum = times.reduce((acc, v) => acc + v, 0);
      resolve({
        plays,
        frames: times.length,
        seconds: (now - start) / 1000,
        maxFrameMs: times[times.length - 1] ?? 0,
        p99FrameMs: times[Math.min(times.length - 1, Math.floor(times.length * 0.99))] ?? 0,
        framesOver16_7: times.filter((t) => t > 16.7).length,
        meanFrameMs: times.length === 0 ? 0 : sum / times.length,
      });
    };
    requestAnimationFrame(step);
  });
}

// ------------------------------------------------------ Knoten-Buchfuehrung

export interface NodeCounters {
  gains: number;
  sources: number;
}

/**
 * Jede Knoten-Erzeugung des Kontexts mitzaehlen.
 *
 * Damit laesst sich am laufenden Spiel belegen, was im Abspielpfad tatsaechlich
 * entsteht — statt es aus dem Quelltext zu schliessen.
 */
export function instrumentContext(ctx: BaseAudioContext): NodeCounters {
  const counters: NodeCounters = { gains: 0, sources: 0 };
  const createGain = ctx.createGain.bind(ctx);
  const createBufferSource = ctx.createBufferSource.bind(ctx);
  ctx.createGain = () => {
    counters.gains++;
    return createGain();
  };
  ctx.createBufferSource = () => {
    counters.sources++;
    return createBufferSource();
  };
  return counters;
}
