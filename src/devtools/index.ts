import { alsSpellIndex } from '../core/combat.js';
/**
 * Entwicklungs-Werkzeuge (GAME-DESIGN 19.1).
 *
 * Nur über die Browser-Konsole ansprechbar — keine Oberfläche, keine Knöpfe,
 * nichts im Bild. Der Auftraggeber sieht beim Testen ausschließlich das Spiel.
 *
 * Der gesamte Inhalt dieser Datei steht hinter `__ROTA_DEVTOOLS__` und wird in
 * der Auslieferungs-Fassung vollständig herausgeschnitten (19.3). Geprüft im
 * CI-Job "build" per grep über `dist/`.
 *
 * Umfang zum jetzigen Stand (Issue E1-1): Leistungs-Messwerte und Zustandsabfrage.
 * Zeitsteuerung, Aufzeichnen, Prüfsumme und Kampfprotokoll folgen mit E1-8.
 */
import type { App } from '../core/app.js';
import { TICK_MS } from '../core/loop.js';
import { PerfCounters, summarize, type SeriesSummary } from '../core/metrics.js';
import { KEYS_PER_LAYER, SLOT_CODES, SLOT_COUNT } from '../input/keys.js';
import { NO_SPELL } from '../core/session.js';
import { HIT_DIRECT, HIT_DOT } from '../core/hits.js';
import { combatChecksum } from '../core/checksum.js';
import type { DamageFactors } from '../core/damage.js';
import type { TalentAllocation } from '../progression/talents.js';
import type { WornValues } from '../gear/worn.js';
import { EVENT_NAMES, type LogEntry } from '../core/log.js';
import { EVENT_NAMES as COMBAT_EVENT_NAMES, NO_KEY, eventIndex } from '../core/events.js';
import { buildSoundReport, type SoundReport } from './sound-report.js';
import { RULE_DAMAGE, RULE_EVENT_COUNT, RULE_EVENT_NAMES, RuleEvents } from '../combat/events.js';
import {
  recordingNames,
  recordings,
  replayRecording,
  type ReplayOptions,
  type ReplayResult,
} from './recording.js';
import type { RunSummary } from '../core/summary.js';
import { OUTCOME_NAMES, PHASE_COUNTDOWN, PHASE_FIGHTING, PHASE_NAMES } from '../core/phases.js';
import { slotRect, sweepHandAngle, type Rect } from '../render/spellbar.js';
import { STAT_ADDITIVE, STAT_MULTIPLICATIVE } from '../gear/format.js';
import { compareItems } from '../loot/compare.js';
import { allocationProblem } from '../progression/talents.js';
import { talentPointsAtLevel } from '../progression/unlocks.js';
import { AllocationProbe, type AllocationResult } from './allocations.js';
import {
  measureCooldown,
  runReference,
  type CooldownMeasurement,
  type ReferenceOptions,
  type ReferenceResult,
  type RotationName,
} from './reference-run.js';
import { erwarteterStand, bessereStuecke } from './expected-gear.js';
import type { OwnedItemLike } from '../gear/catalog.js';

/** Bilder, die vor der ersten Speicher-Messung verstreichen (Aufwärmen). */
const WARMUP_FRAMES = 240;
/** Länge einer Speicher-Messung in Bildern. */
const ALLOCATION_WINDOW_FRAMES = 600;
/** Abstand zwischen zwei Speicher-Messungen in Bildern. */
const ALLOCATION_INTERVAL_FRAMES = 1800;

/**
 * Wie weit die Wiedergabe im Bild vorausschaut, in Takten.
 *
 * Ein Bild rechnet bei 60 Hz ein bis zwei Takte, bei Zeitraffer mehr. Der
 * Vorlauf muss diese Spanne abdecken, damit kein Druck seinen Takt verpasst;
 * der Puffer hält vorzeitig eingespeiste Drücke bis zu ihrem Takt zurück.
 */
const REPLAY_LOOKAHEAD_TICKS = 30;

/** Die Bildzeit-Schwelle aus GAME-DESIGN 20.2 / VERIFICATION § 3.1. */
const FRAME_BUDGET_MS = 16.7;
/** Die Schwelle der Eingabe-Verzögerung aus GAME-DESIGN 20.2 / VERIFICATION § 3.2. */
const INPUT_BUDGET_MS = 16;
/** Namen der Kampf-Zustände in der Reihenfolge aus `combat.ts`. */
// Der vierte kam mit #581 dazu; die Reihenfolge ist die der FIGHT_*-Werte in
// `combat.ts`, nicht eine eigene.
const FIGHT_STATE_NAMES: readonly string[] = ['running', 'won', 'overpressure', 'died'];

export interface PerfSnapshot {
  /** Arbeitszeit je Bild in ms: Logik-Takte plus Zeichnen. Das Maß für Ruckelfreiheit. */
  readonly frameTimes: number[];
  /** Abstand zwischen zwei Bildern in ms. Vom Bildschirm bestimmt, nicht vom Spiel. */
  readonly frameIntervals: number[];
  /** Logik-Takte je Bild. */
  readonly ticksPerFrame: number[];
  /** Eingabe-Verzögerung je Tastendruck in ms, von `keydown` bis zum Zeichnen. */
  readonly inputLatency: number[];
  /** Bilder zwischen `keydown` und dem Bild mit der Wirkung. Muss 1 sein. */
  readonly inputFrameDelay: number[];
  readonly inputLatencySummary: SeriesSummary;
  readonly inputFrameDelaySummary: SeriesSummary;
  /** Speicheranforderung je Bild in Bytes aus der letzten Messung. Erwartet: 0. */
  readonly allocationsPerFrame: number;
  readonly allocations: AllocationResult;
  readonly frameTimeSummary: SeriesSummary;
  readonly frameIntervalSummary: SeriesSummary;
  readonly frames: number;
  readonly ticks: number;
  readonly droppedTicks: number;
  /** Wie oft Zeit für eine Messung verworfen wurde. Nur Werkzeug-Bedingt. */
  readonly discardedForMeasurement: number;
  readonly simTimeMs: number;
}

export interface StateSnapshot {
  readonly running: boolean;
  /** Ganzzahlige Kampfzeit in ms. */
  readonly timeMs: number;
  readonly ticks: number;
  readonly frames: number;
  readonly droppedTicks: number;
  /** Anteil des angebrochenen Takts, in [0, 1). */
  readonly alpha: number;
  /** Restzeit der globalen Wartezeit in ms. */
  readonly gcdRemainingMs: number;
  readonly gcdLengthMs: number;
  /**
   * Der laufende Zauber des Spielers, oder `null` (Issue #304).
   *
   * **Damit ein Nachweis in EINEM Aufruf messbar wird.** Die Wirkzeit dauert
   * zwei Sekunden; zwei getrennte Werkzeug-Aufrufe brauchen laenger, und ein
   * Bild, das danach entsteht, zeigt einen Kampf, der weitergelaufen ist.
   * Gemessen am 2026-08-05: zwischen zwei `playwright-cli`-Aufrufen lagen
   * 6,5 Sekunden — der Zauber war laengst fertig, und der Nachweis haette das
   * Gegenteil dessen belegt, was er behauptete.
   */
  readonly playerCasting: string | null;
  /** Restzeit der laufenden Wirkzeit in ms, sonst 0. */
  readonly playerCastRemainingMs: number;
  /** Wie oft ein Boss-Angriff eine Wirkzeit abgebrochen hat. */
  readonly castsInterrupted: number;
  /** Wie oft ein eigener Druck eine Wirkzeit abgebrochen hat. */
  readonly castsCancelled: number;
  /**
   * Der laufende Zauber des BOSSES, oder `null` (Issue #327).
   *
   * Aus demselben Grund wie `playerCasting`: Ein Nachweis, der zwei
   * Werkzeug-Aufrufe braucht, misst einen Kampf, der zwischen ihnen
   * weitergelaufen ist. Der Zauber von `test-crucible-trial` dauert drei
   * Sekunden — zwischen zwei `playwright-cli`-Aufrufen lagen am 2026-08-05
   * gemessen bis zu siebzehn.
   */
  readonly bossCasting: string | null;
  /** Restzeit des laufenden Boss-Zaubers in ms, sonst 0. */
  readonly bossCastRemainingMs: number;
  /** Wie oft der Spieler einen Boss-Zauber unterbrochen hat. */
  readonly interruptsLanded: number;
  /** Phase des Kampfablaufs (GAME-DESIGN 10). */
  readonly phase: string;
  readonly outcome: string;
  /** Restzeit des Countdowns in ms. */
  readonly countdownRemainingMs: number;
  /** Verstrichene KAMPFzeit in ms — die Uhr, die der Spieler sieht. */
  readonly fightElapsedMs: number;
  readonly overpressureRemainingMs: number;
  /** Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf begann. */
  readonly fightStartedAtMs: number;
  readonly endedAtMs: number;
}

