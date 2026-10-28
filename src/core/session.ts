/**
 * Der Kampf-Zustand: Eingabe, globale Wartezeit, Zauber.
 *
 * Hier treffen die Tastendrücke aus `src/input/` auf die Regeln aus `combat.ts`.
 * Zwei Regeln des Designs werden an dieser Stelle durchgesetzt:
 *
 * - Ein Druck, der nicht gewirkt werden kann, wird **verworfen**, nicht gemerkt
 *   (GAME-DESIGN 8.2 und 21.1). Der Spieler lernt sofort.
 * - Die Rückmeldung darauf ist rotes Aufblinken und ein dumpfer Ton.
 *
 * Der Ablauf je Takt folgt `tools/step1-check.py`: erst die laufenden Wirkungen,
 * dann die Prüfung auf das Kampfende, dann die Tastendrücke dieses Takts.
 */
import {
  Combat,
  FIGHT_DIED,
  FIGHT_OVERPRESSURE,
  FIGHT_RUNNING,
  FIGHT_WON,
  isCastAccepted,
  alsSpellIndex,
  NO_SPELL,
  type SpellIndex,
} from './combat.js';
import { buildSummary, type RunSummary } from './summary.js';
import {
  COUNTDOWN_MS,
  LOW_HEALTH_SHARE,
  OUTCOME_DEATH,
  OUTCOME_NAMES,
  OUTCOME_NONE,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  OVERPRESSURE_WARNING_MS,
  PHASE_COUNTDOWN,
  PHASE_ENDED,
  PHASE_FIGHTING,
  PHASE_PREPARE,
} from './phases.js';
import {
  EV_BOSS_CAST_START,
  EV_BOSS_CAST_SUCCESS,
  EV_BOSS_COUNTDOWN_BY_SECOND,
  EV_BOSS_DEFEATED,
  EV_BOSS_WARNING_BY_KIND,
  EV_BOSS_LOW_HEALTH,
  EV_BOSS_OVERPRESSURE,
  EV_BOSS_OVERPRESSURE_WARNING,
  EV_COUNTDOWN_TICK,
  EV_FIGHT_END,
  EV_FIGHT_START,
  EV_FIGHT_WIPE,
  EV_GCD_END,
  EV_INPUT_REJECTED,
  EV_MENU_ENTER,
  EV_PLAYER_DEATH,
  EV_RESULT_BEST_TIME,
  EV_RESULT_DEFEAT,
  EV_RESULT_VICTORY,
  EV_SPELL_CAST,
  EV_SPELL_READY,
  NO_KEY,
  type FeedbackSink,
} from './events.js';
import {
  BossAnnouncer,
  TIER_LETHAL,
  countdownSecond,
  highlighted,
  warnsAudibly,
  type Announcement,
} from '../combat/boss/announcements.js';
import { LatencyTrace } from './latency.js';
import { combatChecksum } from './checksum.js';
import { CombatLog } from './log.js';
import { RingBufferF64 } from './ring.js';
import type { Simulation } from './loop.js';
import { TICK_MS } from './loop.js';
import { SLOT_COUNT } from '../input/keys.js';
import type { PressQueue } from '../input/queue.js';

/**
 * Der leere Platz — **weitergereicht, nicht hier gebildet** (Issue #444).
 *
 * Er stand bis dahin in dieser Datei. `combat.ts` braucht ihn aber an fünf
 * Stellen, und ein Import zurück wäre ein Zirkelbezug gewesen: `session.ts`
 * holt bereits aus `combat.ts`.
 *
 * **Der Re-Export steht hier, damit jede bestehende Importstelle gilt.** Ihn
 * ersatzlos zu verschieben hiesse, fünfzehn Aufrufer anzufassen für eine
 * Änderung, die sie nichts angeht.
 */
export { NO_SPELL };

/** Nie gedrückt. */
export const NEVER = -1;

export type { FeedbackSink };

/**
 * WELCHE REGELN HIER LIEGEN, UND WAS DIE PARITÄT DAVON SIEHT (#528 AC5).
 *
 * Der Rücksetzer lag bis #528 hier — im Spiel eine Schicht über seiner eigenen
 * Wirkung, im Simulator im Kampf. Die Frage von AC5 war, ob es weitere gibt.
 *
 * **Gemessen, nicht durchgesehen.** Für jeden Kandidaten wurde geprüft, ob
 * eine Abweichung auffällt — indem sie eingebaut und die Suite gefahren wurde.
 *
 * | Regel | Ort | Sieht die Parität sie? |
 * | --- | --- | --- |
 * | Rücksetzer scharf stellen | `apply` ruft `combat.armResetIfNeeded` | **JA** |
 * | Auswahl statt Wirkung | `apply`, `chose ? chooseForReset : cast` | **JA** |
 * | Kampfende fortschreiben | `tick` ruft `combat.checkDefeat` | **NEIN — bewusst** |
 * | Phase (Vorbereitung/Kampf) | `apply`, `phaseValue === PHASE_FIGHTING` | entfällt |
 * | Belegung der Leiste | `bindings` | **JA** |
 *
 * # Die zwei, die die Parität sieht — und wie das gemessen wurde
 *
 * `armResetIfNeeded` wird HIER gerufen, nicht in `Combat.cast`. Eine
 * Aufzeichnung, die `Combat` direkt fährt, muss den Aufruf nachbilden. Der
 * Verdacht war, dass ein Vergessen unbemerkt bliebe — er ist **widerlegt**:
 * Wird der Aufruf aus `alembicPayload` entfernt und neu exportiert, fallen
 * vier von fünf Fällen in `sim/tests/parity_alembic.rs`, darunter die
 * Prüfsumme je Takt.
 *
 * Seit es Aufzeichnungen mit Rücksetzer gibt (#521, #528), ist diese Regel
 * also abgedeckt. Vorher war sie es nicht — nicht weil sie hier liegt,
 * sondern weil keine Aufzeichnung sie fuhr.
 *
 * # Der eine blinde Fleck ist bekannt und gewollt
 *
 * `checkDefeat` schreibt den Kampfzustand fort. **Keine Aufzeichnung ruft
 * es** — `sim/src/fight.rs` hält `FIGHT_RUNNING` deshalb bewusst konstant,
 * und der Kommentar dort nennt den Grund: Die Zustands-Prüfsumme rührt das
 * Feld ein, ein hier gesetzter Sieg-Zustand ergäbe ab dem Sieg-Takt eine
 * abweichende Prüfsumme, ohne dass eine Regel anders gerechnet wäre.
 *
 * Das ist der Unterschied zum Rücksetzer: Dort war die Trennung gewachsen und
 * unbemerkt, hier ist sie entschieden und aufgeschrieben.
 *
 * # Was NICHT hierher gehört, und warum die Liste kurz ist
 *
 * Die übrigen Felder dieser Klasse sind Auswertung (`accepted`, `rejected`,
 * `idleTicks`, `resetsApplied`) oder Anzeige. Sie ändern den Kampfverlauf
 * nicht und können deshalb auch nicht von ihm abweichen.
 *
 * Vier davon wurden ausdrücklich als Kandidaten genannt und einzeln
 * nachgesehen:
 *
 * ```text
 * idleTicksWithReady      3 Stellen: Getter, Zählung, Reset      ZÄHLER
 * idleTicksNothingReady   3 Stellen: dieselben                   ZÄHLER
 * outcomeValue            5 Stellen: Getter, Anzeige, finish()   ZÄHLER
 * phaseValue             18 Stellen, MEHRERE steuernd            REGEL
 * ```
 *
 * Die drei Zähler werden nirgends in einer Bedingung gelesen, die den Ablauf
 * steuert.
 *
 * # `phaseValue`: meine erste Zahl war falsch, und die Mutation hat es gezeigt
 *
 * Ich hatte „eine von 18 Stellen mit Wirkung" gezählt — das Ergebnis eines
 * Filters auf dieselbe Zeile, nicht einer Messung. Tatsächlich steuern
 * mindestens vier: der Takt-Ablauf (`fighting`), die Flanken-Erkennung, die
 * Leerlauf-Zählung und die Annahme eines Drucks in `apply`.
 *
 * Die Mutation an der `apply`-Stelle (Bedingung entfernt) **meldet**:
 *
 * ```text
 * tests/unit/fight-flow.test.ts   3 Fälle fallen
 * ```
 *
 * # Und sie zeigt zugleich, warum die Zeile oben „entfällt" sagt
 *
 * Gefangen hat es ein **Unit-Test**, kein Paritäts-Fall. Das ist kein Mangel:
 * Der Simulator kennt keine Phasen — er beginnt im Kampf. Eine Regel, die es
 * auf der anderen Seite gar nicht gibt, kann die Parität nicht prüfen, und
 * sie muss es auch nicht.
 *
 * Der Unterschied zum Rücksetzer: Der hatte auf beiden Seiten eine
 * Entsprechung und lag nur verschieden. `phaseValue` hat keine.
 */
