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
  erlitten: number;
  umgewandelt: number;
} {
  // **`test-onslaught` statt `test-warden` — nachgetragen am 2026-08-10.**
  //
  // Der erste Entwurf fuhr `test-warden` (720 erlittener Schaden). Damit
  // hatte KEINE der drei Größen Platz:
  //
  // ```text
  //   shield.amount   1800 -> 900   abgefangen blieb 240   nichts bewegt sich
  //   heal.amount     2000 -> 1000  geheilt blieb 480      der Deckel bestimmt
  //   conversion.*    halbiert      converted blieb 0      niemand teilt aus
  // ```
  //
  // Gemessen, nicht vermutet: Mit halbierten Werten blieb der Fall grün,
  // solange man die abgeschriebenen Datensatz-Zeilen wegließ. Er prüfte also
  // die Datei, nicht die Wirkung — dieselbe Klasse, die der Kopf von
  // `docs/GROESSEN-ABDECKUNG.md` beschreibt.
  //
  // `test-onslaught` teilt 1600 je Treffer aus (bei 5000, 12_000, 19_000),
  // und KINDLE füllt jede Wartezeit, damit die Umwandlung etwas umzuwandeln
  // hat. Dieselbe Wahl wie in der Paritäts-Sammelstelle, aus demselben Grund:
  // **Eine Probe braucht einen Träger, bei dem die Wirkung Platz hat.**
  const boss = bossById('test-onslaught');
  if (boss === undefined) throw new Error('data/bosses/test-onslaught.json fehlt.');
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  combat.beginFight(0);

  // Der Wurf liegt NACH dem ersten Treffer und VOR dem zweiten: Die Schicht
  // hält 6000 ms, der Treffer bei 12_000 fällt hinein, und die Sofortheilung
  // findet ein Loch vor, das der erste Treffer gerissen hat.
  const WURF_MS = 6100;

  const kindle = alsSpellIndex(combat.indexOf('kindle'));
  const crucible = alsSpellIndex(combat.indexOf('crucible-skin'));
  expect(crucible, 'crucible-skin fehlt in der Schule').toBeGreaterThanOrEqual(0);
  let geworfen = false;
  for (let t = 0; t <= 13_000; t += TICK_MS) {
    combat.stepPassive(t);
    // **`isReady` statt eines festen Zeitpunkts.** KINDLE füllt jede
    // Wartezeit; bei `t === WURF_MS` war der GCD von seinem Wurf bei 6000 noch
    // belegt, und der Kampf wies CRUCIBLE SKIN ab. Gemessen: alle drei Größen
    // standen mit und ohne Zauber auf null — der Fall hätte „wirkt nicht"
    // gemeldet, obwohl er nie geworfen wurde.
    if (mitZauber && !geworfen && t >= WURF_MS && combat.isReady(crucible, t)) {
      expect(combat.cast(crucible, t).result, 'crucible-skin wurde abgewiesen').toBe(CAST_OK);
      geworfen = true;
    } else if (combat.isReady(kindle, t)) {
      // KINDLE ist kein Beiwerk: Ohne ausgeteilten Schaden hat Weg 3 nichts
      // umzuwandeln, und `converted` stünde in beiden Läufen auf null.
      combat.cast(kindle, t);
    }
  }
  if (mitZauber) expect(geworfen, 'crucible-skin kam nie zum Zug').toBe(true);
  return {
    geheilt: combat.healed,
    absorbiert: combat.shieldAbsorbed,
    erlitten: combat.damageTaken,
    umgewandelt: combat.converted,
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

  /**
   * **Die Größen, nicht nur ihre Richtung** (nachgetragen am 2026-08-10).
   *
   * Die Fälle darüber prüfen, DASS der Zauber wirkt: `toBeGreaterThan(ohne)`.
   * Halbiert jemand `shield.amount`, fängt die Schicht immer noch mehr ab als
   * gar keine — der Fall bleibt grün, und der Zauber ist halb so stark.
   *
   * Gemessen hat das der volle Lauf von `tools/groessen-mutation.py` am
   * 2026-08-10: Alle vier Zahlen dieses Zaubers standen als „geschuetzt", und
   * die vier Prüffälle dahinter waren `coverage`, `jede-groesse-ist-gemessen`,
   * `sound-triggers` und die Frische-Prüfung — Listen und Frische, nicht
   * Wirkung. Sie werden rot, weil die Aufzeichnung nicht mehr zur Datei passt;
   * wer sie neu erzeugt, ist die Meldung los.
   *
   * Zum Vergleich: POULTICE, MEND, SUTURE und RIPOSTE tragen 8 bis 17
   * Zusicherungen auf konkrete Zahlen. Dieser Fall trug eine.
   */
  it('nagelt die Beträge fest, nicht nur ihre Richtung', () => {
    const zauber = CALCINATION.value.spells.find((s) => s.id === 'crucible-skin');
    // Die Datensatz-Werte zuerst — sie sind die Quelle der Zahlen darunter.
    expect(zauber!.shield!.amount).toBe(1800);
    expect(zauber!.shield!.durationMs).toBe(6000);
    expect(zauber!.shield!.maxStacks).toBe(1);
    expect(zauber!.heal!.amount).toBe(2000);
    expect(zauber!.conversion!.sharePercent).toBe(50);
    expect(zauber!.conversion!.durationMs).toBe(6000);

    const mit = lauf(true);
    const ohne = lauf(false);
    // **Alle Zahlen sind GEMESSEN, nicht hergeleitet.** Mein erster Entwurf
    // trug 960 — ausgerechnet aus „der Boss teilt 960 aus, die Schicht trägt
    // 1800". Gemessen waren es 240, weil WARD vorher mindert und der Schlag
    // nur teilweise in die Schicht fällt. Eine plausible Herleitung ist keine
    // Messung.
    expect(ohne.erlitten, 'ohne den Zauber kommt anderer Schaden an').toBe(3200);
    expect(mit.absorbiert, 'die Schicht fängt einen anderen Betrag ab').toBe(1600);
    expect(mit.geheilt, 'es wird ein anderer Betrag geheilt').toBe(1600);
    expect(mit.umgewandelt, 'es wird ein anderer Betrag umgewandelt').toBe(150);
    // Der eigentliche Beleg für Weg 1: Mit dem Zauber kommt weniger an, und
    // zwar um genau den abgefangenen Betrag.
    expect(ohne.erlitten - mit.erlitten, 'die Schicht mindert nicht um ihren Betrag').toBe(1600);
  });
});
