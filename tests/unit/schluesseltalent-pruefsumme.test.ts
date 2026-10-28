/**
 * Der Zustand der Schlüsseltalente steht in der Prüfsumme (Issue #419, #418 AC5).
 *
 * # Was fehlte
 *
 * Zwei der fünf Schlüsseltalente legen Kampfzustand an, der zwischen zwei
 * Takten lebt:
 *
 * ```text
 *   decay-echo     (PUTREFACTION)  decayEchoUntilMs   bis wann ein erloschener Brand nachläuft
 *   shield-refund  (COAGULATION)   shieldRefundUsed   ob die Schicht ihre Nachlage schon bekam
 *                                  shieldLastAmount   wovon die Nachlage die Hälfte gibt
 * ```
 *
 * Keines der drei Felder stand in `combatChecksum` (und keines in `checksum_at`
 * auf der Simulator-Seite). Rechnete eine Seite eines davon anders, sähe der
 * Abgleich es erst, wenn die Folge im Schaden oder im Schild ankommt — Takte
 * später, an einer Stelle, die nicht mehr auf die Ursache zeigt. Oder nie, wenn
 * die Runde vorher endet.
 *
 * # Wie gemessen wird
 *
 * Wie in `checksum-effectiveness.test.ts`: an **einem** Zustand mit **einer**
 * gezielten Störung. Zwei gleiche Kämpfe, in einem wird genau ein Feld
 * verstellt. Zwei Läufe mit verschiedenen Eingaben verschöben die Summe auch
 * dann, wenn das Feld gar nicht eingeht.
 *
 * **Zu jeder Störung die positive Kontrolle:** Der verstellte Kampf läuft
 * danach messbar anders. Ohne sie bewiese eine gleiche Prüfsumme nur, dass das
 * Feld bedeutungslos ist — und dann gehörte es auch nicht hinein.
 *
 * # Nur bei verteiltem Talent
 *
 * Ohne das Talent liest niemand die drei Felder; `merkeDecayEcho` und das
 * Legen eines Schilds schreiben sie trotzdem. Gingen sie dann ein, änderte
 * sich die Prüfsumme jedes Kampfes mit einem Brand oder einer Schicht, und jede
 * eingecheckte Aufzeichnung wäre falsch. Der letzte Block hält fest, dass sie
 * ohne Talent draußen bleiben — und dass sie dort auch nichts bewirken.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, isCastAccepted } from '../../src/core/combat.js';
import { combatChecksum, recordFieldOrder } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById, type BossDef } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Die privaten Felder, um die es geht — nur für diesen Prüffall geöffnet.
 *
 * Über die Felder selbst und nicht über Getter: Der Fall muss VOR der Behebung
 * laufen und rot werden, und dann gibt es noch keinen Getter.
 */
interface Schluesselzustand {
  decayEchoUntilMs: Float64Array;
  shieldRefundUsed: boolean;
  shieldLastAmount: number;
}

function innen(combat: Combat): Schluesselzustand {
  return combat as unknown as Schluesselzustand;
}

/** Spielbare Verteilungen: jeder Weg mit seinem Stamm (`allocationProblem`). */
const NACHLAUF = {
  'putrefaction-trunk': 1,
  'putrefaction-duration': 1,
  'putrefaction-keystone': 1,
};
const NACHLAGE = {
  'coagulation-trunk': 1,
  'coagulation-thicker': 1,
  'coagulation-keystone': 1,
};

function bossOder(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

/**
 * Lässt den Kampf Takt für Takt bis `bisMs` laufen und wirft, was `plan` sagt.
 *
 * `vonMs` erlaubt, einen Kampf nach der Messung weiterzuführen — für die
 * positive Kontrolle.
 */
function bis(
  combat: Combat,
  bisMs: number,
  plan: ReadonlyMap<number, string> = new Map(),
  vonMs = 0,
): void {
  for (let t = vonMs; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    const id = plan.get(t);
    if (id === undefined) continue;
    const ergebnis = combat.cast(alsSpellIndex(combat.indexOf(id)), t).result;
    if (!isCastAccepted(ergebnis))
      throw new Error(`${id} bei ${String(t)} ms abgewiesen (${String(ergebnis)})`);
  }
}

// ---------------------------------------------------------------------------
// decay-echo
// ---------------------------------------------------------------------------

/** ROT bei 0 ms; bei 6000 ms ist der Brand aus und der Nachlauf läuft. */
const MESSUNG_NACHLAUF_MS = 6000;

function nachlaufKampf(talents: Record<string, number> = NACHLAUF): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, talents });
  bis(combat, MESSUNG_NACHLAUF_MS, new Map([[0, 'rot']]));
  return combat;
}

