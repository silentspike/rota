/**
 * Die Kampfregeln (GAME-DESIGN 6, 7, 18.3, STEP1-SPEC).
 *
 * Ganzzahlige Zeit in Millisekunden, feste Reihenfolge der Rechenschritte. Diese
 * Datei ist die Seite des Spiels an der Schnittstelle, die es sich mit dem
 * Rust-Simulator teilt — jede Regel hier muss dort gleich lauten.
 *
 * **Reihenfolge je Logik-Takt**, wie in `tools/step1-check.py`:
 *
 *   1. laufende Wirkungen (DoT-Ticks, Verfall des Vorrats)
 *   2. Prüfung, ob der Boss gefallen ist
 *   3. Tastendrücke dieses Takts
 *
 * Die Reihenfolge ist nicht beliebig: Ein DoT-Tick, der den Boss legt, muss zählen,
 * bevor im selben Takt noch etwas gewirkt wird.
 */
import { computeDamage, NEUTRAL_FACTORS, type DamageFactors } from './damage.js';
import { BIG_HIT_HP_SHARE, HIT_DIRECT, HIT_DOT, type HitFn } from './hits.js';
import { EVENT_CAST, EVENT_DAMAGE, EVENT_DOT_TICK, type CombatLog } from './log.js';
import type { BossDef, SchoolDef, SpellDef } from './data.js';
import { GlobalCooldown } from './gcd.js';
import { RandomStreams } from './rng.js';
import { RULE_DAMAGE, RULE_DOT_TICK, RULE_SPELL_CAST, RuleEvents } from '../combat/events.js';
import { registerStackMechanic, type StackMechanic } from '../combat/schools/stacks.js';
import { registerSublimation, type SublimationMechanic } from '../combat/schools/sublimation.js';

/** Ausgang eines Wirk-Versuchs. */
export const CAST_OK = 0;
export const CAST_UNKNOWN = 1;
export const CAST_ON_COOLDOWN = 2;
export const CAST_ON_GCD = 3;
export const CAST_FIGHT_OVER = 4;

/** Stand des Kampfes. */
export const FIGHT_RUNNING = 0;
export const FIGHT_WON = 1;
export const FIGHT_OVERPRESSURE = 2;

export const NOT_ACTIVE = -1;

export interface CombatOptions {
  readonly school: SchoolDef;
  readonly boss: BossDef;
  /** Startwert der Zufallsströme. In Schritt 1 zieht noch niemand daraus. */
  readonly seed?: number;
}

/**
 * Was ein Wirken bewirkt hat. **Ein einziges, wiederverwendetes Objekt** — der
 * Kampf darf im laufenden Bild nichts erzeugen (GAME-DESIGN 18.2).
 */
export interface CastOutcome {
  result: number;
  spellIndex: number;
  /** Zugefügter Schaden, Direkt- und Entladeschaden zusammen. */
  damage: number;
  /** Vorrat vor dem Wirken — für die Schadens-Herleitung (GAME-DESIGN 19.1). */
  resourceBefore: number;
  /** Hat der Entlade-Zauber seinen Bonus mitgenommen? */
  bonusApplied: boolean;
}

export class Combat {
  readonly school: SchoolDef;
  readonly boss: BossDef;
  readonly spells: readonly SpellDef[];
  readonly gcd = new GlobalCooldown();
  readonly random: RandomStreams;

  /**
   * Empfänger für Treffer (GAME-DESIGN 9 „Wucht").
   *
   * Bleibt `null`, wenn niemand zuhört — dann kostet der Kampf keinen Aufruf.
   */
  onHit: HitFn | null = null;

  /**
   * Das Kampfprotokoll (GAME-DESIGN 18.2). Bleibt `null`, wenn niemand mitschreibt.
   *
   * Geschrieben wird in **Kampfzeit**, nicht in der durchlaufenden Uhr: Ein
   * Protokoll, dessen Nullpunkt vom Laden der Seite abhängt, lässt sich zwischen
   * zwei Läufen nicht vergleichen.
   */
  log: CombatLog | null = null;

