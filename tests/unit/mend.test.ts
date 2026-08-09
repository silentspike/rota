/**
 * MEND heilt umso mehr, je weniger Leben da ist (Issue #481).
 *
 * # Warum eine Reihe und nicht ein einzelner Wert
 *
 * Ein Fall, der nur bei halbem Leben prüft, wäre auch dann grün, wenn der
 * Zauber einen festen Betrag heilte, der zufällig passt. Die Aussage des
 * Rasters ist eine RICHTUNG: „je weniger, desto mehr". Deshalb misst der erste
 * Fall unten drei Stände nacheinander und verlangt, dass die Beträge steigen.
 *
 * # Die zweite Aussage steht im Grundbetrag
 *
 * Bei voller Anzeige bleibt nur `amount`. Das ist die Abwägung, die ihn von
 * SALVE unterscheidet: Wer ihn aus Gewohnheit wirft, bekommt wenig. Ein
 * Zuschlag, der auch bei vollem Leben zählte, machte daraus den stärksten
 * Heilzauber der Schule ohne jede Bedingung.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const MEND = CALCINATION.value.spells.find((s) => s.id === 'mend');

/**
 * Ein Kampf, in dem der Boss zu bekannten Zeiten austeilt.
 *
 * `test-onslaught` trifft bei 5000, 12_000 und 19_000 mit je 2000 (nach WARD
 * 1600). Damit lassen sich drei verschieden tiefe Stände ansteuern, ohne eine
 * Zahl zu erfinden — jeder Treffer reißt dasselbe Loch.
 */
function kampf(): Combat {
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('data/bosses/test-onslaught.json fehlt.');
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  combat.beginFight(0);
  return combat;
}

function wirf(combat: Combat, id: string, timeMs: number): void {
  const index = alsSpellIndex(combat.indexOf(id));
  expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
  expect(combat.cast(index, timeMs).result, `${id} wurde abgewiesen`).toBe(CAST_OK);
}

/** Wie viel MEND bei diesem Stand heilt — an `healed` abgelesen, nicht gerechnet. */
function heiltNach(bisMs: number): { geheilt: number; fehlteVorher: number } {
  const combat = kampf();
  for (let t = 0; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
  const fehlteVorher = combat.damageTaken - combat.healed;
  const vorher = combat.healed;
  wirf(combat, 'mend', bisMs);
  return { geheilt: combat.healed - vorher, fehlteVorher };
}

describe('MEND — der Betrag hängt am fehlenden Leben', () => {
  it('trägt die beiden Felder, aus denen die Beträge folgen', () => {
    expect(MEND?.heal?.amount).toBe(800);
    expect(MEND?.heal?.perMissingPercent).toBe(30);
  });

  it('heilt bei voller Anzeige NICHTS — es fehlt ja nichts', () => {
    const combat = kampf();
    wirf(combat, 'mend', 0);
    // Nicht „800": Auch der Grundbetrag deckelt am fehlenden Leben. Der Fall
    // steht trotzdem hier, weil er die untere Grenze festhält.
    expect(combat.healed).toBe(0);
  });

  it('heilt nach jedem weiteren Treffer MEHR — das ist die Aussage des Rasters', () => {
    // Nach dem ersten Treffer (5000), dem zweiten (12_000) und dem dritten
    // (19_000). Die Zeitpunkte liegen je 200 ms danach, damit der Treffer
    // sicher verrechnet ist.
    const stufen = [heiltNach(5200), heiltNach(12_200), heiltNach(19_200)];

    for (const stufe of stufen) {
      expect(stufe.fehlteVorher, 'an diesem Stand fehlte nichts').toBeGreaterThan(0);
    }
    // **Streng steigend, nicht bloß verschieden.** Drei ungleiche Zahlen
    // könnten auch in der falschen Reihenfolge stehen — und genau das wäre der
    // Fehler, den man beim Vorzeichen macht.
    expect(stufen[1]!.geheilt).toBeGreaterThan(stufen[0]!.geheilt);
    expect(stufen[2]!.geheilt).toBeGreaterThan(stufen[1]!.geheilt);
  });

  it('rechnet den Zuschlag aus dem fehlenden Prozentsatz, abgerundet', () => {
    // VESSEL fasst 5000, ein Treffer nimmt 1600: 32 Prozent fehlen.
    // 800 + 32 * 30 = 1760. Das ist MEHR als die fehlenden 1600, also greift
    // der Deckel und es kommen 1600 an — die Rechnung ist hier nicht direkt
    // ablesbar, deshalb steht der ungedeckelte Fall darunter.
    const nachEinem = heiltNach(5200);
    expect(nachEinem.fehlteVorher).toBe(1600);
    expect(nachEinem.geheilt).toBe(1600);

    // Nach zwei Treffern fehlen 3200, also 64 Prozent: 800 + 64 * 30 = 2720.
    // Jetzt ist der Betrag kleiner als das Loch, und die Rechnung steht
    // ungedeckelt da.
    const nachZweien = heiltNach(12_200);
    expect(nachZweien.fehlteVorher).toBe(3200);
    expect(nachZweien.geheilt).toBe(2720);
  });

  it('ist ohne den Zuschlag ein anderer Zauber — die Gegenprobe in Zahlen', () => {
    // Der Grundbetrag allein wäre 800. Gemessen sind 2720 nach zwei Treffern:
    // Der Zuschlag trägt also 1920 davon. Ein Fall, der nur „mehr als 800"
    // prüfte, wäre auch bei einem Zuschlag von 1 grün.
    const nachZweien = heiltNach(12_200);
    expect(nachZweien.geheilt - (MEND?.heal?.amount ?? 0)).toBe(1920);
  });
});
