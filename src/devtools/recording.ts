/**
 * Aufzeichnen und Abspielen (GAME-DESIGN 19.1, 18.4).
 *
 * Eine Aufzeichnung ist eine Liste aus **Takt und Belegung**, dazu der Startwert
 * der Zufallsströme — mehr braucht ein Kampf nicht, um sich exakt zu wiederholen.
 * Weder Zeitstempel des Browsers noch Bildzahlen gehören hinein: Die hängen am
 * Gerät, und genau davon soll die Aufzeichnung frei sein.
 *
 * Sie ist damit zweierlei (GAME-DESIGN 18.4):
 *
 * 1. **Regressionstest** — dieselbe Runde muss immer denselben Schaden ergeben
 * 2. **Abgleich mit dem Simulator** — beide Seiten spielen dieselbe Aufzeichnung
 *
 * Die Prüffälle sind ausdrücklich **aufgezeichnete echte Durchläufe**, nicht von
 * Hand geschriebene Listen. Die eingebaute Aufzeichnung `cooldown-precision`
 * entsteht deshalb, indem die Referenz-Prioritätenliste einmal durchgespielt und
 * dabei mitgeschrieben wird — nicht, indem jemand Takte abtippt.
 */
import { Combat } from '../core/combat.js';
import { combatChecksum } from '../core/checksum.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../core/data.js';
import { GCD_MS } from '../core/gcd.js';
import { TICK_MS } from '../core/loop.js';
import { STEP1_BINDINGS } from '../core/loadout.js';
import { NOTHING_WORN, type WornValues } from '../gear/worn.js';

/** Ein Tastendruck: in welchem Takt, auf welchen Platz. */
export interface RecordedPress {
  readonly tick: number;
  readonly slot: number;
}

export interface Recording {
  readonly name: string;
  /** Startwert der Zufallsströme. In Schritt 1 zieht niemand daraus. */
  readonly seed: number;
  /** Dauer in Takten. */
  readonly ticks: number;
  readonly presses: readonly RecordedPress[];
}

export interface ReplayResult {
  readonly name: string;
  readonly ticks: number;
  readonly presses: number;
  readonly accepted: number;
  readonly rejected: number;
  readonly damage: number;
  /** Kampfzeit des Sieges in ms, oder -1. */
  readonly killedAtMs: number;
  readonly casts: Readonly<Record<string, number>>;
  /** Prüfsumme des Endzustands. */
  readonly finalChecksum: number;
  /** Prüfsumme über die ganze Folge — ein Wert für den schnellen Vergleich. */
  readonly sequenceChecksum: number;
  /**
   * Prüfsumme über die Folge der **Regel-Ereignisse** (Issue E3-1b).
   *
   * Steht neben der Zustands-Prüfsumme, nicht darin: Die Feldfolge von
   * `combatChecksum` bleibt damit unberührt, und die Parität aus #54 gilt
   * weiter. Beide Größen zusammen sagen, ob zwei Umsetzungen denselben
   * Zustand **und** denselben Ablauf haben.
   */
  readonly ruleSequenceChecksum: number;
  /** Wie viele Regel-Ereignisse dabei ausgetragen wurden. */
  readonly ruleEventCount: number;
  /** Prüfsumme je Takt. Nur gefüllt, wenn ausdrücklich verlangt. */
  readonly checksums: number[];
  readonly randomDraws: number;
}

/** Belegung → Zauber-Kennung, wie in `loadout.ts` festgelegt. */
function spellForSlot(combat: Combat, slot: number): number {
  for (const binding of STEP1_BINDINGS) {
    if (binding.slot === slot) return combat.indexOf(binding.spellId);
  }
  return -1;
}

/**
 * Eine Aufzeichnung abspielen — **ohne Browser, ohne Uhr, ohne Bild**.
 *
 * Damit ist das Ergebnis von Bildrate und Systemlast unabhängig; zwei Aufrufe
 * liefern byteweise dasselbe.
 */
