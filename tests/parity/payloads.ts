/**
 * Erzeugt die Inhalte der gemeinsamen Vergleichsdateien (Issue #54).
 *
 * Getrennt von den Werkzeugen, die sie schreiben, damit **derselbe** Code sie
 * auch prüfen kann: `freshness.test.ts` baut die Inhalte neu und vergleicht sie
 * mit dem, was im Repository liegt.
 *
 * **Warum das nötig ist:** Der Rust-Simulator prüft gegen die eingecheckten
 * Dateien. Ändert jemand eine Regel im Spiel und schreibt die Dateien nicht neu,
 * prüft der Simulator weiter gegen die alten Werte — und bleibt grün, obwohl
 * beide Seiten längst auseinanderlaufen. Der Prüffall würde dann genau in dem
 * Moment nichts mehr aussagen, in dem er gebraucht wird.
 */
import { combatChecksum, mixFloat, mixInt, recordFieldOrder } from '../../src/core/checksum.js';
import { STEP1_BINDINGS } from '../../src/core/loadout.js';
import { pressRecorder, slotFor, slotMap } from './press-recorder.js';
import { TICK_MS } from '../../src/core/loop.js';
import { hastedMs, tickCount } from '../../src/core/haste.js';
import {
  CRIT_BASE_PERMILLE,
  critChance,
  critFactor,
  gearEffectFactor,
  NOTHING_WORN,
  type WornValues,
} from '../../src/gear/worn.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { statAtLevel } from '../../src/core/data.js';
import { CombatLog, EVENT_KINDS_WRITTEN_BY_GAME, EVENT_NAMES } from '../../src/core/log.js';
import { BUFF_KIND_CAPACITY, Buffs } from '../../src/combat/schools/buffs.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { RandomStreams, STREAM_COUNT, STREAM_NAMES } from '../../src/core/rng.js';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import {
  bossById,
  BOSSES,
  CALCINATION,
  FIRST_CRUCIBLE,
  parseSchool,
  PLAYER,
  type BossDef,
} from '../../src/core/data.js';
import type { Recording, RecordedPress } from '../../src/devtools/recording.js';
import { recordings, replayRecording } from '../../src/devtools/recording.js';
import {
  POOL_CATALYSIS,
  POOL_COUNT,
  POOL_ESSENCE,
  POOL_GEAR_EFFECTS,
  POOL_NAMES,
  POOL_YIELD,
  ValuePools,
} from '../../src/gear/pools.js';
import {
  MAX_ITEM_LEVEL,
  QUALITY_COMMON,
  QUALITY_EPIC,
  QUALITY_COUNT,
  QUALITY_HAS_EFFECT,
  QUALITY_NAMES,
  QUALITY_AFFIX_SLOTS,
  budgetOf,
  distributeBudget,
} from '../../src/gear/budget.js';
import { BASE_STAT_IDS, SLOT_COUNT, SLOT_NAMES } from '../../src/gear/slots.js';
import {
  BUDGET_PER_PERMILLE,
  SOURCE_COUNT,
  SOURCE_NAMES,
  STAT_COUNT,
  STAT_NAMES,
  affixPermille,
  basePermille,
  ESSENCE_AT_LEVEL_1,
  VESSEL_AT_LEVEL_1,
  WARD_AT_LEVEL_1,
  isPermanent,
  maxOf,
  mixedBuild,
  permanentMax,
  pureBuild,
  totalMax,
} from '../../src/gear/sources.js';
import {
  BONUS_ROTATION,
  BONUS_VALUE,
  NO_SET,
  SET_COUNT,
  SET_TIERS,
  activeRotationEffects,
  countPieces,
  tiersReached,
  valueShareOf,
  type SetDef,
} from '../../src/gear/sets.js';
import {
  NO_SUBJECT,
  RULE_BOSS_PHASE,
  RULE_BUFF_EXPIRED,
  RULE_CRIT,
  RULE_DAMAGE,
  RULE_DOT_TICK,
  RULE_EVENT_NAMES,
  RULE_SPELL_CAST,
  RuleEvents,
} from '../../src/combat/events.js';

/** Fassung des Dateiformats. Der Simulator lehnt alles andere ab. */
export const SCHEMA = 1;

/** Welche Aufzeichnung ausgeschrieben wird. */
export const RECORDING_NAME = 'cooldown-precision';

/** Die Runde mit den absichtlichen Fehlgriffen (Issue #95). */
export const MISPLAY_NAME = 'misplays';

/** Der Prueffall gegen den Boss mit Phasen und Energieleiste (Issue #96). */
export const VESSEL_NAME = 'vessel-phases';

/** Der Prueffall zum eingehenden Schaden und zum Unterbrechen (Issue #110). */
export const PLAYER_DAMAGE_NAME = 'player-damage';

/** Der Name der Aufzeichnung, die ueber den WARD-Deckel geht (Issue #466). */
export const WARD_CAP_NAME = 'ward-cap';

/** Der Prueffall zu den drei Wegen der Koagulation (Issue #79). */
export const COAGULATION_NAME = 'coagulation';

/** Der Prueffall zur Fixierung — Kontroll-Punkte auf den Kampfverlauf (#100). */
export const FIXATION_NAME = 'fixation';

/** Der Prueffall zu den Protokoll-Arten des Spiels (Issue #151). */
export const LOG_PARITY_NAME = 'log-parity';

/** Vergleichsdatei für die wiederholte Zeitleiste (#174). */
export const TIMELINE_LOOP_NAME = 'timeline-loop';

/** Vergleichsdatei für den Gleichlauf der Sublimation (#104). */
export const SUBLIMATION_NAME = 'sublimation';

/** Anfangswert von FNV-1a über 32 Bit, wie in `src/core/checksum.ts`. */
const FNV_OFFSET = 0x811c9dc5;

/** Startwerte, gegen die geprüft wird — Randfälle eingeschlossen. */
const SEEDS = [0, 1, 42, 0xffffffff];

/**
 * Die aufgezeichnete Runde samt Ergebnis und Prüfsumme je Takt.
 *
 * Enthält die Belegung, weil ein Tastendruck einen Platz nennt und keinen
 * Zauber (GAME-DESIGN 8.1). Ohne sie müsste der Simulator `loadout.ts`
 * nachbilden, und eine spätere Umbelegung würde jede alte Aufzeichnung still
 * umdeuten.
 */
export function recordingPayload(): Record<string, unknown> {
  const recording = recordings().get(RECORDING_NAME);
  if (recording === undefined) {
    throw new Error(`Unbekannte Aufzeichnung: ${RECORDING_NAME}`);
  }
  const result = replayRecording(recording, { checksumPerTick: true });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((press) => ({ tick: press.tick, slot: press.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      randomDraws: result.randomDraws,
    },
    checksums: result.checksums,
  };
}

/** Eine Ziehung als rohe 32-Bit-Zahl statt als Bruch in [0, 1). */
function drawU32(stream: { next(): number }): number {
  return Math.floor(stream.next() * 4294967296);
}

/**
 * Die Bausteine, auf denen der Gleichlauf beruht: Zufallsströme und Prüfsumme.
 *
 * Sie stehen getrennt vom Kampf, weil ein Kampf-Prüffall sie in Schritt 1 gar
 * nicht berührt — dort zieht niemand (`STEP1-SPEC` § 2). Ein Simulator mit
 * fremdem Erzeuger bestünde jeden Kampf und risse erst ab Schritt 3.
 */
export function primitivesPayload(): Record<string, unknown> {
  const streams = SEEDS.map((seed) => {
    const set = new RandomStreams(seed);
    return {
      seed,
      streamSeeds: Array.from({ length: STREAM_COUNT }, (_, i) => set.stream(i).seed >>> 0),
      draws: Object.fromEntries(
        Array.from({ length: STREAM_COUNT }, (_, i) => [
          STREAM_NAMES[i] ?? String(i),
          Array.from({ length: 8 }, () => drawU32(set.stream(i))),
        ]),
      ),
    };
  });

  const checksum = {
    offset: FNV_OFFSET >>> 0,
    mixInt: [0, 1, 255, 256, 40510, -1, -40510, 0x7fffffff | 0].map((value) => ({
      value,
      result: mixInt(FNV_OFFSET, value) >>> 0,
    })),
    mixFloat: [0, 1, -1, 1500, 4240, 0.5, 12000].map((value) => ({
      value,
      valueBits: bitsOf(value),
      result: mixFloat(FNV_OFFSET, value) >>> 0,
    })),
    chain: (() => {
      let h = FNV_OFFSET;
      for (const value of [1, 2, 3, 4, 5]) h = mixInt(h, value);
      for (const value of [1.5, 2.5]) h = mixFloat(h, value);
      return h >>> 0;
    })(),
  };

  return { schema: SCHEMA, streamNames: STREAM_NAMES, streams, checksum };
}

/**
 * Eine feste Folge von Regel-Ereignissen samt Prüfsumme (Issue E3-1b).
 *
 * **Warum das noch vor dem Anschluss geprüft wird:** Der Abgleich der
 * Ereignisfolge steht und fällt damit, dass beide Seiten dieselbe Rechnung
 * anstellen. Weichen sie schon hier ab, meldet der spätere Vergleich bei jedem
 * Zauber eine Abweichung und bei keinem eine echte — dieselbe Falle, die
 * `primitivesPayload` für die Zufallsströme abfängt.
 *
 * Die Folge ist bewusst künstlich: Sie enthält jedes der sechs Ereignisse, dazu
 * negative Betreffs, Bruchzahlen und einen Wert, der sich erst im letzten Bit
 * unterscheidet.
 */
export function ruleEventsPayload(): Record<string, unknown> {
  const bus = new RuleEvents();
  const steps: {
    event: number;
    subject: number;
    value: number;
    valueBits: string;
    timeMs: number;
  }[] = [];
  const after: number[] = [];

  const emit = (event: number, subject: number, value: number, timeMs: number): void => {
    bus.emit(event, subject, value, timeMs);
    steps.push({ event, subject, value, valueBits: bitsOf(value), timeMs });
    after.push(bus.sequenceChecksum);
  };

  emit(RULE_SPELL_CAST, 0, 100, 0);
  emit(RULE_DAMAGE, 0, 100, 0);
  emit(RULE_DOT_TICK, 1, 40, 2000);
  emit(RULE_CRIT, 2, 360.5, 3000);
  emit(RULE_BOSS_PHASE, NO_SUBJECT, 2, 40_510);
  emit(RULE_BUFF_EXPIRED, 3, 0, 60_000);
  // Ein Wert, der sich erst im letzten Bit unterscheidet — genau solche
  // Abweichungen sucht der Abgleich.
  emit(RULE_DAMAGE, 0, 1, 60_010);
  emit(RULE_DAMAGE, 0, 1 + Number.EPSILON, 60_020);

  // Dieselbe Aufzeichnung wie in `cooldown-precision.json`, diesmal nach ihrer
  // EREIGNISFOLGE gefragt. Das ist der eigentliche Abgleich: dieselbe
  // Tastenfolge muss auf beiden Seiten dieselben Ereignisse in derselben
  // Reihenfolge ergeben. Der Wert steht hier und nicht in
  // `cooldown-precision.json`, damit die Datei unberührt bleibt, gegen die
  // `parity_recording.rs` prüft.
  const referenz = recordings().get(RECORDING_NAME);
  if (referenz === undefined) throw new Error(`Unbekannte Aufzeichnung: ${RECORDING_NAME}`);
  const lauf = replayRecording(referenz);

  return {
    schema: SCHEMA,
    eventNames: RULE_EVENT_NAMES,
    recording: {
      name: RECORDING_NAME,
      damage: lauf.damage,
      killedAtMs: lauf.killedAtMs,
      ruleEventCount: lauf.ruleEventCount,
      ruleSequenceChecksum: lauf.ruleSequenceChecksum,
    },
    steps,
    /** Prüfsumme nach jedem einzelnen Schritt — sagt, WO es auseinanderläuft. */
    checksumAfterEach: after,
    sequenceChecksum: bus.sequenceChecksum,
    emitted: bus.emittedCount,
  };
}

/**
 * Zeichnet eine Runde auf, in der **absichtlich falsch gespielt wird** (#95).
 *
 * Die Prüffälle aus #54 belegen, dass beide Seiten dieselbe Zahl rechnen, wenn
 * der Spieler alles richtig macht. Sie belegen nicht, dass sie auch dann
 * übereinstimmen, wenn er etwas falsch macht — und das ist die größere Hälfte.
 * Ein Kampf besteht aus Fehlgriffen, verpassten Fenstern und zu früh gedrückten
 * Tasten.
 *
 * Vier Fehlertypen, jeder mindestens einmal:
 *
 * | Fehler | Was das Spiel tut |
 * | --- | --- |
 * | Taste während der globalen Wartezeit | abgelehnt (`CAST_ON_GCD`) |
 * | Zauber auf Abklingzeit | abgelehnt (`CAST_ON_COOLDOWN`) |
 * | Brand-Effekt zu früh erneuert | angenommen, Restlaufzeit verloren |
 * | Leerlauf ohne Eingabe | nichts, der GCD läuft leer |
 *
 * **Aufgezeichnet, nicht abgetippt.** Die Tastendrücke entstehen daraus, dass
 * hier ein schlechter Spieler nachgespielt wird — die abgelehnten stehen mit in
 * der Liste, denn genau sie sind der Prüfgegenstand. Das Spiel merkt sich einen
 * abgelehnten Druck ausdrücklich **nicht** (GAME-DESIGN 21.1, verworfene
 * Eingabe-Vormerkung); er ist damit verloren, und beide Seiten müssen das gleich
 * sehen.
 */
function recordMisplays(ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const smoulder = combat.indexOf('smoulder');
  const pyre = combat.indexOf('pyre');
  const presses: RecordedPress[] = [];
  // Drückt einen Zauber, ob er darf oder nicht — und schreibt den Druck mit.
  //
  // **Über den gemeinsamen Helfer** (Issue #351): Hier stand
  // `slotOf.get(spell) ?? 0`, während dieselbe Datei ein paar tausend Zeilen
  // weiter unten bereits die Prüfung trug. Zwei Kopien, von denen eine sie
  // hatte — genau der Zustand, der zu dem Fehler führte.
  const press = pressRecorder(combat, STEP1_BINDINGS, presses);

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;

    // Fehler 4 — Leerlauf: Zwischen 20 s und 24 s wird gar nichts gedrückt,
    // obwohl der GCD frei ist. Vier Sekunden verschenkte Zeit.
    if (timeMs >= 20_000 && timeMs < 24_000) continue;

    // Fehler 1 — während der globalen Wartezeit drücken. Alle 300 ms ein
    // zusätzlicher Druck auf KINDLE, der meistens auf den laufenden GCD trifft.
    if (!combat.gcd.isReady(timeMs) && timeMs % 300 === 0) {
      press(kindle, tick, timeMs);
      continue;
    }
    if (!combat.gcd.isReady(timeMs)) continue;

    // Fehler 2 — auf Abklingzeit drücken: PYRE alle 2 s, obwohl es 5 s braucht.
    if (timeMs % 2000 === 0 && !combat.isReady(pyre, timeMs)) {
      press(pyre, tick, timeMs);
      continue;
    }

    // Fehler 3 — zu früh erneuern: SMOULDER bei jeder halben Laufzeit statt am
    // Ende. Wird angenommen, wirft aber die halbe Restzeit weg.
    if (combat.dotRemainingMs(smoulder, timeMs) < 6000 && combat.isReady(smoulder, timeMs)) {
      press(smoulder, tick, timeMs);
      continue;
    }

    // PYRE, sobald bereit — ohne auf die Bonus-Schwelle zu warten.
    if (combat.isReady(pyre, timeMs) && combat.resource > 0) {
      press(pyre, tick, timeMs);
      continue;
    }

    if (combat.isReady(kindle, timeMs)) press(kindle, tick, timeMs);
  }

  return { name: MISPLAY_NAME, seed: 0, ticks, presses };
}

/**
 * Die Runde mit Fehlgriffen samt Ergebnis und Prüfsumme je Takt.
 *
 * `rejectedByReason` steht mit in der Datei: Ohne diese Aufschlüsselung ließe
 * sich nicht belegen, dass **jeder** Fehlertyp mindestens einmal vorkommt — eine
 * Aufzeichnung mit fünfzig abgelehnten Drücken derselben Sorte sähe genauso aus.
 */
