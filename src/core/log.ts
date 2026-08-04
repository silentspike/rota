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
 * | 8   | `bossEnergyTriggered` | — | ja     |
 * | 9   | `bossTimelineFired`   | — | ja     |
 * | 10  | `playerDamage`        | — | ja     |
 * | 11  | `playerDeath`         | — | ja     |
 * | 12  | `interruptLanded`     | — | ja     |
 * | 13  | `shieldAbsorb`        | — | ja     |
 * | 14  | `healed`              | — | ja     |
 * | 15  | `convertedToShield`   | — | ja     |
 *
 * Nummer 6 hat mit Issue #96 eine zweite Schreiberin bekommen: Der Simulator
 * führt Boss-Phasen jetzt ebenfalls. Dieselbe Zahl, dieselbe Bedeutung — genau
 * dafür ist der gemeinsame Raum da. 8 und 9 sind neu und gehören vorerst allein
 * dem Simulator, wie `spend` und `kill` auch.
 *
 * Die Namen stehen auch für die Arten, die diese Seite nie schreibt: Wer ein
 * fremdes Protokoll liest, soll denselben Namen sehen und nicht ein Fragezeichen.
 *
 * `tests/parity/recordings/log-kinds.json` hält die Tabelle fest, und
 * `sim/tests/log_kinds.rs` prüft die andere Seite dagegen.
 */
/**
 * **Die eine Quelle des Nummernraums** (Issue #138).
 *
 * Schlüssel ist der Name, Wert die Kennnummer — beides untrennbar in einem
 * Eintrag. Eine Art hinzuzufügen bringt zwangsläufig beides mit.
 *
 * # Was hier bis Issue #138 stand
 *
 * **Zwei parallele Listen von Hand**: die `EVENT_`-Konstanten mit ihren
 * Nummern, und `EVENT_NAMES`, in dem der **Index die Nummer war**. Wer eine
 * Konstante anlegte und die Namensliste nicht erweiterte, bekam an dieser
 * Stelle `undefined` — und umgekehrt.
 *
 * Das war nicht nur unschön. `logKindsPayload()` erzeugt
 * `tests/parity/recordings/log-kinds.json` aus dieser Liste, und genau diese
 * Datei ist die Gegenseite, gegen die `sim/tests/log_kinds.rs` prüft. Wären
 * die beiden Spiel-Listen auseinandergelaufen, hätte der Wächter **gegen eine
 * falsche Wahrheit verglichen**, ohne es bemerken zu können.
 *
 * Auf der Rust-Seite ist derselbe Fehler seit #134 unmöglich — dort erzeugt
 * ein Makro Varianten, Aufzählung und Namen aus einer Liste. Hier tut es
 * dieses Objekt.
 *
 * # Warum die Nummern ausgeschrieben stehen
 *
 * Sie ließen sich aus der Reihenfolge ableiten. Sie stehen trotzdem da, weil
 * der Nummernraum ein **Format zwischen zwei Programmen** ist: Wer einen
 * Eintrag in der Mitte einfügt, verschiebt sonst stillschweigend alle
 * folgenden Nummern — und ein aufgezeichnetes Protokoll bedeutete danach
 * etwas anderes. Ausgeschrieben sieht man beim Einfügen, dass man eine neue
 * Nummer vergeben muss.
 */
const EVENT_KINDS = {
  cast: 0,
  damage: 1,
  dotTick: 2,
  crit: 3,
  spend: 4,
  kill: 5,
  phase: 6,
  buffExpire: 7,
  bossEnergyTriggered: 8,
  bossTimelineFired: 9,
  playerDamage: 10,
  playerDeath: 11,
  interruptLanded: 12,
  shieldAbsorb: 13,
  healed: 14,
  convertedToShield: 15,
  controlSpent: 16,
} as const;

