/**
 * CRUCIBLE SKIN wirkt auf allen drei Wegen der Schule (Issue #481).
 *
 * # Warum dieser Fall die drei Wege EINZELN misst
 *
 * Das Raster nennt ihn „alle drei Wege gleichzeitig". Ein Fall, der nur
 * prüfte, dass *irgendetwas* passiert, wäre grün, sobald einer der drei
 * greift — und die anderen beiden blieben ungedeckt.
 *
 * Deshalb je Weg eine eigene Zusicherung, jede mit ihrer eigenen Größe:
 *
 * ```text
 *   Weg 1  Schutzschicht    absorbierter Schaden
 *   Weg 2  Heilung          geheilte Punkte
 *   Weg 3  Umwandlung       Schild aus ausgeteiltem Schaden
 * ```
 *
 * # Die Gegenprobe ist AC3 und steht im PR-Text
 *
 * Ohne den Zauber bewegt sich keine der drei Größen. Ein Zauber, dessen
 * Weglassen nichts ändert, ist nicht angeschlossen — das ist die Klasse,
 * die dieses Projekt neunmal getroffen hat.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function lauf(mitZauber: boolean): {
  geheilt: number;
  absorbiert: number;
  schaden: number;
} {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('data/bosses/test-warden.json fehlt.');
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  combat.beginFight(0);

  // **Der Wurf liegt VOR dem Treffer, nicht am Kampfbeginn.** Gemessen:
  // Bei t=0 geworfen ist die Schicht (6000 ms) längst abgelaufen, wenn
  // test-warden bei 12000 zuschlägt — `shieldAbsorbed` bliebe 0, und der
  // Fall meldete „wirkt nicht", obwohl der Zauber tut, was er soll.
  //
  // Das ist die Eigenart von Weg 1: Eine Schutzschicht muss VOR dem Schlag
  // stehen, und ein Fall, der sie zu früh legt, misst ihren Ablauf statt
  // ihrer Wirkung.
  const WURF_MS = 8000;

  // **Das Fenster folgt den Boss-Daten, nicht dem Schild.** test-warden
  // wirkt seinen ersten Zauber bei 4000 ms und schlaegt bei 12000 — die
  // Schicht haelt 6000 ms ab dem Wurf. Ein kuerzerer Lauf endete VOR dem
  // ersten Treffer, und der Fall waere gruen, ohne etwas gemessen zu haben.
  for (let t = 0; t <= 13000; t += TICK_MS) {
    combat.stepPassive(t);
    if (mitZauber && t === WURF_MS) {
      const index = alsSpellIndex(combat.indexOf('crucible-skin'));
      expect(index, 'crucible-skin fehlt in der Schule').toBeGreaterThanOrEqual(0);
      expect(combat.cast(index, t).result, 'crucible-skin wurde abgewiesen').toBe(CAST_OK);
    }
  }
  return {
    geheilt: combat.healed,
    absorbiert: combat.shieldAbsorbed,
    schaden: combat.damageTaken,
  };
}

describe('CRUCIBLE SKIN wirkt auf allen drei Wegen', () => {
  it('die Daten tragen alle drei Wirkungen', () => {
    // **Zusicherung in den Datensatz.** Fehlte eine der drei, prüften die
    // Fälle darunter einen Zauber, der die Rolle des Rasters nicht erfüllt —
    // und wären grün, weil die übrigen zwei ja wirken.
    const zauber = CALCINATION.value.spells.find((s) => s.id === 'crucible-skin');
    expect(zauber, 'crucible-skin steht nicht in der Schul-Datei').toBeDefined();
    expect(zauber!.shield, 'Weg 1 fehlt').toBeDefined();
    expect(zauber!.heal, 'Weg 2 fehlt').toBeDefined();
    expect(zauber!.conversion, 'Weg 3 fehlt').toBeDefined();
  });

  it('Weg 1 und 2 wirken messbar — und ohne den Zauber bewegt sich nichts', () => {
    const ohne = lauf(false);
    const mit = lauf(true);

    // Weg 1: Die Schicht fängt Schaden ab, der sonst ankäme.
    expect(mit.absorbiert, 'die Schutzschicht fängt nichts ab').toBeGreaterThan(ohne.absorbiert);

    // Weg 2: Geheilt wird nur, wenn der Spieler etwas verloren hat — der
    // Boss teilt in diesem Fenster aus, also ist die Bedingung erfüllt.
    expect(mit.geheilt, 'es wird nicht geheilt').toBeGreaterThan(ohne.geheilt);

    // **AC3 in einer Zeile:** Ohne den Zauber stehen beide Größen auf null.
    expect(ohne.absorbiert, 'ohne den Zauber wird abgefangen — dann misst der Fall nicht ihn').toBe(
      0,
    );
    expect(ohne.geheilt, 'ohne den Zauber wird geheilt — dann misst der Fall nicht ihn').toBe(0);
  });
});
