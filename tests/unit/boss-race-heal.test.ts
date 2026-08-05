/**
 * Das Wettrennen gegen einen Zauber und der Fehler, der den Boss heilt
 * (GAME-DESIGN 11.2, Bausteine 4 und 5; Issue #218).
 *
 * WARUM BEIDE IN EINER DATEI
 * --------------------------
 * Alle bisherigen Bausteine wirken auf den **Spieler** — sie teilen Schaden
 * aus oder töten. Diese beiden wirken auf den **Fortschritt**: Der eine setzt
 * den ausgeteilten Schaden zeitweise auf null, der andere dreht ihn zurück.
 *
 * DIE PRÜFFÄLLE MESSEN, DASS DER KAMPF ES WIRKT
 * ---------------------------------------------
 * Nicht, dass die Daten das Feld kennen. Drei Issues dieser Woche (#201, #219,
 * #223) sind alle daran entstanden, dass ein Datensatz etwas sagte und kein
 * Aufruf es las — eine fertige Mechanik, die niemand benutzt.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';

const RACER = bossById('test-racer');
if (RACER === undefined) throw new Error('data/bosses/test-racer.json fehlt.');

function neuerKampf(boss = RACER): Combat {
  return new Combat({ school: CALCINATION.value, boss: boss as never });
}

/** Bis `bisMs` laufen lassen und dabei so oft wie erlaubt KINDLE wirken. */
function schlagen(combat: Combat, bisMs: number, abMs = 0): void {
  const kindle = combat.indexOf('kindle');
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) {
    combat.stepPassive(t);
    if (t >= abMs && combat.gcd.isReady(t) && combat.isReady(kindle, t)) combat.cast(kindle, t);
  }
}

describe('AC1 — im Fenster kommt kein Schaden am Boss an, aber er ist nicht weg', () => {
  it('bucht ihn als VERLOREN statt ihn verschwinden zu lassen', () => {
    const combat = neuerKampf();
    // Bis kurz vor das Fenster schlagen, dann den Stand merken.
    schlagen(combat, 9990);
    const vorFenster = combat.damageDealt;
    expect(vorFenster).toBeGreaterThan(0);
    expect(combat.raceOpen).toBe(false);
    expect(combat.damageLostToRace).toBe(0);

    // Und jetzt durch das Fenster.
    const kindle = combat.indexOf('kindle');
    for (let t = 10000; t <= 12000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.gcd.isReady(t) && combat.isReady(kindle, t)) combat.cast(kindle, t);
    }

    expect(combat.raceOpen, 'das Fenster steht offen').toBe(true);
    // **Beide Hälften.** „Der Boss nimmt keinen Schaden" allein wäre mit
    // `× 0` erfüllt — und dann fehlte die Zahl in der Fehleranalyse.
    expect(combat.damageDealt, 'am Boss kommt nichts an').toBe(vorFenster);
    expect(combat.damageLostToRace, 'die Zahl kommt bei den Verlusten an').toBeGreaterThan(0);
  });

  it('zählt den verlorenen Schaden nicht in die Zauber-Tabelle', () => {
    // Sonst stimmte die Summe der Tabelle nicht mehr mit dem ausgeteilten
    // Schaden überein — und die Tabelle ist das Werkzeug, mit dem tote Zauber
    // gefunden werden.
    const combat = neuerKampf();
    schlagen(combat, 13000);
    const summe = combat.spells.reduce((s, _, i) => s + combat.damageOf(alsSpellIndex(i)), 0);
    expect(summe).toBe(combat.damageDealt);
    expect(combat.damageLostToRace).toBeGreaterThan(0);
  });
});

describe('AC2 — das Fenster hat BEIDE Ausgänge', () => {
  it('endet früh, wenn der Schaden die Schwelle erreicht', () => {
    const combat = neuerKampf();
    schlagen(combat, 16000);

    // Genug Schaden im Fenster: Das Ziel ist zerstört, das Fenster endete
    // vor Ablauf.
    expect(combat.raceWon, 'das Ziel wurde zerstört').toBe(true);
    expect(combat.raceProgress).toBeGreaterThanOrEqual(RACER.race!.threshold);
    expect(combat.raceOpen).toBe(false);
  });

  it('endet nach Ablauf, wenn die Schwelle NICHT erreicht wird', () => {
    // Derselbe Boss, dieselbe Zeit — nur wirkt der Spieler nichts. Der
    // Unterschied kommt allein daher.
    const combat = neuerKampf();
    combat.beginFight(0);
    for (let t = 0; t <= 16000; t += TICK_MS) combat.stepPassive(t);

    expect(combat.raceWon, 'ohne Schaden ist nichts zerstört').toBe(false);
    expect(combat.raceProgress).toBe(0);
    // Abgelaufen ist es trotzdem: Das Fenster hängt an der Uhr.
    expect(combat.raceOpen).toBe(false);
  });

  it('steht dazwischen wirklich offen — sonst prüften beide Fälle dasselbe', () => {
    const combat = neuerKampf();
    combat.beginFight(0);
    for (let t = 0; t <= 12000; t += TICK_MS) combat.stepPassive(t);
    expect(combat.raceOpen).toBe(true);
  });
});