describe('decay-echo: der Nachlauf steht in der Prüfsumme', () => {
  it('die Messung trifft einen laufenden Nachlauf', () => {
    // **Die Zusicherung vor der Messung.** Steht der Endpunkt auf
    // `NOT_ACTIVE`, verstellte die Störung unten einen Ruhewert.
    const combat = nachlaufKampf();
    const rot = combat.indexOf('rot');
    expect(combat.isDotActive(rot, MESSUNG_NACHLAUF_MS), 'ROT brennt noch').toBe(false);
    expect(combat.decayStacks, 'ohne Stapel gäbe es nichts nachzuhalten').toBeGreaterThan(0);
    expect(innen(combat).decayEchoUntilMs[rot], 'kein Nachlauf vorgemerkt').toBeGreaterThan(
      MESSUNG_NACHLAUF_MS,
    );
  });

  it('ein verstellter Endpunkt ändert die Prüfsumme', () => {
    const echt = nachlaufKampf();
    const verstellt = nachlaufKampf();
    const rot = echt.indexOf('rot');
    // Der Nachlauf endet damit bei 7000 statt bei 16 000 ms.
    innen(verstellt).decayEchoUntilMs[rot] = 7000;

    expect(
      combatChecksum(verstellt, MESSUNG_NACHLAUF_MS),
      'zwei Kämpfe mit verschiedenem Nachlauf ergeben dieselbe Prüfsumme — ' +
        'decayEchoUntilMs geht nicht in combatChecksum ein',
    ).not.toBe(combatChecksum(echt, MESSUNG_NACHLAUF_MS));

    // **Die positive Kontrolle:** Der Unterschied ist einer, der später zählt.
    bis(echt, 16_000, new Map(), MESSUNG_NACHLAUF_MS + TICK_MS);
    bis(verstellt, 16_000, new Map(), MESSUNG_NACHLAUF_MS + TICK_MS);
    expect(verstellt.decayStacks, 'der verstellte Nachlauf wirkt gar nicht anders').not.toBe(
      echt.decayStacks,
    );
  });
});

// ---------------------------------------------------------------------------
// shield-refund
// ---------------------------------------------------------------------------

/**
 * CRUST bei 0 ms gegen `test-onslaught`; bei 1000 ms liegt die Schicht, der
 * erste Schlag (3000 ms Beginn + 2000 ms Wirkzeit) kommt erst bei 5000 ms.
 */
const MESSUNG_NACHLAGE_MS = 1000;
const ERSTER_SCHLAG_MS = 5000;

function nachlageKampf(talents: Record<string, number> = NACHLAGE): Combat {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: bossOder('test-onslaught'),
    talents,
  });
  bis(combat, MESSUNG_NACHLAGE_MS, new Map([[0, 'crust']]));
  return combat;
}

describe('shield-refund: die Nachlage steht in der Prüfsumme', () => {
  it('die Messung trifft eine liegende, unverbrauchte Schicht', () => {
    const combat = nachlageKampf();
    expect(combat.shield, 'CRUST legt keine Schicht').toBeGreaterThan(0);
    expect(innen(combat).shieldRefundUsed, 'die Nachlage ist schon verbraucht').toBe(false);
    expect(innen(combat).shieldLastAmount, 'die Dicke ist nicht gemerkt').toBe(combat.shield);
  });

  it('ein verbrauchter Merker ändert die Prüfsumme', () => {
    const echt = nachlageKampf();
    const verstellt = nachlageKampf();
    innen(verstellt).shieldRefundUsed = true;

    expect(
      combatChecksum(verstellt, MESSUNG_NACHLAGE_MS),
      'zwei Kämpfe, einer mit verbrauchter Nachlage, ergeben dieselbe Prüfsumme — ' +
        'shieldRefundUsed geht nicht in combatChecksum ein',
    ).not.toBe(combatChecksum(echt, MESSUNG_NACHLAGE_MS));

    // Positive Kontrolle: Nach dem Bruch legt nur der echte nach.
    bis(echt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    bis(verstellt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    expect(echt.shield, 'der Schlag hat die Schicht nicht gebrochen').toBeGreaterThan(0);
    expect(verstellt.shield, 'der verbrauchte Merker legt trotzdem nach').toBe(0);
  });

  it('eine verstellte Dicke ändert die Prüfsumme', () => {
    const echt = nachlageKampf();
    const verstellt = nachlageKampf();
    innen(verstellt).shieldLastAmount = 400;

    expect(
      combatChecksum(verstellt, MESSUNG_NACHLAGE_MS),
      'zwei Kämpfe mit verschiedener Nachlage-Grundlage ergeben dieselbe Prüfsumme — ' +
        'shieldLastAmount geht nicht in combatChecksum ein',
    ).not.toBe(combatChecksum(echt, MESSUNG_NACHLAGE_MS));

    // Positive Kontrolle: Die Nachlage ist die Hälfte dessen, was gemerkt war.
    bis(echt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    bis(verstellt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    expect(verstellt.shield).toBe(200);
    expect(echt.shield).not.toBe(verstellt.shield);
  });
});

// ---------------------------------------------------------------------------
// Die Felder tragen ihre Namen (#236)
// ---------------------------------------------------------------------------

describe('Die neuen Felder tragen ihre Namen', () => {
  it('mit beiden Talenten stehen sie in der Namensfolge, angehängt', () => {
    // **Am Ende, nicht dazwischen** (#96): Ein Einschub verschöbe jede
    // Position danach, und der Gleichlauf-Wächter zeigte auf die falsche
    // Stelle. Die Folge ohne Talent muss deshalb ein ANFANG der Folge mit
    // Talent sein.
    const beide = { ...NACHLAUF, ...NACHLAGE };
    const mit = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: beide,
    });
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });

    const folgeMit = recordFieldOrder(() => combatChecksum(mit, 0));
    const folgeOhne = recordFieldOrder(() => combatChecksum(ohne, 0));

    expect(folgeMit.slice(0, folgeOhne.length)).toEqual(folgeOhne);
    expect(folgeMit.slice(folgeOhne.length)).toEqual([
      ...Array.from({ length: mit.spells.length }, () => 'decayEchoUntil'),
      'shieldRefundUsed',
      'shieldLastAmount',
    ]);
  });
});

