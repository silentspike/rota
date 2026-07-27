import { describe, expect, it } from 'vitest';
import { Mixer } from '../../src/audio/mixer.js';
import { BUSES } from '../../src/audio/types.js';
import { FakeAudioContext, asContext, type FakeGainNode } from '../helpers/fake-audio.js';

function setup(): { ctx: FakeAudioContext; mixer: Mixer } {
  const ctx = new FakeAudioContext();
  return { ctx, mixer: new Mixer(asContext(ctx)) };
}

describe('Mixer — drei getrennte Lautstaerke-Wege (GAME-DESIGN 17.1)', () => {
  it('legt Gesamt-Regler und je einen Weg an, alle offen', () => {
    const { ctx, mixer } = setup();
    // 1 Gesamt + 3 Wege
    expect(ctx.gainsCreated).toBe(4);
    for (const bus of BUSES) {
      expect(mixer.volume(bus)).toBe(1);
      expect((mixer.input(bus) as unknown as FakeGainNode).gain.value).toBe(1);
    }
  });

  it('haengt jeden Weg am Gesamt-Regler und den am Ausgang', () => {
    const { ctx, mixer } = setup();
    for (const bus of BUSES) {
      const input = mixer.input(bus) as unknown as FakeGainNode;
      expect(input.connections).toContain(mixer.master as unknown as FakeGainNode);
    }
    expect((mixer.master as unknown as FakeGainNode).connections).toContain(ctx.destination);
  });

  it('rechnet die Reglerstellung quadratisch um', () => {
    const { mixer } = setup();
    mixer.setVolume('music', 0.5);
    expect(mixer.volume('music')).toBe(0.5);
    expect((mixer.input('music') as unknown as FakeGainNode).gain.value).toBeCloseTo(0.25, 10);
  });

  it('regelt einen Weg auf exakt null, ohne die anderen zu beruehren', () => {
    const { mixer } = setup();
    mixer.setVolume('music', 0);
    expect((mixer.input('music') as unknown as FakeGainNode).gain.value).toBe(0);
    expect((mixer.input('sfx') as unknown as FakeGainNode).gain.value).toBe(1);
    expect((mixer.input('ui') as unknown as FakeGainNode).gain.value).toBe(1);
  });

  it('bewegt den Regler ueber eine Rampe — ein Sprung im Pegel knackt', () => {
    const { ctx, mixer } = setup();
    ctx.currentTime = 3;
    mixer.setVolume('sfx', 0.25);
    const events = (mixer.input('sfx') as unknown as FakeGainNode).gain.events;
    expect(events.map((e) => e.type)).toEqual(['cancel', 'set', 'ramp']);
    const ramp = events[2];
    expect(ramp?.time).toBeGreaterThan(3);
    expect(ramp?.time).toBeLessThanOrEqual(3.05);
  });

  it('faengt Werte ausserhalb von 0..1 und NaN ab', () => {
    const { mixer } = setup();
    mixer.setVolume('ui', -1);
    expect(mixer.volume('ui')).toBe(0);
    mixer.setVolume('ui', 4);
    expect(mixer.volume('ui')).toBe(1);
    mixer.setVolume('ui', Number.NaN);
    expect(mixer.volume('ui')).toBe(0);
  });

  it('hat einen eigenen Gesamt-Regler', () => {
    const { mixer } = setup();
    mixer.setMasterVolume(0.5);
    expect(mixer.masterVolume()).toBe(0.5);
    expect((mixer.master as unknown as FakeGainNode).gain.value).toBeCloseTo(0.25, 10);
  });
});
