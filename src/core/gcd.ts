/**
 * Die globale Wartezeit (GAME-DESIGN 6.6).
 *
 * Alle Zauber lösen dieselbe Wartezeit aus — der gleichmäßige Grundtakt ist die
 * Voraussetzung dafür, dass Muskelgedächtnis entsteht. Genau eine Ausnahme ist
 * vorgesehen: Unterbrechen löst sie nicht aus (Epic E4).
 *
 * Ganzzahlige Zeit in Millisekunden (GAME-DESIGN 18.4): Ein Fließkomma-Zähler
 * würde über hunderte Takte driften, und Spiel und Simulator müssten auf denselben
 * Wert kommen.
 */

/** Grundwert. Per Talent später bis etwa 1,0 s senkbar (GAME-DESIGN 6.4). */
export const GCD_MS = 1500;

export class GlobalCooldown {
  /** Zeitpunkt, ab dem wieder gewirkt werden darf. */
  private readyAtMs = 0;
  private durationMs = GCD_MS;

  /** Dauer der Wartezeit. Ab Epic E5 von CATALYSIS beeinflusst. */
  get lengthMs(): number {
    return this.durationMs;
  }

  setLength(durationMs: number): void {
    if (!Number.isInteger(durationMs) || durationMs < 0) {
      throw new RangeError(`Wartezeit muss ganzzahlig und nicht negativ sein: ${durationMs}`);
    }
    this.durationMs = durationMs;
  }

  isReady(nowMs: number): boolean {
    return nowMs >= this.readyAtMs;
  }

  remainingMs(nowMs: number): number {
    const left = this.readyAtMs - nowMs;
    return left > 0 ? left : 0;
  }

  /** Anteil der abgelaufenen Wartezeit, 0 bis 1. Für die Anzeige. */
  progress(nowMs: number): number {
    if (this.durationMs === 0) return 1;
    const left = this.remainingMs(nowMs);
    if (left === 0) return 1;
    return 1 - left / this.durationMs;
  }

  /** Startet die Wartezeit neu. */
  trigger(nowMs: number): void {
    this.readyAtMs = nowMs + this.durationMs;
  }

  get readyAt(): number {
    return this.readyAtMs;
  }

  /**
   * Zieht Millisekunden von der laufenden Wartezeit ab (RETORT, Issue #480).
   *
   * **Nicht dasselbe wie `reset`**: Der räumt sie ganz weg, das hier gibt
   * einen bestimmten Betrag zurück — nämlich den, den ein Boss-Abbruch
   * gekostet hat.
   *
   * Bei null landet sie, nie darunter. Ein Betrag, der grösser wäre als die
   * Restzeit, macht sie sofort bereit; eine negative Bereitschaftszeit hätte
   * keine Bedeutung und ginge in die Prüfsumme ein.
   *
   * Spiegelbild zu `gcd_ready_at.saturating_sub(...)` in `sim/src/fight.rs`.
   */
  refund(ms: number): void {
    if (ms <= 0) return;
    this.readyAtMs = Math.max(0, this.readyAtMs - ms);
  }

  reset(): void {
    this.readyAtMs = 0;
  }
}
