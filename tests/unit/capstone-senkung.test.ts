/**
 * CAPSTONE senkt die Phasen-Schwelle, statt den Wechsel abzuschalten
 * (Issue #478).
 *
 * # Der Befund, der dazu geführt hat
 *
 * `allThree` rechnete **eine** Stärke aus und gab sie an alle drei Wirkungen
 * weiter. `slowCast` und `delayStrike` rechnen in Millisekunden, `holdPhase`
 * in Prozentpunkten — CAPSTONEs 2000 landeten damit im Prozent-Feld.
 *
 * Gemessen an `test-phase`, wann der Wechsel fiel:
 *
 * ```text
 *   ohne Zauber    6010 ms
 *   ANCHOR (10 %)  9010 ms   der Boss muss 10 % tiefer fallen
 *   CAPSTONE      14010 ms   genau PHASE_HOLD_DURATION_MS nach dem Wurf
 * ```
 *
 * 2000 Prozentpunkte sind keine Senkung, sondern eine **Abschaltung**: Der
 * Wechsel fiel erst, als die Frist ablief. Und **jede** Zahl über 100 hätte
 * dasselbe getan — eine Halbierung von 2000 auf 1000 wäre an dieser Wirkung
 * unsichtbar geblieben.
 *
 * Der Plan verlangt das Gegenteil: „Die Phasen-Schwelle SINKT vorübergehend",
 * „alle kaufen einen einzelnen Moment, keinen Zustand". Bei vollständiger
 * Blockade muss man den Moment nicht **treffen**, man muss nur irgendwann
 * drücken.
 *
 * # Was gebaut wurde
 *
 * `capstone` trägt jetzt beide Einheiten, und der Parser führt sie getrennt:
 *
 * ```text
 *   amountMs      2000   slowCast und delayStrike
 *   amountPercent   10   holdPhase
 * ```
 *
 * Zehn Prozentpunkte sind **gleich stark wie beim Spezialisten ANCHOR**, aber
 * für 8 Punkte statt 6. CAPSTONEs Wert ist die Breite, nicht die Tiefe — wäre
 * er zusätzlich stärker, wäre ANCHOR tot.
 */
import { describe, expect, it } from 'vitest';

import { CAST_OK, Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Kosten der beiden Zauber — sie bestimmen, wie lange gesammelt wird. */
const KOSTEN: Record<string, number> = { anchor: 6, capstone: 8 };

interface Lauf {
  readonly wurfBei: number;
  /** `phaseHoldPercent` unmittelbar nach dem Wurf. */
  readonly senkung: number;
  /** Wann der Boss die Phase wechselte. */
  readonly wechselBei: number;
}

/** Sammelt Punkte und wirft `werfe`, sobald sie reichen. */
function gegenPhasenboss(werfe: string | null): Lauf {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: bossById('test-phase') ?? FIRST_CRUCIBLE.value,
  });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const ember = alsSpellIndex(combat.indexOf('ember'));
  const ziel = werfe === null ? alsSpellIndex(0) : alsSpellIndex(combat.indexOf(werfe));
  const noetig = werfe === null ? 0 : (KOSTEN[werfe] ?? 6);

  let wurfBei = -1;
  let senkung = -1;
  let wechselBei = -1;

  for (let tick = 0; tick < 2000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (werfe !== null && wurfBei < 0 && combat.control >= noetig && combat.isReady(ziel, timeMs)) {
      if (combat.cast(ziel, timeMs).result === CAST_OK) {
        wurfBei = timeMs;
        senkung = combat.phaseHoldPercent;
      }
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    } else if (ember >= 0 && combat.isReady(ember, timeMs)) {
      combat.cast(ember, timeMs);
    }

    if (combat.bossPhase > 0 && wechselBei < 0) wechselBei = timeMs;
  }
  return { wurfBei, senkung, wechselBei };
}

