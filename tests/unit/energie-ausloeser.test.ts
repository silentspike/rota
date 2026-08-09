/**
 * Der Energie-Auslöser richtet Schaden an und schreibt ihn in den Verlauf
 * (Issue #568).
 *
 * # Warum dieser Fall NICHT der Parität überlassen wird
 *
 * Der Verlauf steht in **keiner** Aufzeichnung, die einen Boss mit
 * Energieleiste fährt. Gemessen: Von allen Aufzeichnungen unter
 * `tests/parity/recordings/` tragen genau zwei überhaupt `"kind"`-Einträge
 * (`sets.json`, `log-kinds.json`). `vessel-phases.json` — die Aufzeichnung
 * des Bosses, um den es hier geht — hat null.
 *
 * Und `log-kinds.json` prüft etwas anderes, als sein Name nahelegt: den
 * gemeinsamen **Nummernraum**. Sie belegt, dass Art 8 auf beiden Seiten
 * `bossEnergyTriggered` *heißt* — nicht, dass sie je *geschrieben* wird.
 *
 * Daraus folgt der Satz, der zu diesem Fall geführt hat:
 *
 * > Eine Ereignisart kann korrekt vergeben, dokumentiert, von `log-kinds`
 * > gedeckt und von `freshness` gezählt sein — und auf einer Seite nie
 * > geschrieben werden. Nichts meldet es.
 *
 * Genau das war bei Art 8 der Fall: `EVENT_BOSS_ENERGY_TRIGGERED` hatte im
 * ganzen `src/` **einen** Treffer, nämlich seine Deklaration. Der Einbau
 * könnte fehlen, falsch sein oder stimmen — alle drei Läufe wären grün.
 * Deshalb steht hier ein eigener Fall mit einer harten Zahl.
 *
 * Wie viele *andere* Arten dasselbe Bild zeigen, ist ungemessen und steht
 * als Issue #676. Hier wird nur Art 8 geprüft.
 *
 * # Die Zahl ist genau, nicht „mindestens"
 *
 * Vier Auslöser, vier Einträge. Ein `toBeGreaterThan(0)` ginge auch durch,
 * wenn der Auslöser doppelt schriebe oder ein fremder Eintrag mitzählte —
 * und beides wäre ein Fehler, der sich später als Paritäts-Abweichung
 * zeigte, ohne dass jemand hier nachsähe.
 */
import { describe, expect, it } from 'vitest';

import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { CombatLog } from '../../src/core/log.js';
import { TICK_MS } from '../../src/core/loop.js';

// `toArray()` liefert die Art als NAMEN, nicht als Zahl — ein Vergleich
// gegen die Konstanten aus `log.ts` findet nichts und faerbt den Fall rot,
// obwohl der Einbau stimmt.
const AUSLOESER = 'bossEnergyTriggered';
const SPIELERSCHADEN = 'playerDamage';

const VESSEL = bossById('test-vessel');
if (VESSEL === undefined) throw new Error('data/bosses/test-vessel.json fehlt.');

/** Der Kampf läuft ohne einen einzigen Tastendruck — die Leiste steigt allein. */
function laufenMitLog(bisMs: number): { combat: Combat; log: CombatLog } {
  const combat = new Combat({ school: CALCINATION.value, boss: VESSEL as never });
  const log = new CombatLog(4096);
  combat.log = log;
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
  return { combat, log };
}

describe('die Energieleiste löst einen Boss-Schlag aus', () => {
  it('die Daten tragen die Wirkung überhaupt', () => {
    // **Zusicherung in den Datensatz.** Stünde `onTrigger` nicht in der
    // Datei, prüften alle Fälle darunter eine Leiste ohne Wirkung — und
    // wären grün, weil dann eben nichts passiert.
    const energy = VESSEL.energy;
    expect(energy, 'test-vessel hat keine Energieleiste mehr').not.toBeNull();
    expect(energy!.onTrigger.mechanic).toBe('strike');
    expect(energy!.onTrigger.damage, 'ein Schlag ohne Schaden bewirkt nichts').toBeGreaterThan(0);
  });

  it('vier Auslöser schreiben genau vier Einträge der Art 8', () => {
    // 100 bei 7 je Sekunde: der fünfzehnte Schritt erreicht die Schwelle,
    // danach alle 15 s wieder. In 60 s also vier Auslösungen.
    const { combat, log } = laufenMitLog(60_000);
    expect(combat.bossEnergyTriggers, 'die Leiste hat die Schwelle nie erreicht').toBe(4);

    const eintraege = log.toArray().filter((e) => e.type === AUSLOESER);
    expect(eintraege.length, 'Auslöser und Einträge gehen auseinander').toBe(4);

    // Der Wert zählt mit — ein Eintrag, der immer dieselbe Zahl trüge, wäre
    // von einem festen Wert nicht zu unterscheiden.
    expect(eintraege.map((e) => e.value)).toEqual([1, 2, 3, 4]);
  });

  it('der Eintrag steht VOR dem Schaden, den er auslöst', () => {
    // **Die Reihenfolge ist Parität, nicht Geschmack:** `fight.rs` schreibt
    // den Auslöser vor dem Schlag. Fachlich gilt dasselbe — tötet der
    // Schlag, steht der Grund davor und nicht dahinter.
    const { log } = laufenMitLog(20_000);
    const arten = log
      .toArray()
      .filter((e) => e.type === AUSLOESER || e.type === SPIELERSCHADEN)
      .map((e) => e.type);

    const ersterAusloeser = arten.indexOf(AUSLOESER);
    expect(ersterAusloeser, 'kein Auslöser im Verlauf').toBeGreaterThanOrEqual(0);
    expect(
      arten[ersterAusloeser + 1],
      'auf den Auslöser folgt kein Spielerschaden — der Schlag fehlt oder steht davor',
    ).toBe(SPIELERSCHADEN);
  });

  it('der Schlag kommt beim Spieler an, und zwar in Höhe des Datenwerts', () => {
    // **Der Log führt den ANGEKOMMENEN Schaden, nicht den Rohwert.** Ein
    // Vergleich gegen `onTrigger.damage` schlägt deshalb fehl, obwohl der
    // Einbau stimmt — Rüstung und Widerstände liegen dazwischen. Gemessen
    // wird also, was den Spieler erreicht: vier gleich hohe Treffer, jeder
    // über null.
    const { combat, log } = laufenMitLog(60_000);
    const eintraege = log.toArray();

    const nachAusloeser: number[] = [];
    for (let i = 0; i < eintraege.length - 1; i++) {
      if (eintraege[i]!.type === AUSLOESER && eintraege[i + 1]!.type === SPIELERSCHADEN) {
        nachAusloeser.push(eintraege[i + 1]!.value);
      }
    }

    expect(nachAusloeser.length, 'nicht jeder Auslöser hat einen Treffer hinter sich').toBe(
      combat.bossEnergyTriggers,
    );
    // Alle vier gleich hoch: Der Schlag hängt an einem festen Datenwert und
    // nicht am Kampfverlauf. Wären sie verschieden, käme der Schaden von
    // woanders her.
    expect(new Set(nachAusloeser).size, 'die Treffer sind verschieden hoch').toBe(1);
    expect(nachAusloeser[0], 'der Treffer kommt mit null an').toBeGreaterThan(0);
  });
});
