/**
 * Die letzten fünf Größen der obersten Ebene (Issue #489, #498, #518).
 *
 * # Zweimal „der erste Träger ist geprüft, die anderen nicht"
 *
 * ```text
 *   controlGain           clasp        1 -> 0        NUR FRISCHE
 *   controlGain           tether       2 -> 1        NUR FRISCHE
 *   perDecayStackPercent  miasma      12 -> 6        NUR FRISCHE
 *   extendShieldMs        harden   12000 -> 6000     NUR FRISCHE
 *   extendDecayMs         sepsis   12000 -> 6000     NUR FRISCHE
 * ```
 *
 * Bei den ersten drei liegt der Befund nicht im Feld, sondern im **Träger**:
 *
 * ```text
 *   controlGain           bind      13 Prüffälle   clasp/tether  0
 *   perDecayStackPercent  fester     5 Prüffälle   miasma        0
 *                         necrosis   1 (seit #768)
 * ```
 *
 * Wer nur den ersten Träger misst, bekommt für das Feld eine Antwort, die für
 * die anderen falsch ist — genau davor warnt der Kommentar in `felder()` des
 * Mutationswerkzeugs. `necrosis` war beim ersten Messen ebenfalls offen und
 * ist inzwischen durch den Schalter-Fall mitgedeckt; **nachgemessen, nicht
 * angenommen**, sonst stünde hier ein zweiter Fall für dieselbe Zahl.
 *
 * # Die beiden Verlängerungen brauchen einen Messpunkt JENSEITS der halben Frist
 *
 * ```text
 *                       12000   15000   18000   21000   24000   27000
 *   PLATE allein         2400    2400       0       0       0       0
 *   + HARDEN             2400    2400    2400    2400    2400    2400
 *   + HARDEN halbiert    2400    2400    2400    2400       0       0
 * ```
 *
 * Bei 18000 trennt sich nur „mit" von „ohne" — die Halbierung sieht man dort
 * nicht. Erst bei 24000 gehen die beiden Fassungen auseinander. Beide
 * Messpunkte stehen deshalb im Fall, und sie prüfen Verschiedenes.
 */
import { describe, expect, it } from 'vitest';