/** Ein Zauber, wie er gerade dasteht. */
export interface SpellState {
  readonly id: string;
  readonly name: string;
  /** Platz auf der Leiste, oder -1. */
  readonly slot: number;
  readonly cooldownMs: number;
  readonly cooldownRemainingMs: number;
  readonly ready: boolean;
  readonly casts: number;
  readonly damage: number;
  /** Restlaufzeit des Effekts über Zeit in ms, 0 wenn keiner läuft. */
  readonly dotRemainingMs: number;
  readonly dotTicks: number;
  readonly lostDotTicks: number;
}

/** Zustand des Kampfes (GAME-DESIGN 19.1: Zustand abfragen und setzen). */
export interface CombatSnapshot {
  readonly timeMs: number;
  readonly state: string;
  readonly damage: number;
  readonly bossName: string;
  readonly bossHp: number;
  readonly bossHpRemaining: number;
  readonly overpressureMs: number;
  readonly endedAtMs: number;
  /** Der Vorrat der Schul-Mechanik — in Schritt 1 die Hitze. */
  readonly resource: number;
  readonly resourceCap: number;
  readonly resourceHighlightAt: number;
  readonly resourceOverflow: number;
  readonly idleMs: number;
  readonly spells: SpellState[];
  /** Ziehungen je Zufallsstrom. In Schritt 1 müssen alle 0 sein. */
  readonly randomDraws: number[];
  /** Was beim Lesen der Spieldaten nicht stimmte. Leer heißt: alles in Ordnung. */
  readonly dataProblems: readonly string[];

  // ===================================================================
  // ALLE übrigen öffentlichen Größen des Kampfes (Issue #625).
  //
  // **Keine Auswahl, und das ist die Entscheidung.** Wer auswählt, muss
  // wissen, welche Größe morgen gebraucht wird — und genau daran ist es
  // zweimal an einem Tag gescheitert: Eine Sitzung suchte `bossEnergy`
  // und `bossStacks`, eine andere `talentAllocation` und `level`. Beide
  // Größen lagen in `Combat` bereit und waren nur nicht durchgereicht;
  // beide Male hatte jemand früher eine sinnvolle Auswahl getroffen, die
  // später nicht mehr passte.
  //
  // Ein Dev-Werkzeug hat keinen Grund zu filtern: Der Produktions-Bau
  // entfernt es, es hat keinen Nutzer, den zu viele Felder verwirren, und
  // ein vollständiger Schnappschuss aller Getter kostet **0,155 ms**
  // (gemessen über 200 Durchläufe an `test-doomsayer`).
  //
  // `schnappschuss-vollstaendig.test.ts` hält das fest: Kommt ein Getter
  // dazu und wird nicht durchgereicht, wird der Fall ROT.
  // ===================================================================

  readonly activeSetEffects: readonly string[];
  readonly afflicted: boolean;
  /**
   * Auch die beiden Sammel-Getter, obwohl `damageFactors()` und `worn()` sie
   * eigens führen.
   *
   * Sie standen als einzige noch außen vor, weil sie über eigene Abfragen
   * erreichbar sind — und genau darin liegt die Falle: „irgendwo erreichbar"
   * ist nicht „im Schnappschuss". Wer `combat()` liest, soll nicht wissen
   * müssen, dass zwei Größen woanders wohnen.
   */
  readonly damageFactors: DamageFactors;
  readonly wornValues: WornValues;
  readonly afflictionsCleared: number;
  readonly bossEnergy: number;
  readonly bossEnergyTriggers: number;
  readonly bossHealed: number;
  readonly bossHpFraction: number;
  readonly bossHpPercent: number;
  readonly bossPhase: number;
  readonly bossStacks: number;
  readonly buffWindowSum: number;
  readonly castingStartedAtMs: number;
  readonly control: number;
  readonly controlFailed: number;
  readonly controlSpent: number;
  /** Wie oft THE LONG GRIP einen Ausgeber umsonst gemacht hat (13.5). */
  readonly freeControlSpendCount: number;
  readonly conversionPercent: number;
  readonly converted: number;
  readonly critFactorsMirrorBase: boolean;
  readonly cycle: number;
  readonly damageLostToRace: number;
  readonly damageTaken: number;
  readonly decayStackCap: number;
  readonly decayStacks: number;
  readonly diedAtMs: number;
  readonly doom: number;
  readonly doomCharges: number;
  readonly healed: number;
  readonly interruptsMissed: number;
  readonly level: number;
  readonly nextCastDoubles: boolean;
  readonly nextCastFreeGcd: boolean;
  readonly phaseHoldPercent: number;
  readonly playerDamage: number;
  readonly playerDead: boolean;
  readonly playerHp: number;
  readonly raceOpen: boolean;
  readonly raceProgress: number;
  readonly raceWon: boolean;
  readonly reflectPercent: number;
  readonly reflected: number;
  readonly resetArmed: boolean;
  readonly resetArmedFor: number;
  readonly resetChargesLeft: number;
  readonly resourceId: string;
  readonly retortArmed: boolean;
  readonly shield: number;
  readonly shieldAbsorbed: number;
  readonly shieldStacks: number;
  readonly stabilised: number;
  readonly synergyAvailable: number;
  readonly synergyUsed: number;
  readonly synergyWindowSum: number;
  readonly talentAllocation: TalentAllocation;
  readonly timelineIndex: number;
  readonly vulnerability: number;
  readonly wardReduction: number;
}

/**
 * Ein Platz der Zauberleiste, wie er gerade gezeichnet wird.
 *
 * Die Maße stehen in **Geräte-Pixeln** auf der Zeichenfläche, nicht in
 * Entwurfs-Koordinaten: So lässt sich mit `getImageData` nachmessen, was
 * tatsächlich im Bild steht, statt dem Spiel zu glauben.
 */
export interface HudSlotSnapshot {
  readonly slot: number;
  readonly spellId: string;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly centerX: number;
  readonly centerY: number;
  readonly ready: boolean;
  readonly cooldownMs: number;
  readonly cooldownRemainingMs: number;
  /** Restanteil der Abklingzeit, 0 bis 1 — der Winkel der Wischblende folgt ihm. */
  readonly sweepShare: number;
  /** Winkel der Blenden-Kante in Radiant, 0 = drei Uhr, wachsend im Uhrzeigersinn. */
  readonly sweepHandAngle: number;
  readonly gcdRemainingMs: number;
  readonly gcdShare: number;
  readonly dotRemainingMs: number;
  readonly dotDurationMs: number;
}

export interface HudSnapshot {
  readonly canvasWidth: number;
  readonly canvasHeight: number;
  readonly scale: number;
  readonly offsetX: number;
  readonly offsetY: number;
  /** Zeichenvorgänge insgesamt. Muss der Zahl der Bilder entsprechen. */
  readonly drawPasses: number;
  readonly frames: number;
  readonly slots: HudSlotSnapshot[];
}

