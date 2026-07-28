import { beforeEach, describe, expect, it } from 'vitest';
import { AudioEngine } from '../../src/audio/engine.js';
import { MusicDirector } from '../../src/audio/music.js';
import type { SustainedDef } from '../../src/audio/types.js';
import {
  FakeAudioBuffer,
  FakeAudioContext,
  FakeBufferSource,
  asBuffer,
  asContext,
  type FakeGainNode,
} from '../helpers/fake-audio.js';

function layerDef(id: string): SustainedDef {
  return { kind: 'sustained', id, bus: 'music', bodySrc: `audio/${id}.ogg`, pitchJitterCents: 0 };
}

const LAYERS = ['bed', 'tension', 'push'] as const;

let ctx: FakeAudioContext;
let engine: AudioEngine;
let music: MusicDirector;

function gainOf(id: string): FakeGainNode {
  const handle = (music as unknown as { states: Map<string, { handle: number }> }).states.get(id);
  const slot = engine.pool.resolve(handle!.handle);
  return slot!.gain as unknown as FakeGainNode;
}

beforeEach(() => {
  ctx = new FakeAudioContext();
  engine = new AudioEngine(asContext(ctx));
  const defs = LAYERS.map(layerDef);
  engine.library.register(defs);
  for (const def of defs) engine.library.put(def.bodySrc, asBuffer(new FakeAudioBuffer(8)));
  music = new MusicDirector(engine, { barSeconds: 2, fadeSeconds: 1 });
});

describe('MusicDirector — Schichten, die auf den Kampf reagieren (GAME-DESIGN 17.1)', () => {
  it('startet alle Schichten, hoerbar ist nur der Grundklang', () => {
    const failed = music.start([{ id: 'bed', base: true }, { id: 'tension' }, { id: 'push' }]);
    expect(failed).toEqual([]);
    expect(music.runningLayers()).toEqual(['bed', 'tension', 'push']);
    expect(music.activeLayers()).toEqual(['bed']);
    expect(gainOf('bed').gain.value).toBe(1);
    expect(gainOf('tension').gain.value).toBe(0);
  });

  it('laesst stumme Schichten wirklich mitlaufen — sonst gaebe es keinen Gleichlauf', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }]);
    const source = engine.pool.resolve(
      (music as unknown as { states: Map<string, { handle: number }> }).states.get('tension')!
        .handle,
    )!.source as unknown as FakeBufferSource;
    expect(source.startedAt).toBe(0);
    expect(source.loop).toBe(true);
  });

  it('meldet Schichten, deren Datei fehlt, statt sie zu verschweigen', () => {
    const failed = music.start([{ id: 'bed', base: true }, { id: 'gibt-es-nicht' }]);
    expect(failed).toEqual(['gibt-es-nicht']);
    expect(music.runningLayers()).toEqual(['bed']);
  });

  it('schaltet eine Schicht einzeln zu und wieder ab', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }, { id: 'push' }]);
    music.enable('tension');
    expect(music.activeLayers()).toEqual(['bed', 'tension']);
    expect(music.isEnabled('push')).toBe(false);
    music.disable('tension');
    expect(music.activeLayers()).toEqual(['bed']);
  });

  it('blendet um, statt den Pegel springen zu lassen', () => {
    music.start([
      { id: 'bed', base: true },
      { id: 'tension', gain: 0.7 },
    ]);
    const gain = gainOf('tension').gain;
    gain.events.length = 0;
    music.enable('tension');
    const ramp = gain.events.find((e) => e.type === 'ramp');
    expect(ramp?.value).toBe(0.7);
    // Blende von einer Sekunde ab der Taktgrenze bei 2 s.
    expect(ramp?.time).toBeCloseTo(3, 10);
  });

  it('legt den Uebergang auf die naechste Taktgrenze', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }]);
    ctx.currentTime = 2.4;
    expect(music.nextBar()).toBeCloseTo(4, 10);
    ctx.currentTime = 5.9;
    expect(music.nextBar()).toBeCloseTo(6, 10);
  });

  it('blendet auf Wunsch sofort — fuer das, was nicht warten kann', () => {
    music.start([{ id: 'bed', base: true }, { id: 'overpressure' }, { id: 'tension' }]);
    ctx.currentTime = 2.4;
    const gain = gainOf('tension').gain;
    gain.events.length = 0;
    music.enable('tension', true);
    const ramp = gain.events.find((e) => e.type === 'ramp');
    expect(ramp?.time).toBeCloseTo(3.4, 10);
  });

  it('ignoriert das Zuschalten einer bereits laufenden Schicht', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }]);
    music.enable('tension');
    const gain = gainOf('tension').gain;
    const before = gain.events.length;
    music.enable('tension');
    expect(gain.events.length).toBe(before);
  });

  it('kennt keine unbekannte Schicht', () => {
    music.start([{ id: 'bed', base: true }]);
    expect(music.enable('gibt-es-nicht')).toBe(false);
  });

  it('blendet beim Ausklang alles herunter', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }]);
    music.enable('tension');
    music.fadeOutAll();
    expect(music.activeLayers()).toEqual([]);
    expect(gainOf('bed').gain.value).toBe(0);
  });

  it('haelt an und gibt alle Stimmen frei', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }, { id: 'push' }]);
    expect(engine.metrics().activeVoices).toBe(3);
    music.stop();
    expect(engine.metrics().activeVoices).toBe(0);
    expect(music.isRunning()).toBe(false);
  });

  it('startet neu, ohne Stimmen liegen zu lassen', () => {
    music.start([{ id: 'bed', base: true }, { id: 'tension' }]);
    music.start([{ id: 'bed', base: true }, { id: 'push' }]);
    expect(engine.metrics().activeVoices).toBe(2);
    expect(music.runningLayers()).toEqual(['bed', 'push']);
  });
});
