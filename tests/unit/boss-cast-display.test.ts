/**
 * Was die Anzeige über den laufenden Boss-Zauber erfährt (Issue #327).
 *
 * WAS ES VORHER GAB: EIN ZUSTAND, DEN NUR DIE PRÜFSUMME LAS
 * ----------------------------------------------------------
 * ```
 * $ grep -rn "\.casting\b" src/ --include=*.ts
 * src/core/checksum.ts:143
 * src/core/checksum.ts:204
 * ```
 *
 * Zwei Fundstellen, beide in der Prüfsumme. Das Unterbrechen ist seit #110
 * gebaut, die Eskalation verkürzt sein Fenster (#240), FIXATION verlängert es
 * (#100) — und nichts davon war zu sehen.
 *
 * **Das verletzt eine Design-Anforderung, nicht nur eine Bequemlichkeit.**
 * GAME-DESIGN 7.1 verlangt für den Abbruch „vorher **erkennbare**
 * Boss-Angriffe", und 21.2 nimmt die Reaktionszeit ausdrücklich aus der
 * Bewertung heraus.
 *
 * WAS HIER GEPRÜFT WIRD UND WAS NICHT
 * -----------------------------------
 * Hier stehen die Zugänge, aus denen der Balken seine Zahlen zieht. Der Balken
 * selbst braucht ein Dokument und ist im Browser belegt (AC1 bis AC5).
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function boss(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

/** `test-crucible-trial` wirkt bei 6000 ms drei Sekunden lang. */
const TRIAL = boss('test-crucible-trial');

