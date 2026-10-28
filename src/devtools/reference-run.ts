/**
 * Durchrechnen einer Runde nach einer festen Prioritätenliste — **Werkzeug, kein
 * Spielinhalt** (GAME-DESIGN 19.1).
 *
 * Steht hinter `__ROTA_DEVTOOLS__` und fällt aus der Auslieferung heraus. Das ist
 * nicht nur Sparsamkeit: GAME-DESIGN 18.5 verlangt ausdrücklich, dass die
 * Referenz-Rotationen **nirgends im Spiel** erscheinen, auch nicht als Lernhilfe.
 * Das Herausfinden der besten Reihenfolge ist der Kern des Spiels.
 *
 * Wozu es hier gebraucht wird: Der Abgleich mit `tools/step1-check.py`. Beide
 * Seiten müssen bei derselben Prioritätenliste denselben Schaden ergeben, sonst
 * stimmt eine Regel nicht (GAME-DESIGN 18.4).
 *
 * Die beiden Listen sind wortgleich zu `apl_good` und `apl_bad` der Referenz.
 */
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../core/data.js';
import { Combat, FIGHT_RUNNING, type SpellIndex, alsSpellIndex } from '../core/combat.js';
import type { WornValues } from '../gear/worn.js';
import { GCD_MS } from '../core/gcd.js';
import { TICK_MS } from '../core/loop.js';

/** Nichts wirken. */
const IDLE = -1;

export type RotationName = 'good' | 'bad' | 'dot-error' | 'pyre-error';

export interface ReferenceResult {
  readonly rotation: RotationName;
  /**
   * Gegen welchen Boss gerechnet wurde (Issue #338).
   *
   * **Die Zahl allein ist mehrdeutig.** Die Referenz `4240 @ 40510` gilt für
   * `first-crucible`; ein Lauf gegen einen anderen Boss liefert eine Zahl,
   * die mit ihr nichts zu tun hat. Steht der Boss daneben, ist das keine
   * Verwechslungsgefahr mehr, sondern eine Angabe.
   */
  readonly bossId: string;
  readonly damage: number;
  /** Kampfzeit des Sieges in ms, oder -1, wenn der Boss überlebt hat. */
  readonly killedAtMs: number;
  readonly remainingMs: number;
  readonly casts: Readonly<Record<string, number>>;
  readonly damageBySpell: Readonly<Record<string, number>>;
  readonly lostDotTicks: number;
  readonly resourceOverflow: number;
  readonly idleMs: number;
  readonly bossHpFraction: number;
  /** Ziehungen aus den Zufallsströmen. In Schritt 1 muss das 0 sein. */
  readonly randomDraws: number;
}

interface Indices {
  readonly kindle: SpellIndex;
  readonly smoulder: SpellIndex;
  readonly pyre: SpellIndex;
}

/**
 * Gute Rotation: nutzt beide Entscheidungen, die Schritt 1 anbietet.
 *
 * 1. SMOULDER erst erneuern, wenn er ausläuft — keine Sekunde früher
 * 2. PYRE erst ab der Bonus-Schwelle zünden
 * 3. sonst KINDLE als Füller
 */
function aplGood(combat: Combat, index: Indices, timeMs: number): number {
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < GCD_MS &&
    combat.isReady(index.smoulder, timeMs)
  ) {
    return index.smoulder;
  }
  const spender = combat.spellAt(index.pyre)?.spender;
  const threshold = spender?.bonusThreshold ?? 0;
  if (combat.isReady(index.pyre, timeMs) && combat.resource >= threshold) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Schlechte Rotation: plausibel falsch, nicht absichtlich dumm. */
function aplBad(combat: Combat, index: Indices, timeMs: number): number {
  const half = Math.floor((combat.spellAt(index.smoulder)?.dot?.durationMs ?? 0) / 2);
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < half &&
    combat.isReady(index.smoulder, timeMs)
  )
    return index.smoulder;
  if (combat.isReady(index.pyre, timeMs) && combat.resource > 0) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Nur der DoT-Fehler: erneuert zu früh, zündet PYRE aber richtig. */
function aplDotError(combat: Combat, index: Indices, timeMs: number): number {
  const half = Math.floor((combat.spellAt(index.smoulder)?.dot?.durationMs ?? 0) / 2);
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < half &&
    combat.isReady(index.smoulder, timeMs)
  )
    return index.smoulder;
  const threshold = combat.spellAt(index.pyre)?.spender?.bonusThreshold ?? 0;
  if (combat.isReady(index.pyre, timeMs) && combat.resource >= threshold) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

