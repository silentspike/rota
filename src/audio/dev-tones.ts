/**
 * Synthetische Klaenge fuer Messungen — Entwicklungs-Werkzeug, nie im Spiel.
 *
 * Warum kuenstlich und nicht die echten Dateien: Bei einer erzeugten Schleife
 * ist die Naht **rechnerisch bekannt**. Ein Sinus mit ganzzahlig vielen Perioden
 * im Puffer muss beim Umlauf stetig weiterlaufen; jede Stufe im Signal ist dann
 * eindeutig ein Fehler der Ausgabe und keine Eigenschaft der Datei.
 *
 * Die echten Klaenge werden zusaetzlich geprueft (Laenge, Pegel, Anfangs-Stille,
 * Clipping) — aber sie taugen nicht als Massstab fuer die Naht.
 */

/**
 * Sinus-Schleife, deren Puffer exakt `cycles` volle Perioden enthaelt.
 *
 * Dadurch ist der letzte Abtastwert der natuerliche Vorgaenger des ersten:
 * eine ideale Naht.
 */
export function makeSeamlessTone(
  ctx: BaseAudioContext,
  frequency = 220,
  cycles = 22,
  amplitude = 0.5,
): AudioBuffer {
  const sampleRate = ctx.sampleRate;
  const length = Math.round((cycles * sampleRate) / frequency);
  const buffer = ctx.createBuffer(1, length, sampleRate);
  const data = buffer.getChannelData(0);
  const step = (2 * Math.PI * cycles) / length;
  for (let i = 0; i < length; i++) data[i] = Math.sin(i * step) * amplitude;
  return buffer;
}

/** Kurzer perkussiver Klang mit exponentiellem Abklingen — steht fuer einen Treffer. */
export function makeClick(
  ctx: BaseAudioContext,
  seconds = 0.12,
  frequency = 660,
  amplitude = 0.7,
): AudioBuffer {
  const sampleRate = ctx.sampleRate;
  const length = Math.round(seconds * sampleRate);
  const buffer = ctx.createBuffer(1, length, sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;
    data[i] = Math.sin(2 * Math.PI * frequency * t) * Math.exp(-t * 30) * amplitude;
  }
  return buffer;
}

/** Rauschartiger Einschlag — steht fuer einen Zauber-Treffer ohne Tonhoehe. */
export function makeNoiseHit(ctx: BaseAudioContext, seconds = 0.3, amplitude = 0.6): AudioBuffer {
  const sampleRate = ctx.sampleRate;
  const length = Math.round(seconds * sampleRate);
  const buffer = ctx.createBuffer(1, length, sampleRate);
  const data = buffer.getChannelData(0);
  // Fester Startwert: Messungen sollen wiederholbar sein.
  let state = 0x1234567;
  for (let i = 0; i < length; i++) {
    state ^= state << 13;
    state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    const noise = state / 0x100000000 - 0.5;
    data[i] = noise * 2 * Math.exp(-(i / sampleRate) * 12) * amplitude;
  }
  return buffer;
}
