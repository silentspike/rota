import { beforeEach, describe, expect, it } from 'vitest';
import { AudioEngine } from '../../src/audio/engine.js';
import { NO_VOICE, type OneShotDef, type SustainedDef } from '../../src/audio/types.js';
import {
  FakeAudioBuffer,
  FakeAudioContext,
  FakeBufferSource,
  asBuffer,
  asContext,
  type FakeGainNode,
} from '../helpers/fake-audio.js';

const HIT: OneShotDef = {
  kind: 'oneshot',
  id: 'spell-kindle',
  bus: 'sfx',
  src: 'audio/kindle.ogg',
  pitchJitterCents: 25,
};

const TONAL: OneShotDef = {
  kind: 'oneshot',
  id: 'ui-ready',
  bus: 'ui',
  src: 'audio/ready.ogg',
  // Tonaler Klang: keine Streuung, sonst wirkt es verstimmt (GAME-DESIGN 17.1).
  pitchJitterCents: 0,
  gain: 0.6,
};

const CHANNEL: SustainedDef = {
  kind: 'sustained',
  id: 'spell-channel',
  bus: 'sfx',
  attackSrc: 'audio/channel-attack.ogg',
  bodySrc: 'audio/channel-body.ogg',
  releaseSrc: 'audio/channel-release.ogg',
  loopStart: 0.25,
  loopEnd: 8.0,
  fadeInSeconds: 0.05,
  fadeOutSeconds: 0.2,
};

let ctx: FakeAudioContext;
let engine: AudioEngine;

function build(seed = 12345): AudioEngine {
  ctx = new FakeAudioContext();
  const created = new AudioEngine(asContext(ctx), { seed, poolSizes: { music: 4, sfx: 8, ui: 4 } });
  created.library.register([HIT, TONAL, CHANNEL]);
  created.library.put(HIT.src, asBuffer(new FakeAudioBuffer(1.2)));
  created.library.put(TONAL.src, asBuffer(new FakeAudioBuffer(0.4)));
  created.library.put(CHANNEL.attackSrc!, asBuffer(new FakeAudioBuffer(0.4)));
  created.library.put(CHANNEL.bodySrc, asBuffer(new FakeAudioBuffer(8.5)));
  created.library.put(CHANNEL.releaseSrc!, asBuffer(new FakeAudioBuffer(0.6)));
  return created;
}

/** Die zuletzt erzeugte Quelle des Platzes hinter einem Bezeichner. */
function sourceOf(handle: number): FakeBufferSource {
  const slot = engine.pool.resolve(handle);
  if (!slot?.source) throw new Error('Stimme laeuft nicht');
  return slot.source as unknown as FakeBufferSource;
}

describe('AudioEngine — One-Shots', () => {
  beforeEach(() => {
    engine = build();
  });

  it('spielt einen angemeldeten Klang und meldet einen gueltigen Bezeichner', () => {
    const handle = engine.play('spell-kindle');
    expect(handle).not.toBe(NO_VOICE);
    expect(engine.isPlaying(handle)).toBe(true);
    const source = sourceOf(handle);
    expect(source.buffer?.duration).toBe(1.2);
    expect(source.startedAt).toBe(0);
    expect(engine.metrics().plays).toBe(1);
  });

  it('meldet einen unbekannten Klang, statt zu werfen', () => {
    expect(engine.play('gibt-es-nicht')).toBe(NO_VOICE);
    expect(engine.metrics().missing).toBe(1);
    expect(engine.metrics().plays).toBe(0);
  });

  it('meldet einen angemeldeten Klang ohne geladene Datei', () => {
    engine.library.register([{ kind: 'oneshot', id: 'leer', bus: 'sfx', src: 'audio/fehlt.ogg' }]);
    expect(engine.play('leer')).toBe(NO_VOICE);
    expect(engine.metrics().missing).toBe(1);
  });

  it('haengt die Quelle an den Platz, nicht direkt an den Weg', () => {
    const handle = engine.play('spell-kindle');
    const slot = engine.pool.resolve(handle)!;
    expect(sourceOf(handle).connections).toEqual([slot.gain as unknown as FakeGainNode]);
  });

  it('uebernimmt die Grundlautstaerke des Klangs', () => {
    const handle = engine.play('ui-ready');
    const slot = engine.pool.resolve(handle)!;
    expect((slot.gain as unknown as FakeGainNode).gain.value).toBeCloseTo(0.6, 10);
  });

  it('gibt den Platz frei, sobald der Klang zu Ende ist', () => {
    const handle = engine.play('spell-kindle');
    sourceOf(handle).finish();
    expect(engine.isPlaying(handle)).toBe(false);
    expect(engine.metrics().activeVoices).toBe(0);
  });

  it('spielt zum angegebenen Zeitpunkt, wenn er in der Zukunft liegt', () => {
    ctx.currentTime = 10;
    const handle = engine.play('spell-kindle', 10.5);
    expect(sourceOf(handle).startedAt).toBe(10.5);
  });
});