/** Nur der PYRE-Fehler: DoT richtig, aber jede Zündung zu früh. */
function aplPyreError(combat: Combat, index: Indices, timeMs: number): number {
  if (
    combat.dotRemainingMs(index.smoulder, timeMs) < GCD_MS &&
    combat.isReady(index.smoulder, timeMs)
  ) {
    return index.smoulder;
  }
  if (combat.isReady(index.pyre, timeMs) && combat.resource > 0) return index.pyre;
  if (combat.isReady(index.kindle, timeMs)) return index.kindle;
  return IDLE;
}

const ROTATIONS: Record<RotationName, (c: Combat, i: Indices, t: number) => number> = {
  good: aplGood,
  bad: aplBad,
  'dot-error': aplDotError,
  'pyre-error': aplPyreError,
};

export interface ReferenceOptions {
  /**
   * Gegen welchen Boss gerechnet wird. Ohne Angabe `first-crucible`.
   *
   * **Er fehlte bis Issue #338** — und wurde trotzdem übergeben: Im Browser
   * gibt es kein TypeScript, ein unbekanntes Feld kommt an und wird nie
   * gelesen. Drei Aufrufe mit drei Bossen ergaben dreimal dieselbe Zahl, und
   * das sah aus wie ein Befund über die Widerstände.
   */
  readonly bossId?: string;
  /** Kampfdauer. Ohne Angabe der Überdruck-Zeitpunkt des Bosses. */
  readonly durationMs?: number;
  /**
   * Beim Sieg abbrechen. `false` rechnet die vollen Sekunden durch — so werden
   * zwei Rotationen über dieselbe Zeit verglichen (STEP1-SPEC § 4).
   */
  readonly stopOnKill?: boolean;
  /**
   * Die getragene Ausrüstung. **Ohne Angabe bleibt der Läufer nackt** — genau
   * wie vor Issue #791, Zeichen für Zeichen.
   *
   * # Warum ein zweiter Weg und kein geänderter erster
   *
   * GAME-DESIGN 20.5 verlangt, Bosse gegen den erwarteten Ausrüstungsstand
   * ihrer Instanz zu kalibrieren statt gegen einen nackten Spieler. Der
   * naheliegende Schritt wäre, hier fest Ausrüstung einzusetzen — und er wäre
   * falsch: Der nackte Lauf ist ein **fester Bezugspunkt für Vergleiche**.
   * Zwei Rotationen gegeneinander zu stellen, verlangt, dass sich zwischen den
   * Läufen nur die Rotation ändert.
   *
   * Deshalb ist das hier ein Zusatz, kein Ersatz. Wer nichts übergibt, bekommt
   * die Zahlen von gestern; wer kalibriert, übergibt den Stand ausdrücklich
   * und sieht am Aufrufer, gegen was gerechnet wurde.
   */
  readonly worn?: WornValues;
}

/** Was die Schleife unter einer Prioritätenliste ausser dem Kampf selbst hinterlässt. */
export interface DriveOutcome {
  /** Kampfzeit des Sieges in ms, oder -1, wenn der Boss überlebt hat. */
  readonly killedAtMs: number;
  /** Je Takt, in dem die globale Wartezeit bereit war und nichts gewirkt wurde, 10 ms. */
  readonly idleMs: number;
}

/**
 * Wie viele Würfe die Schleife beim **selben** Zeitstempel höchstens zulässt
 * (Issue #1027, Teil c).
 *
 * Nach einem Wurf entscheidet die Liste beim selben Zeitstempel erneut (siehe
 * {@link driveRotation}). Das endet, sobald sie nichts mehr wählt — und das
 * kommt, weil jeder Wurf etwas verbraucht: Ein Zauber mit Wartezeit sperrt alle
 * anderen mit Wartezeit, jeder Zauber seine eigene Abklingzeit. Ohne Rücksetzer
 * wirkt also jeder Zauber höchstens einmal je Zeitstempel, und die Zahl der
 * Zauber ist eine Grenze, die kein gültiger Datensatz erreicht.
 *
 * **Wer sie erreicht, hat einen Nullzeit-Zyklus gebaut** — etwa einen Zauber
 * ohne Wartezeit, der nach dem Wurf bereit bleibt. Das ist ein Datenfehler und
 * wird abgelehnt, nicht endlos gerechnet (`docs/SIM-ARCHITEKTUR-PLAN.md`,
 * „Das Optimum-Orakel"). Der Simulator führt dieselbe Grenze
 * (`NULLZEIT_BUDGET` in `sim/src/fight.rs`, `SPELL_COUNT`); die Aufzeichnung
 * `gcd-freier-wurf.json` trägt sie, damit beide Seiten sie gegeneinander
 * prüfen.
 */