export function misplaysPayload(): Record<string, unknown> {
  const recording = recordMisplays(6000);
  const result = replayRecording(recording, { checksumPerTick: true });

  // Die Aufzeichnung noch einmal durchspielen und dabei zählen, WAS genau
  // schiefging. `replayRecording` liefert nur "angenommen" und "abgelehnt" —
  // damit ließe sich nicht belegen, dass jeder der vier Fehlertypen vorkommt.
  const analyse = analyzeMisplays(recording);

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      randomDraws: result.randomDraws,
      rejectedByReason: analyse.reasons,
      lostDotTicks: analyse.lostDotTicks,
      idleTicks: analyse.idleTicks,
    },
    checksums: result.checksums,
  };
}

/**
 * Der gemeinsame Prüffall gegen einen Boss **mit** Phasen, Energieleiste und
 * Zeitleiste (Issue #96, AC4).
 *
 * Die drei bestehenden Vergleichsdateien laufen alle gegen `first-crucible` —
 * einen Boss ohne all das. Sie belegen damit genau **nichts** über die neuen
 * Felder: Phase, Energie und Zeitleisten-Zeiger stehen dort von Anfang bis Ende
 * auf null, und eine Umsetzung, die sie überhaupt nicht führt, bestünde jeden
 * dieser Prüffälle.
 *
 * Deshalb hier dieselbe gute Rotation gegen `test-vessel`. Was sich zwischen
 * beiden Aufzeichnungen unterscheidet, kommt aus den Boss-Daten und aus nichts
 * sonst.
 *
 * **Die drei Zustandsspuren stehen ausdrücklich neben der Prüfsumme.** Sie
 * gehen ohnehin in sie ein — aber eine abweichende Prüfsumme sagt nur *dass*
 * etwas anders ist, nicht *was*. Mit den Spuren zeigt ein Fehlschlag direkt auf
 * die Phase, die Energie oder die Zeitleiste.
 */
export function bossStatePayload(): Record<string, unknown> {
  const recording = recordings().get(VESSEL_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${VESSEL_NAME}`);
  const boss = bossById('test-vessel');
  if (boss === undefined) throw new Error('Der Boss test-vessel fehlt.');

  const result = replayRecording(recording, { checksumPerTick: true, boss });

  // Zweiter Durchlauf für die Zustandsspuren. Ein eigener Durchlauf und nicht
  // ein erweitertes `ReplayResult`: Die Spuren interessieren nur diesen
  // Prüffall, und `replayRecording` ist die Schnittstelle aller anderen.
  const phasen: number[] = [];
  const energie: number[] = [];
  const zeitleiste: number[] = [];
  // Der Anteil der Lebenspunkte je Takt, **so wie die passive Phase ihn sieht**
  // — also vor dem Wirken dieses Takts.
  //
  // Unentbehrlich fuer AC2: Ohne diese Spur liesse sich der Phasenwechsel nur
  // gegen die Phase selbst pruefen, und das waere zirkulaer.
  //
  // Der Zeitpunkt ist dabei die halbe Miete. Der Phasenwechsel geschieht in
  // `stepPassive`, also **bevor** der Spieler in diesem Takt handelt. Ein nach
  // dem Wirken abgelesener Anteil gehoerte damit schon zum naechsten Takt, und
  // die Pruefung "der Takt davor lag noch ueber der Schwelle" waere um einen
  // Takt verschoben — sie wuerde fehlschlagen, obwohl beide Seiten richtig
  // rechnen. Genau das ist beim ersten Versuch passiert.
  const hpAnteil: number[] = [];
  // Die Lebenspunkte des Spielers je Takt (Issue #110). Ohne diese Spur bliebe
  // der eingehende Schaden in der Prüfsumme verborgen: Sie meldete zwar eine
  // Abweichung, aber nicht, dass es um den Spieler geht.
  const spielerHp: number[] = [];
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    // Vor dem Wirken abgelesen — siehe oben.
    const anteilVorDemWirken = combat.bossHpPercent;
    if (combat.damageDealt >= combat.boss.hp) {
      phasen.push(combat.bossPhase);
      energie.push(combat.bossEnergy);
      zeitleiste.push(combat.timelineIndex);
      hpAnteil.push(anteilVorDemWirken);
      spielerHp.push(combat.playerHp);
      break;
    }
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of STEP1_BINDINGS) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    phasen.push(combat.bossPhase);
    energie.push(combat.bossEnergy);
    zeitleiste.push(combat.timelineIndex);
    hpAnteil.push(anteilVorDemWirken);
    spielerHp.push(combat.playerHp);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    // **Beide Seiten muessen dieselbe Menge sehen.** Das Spiel liest das
    // Verzeichnis ueber `import.meta.glob`, der Simulator ueber `build.rs` —
    // zwei voellig verschiedene Wege zum selben Ergebnis. Kennt einer einen
    // Boss, den der andere nicht hat, faellt das ohne diese Liste erst auf,
    // wenn jemand gegen ihn rechnen will.
    allBossIds: [...BOSSES.keys()],
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      phases: boss.phases.map((p) => ({ atHpPercent: p.atHpPercent, name: p.name })),
      energy: boss.energy,
      timeline: boss.timeline.map((e) => ({ atMs: e.atMs, mechanic: e.mechanic })),
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      randomDraws: result.randomDraws,
      finalPhase: phasen[phasen.length - 1] ?? 0,
      energyTriggers: combat.bossEnergyTriggers,
      finalTimelineIndex: zeitleiste[zeitleiste.length - 1] ?? 0,
      finalPlayerHp: combat.playerHp,
      damageTaken: combat.damageTaken,
      diedAtMs: combat.diedAtMs,
      interruptsLanded: combat.interruptsLanded,
      interruptsMissed: combat.interruptsMissed,
    },
    player: { vessel: PLAYER.vessel, wardReduction: PLAYER.wardReduction },
    checksums: result.checksums,
    phases: phasen,
    energy: energie,
    timelineIndex: zeitleiste,
    hpPercent: hpAnteil,
    playerHp: spielerHp,
  };
}

/** Der Prueffall zu Zaehler und exponentiellem Stapel (Issue #216). */
export const DOOM_STACKS_NAME = 'doom-stacks';

/**
 * Zaehler und exponentieller Stapel, Takt fuer Takt (Issue #216, AC5).
 *
 * Gebaut wie `bossStatePayload`, mit drei zusaetzlichen Spuren. Sie gehen
 * ohnehin in die Pruefsumme ein — aber eine abweichende Pruefsumme sagt nur,
 * **dass** etwas anders ist. Mit den Spuren zeigt ein Fehlschlag direkt auf den
 * Zaehler, seine Ladungen oder die Stapel.
 *
 * **Der Lauf geht ueber den Tod hinaus.** Der Zaehler toetet bei 20 s, die
 * Aufzeichnung laeuft 30 s. Die zehn Sekunden danach sind kein Beiwerk: Sie
 * belegen, dass beide Seiten nach dem Tod dasselbe tun — naemlich nichts mehr.
 * Genau dort liefen zwei Umsetzungen sonst auseinander, ohne dass es jemand
 * merkte, weil der Kampf ja ohnehin vorbei ist.
 */
export function doomStacksPayload(): Record<string, unknown> {
  const recording = recordings().get(DOOM_STACKS_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${DOOM_STACKS_NAME}`);
  const boss = bossById('test-doomsayer');
  if (boss === undefined) throw new Error('Der Boss test-doomsayer fehlt.');

  const zaehler: number[] = [];
  const ladungen: number[] = [];
  const stapel: number[] = [];
  const spielerHp: number[] = [];
  const checksums: number[] = [];
  let sequence = 0;

  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    // **Der Spielertod beendet den Versuch** (GAME-DESIGN A.3), mit der
    // Prüfsumme dieses Takts — dieselbe Stelle und dieselbe Reihenfolge wie in
    // `playerDamagePayload` und in `Fight::replay`.
    //
    // Nicht `replayRecording`: Die kennt nur den Tod des BOSSES und liefe hier
    // bis zum Ende der Aufzeichnung weiter. Gemessen hat der Simulator 2001
    // Einträge geschrieben und meine erste Fassung 3000 — bei auf jeden Takt
    // gleichen Werten. Nicht die Rechnung lag daneben, sondern die Frage,
    // wann ein Lauf endet.
    if (combat.playerDead) {
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      checksums.push(hash);
      zaehler.push(combat.doom);
      ladungen.push(combat.doomCharges);
      stapel.push(combat.bossStacks);
      spielerHp.push(combat.playerHp);
      break;
    }

    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of STEP1_BINDINGS) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
    zaehler.push(combat.doom);
    ladungen.push(combat.doomCharges);
    stapel.push(combat.bossStacks);
    spielerHp.push(combat.playerHp);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      doom: boss.doom,
      stacks: boss.stacks,
    },
    expected: {
      damage: combat.damageDealt,
      // **Nicht die letzte Takt-Prüfsumme.** Beide Seiten stempeln den
      // Schlusswert mit `(ticks - 1) * TICK_MS`, auch wenn der Lauf früher
      // endet — `Fight::replay` mit `final_time` und `replayRecording` mit
      // derselben Formel. Die Zeit geht in die Prüfsumme ein, also ergibt der
      // Wert bei 20 000 ms einen anderen als bei 29 990. Mein erster Entwurf
      // nahm den letzten Takt und wich damit ab, obwohl alle 2001
      // Takt-Prüfsummen übereinstimmten.
      finalChecksum: combatChecksum(combat, (recording.ticks - 1) * TICK_MS),
      sequenceChecksum: sequence,
      finalDoom: combat.doom,
      finalDoomCharges: combat.doomCharges,
      finalStacks: combat.bossStacks,
      finalPlayerHp: combat.playerHp,
      damageTaken: combat.damageTaken,
      diedAtMs: combat.diedAtMs,
    },
    player: { vessel: PLAYER.vessel, wardReduction: PLAYER.wardReduction },
    // **Die Namensfolge der Prüfsumme** (Issue #236).
    //
    // Gemessen wird sie am ENDZUSTAND, nicht bei t = 0 — und das ist der
    // ganze Punkt. `combatChecksum` mischt Zerfallsstapel in einer Schleife;
    // bei einem frischen Kampf ist die Schleife leer, der Name `decayExpiry`
    // erschiene nie, und der Wächter wäre für genau die Stelle blind, an der
    // eine Reihenfolge am ehesten auseinanderläuft.
    //
    // Deshalb steht sie hier und nicht in `primitivesPayload`: Nur hier gibt
    // es einen Kampf, der etwas erlebt hat.
    fieldOrder: recordFieldOrder(() => {
      combatChecksum(combat, (recording.ticks - 1) * TICK_MS);
    }),
    checksums,
    doom: zaehler,
    doomCharges: ladungen,
    stacks: stapel,
    playerHp: spielerHp,
  };
}

/** Der Prueffall zum Auswahlmodus des Ruecksetzers (Issue #223). */
export const RESET_RUN_NAME = 'cooldown-reset-run';

/**
 * Der Auswahlmodus, Takt für Takt (Issue #223).
 *
 * **Über eine `Session`, nicht über `replayRecording`.** Der Modus lebt in der
 * Sitzung: Sie entscheidet, ob ein Druck wirkt oder wählt. `replayRecording`
 * ruft `combat.cast` und würde REFLUX als gewöhnlichen Zauber wirken — die
 * Prüfsummen wären dann die eines Laufs, den es so nie gab.
 *
 * Zwei Spuren neben den Prüfsummen: was der Modus getan hat (angewandt oder
 * verpufft) und ob er gerade scharf steht. Eine abweichende Prüfsumme sagt
 * nur, **dass** etwas anders ist; die Spuren sagen **was**.
 *
 * **Nicht zu verwechseln mit `cooldownResetPayload`** (Issue E3-3c). Der prüft
 * die REGELN des Modus an einem erfundenen Zauberkasten und einer festen
 * Schrittfolge — und das tat er, als kein ausgelieferter Zauber ihn benutzte.
 * Dieser hier prüft den **Druck-Pfad im Kampf** an einem echten Lauf mit
 * REFLUX. Genau der Pfad fehlte im Simulator (Issue #223).
 */
export function resetRunPayload(): Record<string, unknown> {
  const recording = recordings().get(RESET_RUN_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${RESET_RUN_NAME}`);

  const belegung = [
    { slot: 0, spellId: 'kindle' },
    { slot: 1, spellId: 'pyre' },
    { slot: 2, spellId: 'reflux' },
  ];
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    seed: recording.seed,
  });
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  for (const { slot, spellId } of belegung) session.bind(slot, combat.indexOf(spellId));

  const checksums: number[] = [];
  const scharf: number[] = [];
  let sequence = 0;
  let pressIndex = 0;

  // `startFight` statt `beginCountdown` — siehe `recordResetRotation`: Der
  // Simulator kennt keinen Countdown.
  session.startFight(0);
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      queue.push(press.slot, tick, timeMs, tick);
    }
    session.tick(timeMs);

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
    scharf.push(session.isResetArmed ? 1 : 0);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: belegung,
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: combat.damageDealt,
      // Wie oft der Modus gegriffen hat und wie oft er verpufft ist. Beide
      // Zahlen gehoeren dazu: Ein Lauf, in dem jede Wahl verpufft, saehe
      // ohne sie aus wie einer, in dem jede greift.
      resetsApplied: session.resetsAppliedCount,
      resetsWasted: session.resetsWastedCount,
      accepted: session.acceptedCount,
      rejected: session.rejectedCount,
      finalChecksum: combatChecksum(combat, (recording.ticks - 1) * TICK_MS),
      sequenceChecksum: sequence,
    },
    checksums,
    armed: scharf,
  };
}

/** Der Prueffall zum Wettrennen und zur Boss-Heilung (Issue #218). */
export const RACE_HEAL_NAME = 'race-heal';

/**
 * Wettrennen und Boss-Heilung, Takt für Takt (Issue #218, AC6).
 *
 * **Beide Bausteine in EINEM Lauf**: Das Fenster liegt bei 10 s, der
 * Boss-Zauber bei 30 s. Sie getrennt zu prüfen würde nicht zeigen, dass sie
 * sich nicht ins Gehege kommen — und genau dort läuft so etwas auseinander.
 *
 * Vier Spuren neben den Prüfsummen. Sie gehen ohnehin in die Prüfsumme ein,
 * aber eine abweichende Prüfsumme sagt nur, **dass** etwas anders ist; die
 * Spuren sagen **was**: das offene Fenster, der geladene Zähler, der verlorene
 * Schaden und der zurückgedrehte Fortschritt.
 */
export function raceHealPayload(): Record<string, unknown> {
  const recording = recordings().get(RACE_HEAL_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${RACE_HEAL_NAME}`);
  const boss = bossById('test-racer');
  if (boss === undefined) throw new Error('Der Boss test-racer fehlt.');

  const result = replayRecording(recording, { checksumPerTick: true, boss });

  const offen: number[] = [];
  const geladen: number[] = [];
  const verloren: number[] = [];
  const geheilt: number[] = [];
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) {
      offen.push(combat.raceOpen ? 1 : 0);
      geladen.push(combat.raceProgress);
      verloren.push(combat.damageLostToRace);
      geheilt.push(combat.bossHealed);
      break;
    }
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of STEP1_BINDINGS) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    offen.push(combat.raceOpen ? 1 : 0);
    geladen.push(combat.raceProgress);
    verloren.push(combat.damageLostToRace);
    geheilt.push(combat.bossHealed);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      race: boss.race,
      bossHeal: boss.bossHeal,
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      raceWon: combat.raceWon,
      finalRaceProgress: combat.raceProgress,
      damageLostToRace: combat.damageLostToRace,
      bossHealed: combat.bossHealed,
      interruptsMissed: combat.interruptsMissed,
    },
    checksums: result.checksums,
    raceOpen: offen,
    raceProgress: geladen,
    damageLostToRace: verloren,
    bossHealed: geheilt,
  };
}

export const LEVEL_30_NAME = 'level-30';

