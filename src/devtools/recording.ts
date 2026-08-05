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
import { PressQueue } from '../input/queue.js';
import { Session } from '../core/session.js';
import { PHASE_FIGHTING } from '../core/phases.js';
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
 * Eine Runde, die den **Auswahlmodus** benutzt (Issue #223).
 *
 * Sie ist der einzige Weg, den Ruecksetzer in einer Aufzeichnung zu haben —
 * und Aufzeichnungen entstehen aus einem echten Lauf, nicht aus einer
 * abgetippten Druckliste (GAME-DESIGN 18.4).
 *
 * **Ueber eine `Session`, nicht ueber `Combat`.** Der Auswahlmodus lebt in der
 * Sitzung: Sie entscheidet, ob ein Druck wirkt oder waehlt. Ein Rekorder, der
 * `combat.cast` ruft, wuerde REFLUX als gewoehnlichen Zauber wirken — und die
 * Aufzeichnung enthielte den Modus nie.
 *
 * Die Abfolge ist bewusst schlicht und wiederholt sich: PYRE wirken (er hat
 * die laengste Abklingzeit der drei), dann REFLUX, dann PYRE waehlen — er ist
 * damit sofort wieder bereit. Dazwischen KINDLE als Fueller.
 */
function recordResetRotation(name: string, ticks: number): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const queue = new PressQueue();
  const session = new Session(queue, combat);
  const belegung = [
    { slot: 0, id: 'kindle' },
    { slot: 1, id: 'pyre' },
    { slot: 2, id: 'reflux' },
  ];
  for (const { slot, id } of belegung) session.bind(slot, combat.indexOf(id));
  const kindle = combat.indexOf('kindle');
  const pyre = combat.indexOf('pyre');
  const reflux = combat.indexOf('reflux');

  const presses: RecordedPress[] = [];
  // **`startFight` statt `beginCountdown`.** Der Simulator kennt keinen
  // Countdown: `Fight::replay` beginnt bei Takt 0 mit der Kampfuhr. Mit
  // Countdown liefe die Kampfuhr hier drei Sekunden spaeter los, und die
  // Zustaende waeren von Anfang an verschieden — die Paritaet scheiterte an
  // der Aufzeichnung, nicht an der Rechnung. Derselbe Weg wie in
  // `cooldownResetPayload` (E3-3c).
  session.startFight(0);
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    // Der Druck wird VOR dem Takt eingereiht — die Sitzung arbeitet ihn in
    // diesem Takt ab, genau wie im Spiel.
    let slot = -1;
    if (session.phase === PHASE_FIGHTING && combat.gcd.isReady(timeMs)) {
      if (session.isResetArmed) {
        // Steht der Modus scharf, ist der naechste Druck die WAHL. PYRE liegt
        // dann auf Abklingzeit — genau darauf soll sie fallen.
        slot = 1;
      } else if (combat.isReady(reflux, timeMs) && !combat.isOffCooldown(pyre, timeMs)) {
        slot = 2;
      } else if (combat.isReady(pyre, timeMs)) {
        slot = 1;
      } else if (combat.isReady(kindle, timeMs)) {
        slot = 0;
      }
    }
    if (slot >= 0) {
      queue.push(slot, tick, timeMs, tick);
      presses.push({ tick, slot });
    }
    session.tick(timeMs);
  }
  return { name, seed: 0, ticks, presses };
}

/**
 * Die beiden letzten Bausteine in EINEM Lauf (Issue #240, AC6).
 *
 * **Mit der Zauberleiste aus Schritt 1**, ohne eigene Belegung. Sie war einen
 * Entwurf lang noetig, weil CRUST dort keinen Platz hat und die Reaktion sonst
 * nie gehoben worden waere; mit dem Herausschneiden von Baustein 6 ist der
 * Grund entfallen — und ein Sonderweg ohne Grund ist einer zu viel. QUENCH
 * steht auf Platz 3 und entfernt die Schwaechung.
 *
 * Die Abfolge ist bewusst schlicht: QUENCH, sobald die Schwaechung liegt,
 * sonst SMOULDER und KINDLE als Fueller. Damit steht neben den passiven
 * Haelften (Verkuerzen, Schaden) auch die Handlung im Lauf, mit der der
 * Spieler dem Baustein begegnet.
 */