export function nullzeitBudget(combat: Combat): number {
  return combat.spells.length;
}

/**
 * Ob `Combat.cast` diesen Zauber jetzt annähme: bereit, und der Kampf läuft
 * noch. Dieselben Prüfungen wie dort, ohne zu wirken.
 */
function nimmtAn(combat: Combat, spell: SpellIndex, timeMs: number): boolean {
  return combat.isReady(spell, timeMs) && combat.fightState === FIGHT_RUNNING && !combat.playerDead;
}

/**
 * Die Schleife hinter {@link runReference}, für eine beliebige Prioritätenliste
 * (Issue #1027).
 *
 * # Wann entschieden wird
 *
 * **In jedem Takt, und nach jedem Wurf beim selben Zeitstempel erneut, bis die
 * Liste nichts mehr wählt.** Das ist das Entscheidungsmodell, das der
 * Orchestrator am 2026-10-28 an #1027 festgelegt hat, und es ist dasselbe, was
 * das Spiel einem Spieler erlaubt: `Session.tick` arbeitet alle Drücke eines
 * Takts ab, und ein Zauber ohne Wartezeit wird von keiner laufenden gesperrt.
 *
 * | | |
 * | --- | --- |
 * | (a) | nach einem Wurf ohne Wartezeit (QUENCH, oder der Wurf, den SUBLIME befreit) wählt die Liste beim selben Zeitstempel weiter |
 * | (b) | während einer laufenden Wartezeit kommt ein Zauber ohne Wartezeit zum Zug — auch im Takt des Wurfs, der sie ausgelöst hat |
 * | (c) | mehr als {@link nullzeitBudget} Würfe beim selben Zeitstempel sind ein Nullzeit-Zyklus und brechen ab |
 *
 * Bis dahin entschied die Schleife je Takt einmal und nur bei bereiter
 * Wartezeit: nach QUENCH erst im nächsten Takt, und QUENCH nie während eines
 * GCD. Ohne Zauber ohne Wartezeit ändert sich nichts — dann ist während einer
 * laufenden Wartezeit kein Zauber bereit, und nach einem Wurf mit Wartezeit
 * auch nicht.
 *
 * **Ein gewählter Zauber, den der Kampf nicht annähme, beendet die Entscheidung
 * beim Zeitstempel** — er ist nicht bereit, oder der Kampf ist vorbei
 * (`Combat.cast` meldet dann `CAST_FIGHT_OVER`, während `isReady` den Zauber
 * weiter bereit nennt). Der Druck bliebe ohne Wirkung, und ihn erneut zu wählen
 * änderte nichts: Ohne diese Regel liefe die Schleife nach dem Kampfende in
 * einen Nullzeit-Zyklus — gemessen an `boss-1-2`, wo die gute Rotation den
 * Spieler bei 18000 über das Verhängnis verliert.
 *
 * # Leerlauf
 *
 * Je Takt, in dem die Wartezeit bereit war und die Liste **nichts wählte**,
 * 10 ms — wie bisher. Ein gewählter, aber abgewiesener Zauber ist kein leerer
 * Takt, ebenso wenig ein Takt mitten in einer Wartezeit.
 *
 * **Eine eigene Funktion, damit die Aufzeichnung DIESE Schleife fährt** und
 * keinen Nachbau. Bis #1027 stand sie nur in `runReference`, fest verdrahtet mit
 * den vier Rotationen aus Schritt 1 — eine Aufzeichnung mit QUENCH hätte sie
 * abschreiben müssen, und eine Abschrift ist genau die zweite Quelle, die
 * plausibel antwortet und nichts belegt. Der Simulator muss sie bitgleich
 * nachbilden; geprüft wird das an `tests/parity/recordings/gcd-freier-wurf.json`
 * und `strikter-umschlag.json`.
 *
 * @param choose Der gewählte Zauber-Index, oder -1 für warten.
 * @param cast Wirkt den gewählten Zauber; die Aufzeichnung schreibt hier mit.
 */
