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
import { RandomStreams, STREAM_COUNT, STREAM_NAMES } from '../../src/core/rng.js';
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

/** So werden die Dateien geschrieben — an einer Stelle festgelegt. */
export function serialize(payload: unknown): string {
  return `${JSON.stringify(payload, null, 2)}\n`;
}