function recordTrialRotation(name: string, ticks: number, boss: BossDef): Recording {
  const combat = new Combat({ school: CALCINATION.value, boss });
  const index = new Map(STEP1_BINDINGS.map(({ slot, spellId }) => [slot, combat.indexOf(spellId)]));

  const presses: RecordedPress[] = [];
  // **Kein `beginFight`** — die Wiedergabe (`replayRecording`) ruft es
  // ebenfalls nicht, und Aufnahme und Wiedergabe müssen denselben Zustand
  // erzeugen. Was `beginFight` an Uhren setzt, setzt der Konstruktor schon.
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp) break;
    if (!combat.gcd.isReady(timeMs)) continue;

    let slot = -1;
    for (const kandidat of [3, 1, 0]) {
      const i = index.get(kandidat) ?? -1;
      if (i < 0 || !combat.isReady(i, timeMs)) continue;
      // Platz 3 nur, wenn es etwas zu entfernen gibt — sonst waere der GCD
      // verschenkt (#224).
      if (kandidat === 3 && !combat.afflicted) continue;
      if (kandidat === 1 && combat.dotRemainingMs(i, timeMs) >= GCD_MS) continue;
      slot = kandidat;
      break;
    }
    if (slot >= 0 && combat.cast(index.get(slot) ?? -1, timeMs)) {
      presses.push({ tick, slot });
    }
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
    // Der Prueffall zum Auswahlmodus (Issue #223). 60 s in Takten: REFLUX hat
    // 45 s Abklingzeit, ein kuerzerer Lauf zeigte ihn nur EINMAL. Gemessen —
    // bei 20 s stand genau ein Druck auf Platz 2 in der Aufzeichnung.
    builtIn.set('cooldown-reset-run', recordResetRotation('cooldown-reset-run', 6000));
    // Der Prueffall zum Wettrennen und zur Boss-Heilung (Issue #218). 40 s in
    // Takten: Das Fenster liegt bei 10 s, der Boss-Zauber bei 30 s und trifft
    // nach 3 s — beide Bausteine kommen also in EINEM Lauf vor.
    const racer = bossById('test-racer');
    if (racer !== undefined) {
      builtIn.set('race-heal', recordGoodRotation('race-heal', 4000, racer));
    }
    // Der Prueffall zu den letzten beiden Bausteinen (Issue #240, AC6). 90 s
    // in Takten, und die Laenge ist gerechnet, nicht geschaetzt: Die
    // Schwaechung greift bei 8 s, die Zeitleiste wiederholt sich alle 12 s
    // (nach 90 s sind sieben Zyklen durch, und die Wirkzeit steht laengst auf
    // ihrer Untergrenze). BEIDE Bausteine kommen damit in EINEM Lauf vor —
    // getrennt geprueft zeigte sich nicht, dass sie einander nicht ins Gehege
    // kommen.
    //
    // Baustein 6 stand hier bis zum 2026-08-05 mit dabei und ist herausgenommen:
    // Der Boss-Ring IST die Reaktion (GAME-DESIGN 9.1), ein zweiter Wert daneben
    // waere eine Dublette gewesen. Er bekommt ein eigenes Issue.
    const trial = bossById('test-crucible-trial');
    if (trial !== undefined) {
      builtIn.set('trial-blocks', recordTrialRotation('trial-blocks', 9000, trial));
    }
  }
  return builtIn;
}

export function recordingNames(): string[] {
  return [...recordings().keys()];
}
