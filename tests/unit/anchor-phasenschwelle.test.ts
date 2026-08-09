/**
 * ANCHOR hält die Phase zurück — gemessen, nicht behauptet (Issue #667).
 *
 * # Was ungemessen war
 *
 * ANCHOR trägt `holdPhase`: Die Phasenschwelle sinkt vorübergehend, der
 * nächste Wechsel kommt später. Der Zauber wurde in Aufzeichnungen gedrückt
 * und stand dort mit `casts: 1` — seine Wirkung prüfte trotzdem niemand:
 *
 * ```text
 *   anchor.control.amountPercent  10 -> 1    0 echte Fälle rot
 * ```
 *
 * Den Wert von zehn Prozentpunkten auf einen zu senken ist keine Feinheit.
 *
 * # Warum es einen Prüf-Boss braucht
 *
 * Die Wirkung ist nur sichtbar, wenn der Boss seine Schwelle **während des
 * Fensters** erreicht. Der einzige spielbare Boss mit Phasen kann das nicht:
 *
 * ```text
 *   boss-1-2   6400 HP, Schwelle bei 65 %   ->  2240 Schaden nötig
 *   gemessen   1200 Schaden, letzte Änderung bei 16500 ms
 *              Spieler tot bei 17000 ms, playerHp = 0
 * ```
 *
 * Der Kampf ist vorbei, bevor der Boss die Schwelle sieht — deshalb ergaben
 * 60 und 200 Sekunden Lauf dieselbe Zahl. `test-phase` hat 800 HP und keine
 * Zeitleiste; die Begründung steht ausführlich in seiner Datei.
 *
 * # Der Wurfzeitpunkt gehört zur Lage
 *
 * `holdPhase` senkt die Schwelle für das laufende Fenster. Wer **nach** dem
 * Wechsel wirft, ändert nichts mehr — mein erster Versuch warf bei 6000 ms,
 * der Wechsel lag bei 4510, und beide Läufe zeigten dieselbe Zahl. Der Fall
 * unten wirft, sobald die Punkte reichen, und sichert mit `wurfBei` ab, dass
 * es überhaupt geschah.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

type Lauf = {
  /** Wann ANCHOR angenommen wurde, oder -1. */
  wurfBei: number;
  /** Wann der Boss seine Phase wechselte, oder -1. */
  wechselBei: number;
};

function lauf(mitAnchor: boolean): Lauf {
  const boss = bossById('test-phase') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const kindle = alsSpellIndex(combat.indexOf('kindle'));
  const anchor = alsSpellIndex(combat.indexOf('anchor'));

  let wurfBei = -1;
  let wechselBei = -1;
  let letztePhase = combat.bossPhase;

  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    // **So früh wie möglich werfen.** ANCHOR kostet 6 Punkte, BIND bringt 2 —
    // drei Würfe. Danach sofort, damit das Fenster den Wechsel noch erwischt.
    if (mitAnchor && wurfBei < 0 && combat.control >= 6 && combat.isReady(anchor, timeMs)) {
      if (combat.cast(anchor, timeMs).result === CAST_OK) wurfBei = timeMs;
    } else if (combat.control < 6 && combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    } else if (combat.isReady(kindle, timeMs)) {
      combat.cast(kindle, timeMs);
    }

    if (combat.bossPhase !== letztePhase && wechselBei < 0) {
      wechselBei = timeMs;
      letztePhase = combat.bossPhase;
    }
  }
  return { wurfBei, wechselBei };
}

describe('AC1 — die Lage misst überhaupt etwas', () => {
  it('ohne ANCHOR wechselt der Boss seine Phase, zu einer festen Zeit', () => {
    // **Die Zusicherung vor der Messung.** Erreichte der Boss seine Schwelle
    // nicht, verglichen die Fälle darunter zwei Mal `-1` und wären grün, ohne
    // etwas zu zeigen — genau die Lage bei `boss-1-2`.
    const ohne = lauf(false);
    expect(ohne.wechselBei, 'der Boss erreicht seine Phasenschwelle nie').toBeGreaterThan(0);
    expect(ohne.wechselBei, 'der Wechsel liegt anders als beim Festnageln').toBe(4510);
  });
});

describe('AC2 — ANCHOR schiebt den Phasenwechsel nach hinten', () => {
  it('der Wechsel rückt von 4510 auf 9010 ms', () => {
    const mit = lauf(true);

    // **Ohne diese Zeile misst der Vergleich nichts.** Wird ANCHOR nie
    // angenommen — zu wenige Punkte, Abklingzeit —, läuft der Fall wie der
    // ohne Verstärkung und wäre grün.
    expect(mit.wurfBei, 'ANCHOR wurde nie geworfen').toBeGreaterThanOrEqual(0);

    // Die Zahl steht HART. Ein `> 4510` bliebe grün, wenn `amountPercent` von
    // zehn auf einen fiele — und genau das war die Lücke.
    expect(mit.wechselBei, 'ANCHOR hält die Phase nicht um seinen vollen Betrag zurück').toBe(9010);
  });

  it('AC3 — der Wurf liegt VOR dem Wechsel, sonst wirkt er nicht mehr', () => {
    // **Der Grund, aus dem mein erster Messversuch nichts zeigte.** Er warf
    // bei 6000 ms; der Wechsel war bei 4510 längst durch, und beide Läufe
    // ergaben dieselbe Zahl. `holdPhase` senkt die Schwelle für das laufende
    // Fenster — danach ist nichts mehr zurückzuhalten.
    const ohne = lauf(false);
    const mit = lauf(true);
    expect(mit.wurfBei).toBeLessThan(ohne.wechselBei);
  });
});