  /**
   * Das Ereignis-System (GAME-DESIGN 18.3, Issue E3-1b).
   *
   * **Erst der Schaden, dann die Wirkung** — Entscheidung des Auftraggebers
   * vom 2026-07-29. „Zauber gewirkt" meldet eine **abgeschlossene** Handlung:
   * Ein Empfänger beeinflusst den nächsten Zauber, nicht den laufenden. Der
   * Zersetzungs-Stapel der PUTREFACTION wirkt damit ab dem nächsten Treffer,
   * nicht auf den auslösenden.
   *
   * Der Grund ist kein Geschmack: Bei der umgekehrten Reihenfolge könnte ein
   * Effekt den Schaden verändern, der ihn ausgelöst hat, und solche
   * Rückkopplungen lassen sich in einer Optimums-Suche später nicht mehr
   * auseinanderdividieren.
   *
   * Dieselbe Reihenfolge wie das Kampfprotokoll — zwei Erzählungen desselben
   * Kampfes, die auseinanderliefen, wären beim Abgleich schlimmer als eine.
   */
  readonly events = new RuleEvents();

  /** Wiederverwendetes Ergebnis-Objekt, siehe `CastOutcome`. */
  private readonly outcome: CastOutcome = {
    result: CAST_OK,
    spellIndex: -1,
    damage: 0,
    resourceBefore: 0,
    bonusApplied: false,
  };

  private readonly indexById = new Map<string, number>();
  private readonly cooldownReadyAtMs: Float64Array;
  private readonly castCounts: Int32Array;
  private readonly damageBySpell: Float64Array;
  private readonly dotExpiresAtMs: Float64Array;
  private readonly dotNextTickAtMs: Float64Array;
  private readonly dotTicksLost: Int32Array;
  private readonly dotTickCounts: Int32Array;

  private readonly resistanceFactor: number;
  private readonly factors: DamageFactors;

  private resourceValue = 0;
  private resourceOverflowValue = 0;
  private nextDecayAtMs: number;
  /** Welche Phase gerade läuft. 0 ist der Start (GAME-DESIGN 11.1). */
  private bossPhaseValue = 0;
  /** Stand der Energieleiste — der zweiten Uhr (GAME-DESIGN 11.2). */
  private bossEnergyValue = 0;
  /** Wann die Energieleiste das nächste Mal steigt. */
  private nextEnergyAtMs: number;
  /** Wie oft sie ausgelöst hat. */
  private bossEnergyTriggersValue = 0;

  /**
   * Wie weit die Boss-Zeitleiste abgearbeitet ist — Zeiger auf den **nächsten**
   * fälligen Eintrag (GAME-DESIGN 11.1).
   */
  private timelineIndexValue = 0;
  private damageDealtValue = 0;
  private stateValue = FIGHT_RUNNING;
  private endedAtMs = NOT_ACTIVE;
  private fightStartedAtMs = 0;

