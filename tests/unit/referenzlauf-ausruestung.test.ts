/**
 * Der Referenzlauf trägt Ausrüstung — als ZWEITER Weg (GAME-DESIGN 20.5, #791).
 *
 * **Woher die Frage kommt.** 20.5 verlangt, Bosse gegen den erwarteten
 * Ausrüstungsstand ihrer Instanz zu kalibrieren statt gegen einen nackten
 * Spieler. `runReference` baute den Kampf bisher ohne `worn` — nachweisbar an
 * der Zahl, nicht nur am Quelltext.
 *
 * **Warum nicht einfach Ausrüstung einsetzen.** Der nackte Lauf ist ein fester
 * Bezugspunkt: Zwei Rotationen zu vergleichen verlangt, dass sich zwischen den
 * Läufen nur die Rotation ändert. Deshalb prüft diese Datei BEIDE Richtungen —
 * dass der alte Weg unverändert ist UND dass der neue ankommt. Ein Test, der
 * nur die Wirkung zeigt, ließe offen, ob der Bezugspunkt dabei verlorenging.
 */

import { describe, expect, it } from 'vitest';

import { runReference } from '../../src/devtools/reference-run.js';
import { NOTHING_WORN, type WornValues } from '../../src/gear/worn.js';

/** Ausrüstung, die nur CATALYSIS trägt — die Größe mit dem klarsten Ausschlag. */
const MIT_CATALYSIS: WornValues = { ...NOTHING_WORN, catalysis: 500 };

describe('#791 — runReference nimmt Ausrüstung entgegen', () => {
  /**
   * **Träger-Zusicherung.** Alles Weitere vergleicht Schadenszahlen. Ist die
   * Zahl null, wäre jeder Vergleich unten richtig und nichtssagend.
   */
  it('der Läufer liefert überhaupt Schaden', () => {
    expect(runReference('good').damage).toBeGreaterThan(0);
  });

  /**
   * Der Bezugspunkt bleibt. Ohne `worn` muss dieselbe Zahl herauskommen wie
   * ohne das Feld — sonst hat der Zusatz den ersten Weg verändert.
   */
  it('ohne Angabe rechnet er weiter nackt', () => {
    const ohneFeld = runReference('good');
    const mitLeerem = runReference('good', {});
    expect(mitLeerem.damage).toBe(ohneFeld.damage);
  });

  /**
   * **Der eigentliche Nachweis: die Leitung ist angeschlossen.**
   *
   * Ein Feld im Typ und ein Eintrag in `GUELTIGE_OPTIONEN` beweisen nur, dass
   * der Wert entgegengenommen wird — nicht, dass er im Kampf ankommt. Genau
   * diese Lücke hat in diesem Projekt schon sechs Effekte grün und wirkungslos
   * dastehen lassen.
   */
  it('mit Ausrüstung rechnet er anders', () => {
    const nackt = runReference('good', { stopOnKill: false });
    const angezogen = runReference('good', { stopOnKill: false, worn: MIT_CATALYSIS });
    expect(angezogen.damage).toBeGreaterThan(nackt.damage);
  });

  /**
   * Die Gegenrichtung: `NOTHING_WORN` ausdrücklich übergeben muss dasselbe
   * ergeben wie nichts übergeben. Sonst hinge die Zahl nicht an den Werten,
   * sondern daran, DASS ein Objekt kam.
   */
  it('ausdrücklich leere Ausrüstung ist wie keine', () => {
    const ohne = runReference('good', { stopOnKill: false });
    const leer = runReference('good', { stopOnKill: false, worn: NOTHING_WORN });
    expect(leer.damage).toBe(ohne.damage);
  });

  /**
   * Der Wächter aus #338 muss weiter greifen. Er ist aus dem Typ abgeleitet,
   * also könnte ein neues Feld ihn versehentlich weiten — hier steht, dass er
   * ein erfundenes Feld immer noch abweist.
   */
  it('ein erfundenes Feld wird weiterhin abgewiesen', () => {
    expect(() =>
      runReference('good', { wornn: NOTHING_WORN } as unknown as { worn?: WornValues }),
    ).toThrow(/wornn/);
  });
});