/**
 * Derselbe Lauf auf STUFE 30 (Issue #289, AC5).
 *
 * **Warum diese Datei ueberhaupt entsteht:** Bis zum 2026-08-05 lag keine
 * einzige Vergleichsdatei ueber Stufe 1 — und dort ist `statAtLevel` neutral.
 * Die Bitgleichheit war fuer jede hoehere Stufe unbelegt, und ein
 * Auseinanderlaufen waere unsichtbar geblieben.
 *
 * Das ist die Klasse „der Prueffall kann den Unterschied nicht zeigen", an
 * der teuersten Stelle, an der sie stehen kann: Die Bitgleichheit ist das
 * Fundament, auf dem der Balance-Laeufer ruht.
 *
 * **Stufe 30, nicht 2.** Der Faktor ist dort 4,116 und trennt die Zahlen
 * deutlich; bei 1,05 saehe ein Rundungsfehler aus wie Uebereinstimmung.
 *
 * Die Stufe steht IN der Datei (`playerLevel`), nicht in der Annahme des
 * Lesers — wer sie liest, weiss, womit gerechnet wurde.
 */
export function level30Payload(): Record<string, unknown> {
  const recording = recordings().get(LEVEL_30_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${LEVEL_30_NAME}`);
  const LEVEL = 30;
  const result = replayRecording(recording, { checksumPerTick: true, level: LEVEL });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    playerLevel: LEVEL,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    player: {
      vessel: PLAYER.vessel,
      wardReduction: PLAYER.wardReduction,
      // Der Wert DIESER Stufe — die Zahl, um die es geht.
      vesselAtLevel: statAtLevel(PLAYER.vessel, LEVEL),
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
    },
    checksums: result.checksums,
  };
}

export const RUNAWAY_PROGRESS_NAME = 'runaway-progress';

/**
 * Die davonlaufende Reaktion mit SPIELERSCHADEN, Takt für Takt (Issue #256, AC3).
 *
 * **Bis zum 2026-08-05 hat keine einzige Vergleichsdatei einen Boss mit
 * `runaway` gefahren.** `parity_runaway.rs` prüft Zahlen und keine
 * Prüfsummen; alle Prüfsummen-Dateien laufen gegen Bosse ohne Reaktion. Der
 * Simulator war an dieser Stelle also ungeprüft — und das fiel an einer
 * Gegenprobe auf, die NICHT anschlug: Wer `runaway_tick` die falsche Herkunft
 * mitgeben ließ, brach keinen Test.
 *
 * Das ist wörtlich die Frage aus der vierten Gestalt, einmal höher gestellt:
 * Könnte diese Prüfung den Unterschied überhaupt zeigen?
 *
 * Zwei Spuren neben den Prüfsummen. Beide gehen ohnehin in die Prüfsumme ein,
 * aber eine abweichende Prüfsumme sagt nur, **dass** etwas anders ist; die
 * Spuren sagen **was** — der Ring-Verlust insgesamt und der Anteil des
 * Spielers daran. Genau diese Trennung baut #256, und ohne sie sähen ein Lauf
 * ohne Tastendruck und ein Lauf mit KINDLE gleich aus.
 */
export function runawayProgressPayload(): Record<string, unknown> {
  const recording = recordings().get(RUNAWAY_PROGRESS_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${RUNAWAY_PROGRESS_NAME}`);
  const boss = bossById('test-runaway');
  if (boss === undefined) throw new Error('Der Boss test-runaway fehlt.');

  const result = replayRecording(recording, { checksumPerTick: true, boss });

  const ring: number[] = [];
  const vomSpieler: number[] = [];
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) {
      ring.push(combat.damageDealt);
      vomSpieler.push(combat.playerDamage);
      break;
    }
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of STEP1_BINDINGS) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    ring.push(combat.damageDealt);
    vomSpieler.push(combat.playerDamage);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      runaway: boss.runaway,
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      finalPlayerDamage: combat.playerDamage,
    },
    checksums: result.checksums,
    ringDamage: ring,
    playerDamage: vomSpieler,
  };
}

export const RESISTANCE_NAME = 'resistances';

/**
 * Ein Kampf gegen einen Boss mit WIDERSTAENDEN, Takt für Takt (Issue #328, AC7).
 *
 * **Bis zu diesem Issue hat der Simulator das Feld nicht gelesen.**
 * `grep -c "resist" sim/build.rs` ergab 0; `factors.resistance` blieb dort auf
 * 1,0, während das Spiel den Boss-Widerstand seit langem anwendet. Die
 * Abweichung war unsichtbar, weil alle elf Boss-Dateien `resistances: {}`
 * trugen und die Suche im Spiel dann ebenfalls 1,0 lieferte — eine Abweichung,
 * die auf ihre erste Datenzeile gewartet hat.
 *
 * **Zwei Läufe, eine Druckfolge.** Dieselbe Aufzeichnung läuft zusätzlich gegen
 * einen Boss OHNE Widerstände. Beide Zahlen stehen in der Datei, und der
 * Rust-Prüffall verlangt, dass sie sich unterscheiden: Ohne diesen Vergleich
 * wäre die Bitgleichheit auch dann grün, wenn der Widerstand auf BEIDEN Seiten
 * nicht ankommt. Genau das war der Zustand vor diesem Issue.
 */
export function resistancePayload(): Record<string, unknown> {
  const recording = recordings().get(RESISTANCE_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${RESISTANCE_NAME}`);
  const boss = bossById('test-resistant');
  if (boss === undefined) throw new Error('Der Boss test-resistant fehlt.');
  const ohne = bossById('test-metronome');
  if (ohne === undefined) throw new Error('Der Boss test-metronome fehlt.');

  const result = replayRecording(recording, { checksumPerTick: true, boss });
  // Derselbe Druckverlauf gegen den Boss ohne Widerstand — nur für den
  // Vergleich, nicht für die Prüfsummen.
  const neutral = replayRecording(recording, { boss: ohne });

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      // Die Zahl steht in der Datei, damit der Rust-Prüffall sie nicht
      // abschreiben muss: Beide Seiten lesen dieselbe Quelle.
      resistances: boss.resistances,
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      // Derselbe Lauf ohne Widerstand. Der Unterschied IST die Aussage.
      damageWithoutResistance: neutral.damage,
    },
    checksums: result.checksums,
  };
}

export const RESISTANCE_MIXED_NAME = 'resistances-mixed';

/**
 * Ein Kampf mit Zaubern aus DREI Schulen gegen einen Boss, der eine davon
 * abhält (Issue #328, AC7).
 *
 * **Der sechste Paritäts-Fall, und er misst als einziger die Zuordnung.** Seit
 * der Widerstand am ZAUBER hängt und nicht am Kampf, belegt eine Aufzeichnung
 * mit lauter Kalzinations-Zaubern nur den Fall, in dem alle dieselbe Schule
 * haben — also genau den, in dem die Umstellung nichts ändert.
 *
 * Hier machen die Zauber verschieden viel Schaden, und beide Seiten müssen
 * dieselbe **Zuordnung** treffen. Eine Umsetzung, die den Widerstand auf alles
 * anwendet, käme auf eine andere Summe; eine, die ihn auf nichts anwendet,
 * ebenfalls. Nur die richtige trifft die Zahl in dieser Datei.
 *
 * Die Schadens-Aufteilung je Zauber steht mit in der Datei: Weichen die Seiten
 * ab, sagt sie **welcher** Zauber falsch gerechnet wurde, statt nur dass die
 * Summe nicht stimmt.
 */
export function resistanceMixedPayload(): Record<string, unknown> {
  const recording = recordings().get(RESISTANCE_MIXED_NAME);
  if (recording === undefined) {
    throw new Error(`Unbekannte Aufzeichnung: ${RESISTANCE_MIXED_NAME}`);
  }
  const boss = bossById('test-resistant');
  if (boss === undefined) throw new Error('Der Boss test-resistant fehlt.');
  const bindings = [
    { slot: 0, spellId: 'bind' },
    { slot: 1, spellId: 'canker' },
    { slot: 2, spellId: 'kindle' },
    // Der einzige Zauber mit einem Effekt über Zeit. Er trägt den Fall, in dem
    // ein Brand unter Widerstand tickt, WÄHREND ein Zauber einer anderen
    // Schule dazwischen gewirkt wird.
    { slot: 3, spellId: 'smoulder' },
  ];

  const result = replayRecording(recording, { checksumPerTick: true, boss, bindings });

  // Der zweite Durchgang für den Schaden je Zauber. Er läuft mit denselben
  // Eingaben und liefert deshalb denselben Kampf.
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of bindings) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
  }
  const jeZauber: Record<string, number> = {};
  for (const binding of bindings) {
    jeZauber[binding.spellId] = combat.damageOf(combat.indexOf(binding.spellId));
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings,
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: { resistances: boss.resistances },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
    },
    damageBySpell: jeZauber,
    checksums: result.checksums,
  };
}

export const CAST_TIME_NAME = 'cast-time';

/**
 * Die WIRKZEIT des Spielers, Takt für Takt (Issue #304, AC5).
 *
 * **Bis zu diesem Issue hat keine Vergleichsdatei eine Spieler-Wirkzeit
 * gefahren** — es gab keine: Alle fünfzehn Zauber standen auf `castTimeMs: 0`,
 * und `Combat.cast` fragte das Feld an keiner Stelle. Der ganze Zweig war
 * damit unbelegt, und ein Auseinanderlaufen wäre unsichtbar geblieben, weil
 * kein Fall es zeigen konnte. Dieselbe Lage wie bei `runaway` vor #256.
 *
 * **Die Belegung ist eine eigene** und nicht die aus Schritt 1: Der Prüf-Zauber
 * steht in keiner Leiste des Spiels. Sie steht in der Datei, und der
 * Rust-Prüffall liest sie von dort — beide Seiten haben damit dieselbe Quelle
 * statt zweier Abschriften.
 *
 * Zwei Spuren neben den Prüfsummen. Beide gehen ohnehin in die Prüfsumme ein,
 * aber eine abweichende Prüfsumme sagt nur, **dass** etwas anders ist; die
 * Spuren sagen **was**: welcher Zauber gerade gewirkt wird und wann er fällig
 * ist. Wer sie liest, sieht den Abbruch bei 9000 ms als Sprung auf -1.
 */
export function castTimePayload(): Record<string, unknown> {
  const recording = recordings().get(CAST_TIME_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${CAST_TIME_NAME}`);
  const boss = bossById('test-caster');
  if (boss === undefined) throw new Error('Der Boss test-caster fehlt.');
  const bindings = [
    { slot: 0, spellId: 'kindle' },
    { slot: 1, spellId: 'test-cast' },
  ];

  const result = replayRecording(recording, { checksumPerTick: true, boss, bindings });

  // Der zweite Durchgang für die Spuren. Er läuft mit denselben Eingaben und
  // liefert deshalb denselben Kampf — dieselbe Bauart wie in
  // `runawayProgressPayload`.
  const casting: number[] = [];
  const endsAt: number[] = [];
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) {
      casting.push(combat.playerCasting);
      endsAt.push(combat.playerCasting >= 0 ? combat.playerCastEndsAtMs : -1);
      break;
    }
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of bindings) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    casting.push(combat.playerCasting);
    endsAt.push(combat.playerCasting >= 0 ? combat.playerCastEndsAtMs : -1);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings,
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      castsInterrupted: combat.castsInterrupted,
      // Seit der Umkehrung vom 2026-08-05 gibt es zwei Abbruch-Arten
      // (GAME-DESIGN 7.1). Beide Zahlen stehen hier, weil eine Prüfsumme sie
      // nicht führt: Zwei Umsetzungen können bei gleicher Summe verschieden
      // oft abgebrochen haben, solange sich die Unterschiede aufheben.
      castsCancelled: combat.castsCancelled,
      testCastCasts: combat.castCount(combat.indexOf('test-cast')),
    },
    checksums: result.checksums,
    playerCasting: casting,
    playerCastEndsAt: endsAt,
  };
}

export const TRIAL_BLOCKS_NAME = 'trial-blocks';

/**
 * Die letzten beiden Bausteine, Takt für Takt (Issue #240, AC6).
 *
 * **Beide in EINEM Lauf**: Die Schwächung greift bei 8 s und wird von QUENCH
 * entfernt, die Wirkzeit der Zeitleiste schrumpft mit jedem der sieben Zyklen.
 * Sie getrennt zu prüfen würde nicht zeigen, dass sie einander nicht ins
 * Gehege kommen.
 *
 * Die zwei Reihen gehen mit, nicht nur das Ergebnis: Weichen die Seiten ab,
 * sagt der erste unterschiedliche Eintrag, **welcher Baustein** es war.
 */
export function trialBlocksPayload(): Record<string, unknown> {
  const recording = recordings().get(TRIAL_BLOCKS_NAME);
  if (recording === undefined) throw new Error(`Unbekannte Aufzeichnung: ${TRIAL_BLOCKS_NAME}`);
  const boss = bossById('test-crucible-trial');
  if (boss === undefined) throw new Error('Der Boss test-crucible-trial fehlt.');

  const result = replayRecording(recording, { checksumPerTick: true, boss });

  const geschwaecht: number[] = [];
  const landet: number[] = [];
  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  let pressIndex = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of STEP1_BINDINGS) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    geschwaecht.push(combat.afflicted ? 1 : 0);
    landet.push(combat.casting >= 0 ? combat.castLandsAtMs : -1);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: boss.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      escalation: boss.escalation,
      affliction: boss.affliction,
      timelineLoopMs: boss.timelineLoopMs,
    },
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      afflictionsCleared: combat.afflictionsCleared,
      cycle: combat.cycle,
      diedAtMs: combat.diedAtMs,
    },
    checksums: result.checksums,
    afflicted: geschwaecht,
    castLandsAtMs: landet,
  };
}

/**
 * Eingehender Schaden, Spielertod und Unterbrechen (Issue #110).
 *
 * **Zwei Läufe gegen denselben Boss, die sich in genau einer Sache
 * unterscheiden:** Im einen wirkt der Spieler QUENCH, im anderen nicht. Alles
 * andere ist gleich — derselbe Startwert, dieselbe Rotation, dieselben
 * Zeitpunkte.
 *
 * Das Ergebnis trennt sich trotzdem vollständig:
 *
 * | | ohne QUENCH | mit QUENCH |
 * | --- | --- | --- |
 * | Boss-Zauber durchgekommen | 3 | 0 |
 * | erlittener Schaden | 5200 | 400 |
 * | Spieler | **tot** | lebt |
 *
 * Damit tragen zwei Läufe drei Acceptance Criteria: AC2 (der Tod beendet den
 * Versuch), AC3 (unterbrochen richtet nichts an, nicht unterbrochen den vollen
 * Schaden) und AC4 (die Wartezeit) — und zwar so, dass **eine** Ursache das
 * ganze Ergebnis erklärt. Ein Prüffall, in dem sich zwei Dinge gleichzeitig
 * ändern, könnte das nicht.
 */
export function playerDamagePayload(): Record<string, unknown> {
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('Der Boss test-onslaught fehlt.');

  return {
    schema: SCHEMA,
    name: PLAYER_DAMAGE_NAME,
    bossId: boss.id,
    tickMs: TICK_MS,
    bindings: STEP1_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    player: { vessel: PLAYER.vessel, wardReduction: PLAYER.wardReduction },
    boss: {
      hp: boss.hp,
      overpressureMs: boss.overpressureMs,
      timeline: boss.timeline.map((e) => ({
        atMs: e.atMs,
        mechanic: e.mechanic,
        name: e.name,
        damage: e.damage,
        castTimeMs: e.castTimeMs,
      })),
    },
    runs: [interruptRun(boss, false), interruptRun(boss, true)],
  };
}

/**
 * Ein Durchgang gegen den Boss, wahlweise mit oder ohne Unterbrechen.
 *
 * Die Rotation ist bewusst schlicht: KINDLE, sooft es geht — und QUENCH, sobald
 * ein Boss-Zauber läuft. Sie soll nicht gut sein, sondern **den Unterschied
 * zeigen**.
 */