/** Zustand der Trefferwucht (GAME-DESIGN 9). */
export interface ImpactSnapshot {
  /** Plätze im vorab angelegten Pool. */
  readonly poolCapacity: number;
  /** Zahlen, die gerade unterwegs sind. */
  readonly activeCount: number;
  /** Wie oft eine noch sichtbare Zahl wiederverwendet werden musste. */
  readonly recycleCount: number;
  /** Wie viele Zahlen insgesamt gestartet sind. */
  readonly spawnCount: number;
  /** Aktuelle Auslenkung des Bild-Ruckelns in Entwurfs-Pixeln. */
  readonly shakeOffsetPx: number;
  /** Aktuelle Stärke des Aufblitzens, 0 bis 1. */
  readonly flashStrength: number;
}

/** Woraus sich der Schaden eines Treffers zusammensetzt (GAME-DESIGN 6.3, 19.1). */
export interface DamageBreakdown {
  readonly spellId: string;
  readonly baseDamage: number;
  /** Entladeschaden aus dem Vorrat, falls der Zauber einer ist. */
  readonly spenderDamage: number;
  readonly resource: number;
  readonly bonusApplied: boolean;
  /** Die Faktoren in der Reihenfolge, in der sie angewandt werden. */
  readonly factors: { readonly name: string; readonly value: number }[];
  readonly total: number;
}

/** Kennzahlen des Kampfprotokolls. */
export interface LogStats {
  readonly entries: number;
  readonly capacity: number;
  readonly totalWritten: number;
  /** Muss 0 sein. Darüber ist der Puffer zu klein ausgelegt. */
  readonly dropped: number;
  readonly overflowed: boolean;
  readonly byteLength: number;
  readonly counts: Readonly<Record<string, number>>;
}

/** Zustand der Eingabe (GAME-DESIGN 8). */
export interface InputSnapshot {
  /** Angenommene Tastendrücke — sie haben die globale Wartezeit ausgelöst. */
  readonly accepted: number;
  /** Verworfene Tastendrücke — sie fielen in die laufende Wartezeit. */
  readonly rejected: number;
  /** Tastendrücke, die in den Puffer gelangt sind. */
  readonly pressed: number;
  /** Unterdrückte Wiederholungen durch Gedrückthalten. Muss dem Halten entsprechen. */
  readonly suppressedRepeats: number;
  /** Tastendrücke auf nicht belegte Tasten. */
  readonly unbound: number;
  /** Übergangene Tastenfolgen mit Strg, Alt oder Meta. */
  readonly modifierIgnores: number;
  /** Tastendrücke, die der Puffer nicht mehr fassen konnte. Muss 0 sein. */
  readonly lost: number;
  /** Noch nicht abgearbeitete Drücke. Zwischen zwei Takten höchstens wenige. */
  readonly queued: number;
  readonly lastPressedSlot: number;
  /** Woher die Beschriftung stammt. */
  readonly labelSource: string;
  /** Beschriftung der dreizehn Stellen in der Reihenfolge der Leiste. */
  readonly labels: string[];
  /** Tastenkennungen derselben dreizehn Stellen. */
  readonly codes: string[];
  readonly audioReady: boolean;
  readonly audioRequests: number;
  readonly audioPlays: number;
  readonly audioMissing: number;
}

export interface DevTools {
  /** Kennung, an der der CI-Job "build" prüft, dass nichts durchrutscht. */
  readonly marker: '__ROTA_DEVTOOLS__';
  version(): string;
  /** Leistungs-Messwerte (VERIFICATION § 3). */
  perf(): PerfSnapshot;
  /** Zustand des Zeitgerüsts. */
  state(): StateSnapshot;
  /** Zustand der Eingabe (GAME-DESIGN 8). */
  input(): InputSnapshot;
  /** Zustand des Kampfes. */
  combat(): CombatSnapshot;
  /** Lage und Zeitanzeigen der Zauberleiste, in Geräte-Pixeln. */
  hud(): HudSnapshot;
  /**
   * Einen **Prüfgegenstand** auf dem Vorbereitungs-Bildschirm anzeigen
   * (Issue E6-1, AC4).
   *
   * **Nur hier und nicht im Spiel**, weil es noch keine Gegenstände gibt: Der
   * Katalog steht vor E6 aus (GAME-DESIGN 22). Ohne Aufruf bleibt die Karte
   * leer und die ausgelieferte Fassung zeigt nichts — genau wie das
   * Abhör-Soundboard nur im Entwicklungs-Bau entsteht (19.3).
   *
   * Der Name trägt die Kennzeichnung `test-`, damit er nicht als Spielinhalt
   * gilt — dieselbe Linie wie `test-vessel` bei den Bossen.
   */
  showTestGear(): void;
  /**
   * Einen **Prüf-Fund** zeigen (Issue E6-4, AC3 bis AC5).
   *
   * Nur hier und nicht im Spiel, weil es noch keine Gegenstände gibt — der
   * Katalog steht vor E6 aus. Ohne Aufruf zeigt die ausgelieferte Fassung
   * nichts.
   */
  showTestLoot(quality: number): void;
  /** Die Inszenierung überspringen — der Nachweis zu AC4. */
  skipLoot(): boolean;
  /** Zustand der Trefferwucht: Pool, Zahlen, Ruckeln. */
  impact(): ImpactSnapshot;
  /**
   * Das Kampfprotokoll als lesbare Liste (GAME-DESIGN 19.1, VERIFICATION § 3.4).
   *
   * **Erzeugt Speicher** — nur für Werkzeuge. Im Spiel bleibt das Protokoll
   * intern; einen sichtbaren Textstrom gibt es nicht (GAME-DESIGN 21.2).
   */
  log(): LogEntry[];
  /** Kennzahlen des Protokolls, ohne die ganze Liste zu erzeugen. */
  logStats(): LogStats;
  /** Die Kurzfassung nach dem Kampf (GAME-DESIGN 15.1). */
  summary(): RunSummary;
  /** Das Protokoll als Klartext ausgeben (GAME-DESIGN 19.1). */
  logText(limit?: number): string;

  // ---------------------------------------------------- Zeitsteuerung (19.1)
  /** Anhalten. Bilder werden weiter gezeichnet und gemessen. */
  pause(): void;
  resume(): void;
  /** Einzelschritt: `count` Logik-Takte sofort rechnen, ohne auf ein Bild zu warten. */
  step(count?: number): number;
  /** Zeitlupe (< 1) oder Zeitraffer (> 1). Der Takt bleibt 10 ms. */
  setTimeScale(scale: number): void;
  timeScale(): number;

  // ------------------------------------------- Aufzeichnen und Abspielen (19.1)
  /** Namen der eingebauten Aufzeichnungen. */
  recordings(): string[];
  /**
   * Eine Aufzeichnung abspielen — ohne Uhr, ohne Bild, sofort.
   *
   * Zwei Aufrufe liefern byteweise dasselbe. Mit `checksumPerTick` kommt die
   * volle Prüfsummenfolge mit; ohne nur die beiden Gesamtwerte, weil 6000
   * Zahlen selten gebraucht werden.
   */
  replay(name: string, options?: ReplayOptions): ReplayResult;
  /** Dieselbe Aufzeichnung im laufenden Spiel abspielen — für die Bildprüfung. */
  replayLive(name: string): boolean;
  /** Läuft gerade eine Wiedergabe im Spiel? */
  isReplaying(): boolean;

