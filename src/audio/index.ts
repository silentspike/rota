/**
 * Klang-Ausgabe (Epic E8, GAME-DESIGN 17.1).
 *
 * Einstiegspunkt fuer den Rest des Spiels. Der Kampf ruft ausschliesslich
 * `play()` und `startSustained()`/`stopSustained()` — er kennt weder Puffer
 * noch Knoten.
 */
export { AudioEngine, type AudioEngineOptions } from './engine.js';
export { SoundLibrary, type LoadReport, type LoadFailure } from './library.js';
export { Mixer } from './mixer.js';
export { AudioRandom } from './rng.js';
export { DEFAULT_POOL_SIZES, VoicePool, type PoolSizes, type VoiceSlot } from './voice-pool.js';
export {
  BUSES,
  NO_VOICE,
  type Bus,
  type OneShotDef,
  type SoundDef,
  type SustainedDef,
  type VoiceHandle,
} from './types.js';

import { AudioEngine, type AudioEngineOptions } from './engine.js';
import { resolveAssetUrl } from './asset-urls.js';

export { resolveAssetUrl, knownAssets } from './asset-urls.js';

/**
 * Den fuer das Spiel richtigen Kontext anlegen.
 *
 * `latencyHint: 'interactive'` waehlt den kleinstmoeglichen Ausgabepuffer —
 * genau das, was ein Spiel braucht, dessen Klang unmittelbar auf den
 * Tastendruck folgen muss.
 *
 * Die Adressen der Dateien loest der Bau-Vorgang auf (`asset-urls.ts`).
 */
export function createGameAudio(options?: { seed?: number; sampleRate?: number }): AudioEngine {
  const ctx = new AudioContext(
    options?.sampleRate
      ? { latencyHint: 'interactive', sampleRate: options.sampleRate }
      : { latencyHint: 'interactive' },
  );
  const engineOptions: AudioEngineOptions =
    options?.seed !== undefined
      ? { seed: options.seed, resolveUrl: resolveAssetUrl }
      : { resolveUrl: resolveAssetUrl };
  return new AudioEngine(ctx, engineOptions);
}