describe('AC3 — ein durchgekommener Boss-Zauber dreht Fortschritt zurück', () => {
  it('lässt den Fortschritt messbar rückwärts laufen', () => {
    const combat = neuerKampf();
    // Bis nach dem Fenster schlagen, damit Fortschritt da ist, den man
    // zurückdrehen kann.
    schlagen(combat, 19990);
    const vorDemZauber = combat.damageDealt;
    const anteilVorher = combat.bossHpPercent;
    expect(vorDemZauber).toBeGreaterThan(RACER.bossHeal!.amount);

    // Der Zauber beginnt bei 20 s und trifft nach 3 s Wirkzeit — niemand
    // unterbricht ihn.
    for (let t = 20000; t <= 24000; t += TICK_MS) combat.stepPassive(t);

    expect(combat.interruptsMissed, 'der Zauber kam durch').toBe(1);
    expect(combat.bossHealed).toBe(RACER.bossHeal!.amount);
    expect(combat.damageDealt).toBe(vorDemZauber - RACER.bossHeal!.amount);
    // Der Balken läuft rückwärts — und zwar sichtbar, nicht nur rechnerisch.
    expect(combat.bossHpPercent).toBeGreaterThan(anteilVorher);
  });

  it('dreht nie unter null zurück', () => {
    // Ein Boss, der mehr „geheilt" wird, als je Schaden gefallen ist, hätte
    // negativen Fortschritt — und jede Prozentrechnung darüber wäre unsinnig.
    const combat = neuerKampf();
    combat.beginFight(0);
    for (let t = 0; t <= 24000; t += TICK_MS) combat.stepPassive(t);

    expect(combat.damageDealt).toBe(0);
    expect(combat.bossHealed).toBe(0);
  });
});

describe('AC5 — ein Boss ohne die Felder bleibt unberührt', () => {
  it('hat weder Fenster noch Heilung, und beide Größen stehen auf null', () => {
    const ohne = FIRST_CRUCIBLE.value;
    expect(ohne.race).toBeNull();
    expect(ohne.bossHeal).toBeNull();

    const combat = neuerKampf(ohne);
    schlagen(combat, 20000);

    expect(combat.raceOpen).toBe(false);
    expect(combat.raceProgress).toBe(0);
    expect(combat.damageLostToRace).toBe(0);
    expect(combat.bossHealed).toBe(0);
    // Und der Schaden kommt ungehindert an.
    expect(combat.damageDealt).toBeGreaterThan(0);
  });
});

describe('Der Wächter: der KAMPF wirkt es, nicht nur die Daten', () => {
  it('unterscheidet den Boss MIT Fenster vom Boss OHNE — bei gleicher Rotation', () => {
    // **Der Fall, der die drei Issues dieser Woche gefangen hätte.** Ein
    // Datensatz, den niemand liest, sähe genauso aus wie einer, der wirkt —
    // solange man nur die Daten prüft.
    //
    // Zwei Läufe, gleiche Zeit, gleiche Rotation, verschiedene Bosse: Der
    // Unterschied kann nur aus dem Fenster kommen.
    const mit = neuerKampf();
    const ohne = neuerKampf(FIRST_CRUCIBLE.value);
    schlagen(mit, 14000);
    schlagen(ohne, 14000);

    expect(mit.damageLostToRace).toBeGreaterThan(0);
    expect(ohne.damageLostToRace).toBe(0);
    expect(mit.damageDealt).toBeLessThan(ohne.damageDealt);
  });

  it('und der verlorene Schaden ist genau der Unterschied', () => {
    // Ohne diese Zusicherung bliebe offen, ob das Fenster den Schaden
    // verschluckt oder nur verschiebt. Beide Läufe wirken dieselbe Rotation;
    // was dem einen fehlt, muss beim anderen bei den Verlusten stehen.
    const mit = neuerKampf();
    const ohne = neuerKampf(FIRST_CRUCIBLE.value);
    // Bis kurz vor die Schwelle: danach laufen die Läufe auseinander, weil
    // das Fenster im einen frueher endet.
    schlagen(mit, 11000);
    schlagen(ohne, 11000);

    expect(mit.damageDealt + mit.damageLostToRace).toBe(ohne.damageDealt);
  });

  it('GCD und Abklingzeiten laufen im Fenster normal weiter', () => {
    // Das Fenster nimmt dem Schaden die Wirkung, nicht dem Spieler die
    // Handlung. Wer im Fenster nicht wirken könnte, hätte keine Chance, das
    // Ziel zu zerstören — die Mechanik widerspräche sich selbst.
    const combat = neuerKampf();
    schlagen(combat, 12000);
    expect(combat.castCount(combat.indexOf('kindle'))).toBeGreaterThan(Math.floor(10000 / GCD_MS));
  });
});
