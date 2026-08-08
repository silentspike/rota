/**
 * Die Kontroll-Wirkungen bewegen wirklich etwas (Issue #667).
 *
 * # Der Befund: nicht die Zuordnung fehlte, sondern die Messung
 *
 * `stay` und `latch` tragen beide `delayStrike` — sie verzögern den nächsten
 * Zeitleisten-Eintrag des Bosses. Beide werden in Aufzeichnungen **gedrückt**,
 * beide stehen dort mit `casts: 1`. Trotzdem war ihre Wirkung ungemessen:
 *
 * ```text
 *   stay.control.amountMs   3000 -> 100    0 echte Fälle rot
 *   latch.control.amountMs  4000 -> 100    0 echte Fälle rot
 * ```
 *
 * Eine Wirkung von drei Sekunden auf ein Zehntel zu senken ist keine
 * Feinheit. Dass nichts färbte, heißt: Die Zauber wurden geworfen, aber
 * niemand sah hin — dieselbe Lage, die der Kopf von `recordBuffTargets` für
 * GRIPLOCK und VIRULENCE beschreibt.
 *
 * # Warum der WURFZEITPUNKT zur Lage gehört
 *
 * `delayStrike` verschiebt **nur den nächsten** Eintrag. Wer den Zauber vor
 * dem ersten Eintrag wirft, trifft diesen — und dann ist die Zahl nicht mehr
 * der reine Betrag, weil die Wirkzeit des Boss-Zaubers dazukommt. Gemessen:
 *
 * ```text
 *   ohne                        Schläge bei 5500,  8000
 *   latch, Wurf bei 1500        Schläge bei 7010,  8000     <- trifft den ERSTEN
 *   latch, Wurf bei 4500        Schläge bei 5500, 12000     <- +4000, sauber
 * ```
 *
 * `latch` kostet 2 Punkte, `stay` kostet 3. Eine Rotation „wirf, sobald die
 * Punkte reichen" misst bei beiden deshalb **Verschiedenes** — allein wegen
 * des Preises. Beide Fälle unten warten deshalb ausdrücklich, bis der erste
 * Schlag gefallen ist.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** `boss-1-1` trägt eine Zeitleiste bei 3000, 8000 und 13000 ms. */
const BOSS = 'boss-1-1';

type Lauf = {
  /** Wann der Kontroll-Zauber angenommen wurde, oder -1. */
  wurfBei: number;
  /** Die Zeitpunkte, an denen der Spieler Schaden genommen hat. */
  schlaege: number[];
};

/**
 * Fährt einen Lauf und verzögert wahlweise mit einem Kontroll-Zauber.
 *
 * **Der Wurf wartet auf den ersten Schlag.** Ohne das trifft ein billiger
 * Zauber wie LATCH den ersten Eintrag statt den zweiten, und die Messung
 * vergleicht zwei verschiedene Dinge (siehe Kopf).
 */
function lauf(verzoegerer: string | null, kosten: number): Lauf {
  const boss = bossById(BOSS) ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const zauber = verzoegerer === null ? -1 : alsSpellIndex(combat.indexOf(verzoegerer));

  let wurfBei = -1;
  let letzterSchaden = 0;
  const schlaege: number[] = [];

  for (let tick = 0; tick < 2000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    const ersterGefallen = schlaege.length > 0;
    if (
      zauber >= 0 &&
      wurfBei < 0 &&
      ersterGefallen &&
      combat.control >= kosten &&
      combat.isReady(alsSpellIndex(zauber), timeMs)
    ) {
      if (combat.cast(alsSpellIndex(zauber), timeMs).result === CAST_OK) wurfBei = timeMs;
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }

    if (combat.damageTaken > letzterSchaden) {
      schlaege.push(timeMs);
      letzterSchaden = combat.damageTaken;
    }
  }
  return { wurfBei, schlaege };
}

describe('AC1 — die Lage misst überhaupt etwas', () => {
  it('ohne Verzögerer fallen zwei Schläge, zu festen Zeiten', () => {
    // **Die Zusicherung vor jeder Messung.** Fiele hier kein zweiter Schlag,
    // verglichen die Fälle darunter eine Zahl mit `undefined` — und wären
    // grün, ohne etwas zu zeigen.
    const ohne = lauf(null, 0);
    expect(ohne.schlaege.length, 'der Boss schlägt gar nicht zu').toBeGreaterThanOrEqual(2);
    expect(ohne.schlaege[0], 'der erste Schlag liegt anders als beim Festnageln').toBe(5500);
    expect(ohne.schlaege[1], 'der zweite Schlag liegt anders als beim Festnageln').toBe(8000);
  });
});

describe('AC2 — STAY verzögert den nächsten Schlag um seinen vollen Betrag', () => {
  it('der zweite Schlag rückt von 8000 auf 11000 ms', () => {
    const mit = lauf('stay', 3);

    // **Die zweite Zahl, ohne die der Vergleich nichts sagt:** Der Zauber muss
    // überhaupt angenommen worden sein. Genau diese Zeile hat beim Messen
    // einen falschen Befund verhindert — ein zu enger Zeitfilter ließ STAY nie
    // zum Zug kommen, und beide Läufe zeigten dieselbe Zahl.
    expect(mit.wurfBei, 'STAY wurde nie geworfen — dann misst der Fall nichts').toBeGreaterThan(0);

    // 3000 ms ist `stay.control.amountMs`. Die Zahl steht HART und nicht als
    // `> 8000`: Eine Richtung bliebe grün, wenn der Betrag auf 100 ms fiele.
    expect(mit.schlaege[1], 'STAY verzögert nicht um seine vollen 3000 ms').toBe(11000);
  });

  it('und der ERSTE Schlag bleibt, wo er war', () => {
    // **Der Beleg für „nur der nächste Eintrag"** (der Kommentar an der
    // Rechenstelle). Verschöbe sich auch der erste, träfe die Wirkung mehr als
    // sie soll — und der Wurf läge vor dem falschen Eintrag.
    const mit = lauf('stay', 3);
    expect(mit.schlaege[0], 'der erste Schlag hat sich mitbewegt').toBe(5500);
  });
});

describe('AC3 — LATCH ebenso, mit seinem eigenen Betrag', () => {
  it('der zweite Schlag rückt von 8000 auf 12000 ms', () => {
    const mit = lauf('latch', 2);
    expect(mit.wurfBei, 'LATCH wurde nie geworfen').toBeGreaterThan(0);

    // 4000 statt 3000 — die beiden Zauber tragen dieselbe Wirkung mit
    // verschiedenem Betrag. Stünde hier dieselbe Zahl wie bei STAY, prüfte
    // einer der beiden Fälle den anderen mit.
    expect(mit.schlaege[1], 'LATCH verzögert nicht um seine vollen 4000 ms').toBe(12000);
  });

  it('die beiden Beträge sind wirklich verschieden — sonst prüft ein Fall zwei Mal dasselbe', () => {
    const mitStay = lauf('stay', 3);
    const mitLatch = lauf('latch', 2);
    expect(mitLatch.schlaege[1]).not.toBe(mitStay.schlaege[1]);
    expect(
      (mitLatch.schlaege[1] ?? 0) - (mitStay.schlaege[1] ?? 0),
      'der Unterschied ist nicht die Differenz der beiden amountMs (4000 - 3000)',
    ).toBe(1000);
  });
});
