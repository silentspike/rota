/**
 * Der Beute-Moment (GAME-DESIGN 13.6, Issue E6-4).
 *
 * > Common erscheint sofort und beiläufig, Rare mit kurzem Aufblitzen, Epic
 * > mit deutlichem Effekt — **ein Legendary hält alles an**: eigener Klang,
 * > Bild verdunkelt sich, Gegenstand erscheint groß. **99 von 100 Siegen
 * > laufen schnell durch, der eine Moment brennt sich ein.**
 *
 * # Warum die Staffelung mehr ist als Zierrat
 *
 * Der letzte Satz ist die eigentliche Regel: Der seltene Moment wirkt nur,
 * **weil** die häufigen kurz sind. Eine Inszenierung, die auch bei Common
 * eine Sekunde kostet, macht den hundertsten Sieg zäh und den Legendary-Fund
 * gewöhnlich — sie zerstört genau das, wofür sie da ist.
 *
 * # Vollständig überspringbar
 *
 * > „**Vollständig überspringbar** — beim hundertsten Sieg will man zu den
 * > Zahlen."
 *
 * Auch der Legendary-Moment. Ein Moment, den man nicht abbrechen kann, ist
 * beim zweiten Mal eine Zumutung.
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */

import { QUALITY_COUNT, QUALITY_LEGENDARY, QUALITY_NAMES } from '../gear/budget.js';

/** Wie ein Fund inszeniert wird. */
export interface Presentation {
  /** Wie lange die Inszenierung läuft, in Millisekunden. */
  readonly durationMs: number;
  /**
   * Hält sie den Ablauf an?
   *
   * **Nur beim Legendary** — das ist der Unterschied, der den Moment trägt.
   */
  readonly halts: boolean;
  /** Wie stark sich das Bild verdunkelt, 0 bis 1. */
  readonly dim: number;
  /** Der Klang, der dazu gespielt wird. Leer heißt: keiner. */
  readonly sound: string;
  /** Die CSS-Klasse, an der die Anzeige die Stufe erkennt. */
  readonly className: string;
}

/**
 * Die vier Inszenierungen. **Die Zahlen sind vorläufig**, die Ordnung nicht:
 * Jede Stufe muss deutlicher sein als die darunter, und nur die höchste hält
 * an.
 */
const PRESENTATIONS: readonly Presentation[] = [
  // Common — „sofort und beiläufig".
  { durationMs: 400, halts: false, dim: 0, sound: '', className: 'rota-loot-common' },
  // Rare — „kurzes Aufblitzen".
  {
    durationMs: 900,
    halts: false,
    dim: 0.1,
    sound: 'loot-rare',
    className: 'rota-loot-rare',
  },
  // Epic — „deutlicher Effekt".
  {
    durationMs: 1600,
    halts: false,
    dim: 0.25,
    sound: 'loot-epic',
    className: 'rota-loot-epic',
  },
  // Legendary — „hält alles an": eigener Klang, Bild verdunkelt sich,
  // Gegenstand erscheint groß.
  {
    durationMs: 3200,
    halts: true,
    dim: 0.75,
    sound: 'loot-legendary',
    className: 'rota-loot-legendary',
  },
];

/** Die Inszenierung einer Qualität. */
export function presentationFor(quality: number): Presentation {
  const stufe = PRESENTATIONS[quality];
  return stufe ?? (PRESENTATIONS[0] as Presentation);
}

/** Der Name der Qualität, wie er am Fund steht — englisch (Design-Gesetz 5). */
export function labelFor(quality: number): string {
  const name = QUALITY_NAMES[quality] ?? QUALITY_NAMES[0] ?? '';
  return name.toUpperCase();
}

/**
 * Sind die vier Inszenierungen tatsächlich unterscheidbar?
 *
 * Der Prüffall zu AC3 in einer Funktion — und zwar so, dass er **jedes** Paar
 * ansieht. „Vier verschiedene" heißt nicht „die erste und die letzte
 * unterscheiden sich".
 */
export function presentationsAreDistinct(): boolean {
  for (let a = 0; a < QUALITY_COUNT; a++) {
    for (let b = a + 1; b < QUALITY_COUNT; b++) {
      const eine = presentationFor(a);
      const andere = presentationFor(b);
      if (
        eine.durationMs === andere.durationMs ||
        eine.className === andere.className ||
        eine.dim === andere.dim
      ) {
        return false;
      }
    }
  }
  return true;
}

/**
 * Der laufende Beute-Moment.
 *
 * Vorab angelegt, damit im Takt nichts entsteht (18.2). Ein Moment ist
 * Zustand, kein Objekt je Fund.
 */
export class LootMoment {
  private quality = -1;
  private startedAtMs = 0;
  private skipped = false;
  private shownCount = 0;
  private skippedCount = 0;

  get isRunning(): boolean {
    return this.quality >= 0;
  }

  get currentQuality(): number {
    return this.quality;
  }

  /** Hält der laufende Moment den Ablauf an? */
  get halts(): boolean {
    return this.isRunning && presentationFor(this.quality).halts && !this.skipped;
  }

  get shownTotal(): number {
    return this.shownCount;
  }

  get skippedTotal(): number {
    return this.skippedCount;
  }

  /** Einen Fund zeigen. */
  show(quality: number, nowMs: number): void {
    this.quality = quality;
    this.startedAtMs = nowMs;
    this.skipped = false;
    this.shownCount++;
  }

  /**
   * **Die Taste.** Bricht die Inszenierung sofort ab — auch die des
   * Legendary.
   *
   * Gibt `false` zurück, wenn gar nichts lief: Ein Tastendruck ins Leere ist
   * kein Überspringen und wird nicht gezählt.
   */
  skip(): boolean {
    if (!this.isRunning) return false;
    this.quality = -1;
    this.skipped = true;
    this.skippedCount++;
    return true;
  }

  /** Läuft der Moment zu diesem Zeitpunkt noch? */
  isActiveAt(nowMs: number): boolean {
    if (!this.isRunning) return false;
    return nowMs - this.startedAtMs < presentationFor(this.quality).durationMs;
  }

  /** Den Moment beenden, wenn seine Zeit um ist. */
  step(nowMs: number): void {
    if (this.isRunning && !this.isActiveAt(nowMs)) this.quality = -1;
  }

  reset(): void {
    this.quality = -1;
    this.startedAtMs = 0;
    this.skipped = false;
    this.shownCount = 0;
    this.skippedCount = 0;
  }
}

/** Der höchste Wert, den [`presentationFor`] kennt — für Prüfungen. */
export const HIGHEST_PRESENTATION = QUALITY_LEGENDARY;