function interruptRun(boss: BossDef, mitQuench: boolean): Record<string, unknown> {
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = combat.indexOf('kindle');
  const quench = combat.indexOf('quench');
  const ticks = 3000; // 30 s

  // **Die Leiste steht eine Ebene höher** (Issue #405): Dieser Lauf gibt keine
  // eigene zurück, der Eltern-Payload trägt STEP1_BINDINGS für alle seine
  // Läufe. Der Platz kommt trotzdem aus der Bindung und nicht aus einer Zahl.
  const slots = slotMap(combat, STEP1_BINDINGS);
  const presses: { tick: number; slot: number }[] = [];
  const checksums: number[] = [];
  const playerHp: number[] = [];
  let sequence = 0;

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    // **Der Spielertod beendet den Versuch** (GAME-DESIGN A.3) — keine Strafe,
    // die Auswertung erscheint wie bei jedem anderen Ende. Die Prüfsumme dieses
    // Takts wird noch geschrieben, damit beide Seiten denselben letzten Eintrag
    // haben.
    if (combat.playerDead) {
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      checksums.push(hash);
      playerHp.push(combat.playerHp);
      break;
    }

    // QUENCH zuerst: Er löst keine Wartezeit aus, also kostet er den
    // Schadenszauber dieses Takts nichts. Genau das ist der Sinn der Ausnahme
    // (GAME-DESIGN 6.6) — und hier wird sie zum ersten Mal wirklich gebraucht.
    if (mitQuench && combat.casting >= 0 && combat.isReady(quench, timeMs)) {
      if (combat.cast(quench, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(quench), combat) });
    }
    if (combat.isReady(kindle, timeMs) && combat.gcd.isReady(timeMs)) {
      if (combat.cast(kindle, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(kindle), combat) });
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
    playerHp.push(combat.playerHp);
  }

  return {
    withInterrupt: mitQuench,
    ticks,
    presses,
    expected: {
      playerHp: combat.playerHp,
      damageTaken: combat.damageTaken,
      diedAtMs: combat.diedAtMs,
      interruptsLanded: combat.interruptsLanded,
      interruptsMissed: combat.interruptsMissed,
      castsQuench: combat.castCount(quench),
      castsKindle: combat.castCount(kindle),
      sequenceChecksum: sequence,
    },
    checksums,
    playerHpPerTick: playerHp,
  };
}

/**
 * Die drei Wege der Koagulation, gegen **denselben** Boss (Issue #79).
 *
 * **Vier Läufe, die sich in genau einer Sache unterscheiden:** welchen
 * Verteidigungszauber der Spieler wirkt — keinen, CRUST, SALVE oder TEMPER.
 * Derselbe Boss, dasselbe Schadensprofil, dieselbe Angriffsrotation,
 * derselbe Startwert.
 *
 * **Das ist der Kern von AC1.** Drei Wege, die alle „irgendwie überleben
 * helfen", wären kein Nachweis; drei Wege, die bei gleichem eingehenden
 * Schaden zu drei verschiedenen Endzuständen führen, sind einer. Ein Lauf je
 * Weg genügt dafür nicht — erst der Vergleich zeigt, dass sie sich wirklich
 * unterscheiden und nicht bloß alle dasselbe tun.
 *
 * Der Lauf ohne Verteidigung ist die Meßlatte: Er zeigt, dass der Boss
 * überhaupt gefährlich ist. Ohne ihn könnten alle drei Wege wirkungslos sein,
 * ohne dass es auffällt.
 */
export function coagulationPayload(): Record<string, unknown> {
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('Der Boss test-onslaught fehlt.');

  return {
    schema: SCHEMA,
    name: COAGULATION_NAME,
    bossId: boss.id,
    tickMs: TICK_MS,
    player: { vessel: PLAYER.vessel, wardReduction: PLAYER.wardReduction },
    runs: ['none', 'crust', 'salve', 'temper'].map((weg) => defenceRun(boss, weg)),
  };
}
/**
 * Die Leiste der Verteidigungs-Aufzeichnung (Issue #405).
 *
 * **Sie hängt vom Lauf ab** — anders als die übrigen vier: Der zweite Platz
 * trägt den Zauber, der gerade geprüft wird, und bei `'none'` bleibt er leer.
 * Deshalb eine Funktion statt einer Konstante; die Bindung entsteht trotzdem
 * VOR den Drücken und nicht erst im `return`.
 */
function defenceBindings(weg: string): { slot: number; spellId: string }[] {
  const leiste = [{ slot: 0, spellId: 'kindle' }];
  if (weg !== 'none') leiste.push({ slot: 1, spellId: weg });
  return leiste;
}

/**
 * Ein Durchgang mit einem der drei Wege — oder ohne.
 *
 * Die Angriffsrotation ist in allen vier Läufen dieselbe: KINDLE, sooft es
 * geht. Der Verteidigungszauber kommt dazu, sobald er bereit ist. Beides löst
 * den vollen GCD aus (GAME-DESIGN 6.6), und genau das ist die Abwägung:
 * **Überleben kostet Schaden und Zeit.**
 */
function defenceRun(boss: BossDef, weg: string): Record<string, unknown> {
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = combat.indexOf('kindle');
  const defence = weg === 'none' ? alsSpellIndex(-1) : combat.indexOf(weg);
  const bindings = defenceBindings(weg);
  const slots = slotMap(combat, bindings);
  const ticks = 3000; // 30 s

  const checksums: number[] = [];
  const playerHp: number[] = [];
  const shield: number[] = [];
  const presses: { tick: number; slot: number }[] = [];
  let sequence = 0;

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (combat.playerDead) {
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      checksums.push(hash);
      playerHp.push(combat.playerHp);
      shield.push(combat.shield);
      break;
    }

    // Der Verteidigungszauber hat Vorrang, wenn er bereit ist — sonst käme er
    // nie zum Zug, weil KINDLE jeden GCD belegt.
    if (defence >= 0 && combat.isReady(defence, timeMs)) {
      if (combat.cast(defence, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, defence, combat) });
    } else if (combat.isReady(kindle, timeMs)) {
      if (combat.cast(kindle, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, kindle, combat) });
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
    playerHp.push(combat.playerHp);
    shield.push(combat.shield);
  }

  return {
    way: weg,
    ticks,
    // Die Drücke stehen in der Datei, damit der Simulator dieselbe Runde
    // nachspielt statt die Regel nachzubauen. Eine nachgebaute Regel, die
    // minimal abweicht, ergäbe eine Abweichung, die wie ein Rechenfehler
    // aussieht.
    bindings: [
      { slot: 0, spellId: 'kindle' },
      ...(weg === 'none' ? [] : [{ slot: 1, spellId: weg }]),
    ],
    presses,
    expected: {
      playerHp: combat.playerHp,
      damageTaken: combat.damageTaken,
      diedAtMs: combat.diedAtMs,
      damageDealt: combat.damageDealt,
      shieldAbsorbed: combat.shieldAbsorbed,
      healed: combat.healed,
      converted: combat.converted,
      castsDefence: defence >= 0 ? combat.castCount(defence) : 0,
      castsKindle: combat.castCount(kindle),
      sequenceChecksum: sequence,
    },
    checksums,
    playerHpPerTick: playerHp,
    shieldPerTick: shield,
  };
}

/**
 * Die Fixierung greift in den Kampfverlauf ein (Issue #100).
 *
 * **Zwei Läufe gegen denselben Boss, die sich in genau einer Sache
 * unterscheiden:** ob der Spieler seine Kontroll-Punkte ausgibt. Derselbe
 * Boss, derselbe Startwert, dieselbe Angriffsrotation.
 *
 * Der Prüffall trägt drei Nachweise, und zwei davon zeigen, dass sich etwas
 * **nicht** ändert:
 *
 * | Nachweis | Was er zeigt |
 * | --- | --- |
 * | AC1 | Der Boss verhält sich messbar anders — nicht nur ein Zähler läuft hoch |
 * | STAY | Der **zweite** Schlag rückt NICHT mit, nur der erste |
 * | ANCHOR | Läuft die Fixierung während einer höheren Phase aus, **bleibt** die Phase |
 *
 * Ohne die letzten beiden wären die zugehörigen Zeilen nur zufällig richtig:
 * Ein Lauf mit nur einem Schlag bestünde auch die verworfene STAY-Fassung, und
 * eine Phase, die nie zurückfallen könnte, belegt `check_phase` nicht.
 */
export function fixationPayload(): Record<string, unknown> {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('Der Boss test-warden fehlt.');

  return {
    schema: SCHEMA,
    name: FIXATION_NAME,
    bossId: boss.id,
    tickMs: TICK_MS,
    boss: {
      hp: boss.hp,
      phases: boss.phases.map((f) => ({ atHpPercent: f.atHpPercent, name: f.name })),
      timeline: boss.timeline.map((e) => ({
        atMs: e.atMs,
        mechanic: e.mechanic,
        name: e.name,
        damage: e.damage,
        castTimeMs: e.castTimeMs,
      })),
    },
    runs: [false, true].map((mitAusgabe) => controlRun(boss, mitAusgabe)),
  };
}
/**
 * Die Leiste der Kontroll-Aufzeichnung (Issue #405).
 *
 * Als Konstante VOR der Funktion, damit `slotMap` sie sehen kann — im
 * `return`-Objekt entsteht sie zu spät für die Drücke.
 */
const CONTROL_BINDINGS = [
  { slot: 0, spellId: 'bind' },
  { slot: 1, spellId: 'stay' },
  { slot: 2, spellId: 'anchor' },
  { slot: 3, spellId: 'stall' },
] as const;

/**
 * Ein Durchgang mit oder ohne Ausgabe der Kontroll-Punkte.
 *
 * BIND baut auf, sooft es geht; STAY und ANCHOR geben aus, sobald sie bereit
 * sind und die Punkte reichen. Ohne Ausgabe wirkt der Spieler nur BIND — die
 * Punkte sammeln sich und bleiben liegen.
 */
function controlRun(boss: BossDef, mitAusgabe: boolean): Record<string, unknown> {
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = combat.indexOf('bind');
  const stay = combat.indexOf('stay');
  const anchor = combat.indexOf('anchor');
  const stall = combat.indexOf('stall');
  const ticks = 3000; // 30 s

  const checksums: number[] = [];
  const control: number[] = [];
  const phase: number[] = [];
  const timelineIndex: number[] = [];
  const slots = slotMap(combat, CONTROL_BINDINGS);
  const presses: { tick: number; slot: number }[] = [];
  let sequence = 0;
  let stayGewirkt = false;

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) break;

    // **STAY genau einmal, und zwar VOR dem ersten Schlag** (Issue #100).
    // Sooft-wie-möglich wäre bequemer, würde den Nachweis aber wertlos machen:
    // Rückt der zweite Schlag nicht mit, weil die Verzögerung gerade
    // verbraucht war statt weil die Regel greift, zeigt der Fall nichts. Der
    // erste Schlag liegt bei 12 s — eine Ausgabe bei ~10 s trifft ihn, und der
    // zweite bei 19 s muss dann unverändert stehen.
    // Ein FENSTER, kein Zeitpunkt: Bei Takt 1000 läuft womöglich gerade die
    // Wartezeit von BIND, und ein Druck, der daran scheitert, verschöbe
    // nichts — der Nachweis wäre still weg.
    const imFenster = !stayGewirkt && tick >= 1000 && tick < 1150;
    if (mitAusgabe && imFenster && combat.control >= 3 && combat.isReady(stay, timeMs)) {
      if (combat.cast(stay, timeMs).result === 0) {
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(stay), combat) });
        stayGewirkt = true;
      }
    } else if (mitAusgabe && combat.isReady(anchor, timeMs) && combat.control >= 6) {
      if (combat.cast(anchor, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(anchor), combat) });
    } else if (mitAusgabe && combat.isReady(stall, timeMs) && combat.control >= 10) {
      // **STALL vor BIND**, wenn Punkte da sind: Er kostet 4 und verlangsamt
      // einen Boss-Zauber. Steht er hinter dem Aufbauer, kommt er nie zum Zug —
      // BIND ist oefter bereit und verbraucht jede Gelegenheit.
      if (combat.cast(stall, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(stall), combat) });
    } else if (combat.isReady(bind, timeMs)) {
      if (combat.cast(bind, timeMs).result === 0)
        presses.push({ tick, slot: slotFor(slots, alsSpellIndex(bind), combat) });
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
    control.push(combat.control);
    phase.push(combat.bossPhase);
    timelineIndex.push(combat.timelineIndex);
  }

  return {
    withSpending: mitAusgabe,
    ticks,
    bindings: CONTROL_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses,
    expected: {
      control: combat.control,
      controlSpent: combat.controlSpent,
      controlFailed: combat.controlFailed,
      phaseHoldPercent: combat.phaseHoldPercent,
      finalPhase: combat.bossPhase,
      damageTaken: combat.damageTaken,
      damageDealt: combat.damageDealt,
      sequenceChecksum: sequence,
    },
    checksums,
    controlPerTick: control,
    phasePerTick: phase,
    timelineIndexPerTick: timelineIndex,
  };
}

/**
 * Was das Spiel ins Protokoll schreibt — je Art gezählt (Issue #151).
 *
 * **Die ZAHL ist der Nachweis, nicht das Vorkommen.** Dass eine Art überhaupt
 * auftaucht, sagt wenig; die Zahl sagt, ob sie an denselben Stellen
 * geschrieben wird wie im Simulator. Ein Spiel, das den Schlag einmal und der
 * Simulator dreimal protokolliert, sähe plausibel aus und wäre falsch.
 *
 * Das ist hier keine theoretische Sorge: Im Simulator läuft jeder ausgeteilte
 * Schaden durch `deal()`, im Spiel gibt es diese eine Stelle nicht. Die
 * Schreibpunkte liegen verschieden — nur ihre Zahl kann zeigen, dass sie
 * trotzdem dasselbe erfassen.
 */
/**
 * Die Leiste der Protokoll-Aufzeichnung (Issue #405).
 *
 * **Sie steht hier als Konstante und nicht erst im `return`.** Genau daran lag
 * es, dass die Plätze weiter unten von Hand geschrieben waren: Zum Zeitpunkt
 * des Drucks gab es die Leiste noch gar nicht als Wert, also blieb nur die
 * Zahl — und niemand prüfte sie je gegen die Bindung.
 */
const LOG_PARITY_BINDINGS = [
  { slot: 0, spellId: 'kindle' },
  { slot: 1, spellId: 'crust' },
  { slot: 2, spellId: 'quench' },
  { slot: 3, spellId: 'salve' },
] as const;

export function logParityPayload(): Record<string, unknown> {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('Der Boss test-warden fehlt.');

  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const log = new CombatLog();
  combat.log = log;
  const kindle = combat.indexOf('kindle');
  const crust = combat.indexOf('crust');
  const salve = combat.indexOf('salve');
  const quench = combat.indexOf('quench');
  const ticks = 2500; // 25 s

  const slots = slotMap(combat, LOG_PARITY_BINDINGS);
  const presses: { tick: number; slot: number }[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) break;

    // Eine Rotation, die JEDE der fünf Arten auslöst: CRUST baut die
    // Schutzschicht auf (shieldAbsorb), QUENCH unterbricht den Boss-Zauber
    // (interruptLanded), KINDLE füllt die Lücken. Der Boss liefert
    // bossTimelineFired und playerDamage von selbst.
    // CRUST kommt **nur einmal**, ganz am Anfang. Wirkte er sooft es geht,
    // finge er jeden Schlag ab, der Spieler bliebe auf voller Anzeige, und
    // SALVE heilte nichts — `healed` stünde auf null, und der Nachweis für die
    // Heilung wäre still weg. Eine Art mit null Einträgen belegt nichts.
    let gewaehlt = -1;
    if (combat.casting >= 0 && combat.isReady(quench, timeMs)) gewaehlt = quench;
    else if (tick < 100 && combat.isReady(crust, timeMs)) gewaehlt = crust;
    else if (combat.playerHp < PLAYER.vessel && combat.isReady(salve, timeMs)) gewaehlt = salve;
    else if (combat.isReady(kindle, timeMs)) gewaehlt = kindle;
    if (gewaehlt < 0) continue;

    if (combat.cast(alsSpellIndex(gewaehlt), timeMs).result === 0) {
      // **Der Platz kommt aus der Bindung, nicht aus einem Vergleich** (#405).
      // `slotFor` wirft, wenn der Zauber auf keinem Platz liegt — die Zahl 3
      // als stiller Rückfall für „alles andere" ist damit weg.
      presses.push({ tick, slot: slotFor(slots, alsSpellIndex(gewaehlt), combat) });
    }
  }

  // Je Art zählen — das ist der Vergleichswert.
  const counts: Record<string, number> = {};
  for (const kind of EVENT_KINDS_WRITTEN_BY_GAME) {
    counts[EVENT_NAMES[kind] ?? String(kind)] = log.countOf(kind);
  }

  return {
    schema: SCHEMA,
    name: LOG_PARITY_NAME,
    bossId: boss.id,
    ticks,
    tickMs: TICK_MS,
    bindings: LOG_PARITY_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses,
    /** Anzahl je Protokoll-Art. Der Simulator muss dieselben Zahlen ergeben. */
    counts,
    expected: {
      damageDealt: combat.damageDealt,
      damageTaken: combat.damageTaken,
      shieldAbsorbed: combat.shieldAbsorbed,
      interruptsLanded: combat.interruptsLanded,
      healed: combat.healed,
      timelineIndex: combat.timelineIndex,
      logEntries: log.length,
    },
  };
}
/** Die Leiste der Sublimations-Aufzeichnung — VOR der Funktion (Issue #405). */
const SUBLIMATION_BINDINGS = [
  { slot: 0, spellId: 'kindle' },
  { slot: 1, spellId: 'vapour' },
  { slot: 2, spellId: 'smoulder' },
  { slot: 3, spellId: 'pyre' },
] as const;

