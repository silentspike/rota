/**
 * POULTICE heilt ÜBER ZEIT, nicht sofort (Issue #481).
 *
 * # Was hier eigentlich geprüft wird
 *
 * Die Summe allein sagt über diesen Zauber nichts: 1800 auf einmal und 1800
 * über sechs Sekunden ergeben am Ende dieselbe Zahl. Der Unterschied IST der
 * Zauber — deshalb misst der erste Fall unten den Verlauf und nicht das
 * Ergebnis, und der zweite hält fest, dass im Moment des Wurfes noch nichts
 * angekommen ist.
 *
 * # Warum ein eigener Fall zur Ganzzahligkeit
 *
 * Der erste Entwurf trug `perTick = amount / schritte`. Bei POULTICE
 * (1800 über sechs Schritte) ergibt das glatte 300, und jeder Fall wäre grün
 * gewesen. Der nächste Zauber mit einem nicht teilbaren Betrag hätte eine
 * Fließkommazahl bekommen, die Spiel und Simulator unabhängig hätten runden
 * müssen — GAME-DESIGN 18.4 schließt genau das aus. Der Fall `verteilt einen
 * unteilbaren Betrag` unten fährt diesen Fall künstlich herbei.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Der Zauber, wie er ausgeliefert wird — die Zahlen unten stammen von hier. */
const POULTICE = CALCINATION.value.spells.find((s) => s.id === 'poultice');

function frischerKampf(): Combat {
  // **Ein Boss, der austeilt.** Eine Heilung deckelt am fehlenden Leben; ohne
  // Schaden verpufft jeder ihrer Schritte, und der Fall wäre grün, ohne etwas
  // gemessen zu haben. `test-onslaught` schlägt bei 5000 mit 2000 zu — mehr,
  // als POULTICE zurückholt.
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('data/bosses/test-onslaught.json fehlt.');
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  combat.beginFight(0);
  return combat;
}

/**
 * Wie viel Leben gerade fehlt.
 *
 * Aus den beiden öffentlichen Zahlen gerechnet statt aus `vesselMax` — das ist
 * privat, und ein Prüffall ist kein Grund, ein Feld zu öffnen. Solange in
 * diesem Lauf nur POULTICE heilt, ist die Differenz genau das Fehlende.
 */
function fehlend(combat: Combat): number {
  return combat.damageTaken - combat.healed;
}

function wirf(combat: Combat, id: string, timeMs: number): void {
  const index = alsSpellIndex(combat.indexOf(id));
  expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
  expect(combat.cast(index, timeMs).result, `${id} wurde abgewiesen`).toBe(CAST_OK);
}