export const EVENT_CAST = EVENT_KINDS.cast;
export const EVENT_DAMAGE = EVENT_KINDS.damage;
export const EVENT_DOT_TICK = EVENT_KINDS.dotTick;
export const EVENT_CRIT = EVENT_KINDS.crit;
/** Nur der Simulator schreibt sie — hier steht sie für den gemeinsamen Raum. */
export const EVENT_SPEND = EVENT_KINDS.spend;
/** Nur der Simulator schreibt sie. */
export const EVENT_KILL = EVENT_KINDS.kill;
export const EVENT_PHASE = EVENT_KINDS.phase;
export const EVENT_BUFF_EXPIRE = EVENT_KINDS.buffExpire;
/**
 * Die Energieleiste des Bosses hat **ausgelöst** (Issue #96).
 *
 * Der Name sagt ausdrücklich `Triggered`, weil dies ein **Vorgang** ist und
 * kein Zustand: Geschrieben wird nur im Moment des Auslösens, nicht bei jedem
 * Anstieg. Der Stand der Leiste heißt `Combat.bossEnergy` und ist etwas
 * anderes — beim ersten Lesen wurden die beiden verwechselt, deshalb jetzt
 * zwei unterscheidbare Namen.
 *
 * Nur der Simulator schreibt sie.
 */
export const EVENT_BOSS_ENERGY_TRIGGERED = EVENT_KINDS.bossEnergyTriggered;
/**
 * Ein Eintrag der Boss-Zeitleiste ist **fällig geworden** (Issue #96).
 *
 * Auch hier ein Vorgang, kein Zustand: Der Zeiger heißt
 * `Combat.timelineIndex`.
 *
 * Nur der Simulator schreibt sie.
 */
export const EVENT_BOSS_TIMELINE_FIRED = EVENT_KINDS.bossTimelineFired;
/** Nur der Simulator schreibt sie — Schaden am Spieler (Issue #110). */
export const EVENT_PLAYER_DAMAGE = EVENT_KINDS.playerDamage;
/** Nur der Simulator schreibt sie — der Spieler ist gestorben (#110). */
export const EVENT_PLAYER_DEATH = EVENT_KINDS.playerDeath;
/** Nur der Simulator schreibt sie — ein Boss-Zauber wurde unterbrochen (#110). */
export const EVENT_INTERRUPT_LANDED = EVENT_KINDS.interruptLanded;
/** Nur der Simulator schreibt sie — die Schutzschicht hat aufgefangen (#79). */
export const EVENT_SHIELD_ABSORB = EVENT_KINDS.shieldAbsorb;
/** Nur der Simulator schreibt sie — der Spieler wurde geheilt (#79). */
export const EVENT_HEALED = EVENT_KINDS.healed;
/** Nur der Simulator schreibt sie — Schaden wurde in Schutz verwandelt (#79). */
export const EVENT_CONVERTED_TO_SHIELD = EVENT_KINDS.convertedToShield;
/** Nur der Simulator schreibt sie — Kontroll-Punkte ausgegeben (Issue #100). */
export const EVENT_CONTROL_SPENT = EVENT_KINDS.controlSpent;

/**
 * Die Namen, **Index gleich Kennnummer** — aus [`EVENT_KINDS`] erzeugt.
 *
 * Über die Werte aufgebaut und nicht über die Schlüsselreihenfolge: So hängt
 * die Zuordnung an der ausgeschriebenen Nummer und nicht daran, in welcher
 * Reihenfolge jemand die Einträge hingeschrieben hat.
 */
export const EVENT_NAMES: readonly string[] = (() => {
  const namen: string[] = [];
  for (const [name, kind] of Object.entries(EVENT_KINDS)) namen[kind] = name;
  return namen;
})();

/** Welche Arten **diese** Seite schreibt. Der Rest gehört dem Simulator. */
export const EVENT_KINDS_WRITTEN_BY_GAME: readonly number[] = [
  EVENT_CAST,
  EVENT_DAMAGE,
  EVENT_DOT_TICK,
  EVENT_CRIT,
  EVENT_PHASE,
  EVENT_BUFF_EXPIRE,
  // **Seit Issue #151 schreibt das Spiel diese fünf ebenfalls.** Bis dahin
  // führte sie nur der Simulator — und die Fehleranalyse läuft im Browser,
  // nicht im Simulator. Ein Protokoll, in dem die Größe überall fehlt, kann
  // eine Gelegenheit nicht belegen (GAME-DESIGN 15.2, Issue #81).
  EVENT_BOSS_TIMELINE_FIRED,
  EVENT_PLAYER_DAMAGE,
  EVENT_INTERRUPT_LANDED,
  EVENT_SHIELD_ABSORB,
  EVENT_HEALED,
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