const OHNE = gegenPhasenboss(null);
const ANCHOR = gegenPhasenboss('anchor');
const CAPSTONE = gegenPhasenboss('capstone');

describe('die Messanordnung trägt', () => {
  it('ohne Zauber wechselt der Boss bei 6010 ms', () => {
    // Der Bezugspunkt. Verschiebt sich `test-phase`, gehören die Zahlen
    // darunter nachgezogen — dieser Fall sagt dann, warum.
    expect(OHNE.wechselBei, 'der Phasenwechsel fällt woanders').toBe(6010);
  });

  it('und beide Zauber kommen zustande', () => {
    expect(ANCHOR.wurfBei, 'ANCHOR kam nicht zustande').toBeGreaterThan(0);
    expect(CAPSTONE.wurfBei, 'CAPSTONE kam nicht zustande').toBeGreaterThan(0);
  });
});

describe('CAPSTONE senkt um zehn Prozentpunkte (control.amountPercent)', () => {
  it('phaseHoldPercent steht auf 10, nicht auf 2000', () => {
    // **Der Fänger.** Halbiert man `amountPercent` auf 5, steht hier 5.
    expect(CAPSTONE.senkung, 'CAPSTONE senkt nicht um 10 Prozentpunkte').toBe(10);
  });

  it('und der Wechsel fällt SPÄTER als ohne, aber nicht erst nach der Frist', () => {
    // **Die eigentliche Aussage.** Vorher fiel er bei 14010 — genau
    // `PHASE_HOLD_DURATION_MS` nach dem Wurf, also erst beim Ablaufen der
    // Frist. Das war eine Abschaltung. Jetzt fällt er bei 9010: gesenkt,
    // nicht abgeschaltet.
    expect(CAPSTONE.wechselBei, 'der Wechsel fällt wie bei vollständiger Blockade').toBe(9010);
    expect(CAPSTONE.wechselBei).toBeGreaterThan(OHNE.wechselBei);
  });
});

describe('ANCHOR und CAPSTONE teilen sich die Frist', () => {
  it('gleiche Senkung, gleicher Wechsel-Zeitpunkt', () => {
    // **Gemessen, nicht im Code verglichen.** Der Plan: „Die Frist gilt für
    // ALLE ZAUBER GLEICH — sie gehört zur Wirkung, nicht zum einzelnen
    // Zauber; sonst hieße ‚hinauszögern' bei jedem etwas anderes."
    //
    // Beide werfen zu verschiedenen Zeiten (ANCHOR früher, weil er 6 statt 8
    // Punkte kostet) und erreichen trotzdem denselben Wechsel-Zeitpunkt: Die
    // Senkung entscheidet, nicht der Wurfzeitpunkt.
    expect(ANCHOR.senkung).toBe(10);
    expect(CAPSTONE.senkung).toBe(ANCHOR.senkung);
    expect(CAPSTONE.wechselBei).toBe(ANCHOR.wechselBei);
  });

  it('und sie werfen wirklich zu verschiedenen Zeiten', () => {
    // Ohne diese Zeile könnte der Vergleich oben zwei identische Läufe
    // vergleichen — dann sagte er nichts über die Frist.
    expect(ANCHOR.wurfBei).not.toBe(CAPSTONE.wurfBei);
  });
});