describe('POULTICE — die Heilung kommt verteilt an', () => {
  it('trägt die Felder, aus denen der Verlauf folgt', () => {
    // Die Zahlen der Fälle darunter sind aus diesen dreien abgeleitet, nicht
    // abgeschrieben. Steht hier etwas anderes, sollen die Fälle rot werden —
    // aber mit dieser Zeile als erstem Hinweis statt mit einer krummen Summe.
    expect(POULTICE?.heal?.amount).toBe(1800);
    expect(POULTICE?.heal?.overMs).toBe(6000);
    expect(POULTICE?.heal?.tickIntervalMs).toBe(1000);
  });

  it('heilt IM MOMENT DES WURFES nichts — das ist der Unterschied zu SALVE', () => {
    const combat = frischerKampf();
    // Erst Schaden nehmen, sonst deckelt die Heilung bei null.
    for (let t = 0; t <= 5200; t += TICK_MS) combat.stepPassive(t);
    expect(combat.damageTaken, 'der Boss hat nicht ausgeteilt').toBeGreaterThan(0);

    wirf(combat, 'poultice', 5200);
    expect(combat.healed, 'POULTICE hat sofort geheilt statt über die Zeit').toBe(0);
  });

  it('heilt in sechs Schritten zu 300, nicht in einem zu 1800', () => {
    const combat = frischerKampf();
    // **Zwei Treffer abwarten, nicht einen.** Gemessen: Nach dem ersten fehlen
    // 1600 — weniger als POULTICEs 1800, und der letzte Schritt kam nur zu 100
    // an. Der Fall haette dann den Deckel gemessen statt der Verteilung.
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    expect(fehlend(combat), 'zu wenig fehlt').toBeGreaterThan(1800);
    wirf(combat, 'poultice', 12_200);

    // Nach jedem vollen Abstand ein Schritt mehr. Die Liste ist die Aussage:
    // Eine Prüfung nur auf die Endsumme wäre bei einer Sofortheilung ebenso
    // grün.
    const gemessen: number[] = [];
    for (let t = 12_200 + TICK_MS; t <= 12_200 + 6000; t += TICK_MS) {
      combat.stepPassive(t);
      if ((t - 12_200) % 1000 === 0) gemessen.push(combat.healed);
    }
    expect(gemessen).toEqual([300, 600, 900, 1200, 1500, 1800]);
  });

  it('hört nach der Dauer auf — der siebte Schritt kommt nicht', () => {
    const combat = frischerKampf();
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    wirf(combat, 'poultice', 12_200);
    for (let t = 12_200 + TICK_MS; t <= 12_200 + 6000; t += TICK_MS) combat.stepPassive(t);

    // 1800 und nicht mehr — ohne diese Grenze liefe die Heilung weiter,
    // solange der Kampf dauert, und das fiele erst bei langen Läufen auf.
    expect(combat.healed).toBe(1800);
  });

  it('deckelt am fehlenden Leben — was darüber hinausginge, verpufft', () => {
    const combat = frischerKampf();
    // Kein Schaden: Es fehlt nichts, also kann nichts zurückkommen.
    wirf(combat, 'poultice', 0);
    for (let t = TICK_MS; t <= 4000; t += TICK_MS) combat.stepPassive(t);

    // Vor dem ersten Schlag des Bosses (5000 ms) ist die Anzeige voll.
    expect(combat.healed, 'die Heilung hat über die volle Anzeige hinaus gezählt').toBe(0);
  });

  it('setzt bei einem zweiten Wurf NEU auf, statt sich zu überlagern', () => {
    const combat = frischerKampf();
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    wirf(combat, 'poultice', 12_200);
    // Der erste Wurf läuft ganz durch, dann kommt der dritte Treffer (19_000)
    // und reißt wieder genug auf, damit der zweite Wurf nicht am Deckel misst.
    for (let t = 12_200 + TICK_MS; t <= 20_200; t += TICK_MS) combat.stepPassive(t);
    const nachDemErsten = combat.healed;
    expect(fehlend(combat), 'zu wenig fehlt').toBeGreaterThan(1800);
    wirf(combat, 'poultice', 20_200);
    for (let t = 20_200 + TICK_MS; t <= 20_200 + 6000; t += TICK_MS) combat.stepPassive(t);

    // Der zweite Wurf bringt seine eigenen 1800 — nicht mehr. Liefen beide
    // nebeneinander, stünde hier mehr, und die Heilung wäre doppelt so stark
    // wie ihr Datensatz sagt.
    expect(combat.healed - nachDemErsten).toBe(1800);
  });

  it('endet mit der Runde — sie läuft nicht in die nächste hinein', () => {
    const combat = frischerKampf();
    for (let t = 0; t <= 12_200; t += TICK_MS) combat.stepPassive(t);
    wirf(combat, 'poultice', 12_200);
    combat.stepPassive(12_300);

    // **Die neue Runde beginnt sofort, und das ist der Punkt.** Eine
    // Rücksetzung, die nur die Zahl `healed` nullt, sähe hier genauso aus wie
    // eine, die auch den LAUF beendet — solange die offenen Schritte in eine
    // Zeit fallen, in der die Anzeige ohnehin voll ist. Deshalb liegt der
    // Schnitt direkt nach dem Wurf: Die Schritte des alten Wurfes reichen dann
    // bis 18_200, der erste Treffer der neuen Runde kommt bei 17_300, und ein
    // Rest, der weiterliefe, hätte etwas zu heilen.
    //
    // Gemessen: Mit `beginFight(0)` und mit `beginFight(30_000)` blieb dieser
    // Fall grün, auch wenn die Rücksetzung in `Combat.reset` ganz fehlte.
    combat.reset();
    combat.beginFight(12_300);
    for (let t = 12_300; t <= 18_300; t += TICK_MS) combat.stepPassive(t);

    expect(combat.damageTaken, 'der Treffer der neuen Runde blieb aus').toBeGreaterThan(0);
    expect(combat.healed, 'die Heilung der vorigen Runde lief weiter').toBe(0);
  });
});

describe('POULTICE — Ganzzahligkeit', () => {
  it('verteilt einen unteilbaren Betrag ohne Bruch und ohne Verlust', () => {
    // 1000 über 6 Schritte sind 166,66… je Schritt. Kumulativ gerechnet
    // kommen 166, 333, 500, 666, 833, 1000 an: jeder Wert ganzzahlig, die
    // Summe exakt der Betrag. Ein vorab gerechneter Betrag je Schritt hätte
    // hier eine Fließkommazahl in die Anzeige und in die Prüfsumme getragen.
    const geheilt = kumulativ(1000, 6);
    expect(geheilt).toEqual([166, 333, 500, 666, 833, 1000]);
    for (const wert of geheilt) expect(Number.isInteger(wert)).toBe(true);
    expect(geheilt[geheilt.length - 1]).toBe(1000);
  });

  /**
   * Dieselbe Rechnung wie in `Combat.stepPassive` — und das ist hier
   * ausnahmsweise richtig so.
   *
   * Der Kampf-Code kann diesen Fall nicht liefern, weil kein ausgelieferter
   * Zauber einen unteilbaren Betrag trägt: POULTICE hat 1800 über sechs
   * Schritte. Geprüft wird deshalb die REGEL, nicht ihr Aufruf — und dass der
   * Kampf-Code sie anwendet, hält der Verlaufs-Fall oben fest.
   */
  function kumulativ(amount: number, schritte: number): number[] {
    const aus: number[] = [];
    let gutgeschrieben = 0;
    for (let getan = 1; getan <= schritte; getan++) {
      const soll = Math.floor((amount * getan) / schritte);
      gutgeschrieben += soll - gutgeschrieben;
      aus.push(gutgeschrieben);
    }
    return aus;
  }
});
