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
 * **Jede Seite benutzt nur ihre Teilmenge**, statt leere Einträge zu führen.
 * Welche Seite welche Art schreiben soll, steht in [`EVENT_KIND_INTENT`] —
 * **dort und nur dort**.
 *
 * # Warum hier keine Tabelle mehr steht (Issue #676)
 *
 * Bis zu diesem Issue stand an dieser Stelle dieselbe Aussage ein zweites
 * Mal, als Markdown-Tabelle. Sie lief mit der Wirklichkeit auseinander, und
 * niemand konnte es bemerken, weil keine Maschine sie liest:
 *
 * ```text
 *   Art  6  phase        stand auf "nur Spiel"      Simulator schreibt seit #96
 *   Art  7  buffExpire   stand auf "Spiel: ja"      keine Seite schreibt sie
 *   Art  8  bossEnergy   stand auf "nur Simulator"  Spiel schreibt seit #679
 *   Art 16  controlSpent fehlte ganz                die Tabelle endete bei 15
 * ```
 *
 * Vier Zeilen von siebzehn falsch, und der Text unter der Tabelle nannte die
 * Änderung an Nummer 6 sogar selbst — die Tabellenzeile darüber wurde beim
 * Schreiben dieses Satzes nicht mitgezogen.
 *
 * Die Namen stehen auch für die Arten, die diese Seite nie schreibt: Wer ein
 * fremdes Protokoll liest, soll denselben Namen sehen und nicht ein Fragezeichen.
 *
 * `tests/parity/recordings/log-kinds.json` hält den Nummernraum fest,
 * `sim/tests/log_kinds.rs` prüft die andere Seite dagegen, und
 * `tests/unit/log-absicht.test.ts` prüft die Absicht gegen echte Läufe.
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

/**
 * **Wer soll welche Art schreiben** — die Absicht als Datenstruktur (#676).
 *
 * # Warum das nicht länger ein Kommentar sein darf
 *
 * Bis zu diesem Issue stand dieselbe Aussage an drei Stellen: in der
 * Markdown-Tabelle oben, in `EVENT_KINDS_WRITTEN_BY_GAME` als handgeführte
 * Nummernliste, und im Code als tatsächliche Schreibstelle. Drei Wahrheiten,
 * die niemand gegeneinander prüfte — und alle drei liefen auseinander:
 *
 * ```text
 *   Art  6  phase          Tabelle "nur Spiel", Simulator schreibt seit #96
 *   Art  7  buffExpire     Tabelle "Spiel: ja", KEINE Seite schreibt sie
 *   Art  8  bossEnergy     Tabelle "nur Simulator", Spiel schreibt seit #679
 *   Art 16  controlSpent   in der Tabelle gar nicht geführt
 * ```
 *
 * Art 8 ist der Fall, der das Issue ausgelöst hat, und er zeigt die Klasse
 * am deutlichsten: #679 hat die Schreibstelle gebaut
 * (`combat.ts`, `EVENT_BOSS_ENERGY_TRIGGERED`) — die Nummernliste darunter
 * blieb unverändert, also zählt `logKindsPayload()` die Art bis heute nicht
 * mit. **Geschrieben und trotzdem nicht verglichen.**
 *
 * Deshalb ist die Absicht jetzt eine Struktur, die eine Maschine lesen kann,
 * und `EVENT_KINDS_WRITTEN_BY_GAME` wird daraus **abgeleitet** statt daneben
 * geführt. Wer eine Art einträgt, kann die Ableitung nicht vergessen.
 *
 * `tests/unit/log-absicht.test.ts` prüft diese Tabelle gegen die
 * Wirklichkeit: Steht hier `true` und schreibt niemand, wird er rot — und
 * umgekehrt.
 */