/**
 * Ein Durchgang gegen einen Boss mit **wiederholter** Zeitleiste (#174).
 *
 * # Warum mindestens zwei Zyklen
 *
 * Eine Wiederholungs-Regel, die nur eine Seite kennt, bricht die Parität
 * **erst nach dem ersten Zyklus**. Ein Lauf, der einen Zyklus abdeckt, liefe
 * grün — und die Abweichung fiele erst auf, wenn jemand länger übt.
 *
 * Der Prüf-Boss `test-metronome` hat eine Zykluslänge von 6 s; der Lauf
 * umfasst 30 s, also fünf Zyklen. Die Prüfung darauf steht in
 * `sim/tests/parity_timeline_loop.rs` und ist hart: Weniger als zwei Zyklen
 * lässt der Fall nicht durchgehen.
 */
/**
 * Ein Durchlauf, in dem **wirklich verkürzt wird** (Issue #104).
 *
 * # Warum es diesen Payload gibt
 *
 * AC5 von #78 lautete „die Mechanik rechnet auf beiden Seiten gleich" und war
 * damals nicht baubar: Kein ausgelieferter Zauber trug
 * `cooldownReductionMs`, ein Prüffall hätte zwei Nullen verglichen. Worker-1
 * hat das gemeldet, statt den AC trivial grün zu setzen — daraus wurde dieses
 * Issue.
 *
 * Seit #201 trägt VAPOUR die Verkürzung. Der Lauf hier wirkt ihn wiederholt
 * zwischen KINDLE-Würfen: Jede Wirkung nimmt allen laufenden Abklingzeiten
 * denselben festen Betrag (GAME-DESIGN 4.1 — ein Betrag, kein Anteil, damit
 * der Spieler abzählen kann).
 *
 * # Was die Prüfsumme je Takt hier leistet
 *
 * Eine Verkürzung verschiebt **Zeitpunkte**, nicht Beträge. Ein Vergleich nur
 * am Ende sähe denselben Gesamtschaden auch dann, wenn beide Seiten die
 * Zauber zu verschiedenen Zeiten wirken — solange am Schluss gleich viele
 * durchgelaufen sind. Erst die Summe je Takt hält fest, dass sie es
 * gleichzeitig tun.
 */
export function sublimationPayload(): Record<string, unknown> {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('Der Boss test-warden fehlt.');

  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = combat.indexOf('kindle');
  const vapour = combat.indexOf('vapour');
  const smoulder = combat.indexOf('smoulder');
  const pyre = combat.indexOf('pyre');
  const ticks = 3000; // 30 s

  const slots = slotMap(combat, SUBLIMATION_BINDINGS);
  const presses: { tick: number; slot: number }[] = [];
  const checksums: number[] = [];
  const pyreReadyAt: number[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) {
      checksums.push(combatChecksum(combat, timeMs));
      pyreReadyAt.push(combat.cooldownRemainingMs(pyre, timeMs));
      break;
    }
    // VAPOUR, sobald er bereit ist — sonst SMOULDER für die Brand-Wirkung,
    // sonst KINDLE. So laufen beim Verkürzen wirklich Abklingzeiten, statt
    // dass die Wirkung ins Leere geht.
    let gewaehlt = -1;
    if (combat.isReady(vapour, timeMs)) gewaehlt = vapour;
    else if (combat.dotRemainingMs(smoulder, timeMs) < 1500 && combat.isReady(smoulder, timeMs)) {
      gewaehlt = smoulder;
    } else if (combat.isReady(pyre, timeMs) && combat.resource >= 15) gewaehlt = pyre;
    else if (combat.isReady(kindle, timeMs)) gewaehlt = kindle;

    if (gewaehlt >= 0 && combat.cast(alsSpellIndex(gewaehlt), timeMs).result === 0) {
      // **Der Platz aus der Bindung, nicht aus einer Vergleichskette** (#405).
      // Der alte Ausdruck endete auf `: 3` — jeder nicht genannte Zauber bekam
      // stillschweigend PYREs Platz, ohne dass das je jemand entschieden hätte.
      presses.push({ tick, slot: slotFor(slots, alsSpellIndex(gewaehlt), combat) });
    }
    checksums.push(combatChecksum(combat, timeMs));
    // **Die Restzeit von PYRE**, nicht von KINDLE. Der erste Entwurf maß
    // KINDLE — der hat 1500 ms Abklingzeit und ist beim VAPOUR-Wurf praktisch
    // immer bereit, die Verkürzung um 1000 ms traf also eine Restzeit von 10 ms
    // und wurde auf 0 gekappt. Drei Prüffälle waren grün und belegten, dass
    // beide Seiten dasselbe Nichts rechnen.
    //
    // PYRE hat 8000 ms. Beim VAPOUR-Wurf läuft damit eine echte Restzeit.
    pyreReadyAt.push(combat.cooldownRemainingMs(pyre, timeMs));
  }

  return {
    schema: SCHEMA,
    name: SUBLIMATION_NAME,
    bossId: boss.id,
    ticks,
    tickMs: TICK_MS,
    bindings: SUBLIMATION_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses,
    checksums,
    /** Restzeit von PYRE je Takt — die Größe, die VAPOUR wirklich bewegt. */
    pyreReadyAt,
    expected: {
      damageDealt: combat.damageDealt,
      casts:
        combat.castCount(kindle) +
        combat.castCount(vapour) +
        combat.castCount(smoulder) +
        combat.castCount(pyre),
      vapourCasts: combat.castCount(vapour),
      kindleCasts: combat.castCount(kindle),
    },
  };
}
/** Die Leiste der Zeitleisten-Aufzeichnung — VOR der Funktion (Issue #405). */
const TIMELINE_LOOP_BINDINGS = [
  { slot: 0, spellId: 'kindle' },
  { slot: 1, spellId: 'quench' },
] as const;

export function timelineLoopPayload(): Record<string, unknown> {
  const boss = bossById('test-metronome');
  if (boss === undefined) throw new Error('Der Boss test-metronome fehlt.');
  if (boss.timelineLoopMs === null) {
    throw new Error('test-metronome hat keine timelineLoopMs — der Fall prüfte dann nichts.');
  }

  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = combat.indexOf('kindle');
  const quench = combat.indexOf('quench');
  const ticks = 3000; // 30 s = fünf Zyklen

  const slots = slotMap(combat, TIMELINE_LOOP_BINDINGS);
  const presses: { tick: number; slot: number }[] = [];
  const checksums: number[] = [];
  const timelineIndex: number[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.playerDead) {
      checksums.push(combatChecksum(combat, timeMs));
      timelineIndex.push(combat.timelineIndex);
      break;
    }
    // QUENCH, wenn der Boss zaubert — sonst KINDLE. Das unterbricht in
    // späteren Zyklen ebenso wie im ersten und deckt damit auch den
    // Zeitpunkt des Treffers ab, nicht nur den des Beginns.
    const gewaehlt = combat.casting >= 0 && combat.isReady(quench, timeMs) ? quench : kindle;
    if (combat.cast(alsSpellIndex(gewaehlt), timeMs).result === 0) {
      // **Kein Vergleich mehr** (#405). Diese Zeile war die versteckte von elf:
      // Eine Suche nach `slot: <Ziffer>` findet sie nicht, die Zuordnung steht
      // trotzdem von Hand da.
      presses.push({ tick, slot: slotFor(slots, alsSpellIndex(gewaehlt), combat) });
    }
    checksums.push(combatChecksum(combat, timeMs));
    timelineIndex.push(combat.timelineIndex);
  }

  return {
    schema: SCHEMA,
    name: TIMELINE_LOOP_NAME,
    bossId: boss.id,
    ticks,
    tickMs: TICK_MS,
    timelineLoopMs: boss.timelineLoopMs,
    timelineLength: boss.timeline.length,
    bindings: TIMELINE_LOOP_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses,
    checksums,
    /** Der Fortschrittszähler je Takt — er läuft über die Länge der Liste hinaus. */
    timelineIndex,
    expected: {
      damageDealt: combat.damageDealt,
      damageTaken: combat.damageTaken,
      interruptsLanded: combat.interruptsLanded,
      interruptsMissed: combat.interruptsMissed,
      finalTimelineIndex: combat.timelineIndex,
      finalPlayerHp: combat.playerHp,
    },
  };
}

/**
 * Spielt die Aufzeichnung durch und zählt, **was** dabei schiefging.
 *
 * Nötig, weil `replayRecording` nur „angenommen" und „abgelehnt" liefert. Damit
 * ließe sich AC1 nicht belegen: Eine Runde mit hundert abgelehnten Drücken
 * derselben Sorte sähe genauso aus wie eine mit allen vier Fehlertypen.
 */
function analyzeMisplays(recording: Recording): {
  reasons: Record<string, number>;
  lostDotTicks: number;
  idleTicks: number;
} {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const slotToSpell = new Map<number, number>();
  for (const binding of STEP1_BINDINGS)
    slotToSpell.set(binding.slot, combat.indexOf(binding.spellId));

  const reasons: Record<string, number> = {
    ok: 0,
    unknown: 0,
    onCooldown: 0,
    onGcd: 0,
    fightOver: 0,
  };
  const namen = ['ok', 'unknown', 'onCooldown', 'onGcd', 'fightOver'];
  let pressIndex = 0;
  let idleTicks = 0;

  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;

    let gedrueckt = false;
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      gedrueckt = true;
      const spell = slotToSpell.get(press.slot);
      if (spell === undefined) {
        reasons['unknown'] = (reasons['unknown'] ?? 0) + 1;
        continue;
      }
      const outcome = combat.cast(alsSpellIndex(spell), timeMs);
      const name = namen[outcome.result] ?? 'unknown';
      reasons[name] = (reasons[name] ?? 0) + 1;
    }

    // Leerlauf: Der GCD war frei, und es wurde trotzdem nicht gedrückt.
    if (!gedrueckt && combat.gcd.isReady(timeMs)) idleTicks++;
  }

  let lostDotTicks = 0;
  for (let i = 0; i < combat.spells.length; i++) lostDotTicks += combat.lostDotTicks(i);

  return { reasons, lostDotTicks, idleTicks };
}

/**
 * Eine feste Folge von Verstärkungen samt Faktor nach jedem Schritt (E3-5).
 *
 * **Anders als bei der Sublimation ist dieser Abgleich heute schon
 * aussagekräftig**: Die Stapel-Regel hängt an keinem Zauber und an keinen
 * Daten, die es noch nicht gibt. Sie lässt sich deshalb vollständig
 * gegenrechnen — beide Seiten müssen nach jedem Schritt denselben Faktor
 * haben, bis aufs letzte Bit.
 *
 * Verglichen wird über die **Prüfsumme** des Faktors, nicht über die Zahl
 * selbst: Ein Faktor wie 1,2 × 1,5 ist in JSON nicht verlustfrei zu
 * schreiben, seine Bit-Darstellung schon (`mixFloat`).
 */
/**
 * Ein KAMPF mit aktiver Verstärkung, Prüfsumme je Takt (Issue #464).
 *
 * # Was dieser Abgleich belegt — und was ausdrücklich nicht
 *
 * **Belegt:** Beide Seiten rechnen mit aktiver Verstärkung dasselbe. Verglichen
 * wird die Zustands-Prüfsumme; sie trägt kein eigenes Feld für die
 * Verstärkung, die Wirkung geht über `damage` ein.
 *
 * **Nicht belegt, und das ist Absicht:** Dass die Verstärkung überhaupt wirkt.
 * Das ist ein Prüffall, kein Abgleich — er steht in
 * `tests/unit/buff-in-combat.test.ts` (#420 AC4), mit zwei Läufen und einer
 * Gegenprobe.
 *
 * **Eine Paritäts-Aufzeichnung vergleicht Spiel gegen Simulator, nicht zwei
 * Läufe.** Ein erster Entwurf führte hier zusätzlich einen Lauf ohne
 * Verstärkung — der hätte doppelt geprüft, was schon geprüft ist, und dabei
 * einen zweiten Zauber ins Spiel gebracht, der die Prüfsummen aus einem
 * anderen Grund auseinanderlaufen ließ.
 *
 * # Die Falle, die beim Bauen zuschlug
 *
 * **Der Takt muss laufen.** `expireBuffs` hängt am Takt, nicht am Wirken. Ohne
 * `stepPassive` bliebe `factors.buffs` nach dem Fenster stehen, und der Lauf
 * sähe verstärkt aus, ohne es zu sein.
 */
/**
 * Die drei NEUEN Fixierungs-Zauber in einem Lauf (Issue #478).
 *
 * **Nicht zu verwechseln mit `fixationPayload`**: Der prüft die drei
 * ursprünglichen Ausgeber gegen `test-warden` — einen Boss mit Mechanik, an
 * dem sich Verlangsamen und Verzögern überhaupt zeigen lassen. Dieser hier
 * prüft die neuen Zauber gegen FIRST_CRUCIBLE, wo es nichts zu kontrollieren
 * gibt: genau die Lage, für die WEDGE gebaut wurde.
 *
 * # Wozu diese Aufzeichnung
 *
 * `coverage.test.ts` verlangt, dass jeder **ausgelieferte** Zauber in einem
 * Paritäts-Weg gewirkt wird. Ein Zauber, der nur in der Datei steht, ist
 * genau der Zustand, den dieses Projekt neunmal hatte: gebaut, geprüft, nicht
 * angeschlossen.
 *
 * # Die Reihenfolge ist die Aussage
 *
 * TETHER und BIND laden Kontroll-Punkte auf, dann greift WEDGE (wandelt sie
 * in Schaden) und später CAPSTONE (alle drei Wirkungen auf einmal). Wer die
 * Reihenfolge umdreht, misst zwei Ausgeber ohne Vorrat — beide liefen ins
 * Leere, und die Aufzeichnung wäre grün, ohne etwas zu zeigen.
 */
export function fixationSpellsPayload(): Record<string, unknown> {
  const boss = FIRST_CRUCIBLE.value;
  // **2400 Takte, nicht 500.** Ein erster Bau lief 500 und drückte CAPSTONE
  // bei Takt 2200 — der Druck wurde nie erreicht, und die Aufzeichnung maß
  // zwei Ausgeber, die nie liefen. Aufgefallen ist es an der Zusicherung
  // `controlSpent` im Datensatz selbst: Sie stand auf 0.
  const ticks = 2400;
  const combat = new Combat({ school: CALCINATION.value, boss });
  // Die Namen stehen in `presses`; `combat.indexOf` löst sie beim Wirken auf.
  // So liest die Rust-Seite dieselbe Datei, ohne Indizes zu kennen.

  const checksums: number[] = [];
  // **Plätze statt Namen in `presses`** — dasselbe Format wie jede andere
  // Aufzeichnung. `coverage.test.ts` löst jeden Druck über die `bindings`
  // auf; eine Datei mit Namen direkt im Druck fällt dort durch, und die
  // Zauber gälten als ungedeckt, obwohl sie gewirkt werden.
  const bindings = [
    { slot: 0, spellId: 'tether' },
    { slot: 1, spellId: 'bind' },
    { slot: 2, spellId: 'wedge' },
    { slot: 3, spellId: 'capstone' },
  ];
  const platzVon = new Map(bindings.map((b) => [b.spellId, b.slot]));
  const presses: { tick: number; slot: number }[] = [];
  const druecke = (tick: number, id: string): void => {
    presses.push({ tick, slot: platzVon.get(id) ?? 0 });
  };

  // Erst aufladen — sonst laufen die beiden Ausgeber ins Leere.
  for (let n = 0; n < 4; n++) druecke(n * 150, 'tether');
  for (let n = 0; n < 4; n++) druecke(600 + n * 150, 'bind');
  druecke(1300, 'wedge');
  // CAPSTONE braucht acht Punkte, also nach einer zweiten Ladephase.
  for (let n = 0; n < 5; n++) druecke(1400 + n * 150, 'bind');
  druecke(2200, 'capstone');

  let naechster = 0;
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    while (naechster < presses.length && presses[naechster]!.tick <= tick) {
      // **Nicht über eine Zwischenvariable `platz`** — der Wächter aus #405
      // sucht nach `const … = … .slot` und kann Lesen nicht von Setzen
      // unterscheiden. Hier wird der Platz aus einem fertigen Eintrag
      // GELESEN, nicht von Hand vergeben; die Form vermeidet den Fehlalarm.
      const eintrag = presses[naechster]!;
      const id = bindings.find((b) => b.slot === eintrag.slot)?.spellId ?? '';
      combat.cast(combat.indexOf(id), timeMs);
      naechster++;
    }
    checksums.push(combatChecksum(combat, timeMs) >>> 0);
  }

  return {
    schema: SCHEMA,
    ticks,
    tickMs: TICK_MS,
    bossId: boss.id,
    bindings,
    presses,
    checksums,
    expected: {
      damage: combat.damageDealt,
      /**
       * **Die Zusicherung im Datensatz:** Wären die Punkte nie ausgegeben
       * worden, liefe die Aufzeichnung über zwei Zauber, die nichts tun.
       */
      controlSpent: combat.controlSpent,
    },
  };
}