import { CAST_OK, Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** Kontroll-Punkte nach einem einzelnen Wurf. */
function punkteNach(id: string): number {
  const combat = neu();
  combat.stepPassive(0);
  expect(combat.cast(alsSpellIndex(combat.indexOf(id)), 0).result, `${id} kam nicht durch`).toBe(
    CAST_OK,
  );
  return combat.control;
}

describe('CLASP und TETHER bringen verschieden viele Punkte (controlGain)', () => {
  it('CLASP bringt einen', () => {
    expect(punkteNach('clasp'), 'CLASP bringt nicht genau einen Punkt').toBe(1);
  });

  it('TETHER bringt zwei', () => {
    expect(punkteNach('tether'), 'TETHER bringt nicht genau zwei Punkte').toBe(2);
  });

  it('und die beiden unterscheiden sich wirklich', () => {
    // **Der eigentliche Schutz.** Beide Zahlen einzeln festzunageln reicht
    // nicht: Läse der Code für alle Zauber denselben Wert, wäre einer der
    // Fälle oben schon rot — aber welcher, sagte nur dieser hier.
    expect(punkteNach('tether')).toBe(punkteNach('clasp') * 2);
  });

  it('BIND liegt gleichauf mit TETHER — auch das ist gemessen', () => {
    // BIND ist mit 13 Prüffällen breit getragen; er steht hier als
    // Bezugspunkt, damit sichtbar ist, dass TETHERs 2 kein Tippfehler sind.
    expect(punkteNach('bind')).toBe(2);
  });
});

describe('MIASMA rechnet mit 12 % je Fäulnis-Stapel (perDecayStackPercent)', () => {
  /** Baut fünf Stapel auf und wirft dann `id`. */
  function schadenMitStapeln(id: string): { stapel: number; schaden: number } {
    const combat = neu();
    const canker = alsSpellIndex(combat.indexOf('canker'));
    const blight = alsSpellIndex(combat.indexOf('blight'));

    let zeit = 0;
    for (let i = 0; i < 6; i++) {
      combat.stepPassive(zeit);
      if (combat.isReady(canker, zeit)) combat.cast(canker, zeit);
      else if (combat.isReady(blight, zeit)) combat.cast(blight, zeit);
      zeit += 1500;
    }
    combat.stepPassive(zeit);
    const stapel = combat.decayStacks;
    const vorher = combat.damageDealt;
    expect(combat.cast(alsSpellIndex(combat.indexOf(id)), zeit).result).toBe(CAST_OK);
    return { stapel, schaden: combat.damageDealt - vorher };
  }

  it('die Zusicherung: es liegen fünf Stapel', () => {
    expect(schadenMitStapeln('miasma').stapel, 'der Aufbau erreicht keine fünf Stapel').toBe(5);
  });

  it('aus 25 Grundschaden werden 48', () => {
    // Halbiert stünden hier 40. Gemessen mit mutierter Datei — die Zahl
    // enthält neben den 12 % auch die Verwundbarkeit aus den Stapeln, sie
    // lässt sich also nicht im Kopf nachrechnen.
    expect(schadenMitStapeln('miasma').schaden, 'MIASMA rechnet nicht mit 12 %').toBe(48);
  });

  it('FESTER kommt mit demselben Aufbau auf 173 — verschiedene Sätze', () => {
    // **Die Gegenprobe zum Träger.** FESTER trägt 20 %, MIASMA 12. Stünden
    // hier gleiche Zahlen, läse der Code den Satz nicht je Zauber.
    expect(schadenMitStapeln('fester').schaden).toBe(173);
  });
});

describe('HARDEN verlängert die Schicht um zwölf Sekunden (extendShieldMs)', () => {
  /** Schild zu mehreren Zeitpunkten, mit und ohne HARDEN. */
  function schildBei(beiMs: number, mitHarden: boolean): number {
    const combat = neu();
    combat.stepPassive(0);
    combat.cast(alsSpellIndex(combat.indexOf('plate')), 0);
    if (mitHarden) {
      combat.stepPassive(1500);
      expect(combat.cast(alsSpellIndex(combat.indexOf('harden')), 1500).result).toBe(CAST_OK);
    }
    for (let timeMs = 0; timeMs <= beiMs; timeMs += TICK_MS) combat.stepPassive(timeMs);
    return combat.shield;
  }

  it('ohne HARDEN ist die Schicht bei 18000 ms weg', () => {
    expect(schildBei(18000, false), 'PLATEs Schicht hält ohne HARDEN zu lange').toBe(0);
  });

  it('mit HARDEN steht sie dort noch', () => {
    expect(schildBei(18000, true), 'HARDEN verlängert gar nicht').toBe(2400);
  });

  it('und noch bei 24000 — dort trennt sich erst die halbierte Fassung', () => {
    // **Der Messpunkt, der die GRÖSSE prüft.** Bei 18000 zeigt auch eine
    // halbierte Verlängerung noch 2400; erst hier steht sie auf 0.
    expect(schildBei(24000, true), 'die Verlängerung ist kürzer als zwölf Sekunden').toBe(2400);
  });
});

describe('SEPSIS verlängert die Fäulnis-Stapel um zwölf Sekunden (extendDecayMs)', () => {
  function stapelBei(beiMs: number, mitSepsis: boolean): number {
    const combat = neu();
    combat.stepPassive(0);
    combat.cast(alsSpellIndex(combat.indexOf('canker')), 0);
    if (mitSepsis) {
      combat.stepPassive(1500);
      expect(combat.cast(alsSpellIndex(combat.indexOf('sepsis')), 1500).result).toBe(CAST_OK);
    }
    for (let timeMs = 0; timeMs <= beiMs; timeMs += TICK_MS) combat.stepPassive(timeMs);
    return combat.decayStacks;
  }

  it('ohne SEPSIS ist der Stapel bei 12000 ms verfallen', () => {
    expect(stapelBei(12000, false), 'der Stapel hält ohne SEPSIS zu lange').toBe(0);
  });

  it('mit SEPSIS liegt er bei 18000 ms noch — dort trennt die Halbierung', () => {
    // Halbiert reichte die Verlängerung bis 15000; hier stünde dann 0.
    expect(stapelBei(18000, true), 'SEPSIS verlängert nicht um zwölf Sekunden').toBe(1);
  });

  it('und bei 24000 ist auch er verfallen — die Verlängerung ist endlich', () => {
    // Ohne diesen Fall wäre auch eine unendliche Verlängerung grün.
    expect(stapelBei(24000, true), 'SEPSIS macht den Stapel unvergänglich').toBe(0);
  });
});