  // --------------------------------------------- Prüfsumme und Herleitung (19.1)
  /** Prüfsumme des jetzigen Zustands. */
  checksum(): number;
  /** Prüfsumme je Takt mitschreiben. Kostet Rechenzeit, daher abschaltbar. */
  setChecksumTracking(on: boolean): void;
  checksumTrail(limit?: number): number[];
  /** Schadens-Herleitung eines Zaubers beim jetzigen Zustand. */
  explainDamage(spellId: string): DamageBreakdown;
  /** Eingabe-Zeitmessung: ms nach Ablauf der globalen Wartezeit, je Druck. */
  inputTiming(): number[];
  /** Alle Abklingzeiten sofort bereit machen (GAME-DESIGN 19.1: Zustand setzen). */
  setCooldownsReady(): void;
  /**
   * Schadenszahlen einspeisen, ohne den Kampf anzufassen.
   *
   * Nur ein Messmittel: Zwanzig Treffer je Sekunde entstehen in Schritt 1 nicht
   * von selbst (ein DoT tickt alle zwei Sekunden), im späteren Spiel mit vielen
   * gleichzeitigen Effekten sehr wohl. Der Kampf bleibt unberührt — es wird
   * nichts gewirkt und kein Schaden gezählt.
   */
  feedHits(count: number, damage?: number, big?: boolean): void;

  // ------------------------------------------------- Ereignis-System (18.3)
  /**
   * Regel-Ereignisse einspeisen, ohne den Kampf anzufassen.
   *
   * Messmittel für den Speicher-Nachweis: Ein Kampf trägt rund sechzig
   * Ereignisse aus — zu wenig, um eine Anforderung je Ereignis von der
   * Messschwankung zu unterscheiden.
   *
   * **Läuft über einen eigenen Bus**, nicht über den des Kampfes: Sonst
   * verschöbe jede Messung die Ereignisfolge, an der der Abgleich mit dem
   * Simulator hängt.
   */
  emitRuleEvents(count: number, event?: number): void;
  /**
   * Einen zählenden Empfänger anmelden.
   *
   * Ohne ihn misst der Speicher-Nachweis nur die Verteilung, nicht den Empfang
   * — und der ist der Teil, an dem später die Schul-Mechaniken hängen. Er
   * hängt am Mess-Bus, nicht am Kampf.
   *
   * @returns wie oft dieser Empfänger schon gerufen wurde, als Ablesefunktion
   */
  addRuleProbe(event: number, id?: string): () => number;
  /** Empfänger eines Ereignisses **des Kampfes**, in ihrer Reihenfolge. */
  ruleReceivers(event: number): string[];
  /**
   * Zustand des Ereignis-Systems **des Kampfes**: Folge, Anzahl, Empfänger.
   *
   * `measurementEmitted` zählt getrennt, was über `emitRuleEvents` eingespeist
   * wurde — das läuft über einen eigenen Bus und verfälscht die Kampf-Folge
   * nicht.
   */
  ruleEvents(): {
    names: string[];
    emitted: number;
    sequenceChecksum: number;
    receivers: Record<string, string[]>;
    measurementEmitted: number;
  };
  /**
   * Einen Zauber auf einen Platz legen — für die Prüfung wachsender Leisten.
   *
   * `null` **leert** den Platz (Issue #306). Ohne das lässt sich eine Leiste
   * ohne einen zuvor gebundenen Zauber gar nicht herstellen.
   */
  bindSlot(slot: number, spellId: string | null): boolean;
  /** Hitze setzen — erspart Wartezeit beim Prüfen (GAME-DESIGN 19.1). */
  setResource(value: number): void;
  /** Verbleibende Lebenspunkte des Bosses setzen (GAME-DESIGN 19.1). */
  /**
   * Die Stufe setzen — fuer Nachweise, die Plaetze oder Grundwerte brauchen.
   *
   * @returns die erreichte Stufe
   */
  setLevel(level: number): number;
  /**
   * Talente verteilen (Issue #417).
   *
   * **Weil es noch keine Oberfläche zum Verteilen gibt**: Der Talentbaum-
   * Bildschirm ist ein eigenes Issue. Ohne diesen Einstieg wäre der Weg vom
   * Spielstand in den Kampf gebaut und im Browser nicht zu zeigen — genau die
   * Klasse, gegen die AC7 geschrieben ist.
   *
   * Gibt zurück, was `allocationProblem` sagt: `null`, wenn die Verteilung
   * zulässig war, sonst den Grund. Sie wird trotzdem gesetzt — ein Dev-Werkzeug
   * soll auch einen Zustand herstellen können, den die Oberfläche verweigert.
   */
  setTalents(allocation: Readonly<Record<string, number>>): string | null;
  /**
   * Ausrüstung anlegen (Issue #421a).
   *
   * **Weil es noch keine Ausrüst-Oberfläche gibt** — die ist ein eigenes
   * Issue, genau wie der Talentbaum-Bildschirm. Ohne diesen Einstieg wäre die
   * Kette vom Spielstand in den Kampf gebaut und im Browser nicht zu zeigen.
   *
   * Gibt die getragenen Werte zurück, damit der Aufrufer sieht, was angekommen
   * ist — ein `setItems`, das schweigt, liesse offen, ob die Kennung überhaupt
   * im Katalog stand.
   */
  setItems(
    items: readonly { itemId: string; upgradeSteps: number; equippedSlot: number | null }[],
  ): unknown;
  /**
   * Der Spielstand: WOHER er kommt und was drinsteht (Issue #378).
   *
   * **Der Grund ist ein Fehler, den zwei Sitzungen an einem Abend unabhängig
   * voneinander gemacht haben**: Der Stand liegt in einem Ringpuffer über fünf
   * Plätze, und der geltende steht in `rota.save.index`. Wer im Browser
   * `localStorage.getItem('rota.save.0')` liest, bekommt einen plausiblen
   * Stand — nur nicht den, der gilt. Beide Male sah es aus wie ein
   * ausgebliebener Fortschritt, beide Male war es der falsche Platz.
   *
   * Es fehlte nicht die Information, sondern der ANLASS, sie im richtigen
   * Moment zu holen. Wer diesen Einstieg benutzt, bekommt den Platz genannt
   * und kann den falschen gar nicht mehr lesen.
   */
  save(): {
    readonly slot: number;
    readonly status: string;
    readonly skipped: number;
    readonly skippedSlots: readonly number[];
    readonly healthy: number;
    readonly data: unknown;
  };
  setBossHp(hp: number): void;
  /** Kampf von vorn beginnen. */
  resetFight(): void;
  /** „Ready" auslösen — der Countdown beginnt. */
  ready(): boolean;
  /** „Wipe Run" auslösen. */
  wipe(): boolean;
  /** Esc-Menü öffnen oder schließen. */
  togglePause(): void;
  /** Ist das Esc-Menü offen? */
  isPaused(): boolean;
  /**
   * Eine Runde nach fester Prioritätenliste durchrechnen und das Ergebnis melden.
   *
   * Das ist der Abgleich gegen `tools/step1-check.py` (GAME-DESIGN 18.4). Reines
   * Werkzeug — im Spiel erscheint keine Referenz-Rotation (GAME-DESIGN 18.5).
   */
  simulate(rotation: RotationName, options?: ReferenceOptions): ReferenceResult;
  /**
   * Der erwartete Ausrüstungsstand einer Instanz (GAME-DESIGN 20.5).
   *
   * Der Maßstab, gegen den Bosse kalibriert werden — **nicht** der eines
   * nackten Spielers. `plaetze` ist überschreibbar, um die Schranken zu
   * rechnen; ohne Angabe alle zehn.
   */
  expectedGear(vorstufe: number, plaetze?: number): readonly OwnedItemLike[];
  /** Wie viele Stücke über `common` liegen — aus den Fallgewichten gerechnet. */
  expectedBetterPieces(vorstufe: number, plaetze: number): number;
  /** Abklingzeit über viele Zyklen messen (VERIFICATION § 3.3). */
  measureCooldown(spellId: string, cycles: number): CooldownMeasurement;
  // ------------------------------------------------- Klang und Musik (17.1)
  /** Abdeckungs-Bericht: Was klingt, was noch nicht, und was ins Leere liefe. */
  sound(): SoundReport;
  /**
   * Ein Kampf-Ereignis von Hand auslösen.
   *
   * Zum Prüfen der Klänge, die ein Lauf nicht erreicht — etwa der Überdruck,
   * wenn man gewinnt. Der Kampf bleibt unberührt; es klingt nur.
   */
  emit(eventName: string): boolean;
  /** Namen aller Kampf-Ereignisse, auf die ein Klang hören kann. */
  events(): string[];
  /** Setzt alle Messreihen zurück. */
  resetPerf(): void;
  /** Misst den Halden-Zuwachs über `frames` Bilder. */
  measureAllocations(frames?: number): Promise<AllocationResult>;
}

