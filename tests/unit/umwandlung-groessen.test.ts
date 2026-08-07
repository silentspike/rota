import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Die GRÖSSEN der Umwandlung — Issue #592, vier Lücken auf einmal.
 *
 * ```text
 *   conversion.sharePercent  temper       56,00 %
 *   conversion.durationMs    temper       50,00 %
 *   buff.factor              quicksilver  24,11 %
 *   buff.gcds                quicksilver  24,11 %
 * ```
 *
 * Alle vier wirken über `converted` — die Größe, die vor #576 gar nicht
 * gemessen wurde. Im Schadenshebel standen sie auf **0,00 %**, und das sah aus
 * wie „belanglos". Es hieß: falsch gemessen.
 *
 * # Was hier wovon abhängt
 *
 * `convertDamageToShield` wandelt **ausgeteilten** Schaden in Schutz um. Der
 * Satz ist `sharePercent × factor(convertBuffs)` — TEMPER stellt den Satz,
 * QUICKSILVER verdoppelt ihn. Beide haben zusätzlich eine Dauer, und die ist
 * eine eigene Größe.
 *
 * Ein erster Entwurf maß gegen **erlittenen** Schaden und bekam durchweg
 * `converted: 0` — die Umwandlung hängt an dem, was der Spieler austeilt. Das
 * ist dieselbe Falle wie bei SALVE: Ohne die richtige Lage misst der Fall
 * nichts und ist trotzdem grün.
 *
 * # Die Zahlen stehen hart
 *
 * KINDLE richtet 100 an. Bei 25 % sind das 25 Schutz, mit QUICKSILVER 50. Wer
 * die Design-Werte ändert, ändert diese Zeilen mit.
 */

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/** KINDLE als Schadensquelle — 100 fest, damit die Prozente aufgehen. */
const KINDLE_SCHADEN = 100;

describe('TEMPER wandelt 25 % des ausgeteilten Schadens um', () => {
  /**
   * **Die Zusicherung vor der Messung.** Richtet KINDLE nicht die erwarteten
   * 100 an, stimmen alle Prozentzahlen darunter nicht mehr — und der Fall
   * wäre grün mit einer falschen Begründung.
   */
  it('KINDLE richtet die 100 an, mit denen hier gerechnet wird', () => {
    const c = neu();
    c.cast(c.indexOf('kindle'), 0);
    expect(c.damageDealt, 'KINDLE richtet nicht 100 an — die Prozente unten gehen nicht auf').toBe(
      KINDLE_SCHADEN,
    );
  });

  it('aus 100 Schaden werden 25 Schutz', () => {
    const c = neu();
    c.cast(c.indexOf('temper'), 0);
    c.cast(c.indexOf('kindle'), 1500);
    expect(c.converted, 'TEMPER wandelt nicht die 25 % aus dem Design um').toBe(25);
  });

  /** Ohne TEMPER wandelt niemand um — sonst käme es woanders her. */
  it('ohne TEMPER bleibt umgewandelt bei null', () => {
    const c = neu();
    c.cast(c.indexOf('kindle'), 0);
    expect(c.converted, 'es wird umgewandelt, ohne dass jemand TEMPER wirkt').toBe(0);
  });

  /**
   * **Die Dauer ist eine eigene Größe.** Bei 10 000 ms Fenster wandelt ein
   * Schlag nach 11 000 ms nichts mehr um. Wird `durationMs` halbiert, hört es
   * schon bei 5000 ms auf — dieser Fall bleibt grün, der darüber nicht.
   */
  it('nach dem Fenster wandelt es nicht mehr um', () => {
    const c = neu();
    c.cast(c.indexOf('temper'), 0);
    for (let t = 0; t <= 11000; t += 10) c.stepPassive(t);
    c.cast(c.indexOf('kindle'), 11000);
    expect(c.converted, 'die Umwandlung läuft über ihr Fenster hinaus').toBe(0);
  });

  it('innerhalb des Fensters wandelt es um', () => {
    const c = neu();
    c.cast(c.indexOf('temper'), 0);
    for (let t = 0; t <= 9000; t += 10) c.stepPassive(t);
    c.cast(c.indexOf('kindle'), 9000);
    expect(c.converted, 'die Umwandlung endet vor ihrem Fenster').toBe(25);
  });
});

describe('QUICKSILVER verdoppelt den Satz', () => {
  it('aus 25 werden 50', () => {
    const c = neu();
    c.cast(c.indexOf('quicksilver'), 0);
    c.cast(c.indexOf('temper'), 1500);
    c.cast(c.indexOf('kindle'), 3000);
    expect(c.converted, 'QUICKSILVER verdoppelt den Satz nicht — Faktor 2 aus dem Design').toBe(50);
  });

  /**
   * **Die Dauer des Verstärkers, gemessen an ihrer Wirkung.** Nach vier GCDs
   * ist QUICKSILVER abgelaufen; ein Schlag danach wandelt wieder mit dem
   * einfachen Satz um. Wird `gcds` halbiert, endet die Verdopplung früher —
   * und der Wert bei 3000 ms fällt auf 25.
   */
  it('nach dem Fenster gilt wieder der einfache Satz', () => {
    const c = neu();
    c.cast(c.indexOf('quicksilver'), 0);
    c.cast(c.indexOf('temper'), 1500);
    for (let t = 0; t <= 9000; t += 10) c.stepPassive(t);
    c.cast(c.indexOf('kindle'), 9000);
    expect(c.converted, 'die Verdopplung läuft über ihre GCDs hinaus').toBe(25);
  });
});
