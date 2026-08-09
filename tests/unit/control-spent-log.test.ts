/**
 * Die ausgegebenen Kontrollpunkte stehen im Verlauf (Issue #683).
 *
 * # Was fehlte
 *
 * Das Spiel **zählte** sie längst — `controlSpentValue` wächst an drei
 * Stellen —, schrieb sie aber nicht ins Protokoll. Der Simulator tat es
 * bereits. Damit fehlte der Fehleranalyse ausgerechnet die Größe, an der
 * sich eine vertane Gelegenheit zeigt, und die Analyse läuft im Browser
 * (GAME-DESIGN 15.2, #81).
 *
 * # Drei Ausgabestellen, drei Einträge
 *
 * ```text
 *   der Mindesteinsatz          jeder Kontroll-Zauber, `control.cost`
 *   der Rest bei ALLE DREI      `allThree` leert den Vorrat
 *   der Rest bei UMWANDLUNG     `convertToDamage` ebenso
 * ```
 *
 * Sie schreiben **getrennt**, nicht verrechnet: Wer den Verlauf liest, soll
 * sehen, dass über die Kosten hinaus geleert wurde — sonst sähe eine Ausgabe
 * von 4 + 6 aus wie eine von 10, und der Unterschied ist genau der Punkt der
 * beiden Wirkungen.
 *
 * # Warum der Vorrat erst gefüllt werden muss
 *
 * Gemessen: Ohne einen Erzeuger bleibt `control` über den ganzen Lauf auf 0,
 * und jeder Kontroll-Zauber scheitert am leeren Vorrat. Ein Fall ohne BIND
 * wäre grün, ohne je eine Ausgabe zu sehen.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { CombatLog } from '../../src/core/log.js';
import { TICK_MS } from '../../src/core/loop.js';

const AUSGABE = 'controlSpent';

/**
 * Füllt den Vorrat mit BIND und wirft dann den genannten Kontroll-Zauber.
 *
 * `mindestVorrat` ist die Schwelle, ab der geworfen wird — CAPSTONE kostet
 * acht, und mit einer Schwelle von vier käme er nie durch. Gemessen: bei
 * Schwelle 8 kommen WEDGE achtmal und CAPSTONE zweimal durch.
 */
function laufMitAusgabe(
  kontrollZauber: string,
  mindestVorrat = 8,
): {
  eintraege: number[];
  wuerfe: number;
  hoechsterVorrat: number;
} {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('data/bosses/test-warden.json fehlt.');
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  const log = new CombatLog(8192);
  combat.log = log;
  combat.beginFight(0);

  const bind = alsSpellIndex(combat.indexOf('bind'));
  const kontrolle = alsSpellIndex(combat.indexOf(kontrollZauber));
  let wuerfe = 0;
  let hoechsterVorrat = 0;

  for (let tick = 0; tick < 6000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.control > hoechsterVorrat) hoechsterVorrat = combat.control;

    if (
      kontrolle >= 0 &&
      combat.control >= mindestVorrat &&
      combat.isReady(kontrolle, timeMs) &&
      combat.cast(kontrolle, timeMs).result === CAST_OK
    ) {
      wuerfe++;
    } else if (bind >= 0 && combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }
  }

  const eintraege = log
    .toArray()
    .filter((e) => e.type === AUSGABE)
    .map((e) => e.value);
  return { eintraege, wuerfe, hoechsterVorrat };
}

describe('jede Ausgabe von Kontrollpunkten steht im Verlauf', () => {
  it('die Voraussetzung: der Vorrat wächst und der Zauber kommt durch', () => {
    // **Die Zusicherung vor der Messung.** Bliebe der Vorrat auf 0, käme
    // kein Kontroll-Zauber durch — und alle Fälle darunter wären grün,
    // ohne je eine Ausgabe gesehen zu haben.
    const { wuerfe, hoechsterVorrat } = laufMitAusgabe('stall');
    expect(hoechsterVorrat, 'der Vorrat wächst nicht — fehlt der Erzeuger?').toBeGreaterThanOrEqual(
      4,
    );
    expect(wuerfe, 'STALL kam kein einziges Mal durch').toBeGreaterThan(0);
  });

  it('n Ausgaben erzeugen n Einträge, und die Werte sind die Kosten', () => {
    const { eintraege, wuerfe } = laufMitAusgabe('stall');

    // **Harte Zahl, nicht „mindestens einer".** STALL nimmt nur den
    // Mindesteinsatz — eine Stelle, ein Eintrag je Wurf.
    expect(eintraege.length, 'Würfe und Einträge gehen auseinander').toBe(wuerfe);

    // Der Wert ist die Zahl der Punkte dieser Ausgabe, nicht der Stand
    // danach. STALL kostet vier.
    expect(new Set(eintraege), 'STALL kostet immer dasselbe').toEqual(new Set([4]));
  });

  it('WEDGE leert den Vorrat und schreibt den Rest EIGENS', () => {
    // **Der Unterschied, den getrennte Einträge sichtbar machen.** WEDGE
    // nimmt die Kosten UND den Rest. Würden beide verrechnet, sähe eine
    // Ausgabe von 4 + 6 aus wie eine von 10 — und der eigentliche Punkt
    // dieser Wirkung („aufsparen lohnt sich") wäre aus dem Verlauf nicht
    // mehr abzulesen.
    const { eintraege, wuerfe } = laufMitAusgabe('wedge');

    // **Kein stilles Überspringen.** Ein `if (wuerfe === 0) return` stand
    // hier im ersten Entwurf — und genau daran fiel auf, dass dieser Fall
    // nichts prüfte: Die Gegenprobe zur zweiten Ausgabestelle färbte nicht.
    expect(wuerfe, 'WEDGE kam nicht durch — dann prüft dieser Fall nichts').toBeGreaterThan(0);

    expect(eintraege.length, 'WEDGE schreibt nicht zwei Einträge je Wurf').toBe(wuerfe * 2);
  });

  it('CAPSTONE nimmt alles und schreibt beide Teile', () => {
    // Die dritte Ausgabestelle (`allThree`). CAPSTONE kostet acht und leert
    // danach den Rest — zwei Einträge je Wurf, wie bei WEDGE.
    const { eintraege, wuerfe } = laufMitAusgabe('capstone');
    expect(wuerfe, 'CAPSTONE kam nicht durch — dann prüft dieser Fall nichts').toBeGreaterThan(0);
    expect(eintraege.length, 'CAPSTONE schreibt nicht zwei Einträge je Wurf').toBe(wuerfe * 2);

    // Der Mindesteinsatz steht als eigener Wert da, nicht verrechnet.
    expect(eintraege, 'die acht Punkte des Mindesteinsatzes fehlen').toContain(8);
  });
});
