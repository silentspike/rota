/**
 * RIPOSTE gibt einen Teil des eingehenden Schadens an den Boss zurück
 * (Issue #481).
 *
 * # Der Gegenentwurf zu TEMPER
 *
 * Beide gehören zu Weg 3 und hängen an entgegengesetzten Enden:
 *
 * ```text
 *   conversion  AUSGETEILTER Schaden  ->  Schutz
 *   reflect     EINGEHENDER Schaden   ->  Schaden am Boss
 * ```
 *
 * # Die Entscheidung, die dieser Fall festhält
 *
 * Der Rückschlag hängt am **angekommenen** Schaden, nicht am rohen. Ein
 * Schlag, den die Schicht ganz auffängt, gibt nichts zurück — sonst wären
 * Schicht und Rückschlag zusammen doppelt stark, und der Spieler bekäme
 * Schaden geschenkt, den er gar nicht genommen hat.
 *
 * Das steht nicht im Raster; es sagt nur „ein Teil des eingehenden Schadens".
 * Der vorletzte Fall unten macht die Entscheidung sichtbar, damit ein
 * Widerspruch sie findet.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const RIPOSTE = CALCINATION.value.spells.find((s) => s.id === 'riposte');

function kampf(): Combat {
  // `test-onslaught` trifft bei 5000, 12_000 und 19_000 mit je 1600 (nach
  // WARD). Bekannte Zeiten und ein Betrag, der groß genug ist, dass die Hälfte
  // davon nicht in der Rundung verschwindet.
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

describe('RIPOSTE — der Schlag kommt zurück', () => {
  it('trägt die beiden Felder, aus denen die Beträge folgen', () => {
    expect(RIPOSTE?.reflect?.sharePercent).toBe(50);
    expect(RIPOSTE?.reflect?.durationMs).toBe(8000);
  });

  it('gibt die Hälfte des angekommenen Schadens zurück', () => {
    const combat = kampf();
    // Vor dem Treffer bei 12_000 werfen, damit das Fenster (8000 ms) offen
    // steht, wenn er landet.
    for (let t = 0; t <= 11_000; t += TICK_MS) combat.stepPassive(t);
    const vorher = combat.damageDealt;
    wirf(combat, 'riposte', 11_000);
    for (let t = 11_010; t <= 12_500; t += TICK_MS) combat.stepPassive(t);

    // Der Treffer nimmt 1600, also gehen 800 zurück.
    expect(combat.reflected).toBe(800);
    // Und dieser Schaden zählt wirklich am Boss, nicht nur in einem Zähler.
    expect(combat.damageDealt - vorher).toBe(800);
  });

  it('gibt NACH dem Fenster nichts mehr zurück', () => {
    const combat = kampf();
    // Bei 3000 geworfen läuft das Fenster bis 11_000 — der Treffer bei 12_000
    // liegt danach.
    wirf(combat, 'riposte', 3000);
    for (let t = 3010; t <= 12_500; t += TICK_MS) combat.stepPassive(t);

    // Der Treffer bei 5000 fällt noch INS Fenster: 800. Der bei 12_000 nicht.
    expect(combat.reflected).toBe(800);
    expect(combat.reflectPercent, 'das Fenster steht noch offen').toBe(0);
  });

  it('gibt OHNE den Zauber gar nichts zurück — die Gegenprobe im selben Lauf', () => {
    const combat = kampf();
    for (let t = 0; t <= 12_500; t += TICK_MS) combat.stepPassive(t);
    expect(combat.damageTaken, 'der Boss hat nicht ausgeteilt').toBeGreaterThan(0);
    expect(combat.reflected).toBe(0);
  });

  it('hängt am ANGEKOMMENEN Schaden — was die Schicht auffängt, kommt nicht zurück', () => {
    const combat = kampf();
    for (let t = 0; t <= 10_000; t += TICK_MS) combat.stepPassive(t);
    // PLATE legt 2400 — mehr als der Treffer bei 12_000 (1600) austeilt.
    // **Der Abstand ist eine Wartezeit, keine Bequemlichkeit**: Bei 100 ms
    // Abstand wies der Kampf RIPOSTE mit `CAST_ON_GCD` ab, und der Fall war
    // rot, ohne dass an der Mechanik etwas fehlte.
    wirf(combat, 'plate', 10_000);
    for (let t = 10_010; t <= 11_500; t += TICK_MS) combat.stepPassive(t);
    wirf(combat, 'riposte', 11_500);
    const abgefangenVorher = combat.shieldAbsorbed;
    for (let t = 11_510; t <= 12_500; t += TICK_MS) combat.stepPassive(t);

    expect(combat.shieldAbsorbed - abgefangenVorher, 'die Schicht hat nicht aufgefangen').toBe(
      1600,
    );
    // Nichts ist angekommen, also geht nichts zurück. Hinge der Rückschlag am
    // ROHEN Schaden, stünden hier 800 — und Schicht und Rückschlag wären
    // zusammen doppelt stark.
    expect(combat.reflected).toBe(0);
  });

  it('endet mit der Runde — das Fenster läuft nicht in die nächste hinein', () => {
    const combat = kampf();
    wirf(combat, 'riposte', 3000);
    for (let t = 3010; t <= 5500; t += TICK_MS) combat.stepPassive(t);
    expect(combat.reflected, 'im ersten Durchgang kam nichts zurück').toBe(800);

    combat.reset();
    combat.beginFight(3100);
    for (let t = 3100; t <= 9000; t += TICK_MS) combat.stepPassive(t);
    // Ohne die Rücksetzung stünde das Fenster noch offen, und der erste
    // Treffer der neuen Runde käme zurück, ohne dass jemand RIPOSTE geworfen
    // hat.
    expect(combat.reflected).toBe(0);
    expect(combat.reflectPercent).toBe(0);
  });
});
