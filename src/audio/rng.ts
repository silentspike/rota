/**
 * Eigener Zufallsstrom fuer die Tonhoehen-Streuung.
 *
 * Warum nicht `Math.random()`: Das Design verlangt getrennte Zufallsstroeme,
 * damit eine zusaetzliche Ziehung nicht alles Nachfolgende verschiebt
 * (GAME-DESIGN 18.4). Der Klang darf die Stroeme der Kampfrechnung unter
 * keinen Umstaenden beruehren — er ist rein kosmetisch und geht in keine
 * Schadensformel ein.
 *
 * Nebeneffekt: Mit gesetztem Startwert sind Tests wiederholbar.
 *
 * Verfahren: xorshift32. Genug fuer eine Streuung von wenigen Cent, erzeugt
 * keinen Muell und braucht keine Bibliothek.
 */
export class AudioRandom {
  private state: number;

  constructor(seed = 0x9e3779b9) {
    this.state = seed >>> 0 || 0x9e3779b9;
  }

  /** Gleichverteilt in [0, 1). */
  next(): number {
    let x = this.state;
    x ^= x << 13;
    x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    this.state = x;
    return x / 0x100000000;
  }

  /** Gleichverteilt in [-span, +span]. */
  centered(span: number): number {
    return (this.next() * 2 - 1) * span;
  }

  reseed(seed: number): void {
    this.state = seed >>> 0 || 0x9e3779b9;
  }
}