  constructor(options: CombatOptions) {
    this.school = options.school;
    this.boss = options.boss;
    this.spells = options.school.spells;
    this.random = new RandomStreams(options.seed ?? 0);

    const count = this.spells.length;
    this.cooldownReadyAtMs = new Float64Array(count);
    this.castCounts = new Int32Array(count);
    this.damageBySpell = new Float64Array(count);
    this.dotExpiresAtMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.dotNextTickAtMs = new Float64Array(count).fill(NOT_ACTIVE);
    this.dotTicksLost = new Int32Array(count);
    this.dotTickCounts = new Int32Array(count);
    for (let i = 0; i < count; i++) this.indexById.set((this.spells[i] as SpellDef).id, i);

    this.nextDecayAtMs = options.school.resource.decayPerStepMs;
    // Genauso wie der Verfall im Konstruktor: Der Replay ruft `beginFight`
    // nicht, und ein Startwert von 0 ließe die Leiste schon im Takt 0 steigen —
    // der Simulator plant den ersten Anstieg bei `perStepMs`. Genau so eine
    // Kleinigkeit lässt beide Seiten auseinanderlaufen.
    this.nextEnergyAtMs = options.boss.energy?.perStepMs ?? 0;
    this.resistanceFactor = options.boss.resistances[options.school.school] ?? 1;
    this.factors = { ...NEUTRAL_FACTORS, resistance: this.resistanceFactor };

    // Die Schul-Mechanik hört ab hier auf Ereignisse (E3-2). Eine Zeile für
    // alle Schulen: Welcher Vorrat gefüllt wird und wie viel, steht in den
    // Daten des jeweiligen Zaubers.
    this.stacks = registerStackMechanic(this);
    this.sublimation = registerSublimation(this);
  }

  /** Die Stapel-Mechanik dieser Schule. Öffentlich für Prüfung und Werkzeuge. */
  readonly stacks: StackMechanic;

  /** Die Verkürzung der Abklingzeiten (E3-3). */
  readonly sublimation: SublimationMechanic;

  // ---------------------------------------------------------------- Abfragen

  indexOf(spellId: string): number {
    return this.indexById.get(spellId) ?? -1;
  }

  spellAt(index: number): SpellDef | undefined {
    return this.spells[index];
  }

  get damageDealt(): number {
    return this.damageDealtValue;
  }

  get bossHpRemaining(): number {
    const left = this.boss.hp - this.damageDealtValue;
    return left > 0 ? left : 0;
  }

  /** Anteil der verbleibenden Lebenspunkte, 0 bis 1. Für den Reaktionsgefäß-Ring. */
  get bossHpFraction(): number {
    return this.bossHpRemaining / this.boss.hp;
  }

  /** Der Vorrat der Schul-Mechanik — in Schritt 1 die Hitze. */
  get resource(): number {
    return this.resourceValue;
  }

  /** Wie viel Vorrat an der Obergrenze verpufft ist. */
  /**
   * Welche Phase gerade läuft. 0 ist der Start.
   *
   * Geht in die Zustands-Prüfsumme ein (Issue #96): Ohne sie wären zwei Läufe
   * in verschiedenen Phasen nicht zu unterscheiden.
   */
  get bossPhase(): number {
    return this.bossPhaseValue;
  }

  /** Stand der Energieleiste. Geht ebenfalls in die Prüfsumme ein. */
  get bossEnergy(): number {
    return this.bossEnergyValue;
  }

  /** Wie oft die Energieleiste ausgelöst hat. */
  get bossEnergyTriggers(): number {
    return this.bossEnergyTriggersValue;
  }

  /** Wie weit die Boss-Zeitleiste abgearbeitet ist. */
  get timelineIndex(): number {
    return this.timelineIndexValue;
  }

  /** Verbleibende Lebenspunkte in Prozent, ganzzahlig abgerundet. */
  get bossHpPercent(): number {
    if (this.boss.hp <= 0) return 0;
    const übrig = this.boss.hp - this.damageDealtValue;
    return Math.floor((Math.max(0, übrig) * 100) / this.boss.hp);
  }

  get resourceOverflow(): number {
    return this.resourceOverflowValue;
  }

  get fightState(): number {
    return this.stateValue;
  }

  /** Kampfzeit, zu der der Kampf endete, sonst `NOT_ACTIVE`. */
  get endedAt(): number {
    return this.endedAtMs;
  }