/**
 * Die acht Schadensfaktoren als benannte Liste, in der Reihenfolge, in der
 * `computeDamage` sie anwendet.
 *
 * # Warum das eine eigene Funktion ist und nicht acht Zeilen im Aufriss
 *
 * Bis zum 2026-08-10 stand in `explainDamage` eine Liste aus **Konstanten**:
 * fünf von sechs Einträgen fest auf `1`, nur `resistance` gelesen. Das
 * Werkzeug heißt „erklär mir den Schaden" und gab Zahlen aus, die mit dem
 * Kampf nichts zu tun hatten.
 *
 * **Was das gekostet hat:** Es zeigte mit angelegtem Gegenstand `essence: 1`,
 * während derselbe Wurf von 100 auf 221 stieg. Das sah nach dem Befund
 * „Ausrüstung kommt im Spiel nicht an" aus — der wäre falsch gewesen und
 * hätte drei Sitzungen auf die Suche geschickt. Gefangen hat es nur, dass
 * `setItems` daneben `essence: 474` zurückgab: zwei Auskünfte desselben
 * Programms, die sich widersprachen.
 *
 * Als eigene Funktion ist sie **prüfbar**, ohne eine ganze `App` aufzubauen —
 * und ein Rückfall auf Konstanten färbt `tests/unit/explain-damage.test.ts`.
 * Ein Werkzeug, das nur von Menschen gelesen wird, hat sonst keinen Wächter.
 *
 * # Acht, nicht sechs
 *
 * Die alte Liste ließ `gearEffects` und `vulnerability` ganz aus. Sie war
 * also nicht nur falsch, sondern unvollständig: `computeDamage` multipliziert
 * acht Faktoren, und wer sechs zeigt, erklärt einen anderen Schaden als den
 * gefallenen.
 *
 * # `crit` zeigt den Nicht-Krit-Fall, und das ist Absicht
 *
 * `crit` ist in `setDamageFactor` ausdrücklich ausgenommen und lebt nur in
 * `factorsOnCrit`. Ein Aufriss, der den Krit einrechnete, zeigte einen
 * Schaden, den der nächste Wurf nur mit Glück trifft.
 */
export function damageFactorList(
  factors: DamageFactors,
): { readonly name: string; readonly value: number }[] {
  return [
    { name: 'essence', value: factors.essence },
    { name: 'talents', value: factors.talents },
    { name: 'gearEffects', value: factors.gearEffects },
    { name: 'buffs', value: factors.buffs },
    { name: 'synergy', value: factors.synergy },
    { name: 'resistance', value: factors.resistance },
    { name: 'vulnerability', value: factors.vulnerability },
    { name: 'crit', value: factors.crit },
  ];
}

/**
 * Den Widerstand des **gefragten** Zaubers in die Faktorenliste setzen.
 *
 * # Warum das nicht aus `factors.resistance` kommt
 *
 * `combat.ts` setzt diesen Faktor **erst beim Wirken**
 * (`setDamageFactor('resistance', resistanceOfSpell[index])`). Vor dem ersten
 * Wurf steht dort der Neutralwert 1, danach der Wert des **zuletzt
 * gewirkten** Zaubers — beides sagt nichts über den Zauber, nach dem gefragt
 * wurde.
 *
 * Gemessen im Browser am 2026-08-10 gegen `boss-3-1` (CALCINATION 0.8):
 *
 * ```text
 *   vor dem ersten Wurf   explainDamage('kindle').resistance = 1
 *   nach einem KINDLE     dieselbe Abfrage                  = 0.8
 * ```
 *
 * Der Schaden war beide Male 80 statt 100 — die Rechnung stimmte, der Aufriss
 * nicht. **Derselbe Fehler wie die acht Einsen davor**, nur kleiner: ein
 * Wert, der plausibel aussieht und aus der falschen Quelle stammt.
 *
 * Die Abbildung ist dieselbe wie in `combat.ts:1111` beim Aufbau:
 * `boss.resistances[spell.school] ?? 1`. Sie steht hier als eigene Funktion,
 * damit ein Prüffall sie messen kann, ohne eine ganze `App` aufzubauen — sonst
 * wäre die Korrektur wieder ungedeckt.
 */
export function mitWiderstandDesZaubers(
  factors: readonly { readonly name: string; readonly value: number }[],
  resistances: Readonly<Record<string, number>>,
  school: string | null | undefined,
): { readonly name: string; readonly value: number }[] {
  const eigener = school === null || school === undefined ? 1 : (resistances[school] ?? 1);
  return factors.map((f) => (f.name === 'resistance' ? { name: f.name, value: eigener } : f));
}

