import { beforeEach, describe, expect, it } from 'vitest';
import { Mixer } from '../../src/audio/mixer.js';
import { NO_VOICE } from '../../src/audio/types.js';
import { DEFAULT_POOL_SIZES, VoicePool, type PoolSizes } from '../../src/audio/voice-pool.js';
import {
  FakeAudioContext,
  FakeBufferSource,
  asContext,
  type FakeGainNode,
} from '../helpers/fake-audio.js';

const SMALL: PoolSizes = { music: 1, sfx: 2, ui: 1 };

let ctx: FakeAudioContext;
let pool: VoicePool;

function build(sizes = SMALL): VoicePool {
  ctx = new FakeAudioContext();
  const mixer = new Mixer(asContext(ctx));
  return new VoicePool(asContext(ctx), mixer, sizes);
}

/** Index-Anteil eines Bezeichners (untere 8 Bit). */
function handleIndex(handle: number): number {
  return handle & 0xff;
}

/** Eine Quelle so belegen, wie die Ausgabe es tut. */
function occupy(slotIndexBus: 'music' | 'sfx' | 'ui', endsAt: number): number {
  const slot = pool.acquire(slotIndexBus);
  const source = new FakeBufferSource();
  source.start(0);
  source.onended = slot.onEnded;
  slot.source = source as unknown as AudioBufferSourceNode;
  slot.active = true;
  slot.endsAt = endsAt;
  return pool.handleOf(slot);
}

describe('VoicePool — vorab angelegte Abspiel-Plaetze (GAME-DESIGN 18.2)', () => {
  beforeEach(() => {
    pool = build();
  });

  it('legt alle Lautstaerke-Knoten beim Bauen an', () => {
    // 4 Knoten fuer den Mixer, danach je Platz einer.
    expect(ctx.gainsCreated).toBe(4 + SMALL.music + SMALL.sfx + SMALL.ui);
    expect(pool.size()).toBe(4);
    expect(pool.size('sfx')).toBe(2);
  });

  it('erzeugt beim Holen eines Platzes keinen neuen Knoten', () => {
    const before = ctx.gainsCreated;
    for (let i = 0; i < 50; i++) {
      const slot = pool.acquire('sfx');
      slot.active = true;
      slot.endsAt = i;
    }
    expect(ctx.gainsCreated).toBe(before);
  });

  it('haengt jeden Platz fest an seinem Weg', () => {
    const slot = pool.acquire('ui');
    expect(slot.bus).toBe('ui');
    expect((slot.gain as unknown as FakeGainNode).connections).toHaveLength(1);
  });

  it('loest einen Bezeichner auf, solange die Stimme laeuft', () => {
    const handle = occupy('sfx', 1);
    expect(pool.resolve(handle)).not.toBeNull();
  });

  it('entwertet den Bezeichner, sobald der Platz neu vergeben wurde', () => {
    const handle = occupy('music', 1);
    const slot = pool.resolve(handle);
    expect(slot).not.toBeNull();
    pool.release(slot!);
    expect(pool.resolve(handle)).toBeNull();

    // Derselbe Platz, neue Stimme: der alte Bezeichner darf sie nicht treffen.
    const second = occupy('music', 2);
    expect(second).not.toBe(handle);
    expect(pool.resolve(handle)).toBeNull();
    expect(pool.resolve(second)).not.toBeNull();
  });

  it('kennt keinen Bezeichner NO_VOICE', () => {
    expect(pool.resolve(NO_VOICE)).toBeNull();
  });

  it('raeumt bei Ueberlauf den Platz, der als naechstes ohnehin auslaeuft', () => {
    const early = occupy('sfx', 1.0);
    const late = occupy('sfx', 9.0);
    expect(pool.activeCount('sfx')).toBe(2);

    const reused = pool.acquire('sfx');
    expect(pool.stolenCount()).toBe(1);
    expect(pool.resolve(early)).toBeNull();
    expect(pool.resolve(late)).not.toBeNull();
    // Der geraeumte Platz ist genau der, den die frueher endende Stimme hatte.
    expect(reused.index).toBe(handleIndex(early));
    expect(reused.active).toBe(false);
  });

  it('raeumt Schleifen zuletzt — sie enden nicht von selbst', () => {
    const loop = occupy('sfx', Number.POSITIVE_INFINITY);
    const oneShot = occupy('sfx', 5);
    pool.acquire('sfx');
    expect(pool.resolve(oneShot)).toBeNull();
    expect(pool.resolve(loop)).not.toBeNull();
  });

  it('stoppt und trennt die Quelle beim Freigeben', () => {
    const handle = occupy('ui', 2);
    const slot = pool.resolve(handle)!;
    const source = slot.source as unknown as FakeBufferSource;
    pool.release(slot);
    expect(source.stoppedAt).not.toBeNull();
    expect(source.onended).toBeNull();
    expect(source.disconnected).toBe(1);
    expect(slot.source).toBeNull();
  });

  it('gibt den Platz frei, wenn die Quelle von selbst endet', () => {
    const handle = occupy('sfx', 1);
    const slot = pool.resolve(handle)!;
    (slot.source as unknown as FakeBufferSource).finish();
    expect(pool.resolve(handle)).toBeNull();
    expect(pool.activeCount()).toBe(0);
  });

  it('raeumt mit releaseAll alles ab', () => {
    occupy('sfx', 1);
    occupy('music', 1);
    occupy('ui', 1);
    expect(pool.activeCount()).toBe(3);
    pool.releaseAll();
    expect(pool.activeCount()).toBe(0);
  });

  it('bleibt mit dem Standard-Vorrat innerhalb des Bezeichner-Formats', () => {
    const big = build(DEFAULT_POOL_SIZES);
    expect(big.size()).toBe(
      DEFAULT_POOL_SIZES.music + DEFAULT_POOL_SIZES.sfx + DEFAULT_POOL_SIZES.ui,
    );
    expect(big.size()).toBeLessThanOrEqual(256);
  });
});
