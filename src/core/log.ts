/**
 * Das Kampfprotokoll (GAME-DESIGN 18.2, 18.3).
 *
 * **Vorab angelegter Ringpuffer mit gepackten Zahlen** — keine Objekte. Ein
 * Zehn-Minuten-Kampf erzeugt rund 10.000 Einträge; als Objektliste wäre das
 * Aufräum-Last mitten im Kampf und damit genau die Ruckelquelle, die
 * GAME-DESIGN 18.2 ausschließt.
 *
 * Vier Zahlen je Eintrag, in vier getrennten Feldern statt in einem Objekt:
 *
 * | Feld      | Typ     | Bytes | Inhalt                                    |
 * | --------- | ------- | ----- | ----------------------------------------- |
 * | Zeitpunkt | Int32   | 4     | Kampfzeit in ms                           |
 * | Art       | Uint8   | 1     | siehe `EVENT_*`                           |
 * | Zauber    | Int8    | 1     | Kennnummer im Kampf, −1 wenn ohne Bezug   |
 * | Wert      | Float64 | 8     | Schaden, Stapel, Phase — je nach Art      |
 *
 * 14 Byte je Eintrag, 10.000 Einträge also **140 KB**, einmal beim Start
 * angefordert und danach nie wieder.
 *
 * **Das Protokoll bleibt intern** (GAME-DESIGN 21.2): Es gibt keinen sichtbaren
 * Textstrom. Lesbar ist es nur über die Entwicklungs-Werkzeuge und später über
 * die Auswertung.
 */

/**
 * Die Arten eines Protokoll-Eintrags — **ein gemeinsamer Nummernraum mit dem
 * Simulator** (Issue E3-6).
 *
 * Bis dahin führten beide Seiten eigene Nummern, und ab der 2 bedeutete
 * dieselbe Zahl etwas anderes: Eine 3 hieß im Simulator „Entladung", hier
 * „DoT-Tick"; eine 4 dort „Boss gefallen", hier „Phasenwechsel". Verglichen
 * hat das nie jemand, deshalb ist es nie aufgefallen — die Auswertung in Epic
 * E7 liest aber genau diese Daten, und eine verschobene Bedeutung ergäbe eine
 * Fehleranalyse, die plausibel aussieht und falsch ist.
 *
 * **Jede Seite benutzt nur ihre Teilmenge**, statt leere Einträge zu führen:
 *
 * | Nr. | Art          | Spiel | Simulator |
 * | --- | ------------ | ----- | --------- |
 * | 0   | `cast`       | ja    | ja        |
 * | 1   | `damage`     | ja    | ja        |
 * | 2   | `dotTick`    | ja    | ja        |
 * | 3   | `crit`       | ja    | —         |
 * | 4   | `spend`      | —     | ja        |
 * | 5   | `kill`       | —     | ja        |
 * | 6   | `phase`      | ja    | —         |
 * | 7   | `buffExpire` | ja    | —         |
 *
 * Die Namen stehen auch für die Arten, die diese Seite nie schreibt: Wer ein
 * fremdes Protokoll liest, soll denselben Namen sehen und nicht ein Fragezeichen.
 *
 * `tests/parity/recordings/log-kinds.json` hält die Tabelle fest, und
 * `sim/tests/log_kinds.rs` prüft die andere Seite dagegen.
 */
export const EVENT_CAST = 0;
export const EVENT_DAMAGE = 1;
export const EVENT_DOT_TICK = 2;
export const EVENT_CRIT = 3;
/** Nur der Simulator schreibt sie — hier steht sie für den gemeinsamen Raum. */
export const EVENT_SPEND = 4;
/** Nur der Simulator schreibt sie. */
export const EVENT_KILL = 5;
export const EVENT_PHASE = 6;
export const EVENT_BUFF_EXPIRE = 7;

export const EVENT_NAMES: readonly string[] = [
  'cast',
  'damage',
  'dotTick',
  'crit',
  'spend',
  'kill',
  'phase',
  'buffExpire',
];

/** Welche Arten **diese** Seite schreibt. Der Rest gehört dem Simulator. */
export const EVENT_KINDS_WRITTEN_BY_GAME: readonly number[] = [
  EVENT_CAST,
  EVENT_DAMAGE,
  EVENT_DOT_TICK,
  EVENT_CRIT,
  EVENT_PHASE,
  EVENT_BUFF_EXPIRE,
];

/** Kein Zauber-Bezug. */
export const NO_SPELL_REF = -1;