describe('KREUZPROBE: amountMs trägt weiterhin die Millisekunden-Wirkungen', () => {
  /** Erster Boss-Schlag — `delayStrike` verschiebt ihn um `amountMs`. */
  function ersterTreffer(werfe: string | null, stand: number): number {
    const combat = new Combat({
      school: CALCINATION.value,
      boss: bossById('test-caster') ?? FIRST_CRUCIBLE.value,
    });
    const bind = alsSpellIndex(combat.indexOf('bind'));
    const ziel = werfe === null ? alsSpellIndex(0) : alsSpellIndex(combat.indexOf(werfe));
    let wurfBei = -1;
    let treffer = -1;
    let hpVor = combat.playerHp;

    for (let tick = 0; tick < 2000; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      if (
        werfe !== null &&
        wurfBei < 0 &&
        combat.control === stand &&
        combat.isReady(ziel, timeMs)
      ) {
        if (combat.cast(ziel, timeMs).result === CAST_OK) wurfBei = timeMs;
      } else if (wurfBei < 0 && combat.isReady(bind, timeMs)) {
        combat.cast(bind, timeMs);
      }
      if (combat.playerHp < hpVor) {
        if (treffer < 0) treffer = timeMs;
        hpVor = combat.playerHp;
      }
    }
    return treffer;
  }

  it('CAPSTONE verzögert den Schlag um 2000 ms — aus amountMs, nicht aus amountPercent', () => {
    // **Die zwei Felder dürfen sich nicht gegenseitig bewegen.** Läse
    // `delayStrike` versehentlich `amountPercent`, käme hier 8510 statt
    // 10500 heraus.
    expect(ersterTreffer(null, 0), 'der Boss schlägt nicht bei 8500 zu').toBe(8500);
    expect(ersterTreffer('capstone', 8), 'CAPSTONE verzögert nicht um 2000 ms').toBe(10500);
  });

  it('und LATCH um seine eigenen 4000 — die Millisekunden werden je Zauber gelesen', () => {
    expect(ersterTreffer('latch', 6)).toBe(12500);
  });
});

describe('die Phase kann nur STEIGEN — eine Senkung danach holt sie nicht zurück', () => {
  /** Wirft `werfe` erst NACH dem Wechsel. */
  function nachDemWechsel(werfe: string): { wechselBei: number; wurfBei: number; phase: number } {
    const combat = new Combat({
      school: CALCINATION.value,
      boss: bossById('test-phase') ?? FIRST_CRUCIBLE.value,
    });
    const bind = alsSpellIndex(combat.indexOf('bind'));
    const ember = alsSpellIndex(combat.indexOf('ember'));
    const ziel = alsSpellIndex(combat.indexOf(werfe));
    const noetig = KOSTEN[werfe] ?? 6;

    let wechselBei = -1;
    let wurfBei = -1;

    for (let tick = 0; tick < 2500; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      if (combat.bossPhase > 0 && wechselBei < 0) wechselBei = timeMs;

      if (
        wechselBei >= 0 &&
        wurfBei < 0 &&
        combat.control >= noetig &&
        combat.isReady(ziel, timeMs)
      ) {
        if (combat.cast(ziel, timeMs).result === CAST_OK) wurfBei = timeMs;
      } else if (combat.isReady(bind, timeMs)) {
        combat.cast(bind, timeMs);
      } else if (ember >= 0 && combat.isReady(ember, timeMs)) {
        combat.cast(ember, timeMs);
      }
    }
    return { wechselBei, wurfBei, phase: combat.bossPhase };
  }

  it('CAPSTONE nach dem Wechsel lässt die Phase stehen', () => {
    // Die Regel stand laut Design seit dem 2026-08-04 im Code und war
    // ungeprüft, weil das abgefangene Ereignis gar nicht vorkommen konnte:
    // Bei vollständiger Blockade wechselte der Boss erst NACH der Frist, ein
    // Wurf danach traf auf keine laufende Senkung mehr.
    const { wechselBei, wurfBei, phase } = nachDemWechsel('capstone');
    expect(wechselBei, 'ohne Wechsel prüft der Fall nichts').toBeGreaterThan(0);
    expect(wurfBei, 'der Wurf kam nicht NACH dem Wechsel zustande').toBeGreaterThan(wechselBei);
    expect(phase, 'die Phase ist zurückgefallen').toBe(1);
  });

  it('und ANCHOR ebenso', () => {
    const { wurfBei, wechselBei, phase } = nachDemWechsel('anchor');
    expect(wurfBei).toBeGreaterThan(wechselBei);
    expect(phase).toBe(1);
  });
});