/**
 * Welcher Ausgang gehört zu welchem Kampfzustand (Issue #581).
 *
 * **Eine Abbildung statt einer Wenn-dann-Kette**, weil die Kette bis zum
 * 2026-08-07 genau zwei Fälle kannte und der dritte still im `else` landete:
 *
 * ```ts
 * this.combat.fightState === FIGHT_WON ? OUTCOME_VICTORY : OUTCOME_OVERPRESSURE
 * ```
 *
 * Ein neuer Zustand hätte dort keinen Fehler erzeugt, sondern eine falsche
 * Auskunft — der Tod wäre als Überdruck ausgewiesen worden, und niemand hätte
 * es an einem Absturz gemerkt. `OUTCOME_DEATH` und die Überschrift
 * `VESSEL LOST` gab es zu diesem Zeitpunkt bereits; erreichbar waren sie nie.
 *
 * Der unbekannte Zustand fällt jetzt auf: `OUTCOME_NONE` ist kein Ende, und
 * ein Kampf, der mit „kein Ende" endet, zeigt sich sofort statt sich als
 * plausible Niederlage zu tarnen.
 */
function ausgangFuer(fightState: number): number {
  if (fightState === FIGHT_WON) return OUTCOME_VICTORY;
  if (fightState === FIGHT_DIED) return OUTCOME_DEATH;
  if (fightState === FIGHT_OVERPRESSURE) return OUTCOME_OVERPRESSURE;
  return OUTCOME_NONE;
}

export class Session implements Simulation {
  readonly latency = new LatencyTrace();