export function driveRotation(
  combat: Combat,
  choose: (timeMs: number) => number,
  durationMs: number,
  stopOnKill: boolean,
  cast: (spell: SpellIndex, timeMs: number) => void,
): DriveOutcome {
  let killedAtMs = -1;
  let idleMs = 0;
  let timeMs = 0;
  const budget = nullzeitBudget(combat);

  while (timeMs < durationMs) {
    combat.stepPassive(timeMs);
    if (killedAtMs < 0 && combat.damageDealt >= combat.boss.hp) {
      killedAtMs = timeMs;
      if (stopOnKill) break;
    }
    const gcdBereit = combat.gcd.isReady(timeMs);
    let gewaehlt = false;
    let wuerfe = 0;
    for (;;) {
      const choice = choose(timeMs);
      if (choice === IDLE) break;
      gewaehlt = true;
      const spell = alsSpellIndex(choice);
      if (!nimmtAn(combat, spell, timeMs)) break;
      if (wuerfe === budget) {
        throw new Error(
          `Nullzeit-Zyklus bei ${String(timeMs)} ms: ${String(wuerfe)} Würfe beim selben ` +
            `Zeitstempel, und die Liste wählt weiter (${combat.spells[spell]?.id ?? '?'}). ` +
            `Das ist ein Datenfehler — ein Zauber bleibt nach seinem Wurf bereit.`,
        );
      }
      cast(spell, timeMs);
      wuerfe++;
    }
    if (!gewaehlt && gcdBereit) idleMs += TICK_MS;
    timeMs += TICK_MS;
  }

  return { killedAtMs, idleMs };
}

/** Eine Runde nach fester Prioritätenliste durchrechnen. */
/**
 * Die gültigen Schlüssel — **abgeleitet, nicht gepflegt** (Issue #338).
 *
 * Aus einem Objekt, das jedes Feld einmal nennt: Der Übersetzer erzwingt
 * seine Vollständigkeit (`Required<…>`), also wächst die Liste mit dem Typ
 * mit. Eine von Hand geführte Liste wäre die, die zurückbleibt — dasselbe
 * Problem wie in #346 und #371.
 */
const GUELTIGE_OPTIONEN: Required<{ [K in keyof ReferenceOptions]: true }> = {
  durationMs: true,
  worn: true,
  stopOnKill: true,
  bossId: true,
};

/**
 * Meldet ein Feld, das es nicht gibt — **mit Namen**.
 *
 * # Warum zur Laufzeit
 *
 * Die Dev-Werkzeuge sind eine **Laufzeit**-Schnittstelle: Im Browser über
 * `window.__rota` sind die Typen weg. Ein Feld, das der Typ verbietet, kommt
 * dort an und wird stillschweigend ignoriert.
 *
 * # Warum mit Namen
 *
 * „Unbekannte Option" lässt den Aufrufer suchen. Bei drei Feldern ist das
 * gleichgültig; bei einem Werkzeugkasten mit zwanzig Einträgen ist der Name
 * der Unterschied zwischen „wo?" und „da".
 */
function pruefeOptionen(options: ReferenceOptions): void {
  const unbekannt = Object.keys(options).filter((key) => !(key in GUELTIGE_OPTIONEN));
  if (unbekannt.length > 0) {
    throw new Error(
      `runReference kennt ${unbekannt.length === 1 ? 'das Feld' : 'die Felder'} ` +
        `${unbekannt.map((k) => `"${k}"`).join(', ')} nicht. ` +
        `Gueltig: ${Object.keys(GUELTIGE_OPTIONEN).join(', ')}.`,
    );
  }
}