export function installDevTools(app: App): void {
  if (!__ROTA_DEVTOOLS__) return;

  const perf = new PerfCounters();
  const probe = new AllocationProbe();
  /**
   * Ein eigenstaendiges Ereignis-System zum Messen (Issue E3-1).
   *
   * Es haengt bewusst NICHT am Kampf: Der Anschluss ist Issue E3-1b und wird
   * auf beiden Seiten zugleich gemacht, weil die Stellen, an denen ein Ereignis
   * entsteht, die Reihenfolge festlegen — und die gilt nie einseitig.
   */
  const ruleBus = new RuleEvents();
  /** Name der laufenden Wiedergabe im Bild, sonst `null`. */
  let liveReplay: string | null = null;
  // Die erzwungene Aufräumung hält die Seite an. Ohne diesen Anschluss zählte
  // die Schleife die dabei verlorene Zeit als verworfene Takte des Spiels.
  probe.onCollected = () => {
    app.loop.discardPendingTime();
  };
  let framesSinceMeasurement = 0;
  let warmupLeft = WARMUP_FRAMES;

  app.loop.onFrameMeasured = (frameWorkMs, frameIntervalMs, ticksThisFrame): void => {
    perf.recordFrame(frameWorkMs, frameIntervalMs, ticksThisFrame);

    // Direkt nach dem Zeichnen: Was der Takt sichtbar gemacht hat, ist jetzt im
    // Bild. Genau bis hierhin misst VERIFICATION § 3.2.
    const trace = app.session.latency;
    if (trace.size > 0) {
      const shownAtMs = performance.now();
      const frameIndex = app.loop.frameCount;
      while (trace.size > 0) {
        perf.recordInputLatency(shownAtMs - trace.frontStamp(), frameIndex - trace.frontFrame());
        trace.shift();
      }
    }

    probe.onFrame();
    if (probe.isMeasuring) return;
    if (warmupLeft > 0) {
      warmupLeft--;
      if (warmupLeft === 0) probe.begin(ALLOCATION_WINDOW_FRAMES);
      return;
    }
    framesSinceMeasurement++;
    if (framesSinceMeasurement >= ALLOCATION_INTERVAL_FRAMES) {
      framesSinceMeasurement = 0;
      probe.begin(ALLOCATION_WINDOW_FRAMES);
    }
  };

  const tools: DevTools = {
    marker: '__ROTA_DEVTOOLS__',
    version: () => __ROTA_VERSION__,
    perf: () => ({
      frameTimes: perf.frameWork.toArray(),
      frameIntervals: perf.frameInterval.toArray(),
      ticksPerFrame: perf.tickWork.toArray(),
      inputLatency: perf.inputLatency.toArray(),
      inputFrameDelay: perf.inputFrameDelay.toArray(),
      inputLatencySummary: summarize(perf.inputLatency, INPUT_BUDGET_MS),
      inputFrameDelaySummary: summarize(perf.inputFrameDelay, 1),
      allocationsPerFrame: probe.lastResult.bytesPerFrame,
      allocations: probe.lastResult,
      frameTimeSummary: summarize(perf.frameWork, FRAME_BUDGET_MS),
      frameIntervalSummary: summarize(perf.frameInterval, FRAME_BUDGET_MS),
      frames: perf.frameCount,
      ticks: perf.tickCount,
      droppedTicks: app.loop.droppedTickCount,
      discardedForMeasurement: app.loop.discardCount,
      simTimeMs: app.loop.timeMs,
    }),
    state: () => ({
      running: app.loop.isRunning,
      timeMs: app.loop.timeMs,
      ticks: app.loop.tickCount,
      frames: app.loop.frameCount,
      droppedTicks: app.loop.droppedTickCount,
      alpha: app.loop.alpha,
      gcdRemainingMs: app.session.gcd.remainingMs(app.loop.timeMs),
      gcdLengthMs: app.session.gcd.lengthMs,
      playerCasting: app.combat.spellAt(app.combat.playerCasting)?.id ?? null,
      playerCastRemainingMs:
        app.combat.playerCasting >= 0
          ? Math.max(0, app.combat.playerCastEndsAtMs - app.loop.timeMs)
          : 0,
      castsInterrupted: app.combat.castsInterrupted,
      castsCancelled: app.combat.castsCancelled,
      bossCasting: app.combat.castingName,
      bossCastRemainingMs:
        app.combat.casting >= 0
          ? Math.max(0, app.combat.castLandsAtMs - app.session.fightElapsedMs(app.loop.timeMs))
          : 0,
      interruptsLanded: app.combat.interruptsLanded,
      phase: PHASE_NAMES[app.session.phase] ?? '?',
      outcome: OUTCOME_NAMES[app.session.outcome] ?? '?',
      countdownRemainingMs: app.session.countdownRemainingMs(app.loop.timeMs),
      fightElapsedMs: app.session.fightElapsedMs(app.loop.timeMs),
      overpressureRemainingMs: app.session.overpressureRemainingMs(app.loop.timeMs),
      fightStartedAtMs: app.combat.fightStartedAt,
      endedAtMs: app.session.endedAtMs,
    }),
    input: () => {
      const metrics = app.audio.audio?.metrics();
      const labels: string[] = [];
      const codes: string[] = [];
      for (let position = 0; position < KEYS_PER_LAYER; position++) {
        labels.push(app.labels.label(position));
        codes.push(SLOT_CODES[position] as string);
      }
      return {
        accepted: app.session.acceptedCount,
        rejected: app.session.rejectedCount,
        pressed: app.input.pressed,
        suppressedRepeats: app.input.suppressedRepeats,
        unbound: app.input.unbound,
        modifierIgnores: app.input.modifierIgnores,
        lost: app.pressQueue.lostCount,
        queued: app.pressQueue.size,
        lastPressedSlot: app.session.lastPressedSlot,
        labelSource: app.labels.source,
        labels,
        codes,
        audioReady: app.audio.isReady,
        audioRequests: app.audio.requestCount,
        audioPlays: metrics?.plays ?? 0,
        audioMissing: metrics?.missing ?? 0,
      };
    },
    combat: () => {
      const combat = app.combat;
      const timeMs = app.loop.timeMs;
      const spells: SpellState[] = [];
      for (let i = 0; i < combat.spells.length; i++) {
        const spell = combat.spells[i];
        if (spell === undefined) continue;
        let slot = -1;
        for (let s = 0; s < SLOT_CODES.length * 2; s++) {
          if (app.session.spellOnSlot(s) === i) {
            slot = s;
            break;
          }
        }
        spells.push({
          id: spell.id,
          name: spell.name,
          slot,
          cooldownMs: spell.cooldownMs,
          cooldownRemainingMs: combat.cooldownRemainingMs(i, timeMs),
          ready: combat.isReady(alsSpellIndex(i), timeMs),
          casts: combat.castCount(alsSpellIndex(i)),
          damage: combat.damageOf(alsSpellIndex(i)),
          dotRemainingMs: combat.dotRemainingMs(alsSpellIndex(i), timeMs),
          dotTicks: combat.dotTickCount(i),
          lostDotTicks: combat.lostDotTicks(i),
        });
      }
      return {
        timeMs,
        state: FIGHT_STATE_NAMES[combat.fightState] ?? '?',
        damage: combat.damageDealt,
        bossName: combat.boss.name,
        bossHp: combat.boss.hp,
        bossHpRemaining: combat.bossHpRemaining,
        overpressureMs: combat.boss.overpressureMs,
        endedAtMs: combat.endedAt,
        resource: combat.resource,
        resourceCap: combat.school.resource.cap,
        resourceHighlightAt: combat.school.resource.highlightAt,
        resourceOverflow: combat.resourceOverflow,
        idleMs: app.session.idleMs,
        spells,
        randomDraws: combat.random.drawCounts(),
        dataProblems: app.dataProblems,

        // Alle übrigen Getter — siehe die Begründung am Typ. Alphabetisch,
        // damit ein neuer sichtbar an seinen Platz gehört und nicht hinten
        // angehängt wird.
        activeSetEffects: combat.activeSetEffects,
        afflicted: combat.afflicted,
        damageFactors: combat.damageFactors,
        wornValues: combat.wornValues,
        afflictionsCleared: combat.afflictionsCleared,
        bossEnergy: combat.bossEnergy,
        bossEnergyTriggers: combat.bossEnergyTriggers,
        bossHealed: combat.bossHealed,
        bossHpFraction: combat.bossHpFraction,
        bossHpPercent: combat.bossHpPercent,
        bossPhase: combat.bossPhase,
        bossStacks: combat.bossStacks,
        buffWindowSum: combat.buffWindowSum,
        castingStartedAtMs: combat.castingStartedAtMs,
        control: combat.control,
        controlFailed: combat.controlFailed,
        controlSpent: combat.controlSpent,
        freeControlSpendCount: combat.freeControlSpendCount,
        conversionPercent: combat.conversionPercent,
        converted: combat.converted,
        critFactorsMirrorBase: combat.critFactorsMirrorBase,
        cycle: combat.cycle,
        damageLostToRace: combat.damageLostToRace,
        damageTaken: combat.damageTaken,
        decayStackCap: combat.decayStackCap,
        decayStacks: combat.decayStacks,
        diedAtMs: combat.diedAtMs,
        doom: combat.doom,
        doomCharges: combat.doomCharges,
        healed: combat.healed,
        interruptsMissed: combat.interruptsMissed,
        level: combat.level,
        nextCastDoubles: combat.nextCastDoubles,
        nextCastFreeGcd: combat.nextCastFreeGcd,
        phaseHoldPercent: combat.phaseHoldPercent,
        playerDamage: combat.playerDamage,
        playerDead: combat.playerDead,
        playerHp: combat.playerHp,
        raceOpen: combat.raceOpen,
        raceProgress: combat.raceProgress,
        raceWon: combat.raceWon,
        reflectPercent: combat.reflectPercent,
        reflected: combat.reflected,
        resetArmed: combat.resetArmed,
        resetArmedFor: combat.resetArmedFor,
        resetChargesLeft: combat.resetChargesLeft,
        resourceId: combat.resourceId,
        retortArmed: combat.retortArmed,
        shield: combat.shield,
        shieldAbsorbed: combat.shieldAbsorbed,
        shieldStacks: combat.shieldStacks,
        stabilised: combat.stabilised,
        synergyAvailable: combat.synergyAvailable,
        synergyUsed: combat.synergyUsed,
        synergyWindowSum: combat.synergyWindowSum,
        talentAllocation: combat.talentAllocation,
        timelineIndex: combat.timelineIndex,
        vulnerability: combat.vulnerability,
        wardReduction: combat.wardReduction,
      };
    },
    hud: () => {
      const combat = app.combat;
      const surface = app.surface;
      // Dieselbe Zeit, die auch der Renderer benutzt: die interpolierte. Sonst
      // meldete die Abfrage einen anderen Stand als der, der im Bild steht, und
      // ein Vergleich mit den Bildpunkten wäre wertlos.
      const timeMs = app.loop.timeMs + app.loop.alpha * TICK_MS;
      const scale = surface.designScale;
      const rect: Rect = { x: 0, y: 0, width: 0, height: 0 };
      const slots: HudSlotSnapshot[] = [];
      for (let slot = 0; slot < SLOT_COUNT; slot++) {
        const spellIndex = app.session.spellOnSlot(slot);
        if (spellIndex === NO_SPELL) continue;
        const spell = combat.spellAt(spellIndex);
        if (spell === undefined) continue;
        slotRect(slot, rect);
        const cooldownRemainingMs = combat.cooldownRemainingMs(spellIndex, timeMs);
        const sweepShare = spell.cooldownMs > 0 ? cooldownRemainingMs / spell.cooldownMs : 0;
        const gcdRemainingMs = combat.gcd.remainingMs(timeMs);
        const x = surface.designOffsetX + rect.x * scale;
        const y = surface.designOffsetY + rect.y * scale;
        const width = rect.width * scale;
        const height = rect.height * scale;
        slots.push({
          slot,
          spellId: spell.id,
          x,
          y,
          width,
          height,
          centerX: x + width * 0.5,
          centerY: y + height * 0.5,
          ready: cooldownRemainingMs === 0 && gcdRemainingMs === 0,
          cooldownMs: spell.cooldownMs,
          cooldownRemainingMs,
          sweepShare,
          sweepHandAngle: sweepHandAngle(sweepShare),
          gcdRemainingMs,
          gcdShare: gcdRemainingMs / combat.gcd.lengthMs,
          dotRemainingMs: combat.dotRemainingMs(spellIndex, timeMs),
          dotDurationMs: spell.dot?.durationMs ?? 0,
        });
      }
      return {
        canvasWidth: surface.canvas.width,
        canvasHeight: surface.canvas.height,
        scale,
        offsetX: surface.designOffsetX,
        offsetY: surface.designOffsetY,
        drawPasses: surface.drawPasses,
        frames: app.loop.frameCount,
        slots,
      };
    },
    impact: () => {
      const shownMs = app.loop.timeMs + app.loop.alpha * TICK_MS;
      return {
        poolCapacity: app.impact.poolCapacity,
        activeCount: app.impact.activeCount,
        recycleCount: app.impact.recycleCount,
        spawnCount: app.impact.spawnCount,
        shakeOffsetPx: app.impact.shakeOffset(shownMs),
        flashStrength: app.impact.flashStrength(shownMs),
      };
    },
    log: () => app.session.log.toArray(),
    logStats: () => {
      const log = app.session.log;
      const counts: Record<string, number> = {};
      for (let type = 0; type < EVENT_NAMES.length; type++) {
        counts[EVENT_NAMES[type] as string] = log.countOf(type);
      }
      return {
        entries: log.length,
        capacity: log.capacity,
        totalWritten: log.totalWritten,
        dropped: log.droppedCount,
        overflowed: log.hasOverflowed,
        byteLength: log.byteLength,
        counts,
      };
    },
    summary: () => app.session.summary(app.loop.timeMs),
    logText: (limit = 200) => {
      const entries = app.session.log.toArray();
      const from = Math.max(0, entries.length - limit);
      let out = '';
      for (let i = from; i < entries.length; i++) {
        const e = entries[i];
        if (e === undefined) continue;
        const spell =
          e.spell >= 0 ? (app.combat.spellAt(alsSpellIndex(e.spell))?.id ?? String(e.spell)) : '-';
        out += `${(e.timeMs / 1000).toFixed(2)}  ${e.type.padEnd(10)} ${spell.padEnd(9)} ${e.value}\n`;
      }
      return out;
    },
    pause: () => {
      app.loop.setPaused(true);
    },
    resume: () => {
      app.loop.setPaused(false);
    },
    step: (count = 1) => {
      app.loop.advanceTicks(count);
      return app.loop.timeMs;
    },
    setTimeScale: (scale: number) => {
      app.loop.setTimeScale(scale);
    },
    timeScale: () => app.loop.timeScale,

    recordings: () => recordingNames(),
    replay: (name: string, options: ReplayOptions = {}) => {
      const recording = recordings().get(name);
      if (recording === undefined) {
        throw new Error(
          `Unbekannte Aufzeichnung: ${name}. Bekannt: ${recordingNames().join(', ')}`,
        );
      }
      return replayRecording(recording, options);
    },
    replayLive: (name: string) => {
      const recording = recordings().get(name);
      if (recording === undefined) return false;
      // Die Wiedergabe legt ihre Drücke in denselben Puffer, den auch die
      // Tastatur benutzt — sonst prüfte sie einen anderen Weg als den echten.
      // Der Haken sitzt hier und nicht in `App`, damit weder die Aufzeichnungen
      // noch die Prioritätenliste, aus der sie entstehen, im Auslieferungs-
      // Bundle landen (GAME-DESIGN 18.5 und 19.3).
      app.backToMenu();
      let index = 0;
      /** Takt der Schleife, in dem der Kampf beginnt. -1 = noch unbekannt. */
      let startTick = -1;
      liveReplay = name;
      app.loop.onBeforeTick = (): void => {
        // **Jeder Druck wird auf seinen Takt gelegt, nicht auf „jetzt".**
        //
        // Dieser Haken läuft einmal je **Bild**, in dem ein bis zwei Takte
        // gerechnet werden — er kann den Takt der Aufzeichnung also gar nicht
        // treffen. Ein Druck 10 ms vor Ablauf der Wartezeit wird verworfen, und
        // damit fällt eine Wirkung der Rotation aus.
        //
        // Deshalb wird der Start-Takt schon im Countdown **vorausberechnet**
        // und jeder Druck mit seinem Ziel-Takt in den Puffer gelegt; der hält
        // ihn bis dahin zurück. Gefunden bei E1-9: Der Siegeslauf endete im
        // Überdruck mit 4004 statt 4240 Schaden, die erste Wirkung lag bei
        // 10 ms statt 0 und die zweite kam 1490 ms später.
        const phase = app.session.phase;
        if (startTick < 0) {
          if (phase === PHASE_COUNTDOWN) {
            const left = app.session.countdownRemainingMs(app.loop.timeMs);
            startTick = app.loop.tickCount + Math.ceil(left / TICK_MS);
          } else if (phase === PHASE_FIGHTING) {
            startTick =
              app.loop.tickCount - Math.round(app.combat.fightElapsedMs(app.loop.timeMs) / TICK_MS);
          } else {
            return; // Vorbereitung: Es ist noch nichts entschieden.
          }
        }

        const dueUntil = app.loop.tickCount - startTick + REPLAY_LOOKAHEAD_TICKS;
        while (index < recording.presses.length) {
          const press = recording.presses[index];
          if (press === undefined || press.tick > dueUntil) break;
          index++;
          app.pressQueue.push(
            press.slot,
            press.tick + startTick,
            performance.now(),
            app.loop.frameCount,
          );
        }
        if (index >= recording.presses.length) {
          liveReplay = null;
          app.loop.onBeforeTick = null;
        }
      };
      app.ready();
      return true;
    },
    isReplaying: () => liveReplay !== null,

    checksum: () => combatChecksum(app.combat, app.loop.timeMs),
    setChecksumTracking: (on: boolean) => {
      app.session.setChecksumTracking(on);
    },
    checksumTrail: (limit = 600) => app.session.checksumTrail(limit),
    explainDamage: (spellId: string) => {
      const index = app.combat.indexOf(spellId);
      const spell = app.combat.spellAt(index);
      if (spell === undefined) throw new Error(`Unbekannter Zauber: ${spellId}`);
      const spender = spell.spender;
      const resource = app.combat.resource;
      const bonusApplied = spender !== undefined && resource >= spender.bonusThreshold;
      const spenderRaw =
        spender === undefined
          ? 0
          : spender.damagePerPoint * resource * (bonusApplied ? spender.bonusMultiplier : 1);
      // **Der Widerstand DIESES Zaubers, nicht der des letzten Wurfs.**
      //
      // `factors.resistance` wird erst beim Wirken gesetzt
      // (`setDamageFactor('resistance', resistanceOfSpell[index])`). Vor dem
      // ersten Wurf steht dort der Neutralwert 1, danach der Wert des zuletzt
      // gewirkten Zaubers — beides sagt nichts über den Zauber, nach dem
      // gefragt wurde.
      //
      // Gemessen am 2026-08-10 gegen `boss-3-1` (CALCINATION 0.8):
      //
      // ```text
      //   vor dem ersten Wurf   explainDamage('kindle').resistance = 1
      //   nach einem KINDLE     dieselbe Abfrage                  = 0.8
      // ```
      //
      // Der Schaden war beide Male 80 statt 100 — die Rechnung stimmte, der
      // Aufriss nicht. **Das ist derselbe Fehler wie die acht Einsen davor**,
      // nur kleiner: ein Wert, der plausibel aussieht und aus der falschen
      // Quelle stammt.
      //
      // `resistanceOfSpell` ist privat; die Schule des Zaubers und die
      // Widerstände des Bosses sind es nicht — und `combat.ts` bildet sie
      // genauso ab (`boss.resistances[spell.school] ?? 1`).
      const factors = mitWiderstandDesZaubers(
        damageFactorList(app.combat.damageFactors),
        app.combat.boss.resistances,
        spell.school,
      );
      let total = spell.baseDamage + spenderRaw;
      for (const factor of factors) total *= factor.value;
      return {
        spellId,
        baseDamage: spell.baseDamage,
        spenderDamage: spenderRaw,
        resource,
        bonusApplied,
        factors,
        total: Math.round(total),
      };
    },
    inputTiming: () => app.session.inputTiming.toArray(),
    setCooldownsReady: () => {
      app.combat.setCooldownsReady();
    },

    feedHits: (count: number, damage = 100, big = false) => {
      const now = app.loop.timeMs;
      for (let i = 0; i < count; i++) {
        app.impact.onHit(damage, i % 3 === 0 ? HIT_DOT : HIT_DIRECT, false, big, now);
      }
    },
    emitRuleEvents: (count: number, event = RULE_DAMAGE) => {
      const now = app.loop.timeMs;
      for (let i = 0; i < count; i++) ruleBus.emit(event, i % 3, 100, now);
    },
    addRuleProbe: (event: number, id = 'probe') => {
      let count = 0;
      ruleBus.register(
        { id, on: event },
        {
          onRuleEvent: () => {
            count++;
          },
        },
      );
      return () => count;
    },
    ruleReceivers: (event: number) => app.combat.events.receiverIds(event),
    ruleEvents: () => {
      const receivers: Record<string, string[]> = {};
      for (let event = 0; event < RULE_EVENT_COUNT; event++) {
        receivers[RULE_EVENT_NAMES[event] ?? String(event)] = app.combat.events.receiverIds(event);
      }
      return {
        names: [...RULE_EVENT_NAMES],
        emitted: app.combat.events.emittedCount,
        sequenceChecksum: app.combat.events.sequenceChecksum,
        receivers,
        measurementEmitted: ruleBus.emittedCount,
      };
    },
    bindSlot: (slot: number, spellId: string | null) => {
      // **`null` leert den Platz** (Issue #306, AC7). Vorher lief das ueber
      // `indexOf(null)`, das -1 lieferte und den Aufruf mit `false` abbrechen
      // liess — der Platz blieb belegt, und AC4 war im Browser nicht
      // nachweisbar: Man bekam nie eine Leiste OHNE den Zauber.
      if (spellId === null) {
        app.session.bind(slot, NO_SPELL);
        return true;
      }
      const index = app.combat.indexOf(spellId);
      if (index < 0) return false;
      app.session.bind(slot, index);
      return true;
    },
    setLevel: (level: number) => {
      // **Fuer Nachweise, die eine hoehere Stufe brauchen** (Issue #264): Die
      // Leiste waechst mit ihr, und ohne Platz laesst sich nicht zeigen, dass
      // ein Zauber darauf ankommt. Acht Kaempfe im Browser zu spielen, um
      // Stufe 3 zu erreichen, waere ein Nachweis ueber die Erfahrungskurve —
      // und die ist woanders belegt.
      //
      // Schreibt nur, WAS erreicht wurde (Stufe, Erfahrung auf null): Alles
      // andere wird daraus gerechnet, wie im Spiel (#248, AC5).
      app.setPlayerLevel(level);
      return app.persistence.state.level;
    },
    save: () => ({ ...app.persistence.origin, data: app.persistence.save }),
    setTalents: (allocation: Readonly<Record<string, number>>) => {
      const problem = allocationProblem(
        allocation,
        talentPointsAtLevel(app.persistence.state.level),
      );
      app.setTalents(allocation);
      return problem;
    },
    setItems: (items) => {
      app.setItems(items as never);
      return app.combat.wornValues;
    },
    setBossHp: (hp: number) => {
      app.combat.setBossHpRemaining(hp);
    },
    setResource: (value: number) => {
      app.combat.setResource(value);
    },
    resetFight: () => {
      app.backToMenu();
    },
    ready: () => app.ready(),
    wipe: () => app.wipe(),
    togglePause: () => {
      app.togglePause();
    },
    isPaused: () => app.overlay.isPaused,
    showTestLoot: (quality) => {
      // Ein Fund, der besser ist als das Getragene — so trägt die Anzeige
      // beide Merkmale der Näherung und einen sichtbaren Unterschied.
      app.overlay.showLoot(
        quality,
        'TEST-PRISM OF THE FIRST CRUCIBLE',
        compareItems(1234, 1000),
        performance.now(),
      );
    },
    skipLoot: () => app.overlay.skipLoot(),
    showTestGear: () => {
      // Ein Gegenstand, der BEIDE Arten trägt — nur so lässt sich der
      // Unterschied überhaupt zeigen. Ein Prüfstück mit nur einer Art wäre
      // der triviale Fall, den AC4 ausdrücklich nicht meint.
      app.overlay.showGear('TEST-RELIC · ITEM LEVEL 40 · EPIC', [
        { label: 'ESSENCE', share: 0.2, kind: STAT_ADDITIVE },
        { label: 'SET BONUS (4)', share: 0.3, kind: STAT_ADDITIVE },
        { label: 'CATALYSIS', share: 0.12, kind: STAT_MULTIPLICATIVE },
        { label: 'PURITY', share: 0.08, kind: STAT_MULTIPLICATIVE },
      ]);
    },
    simulate: (rotation, options) => runReference(rotation, options),
    expectedGear: (vorstufe, plaetze) => erwarteterStand(vorstufe, plaetze),
    expectedBetterPieces: (vorstufe, plaetze) => bessereStuecke(vorstufe, plaetze),
    measureCooldown: (spellId, cycles) => measureCooldown(spellId, cycles),
    sound: () => buildSoundReport(app),
    events: () => [...COMBAT_EVENT_NAMES],
    emit: (eventName) => {
      const event = eventIndex(eventName);
      if (event < 0) return false;
      app.audio.onEvent(event, NO_KEY, app.loop.timeMs);
      return true;
    },
    resetPerf: () => {
      perf.reset();
      framesSinceMeasurement = 0;
      warmupLeft = WARMUP_FRAMES;
    },
    measureAllocations: (frames = ALLOCATION_WINDOW_FRAMES) =>
      new Promise<AllocationResult>((resolve) => {
        probe.begin(frames, resolve);
      }),
  };

  (window as unknown as { __rota: DevTools }).__rota = tools;
  console.info('[rota] Entwicklungs-Werkzeuge aktiv: window.__rota');
}