/** Auslegung für einen Zehn-Minuten-Kampf (GAME-DESIGN 18.2). */
export const LOG_CAPACITY = 10_000;

/** Ein Eintrag, wie ihn die Entwicklungs-Werkzeuge ausgeben. */
export interface LogEntry {
  readonly timeMs: number;
  /** Name aus `EVENT_NAMES` — VERIFICATION § 3.4 filtert darauf. */
  readonly type: string;
  readonly spell: number;
  readonly value: number;
}

export class CombatLog {
  private readonly times: Int32Array;
  private readonly types: Uint8Array;
  private readonly spellRefs: Int8Array;
  private readonly values: Float64Array;

  private writeIndex = 0;
  private filled = 0;
  private written = 0;

  constructor(capacity: number = LOG_CAPACITY) {
    if (!Number.isInteger(capacity) || capacity <= 0) {
      throw new RangeError(`Protokollgröße muss positiv und ganzzahlig sein: ${capacity}`);
    }
    this.times = new Int32Array(capacity);
    this.types = new Uint8Array(capacity);
    this.spellRefs = new Int8Array(capacity);
    this.values = new Float64Array(capacity);
  }

  get capacity(): number {
    return this.times.length;
  }

  /** Gültige Einträge — höchstens `capacity`. */
  get length(): number {
    return this.filled;
  }

  /** Alle je geschriebenen Einträge, auch die überschriebenen. */
  get totalWritten(): number {
    return this.written;
  }

  /**
   * Wie viele Einträge überschrieben wurden.
   *
   * **Muss 0 bleiben.** Ein Wert darüber heißt, dass der Puffer zu klein
   * ausgelegt ist — das wird gemeldet, nicht stillschweigend hingenommen.
   */
  get droppedCount(): number {
    return this.written - this.filled;
  }

  get hasOverflowed(): boolean {
    return this.written > this.times.length;
  }

  /** Bytes, die der Puffer belegt. */
  get byteLength(): number {
    return (
      this.times.byteLength +
      this.types.byteLength +
      this.spellRefs.byteLength +
      this.values.byteLength
    );
  }

  /** Einen Eintrag schreiben. Fordert keinen Speicher an. */
  push(timeMs: number, type: number, spell: number, value: number): void {
    const at = this.writeIndex;
    this.times[at] = timeMs;
    this.types[at] = type;
    this.spellRefs[at] = spell;
    this.values[at] = value;
    this.writeIndex = at + 1 === this.times.length ? 0 : at + 1;
    if (this.filled < this.times.length) this.filled++;
    this.written++;
  }

  /** Position des ältesten noch vorhandenen Eintrags. */
  private startIndex(): number {
    return this.filled === this.times.length ? this.writeIndex : 0;
  }

  private slot(index: number): number {
    const pos = this.startIndex() + index;
    return pos < this.times.length ? pos : pos - this.times.length;
  }

  timeAt(index: number): number {
    return this.times[this.slot(index)] as number;
  }

  typeAt(index: number): number {
    return this.types[this.slot(index)] as number;
  }

  spellAt(index: number): number {
    return this.spellRefs[this.slot(index)] as number;
  }

  valueAt(index: number): number {
    return this.values[this.slot(index)] as number;
  }

  /** Einträge einer Art zählen, ohne etwas zu erzeugen. */
  countOf(type: number): number {
    let count = 0;
    for (let i = 0; i < this.filled; i++) {
      if (this.types[this.slot(i)] === type) count++;
    }
    return count;
  }

  /** Summe der Werte einer Art. */
  sumOf(type: number): number {
    let sum = 0;
    for (let i = 0; i < this.filled; i++) {
      if (this.types[this.slot(i)] === type) sum += this.values[this.slot(i)] as number;
    }
    return sum;
  }

  clear(): void {
    this.writeIndex = 0;
    this.filled = 0;
    this.written = 0;
  }

  /**
   * Chronologische Ausgabe als Objektliste.
   *
   * **Erzeugt Speicher** — ausschließlich für die Entwicklungs-Werkzeuge und die
   * Auswertung nach dem Kampf. Niemals im laufenden Bild aufrufen.
   */
  toArray(): LogEntry[] {
    const out: LogEntry[] = new Array<LogEntry>(this.filled);
    for (let i = 0; i < this.filled; i++) {
      const at = this.slot(i);
      out[i] = {
        timeMs: this.times[at] as number,
        type: EVENT_NAMES[this.types[at] as number] ?? '?',
        spell: this.spellRefs[at] as number,
        value: this.values[at] as number,
      };
    }
    return out;
  }
}