export function replayRecording(
  recording: Recording,
  options: {
    readonly checksumPerTick?: boolean;
    /**
     * Gegen welchen Boss abgespielt wird. Fehlt er, ist es der erste
     * (`FIRST_CRUCIBLE`) — so bleibt jeder bestehende Aufruf unverändert.
     *
     * Wählbar seit Issue #96: Der Simulator kann es über `FightConfig.boss`
     * schon länger, und ein gemeinsamer Prüffall gegen einen Boss **mit**
     * Phasen und Energieleiste braucht dieselbe Möglichkeit auf dieser Seite.
     */
    readonly boss?: BossDef;
    /**
     * Die getragenen Sonderwerte (GAME-DESIGN 6.7, Issue #177).
     *
     * Fehlt der Wert, wird nichts getragen — jeder bestehende Aufruf bleibt
     * damit unverändert, und genau das verlangt AC7.
     *
     * Wählbar, weil AC8 eine Prüfsumme je Takt über einen Lauf **mit**
     * Ausrüstung fordert. Ohne diesen Schalter ließe sich die Bitgleichheit
     * nur für den Fall belegen, in dem die neuen Werte gar nichts tun.
     */
    readonly worn?: WornValues;
  } = {},
): ReplayResult {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: options.boss ?? FIRST_CRUCIBLE.value,
    seed: recording.seed,
    worn: options.worn ?? NOTHING_WORN,
  });
  const wantChecksums = options.checksumPerTick === true;
  const checksums: number[] = [];
  let sequence = 0;

  let accepted = 0;
  let rejected = 0;
  let killedAtMs = -1;
  let pressIndex = 0;

  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (killedAtMs < 0 && combat.damageDealt >= combat.boss.hp) {
      killedAtMs = timeMs;
      // Abbrechen wie im Spiel: Der Kampf endet mit dem Sieg, und der Schaden
      // bleibt damit mit der Referenz-Rechnung vergleichbar.
      const hash = combatChecksum(combat, timeMs);
      sequence = (sequence * 31 + hash) >>> 0;
      if (wantChecksums) checksums.push(hash);
      break;
    }

    while (pressIndex < recording.presses.length) {
      const press = recording.presses[pressIndex];
      if (press === undefined || press.tick > tick) break;
      pressIndex++;
      const spellIndex = spellForSlot(combat, press.slot);
      if (spellIndex < 0) {
        rejected++;
        continue;
      }
      if (combat.cast(spellIndex, timeMs).result === 0) accepted++;
      else rejected++;
    }

    const hash = combatChecksum(combat, timeMs);
    sequence = (sequence * 31 + hash) >>> 0;
    if (wantChecksums) checksums.push(hash);
  }

  const casts: Record<string, number> = {};
  for (let i = 0; i < combat.spells.length; i++) {
    casts[combat.spells[i]?.id ?? String(i)] = combat.castCount(i);
  }

  return {
    name: recording.name,
    ticks: recording.ticks,
    presses: recording.presses.length,
    accepted,
    rejected,
    damage: combat.damageDealt,
    killedAtMs,
    casts,
    finalChecksum: combatChecksum(
      combat,
      killedAtMs >= 0 ? killedAtMs : (recording.ticks - 1) * TICK_MS,
    ),
    sequenceChecksum: sequence,
    ruleSequenceChecksum: combat.events.sequenceChecksum,
    ruleEventCount: combat.events.emittedCount,
    checksums,
    randomDraws: combat.random.totalDraws,
  };
}

/**
 * Die gute Rotation einmal durchspielen und dabei mitschreiben.
 *
 * Ergibt eine **echte** Aufzeichnung statt einer abgetippten Liste — die
 * Bedingung aus GAME-DESIGN 18.4. Die Prioritätenliste ist dieselbe wie in
 * `reference-run.ts` und wie in `tools/step1-check.py`.
 */