describe('AudioEngine — Tonhoehen-Streuung (GAME-DESIGN 17.1)', () => {
  beforeEach(() => {
    engine = build();
  });

  it('klingt bei zwanzig schnellen Wiederholungen nicht zwanzigmal gleich', () => {
    for (let i = 0; i < 20; i++) {
      const handle = engine.play('spell-kindle');
      sourceOf(handle).finish();
    }
    const rates = engine.recentRates(20);
    expect(rates).toHaveLength(20);
    expect(new Set(rates).size).toBe(20);
  });

  it('bleibt innerhalb der im Datensatz erlaubten Cent', () => {
    for (let i = 0; i < 200; i++) {
      const handle = engine.play('spell-kindle');
      sourceOf(handle).finish();
    }
    const limit = Math.pow(2, 25 / 1200);
    for (const rate of engine.recentRates()) {
      expect(rate).toBeGreaterThanOrEqual(1 / limit);
      expect(rate).toBeLessThanOrEqual(limit);
    }
  });

  it('streut wirklich — nicht alles klebt am Mittelwert', () => {
    for (let i = 0; i < 200; i++) {
      const handle = engine.play('spell-kindle');
      sourceOf(handle).finish();
    }
    const rates = engine.recentRates();
    const spread = Math.max(...rates) - Math.min(...rates);
    // Bei +-25 Cent liegt die volle Spanne bei rund 2,9 %.
    expect(spread).toBeGreaterThan(0.02);
  });

  it('laesst tonale Klaenge unangetastet', () => {
    for (let i = 0; i < 10; i++) {
      const handle = engine.play('ui-ready');
      expect(sourceOf(handle).playbackRate.value).toBe(1);
      sourceOf(handle).finish();
    }
  });

  it('wiederholt sich mit gleichem Startwert exakt', () => {
    const first = build(999);
    engine = first;
    for (let i = 0; i < 10; i++) sourceOf(engine.play('spell-kindle')).finish();
    const a = engine.recentRates(10);

    engine = build(999);
    for (let i = 0; i < 10; i++) sourceOf(engine.play('spell-kindle')).finish();
    expect(engine.recentRates(10)).toEqual(a);
  });
});

describe('AudioEngine — nichts Neues im Abspielpfad (GAME-DESIGN 18.2)', () => {
  beforeEach(() => {
    engine = build();
  });

  it('erzeugt nach dem Aufwaermen keinen einzigen Lautstaerke-Knoten mehr', () => {
    const afterSetup = ctx.gainsCreated;
    for (let i = 0; i < 500; i++) {
      const handle = engine.play('spell-kindle');
      sourceOf(handle).finish();
    }
    expect(ctx.gainsCreated).toBe(afterSetup);
  });

  it('erzeugt je Wiedergabe genau eine Quelle — die Spezifikation laesst keine Wiederverwendung zu', () => {
    for (let i = 0; i < 500; i++) {
      const handle = engine.play('spell-kindle');
      sourceOf(handle).finish();
    }
    expect(ctx.sourcesCreated).toBe(500);
    expect(engine.metrics().sourcesCreated).toBe(500);
  });

  it('kommt mit dem festen Vorrat aus, auch wenn gehaemmert wird', () => {
    for (let i = 0; i < 100; i++) engine.play('spell-kindle');
    expect(engine.metrics().activeVoices).toBeLessThanOrEqual(engine.pool.size('sfx'));
    expect(engine.metrics().stolenVoices).toBeGreaterThan(0);
  });
});