export function buffCombatPayload(): Record<string, unknown> {
  const boss = FIRST_CRUCIBLE.value;
  const ticks = 600;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const buff = combat.indexOf('test-buff');
  const kindle = combat.indexOf('kindle');

  const checksums: number[] = [];
  /** Der Faktor an den Messpunkten — für AC1, damit „aktiv" nachweisbar ist. */
  const factorAt: { tick: number; buffs: number }[] = [];

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    // **Der Takt zuerst** — hier läuft die Verstärkung ab.
    combat.stepPassive(timeMs);

    if (tick === 0) combat.cast(buff, timeMs);
    // Drei KINDLE: der erste im Fenster (drei Wartezeiten = 4500 ms), der
    // letzte deutlich dahinter. So steht der Ablauf mit in der Datei.
    if (tick === 150 || tick === 300 || tick === 500) combat.cast(kindle, timeMs);

    checksums.push(combatChecksum(combat, timeMs) >>> 0);
    if (tick === 0 || tick === 150 || tick === 500) {
      factorAt.push({ tick, buffs: combat.damageFactors.buffs });
    }
  }

  return {
    schema: SCHEMA,
    ticks,
    tickMs: TICK_MS,
    bossId: boss.id,
    /** Die Folge, damit die Rust-Seite dieselbe fährt. */
    presses: [
      { tick: 0, spellId: 'test-buff' },
      { tick: 150, spellId: 'kindle' },
      { tick: 300, spellId: 'kindle' },
      { tick: 500, spellId: 'kindle' },
    ],
    checksums,
    factorAt,
    expected: {
      damage: combat.damageDealt,
      /**
       * **AC1 im Datensatz selbst:** Mindestens ein Messpunkt trägt einen
       * Faktor über 1. Wäre das nicht so, prüften beide Seiten einen Lauf, in
       * dem die Verstärkung gar nicht aktiv war — und wären einig darüber,
       * nichts zu tun.
       */
      buffWasActive: factorAt.some((f) => f.buffs !== 1),
    },
  };
}

export function buffsPayload(): Record<string, unknown> {
  const buffs = new Buffs();
  const steps: Record<string, unknown>[] = [];

  const merken = (was: string, timeMs: number): void => {
    const faktor = buffs.factor(timeMs);
    steps.push({
      was,
      timeMs,
      aktiv: buffs.activeCount(timeMs),
      checksum: mixFloat(FNV_OFFSET, faktor) >>> 0,
    });
  };

  // Zwei verschiedene Arten — multiplikativ.
  buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
  merken('art 0 mit 1,2 fuer 3 Wartezeiten', 0);
  buffs.applyForGcds(1, 1.5, 2, 0, GCD_MS);
  merken('art 1 mit 1,5 fuer 2 Wartezeiten', 0);

  // Dieselbe Art erneut — ersetzt, stapelt nicht.
  buffs.applyForGcds(0, 1.2, 3, 0, GCD_MS);
  merken('art 0 erneut — ersetzt', 0);

  // Eine schwächere ersetzt ebenfalls.
  buffs.applyForGcds(1, 1.1, 2, 0, GCD_MS);
  merken('art 1 schwaecher — ersetzt trotzdem', 0);

  // Eine dritte Art mit einem krummen Faktor.
  buffs.applyForGcds(2, 1.0000000000000002, 1, 0, GCD_MS);
  merken('art 2 mit einem Faktor knapp ueber eins', 0);

  // Nach 1,5 s läuft die dritte ab, nach 3 s die zweite.
  buffs.expire(1500);
  merken('nach dem Ablauf der dritten', 1500);
  buffs.expire(3000);
  merken('nach dem Ablauf der zweiten', 3000);
  buffs.expire(4500);
  merken('nach dem Ablauf der ersten', 4500);

  return {
    schema: SCHEMA,
    gcdMs: GCD_MS,
    kindCapacity: BUFF_KIND_CAPACITY,
    steps,
    replacements: buffs.replacementCount,
    expirations: buffs.expirationCount,
  };
}

/**
 * Der gemeinsame Nummernraum der Protokoll-Arten (Issue E3-6).
 *
 * **Warum diese Datei existiert:** Bis E3-6 führten Spiel und Simulator eigene
 * Nummern, und ab der 2 bedeutete dieselbe Zahl etwas anderes. Verglichen hat
 * das nie jemand — deshalb ist es nie aufgefallen, und deshalb steht hier
 * jetzt ein Wächter. Er kostet nichts, solange beide Seiten übereinstimmen,
 * und benennt bei Abweichung die betroffene Kennnummer.
 */
export function logKindsPayload(): Record<string, unknown> {
  return {
    schema: SCHEMA,
    kinds: EVENT_NAMES.map((name, kind) => ({
      kind,
      name,
      writtenByGame: EVENT_KINDS_WRITTEN_BY_GAME.includes(kind),
    })),
    writtenByGame: [...EVENT_KINDS_WRITTEN_BY_GAME],
  };
}

/**
 * Eine feste Folge am Auswahlmodus des Rücksetzers (Issue E3-3c).
 *
 * **Aussagekräftig wie bei den Verstärkungen, nicht trivial wie bei der
 * Sublimation:** Der Modus hängt an keinem ausgelieferten Zauber. Was er tut
 * — Abklingzeit auf null, Wartezeit auslösen, verpuffen — lässt sich auf
 * beiden Seiten mit denselben Schritten nachrechnen.
 *
 * Der Zustand nach jedem Schritt geht als Prüfsumme in die Datei: die
 * Restzeiten aller Zauber, die Wartezeit und die beiden Zähler.
 */
export function cooldownResetPayload(): Record<string, unknown> {
  const school = parseSchool({
    schema: 1,
    school: 'CALCINATION',
    resource: { id: 'heat', name: 'Heat', cap: 20, decayPerStepMs: 2000, decayAmount: 1 },
    spells: [
      { id: 'lang', name: 'Lang', cooldownMs: 30_000, baseDamage: 50 },
      { id: 'kurz', name: 'Kurz', cooldownMs: 1500, baseDamage: 10 },
      // Kurze Abklingzeit, damit die Folge ihn ZWEIMAL benutzen kann — einmal
      // in die Selbstwahl, einmal in den Treffer. Die echte Zahl steht noch
      // nicht fest (Issue #102, „eigene Abklingzeit: offen").
      { id: 'reset', name: 'Reset', cooldownMs: 3000, baseDamage: 0, armsCooldownReset: true },
    ],
  });
  if (school.problems.length > 0) throw new Error(school.problems.join(' '));

  const queue = new PressQueue();
  const combat = new Combat({ school: school.value, boss: FIRST_CRUCIBLE.value });
  const session = new Session(queue, combat);
  for (let slot = 0; slot < 3; slot++) session.bind(slot, alsSpellIndex(slot));
  session.startFight(0);

  const steps: Record<string, unknown>[] = [];
  const druecke = (slot: number, timeMs: number, was: string): void => {
    queue.push(slot, timeMs / TICK_MS, 0, 0);
    session.tick(timeMs);

    let hash = FNV_OFFSET;
    hash = mixInt(hash, timeMs);
    for (let i = 0; i < combat.spells.length; i++) {
      hash = mixFloat(hash, combat.cooldownRemainingMs(i, timeMs));
      hash = mixInt(hash, combat.castCount(alsSpellIndex(i)));
    }
    hash = mixFloat(hash, combat.gcd.remainingMs(timeMs));
    hash = mixInt(hash, session.resetsAppliedCount);
    hash = mixInt(hash, session.resetsWastedCount);

    steps.push({
      was,
      timeMs,
      armed: session.isResetArmed,
      applied: session.resetsAppliedCount,
      wasted: session.resetsWastedCount,
      readyAt: combat.spells.map((_, i) => combat.cooldownRemainingMs(i, timeMs)),
      gcdRemainingMs: combat.gcd.remainingMs(timeMs),
      checksum: hash >>> 0,
    });
  };

  druecke(0, 0, 'LANG wirken — 30 s Abklingzeit');
  druecke(2, 1500, 'Ruecksetzer wirken — Modus wird scharf');
  druecke(2, 3000, 'sich selbst waehlen — verpufft, Modus verbraucht');
  druecke(2, 4500, 'Ruecksetzer erneut wirken — wieder scharf');
  druecke(0, 6000, 'LANG waehlen — Abklingzeit faellt auf null');
  druecke(0, 7500, 'LANG erneut wirken — der Nachdruck');

  return { schema: SCHEMA, gcdMs: GCD_MS, steps };
}

/** So werden die Dateien geschrieben — an einer Stelle festgelegt. */
export function serialize(payload: unknown): string {
  return `${JSON.stringify(payload, null, 2)}\n`;
}

/**
 * Die exakten Bits einer Gleitkommazahl, als 16 Hexzeichen.
 *
 * **Die Regel für dieses Verzeichnis: Jede Gleitkommazahl, die der Simulator
 * aus einer Vergleichsdatei liest, steht dort zweimal** — einmal dezimal,
 * damit ein Diff lesbar bleibt, und einmal als Bitmuster, weil nur das den
 * Vergleich trägt.
 *
 * **Warum die Dezimalschreibweise nicht reicht** (Issue #122): `serde_json`
 * liest `1.6500000000000001` als `1.65` — ein Bit daneben. Gemessen am
 * 2026-07-29 und vom Orchestrator unabhängig nachgemessen:
 *
 * ```text
 *   1.6500000000000001   std=3ffa666666666667   serde=3ffa666666666666  ABWEICHUNG
 *   1.0000000000000002   std=3ff0000000000001   serde=3ff0000000000001  gleich
 *   2.1450000000000005   std=400128f5c28f5c2a   serde=400128f5c28f5c2a  gleich
 * ```
 *
 * Rusts eigener Zahlenleser trifft den Wert, `serde_json` nicht — und **nicht
 * systematisch**. Ein Vergleich über die Dezimalschreibweise ist damit grün
 * aus Glück, nicht aus Konstruktion, und die nächste Zahl, die jemand
 * einträgt, kann ihn stillschweigend entwerten.
 *
 * Gilt auch für **Eingabewerte**: Läse der Simulator die ein Bit daneben,
 * rechnete er mit anderen Zahlen als das Spiel — und meldete eine Abweichung,
 * die es nicht gibt.
 */
