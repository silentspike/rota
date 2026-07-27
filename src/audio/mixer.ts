import { BUSES, type Bus } from './types.js';

/**
 * Zeitkonstante der Reglerbewegung in Sekunden. Ohne Rampe knackt jede
 * Aenderung, weil der Pegel im Signal springt.
 */
const RAMP_SECONDS = 0.02;

/**
 * Die drei getrennten Lautstaerke-Wege plus Gesamt-Regler (GAME-DESIGN 17.1).
 *
 * ```
 *   Stimmen (music) ─┐
 *                    ├─> busGain[music] ─┐
 *   Stimmen (sfx) ───┼─> busGain[sfx] ───┼─> master ─> Ausgang
 *   Stimmen (ui) ────┴─> busGain[ui] ────┘
 * ```
 *
 * Die Knoten werden einmal beim Start angelegt und nie wieder umgesteckt —
 * Umstecken je Wiedergabe waere Arbeit im heissen Pfad (GAME-DESIGN 18.2).
 */
export class Mixer {
  readonly master: GainNode;
  private readonly busGains = new Map<Bus, GainNode>();
  private readonly volumes = new Map<Bus, number>();
  private masterVolumeValue = 1;

  constructor(private readonly ctx: BaseAudioContext) {
    this.master = ctx.createGain();
    this.master.gain.value = 1;
    this.master.connect(ctx.destination);

    for (const bus of BUSES) {
      const gain = ctx.createGain();
      gain.gain.value = 1;
      gain.connect(this.master);
      this.busGains.set(bus, gain);
      this.volumes.set(bus, 1);
    }
  }

  /** Eingang eines Weges — hier haengen die Stimmen dieses Busses. */
  input(bus: Bus): GainNode {
    const gain = this.busGains.get(bus);
    if (!gain) throw new Error(`Unbekannter Bus: ${bus}`);
    return gain;
  }

  /**
   * Reglerstellung 0..1.
   *
   * Umgerechnet wird quadratisch (`gain = v * v`). Ein linearer Regler fuehlt
   * sich falsch an: Bei halber Stellung erwartet das Ohr etwa ein Viertel der
   * Leistung, nicht die Haelfte. 0 ist exakt 0 — Musik aus heisst wirklich aus.
   */
  setVolume(bus: Bus, value: number): void {
    const clamped = clamp01(value);
    this.volumes.set(bus, clamped);
    rampTo(this.input(bus).gain, clamped * clamped, this.ctx.currentTime);
  }

  volume(bus: Bus): number {
    return this.volumes.get(bus) ?? 1;
  }

  setMasterVolume(value: number): void {
    this.masterVolumeValue = clamp01(value);
    rampTo(this.master.gain, this.masterVolumeValue * this.masterVolumeValue, this.ctx.currentTime);
  }

  masterVolume(): number {
    return this.masterVolumeValue;
  }
}

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0;
  if (value < 0) return 0;
  if (value > 1) return 1;
  return value;
}

function rampTo(param: AudioParam, target: number, now: number): void {
  // cancelScheduledValues + setValueAtTime(aktueller Wert) verhindert, dass
  // eine noch laufende Rampe die neue ueberspringt.
  param.cancelScheduledValues(now);
  param.setValueAtTime(param.value, now);
  param.linearRampToValueAtTime(target, now + RAMP_SECONDS);
}