  /**
   * Zeitpunkt auf der durchlaufenden Uhr, zu dem der Kampf begann.
   *
   * **Zwei Uhren, eine Zeitachse:** Die Abklingzeiten laufen schon in der
   * Vorbereitung mit (GAME-DESIGN 10), die Kampfuhr beginnt erst mit dem Ende des
   * Countdowns. Beide messen dieselbe durchlaufende Zeit — nur der Nullpunkt der
   * Kampfuhr liegt später. Eine zweite, angehaltene Zeitachse wäre die Sorte
   * Sonderfall, an der später jede Aufzeichnung scheitert.
   */
  get fightStartedAt(): number {
    return this.fightStartedAtMs;
  }

  /** Verstrichene **Kampfzeit** — die Zahl, die der Spieler auf der Uhr sieht. */
  fightElapsedMs(timeMs: number): number {
    const elapsed = timeMs - this.fightStartedAtMs;
    return elapsed > 0 ? elapsed : 0;
  }

  /** Restzeit bis zum Überdruck. */
  overpressureRemainingMs(timeMs: number): number {
    const left = this.boss.overpressureMs - this.fightElapsedMs(timeMs);
    return left > 0 ? left : 0;
  }

  /**
   * Den Kampf beginnen lassen.
   *
   * Setzt den Nullpunkt der Kampfuhr und den Verfall-Plan des Vorrats. Die
   * Abklingzeiten laufen unverändert weiter — was in der Vorbereitung gewirkt
   * wurde, bleibt gewirkt.
   */
  beginFight(timeMs: number): void {
    this.fightStartedAtMs = timeMs;
    this.nextDecayAtMs = timeMs + this.school.resource.decayPerStepMs;
    this.nextEnergyAtMs = timeMs + (this.boss.energy?.perStepMs ?? 0);
  }

  castCount(index: number): number {
    return this.castCounts[index] ?? 0;
  }

  damageOf(index: number): number {
    return this.damageBySpell[index] ?? 0;
  }

  /** Verfallene DoT-Ticks durch zu frühes Erneuern (GAME-DESIGN 7.2). */
  lostDotTicks(index: number): number {
    return this.dotTicksLost[index] ?? 0;
  }

  dotTickCount(index: number): number {
    return this.dotTickCounts[index] ?? 0;
  }

  cooldownRemainingMs(index: number, timeMs: number): number {
    const left = (this.cooldownReadyAtMs[index] ?? 0) - timeMs;
    return left > 0 ? left : 0;
  }

  /**
   * Restlaufzeit eines DoT.
   *
   * Grundlage der wichtigsten Entscheidung in Schritt 1: so spät wie möglich
   * erneuern, aber nicht zu spät (GAME-DESIGN 7.2).
   */
  dotRemainingMs(index: number, timeMs: number): number {
    const expires = this.dotExpiresAtMs[index] ?? NOT_ACTIVE;
    if (expires === NOT_ACTIVE) return 0;
    const left = expires - timeMs;
    return left > 0 ? left : 0;
  }

  isDotActive(index: number, timeMs: number): boolean {
    // Größer-gleich: Der letzte Tick fällt mit dem Ablauf zusammen. Eine Dauer von
    // 12 s bei 2 s Tick-Abstand ergibt 6 Ticks (bei 2, 4, 6, 8, 10, 12 s), nicht 5.
    // Entschieden am 2026-07-28 (STEP1-SPEC § 1.2): Der Spieler sieht 12,0 s
    // Restlaufzeit — bei 5 Ticks wären die letzten zwei Sekunden wirkungslos, und
    // "so spät wie möglich erneuern" (GAME-DESIGN 7.2) hieße heimlich "bei 10 s".
    return (this.dotExpiresAtMs[index] ?? NOT_ACTIVE) >= timeMs;
  }

  /** Ist der Zauber selbst bereit? Die globale Wartezeit zählt hier nicht mit. */
  isOffCooldown(index: number, timeMs: number): boolean {
    return (this.cooldownReadyAtMs[index] ?? 0) <= timeMs;
  }

  /** Kann der Zauber jetzt gewirkt werden? */
  isReady(index: number, timeMs: number): boolean {
    return this.isOffCooldown(index, timeMs) && this.gcd.isReady(timeMs);
  }

