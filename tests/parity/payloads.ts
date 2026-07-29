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
import { mixFloat, mixInt } from '../../src/core/checksum.js';
import { STEP1_BINDINGS } from '../../src/core/loadout.js';
import { TICK_MS } from '../../src/core/loop.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { EVENT_KINDS_WRITTEN_BY_GAME, EVENT_NAMES } from '../../src/core/log.js';
import { BUFF_KIND_CAPACITY, Buffs } from '../../src/combat/schools/buffs.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { RandomStreams, STREAM_COUNT, STREAM_NAMES } from '../../src/core/rng.js';
import { Combat } from '../../src/core/combat.js';
import {
  bossById,
  BOSSES,
  CALCINATION,
  FIRST_CRUCIBLE,
  parseSchool,
  PLAYER,
} from '../../src/core/data.js';
import type { Recording, RecordedPress } from '../../src/devtools/recording.js';
import { recordings, replayRecording } from '../../src/devtools/recording.js';
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
  const steps: { event: number; subject: number; value: number; timeMs: number }[] = [];
  const after: number[] = [];

  const emit = (event: number, subject: number, value: number, timeMs: number): void => {
    bus.emit(event, subject, value, timeMs);
    steps.push({ event, subject, value, timeMs });
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
  const slotOf = new Map<number, number>();
  for (const binding of STEP1_BINDINGS) slotOf.set(combat.indexOf(binding.spellId), binding.slot);

  const presses: RecordedPress[] = [];
  // Drückt einen Zauber, ob er darf oder nicht — und schreibt den Druck mit.
  const press = (spell: number, tick: number, timeMs: number): void => {
    combat.cast(spell, timeMs);
    presses.push({ tick, slot: slotOf.get(spell) ?? 0 });
  };

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
      const outcome = combat.cast(spell, timeMs);
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
  for (let slot = 0; slot < 3; slot++) session.bind(slot, slot);
  session.startFight(0);

  const steps: Record<string, unknown>[] = [];
  const druecke = (slot: number, timeMs: number, was: string): void => {
    queue.push(slot, timeMs / TICK_MS, 0, 0);
    session.tick(timeMs);

    let hash = FNV_OFFSET;
    hash = mixInt(hash, timeMs);
    for (let i = 0; i < combat.spells.length; i++) {
      hash = mixFloat(hash, combat.cooldownRemainingMs(i, timeMs));
      hash = mixInt(hash, combat.castCount(i));
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
