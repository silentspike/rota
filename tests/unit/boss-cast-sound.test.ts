/**
 * Kanal 3 der Boss-Ankündigungen: Beginn und Durchkommen eines Boss-Zaubers sind
 * zu hören (GAME-DESIGN 11.5, Issue #797).
 *
 * WAS VORHER WAR
 * --------------
 * `boss-cast-start.ogg` („Kündigt das Unterbrechen-Fenster an") und
 * `boss-cast-success.ogg` („das Fenster wurde verpasst") stehen im Verzeichnis,
 * ausgelöst von `boss.cast.start` und `boss.cast.success`. Gemessen vor diesem
 * Issue: Kein Aufruf im Quelltext sendete eines der beiden Ereignisse — die
 * zwei Klänge waren gebaut, verzeichnet und stumm.
 *
 * Der Fall hängt an der Sitzung, nicht an einem neuen Modul: Auf dem Stand vor
 * der Behebung läuft er und ist ROT, beim Namen.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, bossById, type BossDef } from '../../src/core/data.js';
import { EV_BOSS_CAST_START, EV_BOSS_CAST_SUCCESS } from '../../src/core/events.js';
import { TICK_MS } from '../../src/core/loop.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

function boss(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

/** `test-warden`: Sealing Word beginnt bei 4000 und wirkt 3000 ms. */
function lauf(quenchAtMs: number | null): number[][] {
  const events: number[][] = [];
  const combat = new Combat({ school: CALCINATION.value, boss: boss('test-warden') });
  const queue = new PressQueue();
  const session = new Session(queue, combat, {
    onEvent: (event, _key, timeMs) => {
      if (event === EV_BOSS_CAST_START || event === EV_BOSS_CAST_SUCCESS) {
        events.push([event, timeMs]);
      }
    },
  });
  session.bind(0, combat.indexOf('quench'));
  session.startFight(0);
  if (quenchAtMs !== null) queue.push(0, quenchAtMs / TICK_MS, quenchAtMs, 0);
  for (let t = 0; t <= 8000; t += TICK_MS) session.tick(t);
  // Sonst prüfte der zweite Fall einen Druck, der nie ankam.
  if (quenchAtMs !== null) expect(combat.interruptsLanded, 'QUENCH hat getroffen').toBe(1);
  return events;
}

describe('Klang: Beginn und Durchkommen eines Boss-Zaubers', () => {
  it('Sealing Word beginnt bei 4000 und kommt bei 7000 durch — je ein Klang', () => {
    expect(lauf(null)).toEqual([
      [EV_BOSS_CAST_START, 4000],
      [EV_BOSS_CAST_SUCCESS, 7000],
    ]);
  });

  it('im selben Takt begonnen und unterbrochen: der Beginn klingt trotzdem, das Durchkommen nicht', () => {
    expect(lauf(4000)).toEqual([[EV_BOSS_CAST_START, 4000]]);
  });

  it('ein Zauber, der einen noch laufenden ablöst, klingt ebenfalls an', () => {
    // `test-collision`: First Word wirkt 4000–7000, Colliding Word beginnt bei
    // 7000. STALL schiebt das Ende des ersten auf 9000 — der zweite beginnt,
    // während der erste noch läuft, und löst ihn ab. Die Summe aus
    // unterbrochenen, durchgekommenen und laufenden Zaubern bleibt dabei gleich;
    // nur der Vergleich mit dem vorigen Takt sieht den Wechsel.
    const events: number[][] = [];
    const combat = new Combat({ school: CALCINATION.value, boss: boss('test-collision') });
    const queue = new PressQueue();
    const session = new Session(queue, combat, {
      onEvent: (event, _key, timeMs) => {
        if (event === EV_BOSS_CAST_START || event === EV_BOSS_CAST_SUCCESS) {
          events.push([event, timeMs]);
        }
      },
    });
    session.bind(0, combat.indexOf('bind'));
    session.bind(1, combat.indexOf('stall'));
    session.startFight(0);
    queue.push(0, 0, 0, 0);
    queue.push(0, 150, 1500, 0);
    queue.push(1, 450, 4500, 0);
    for (let t = 0; t <= 7000; t += TICK_MS) session.tick(t);
    expect(combat.castsInterrupted + combat.interruptsLanded, 'nichts wurde abgebrochen').toBe(0);
    expect(events).toEqual([
      [EV_BOSS_CAST_START, 4000],
      [EV_BOSS_CAST_START, 7000],
    ]);
  });
});