export function bitsOf(value: number): string {
  const buffer = new ArrayBuffer(8);
  new DataView(buffer).setFloat64(0, value);
  return Array.from(new Uint8Array(buffer))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Die vier Töpfe, das Werte-Budget und die zehn Plätze (Issue E6-1).
 *
 * **Warum das ohne einen einzigen ausgelieferten Gegenstand aussagekräftig
 * ist:** Der Gegenstands-Katalog steht noch nicht (GAME-DESIGN 22, vor E6).
 * Geprüft wird deshalb die Rechenregel, nicht ihr Inhalt — dieselbe Lage wie
 * beim Auswahlmodus (#102), wo der Nachweis auch ohne ausgelieferten Zauber
 * getragen hat. Die Regel ist ohne den Katalog vollständig; die Zahlen darin
 * sind ausdrücklich vorläufig (13.1b: „nicht festgelegt, sondern gemessen").
 */
export function gearPayload(): Record<string, unknown> {
  // Eine feste Folge von Beiträgen. Sie deckt beide Regeln ab: gleiche Töpfe
  // addieren, verschiedene multiplizieren — und enthält den Fall, um den es
  // in 13.1b eigentlich geht (Satz-Bonus neben legendärem Effekt).
  const beitraege: readonly [number, number, string][] = [
    [POOL_ESSENCE, 0.2, 'ESSENCE aus einem Gegenstand'],
    [POOL_ESSENCE, 0.3, 'ESSENCE aus einem zweiten — addiert sich zu 0,5'],
    [POOL_CATALYSIS, 0.1, 'CATALYSIS — eigener Faktor, multipliziert'],
    [POOL_GEAR_EFFECTS, 0.3, 'Sechser-Satz'],
    [POOL_GEAR_EFFECTS, 0.2, 'legendärer Effekt — 50 %, NICHT 56 %'],
    [POOL_YIELD, 0.25, 'YIELD — dritter eigener Faktor'],
  ];

  const pools = new ValuePools();
  const poolSteps: Record<string, unknown>[] = [];
  for (const [pool, share, was] of beitraege) {
    pools.add(pool, share);
    let hash = FNV_OFFSET;
    for (let index = 0; index < POOL_COUNT; index++) {
      hash = mixFloat(hash, pools.factor(index));
    }
    hash = mixFloat(hash, pools.product());
    poolSteps.push({
      was,
      pool,
      share,
      shareBits: bitsOf(share),
      factors: Array.from({ length: POOL_COUNT }, (_, index) => pools.factor(index)),
      factorBits: Array.from({ length: POOL_COUNT }, (_, index) => bitsOf(pools.factor(index))),
      product: pools.product(),
      productBits: bitsOf(pools.product()),
      checksum: hash >>> 0,
    });
  }

  // Die Stufenleiter. Nicht alle 100 Stufen — die Aussage steckt in den
  // Rändern und einigen Punkten dazwischen, und eine Datei mit 400 Zeilen
  // liest im Diff niemand mehr.
  const stufen = [1, 2, 3, 10, 25, 50, 75, 100];
  const budgets: Record<string, unknown>[] = [];
  for (let quality = 0; quality < QUALITY_COUNT; quality++) {
    const slots = QUALITY_AFFIX_SLOTS[quality] ?? 0;
    for (const level of stufen) {
      const werte = new Int32Array(slots);
      distributeBudget(level, quality, werte);
      let hash = FNV_OFFSET;
      hash = mixInt(hash, level);
      hash = mixInt(hash, quality);
      hash = mixInt(hash, budgetOf(level, quality));
      for (const wert of werte) hash = mixInt(hash, wert);
      budgets.push({
        quality,
        level,
        budget: budgetOf(level, quality),
        shares: Array.from(werte),
        checksum: hash >>> 0,
      });
    }
  }

  return {
    schema: SCHEMA,
    poolNames: POOL_NAMES,
    qualityNames: QUALITY_NAMES,
    qualityAffixSlots: QUALITY_AFFIX_SLOTS,
    qualityHasEffect: QUALITY_HAS_EFFECT,
    slotNames: SLOT_NAMES,
    baseStatIds: BASE_STAT_IDS,
    poolSteps,
    budgets,
  };
}

/**
 * Sätze: Zählung, Schwellen und Anteile (Issue E6-3).
 *
 * **Aussagekräftig ohne einen einzigen ausgelieferten Satz.** Welche 18 Boni
 * es gibt, ist Katalog-Inhalt (GAME-DESIGN 22, vor E10). Geprüft wird die
 * Rechenregel — dieselbe Lage wie bei den Töpfen (#114) und beim Auswahlmodus
 * (#102).
 *
 * Die Zusammenstellungen decken ausdrücklich den Fall mit ab, in dem sich
 * **nichts** ändern darf: eine Leiste ohne Satz-Teil.
 */
export function setsPayload(): Record<string, unknown> {
  const saetze: SetDef[] = [
    {
      id: 0,
      school: 'CALCINATION',
      tiers: [
        { kind: BONUS_VALUE, share: 0.1, effect: '' },
        { kind: BONUS_VALUE, share: 0.2, effect: '' },
        { kind: BONUS_ROTATION, share: 0, effect: 'test-a-sechser' },
      ],
    },
    {
      id: 1,
      school: 'SUBLIMATION',
      tiers: [
        { kind: BONUS_VALUE, share: 0.05, effect: '' },
        { kind: BONUS_VALUE, share: 0.15, effect: '' },
        { kind: BONUS_ROTATION, share: 0, effect: 'test-b-sechser' },
      ],
    },
  ];

  const leiste = (...teile: number[]): number[] => {
    const out = new Array<number>(SLOT_COUNT).fill(NO_SET);
    for (let i = 0; i < teile.length && i < SLOT_COUNT; i++) out[i] = teile[i] as number;
    return out;
  };

  const faelle: { was: string; equipped: number[] }[] = [
    // **Der Fall, in dem sich nichts ändern darf** — ohne ihn belegte die
    // Folge nur, dass überhaupt etwas passiert.
    { was: 'kein Satz-Teil', equipped: leiste() },
    { was: 'ein Teil — noch keine Stufe', equipped: leiste(0) },
    { was: 'zwei Teile — erste Stufe', equipped: leiste(0, 0) },
    { was: 'drei Teile — weiterhin erste Stufe', equipped: leiste(0, 0, 0) },
    { was: 'vier Teile — zweite Stufe', equipped: leiste(0, 0, 0, 0) },
    { was: 'fuenf Teile — wie vier, nicht weniger', equipped: leiste(0, 0, 0, 0, 0) },
    { was: 'sechs Teile — der Rotations-Effekt', equipped: leiste(0, 0, 0, 0, 0, 0) },
    { was: 'vier von A und zwei von B', equipped: leiste(0, 0, 0, 0, 1, 1) },
    // Der Platz spielt keine Rolle: dieselben vier Teile, weit hinten.
    {
      was: 'vier von A auf hinteren Plaetzen',
      equipped: [NO_SET, NO_SET, 0, NO_SET, 0, NO_SET, NO_SET, 0, NO_SET, 0],
    },
  ];

  const steps = faelle.map(({ was, equipped }) => {
    const pieces = new Int32Array(SET_COUNT);
    const getragen = countPieces(equipped, pieces);
    const anteil = valueShareOf(saetze, pieces);
    let hash = FNV_OFFSET;
    hash = mixInt(hash, getragen);
    for (const zahl of pieces) hash = mixInt(hash, zahl);
    hash = mixFloat(hash, anteil);
    return {
      was,
      equipped,
      pieces: Array.from(pieces),
      wornPieces: getragen,
      tiersReached: Array.from(pieces, (n) => tiersReached(n)),
      valueShare: anteil,
      valueShareBits: bitsOf(anteil),
      rotationEffects: activeRotationEffects(saetze, pieces),
      checksum: hash >>> 0,
    };
  });

  return {
    schema: SCHEMA,
    tiers: SET_TIERS,
    setCount: SET_COUNT,
    sets: saetze,
    steps,
  };
}

/**
 * Die Umrechnung von Tempo auf Zeiten (GAME-DESIGN 6.7, Issue #177).
 *
 * **Warum das ein eigener Vergleich ist und kein Kampf-Prüffall reicht:**
 * Dieselbe Überlegung wie bei `primitives.json`. Ein Kampf ohne getragene
 * Ausrüstung hat CATALYSIS = 0 und läuft nie durch diese Rechnung — eine Seite
 * könnte abrunden statt aufzurunden, und jeder heutige Kampf-Prüffall bliebe
 * grün. Auffallen würde es erst mit dem ersten getragenen Gegenstand, und dann
 * suchte man im Kampfablauf statt in der Umrechnung.
 *
 * Ganzzahlige Werte, also **keine Bitmuster nötig**: Was hier verglichen wird,
 * sind Millisekunden und Tick-Zahlen. Die Regel „jede Gleitkommazahl steht
 * zweimal da" greift nur, wo eine steht.
 */
export function hastePayload(): Record<string, unknown> {
  // Die Basiszeiten decken den Bereich aus 6.4 ab: Abklingzeiten von 3 bis
  // 180 s, dazu die Wartezeit und ein DoT-Abstand. Der krumme Wert 12 345 ist
  // Absicht — an ihm hängt der Sonderfall, in dem AC7 gegen das Takt-Raster
  // gewinnt (siehe `hastedMs`).
  const basiszeiten = [0, 10, 1500, 2000, 3000, 8000, 12_345, 180_000];
  // Von "kein Tempo" bis weit über jeden denkbaren Deckel. Die 500 sind der
  // Wert, an dem 1500 ms auf 1000 ms fallen — die Rechnung, mit der 6.7 den
  // Deckel begründet.
  const tempowerte = [0, 1, 50, 170, 200, 333, 500, 750, 1000, 2000, 10_000];

  const steps: Record<string, unknown>[] = [];
  for (const basis of basiszeiten) {
    for (const permille of tempowerte) {
      const zeit = hastedMs(basis, permille);
      let hash = FNV_OFFSET;
      hash = mixInt(hash, basis);
      hash = mixInt(hash, permille);
      hash = mixInt(hash, zeit);
      steps.push({ baseMs: basis, permille, hastedMs: zeit, checksum: hash >>> 0 });
    }
  }

  // Die DoT-Seite getrennt: Die Dauer bleibt, der Abstand schrumpft, die Zahl
  // der Ticks steigt. `restMs` zeigt, was verfällt — der Wert, den WoW anteilig
  // auszahlen würde und wir nicht.
  const dotSteps = [1000, 2000, 3000].flatMap((basisAbstand) =>
    [0, 170, 200, 500, 1000].map((permille) => {
      const dauer = 12_000;
      const abstand = hastedMs(basisAbstand, permille);
      const ticks = tickCount(dauer, abstand);
      let hash = FNV_OFFSET;
      hash = mixInt(hash, dauer);
      hash = mixInt(hash, abstand);
      hash = mixInt(hash, ticks);
      return {
        durationMs: dauer,
        baseIntervalMs: basisAbstand,
        permille,
        intervalMs: abstand,
        ticks,
        restMs: dauer - ticks * abstand,
        checksum: hash >>> 0,
      };
    }),
  );

  return { schema: SCHEMA, tickMs: TICK_MS, steps, dotSteps };
}

/**
 * Die getragenen Werte, mit denen der Gleichlauf-Lauf aus AC8 rechnet.
 *
 * **Alle drei ungleich null, und keiner glatt.** Ein Lauf mit CATALYSIS 500
 * (also genau der Hälfte) verstünde sich mit jeder Zeit glatt und ließe eine
 * fehlende Rasterung durchgehen; 170 Promille erzwingt krumme Zwischenwerte.
 * PURITY 375 liegt bewusst zwischen den beiden Kurzschlüssen von `chance`, so
 * dass wirklich gewürfelt wird.
 */
const WORN_FOR_PARITY: WornValues = {
  catalysis: 170,
  purity: 375,
  yieldBonus: 250,
  // **Nicht null** (Issue #282, AC6). Stünden die drei Grundwerte hier auf
  // null, liefe der Gleichlauf-Lauf durch die neue Rechnung, ohne sie je zu
  // benutzen — er belegte dann, dass beide Seiten dasselbe tun, wenn nichts
  // zu tun ist.
  //
  // Drei verschiedene Werte, damit eine vertauschte Zuordnung auffällt: Wären
  // sie gleich, blieben `essence` und `ward` vertauscht unbemerkt.
  essence: 220,
  vessel: 400,
  ward: 90,
  gearEffects: 130,
};

/** Der Name der Vergleichsdatei mit getragener Ausrüstung. */
export const WORN_NAME = 'worn-values';

/**
 * Ein aufgezeichneter Lauf **mit getragener Ausrüstung** (Issue #177, AC8).
 *
 * # Warum der bestehende Lauf nicht reicht
 *
 * `cooldown-precision.json` läuft ohne Ausrüstung. Er belegt damit AC7 — dass
 * sich nichts bewegt — aber über die neuen Rechenwege sagt er nichts: Keine
 * Zeit wird umgerechnet, kein Wurf fällt. Eine Seite könnte den Tick-Abstand
 * anders runden oder den Zufallsstrom anders oft drehen, und dieser Lauf bliebe
 * bitgleich.
 *
 * Hier ist beides im Spiel. **Die Prüfsumme je Takt** ist dabei das Schärfere
 * am Vergleich: Ein Endergebnis kann zufällig übereinstimmen, während der Weg
 * dahin auseinanderlief.
 */
export function wornPayload(): Record<string, unknown> {
  const recording = recordings().get(RECORDING_NAME);
  if (recording === undefined) {
    throw new Error(`Unbekannte Aufzeichnung: ${RECORDING_NAME}`);
  }
  const result = replayRecording(recording, { checksumPerTick: true, worn: WORN_FOR_PARITY });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    worn: {
      catalysis: WORN_FOR_PARITY.catalysis,
      purity: WORN_FOR_PARITY.purity,
      yieldBonus: WORN_FOR_PARITY.yieldBonus,
      essence: WORN_FOR_PARITY.essence,
      vessel: WORN_FOR_PARITY.vessel,
      ward: WORN_FOR_PARITY.ward,
      gearEffects: WORN_FOR_PARITY.gearEffects,
    },
    // Topf 4 als Faktor: eins plus die Summe. Fliesskomma, also mit Bitmuster.
    gearEffectFactor: gearEffectFactor(WORN_FOR_PARITY),
    gearEffectFactorBits: bitsOf(gearEffectFactor(WORN_FOR_PARITY)),
    critBasePermille: CRIT_BASE_PERMILLE,
    // Die abgeleiteten Werte stehen mit in der Datei, jeder auch als Bitmuster:
    // Sie sind Fließkommazahlen und gehen in den Schaden ein. Die Regel dieses
    // Verzeichnisses gilt (siehe `bitsOf`) — dezimal für den Diff, Bits für den
    // Vergleich.
    critChance: critChance(WORN_FOR_PARITY),
    critChanceBits: bitsOf(critChance(WORN_FOR_PARITY)),
    critFactor: critFactor(WORN_FOR_PARITY),
    critFactorBits: bitsOf(critFactor(WORN_FOR_PARITY)),
    bindings: STEP1_BINDINGS.map((binding) => ({ slot: binding.slot, spellId: binding.spellId })),
    presses: recording.presses.map((press) => ({ tick: press.tick, slot: press.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      randomDraws: result.randomDraws,
    },
    checksums: result.checksums,
  };
}

/**
 * Die fünf Quellen der Sonderwerte (Issue #178).
 *
 * **Warum das ein eigener Vergleich ist:** Die Zahlen entscheiden, was ein
 * Spieler überhaupt erreichen kann. Liefen sie auseinander, rechnete der
 * Balance-Läufer mit anderen Grenzen als das Spiel — und beide sähen für sich
 * genommen richtig aus.
 *
 * Ganzzahlig in Promille, also **keine Bitmuster nötig**.
 */
export function sourcesPayload(): Record<string, unknown> {
  const stats = [];
  for (let stat = 0; stat < STAT_COUNT; stat++) {
    const je = [];
    for (let source = 0; source < SOURCE_COUNT; source++) {
      je.push({ source, name: SOURCE_NAMES[source], max: maxOf(source, stat) });
    }
    stats.push({
      stat,
      name: STAT_NAMES[stat],
      sources: je,
      permanentMax: permanentMax(stat),
      totalMax: totalMax(stat),
      pure: pureBuild(stat),
      mixed: mixedBuild(stat),
    });
  }

  // Der Affix-Wert je Stufe und Qualität — die Ableitung, an der beide Seiten
  // hängen. Wenige Stützstellen genügen: Bricht die Herleitung, bricht sie
  // überall.
  const affixes = [];
  for (const quality of [QUALITY_COMMON, QUALITY_EPIC]) {
    for (const level of [1, 25, 50, 75, MAX_ITEM_LEVEL]) {
      affixes.push({
        level,
        quality,
        budget: budgetOf(level, quality),
        permille: affixPermille(level, quality),
      });
    }
  }

  // Die Grundwerte je Stufe (Issue #282) — dieselben Stützstellen wie oben.
  //
  // **Die Basis wandert mit.** Sie ist unkalibriert; stünde nur das Ergebnis
  // hier, meldete der Gleichlauf-Wächter beim nächsten Kalibrieren einen
  // Unterschied zwischen den Seiten, wo nur eine Zahl bewegt wurde. Mit der
  // Basis daneben kann die Rust-Seite dieselbe Rechnung machen statt dieselbe
  // Zahl zu erwarten.
  const baseStats = [];
  for (const [name, basis] of [
    ['essence', ESSENCE_AT_LEVEL_1],
    ['vessel', VESSEL_AT_LEVEL_1],
    ['ward', WARD_AT_LEVEL_1],
  ] as const) {
    for (const level of [1, 25, 50, 75, MAX_ITEM_LEVEL]) {
      baseStats.push({ stat: name, level, base: basis, permille: basePermille(level, basis) });
    }
  }

  return {
    schema: SCHEMA,
    budgetPerPermille: BUDGET_PER_PERMILLE,
    sourceNames: SOURCE_NAMES,
    statNames: STAT_NAMES,
    permanent: Array.from({ length: SOURCE_COUNT }, (_, s) => isPermanent(s)),
    stats,
    affixes,
    baseStats,
  };
}

/** Die Leiste der Synergie-Aufzeichnung (Issue #318). */
export const SYNERGY_NAME = 'synergy-window';
const SYNERGY_BINDINGS = [
  { slot: 0, spellId: 'canker' },
  { slot: 1, spellId: 'fester' },
] as const;

/**
 * Eine Runde, in der die Synergie WIRKLICH GEWIRKT WIRD (Issue #318, AC7).
 *
 * # Warum es diese Aufzeichnung braucht
 *
 * Die dreizehn bestehenden belegen die Synergie nicht: CANKER und FESTER werden
 * in keiner von ihnen gedrückt. Sie sind grün, **weil** die Mechanik dort nie
 * anläuft — dieselbe Lage, in der eine fertige Mechanik ohne Nutzer als fertig
 * gilt (#201, #219, #223).
 *
 * # Eigene Leiste, und deshalb ging es bis heute nicht
 *
 * `STEP1_BINDINGS` trägt kindle, smoulder, pyre und quench. Der Simulator liest
 * die Leiste seit jeher aus der Aufzeichnung; das Spiel tat es nicht, sondern
 * fiel auf `STEP1_BINDINGS` zurück. Eine Runde mit CANKER hätte im Spiel KINDLE
 * gewirkt — beide Seiten wären auseinandergelaufen, und zwar nicht wegen der
 * Synergie.
 *
 * # Die Tastenfolge
 *
 * CANKER bei 0, FESTER eine Wartezeit später — also mitten im Fenster, das zwei
 * Wartezeiten offen steht. Danach beide, sobald sie bereit sind: Das prüft, dass
 * ein verbrauchtes Fenster nicht doppelt zählt.
 */
function recordSynergy(ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const canker = combat.indexOf('canker');
  const fester = combat.indexOf('fester');
  const presses: RecordedPress[] = [];
  // Die Prüfung stand hier zuerst, als Einzelstück (Issue #351). Sie ist jetzt
  // im gemeinsamen Helfer — eine zweite Kopie war der Grund, warum die andere
  // Stelle sie nie bekam.
  const press = pressRecorder(combat, SYNERGY_BINDINGS, presses);

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    // Der erste Durchgang von Hand: CANKER, dann FESTER im Fenster.
    if (timeMs === 0) press(canker, tick, timeMs);
    else if (timeMs === GCD_MS) press(fester, tick, timeMs);
    // Danach beide, sobald sie bereit sind. FESTER hat 9 s Abklingzeit und
    // trifft damit mal ins Fenster und mal daneben — genau das soll die
    // Aufzeichnung abdecken.
    else if (combat.isOffCooldown(canker, timeMs)) press(canker, tick, timeMs);
    else if (combat.isOffCooldown(fester, timeMs)) press(fester, tick, timeMs);
  }

  return { name: SYNERGY_NAME, seed: 0, ticks, presses, bindings: SYNERGY_BINDINGS };
}

export function synergyPayload(): Record<string, unknown> {
  const recording = recordSynergy(3000);
  const result = replayRecording(recording, { checksumPerTick: true });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: SYNERGY_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      synergyUsed: result.synergyUsed,
      synergyAvailable: result.synergyAvailable,
    },
    checksums: result.checksums,
  };
}

/** Der Name der Aufzeichnung, die Tempo und Synergie zusammenführt (Issue #456). */
export const SYNERGY_HASTE_NAME = 'synergy-haste';

/**
 * Das Tempo dieser Aufzeichnung — und warum ausgerechnet dieser Wert.
 *
 * # Fast die Hälfte aller Werte taugt nicht
 *
 * `gcds × hasted(GCD)` und `hasted(gcds × GCD)` ergeben bei **499 von 1001**
 * Promille-Werten dasselbe. Ausgerechnet, nicht geschätzt: über den ganzen
 * Bereich, mit `hastedMs` aufgerufen statt nachgebaut.
 *
 * Wer einen Wert ohne diese Rechnung wählt, greift mit rund fünfzig Prozent
 * Wahrscheinlichkeit daneben — und bekommt eine Aufzeichnung, die grün ist und
 * nichts belegt. Bekannt waren 0, 376 und 500; die Zahl ist also nicht drei,
 * sondern fast fünfhundert.
 *
 * Bei 300 ‰ unterscheiden sich beide Rechnungen: 2320 gegen 2310.
 */
const SYNERGY_HASTE_CATALYSIS = 300;

/**
 * Nur Tempo, sonst nichts.
 *
 * **Das ist der ganze Trick an dieser Aufzeichnung.** Nähme sie das volle
 * `WORN_FOR_PARITY`, brächte sie Krit, Ertrag und Topf 4 mit — und bräche sie
 * dann bei der alten Klammerung, bliebe offen, woran es lag. So ist der
 * **einzige** Unterschied zur bestehenden `synergy-window.json` das Tempo.
 */
const WORN_FOR_SYNERGY_HASTE: WornValues = { ...NOTHING_WORN, catalysis: SYNERGY_HASTE_CATALYSIS };

/**
 * Eine Runde mit Tempo **und** offenem Synergie-Fenster (Issue #456, Schritt 2).
 *
 * # Warum keine bestehende Aufzeichnung das leistet
 *
 * Alle dreissig wurden durchgesehen. Zwei kommen nahe und keine trifft:
 *
 *   synergy-window.json   canker und fester auf der Leiste — aber kein `worn`
 *   worn-values.json      catalysis 170 — aber die Leiste trägt kindle,
 *                         smoulder, pyre, quench
 *
 * Jede Hälfte gibt es, nie zusammen. Deshalb konnten die beiden Seiten in
 * Teil 1 verschieden klammern, ohne dass die Paritätsprüfung etwas sah: Sie
 * vergleicht, was aufgezeichnet ist.
 *
 * # DER DRUCK BEI ZWEI WARTEZEITEN IST DER GANZE NACHWEIS
 *
 * Der Unterschied zwischen beiden Klammerungen ist **zehn Millisekunden** — bei
 * 300 ‰ endet das Fenster neu bei 2320 und alt bei 2310. Ein Druck irgendwo im
 * Fenster liegt bei **beiden** drin und zeigt gar nichts.
 *
 * Über den ganzen Lauf gibt es genau **einen** Takt, der nur im neuen Fenster
 * liegt: `2 × GCD`, also 2320. Und das ist kein Zufall, sondern genau die
 * Aussage der neuen Rechnung — das Fenster ist zwei **echte** Wartezeiten lang
 * und endet deshalb exakt dann, wenn die zweite vorbei ist. Die alte Rechnung
 * beschleunigt eine Doppellänge und verliert dabei durch Abrunden zehn
 * Millisekunden; der Druck fällt heraus.
 *
 * Dieser eine Druck wird hier **gezielt gesetzt**, nicht dem Rhythmus
 * überlassen. Eine Aufzeichnung, die ihn zufällig treffen müsste, wäre eine
 * Aufzeichnung, die es meistens nicht tut.
 */
function recordSynergyHasted(ticks: number): Recording {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    worn: WORN_FOR_SYNERGY_HASTE,
  });
  const canker = combat.indexOf('canker');
  const fester = combat.indexOf('fester');
  const presses: RecordedPress[] = [];
  const press = pressRecorder(combat, SYNERGY_BINDINGS, presses);

  // Die beschleunigte Wartezeit. **Aus der Formel geholt, nicht abgeschrieben**
  // — eine zweite Handrechnung liefe beim nächsten Nachziehen auseinander.
  const gcd = hastedMs(GCD_MS, SYNERGY_HASTE_CATALYSIS);
  const fensterEnde = 2 * gcd;

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    // CANKER öffnet das Fenster.
    if (timeMs === 0) press(canker, tick, timeMs);
    // **Der entscheidende Druck.** FESTER genau am Fensterende — im neuen
    // Fenster drin, im alten zehn Millisekunden draussen. Bei `1 × gcd` wird
    // bewusst nichts gewirkt: Ein Druck dort verbrauchte FESTER neun Sekunden
    // lang und nähme diesem Takt seinen Zauber.
    else if (timeMs === fensterEnde) press(fester, tick, timeMs);
    // Danach der übliche Rhythmus, damit die Aufzeichnung Substanz hat und
    // nicht nur aus dem einen Messpunkt besteht.
    else if (timeMs > fensterEnde && combat.isOffCooldown(canker, timeMs)) {
      press(canker, tick, timeMs);
    } else if (timeMs > fensterEnde && combat.isOffCooldown(fester, timeMs)) {
      press(fester, tick, timeMs);
    }
  }

  return { name: SYNERGY_HASTE_NAME, seed: 0, ticks, presses, bindings: SYNERGY_BINDINGS };
}