  // ------------------------------------------------------------------- Takt

  /**
   * Laufende Wirkungen eines Takts: DoT-Ticks und Verfall des Vorrats.
   *
   * Beide laufen unabhängig von Tastendrücken weiter — der Verfall folgt einem
   * festen Plan ab Kampfbeginn, nicht dem letzten Wirken.
   */
  stepPassive(timeMs: number): void {
    if (this.stateValue !== FIGHT_RUNNING) return;

    for (let i = 0; i < this.spells.length; i++) {
      const spell = this.spells[i] as SpellDef;
      const dot = spell.dot;
      if (dot === undefined) continue;
      if (!this.isDotActive(i, timeMs)) continue;
      const nextTick = this.dotNextTickAtMs[i] ?? NOT_ACTIVE;
      if (nextTick === NOT_ACTIVE || timeMs < nextTick) continue;

      const damage = computeDamage(dot.tickDamage, this.factors);
      this.damageDealtValue += damage;
      this.damageBySpell[i] = (this.damageBySpell[i] ?? 0) + damage;
      this.reportHit(damage, HIT_DOT, false, timeMs);
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_DOT_TICK, i, damage);
      // Ein Tick ist **ein** Ereignis, nicht zwei. Wer allen Schaden zählen
      // will, meldet sich für beide Arten an — so bleibt die Folge eindeutig.
      this.events.emit(RULE_DOT_TICK, i, damage, timeMs);
      this.dotTickCounts[i] = (this.dotTickCounts[i] ?? 0) + 1;
      // Der Hitze-Gewinn stand bis E3-2 hier. Er hängt jetzt am Ereignis
      // `dot.tick` (`src/combat/schools/stacks.ts`) — was ein Zauber
      // beisteuert, steht in seinen Daten, nicht im Kampfcode.
      this.dotNextTickAtMs[i] = nextTick + dot.tickIntervalMs;
    }

    const resource = this.school.resource;
    if (resource.decayPerStepMs > 0 && timeMs >= this.nextDecayAtMs) {
      const left = this.resourceValue - resource.decayAmount;
      this.resourceValue = left > 0 ? left : 0;
      this.nextDecayAtMs += resource.decayPerStepMs;
    }

    // Die zweite Uhr (GAME-DESIGN 11.2, Baustein 1). Sie steigt **unabhängig
    // vom Schaden des Spielers** — wer schneller zuschlägt, verschiebt den
    // Auslöser nicht. Genau das macht sie zu einem zweiten Takt, gegen den
    // geplant werden muss.
    const energy = this.boss.energy;
    if (energy !== null && energy.perStepMs > 0) {
      while (timeMs >= this.nextEnergyAtMs) {
        this.bossEnergyValue += energy.amount;
        if (this.bossEnergyValue >= energy.threshold) {
          this.bossEnergyTriggersValue++;
          this.bossEnergyValue = energy.resetTo;
        }
        this.nextEnergyAtMs += energy.perStepMs;
      }
    }

    // Die feste Zeitleiste. Sie hängt weder am Schaden noch an der Phase,
    // sondern allein an der Uhr — deshalb steht sie hier bei den passiven
    // Wirkungen. Die `while`-Schleife holt mehrere Einträge nach, falls ein
    // Takt über sie hinwegspringt; im 10-ms-Takt kann das nur passieren, wenn
    // zwei Einträge auf denselben Zeitpunkt fallen, und dann ist es richtig so.
    //
    // **Was hier noch fehlt, ist die Wirkung** — die Bausteine kommen in E4-2
    // bis E4-6. Geprüft wird vorerst nur der Zeitpunkt.
    // `atMs` zählt **ab Kampfbeginn**, `timeMs` ist die Uhr des Spiels. Beginnt
    // ein Kampf nicht bei null, ist der Unterschied der ganze Versatz — und
    // zwar ein stiller: Die Einträge kämen zu früh, aber der Kampf liefe weiter.
    const seitBeginn = timeMs - this.fightStartedAtMs;
    while (this.timelineIndexValue < this.boss.timeline.length) {
      const eintrag = this.boss.timeline[this.timelineIndexValue];
      if (eintrag === undefined || seitBeginn < eintrag.atMs) break;
      this.timelineIndexValue++;
    }