describe('AudioEngine — dreiteilige Klaenge (GAME-DESIGN 17.1)', () => {
  beforeEach(() => {
    engine = build();
  });

  it('startet Ansatz und Koerper und liefert den Bezeichner des Koerpers', () => {
    const handle = engine.startSustained('spell-channel');
    expect(handle).not.toBe(NO_VOICE);
    expect(engine.metrics().activeVoices).toBe(2);
    const body = sourceOf(handle);
    expect(body.loop).toBe(true);
    expect(body.buffer?.duration).toBe(8.5);
  });

  it('setzt die Schleifen-Grenzen aus dem Datensatz', () => {
    const body = sourceOf(engine.startSustained('spell-channel'));
    expect(body.loopStart).toBe(0.25);
    expect(body.loopEnd).toBe(8.0);
  });

  it('blendet den Koerper ein statt ihn hart einzusetzen', () => {
    const handle = engine.startSustained('spell-channel');
    const gain = engine.pool.resolve(handle)!.gain as unknown as FakeGainNode;
    const types = gain.gain.events.map((e) => e.type);
    expect(types).toContain('ramp');
    const start = gain.gain.events.find((e) => e.type === 'set');
    expect(start?.value).toBe(0);
  });

  it('laeuft weiter, bis er beendet wird — eine Schleife endet nicht von selbst', () => {
    const handle = engine.startSustained('spell-channel');
    ctx.advance(60);
    expect(engine.isPlaying(handle)).toBe(true);
  });

  it('blendet beim Beenden aus und spielt den Ausklang', () => {
    const handle = engine.startSustained('spell-channel');
    const slot = engine.pool.resolve(handle)!;
    const body = slot.source as unknown as FakeBufferSource;
    ctx.currentTime = 5;
    engine.stopSustained(handle);

    const events = (slot.gain as unknown as FakeGainNode).gain.events;
    const last = events[events.length - 1];
    expect(last?.type).toBe('ramp');
    expect(last?.value).toBe(0);
    expect(last?.time).toBeCloseTo(5.2, 10);
    expect(body.stoppedAt).toBeCloseTo(5.2, 10);

    // Ansatz beendet, Koerper laeuft noch aus, Ausklang laeuft.
    const releaseRunning = [...Array(engine.pool.size()).keys()].some((i) => {
      const s = engine.pool.resolve(i | 0);
      return s?.defId === 'spell-channel' && !s.looping;
    });
    expect(releaseRunning).toBe(true);
  });

  it('ignoriert das Beenden eines laengst ausgelaufenen Bezeichners', () => {
    const handle = engine.startSustained('spell-channel');
    engine.stop(handle);
    expect(() => engine.stopSustained(handle)).not.toThrow();
  });

  it('lehnt einen One-Shot als dreiteiligen Klang ab', () => {
    expect(engine.startSustained('spell-kindle')).toBe(NO_VOICE);
    expect(engine.metrics().missing).toBe(1);
  });
});

describe('AudioEngine — Regler und Aufwecken', () => {
  beforeEach(() => {
    engine = build();
  });

  it('reicht die Reglerstellung an den Mixer durch', () => {
    engine.setVolume('music', 0);
    expect(engine.volume('music')).toBe(0);
    expect(engine.volume('sfx')).toBe(1);
  });

  it('weckt einen angehaltenen Kontext auf', async () => {
    ctx.state = 'suspended';
    await engine.unlock();
    expect(ctx.resumeCalls).toBe(1);
    expect(ctx.state).toBe('running');
  });

  it('weckt einen laufenden Kontext nicht erneut', async () => {
    await engine.unlock();
    expect(ctx.resumeCalls).toBe(0);
  });

  it('raeumt mit stopAll alles ab', () => {
    engine.play('spell-kindle');
    engine.startSustained('spell-channel');
    expect(engine.metrics().activeVoices).toBeGreaterThan(0);
    engine.stopAll();
    expect(engine.metrics().activeVoices).toBe(0);
  });
});
