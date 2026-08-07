import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Zwei weitere Zielgrößen für Verstärkungen (E10, Bündel 1).
 *
 * # Warum genau diese zwei und nicht drei
 *
 * Das Bündel aus der Raster-Messung hieß DISTIL, QUICKSILVER, HARDEN — drei
 * Zauber, die alle „einen bestehenden Wert zeitlich begrenzt verstärken".
 * **Gemessen vor dem Bau** trägt das nur für zwei:
 *
 * | Größe | wo gelesen | trägt ein Fenster? |
 * | --- | --- | --- |
 * | `cooldownReductionMs` | bei jedem Wirken | ja |
 * | `conversion.sharePercent` | bei jedem Schaden | ja |
 * | `shield.durationMs` | **einmal beim Anlegen** | nein |
 *
 * HARDEN verspricht „bestehende Schichten halten länger" — ein Faktor auf
 * `durationMs` träfe aber nur Schilde, die danach entstehen. Er gehört zu
 * `delayStrike` (einen Zeitpunkt verschieben), nicht hierher.
 *
 * Das war meine eigene Fehlzuordnung: Ich hatte ihn nach der BESCHREIBUNG
 * eingeordnet, nicht nach der STELLE.
 */

const BOSS = FIRST_CRUCIBLE.value;

function mitVerstaerkung(id: string): Combat {
  const c = new Combat({ school: CALCINATION.value, boss: BOSS });
  c.cast(c.indexOf(id), 0);
  for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
  return c;
}

function ohneVerstaerkung(): Combat {
  const c = new Combat({ school: CALCINATION.value, boss: BOSS });
  for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
  return c;
}

describe('Die zwei neuen Größen öffnen ein Fenster', () => {
  it.each(['test-cdreduce', 'test-convert'])('%s bewegt die Fenstersumme', (id) => {
    expect(mitVerstaerkung(id).buffWindowSum, `${id} öffnet kein Fenster`).toBeGreaterThan(0);
  });

  it.each(['test-cdreduce', 'test-convert'])('%s lässt den Schadens-Faktor in Ruhe', (id) => {
    // **Die zweite Spalte, die wichtigere.** Beide tragen `kind: 0` — in einem
    // gemeinsamen Speicher würden sie den Schadens-Buff verdrängen.
    expect(mitVerstaerkung(id).damageFactors.buffs, `${id} hat den Schaden bewegt`).toBe(1);
  });

  it('und die alten vier bleiben unberührt', () => {
    // Sonst hätte die Erweiterung die bestehenden Speicher umverdrahtet.
    for (const alt of ['test-control', 'test-heat', 'test-tick', 'test-stack']) {
      expect(mitVerstaerkung(alt).buffWindowSum, `${alt} verloren`).toBeGreaterThan(0);
    }
  });

  it('Gegenprobe: ohne Verstärkung ist die Summe 0', () => {
    expect(ohneVerstaerkung().buffWindowSum).toBe(0);
  });
});