// ---------------------------------------------------------------------------
// Ohne Talent bleibt alles, wie es war
// ---------------------------------------------------------------------------

describe('Ohne Schlüsseltalent gehen die Felder nicht ein — und bewirken nichts', () => {
  it('die Namensfolge ist ohne Talent dieselbe wie mit einem Talent ohne Zustand', () => {
    // CALCINATIONs Schlüsseltalent (split-spender) legt keinen Zustand an
    // (`keystone-split-spender.test.ts`, AC5). Mit ihm darf die Folge
    // deshalb nicht länger werden.
    const ohne = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const geteilt = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: { 'calcination-trunk': 1, 'calcination-release': 1, 'calcination-keystone': 1 },
    });
    expect(recordFieldOrder(() => combatChecksum(geteilt, 0))).toEqual(
      recordFieldOrder(() => combatChecksum(ohne, 0)),
    );
  });

  it('ein verstellter Nachlauf ohne Talent ändert weder Summe noch Verlauf', () => {
    // **Der Grund, warum nur mit Talent gemischt wird.** Ohne Talent schreibt
    // `merkeDecayEcho` den Endpunkt trotzdem, aber `imDecayEcho` liest ihn
    // nie. Ginge er ein, änderte sich die Prüfsumme jedes Kampfes mit ROT.
    const echt = nachlaufKampf({});
    const verstellt = nachlaufKampf({});
    const rot = echt.indexOf('rot');
    expect(
      innen(echt).decayEchoUntilMs[rot],
      'der Endpunkt wird ohne Talent nicht geschrieben',
    ).toBeGreaterThan(0);
    innen(verstellt).decayEchoUntilMs[rot] = 7000;

    expect(combatChecksum(verstellt, MESSUNG_NACHLAUF_MS)).toBe(
      combatChecksum(echt, MESSUNG_NACHLAUF_MS),
    );
    bis(echt, 16_000, new Map(), MESSUNG_NACHLAUF_MS + TICK_MS);
    bis(verstellt, 16_000, new Map(), MESSUNG_NACHLAUF_MS + TICK_MS);
    expect(verstellt.decayStacks).toBe(echt.decayStacks);
    expect(combatChecksum(verstellt, 16_000)).toBe(combatChecksum(echt, 16_000));
  });

  it('eine verstellte Nachlage ohne Talent ändert weder Summe noch Verlauf', () => {
    const echt = nachlageKampf({});
    const verstellt = nachlageKampf({});
    innen(verstellt).shieldRefundUsed = true;
    innen(verstellt).shieldLastAmount = 400;

    expect(combatChecksum(verstellt, MESSUNG_NACHLAGE_MS)).toBe(
      combatChecksum(echt, MESSUNG_NACHLAGE_MS),
    );
    bis(echt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    bis(verstellt, ERSTER_SCHLAG_MS, new Map(), MESSUNG_NACHLAGE_MS + TICK_MS);
    expect(echt.shield, 'ohne Talent darf nach dem Bruch nichts nachkommen').toBe(0);
    expect(verstellt.shield).toBe(0);
    expect(combatChecksum(verstellt, ERSTER_SCHLAG_MS)).toBe(
      combatChecksum(echt, ERSTER_SCHLAG_MS),
    );
  });
});