    // Phasenwechsel an der Lebenspunkt-Schwelle. Die Phase kann nur STEIGEN:
    // Heilt der Boss später einmal (Baustein 5), verschwindet eine schon
    // gezeigte Mechanik nicht wieder.
    if (this.boss.phases.length > 0) {
      const prozent = this.bossHpPercent;
      let neue = 0;
      for (let i = 0; i < this.boss.phases.length; i++) {
        const schwelle = this.boss.phases[i];
        if (schwelle !== undefined && prozent <= schwelle.atHpPercent) neue = i + 1;
      }
      if (neue > this.bossPhaseValue) this.bossPhaseValue = neue;
    }
  }

  /** Prüft, ob der Boss gefallen ist. Nach `stepPassive`, vor dem Wirken. */
  checkDefeat(timeMs: number): boolean {
    if (this.stateValue !== FIGHT_RUNNING) return true;
    if (this.damageDealtValue >= this.boss.hp) {
      this.stateValue = FIGHT_WON;
      this.endedAtMs = timeMs;
      return true;
    }
    // Der Überdruck bemisst sich an der KAMPFuhr, nicht an der durchlaufenden:
    // Die Vorbereitung darf nichts von der Kampfzeit abziehen.
    if (this.fightElapsedMs(timeMs) >= this.boss.overpressureMs) {
      this.stateValue = FIGHT_OVERPRESSURE;
      this.endedAtMs = timeMs;
      return true;
    }
    return false;
  }

  // ----------------------------------------------------------------- Wirken

  /**
   * Einen Zauber wirken.
   *
   * @returns Das wiederverwendete Ergebnis-Objekt. Wer es behalten will, kopiert
   *   die Felder heraus — beim nächsten Aufruf steht etwas anderes darin.
   */
  cast(index: number, timeMs: number): CastOutcome {
    const outcome = this.outcome;
    outcome.spellIndex = index;
    outcome.damage = 0;
    outcome.resourceBefore = this.resourceValue;
    outcome.bonusApplied = false;

    const spell = this.spells[index];
    if (spell === undefined) {
      outcome.result = CAST_UNKNOWN;
      return outcome;
    }
    if (this.stateValue !== FIGHT_RUNNING) {
      outcome.result = CAST_FIGHT_OVER;
      return outcome;
    }
    if (!this.gcd.isReady(timeMs)) {
      outcome.result = CAST_ON_GCD;
      return outcome;
    }
    if (!this.isOffCooldown(index, timeMs)) {
      outcome.result = CAST_ON_COOLDOWN;
      return outcome;
    }

    let damage = 0;
    if (spell.baseDamage > 0) damage += computeDamage(spell.baseDamage, this.factors);

    const spender = spell.spender;
    if (spender !== undefined && spender.resource === this.school.resource.id) {
      const amount = this.resourceValue;
      let raw = spender.damagePerPoint * amount;
      if (amount >= spender.bonusThreshold) {
        // "ab 15 Hitze", nicht "über 14": Die Schwelle selbst zählt mit.
        raw *= spender.bonusMultiplier;
        outcome.bonusApplied = true;
      }
      damage += computeDamage(raw, this.factors);
      if (spender.consumesAll) this.resourceValue = 0;
    }

    if (damage > 0) {
      this.damageDealtValue += damage;
      this.damageBySpell[index] = (this.damageBySpell[index] ?? 0) + damage;
      this.reportHit(damage, HIT_DIRECT, false, timeMs);
      this.log?.push(this.fightElapsedMs(timeMs), EVENT_DAMAGE, index, damage);
      this.events.emit(RULE_DAMAGE, index, damage, timeMs);
      // Hierher gehört `RULE_CRIT`. Es fehlt nicht aus Nachlässigkeit: Krit
      // ruht in Schritt 1 (PURITY = 0, GAME-DESIGN 6.5), `reportHit` bekommt
      // überall `false`. Einen Auslöser zu schreiben, der nie zutrifft, wäre
      // toter Code — die Kennung steht bereit, der Ort ist benannt.
    }

    const dot = spell.dot;
    if (dot !== undefined) {
      // Erneuern setzt zurück: Die verbleibende Restzeit ist verloren, und mit
      // ihr die Ticks, die darin noch gekommen wären (GAME-DESIGN 7.2).
      if (this.isDotActive(index, timeMs)) {
        const remaining = this.dotRemainingMs(index, timeMs);
        this.dotTicksLost[index] =
          (this.dotTicksLost[index] ?? 0) + Math.floor(remaining / dot.tickIntervalMs);
      }
      this.dotExpiresAtMs[index] = timeMs + dot.durationMs;
      this.dotNextTickAtMs[index] = timeMs + dot.tickIntervalMs;
    }

    this.cooldownReadyAtMs[index] = timeMs + spell.cooldownMs;
    if (spell.triggersGcd) this.gcd.trigger(timeMs);
    this.castCounts[index] = (this.castCounts[index] ?? 0) + 1;

    this.log?.push(this.fightElapsedMs(timeMs), EVENT_CAST, index, damage);
    // Zuletzt: Die Wirkung ist abgeschlossen — Schaden, Effekt über Zeit,
    // Vorrat, Abklingzeit und Wartezeit stehen.
    this.events.emit(RULE_SPELL_CAST, index, damage, timeMs);

    outcome.result = CAST_OK;
    outcome.damage = damage;
    return outcome;
  }

  /**
   * Einen Treffer melden.
   *
   * Ob er „groß" ist, entscheidet der Kampf und nicht die Anzeige: Es ist eine
   * Aussage über den Schaden, nicht über den Geschmack.
   */
  private reportHit(damage: number, kind: number, isCrit: boolean, timeMs: number): void {
    const sink = this.onHit;
    if (sink === null) return;
    sink(damage, kind, isCrit, damage >= this.boss.hp * BIG_HIT_HP_SHARE, timeMs);
  }

  /**
   * Stapel hinzufügen — der Zugriff, den die Schul-Mechanik benutzt.
   *
   * Deckelt selbst und zählt mit, was an der Obergrenze verpufft. Öffentlich
   * seit E3-2: Der Gewinn entsteht nicht mehr im Kampfcode, sondern in einem
   * Empfänger (`src/combat/schools/stacks.ts`).
   */
  gainResource(amount: number): void {
    this.addResource(amount);
  }

  /** Was ein Zauber bei einer Wirkung beisteuert — aus seinen Daten. */
  gainOfSpell(spellIndex: number, resourceId: string): number {
    return this.spells[spellIndex]?.resourceGain?.[resourceId] ?? 0;
  }

  /** Was ein Zauber je Tick seines Effekts über Zeit beisteuert. */
  gainOfTick(spellIndex: number, resourceId: string): number {
    return this.spells[spellIndex]?.dot?.resourceGainPerTick[resourceId] ?? 0;
  }

  /** Kennung des Vorrats dieser Schule. */
  get resourceId(): string {
    return this.school.resource.id;
  }

  /** Was ein Zauber laut seinen Daten an Abklingzeit verkürzt (E3-3). */
  reductionOfSpell(spellIndex: number): number {
    return this.spells[spellIndex]?.cooldownReductionMs ?? 0;
  }

  /**
   * Alle laufenden Abklingzeiten verkürzen (SUBLIMATION, GAME-DESIGN 4.1).
   *
   * **Nur laufende.** Ein Zauber, der bereit ist, bleibt unberührt — sonst
   * entstünde eine „negative Restzeit", die den nächsten Wirk-Versuch um genau
   * diesen Betrag früher zuließe, ohne dass es je eine Abklingzeit gegeben
   * hätte. Die Zeit wird deshalb auf den Jetzt-Zeitpunkt begrenzt, nicht
   * einfach abgezogen.
   *
   * @returns wie viele Abklingzeiten tatsächlich verkürzt wurden
   */
  shortenCooldowns(amount: number, timeMs: number, exceptIndex: number): number {
    if (amount <= 0) return 0;
    let touched = 0;
    for (let i = 0; i < this.cooldownReadyAtMs.length; i++) {
      if (i === exceptIndex) continue;
      const readyAt = this.cooldownReadyAtMs[i] ?? 0;
      if (readyAt <= timeMs) continue;
      const shortened = readyAt - amount;
      this.cooldownReadyAtMs[i] = shortened > timeMs ? shortened : timeMs;
      touched++;
    }
    return touched;
  }

  private addResource(amount: number): void {
    if (amount === 0) return;
    const cap = this.school.resource.cap;
    const before = this.resourceValue;
    const raised = before + amount;
    this.resourceValue = raised > cap ? cap : raised;
    this.resourceOverflowValue += amount - (this.resourceValue - before);
  }

  // ------------------------------------------------------------ Zustand setzen

  /**
   * Verbleibende Lebenspunkte setzen — Entwicklungs-Werkzeug (GAME-DESIGN 19.1:
   * „Zustand setzen: Boss-Leben"). Erspart drei Minuten Spielzeit für eine
   * Prüfung kurz vor dem Ende.
   */
  setBossHpRemaining(hp: number): void {
    // Zustand setzen ist ein Werkzeug, mit dem sich das Spiel steuern lässt —
    // und damit genau das, was GAME-DESIGN 19.3 aus der Auslieferung heraushält.
    if (!__ROTA_DEVTOOLS__) return;
    const clamped = hp < 0 ? 0 : hp > this.boss.hp ? this.boss.hp : hp;
    this.damageDealtValue = this.boss.hp - clamped;
  }

  /** Alle Abklingzeiten sofort bereit — Entwicklungs-Werkzeug (GAME-DESIGN 19.1). */
  setCooldownsReady(): void {
    if (!__ROTA_DEVTOOLS__) return;
    this.cooldownReadyAtMs.fill(0);
    this.gcd.reset();
  }

  /** Vorrat setzen — Entwicklungs-Werkzeug (GAME-DESIGN 19.1). */
  setResource(value: number): void {
    if (!__ROTA_DEVTOOLS__) return;
    const cap = this.school.resource.cap;
    this.resourceValue = value < 0 ? 0 : value > cap ? cap : value;
  }

  reset(): void {
    this.cooldownReadyAtMs.fill(0);
    this.castCounts.fill(0);
    this.damageBySpell.fill(0);
    this.dotExpiresAtMs.fill(NOT_ACTIVE);
    this.dotNextTickAtMs.fill(NOT_ACTIVE);
    this.dotTicksLost.fill(0);
    this.dotTickCounts.fill(0);
    this.gcd.reset();
    this.random.reset();
    this.resourceValue = 0;
    this.resourceOverflowValue = 0;
    this.nextDecayAtMs = this.school.resource.decayPerStepMs;
    this.damageDealtValue = 0;
    this.stateValue = FIGHT_RUNNING;
    this.endedAtMs = NOT_ACTIVE;
    this.fightStartedAtMs = 0;
    // Nur die Folge, nicht die Anmeldungen: Ein neuer Versuch behält seine
    // Zauber und Talente — er beginnt lediglich wieder bei null Ereignissen.
    this.events.clearSequence();
  }
}