export function runReference(
  rotation: RotationName,
  options: ReferenceOptions = {},
): ReferenceResult {
  pruefeOptionen(options);
  const bossId = options.bossId ?? FIRST_CRUCIBLE.value.id;
  const boss = bossById(bossId);
  if (boss === undefined) {
    throw new Error(`Boss "${bossId}" gibt es nicht.`);
  }
  // **Das Feld wird weggelassen, nicht auf `undefined` gesetzt.** Unter
  // `exactOptionalPropertyTypes` ist beides verschieden: `worn?: WornValues`
  // erlaubt „fehlt", aber nicht „ist undefined". Ein durchgereichtes
  // `options.worn` ist deshalb ein Übersetzungsfehler — den `vitest` nicht
  // sieht, weil es keine Typen prüft (2618 grüne Fälle, `npm run build` rot).
  const combat = new Combat({
    school: CALCINATION.value,
    boss,
    ...(options.worn !== undefined ? { worn: options.worn } : {}),
  });
  const index: Indices = {
    kindle: combat.indexOf('kindle'),
    smoulder: combat.indexOf('smoulder'),
    pyre: combat.indexOf('pyre'),
  };
  const apl = ROTATIONS[rotation];
  const durationMs = options.durationMs ?? combat.boss.overpressureMs;
  const stopOnKill = options.stopOnKill !== false;

  const { killedAtMs, idleMs } = driveRotation(
    combat,
    (timeMs) => apl(combat, index, timeMs),
    durationMs,
    stopOnKill,
    (spell, timeMs) => {
      combat.cast(spell, timeMs);
    },
  );

  const casts: Record<string, number> = {};
  const damageBySpell: Record<string, number> = {};
  let lostDotTicks = 0;
  for (let i = 0; i < combat.spells.length; i++) {
    const id = combat.spells[i]?.id ?? String(i);
    casts[id] = combat.castCount(alsSpellIndex(i));
    damageBySpell[id] = combat.damageOf(alsSpellIndex(i));
    lostDotTicks += combat.lostDotTicks(i);
  }

  return {
    rotation,
    bossId,
    damage: combat.damageDealt,
    killedAtMs,
    remainingMs: killedAtMs < 0 ? 0 : durationMs - killedAtMs,
    casts,
    damageBySpell,
    lostDotTicks,
    resourceOverflow: combat.resourceOverflow,
    idleMs,
    bossHpFraction: combat.bossHpFraction,
    randomDraws: combat.random.totalDraws,
  };
}

export interface CooldownMeasurement {
  readonly spellId: string;
  readonly cycles: number;
  /** Erwartete Gesamtzeit: Zyklen × Abklingzeit. */
  readonly expectedMs: number;
  /** Tatsächlich vergangene Zeit zwischen erster und letzter Wirkung. */
  readonly actualMs: number;
  /** Abweichung in ms. Bei ganzzahliger Zeit muss sie 0 sein. */
  readonly driftMs: number;
  /** Kleinster und größter Abstand zwischen zwei Wirkungen. */
  readonly minIntervalMs: number;
  readonly maxIntervalMs: number;
}

/**
 * Abklingzeit über viele Zyklen messen (VERIFICATION § 3.3).
 *
 * Der Zauber wird gewirkt, sobald er bereit ist, und zwar `cycles + 1` Mal. Die
 * Zeit zwischen erster und letzter Wirkung muss exakt `cycles × Abklingzeit`
 * betragen — ganzzahlige Zeit darf gar nicht driften.
 */
export function measureCooldown(spellId: string, cycles: number): CooldownMeasurement {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const index = combat.indexOf(spellId);
  const spell = combat.spellAt(index);
  if (spell === undefined) {
    return {
      spellId,
      cycles: 0,
      expectedMs: 0,
      actualMs: 0,
      driftMs: Number.NaN,
      minIntervalMs: 0,
      maxIntervalMs: 0,
    };
  }

  // Der Zauber darf den Boss nicht legen — sonst endet die Messung vorzeitig.
  const step = Math.max(spell.cooldownMs, combat.gcd.lengthMs);
  const castTimes: number[] = [];
  let timeMs = 0;
  const limitMs = (cycles + 2) * step + 1000;
  while (timeMs < limitMs && castTimes.length <= cycles) {
    combat.stepPassive(timeMs);
    if (combat.isReady(index, timeMs)) {
      combat.cast(index, timeMs);
      castTimes.push(timeMs);
    }
    timeMs += TICK_MS;
  }

  let minIntervalMs = Number.POSITIVE_INFINITY;
  let maxIntervalMs = 0;
  for (let i = 1; i < castTimes.length; i++) {
    const gap = (castTimes[i] as number) - (castTimes[i - 1] as number);
    if (gap < minIntervalMs) minIntervalMs = gap;
    if (gap > maxIntervalMs) maxIntervalMs = gap;
  }
  const measured = castTimes.length - 1;
  const actualMs = measured > 0 ? (castTimes[measured] as number) - (castTimes[0] as number) : 0;
  const expectedMs = measured * step;

  return {
    spellId,
    cycles: measured,
    expectedMs,
    actualMs,
    driftMs: actualMs - expectedMs,
    minIntervalMs: measured > 0 ? minIntervalMs : 0,
    maxIntervalMs,
  };
}

/** Nur damit der Zustand des Kampfes lesbar bleibt, wenn er nicht mehr läuft. */
export const REFERENCE_FIGHT_RUNNING = FIGHT_RUNNING;
