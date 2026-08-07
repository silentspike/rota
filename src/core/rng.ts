/**
 * Zufallsströme des Kampfes (GAME-DESIGN 18.4).
 *
 * **Getrennte Ströme je Verwendung.** Krit, DoT-Ticks und Heilung ziehen aus je
 * eigenen Strömen. Der Grund ist nicht Ordnungsliebe: Aus einem gemeinsamen Strom
 * würde eine **zusätzliche Ziehung alles Nachfolgende verschieben**, und ein Kampf
 * ließe sich nach der kleinsten Regeländerung nicht mehr nachrechnen. Genau davon
 * hängt der Abgleich mit dem Simulator ab.
 *
 * In Schritt 1 zieht noch niemand: PURITY ist auf Stufe 1 gleich 0, es gibt keinen
 * Krit und keine Heilung (STEP1-SPEC § 2). Die Ströme werden trotzdem von Anfang an
 * angelegt — nachträglich eingezogen würden sie jede bereits aufgezeichnete Runde
 * ungültig machen.
 *
 * Der Erzeuger ist `mulberry32`: klein, schnell, ohne Speicheranforderung je
 * Ziehung und über Sprachgrenzen hinweg in wenigen Zeilen identisch nachzubauen —
 * die Bedingung dafür, dass der Rust-Simulator dieselbe Folge liefert.
 */

/** Die Verwendungen, für die je ein eigener Strom geführt wird. */
export const STREAM_CRIT = 0;
/**
 * Der Krit-Wurf eines **DoT-Ticks** — seit Issue #602, davor lag er in
 * `STREAM_CRIT`.
 *
 * **Was er trennt:** Ohne ihn verschob jeder zusätzliche Tick die Krit-Würfe
 * aller gewirkten Zauber. Beide Seiten taten das gleich, die Parität war
 * intakt — und genau deshalb hat es zwei Monate niemand gesehen. Die Regel aus
 * GAME-DESIGN 18.4 schützt nicht die Parität, sondern die **Vergleichbarkeit
 * zweier Läufe mit verschiedenen Rotationen**: Wer einen DoT hinzufügte, maß
 * im Balance-Vergleich teilweise verschobenen Zufall statt der Rotation.
 *
 * Gefunden wurde es an der Lücke, die dieser Kommentar jetzt schliesst:
 * `STREAM_HEAL` darunter trug seine Begründung, dieser hier trug keine — und
 * bei ihm war der Zufall vorhanden, er wurde nur aus dem falschen Strom
 * gezogen. Zwei gleich aussehende leere Ströme, einer mit Grund, einer ohne.
 */
export const STREAM_DOT_TICK = 1;
export const STREAM_HEAL = 2;
/**
 * **STREAM_HEAL bleibt auf 2, obwohl heute nichts daraus zieht.** Alle drei
 * Koagulations-Wege sind deterministisch (Design 21.1 verwirft Zufalls-Procs
 * außer Krit). Ihn zu entfernen und später wieder einzufügen würde jede
 * aufgezeichnete Prüfsumme zweimal verschieben; ein reservierter Strom kostet
 * nichts.
 */
export const STREAM_COUNT = 3;

/**
 * **Der Beute-Strom gehört NICHT hierher — das ist eine Festlegung, keine
 * Nachlässigkeit.** Er steht in `src/loot/stream.ts`.
 *
 * Die Versuchung ist groß, ihn hier einzureihen: „alle Zufallsquellen an
 * einer Stelle" klingt aufgeräumt. Es wäre falsch, und der Grund steht in
 * 20.2: Eigene Ströme schützen davor, dass **eine zusätzliche Ziehung alles
 * Nachfolgende verschiebt**. Beute wird beim **Sieg** gewürfelt, nicht im
 * Kampf — sie kann nichts verschieben, und nichts kann sie verschieben.
 *
 * Die Kehrseite ist gemessen: Ein vierter Strom hängt in einem Kampf über
 * 7952 Takte **7952 Nullen** an die Zustands-Prüfsumme, für einen Strom, der
 * dort nie zieht. Das macht sie länger, nicht schärfer. Sie soll den
 * **Kampf**-Zustand beschreiben; Beute ist keiner.
 *
 * `tests/unit/loot-stream.test.ts` wacht darüber, dass er nicht doch hier
 * landet — wer ihn einträgt, verschiebt stillschweigend jede aufgezeichnete
 * Prüfsumme.
 */

export const STREAM_NAMES: readonly string[] = ['crit', 'dotTick', 'heal'];

/**
 * Ein Strom. Der Zustand ist eine einzige vorzeichenlose 32-Bit-Zahl; damit ist
 * er vollständig speicherbar und exakt wiederherstellbar.
 */
export class RandomStream {
  private state: number;
  private draws = 0;

  constructor(private readonly seedValue: number) {
    this.state = seedValue >>> 0;
  }

  get seed(): number {
    return this.seedValue;
  }

  /** Zahl der Ziehungen. In Schritt 1 muss sie 0 bleiben. */
  get drawCount(): number {
    return this.draws;
  }

  /** Nächster Wert in [0, 1). */
  next(): number {
    this.draws++;
    this.state = (this.state + 0x6d2b79f5) >>> 0;
    let t = this.state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }

  /** Trifft eine Wahrscheinlichkeit in [0, 1]. */
  chance(probability: number): boolean {
    if (probability <= 0) return false;
    if (probability >= 1) return true;
    return this.next() < probability;
  }

  reset(): void {
    this.state = this.seedValue >>> 0;
    this.draws = 0;
  }
}

/**
 * Der Startwert eines Stroms, abgeleitet aus dem Startwert des Kampfes.
 *
 * **Herausgezogen, damit auch ein Strom AUSSERHALB dieser Sammlung nachweislich
 * dieselbe Ableitung benutzen kann** — der Beute-Strom in `src/loot/stream.ts`
 * ist genau der, der hier den Index 3 bekäme. Stünde die Rechnung nur im
 * Konstruktor, wäre „dieselbe Ableitung" eine Behauptung statt eines Aufrufs.
 */
export function seedForStream(seed: number, index: number): number {
  return (Math.imul(seed >>> 0, 0x9e3779b1) + index * 0x85ebca6b) >>> 0;
}

/**
 * Alle Ströme eines Kampfes, aus einem einzigen Startwert abgeleitet.
 *
 * Aus dem Startwert des Kampfes wird je Strom ein eigener gemacht, damit sie sich
 * nicht überlappen — sonst lieferten zwei Ströme dieselbe Folge.
 */
export class RandomStreams {
  private readonly streams: readonly RandomStream[];

  constructor(readonly seed: number) {
    const list: RandomStream[] = [];
    for (let i = 0; i < STREAM_COUNT; i++) {
      list.push(new RandomStream(seedForStream(seed, i)));
    }
    this.streams = list;
  }

  stream(which: number): RandomStream {
    return this.streams[which] as RandomStream;
  }

  /** Ziehungen je Strom — für die Zustands-Prüfsumme und für Tests. */
  drawCounts(): number[] {
    return this.streams.map((s) => s.drawCount);
  }

  /** Summe aller Ziehungen. In Schritt 1 muss sie 0 bleiben. */
  get totalDraws(): number {
    let total = 0;
    for (const stream of this.streams) total += stream.drawCount;
    return total;
  }

  reset(): void {
    for (const stream of this.streams) stream.reset();
  }
}