export function synergyHastePayload(): Record<string, unknown> {
  const recording = recordSynergyHasted(3000);
  const result = replayRecording(recording, {
    checksumPerTick: true,
    worn: WORN_FOR_SYNERGY_HASTE,
  });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    // Nur `catalysis` weicht ab — die Rust-Seite baut daraus dasselbe `worn`.
    worn: {
      catalysis: WORN_FOR_SYNERGY_HASTE.catalysis,
      purity: WORN_FOR_SYNERGY_HASTE.purity,
      yieldBonus: WORN_FOR_SYNERGY_HASTE.yieldBonus,
      essence: WORN_FOR_SYNERGY_HASTE.essence,
      vessel: WORN_FOR_SYNERGY_HASTE.vessel,
      ward: WORN_FOR_SYNERGY_HASTE.ward,
      gearEffects: WORN_FOR_SYNERGY_HASTE.gearEffects,
    },
    // **Die Fensterlänge steht mit in der Datei**, und zwar beide Rechnungen.
    // Sie ist der Grund für diese Aufzeichnung; sie nur implizit über die
    // Prüfsumme zu vergleichen hiesse, den Nachweis zu verstecken.
    synergyWindowMs: 2 * hastedMs(GCD_MS, SYNERGY_HASTE_CATALYSIS),
    synergyWindowMsOldBracketing: hastedMs(2 * GCD_MS, SYNERGY_HASTE_CATALYSIS),
    hastedGcdMs: hastedMs(GCD_MS, SYNERGY_HASTE_CATALYSIS),
    bindings: SYNERGY_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      synergyUsed: result.synergyUsed,
      synergyAvailable: result.synergyAvailable,
    },
    checksums: result.checksums,
  };
}

/** Die Aufzeichnung, in der die neuen Kalzinations-Zauber gewirkt werden. */
export const NEW_CALCINATION_NAME = 'new-calcination';

/**
 * Der Paritäts-Weg für `bellows` und `searing` (Issue #268).
 *
 * # Warum es ihn gibt
 *
 * `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einem
 * Paritäts-Weg **gewirkt** wird. Ohne diesen Weg gälte die Bitgleichheit für
 * die beiden neuen Zauber als *angenommen* — und eine Annahme ist genau das,
 * was die Vergleichsdateien ersetzen sollen.
 *
 * # Was hier NICHT gemessen wird
 *
 * Nicht, ob die Rotation gut ist. Das beantwortet der Läufer aus #237. Hier
 * zählt allein, dass beide Zauber **gewirkt** werden und beide Seiten dabei
 * Takt für Takt dieselbe Prüfsumme rechnen.
 */
export function newCalcinationPayload(): Record<string, unknown> {
  const recording = recordings().get(NEW_CALCINATION_NAME);
  if (recording === undefined) {
    throw new Error(`Unbekannte Aufzeichnung: ${NEW_CALCINATION_NAME}`);
  }
  const boss = FIRST_CRUCIBLE.value;
  const bindings = [
    { slot: 0, spellId: 'searing' },
    { slot: 1, spellId: 'bellows' },
  ];

  const combat = new Combat({ school: CALCINATION.value, boss, seed: recording.seed });
  const checksums: number[] = [];
  let sequence = 0;
  let pressIndex = 0;

  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      for (const binding of bindings) {
        if (binding.slot === press.slot) combat.cast(combat.indexOf(binding.spellId), timeMs);
      }
    }
    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    checksums.push(hash);
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bossId: 'first-crucible',
    bindings,
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: combat.damageDealt,
      finalChecksum: combatChecksum(combat, (recording.ticks - 1) * TICK_MS),
      sequenceChecksum: sequence,
      // **Je Zauber, damit sichtbar ist, dass BEIDE gewirkt wurden.** Eine
      // Gesamtzahl verstiege sich zu der Aussage, dass überhaupt gedrückt
      // wurde — nicht, dass beide dabei waren.
      casts: Object.fromEntries(
        bindings.map((b) => [b.spellId, combat.castCount(combat.indexOf(b.spellId))]),
      ),
    },
    checksums,
  };
}

/**
 * Der WARD-Deckel, beidseitig nachgerechnet (Issue #466, AC3).
 *
 * # Warum es diese Aufzeichnung braucht
 *
 * Der Deckel ist eine **Regel, keine Zahl in den Daten**: Heute trägt kein
 * Gegenstand genug WARD, um ihn zu erreichen. Ohne eigene Aufzeichnung wäre er
 * damit auf beiden Seiten eingebaut und auf keiner verglichen — genau die Lage
 * aus #456 Schritt 1, wo zwei Seiten monatelang verschieden rechneten, ohne
 * dass die Paritätsprüfung etwas sah.
 *
 * Sobald der Katalog wächst (#421b), wird der Deckel erreicht. Dann soll der
 * Vergleich schon stehen und nicht erst gebaut werden.
 *
 * # Vier Läufe, die sich in genau einer Sache unterscheiden
 *
 * Derselbe Boss, dieselbe Dauer, **keine** Tastendrücke — der Spieler wirkt
 * nichts, der Boss schlägt zu. So misst der Lauf ausschliesslich die Minderung;
 * ein Zauber brächte Schaden, Hitze und womöglich Heilung hinein.
 *
 *   400 ‰   Gesamtminderung 60 %   unter dem Deckel
 *   550 ‰                   75 %   genau auf dem Deckel
 *   900 ‰                  110 %   darüber — gedeckelt auf 75 %
 *  5000 ‰                  520 %   weit darüber, gleiches Ergebnis
 *
 * **Die drei letzten müssen denselben Schaden ergeben, der erste einen
 * anderen.** Beides steht als Zusicherung in der Datei, damit die Rust-Seite
 * es prüfen kann, ohne die Zahlen zu kennen.
 *
 * # Gegen `test-onslaught` und nicht gegen den ersten Tiegel
 *
 * Der erste Tiegel hat keine Zeitleiste — dort verliert der Spieler in dreissig
 * Sekunden null Leben, und alle vier Läufe wären identisch null. Ein Prüffall,
 * der nichts misst und trotzdem grün ist.
 */
export function wardCapPayload(): Record<string, unknown> {
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('Der Boss test-onslaught fehlt.');
  const ticks = 3000;

  const runs = [400, 550, 900, 5000].map((wardPermille) => {
    const combat = new Combat({
      school: CALCINATION.value,
      boss,
      seed: 0,
      worn: { ...NOTHING_WORN, ward: wardPermille },
    });
    const checksums: number[] = [];
    const playerHp: number[] = [];
    let sequence = 0;

    for (let tick = 0; tick < ticks; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      checksums.push(hash);
      playerHp.push(combat.playerHp);
      // Der Spielertod beendet den Versuch, wie in `playerDamagePayload`.
      if (combat.playerDead) break;
    }

    return {
      wardPermille,
      // Die Gesamtminderung, wie der Kampf sie sieht — ungedeckelt. Sie steht
      // dabei, weil sie erklärt, WARUM drei Läufe gleich ausgehen.
      totalPermilleUncapped: PLAYER.wardReduction * 1000 + wardPermille,
      expected: {
        damageTaken: combat.damageTaken,
        playerHp: combat.playerHp,
        diedAtMs: combat.diedAtMs,
        sequenceChecksum: sequence,
      },
      checksums,
      playerHpPerTick: playerHp,
    };
  });

  return {
    schema: SCHEMA,
    name: WARD_CAP_NAME,
    bossId: boss.id,
    tickMs: TICK_MS,
    ticks,
    // Die beiden Zahlen, um die es geht. Aus den Daten gelesen, nicht getippt:
    // Ändert der Auftraggeber den Deckel, wandert er durch bis in diese Datei.
    wardCapPermille: PLAYER.wardCapPermille,
    wardReductionPermille: PLAYER.wardReduction * 1000,
    runs,
  };
}

/** Der Name der Aufzeichnung, in der die PUTREFACTION-Füller gewirkt werden. */
export const PUTREFACTION_FILLER_NAME = 'putrefaction-filler';

/** Die Leiste dieser Aufzeichnung. */
const PUTREFACTION_FILLER_BINDINGS = [
  { slot: 0, spellId: 'canker' },
  { slot: 1, spellId: 'blight' },
] as const;

/**
 * BLIGHT im Paritäts-Weg (Issue #479).
 *
 * # Warum eine EIGENE Aufzeichnung und nicht `synergy-window` erweitert
 *
 * `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einem
 * Paritäts-Weg **gewirkt** wird — sonst gilt die Bitgleichheit für ihn als
 * angenommen. BLIGHT musste also hinein.
 *
 * Der naheliegende Weg wäre gewesen, ihn in `synergy-window` mitlaufen zu
 * lassen: die bestehende PUTREFACTION-Aufzeichnung. Dagegen sprach, dass deren
 * Tastenfolge **die Aussage ist** — CANKER bei 0, FESTER eine Wartezeit
 * später, mitten im Fenster. Ein dritter Zauber dazwischen verschiebt jeden
 * folgenden Druck und damit den Nachweis, für den sie gebaut wurde.
 *
 * Dasselbe gilt für `synergy-haste` aus #456, wo der Druck bei `2 × GCD` der
 * ganze Punkt ist.
 *
 * Eine eigene Datei kostet ein paar Zeilen und lässt beide Nachweise stehen.
 *
 * # Was sie zeigt
 *
 * BLIGHT und CANKER im Wechsel, jeder sobald er bereit ist. Das ist die
 * Rotation, für die BLIGHT gebaut ist: CANKER hält 2 s Abklingzeit, BLIGHT
 * füllt die Lücken und legt dabei selbst Stapel.
 */
function recordPutrefactionFiller(ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const canker = combat.indexOf('canker');
  const blight = combat.indexOf('blight');
  const bindings = PUTREFACTION_FILLER_BINDINGS;
  const presses: RecordedPress[] = [];
  const press = pressRecorder(combat, bindings, presses);

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    // CANKER hat Vorrang: Er öffnet nebenbei das Synergie-Fenster. BLIGHT
    // füllt, was übrig bleibt — genau seine Rolle.
    if (combat.isOffCooldown(canker, timeMs)) press(canker, tick, timeMs);
    else if (combat.isOffCooldown(blight, timeMs)) press(blight, tick, timeMs);
  }

  return { name: PUTREFACTION_FILLER_NAME, seed: 0, ticks, presses, bindings };
}

export function putrefactionFillerPayload(): Record<string, unknown> {
  const recording = recordPutrefactionFiller(3000);
  const result = replayRecording(recording, { checksumPerTick: true });

  return {
    schema: SCHEMA,
    name: recording.name,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    bindings: PUTREFACTION_FILLER_BINDINGS.map((b) => ({ slot: b.slot, spellId: b.spellId })),
    presses: recording.presses.map((p) => ({ tick: p.tick, slot: p.slot })),
    expected: {
      damage: result.damage,
      killedAtMs: result.killedAtMs,
      accepted: result.accepted,
      rejected: result.rejected,
      casts: result.casts,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
    },
    checksums: result.checksums,
  };
}