export interface KindIntent {
  /** Soll **das Spiel** diese Art schreiben? */
  readonly game: boolean;
  /** Soll **der Simulator** sie schreiben? */
  readonly sim: boolean;
  /**
   * Gesetzt, wenn die Absicht feststeht, die Umsetzung aber fehlt.
   *
   * **Das ist kein Freibrief, sondern eine Meldung.** Der Wächter in
   * `tests/unit/log-absicht.test.ts` nimmt eine so markierte Art vom
   * Soll-Ist-Vergleich aus — und nennt sie in seiner Ausgabe. Ohne dieses
   * Feld gäbe es nur zwei Möglichkeiten: die Absicht verbiegen, bis sie zum
   * Ist-Zustand passt (dann ist die Lücke unsichtbar), oder einen dauerhaft
   * roten Prüffall (dann ist der Zweig nicht mehr abnehmbar).
   *
   * Der Text muss das Issue nennen, unter dem die Lücke geführt wird.
   */
  readonly bekannteLuecke?: string;
}

export const EVENT_KIND_INTENT: Readonly<Record<string, KindIntent>> = {
  cast: { game: true, sim: true },
  damage: { game: true, sim: true },
  dotTick: { game: true, sim: true },
  crit: { game: true, sim: false },
  spend: { game: false, sim: true },
  kill: { game: false, sim: true },
  /** Seit #96 führt der Simulator Boss-Phasen ebenfalls. */
  phase: { game: true, sim: true },
  /**
   * **Die Absicht steht, die Umsetzung fehlt** — gemessen am 2026-08-09 (#676).
   *
   * Das Auslaufen einer Verstärkung wird auf beiden Seiten sehr wohl bemerkt:
   *
   * ```text
   *   src/core/combat.ts        this.buffs.expire(timeMs)
   *   src/combat/schools/buffs.ts   lastExpiredKind() — speist buff.expired
   *   sim/src/rules_event.rs    BuffExpired als REGEL-Ereignis
   * ```
   *
   * Ins Protokoll schreibt es niemand — `LogKind::BuffExpire` kommt in
   * `sim/src/` nicht vor, `EVENT_BUFF_EXPIRE` außerhalb dieser Datei nicht.
   * Dieselbe Klasse wie #568: vergeben, dokumentiert, gezählt — und nicht da.
   */
  buffExpire: { game: true, sim: true, bekannteLuecke: '#676 — keine Schreibstelle, beide Seiten' },
  /** Seit #679 schreibt das Spiel sie ebenfalls (Energieleiste, #568). */
  bossEnergyTriggered: { game: true, sim: true },
  // **Seit Issue #151 schreibt das Spiel diese fünf ebenfalls.** Bis dahin
  // führte sie nur der Simulator — und die Fehleranalyse läuft im Browser,
  // nicht im Simulator. Ein Protokoll, in dem die Größe überall fehlt, kann
  // eine Gelegenheit nicht belegen (GAME-DESIGN 15.2, Issue #81).
  bossTimelineFired: { game: true, sim: true },
  playerDamage: { game: true, sim: true },
  playerDeath: { game: false, sim: true },
  interruptLanded: { game: true, sim: true },
  shieldAbsorb: { game: true, sim: true },
  healed: { game: true, sim: true },
  convertedToShield: { game: false, sim: true },
  controlSpent: { game: false, sim: true },
};

/**
 * Welche Arten **diese** Seite schreibt. Der Rest gehört dem Simulator.
 *
 * **Abgeleitet aus [`EVENT_KIND_INTENT`]**, nicht von Hand geführt (#676).
 * Die frühere Liste war eine zweite Wahrheit neben der Tabelle im Kopf, und
 * genau dort ging Art 8 verloren: Die Schreibstelle kam mit #679, die Liste
 * blieb stehen, und `logKindsPayload()` verglich die Art deshalb nie.
 */
export const EVENT_KINDS_WRITTEN_BY_GAME: readonly number[] = Object.entries(EVENT_KIND_INTENT)
  .filter(([, absicht]) => absicht.game)
  .map(([name]) => EVENT_KINDS[name as keyof typeof EVENT_KINDS])
  .sort((a, b) => a - b);

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
