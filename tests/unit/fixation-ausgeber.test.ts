/**
 * ANCHOR, CAPSTONE, LATCH und WEDGE — die vier restlichen Kontroll-Größen
 * (Issue #478).
 *
 * # Vier Lücken, drei verschiedene Bauarten
 *
 * `tools/groessen-mutation.py`, Vollmessung aller 116 Paare:
 *
 * ```text
 *   control.amountPercent  anchor    10 -> 5      NUR FRISCHE
 *   control.amountMs       capstone  2000 -> 1000 NUR FRISCHE
 *   control.cost           latch     2 -> 1       NUR FRISCHE
 *   control.cost           wedge     3 -> 1       SCHWEIGT
 * ```
 *
 * # Warum WEDGE schwieg — und was das über `cost` verrät
 *
 * `SCHWEIGT` heißt: Nicht einmal die Vergleichsdatei färbte. Der Grund steht
 * im Spielcode: `convertToDamage` leert **alle** Punkte, nicht nur die Kosten.
 * Der Gesamtverbrauch ist damit unabhängig von `cost` — 3 plus Rest oder 1
 * plus zwei mehr Rest ergibt dieselbe Summe.
 *
 * `cost` wirkt bei WEDGE also **nur als Schwelle**: ab wann der Zauber
 * überhaupt greift. Diese Schwelle ist messbar, aber nicht dort, wo man sie
 * sucht — der Wurf selbst gelingt (`CAST_OK`, der GCD läuft), nur die Wirkung
 * bleibt aus und `controlFailed` zählt hoch.
 *
 * **Ein Fall, der nur den Rückgabewert von `cast` prüft, sieht davon nichts.**
 * Deshalb messen die Fälle unten `controlFailed` und den Punktestand, nicht
 * das Ergebnis des Wurfs.
 *
 * # Der Vorrat ist der Träger
 *
 * Alle vier Läufe sammeln mit BIND und werfen bei einem **genau bestimmten**
 * Stand. Das ist kein Detail: Wer wirft, sobald der Zauber bereit ist, hat
 * meist zu viele Punkte — und dann ist jede Kostenmutation unsichtbar, weil
 * es reicht, egal wie teuer es ist.
 *
 * ```text
 *   Stand 2   WEDGE (cost 3)     scheitert   ← mit cost 1 gelänge er
 *   Stand 4   WEDGE              greift
 *   Stand 6   LATCH (cost 2)     6 -> 4
 *   Stand 6   ANCHOR (cost 6)    greift, phaseHoldPercent 10
 *   Stand 8   CAPSTONE (cost 8)  greift, phaseHoldPercent 2000
 * ```
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

interface Lauf {
  readonly wurfBei: number;
  readonly vor: number;
  readonly nach: number;
  readonly gescheitert: number;
  readonly phaseHold: number;
  /** Wann der Boss zum ersten Mal zuschlägt — der Beleg für `delayStrike`. */
  readonly ersterTreffer: number;
}

/**
 * Sammelt mit BIND und wirft `werfe` beim **exakten** Stand `beiStand`.
 *
 * `=== beiStand`, nicht `>=`: Beim Warten auf „mindestens" läuft der Vorrat
 * über den gesuchten Wert hinaus, und die Schwelle wäre nicht mehr knapp.
 */
function beiStand(werfe: string, stand: number): Lauf {
  const boss = bossById('test-caster') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const ziel = alsSpellIndex(combat.indexOf(werfe));

  let wurfBei = -1;
  let vor = -1;
  let nach = -1;
  let phaseHold = -1;
  let ersterTreffer = -1;
  let hpVor = combat.playerHp;

  for (let tick = 0; tick < 2000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (wurfBei < 0 && combat.control === stand && combat.isReady(ziel, timeMs)) {
      vor = combat.control;
      if (combat.cast(ziel, timeMs).result === CAST_OK) {
        wurfBei = timeMs;
        nach = combat.control;
        phaseHold = combat.phaseHoldPercent;
      }
    } else if (wurfBei < 0 && combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }

    if (combat.playerHp < hpVor) {
      if (ersterTreffer < 0) ersterTreffer = timeMs;
      hpVor = combat.playerHp;
    }
  }
  return { wurfBei, vor, nach, gescheitert: combat.controlFailed, phaseHold, ersterTreffer };
}

const WEDGE_KNAPP = beiStand('wedge', 2);
const WEDGE_REICHT = beiStand('wedge', 4);
const LATCH = beiStand('latch', 6);
const ANCHOR = beiStand('anchor', 6);
const CAPSTONE = beiStand('capstone', 8);