  /** Kampfzeit des letzten angenommenen Drucks je Belegung, sonst `NEVER`. */
  readonly acceptedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);
  /** Kampfzeit des letzten verworfenen Drucks je Belegung, sonst `NEVER`. */
  readonly rejectedAtMs = new Float64Array(SLOT_COUNT).fill(NEVER);

  /** Belegung → Zauber. `NO_SPELL`, solange der Platz leer ist. */
  private readonly bindings = new Int32Array(SLOT_COUNT).fill(NO_SPELL);

  /** Das Kampfprotokoll (GAME-DESIGN 18.2). */
  readonly log = new CombatLog();

  /**
   * Wird **einmal** beim Kampfende gerufen (Issue #179).
   *
   * Wie `combat.onHit` ein reiner Beobachter: Er liest die Kurzfassung und den
   * Ausgang, greift nicht ein, und ein Spiel ohne Zuhörer rechnet identisch
   * (GAME-DESIGN 18.5). Daran haengt die Fortschrittsmarke -- ohne diesen Ruf
   * bleiben Marke und Verlauf leer, egal wie oft gespielt wird.
   */
  onFinish: ((summary: RunSummary, outcome: number) => void) | null = null;

  /**
   * Eingabe-Zeitmessung (GAME-DESIGN 19.1): Millisekunden nach Ablauf der
   * globalen Wartezeit, je angenommenem Druck.
   *
   * **Im fertigen Spiel bewusst nicht vorgesehen** (GAME-DESIGN 21.2: niemand
   * wird für seine Reaktionszeit getadelt). Sie beantwortet nur die
   * Balance-Frage, wie viel ein Mensch gegenüber der perfekten Rechnung verliert.
   */
  private inputTimingBuffer: RingBufferF64 | null = null;

  /** Prüfsumme je Takt (GAME-DESIGN 19.1). Abschaltbar, weil sie Zeit kostet. */
  private checksums: RingBufferF64 | null = null;
  private trackChecksums = false;

  /**
   * Die Puffer entstehen erst beim ersten Zugriff.
   *
   * Im Auslieferungsstand greift niemand zu — dort werden sie nie angelegt, und
   * die 48 KB des Prüfsummen-Puffers kosten nichts (GAME-DESIGN 19.3).
   */
  get inputTiming(): RingBufferF64 {
    if (this.inputTimingBuffer === null) this.inputTimingBuffer = new RingBufferF64(512);
    return this.inputTimingBuffer;
  }

  /**
   * Leerlauf je Zauber in ms — wie lange er ungenutzt bereit lag.
   *
   * Gezählt wird nur in Takten, in denen **gar nichts** gewirkt wurde. Ein
   * Zauber, der bereit war, während ein anderer gewirkt wurde, war eine
   * Entscheidung, kein Leerlauf.
   */
  readonly idlePerSpellMs: Float64Array;

  /** Liegt der Zauber auf mindestens einem Platz? */
  private readonly boundSpells: Uint8Array;

  /**
   * Kantenerkennung für die Rückmeldungen, die kein Tastendruck auslöst.
   *
   * „Zauber wieder bereit" und „Wartezeit abgelaufen" sind **Übergänge**, keine
   * Zustände. Ohne den Vergleich mit dem vorigen Takt klänge der Hinweis
   * hundertmal je Sekunde, statt einmal.
   */
  private readonly wasReady: Uint8Array;
  private wasGcdReady = true;
  private sawLowHealth = false;
  private warnedOverpressure = false;
  /** Zuletzt geschlagene Countdown-Sekunde, sonst -1. */
  private countdownStep = -1;
  /**
   * Der laufende Boss-Zauber im vorigen Takt und wie viele bis dahin begonnen
   * hatten bzw. durchgekommen waren (GAME-DESIGN 11.5, Kanal 3; Issue #797).
   *
   * **Gezählt, nicht nur verglichen:** Ein Zauber, der in demselben Takt beginnt
   * und unterbrochen wird, steht beim Vergleich nie als laufend da — die Summe
   * aus unterbrochenen, durchgekommenen und laufenden Zaubern sieht ihn trotzdem.
   */
  private lastBossCasting = -1;
  private bossCastsBegun = 0;
  private bossCastsLanded = 0;
  /**
   * Die Ankündigungen, wie der TAKT sie sieht (GAME-DESIGN 11.5, Kanal 3).
   *
   * Eine eigene Instanz neben der des Bildes: Der Klang gehört in den Logik-Takt
   * (zwei Durchläufe klingen gleich), das Bild rechnet zwischen zwei Takten.
   * Beide fragen dieselbe Klasse.
   */
  private readonly announcer = new BossAnnouncer();
  /** Je Fähigkeit: für welche Wiederholung der Warnklang kam, sonst -1. */
  private warnedOccurrence: Int32Array = new Int32Array(0);
  /** Je Fähigkeit: zuletzt gesprochene Countdown-Sekunde, sonst 0. */
  private warnedSecond: Int8Array = new Int8Array(0);
  private warnedFor: readonly Announcement[] | null = null;

  /**
   * Der Auswahlmodus des Cooldown-Rücksetzers (GAME-DESIGN 4.1, Issue E3-3c).
   *
   * Kennnummer des Rücksetzers, der scharf steht — sonst `NO_SPELL`. Der
   * nächste Tastendruck **wählt** dann, statt zu wirken.
   *
   * **Warum ein Modus und kein passiver Effekt auf den nächsten Zauber:**
   * Entscheidung des Auftraggebers vom 2026-07-29. Der Spieler sagt
   * ausdrücklich, welchen Zauber er meint, statt dass der nächste gewirkte
   * ihn zufällig abbekommt. Drei Aktionen — Rücksetzer, Auswahl, Nachdruck —
   * müssen sich gegen drei gewöhnliche Wirkungen rechnen.
   */
  /** Wie oft eine Auswahl eine Abklingzeit zurückgesetzt hat. */
  private resetsApplied = 0;
  /** Wie oft eine Auswahl verpufft ist — der Zauber lag nicht auf Abklingzeit. */
  private resetsWasted = 0;

  /**
   * Beste Kampfzeit dieser Sitzung in ms, oder -1.
   *
   * **Überlebt `reset()` bewusst** — sonst wäre jeder Lauf eine neue Bestzeit.
   * Über das Ende der Seite hinaus hält sie nicht; der Spielstand im Browser
   * gehört zu einem späteren Epic.
   */
  private bestFightMs = -1;

  private accepted = 0;
  private rejected = 0;
  private lastSlot = -1;
  private timeMs = 0;
  private idleTicks = 0;
  /**
   * Der Leerlauf, in zwei Hälften geteilt — die Grundlage von AC7 in Issue #81.
   *
   * `idleTicks` allein ist eine Summe: Aus ihr lässt sich nicht mehr ablesen,
   * ob in einem Takt ein Zauber bereit lag oder nicht. Genau das ist aber der
   * Unterschied zwischen „verpasste Wirkung" und „Leerlauf" — zwei
   * Verlustquellen, die sich nicht überschneiden dürfen.
   *
   * Die Trennung geschieht deshalb **beim Zählen**, in einem einzigen
   * Wenn-Sonst über dieselbe Bedingung. Nachträglich ist sie nicht
   * herstellbar, und zwei getrennt geführte Zähler könnten auseinanderlaufen.
   */
  private idleTicksWithReady = 0;
  private idleTicksNothingReady = 0;
  private phaseValue = PHASE_PREPARE;
  private outcomeValue = OUTCOME_NONE;
  private countdownEndsAtMs = 0;
  private endedAt = -1;

  constructor(
    private readonly queue: PressQueue,
    readonly combat: Combat,
    private readonly feedback: FeedbackSink | null = null,
  ) {
    this.idlePerSpellMs = new Float64Array(combat.spells.length);
    this.boundSpells = new Uint8Array(combat.spells.length);
    this.wasReady = new Uint8Array(combat.spells.length).fill(1);
    combat.log = this.log;
  }

  /**
   * Ein Kampf-Ereignis melden (GAME-DESIGN 17.1).
   *
   * Der Kampf sagt nur, **was** geschehen ist; welcher Klang daran hängt, steht
   * im Verzeichnis. Hört niemand zu, kostet die Meldung einen Vergleich — und
   * das Spiel rechnet identisch (GAME-DESIGN 18.5).
   */
  private emit(event: number, key: number, timeMs: number): void {
    if (this.feedback !== null) this.feedback.onEvent(event, key, timeMs);
  }

  /** Beste Kampfzeit dieser Sitzung in ms, oder -1. */
  get bestTimeMs(): number {
    return this.bestFightMs;
  }

  /** Steht der Auswahlmodus scharf? Für die Anzeige und die Prüfung. */
  get isResetArmed(): boolean {
    return this.combat.resetArmed;
  }

  /** Welcher Rücksetzer scharf steht, sonst `NO_SPELL`. */
  get resetArmedBy(): number {
    const armed = this.combat.resetArmedFor;
    return armed >= 0 ? armed : NO_SPELL;
  }

  /** Wie oft eine Auswahl eine Abklingzeit zurückgesetzt hat. */
  get resetsAppliedCount(): number {
    return this.resetsApplied;
  }

  /** Wie oft eine Auswahl verpufft ist. */
  get resetsWastedCount(): number {
    return this.resetsWasted;
  }

  /** Abkürzung: Die globale Wartezeit gehört dem Kampf. */
  get gcd(): Combat['gcd'] {
    return this.combat.gcd;
  }

  get acceptedCount(): number {
    return this.accepted;
  }

  get rejectedCount(): number {
    return this.rejected;
  }

  /** Zuletzt gedrückte Belegung, oder -1. */
  get lastPressedSlot(): number {
    return this.lastSlot;
  }

  /** Kampfzeit, bis zu der gerechnet wurde. */
  get nowMs(): number {
    return this.timeMs;
  }

  /**
   * Takte, in denen gewirkt werden konnte, aber nichts gewirkt wurde.
   *
   * Grundlage der Kennzahl „verlorene Zeit" aus GAME-DESIGN 15.1; ausgewertet
   * wird sie in E1-7.
   */
  get idleMs(): number {
    return this.idleTicks * TICK_MS;
  }

  /** Leerlauf, in dem ein Zauber bereit lag — verpasste Wirkung (Issue #81). */
  get idleWithReadyMs(): number {
    return this.idleTicksWithReady * TICK_MS;
  }

  /** Leerlauf, in dem **kein** Zauber bereit lag — echter Leerlauf. */
  get idleNothingReadyMs(): number {
    return this.idleTicksNothingReady * TICK_MS;
  }

  bind(slot: number, spellIndex: SpellIndex): void {
    if (slot < 0 || slot >= SLOT_COUNT) return;
    this.bindings[slot] = spellIndex;
    this.rebuildBoundSpells();
  }

  /**
   * `boundSpells` aus den Plätzen neu aufbauen (Issue #306).
   *
   * # Warum abgeleitet und nicht fortgeschrieben
   *
   * Vorher setzte `bind` nur `boundSpells[neu] = 1` und **löschte nie**. Wer
   * einen Zauber von einem Platz nahm, hatte ihn aus der Leiste entfernt und
   * trotzdem weiter in der Leerlauf-Rechnung — die Aufschlüsselung führte ihn
   * mit voller Zeit, obwohl er nirgends mehr lag.
   *
   * Die naheliegende Reparatur wäre, an derselben Stelle `boundSpells[alt] = 0`
   * zu setzen. Sie ist falsch, sobald derselbe Zauber auf zwei Plätzen liegt:
   * Dann löscht das Überschreiben des einen Platzes die Markierung, die der
   * andere noch braucht. Seit GAME-DESIGN 715 ist Doppelbelegung zwar
   * verboten — aber eine Reparatur, die nur unter einer Regel richtig ist,
   * bricht still, wenn die Regel woanders verletzt wird.
   *
   * **Ableiten kann diesen Fehler nicht machen.** Die Plätze sind die einzige
   * Wahrheit; `boundSpells` ist ihre Sicht für die Rechnung.
   *
   * Das ist kein Takt-Vorgang (GAME-DESIGN 18.2 verbietet Erzeugung nur im
   * laufenden Bild): `bind` läuft beim Belegen der Leiste, nicht je Takt, und
   * die Schleife benutzt zwei bestehende Felder ohne zu allozieren.
   */
  private rebuildBoundSpells(): void {
    this.boundSpells.fill(0);
    for (let i = 0; i < SLOT_COUNT; i++) {
      const spellIndex = this.bindings[i] ?? NO_SPELL;
      if (spellIndex >= 0 && spellIndex < this.boundSpells.length) {
        this.boundSpells[spellIndex] = 1;
      }
    }
  }

  spellOnSlot(slot: number): SpellIndex {
    // `bindings` ist ein Int32Array — es kann die Marke nicht tragen. Hier ist
    // die eine Stelle, an der ein Platz zu einem Zauber wird (Issue #372).
    return alsSpellIndex(this.bindings[slot] ?? NO_SPELL);
  }

  /** Plätze mit einem Zauber darauf — die einzigen, die die Leiste zeigt. */
  boundSlotCount(): number {
    let count = 0;
    for (let i = 0; i < SLOT_COUNT; i++) {
      if ((this.bindings[i] ?? NO_SPELL) !== NO_SPELL) count++;
    }
    return count;
  }

  tick(timeMs: number): void {
    this.timeMs = timeMs;

    // 0. Countdown. Der Übergang geschieht VOR allem anderen in diesem Takt:
    //    Uhr und Eingabe beginnen damit im selben Takt, nicht einen später.
    if (this.phaseValue === PHASE_COUNTDOWN && timeMs >= this.countdownEndsAtMs) {
      this.startFight(timeMs);
    } else if (this.phaseValue === PHASE_COUNTDOWN) {
      // Je Sekunde ein Schlag (Klang-Verzeichnis: „Countdown vor dem Kampf, je
      // Sekunde ein Schlag"). Drei Schläge, dann der Kampfbeginn.
      const step = Math.ceil((this.countdownEndsAtMs - timeMs) / 1000);
      if (step !== this.countdownStep) {
        this.countdownStep = step;
        this.emit(EV_COUNTDOWN_TICK, NO_KEY, timeMs);
      }
    }

    const fighting = this.phaseValue === PHASE_FIGHTING;

    // 1. laufende Wirkungen — nur im Kampf. In der Vorbereitung laufen allein die
    //    Abklingzeiten mit (GAME-DESIGN 10), es gibt keine Uhr und keinen Verfall.
    if (fighting) this.combat.stepPassive(timeMs);

    // 2. Kampfende
    if (fighting && this.combat.checkDefeat(timeMs)) {
      this.finish(ausgangFuer(this.combat.fightState), timeMs);
    }

    // 3. Tastendrücke dieses Takts. Alles abarbeiten, was diesem Takt oder einem
    //    früheren zugewiesen war — dadurch kann der Puffer nie zur Warteschlange
    //    werden.
    const tickIndex = timeMs / TICK_MS;
    let casted = false;
    while (this.queue.size > 0 && this.queue.frontTick() <= tickIndex) {
      const slot = this.queue.frontSlot();
      const stampMs = this.queue.frontStamp();
      const frameIndex = this.queue.frontFrame();
      this.queue.shift();
      if (this.apply(slot, timeMs, stampMs, frameIndex)) casted = true;
    }

    if (this.trackChecksums && this.phaseValue === PHASE_FIGHTING) {
      this.checksums?.push(combatChecksum(this.combat, timeMs));
    }

    // 4. Übergänge, die kein Tastendruck auslöst — erst nach den Drücken, damit
    //    ein Zauber, der in diesem Takt gewirkt wurde, nicht zugleich als
    //    „wieder bereit" gilt.
    if (this.phaseValue === PHASE_FIGHTING) this.detectEdges(timeMs);

    if (this.phaseValue === PHASE_FIGHTING && !casted && this.combat.gcd.isReady(timeMs)) {
      this.idleTicks++;
      // Welcher Zauber lag in diesem Takt ungenutzt bereit? Nur belegte zählen —
      // ein Zauber, der nicht auf der Leiste liegt, kann nicht versäumt werden.
      let anyReady = false;
      for (let i = 0; i < this.boundSpells.length; i++) {
        if (this.boundSpells[i] === 1 && this.combat.isOffCooldown(alsSpellIndex(i), timeMs)) {
          this.idlePerSpellMs[i] = (this.idlePerSpellMs[i] ?? 0) + TICK_MS;
          anyReady = true;
        }
      }
      // Ein Wenn-Sonst, kein zweites Wenn: So kann derselbe Takt nicht in beide
      // Hälften geraten, und ihre Summe ist immer `idleTicks`.
      //
      // ---------------------------------------------------------------------
      // WANN KANN `idleTicksNothingReady` ÜBERHAUPT STEIGEN? (Issue #300)
      //
      // Nur, wenn KEIN belegter Zauber bereit ist — und dieser Takt wird
      // überhaupt nur gezählt, wenn der GCD bereit ist. Daraus folgt die
      // Bedingung, und zwar als Rechnung, nicht als Beobachtung:
      //
      //   Ein Zauber mit `cooldownMs <= GCD_MS` startet seine Abklingzeit
      //   gleichzeitig mit dem GCD und ist höchstens so lang. Sobald der GCD
      //   bereit ist, ist er es auch. Ein solcher Zauber auf der Leiste macht
      //   den `else`-Zweig UNERREICHBAR.
      //
      //   GCD_MS = 1500 (gcd.ts), kindle cooldownMs = 1500 — also genau der
      //   Grenzfall, und er sperrt. Solange KINDLE liegt, bleibt die Quelle
      //   „nichts war bereit" strukturell leer.
      //
      //   Mit einem langen Cooldown ist sie erreichbar. Das rechnet
      //   `tests/unit/idle-source-reachable.test.ts` nach — mit REFLUX
      //   (cooldownMs 45000) allein und dem Gegenfall mit KINDLE dazu.
      //
      //   **Wie viele Takte es sind, steht hier bewusst nicht.** Die Zahl
      //   hängt von Kampfdauer und Druckzeitpunkt ab: Zwei richtige Messungen
      //   ergaben 1851 und 2840 Takte. Eine laufabhängige Zahl an einer
      //   Zählstelle ist eine Zahl ohne Bezug — wer nachmisst, bekommt etwas
      //   anderes und weiß nicht, wer sich geirrt hat.
      //
      //   Hier stand vorher eine Zahl („1851 Takte nicht bereit"), die aus
      //   einer fremden Messung abgeschrieben war und die keine Datei
      //   nachrechnete. In einem Vermerk, der gegen unbelegte Behauptungen
      //   geschrieben ist, war das die Behauptung selbst (#300).
      //
      // **Das ist kein Rechenfehler.** GAME-DESIGN 15.3 definiert Leerlauf als
      // „Fenster, in denen kein Zauber bereit war". Ist immer einer bereit,
      // gibt es keinen Leerlauf, und eine leere Quelle ist die richtige
      // Antwort. Der Vermerk steht hier, damit der Nächste sie nicht für
      // kaputt hält.
      //
      // DIE ZWEITE SPERRE WIEGT SCHWERER UND IST WENIGER SICHTBAR:
      // `isOffCooldown` fragt **nur** die Abklingzeit, nie die Ressource.
      //
      //   STALL ohne Kontroll-Punkte  ->  gilt als bereit
      //   PYRE ohne Hitze             ->  gilt als bereit
      //
      // Ein Zauber, dessen Kosten der Spieler gerade nicht aufbringen kann,
      // setzt hier trotzdem `anyReady`. Die Quelle bleibt damit auch dann
      // leer, wenn später Zauber mit Kosten dazukommen und der Spieler sie
      // nicht bezahlen kann — also genau in der Lage, in der er dasteht und
      // nichts tun KANN.
      //
      // Ob ein unbezahlbarer Zauber als bereit gelten soll, ist eine
      // Design-Frage und wird hier NICHT entschieden.
      // ---------------------------------------------------------------------
      if (anyReady) this.idleTicksWithReady++;
      else this.idleTicksNothingReady++;
    }
  }

  /**
   * Die vier Übergänge, die aus dem Zustand folgen statt aus einer Eingabe:
   * Wartezeit abgelaufen, Zauber wieder bereit, Boss im Endspurt, Überdruck
   * kündigt sich an.
   *
   * Läuft in jedem Kampf-Takt und darf deshalb nichts erzeugen: nur Vergleiche
   * auf vorab angelegten Feldern (GAME-DESIGN 18.2).
   */
  private detectEdges(timeMs: number): void {
    const gcdReady = this.combat.gcd.isReady(timeMs);
    if (gcdReady && !this.wasGcdReady) this.emit(EV_GCD_END, NO_KEY, timeMs);
    this.wasGcdReady = gcdReady;

    for (let i = 0; i < this.wasReady.length; i++) {
      // Nur belegte Zauber: Was nicht auf der Leiste liegt, kann nicht bereit
      // werden — der Hinweis ginge ins Leere.
      if (this.boundSpells[i] !== 1) continue;
      const ready = this.combat.isOffCooldown(alsSpellIndex(i), timeMs) ? 1 : 0;
      if (ready === 1 && this.wasReady[i] === 0) this.emit(EV_SPELL_READY, i, timeMs);
      this.wasReady[i] = ready;
    }

    if (
      !this.sawLowHealth &&
      this.combat.damageDealt >= this.combat.boss.hp * (1 - LOW_HEALTH_SHARE)
    ) {
      this.sawLowHealth = true;
      this.emit(EV_BOSS_LOW_HEALTH, NO_KEY, timeMs);
    }

    if (
      !this.warnedOverpressure &&
      this.overpressureRemainingMs(timeMs) <= OVERPRESSURE_WARNING_MS
    ) {
      this.warnedOverpressure = true;
      this.emit(EV_BOSS_OVERPRESSURE_WARNING, NO_KEY, timeMs);
    }

    this.detectBossCastEdges(timeMs);
    this.detectBossWarningEdges(timeMs);
  }

  /**
   * Der Warnklang je Art und der Countdown von Stufe 3 (GAME-DESIGN 11.5).
   *
   * | Stufe | Klang |
   * | --- | --- |
   * | 1 | nur „Schwächung auf dir" und „Gelegenheit"; Allgemeiner Schaden ist die „ruhigere Ankündigung" |
   * | 2 | einmal, wenn sie in ihre letzten Sekunden tritt |
   * | 3 | ebenso, dazu je Sekunde 5-4-3-2-1 |
   *
   * Welche Ankündigung klingt, steht in `warnsAudibly` — eine Regel, die das
   * Bild nicht braucht, aber an derselben Stelle wie die des Bildes.
   *
   * **„Das Nebensächliche kurz unterdrückt"**: Tritt eine Fähigkeit in ihr
   * Fenster, während eine höhere Stufe schon in ihrem ist, bleibt sie stumm —
   * sonst überdeckte der leisere Klang den, auf den es ankommt. Die Liste ist
   * nach Dringlichkeit sortiert, das Höhere kommt also zuerst an die Reihe.
   */
  private detectBossWarningEdges(timeMs: number): void {
    const ann = this.announcer;
    ann.update(this.combat, this.fightElapsedMs(timeMs));
    const abilities = ann.abilities;
    if (abilities !== this.warnedFor) {
      // Einmal je Boss, nicht je Takt.
      this.warnedFor = abilities;
      this.warnedOccurrence = new Int32Array(abilities.length).fill(-1);
      this.warnedSecond = new Int8Array(abilities.length);
    }
    let lauteste = 0;
    for (let rank = 0; rank < ann.count; rank++) {
      const a = ann.at(rank);
      const hervor = highlighted(a);
      const klingt = warnsAudibly(a);
      if (!hervor && !klingt) continue;
      const i = abilities.indexOf(a);
      if (this.warnedOccurrence[i] !== a.occurrence) {
        this.warnedOccurrence[i] = a.occurrence;
        this.warnedSecond[i] = 0;
        if (klingt && a.tier >= lauteste) {
          this.emit(EV_BOSS_WARNING_BY_KIND[a.kind] as number, NO_KEY, timeMs);
        }
      }
      if (a.tier > lauteste) lauteste = a.tier;
      if (a.tier < TIER_LETHAL) continue;
      const sekunde = countdownSecond(a.remainingMs);
      if (
        sekunde !== this.warnedSecond[i] &&
        sekunde >= 1 &&
        sekunde < EV_BOSS_COUNTDOWN_BY_SECOND.length
      ) {
        this.warnedSecond[i] = sekunde;
        this.emit(EV_BOSS_COUNTDOWN_BY_SECOND[sekunde] as number, NO_KEY, timeMs);
      }
    }
  }

  /**
   * Beginn und Durchkommen eines Boss-Zaubers hörbar machen (GAME-DESIGN 11.5,
   * Kanal 3; Issue #797).
   *
   * **Die zwei Klänge lagen fertig im Verzeichnis und klangen nie.**
   * `boss-cast-start.ogg` („Kündigt das Unterbrechen-Fenster an") und
   * `boss-cast-success.ogg` („das Fenster wurde verpasst") hängen an
   * `boss.cast.start` und `boss.cast.success` — gemessen vor dieser Zeile: kein
   * einziger Aufruf im Quelltext sendete eines der beiden Ereignisse.
   */
  private detectBossCastEdges(timeMs: number): void {
    const combat = this.combat;
    const laufend = combat.casting;
    const begonnen = combat.interruptsLanded + combat.interruptsMissed + (laufend >= 0 ? 1 : 0);
    // Zwei Wege, weil jeder einen Fall allein nicht sieht: die Summe den im selben
    // Takt unterbrochenen Zauber, der Vergleich den, der einen laufenden ablöst.
    if (begonnen > this.bossCastsBegun || (laufend >= 0 && laufend !== this.lastBossCasting)) {
      this.emit(EV_BOSS_CAST_START, NO_KEY, timeMs);
    }
    this.bossCastsBegun = begonnen;
    this.lastBossCasting = laufend;
    if (combat.interruptsMissed > this.bossCastsLanded) {
      this.emit(EV_BOSS_CAST_SUCCESS, NO_KEY, timeMs);
    }
    this.bossCastsLanded = combat.interruptsMissed;
  }

  // ------------------------------------------------------------ Kampfablauf

  get phase(): number {
    return this.phaseValue;
  }

  get outcome(): number {
    return this.outcomeValue;
  }

  /** Restzeit des Countdowns in ms, sonst 0. */
  countdownRemainingMs(timeMs: number): number {
    if (this.phaseValue !== PHASE_COUNTDOWN) return 0;
    const left = this.countdownEndsAtMs - timeMs;
    return left > 0 ? left : 0;
  }

  /** Prüfsumme je Takt mitschreiben. Kostet Rechenzeit (GAME-DESIGN 19.1). */
  setChecksumTracking(on: boolean): void {
    if (!__ROTA_DEVTOOLS__) return;
    this.trackChecksums = on;
    if (on) this.checksums ??= new RingBufferF64(6000);
    else this.checksums?.clear();
  }

  get isTrackingChecksums(): boolean {
    return this.trackChecksums;
  }

  /** Die zuletzt mitgeschriebenen Prüfsummen, älteste zuerst. */
  checksumTrail(limit = 600): number[] {
    const all = this.checksums?.toArray() ?? [];
    return all.slice(Math.max(0, all.length - limit));
  }

  /** Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf endete. */
  get endedAtMs(): number {
    return this.endedAt;
  }

  /**
   * Die Uhr, die der Spieler sieht.
   *
   * **Null in der Vorbereitung und im Countdown** (GAME-DESIGN 10: „Keine Uhr"),
   * und nach dem Ende stehengeblieben — die Auswertung zeigt die Zeit des
   * Kampfes, nicht die seit dem Laden der Seite. `Combat` kennt diesen
   * Unterschied nicht; er gehört zum Ablauf und damit hierher.
   */
  fightElapsedMs(timeMs: number): number {
    if (this.phaseValue === PHASE_PREPARE || this.phaseValue === PHASE_COUNTDOWN) return 0;
    return this.combat.fightElapsedMs(this.endedAt >= 0 ? this.endedAt : timeMs);
  }

  /** Restzeit bis zum Überdruck, aus Sicht des Spielers. */
  overpressureRemainingMs(timeMs: number): number {
    const left = this.combat.boss.overpressureMs - this.fightElapsedMs(timeMs);
    return left > 0 ? left : 0;
  }

  /**
   * „Ready" gedrückt: Der Countdown beginnt.
   *
   * Nur aus der Vorbereitung heraus — ein zweiter Druck während des Countdowns
   * darf ihn nicht verlängern.
   */
  beginCountdown(timeMs: number): boolean {
    if (this.phaseValue !== PHASE_PREPARE) return false;
    this.phaseValue = PHASE_COUNTDOWN;
    this.countdownEndsAtMs = timeMs + COUNTDOWN_MS;
    return true;
  }

  /**
   * Abbruch durch den Spieler (GAME-DESIGN 10: „Wipe Run").
   *
   * Gilt als Ende und führt zur **selben** Auswertung wie ein Sieg — nur mit
   * anderem Ausgang. Keine Strafe.
   */
  wipe(timeMs: number): boolean {
    if (this.phaseValue !== PHASE_FIGHTING && this.phaseValue !== PHASE_COUNTDOWN) return false;
    this.finish(OUTCOME_WIPE, timeMs);
    return true;
  }

  /**
   * Die Kurzfassung des Laufs (GAME-DESIGN 15.1).
   *
   * Wird **nach** dem Kampf gerufen, nicht im Bild — hier darf Speicher entstehen.
   */
  summary(timeMs: number): RunSummary {
    return buildSummary({
      combat: this.combat,
      boundSpells: this.boundSpells,
      log: this.log,
      outcome: OUTCOME_NAMES[this.outcomeValue] ?? '?',
      victory: this.outcomeValue === OUTCOME_VICTORY,
      elapsedMs: this.fightElapsedMs(timeMs),
      remainingMs: this.overpressureRemainingMs(timeMs),
      casts: this.accepted,
      lostTimeMs: this.idleMs,
      lostTimeWithReadyMs: this.idleWithReadyMs,
      lostTimeNothingReadyMs: this.idleNothingReadyMs,
      idlePerSpellMs: this.idlePerSpellMs,
      // **Aus dem Kampf, nicht mehr hart auf 0** (Issue #318). Die alte
      // Begründung („eine Schule, keine schulübergreifenden Fenster") ist
      // überholt: Heute liegen alle fünf Schulen im Kampf, und CANKER öffnet
      // FESTER ein Fenster.
      //
      // **Unbestimmt bleibt unbestimmt.** Wer kein Synergie-Paar mitführt, hat
      // `synergyAvailable === 0` — und die Auswertung zeigt dafür weiterhin
      // „none available" statt einer Ausbeute von 0 %. Das ist kein
      // Sonderfall, den diese Stelle behandeln müsste, sondern die Folge
      // daraus, dass sie die gemessenen Zahlen durchreicht statt sie zu
      // erfinden.
      synergyUsed: this.combat.synergyUsed,
      synergyAvailable: this.combat.synergyAvailable,
    });
  }

  /**
   * Den Kampf in diesem Takt beginnen lassen.
   *
   * Derselbe Weg, den der Countdown an seinem Ende nimmt — Uhr und Eingabe gehen
   * dadurch zwangsläufig im selben Takt auf. Für die Entwicklungs-Werkzeuge
   * zusätzlich direkt aufrufbar (GAME-DESIGN 19.1: Zustand setzen).
   */
  startFight(timeMs: number): void {
    if (this.phaseValue === PHASE_ENDED) return;
    this.phaseValue = PHASE_FIGHTING;
    this.countdownEndsAtMs = timeMs;
    this.combat.beginFight(timeMs);

    // Ausgangslage der Kantenerkennung. Ohne sie schlüge im ersten Takt jeder
    // bereite Zauber an — der Kampf begänne mit einem Akkord.
    this.wasGcdReady = this.combat.gcd.isReady(timeMs);
    for (let i = 0; i < this.wasReady.length; i++) {
      this.wasReady[i] = this.combat.isOffCooldown(alsSpellIndex(i), timeMs) ? 1 : 0;
    }
    this.sawLowHealth = false;
    this.warnedOverpressure = false;
    this.lastBossCasting = this.combat.casting;
    this.bossCastsBegun =
      this.combat.interruptsLanded +
      this.combat.interruptsMissed +
      (this.combat.casting >= 0 ? 1 : 0);
    this.bossCastsLanded = this.combat.interruptsMissed;
    // Die Wiederholungen eines neuen Laufs tragen dieselben Nummern wie die des
    // vorigen — ohne diese Zeilen bliebe die erste Warnung jedes Neustarts stumm.
    this.warnedOccurrence.fill(-1);
    this.warnedSecond.fill(0);
    this.emit(EV_FIGHT_START, NO_KEY, timeMs);
  }

  private finish(outcome: number, timeMs: number): void {
    if (this.phaseValue === PHASE_ENDED) return;
    this.phaseValue = PHASE_ENDED;
    this.outcomeValue = outcome;
    this.endedAt = timeMs;

    if (outcome === OUTCOME_VICTORY) {
      this.emit(EV_BOSS_DEFEATED, NO_KEY, timeMs);
      this.emit(EV_RESULT_VICTORY, NO_KEY, timeMs);
      const elapsed = this.fightElapsedMs(timeMs);
      if (this.bestFightMs < 0 || elapsed < this.bestFightMs) {
        this.bestFightMs = elapsed;
        this.emit(EV_RESULT_BEST_TIME, NO_KEY, timeMs);
      }
    } else if (outcome === OUTCOME_OVERPRESSURE) {
      this.emit(EV_BOSS_OVERPRESSURE, NO_KEY, timeMs);
      this.emit(EV_RESULT_DEFEAT, NO_KEY, timeMs);
    } else if (outcome === OUTCOME_WIPE) {
      // Der Abbruch hat einen **eigenen** Klang und ist ausdrücklich keine
      // Niederlage (GAME-DESIGN 10: keine Strafe, 12.8: häufigster Übergang).
      this.emit(EV_FIGHT_WIPE, NO_KEY, timeMs);
    } else if (outcome === OUTCOME_DEATH) {
      this.emit(EV_PLAYER_DEATH, NO_KEY, timeMs);
      this.emit(EV_RESULT_DEFEAT, NO_KEY, timeMs);
    }

    // Zuletzt die Musik: erst der Ausgang, dann der Wechsel des Klangteppichs.
    // Die Auswertung ist ein Bild ohne Kampf und bekommt deshalb den Menü-Teppich.
    this.emit(EV_FIGHT_END, NO_KEY, timeMs);

    // Und ganz zuletzt der Beobachter: Er bekommt einen fertigen Zustand, nicht
    // einen halben. Vor den Meldungen gerufen saehe er einen Kampf, dessen
    // Ausgang noch nirgends angekommen ist.
    this.onFinish?.(this.summary(timeMs), outcome);
    this.emit(EV_MENU_ENTER, NO_KEY, timeMs);
  }

  /** @returns true, wenn tatsächlich gewirkt wurde. */
  private apply(slot: number, timeMs: number, stampMs: number, frameIndex: number): boolean {
    this.lastSlot = slot;
    const spellIndex = this.bindings[slot] ?? NO_SPELL;
    const gcdReadyAt = this.combat.gcd.readyAt;

    // In der Vorbereitung ist kein Ziel vorhanden: Alle Zauber auf den Boss sind
    // ausgegraut (GAME-DESIGN 10). In Schritt 1 sind das alle drei. Ein leerer
    // Platz ist eine Fehleingabe wie jede andere.
    let accepted = false;
    // Steht der Auswahlmodus scharf, WÄHLT dieser Druck, statt zu wirken.
    const chose = this.combat.resetArmed;
    if (
      this.phaseValue === PHASE_FIGHTING &&
      spellIndex !== NO_SPELL &&
      this.combat.fightState === FIGHT_RUNNING
    ) {
      accepted = chose
        ? this.chooseForReset(alsSpellIndex(spellIndex), timeMs)
        : // **Ein begonnener Zauber ist ein angenommener Druck** (Issue #304).
          // `CAST_OK` allein wäre hier falsch: Ein Zauber mit Wirkzeit meldet
          // `CAST_STARTED`, und der Spieler bekäme für einen richtigen Druck
          // ein rotes Blinken.
          isCastAccepted(this.combat.cast(alsSpellIndex(spellIndex), timeMs).result);
    }

    if (accepted) {
      // Wie viele Millisekunden nach Freiwerden der Wartezeit kam der Druck?
      // Nur für die Werkzeuge — im fertigen Spiel ausdrücklich nicht vorgesehen
      // (GAME-DESIGN 21.2: niemand wird für seine Reaktionszeit getadelt).
      if (__ROTA_DEVTOOLS__) this.inputTiming.push(timeMs - gcdReadyAt);
      this.acceptedAtMs[slot] = timeMs;
      // Eine Auswahl ist keine Wirkung: Sie zählt nicht als Zauber und meldet
      // kein `spell.cast`. Sonst stünde in der Auswertung eine Wirkung, die
      // nie stattgefunden hat.
      if (!chose) {
        this.accepted++;
        this.emit(EV_SPELL_CAST, spellIndex, timeMs);
        // Ein Rücksetzer stellt den Auswahlmodus scharf — erst NACH seiner
        // eigenen Wirkung, sonst wählte er sich selbst als nächsten Druck.
        this.combat.armResetIfNeeded(spellIndex, timeMs);
      }
    } else {
      this.rejectedAtMs[slot] = timeMs;
      this.rejected++;
      this.emit(EV_INPUT_REJECTED, NO_KEY, timeMs);
    }
    // Beide Ausgänge sind im nächsten Bild sichtbar: der eine als Aufleuchten,
    // der andere als rotes Blinken. Beide zählen daher für die Verzögerung.
    this.latency.push(stampMs, frameIndex);
    return accepted;
  }

  /**
   * Ein Druck im Auswahlmodus (Issue E3-3c).
   *
   * **Die Auswahl löst die volle Wartezeit aus.** Das steht nicht wörtlich im
   * Issue, folgt aber zwingend aus AC4 („kein Doppel-Wirken im selben Takt")
   * und aus seiner eigenen Rechnung: Drei Aktionen — Rücksetzer, Auswahl,
   * Nachdruck — müssen sich gegen drei gewöhnliche Wirkungen rechnen. Wäre die
   * Auswahl umsonst, fiele der Nachdruck in denselben Takt, und es wären zwei.
   *
   * @returns true, wenn die Auswahl angenommen wurde (kein rotes Blinken)
   */
  private chooseForReset(spellIndex: SpellIndex, timeMs: number): boolean {
    // **Die Regel selbst liegt seit #528 in `Combat`** — dort, wo ihre Wirkung
    // eintritt und wo die Parität sie sieht. Hier bleibt nur, was wirklich der
    // Sitzung gehört: die zwei Zähler für die Auswertung.
    //
    // Der Grund steht ausführlich an `Combat.chooseForReset`: Jede
    // Aufzeichnung fährt `Combat` direkt, eine Regel ausserhalb davon ist für
    // die Parität unsichtbar.
    const gesetzt = this.combat.chooseForReset(alsSpellIndex(spellIndex), timeMs);
    if (gesetzt) this.resetsApplied++;
    else this.resetsWasted++;
    return gesetzt;
  }

  reset(): void {
    this.combat.reset();
    this.acceptedAtMs.fill(NEVER);
    this.rejectedAtMs.fill(NEVER);
    this.latency.clear();
    this.queue.clear();
    this.accepted = 0;
    this.rejected = 0;
    this.lastSlot = -1;
    this.timeMs = 0;
    this.idleTicks = 0;
    this.idleTicksWithReady = 0;
    this.idleTicksNothingReady = 0;
    this.phaseValue = PHASE_PREPARE;
    this.outcomeValue = OUTCOME_NONE;
    this.countdownEndsAtMs = 0;
    this.endedAt = -1;
    this.log.clear();
    this.idlePerSpellMs.fill(0);
    this.inputTimingBuffer?.clear();
    this.checksums?.clear();

    this.wasReady.fill(1);
    this.wasGcdReady = true;
    this.sawLowHealth = false;
    this.warnedOverpressure = false;
    this.countdownStep = -1;
    this.lastBossCasting = -1;
    this.bossCastsBegun = 0;
    this.bossCastsLanded = 0;
    this.warnedOccurrence.fill(-1);
    this.warnedSecond.fill(0);
    this.resetsApplied = 0;
    this.resetsWasted = 0;
    // Erst das Kampfende, dann der Menü-Teppich — genau wie beim regulären
    // Ausgang. Wer aus einem **laufenden** Kampf zurückgeht, hat sonst zwei
    // Klangteppiche übereinander: Der Kampf-Teppich wird nie abgeschaltet.
    // Im Bild aufgefallen, nicht im Prüflauf.
    this.emit(EV_FIGHT_END, NO_KEY, 0);
    // Die Bestzeit bleibt: Sie gehört der Sitzung, nicht dem Lauf.
    this.emit(EV_MENU_ENTER, NO_KEY, 0);
  }
}