function bisMs(combat: Combat, bisMs: number): void {
  for (let t = 0; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
}

function imKampf(ziel: BossDef = TRIAL): Combat {
  const combat = new Combat({ school: CALCINATION.value, boss: ziel });
  combat.beginFight(0);
  return combat;
}

describe('Der Name des laufenden Boss-Zaubers', () => {
  it('steht bereit, sobald einer läuft', () => {
    const combat = imKampf();
    expect(combat.castingName, 'vorher läuft keiner').toBeNull();
    bisMs(combat, 6000);
    expect(combat.casting, 'sonst prüft der Fall einen Kampf ohne Zauber').toBeGreaterThanOrEqual(
      0,
    );
    // Der Name kommt aus dem Boss-Datensatz, nicht aus einer Konstante — die
    // Falle aus #229, an der schon einmal zwei Abschriften hingen.
    expect(combat.castingName).toBe('Cascade');
  });

  it('ist wieder null, wenn der Zauber durchgekommen ist', () => {
    const combat = imKampf();
    bisMs(combat, 9000);
    expect(combat.castingName).toBeNull();
  });
});

describe('Der Anfang des laufenden Boss-Zaubers', () => {
  it('nennt den Zeitpunkt, zu dem er begonnen hat', () => {
    // **Ein Balken braucht Anfang UND Ende.** Das Ende steht in
    // `castLandsAtMs`; ohne den Anfang ließe sich kein Anteil rechnen.
    const combat = imKampf();
    bisMs(combat, 6500);
    expect(combat.castingStartedAtMs).toBe(6000);
    expect(combat.castLandsAtMs, 'drei Sekunden Wirkzeit').toBe(9000);
  });

  it('BLEIBT STEHEN, wenn FIXATION das Ende verschiebt', () => {
    // **Der Grund, aus dem es ein eigenes Feld ist und keine Rechnung.**
    // STALL schiebt `castLandsAtMs` nach hinten, während der Zauber läuft.
    // „Ende minus Dauer" ergäbe danach einen Anfang, den es nie gab — und der
    // Balken spränge zurück, statt länger zu werden.
    const combat = imKampf();
    const bind = combat.indexOf('bind');
    const stall = combat.indexOf('stall');

    // **Die Punkte VOR dem Boss-Zauber holen**, und der Grund ist gemessen:
    // Mein erster Entwurf drückte BIND bei 6500 und 8000 und STALL bei 8010 —
    // die globale Wartezeit aus dem zweiten BIND lief bis 9500, STALL wurde
    // abgewiesen, und das Ende blieb bei 9000. Der Fall meldete „verschiebt
    // nichts", und die Ursache war meine Abfolge, nicht die Umsetzung.
    //
    // BIND gibt zwei Punkte je Wirkung, STALL kostet vier.
    bisMs(combat, 3000);
    combat.cast(bind, 3000);
    bisMs(combat, 4500);
    combat.cast(bind, 4500);
    expect(combat.control, 'ohne vier Punkte verpufft STALL').toBeGreaterThanOrEqual(4);

    bisMs(combat, 6500);
    const anfang = combat.castingStartedAtMs;
    const endeVorher = combat.castLandsAtMs;
    expect(
      combat.casting,
      'der Boss muss wirken, sonst schiebt STALL nichts',
    ).toBeGreaterThanOrEqual(0);
    expect(combat.gcd.isReady(6500), 'sonst weist die Wartezeit den Druck ab').toBe(true);
    combat.cast(stall, 6500);

    expect(combat.castLandsAtMs, 'das Ende rückt nach hinten').toBeGreaterThan(endeVorher);
    expect(combat.castingStartedAtMs, 'der Anfang bleibt, wo er war').toBe(anfang);
  });

  it('ist -1, solange keiner läuft', () => {
    const combat = imKampf();
    expect(combat.castingStartedAtMs).toBe(-1);
  });
});

describe('Ein neuer Versuch fängt ohne laufenden Boss-Zauber an', () => {
  it('räumt den Zustand beim Zurücksetzen', () => {
    // **Gemessen am 2026-08-05, vor dieser Zeile:** Nach `reset()` stand
    // `casting` weiterhin auf 2 und `castLandsAtMs` auf 7000 — der Zustand
    // überlebte den Versuch. Der Simulator räumt ihn seit jeher
    // (`Fight::reset`), das Spiel nicht.
    //
    // Keine Aufzeichnung setzt mitten in einem Boss-Zauber zurück, also hat es
    // nie jemand gemerkt. Aufgefallen ist es erst, als der Balken den Zustand
    // ZEIGT: Nach einem Neustart stünde dort ein Zauber, den es nicht mehr
    // gibt. **Eine Anzeige findet, was keine Rechnung verrät.**
    const combat = imKampf();
    bisMs(combat, 6500);
    expect(combat.casting, 'sonst räumt der Fall nichts').toBeGreaterThanOrEqual(0);

    combat.reset();
    expect(combat.casting).toBe(-1);
    expect(combat.castingName).toBeNull();
    expect(combat.castingStartedAtMs).toBe(-1);
    expect(combat.castLandsAtMs).toBe(-1);
  });
});

describe('Der Abbruch räumt sofort', () => {
  it('nimmt den Zustand mit dem Unterbrechen, nicht erst zum Treffer', () => {
    // AC2: Ein Balken, der bis zum geplanten Trefferzeitpunkt stehen bliebe,
    // behauptete eine Wirkung, die nicht mehr kommt.
    const combat = imKampf();
    bisMs(combat, 6500);
    const ende = combat.castLandsAtMs;
    expect(ende, 'der Treffer läge noch in der Zukunft').toBeGreaterThan(6500);

    combat.cast(combat.indexOf('quench'), 6500);
    expect(combat.casting).toBe(-1);
    expect(combat.castingName).toBeNull();
    expect(combat.castingStartedAtMs).toBe(-1);
    expect(combat.interruptsLanded).toBe(1);
  });
});

describe('Am Balken steht NICHT, ob der Zauber unterbrechbar ist', () => {
  /**
   * **Eine Beschriftung, die immer dasselbe sagt, sagt nichts** — dieselbe
   * Regel wie bei den Widerstands-Zeilen aus #328: „Genau 1 heißt ‚kein
   * Unterschied' und gehört nicht auf den Bildschirm."
   *
   * GAME-DESIGN 11.6 verortet die Markierungen („unterbrechbar · tödlich wenn
   * ignoriert · Gelegenheit · stehlbar · verlangt Schutz") in der
   * **Boss-Übersicht**, also der Vorbereitung. Dort unterscheiden sie zwischen
   * Fähigkeiten. Am laufenden Balken unterscheiden sie zwischen nichts.
   *
   * # WORAN MAN ERKENNT, DASS SICH DAS GEÄNDERT HAT
   *
   * Heute wird `casting` **nur** für `interruptibleCast` gesetzt — ein
   * laufender Boss-Zauber ist damit immer unterbrechbar. Wer einen zweiten
   * Zeitleisten-Typ einführt, der `casting` ebenfalls setzt, hat den Moment
   * erzeugt: Ab dann trägt der Balken eine Aussage, die nicht mehr für alle
   * gilt, und die Markierung gehört an ihn.
   *
   * Dieser Fall ist die Bedingung, nicht die Erinnerung. Ein Vermerk, der nur
   * „später mal" sagt, wird nie gefunden; einer, der die Bedingung prüft, wird
   * von dem gefunden, der die Bedingung herstellt.
   */
  it('setzt `casting` ausschließlich für unterbrechbare Zauber', () => {
    // Über die Daten aller Bösse geprüft, nicht über einen: Ein neuer Typ
    // käme als Datensatz, und dieser Fall soll ihn sehen.
    const alle = ['test-crucible-trial', 'test-metronome', 'test-onslaught', 'test-warden'];
    for (const id of alle) {
      for (const eintrag of boss(id).timeline) {
        if (eintrag.mechanic === 'interruptibleCast') continue;
        expect(
          eintrag.castTimeMs,
          `${id}: ein Eintrag der Art "${eintrag.mechanic}" hat eine Wirkzeit — ` +
            `wird daraus ein laufender Boss-Zauber, ist nicht mehr jeder ` +
            `unterbrechbar, und die Markierung gehört an den Balken`,
        ).toBe(0);
      }
    }
  });

  it('läuft im Kampf, ohne dass die Anzeige eine Art unterscheiden müsste', () => {
    // Der Gegenbeleg zur Regel oben: Solange ein laufender Zauber IMMER
    // unterbrechbar ist, ist `casting >= 0` die ganze Aussage.
    const combat = imKampf();
    bisMs(combat, 6500);
    expect(combat.casting).toBeGreaterThanOrEqual(0);
    expect(combat.castingName).toBe('Cascade');
  });
});

describe('Die Eskalation verkürzt das Fenster — sichtbar in den Zahlen', () => {
  it('gibt jedem Zyklus eine kürzere Wirkzeit', () => {
    // Die Zahlen, aus denen der Balken seine Länge zieht. Im Browser gemessen:
    // 2990, 2490, 1990, 1480 ms Restzeit im ersten Takt jedes Zyklus.
    const combat = imKampf();
    const dauern: number[] = [];
    let vorher = -1;
    for (let t = 0; t <= 45_000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.casting >= 0 && vorher < 0) {
        dauern.push(combat.castLandsAtMs - combat.castingStartedAtMs);
      }
      vorher = combat.casting;
    }

    expect(dauern.length, 'ohne mehrere Zyklen misst der Fall nichts').toBeGreaterThanOrEqual(3);
    expect(dauern[0]).toBe(3000);
    // 500 ms je Zyklus, laut `escalation.castTimeShrinkMs` im Boss-Datensatz.
    expect(dauern[1]).toBe(2500);
    expect(dauern[2]).toBe(2000);
  });
});