describe('die Träger tragen — sonst misst nichts darunter etwas', () => {
  it('jeder der vier Würfe kommt zustande', () => {
    // Ein nie geworfener Zauber sähe wie ein wirkungsloser aus.
    for (const [name, lauf] of [
      ['wedge@2', WEDGE_KNAPP],
      ['wedge@4', WEDGE_REICHT],
      ['latch', LATCH],
      ['anchor', ANCHOR],
      ['capstone', CAPSTONE],
    ] as const) {
      expect(lauf.wurfBei, `${name} kam nicht zustande`).toBeGreaterThan(0);
    }
  });

  it('und jeder trifft den gesuchten Vorrat genau', () => {
    expect(WEDGE_KNAPP.vor).toBe(2);
    expect(WEDGE_REICHT.vor).toBe(4);
    expect(LATCH.vor).toBe(6);
    expect(ANCHOR.vor).toBe(6);
    expect(CAPSTONE.vor).toBe(8);
  });
});

describe('WEDGEs Kosten sind eine Schwelle, kein Abzug (control.cost)', () => {
  it('bei zwei Punkten greift er nicht — drei sind gefordert', () => {
    expect(WEDGE_KNAPP.gescheitert, 'der Wurf greift trotz zu weniger Punkte').toBe(1);
  });

  it('und der Vorrat bleibt dabei unberührt', () => {
    // Der Zauber ist geworfen (der GCD läuft), aber die Buchhaltung steigt
    // vorher aus. Ein Abzug hier wäre ein stiller Verlust.
    expect(WEDGE_KNAPP.nach).toBe(2);
  });

  it('GEGENPROBE: bei vier Punkten greift er und leert ALLES', () => {
    // Ohne diesen Fall bewiese der obige nichts — er wäre auch dann grün,
    // wenn WEDGE grundsätzlich nie greift. Und die 0 zeigt die zweite
    // Eigenschaft: `cost` ist der Mindesteinsatz, der Rest geht mit.
    expect(WEDGE_REICHT.gescheitert).toBe(0);
    expect(WEDGE_REICHT.nach, 'der Rest bleibt liegen — WEDGE nimmt alles').toBe(0);
  });
});

describe('LATCH zieht genau zwei Punkte ab (control.cost)', () => {
  it('sechs werden zu vier', () => {
    expect(LATCH.nach, 'LATCH kostet nicht zwei Punkte').toBe(4);
    expect(LATCH.vor - LATCH.nach).toBe(2);
  });

  it('und er greift überhaupt — sonst wäre die Differenz zufällig', () => {
    expect(LATCH.gescheitert).toBe(0);
  });
});

describe('ANCHOR senkt die Phasenschwelle um zehn Prozent (control.amountPercent)', () => {
  it('phaseHoldPercent steht auf 10', () => {
    expect(ANCHOR.phaseHold, 'die Senkung ist nicht 10 %').toBe(10);
  });

  it('die Zusicherung: ohne Wurf steht dort 0', () => {
    // `phaseHoldPercent` ist auch der Ruhewert 0 — ein Fall, der nur „nicht
    // undefined" prüfte, wäre wertlos.
    expect(WEDGE_REICHT.phaseHold).toBe(0);
  });
});

describe('CAPSTONE trägt seine 2000 in ALLE drei Wirkungen (control.amountMs)', () => {
  it('der nächste Boss-Schlag kommt 2000 ms später: 8500 wird zu 10500', () => {
    // **Das ist der Messpunkt, der die Größe wirklich prüft.** Er misst die
    // WIRKUNG (`delayStrike`), nicht den gesetzten Wert — halbiert man die
    // 2000, steht hier 9500.
    expect(WEDGE_REICHT.ersterTreffer, 'der Boss schlägt nicht bei 8500 zu').toBe(8500);
    expect(CAPSTONE.ersterTreffer, 'der Schlag wurde nicht um 2000 ms verzögert').toBe(10500);
    expect(CAPSTONE.ersterTreffer - WEDGE_REICHT.ersterTreffer).toBe(2000);
  });

  it('dieselbe 2000 landet auch im Prozent-Feld der Phasen-Senkung', () => {
    // **Keine Prozentzahl, und ein Nebeneffekt der geteilten Stärke**:
    // `allThree` rechnet EINE Stärke aus und gibt sie an alle Wirkungen
    // weiter. Für `slowCast` und `delayStrike` sind das Millisekunden; bei
    // `holdPhase` landen dieselben 2000 in einem Prozent-Feld.
    //
    // Gemessen an `test-phase`: Der Wechsel fällt ohne Zauber bei 6010 ms,
    // mit ANCHOR (10 %) bei 9010 — und mit CAPSTONE bei 14010, also genau
    // `PHASE_HOLD_DURATION_MS` nach dem Wurf. Jede Zahl über 100 blockiert
    // dort gleich vollständig; **eine Halbierung wäre an DIESER Wirkung
    // unsichtbar.** Deshalb steht der Fall darüber und nicht dieser für die
    // Größe ein.
    expect(CAPSTONE.phaseHold, 'CAPSTONEs Stärke erreicht holdPhase nicht').toBe(2000);
  });

  it('und er verbraucht alle Punkte, nicht nur die acht', () => {
    expect(CAPSTONE.nach).toBe(0);
    expect(CAPSTONE.gescheitert).toBe(0);
  });
});