function recordGoodRotation(name: string, ticks: number, boss?: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: boss ?? FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const smoulder = combat.indexOf('smoulder');
  const pyre = combat.indexOf('pyre');
  const slotOf = new Map<number, number>();
  for (const binding of STEP1_BINDINGS) slotOf.set(combat.indexOf(binding.spellId), binding.slot);

  const presses: RecordedPress[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    let choice = -1;
    if (combat.dotRemainingMs(smoulder, timeMs) < GCD_MS && combat.isReady(smoulder, timeMs)) {
      choice = smoulder;
    } else if (
      combat.isReady(pyre, timeMs) &&
      combat.resource >= (combat.spellAt(pyre)?.spender?.bonusThreshold ?? 0)
    ) {
      choice = pyre;
    } else if (combat.isReady(kindle, timeMs)) {
      choice = kindle;
    }
    if (choice < 0) continue;
    combat.cast(choice, timeMs);
    presses.push({ tick, slot: slotOf.get(choice) ?? 0 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Eine Runde, die vor allem die Wischblenden bewegt.
 *
 * Für die Animationsprüfung (VERIFICATION § 4): PYRE wird gezündet, sobald es
 * bereit ist — fünf Sekunden Blende, die längste im Spiel —, dazwischen KINDLE.
 * Ob dabei viel Schaden entsteht, ist gleichgültig; sichtbare Bewegung zählt.
 */
function recordCooldownSweep(name: string, ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const kindle = combat.indexOf('kindle');
  const pyre = combat.indexOf('pyre');
  const slotOf = new Map<number, number>();
  for (const binding of STEP1_BINDINGS) slotOf.set(combat.indexOf(binding.spellId), binding.slot);

  const presses: RecordedPress[] = [];
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (!combat.gcd.isReady(timeMs)) continue;
    const choice = combat.isReady(pyre, timeMs)
      ? pyre
      : combat.isReady(kindle, timeMs)
        ? kindle
        : -1;
    if (choice < 0) continue;
    combat.cast(choice, timeMs);
    presses.push({ tick, slot: slotOf.get(choice) ?? 0 });
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die eingebauten Aufzeichnungen.
 *
 * Erzeugt beim ersten Zugriff, nicht beim Laden: Wer die Werkzeuge nicht
 * benutzt, zahlt nichts dafür.
 */
let builtIn: Map<string, Recording> | null = null;

export function recordings(): Map<string, Recording> {
  if (builtIn === null) {
    builtIn = new Map<string, Recording>();
    // 60 s Kampfdauer des Testbosses, in Takten.
    builtIn.set('cooldown-precision', recordGoodRotation('cooldown-precision', 6000));
    builtIn.set('cooldown-sweep', recordCooldownSweep('cooldown-sweep', 2000));
    // Der gemeinsame Pruefall gegen einen Boss **mit** Phasen, Energieleiste und
    // Zeitleiste (Issue #96, AC4). 90 s Ueberdruck des Testgefaesses, in Takten.
    //
    // Dieselbe gute Rotation wie oben, nur gegen einen anderen Boss: Alles, was
    // sich zwischen beiden Aufzeichnungen unterscheidet, kommt damit aus den
    // Boss-Daten und aus nichts sonst.
    const vessel = bossById('test-vessel');
    if (vessel !== undefined) {
      builtIn.set('vessel-phases', recordGoodRotation('vessel-phases', 9000, vessel));
    }
    // Der Pruefall gegen Zaehler und exponentiellen Stapel (Issue #216).
    // 30 s in Takten — der Zaehler toetet bei 20 s, und die zehn Sekunden
    // danach gehoeren dazu: Sie belegen, dass beide Seiten nach dem Tod
    // dasselbe tun (naemlich nichts mehr).
    const doomsayer = bossById('test-doomsayer');
    if (doomsayer !== undefined) {
      builtIn.set('doom-stacks', recordGoodRotation('doom-stacks', 3000, doomsayer));
    }
  }
  return builtIn;
}

export function recordingNames(): string[] {
  return [...recordings().keys()];
}
