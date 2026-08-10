/**
 * Die vier Buff-Größen von DISTIL, FLASHPOINT und VIRULENCE (Issue #547).
 *
 * # Warum diese drei zusammen
 *
 * `tools/groessen-mutation.py` hat am 2026-08-10 gemessen: Diese drei Zauber
 * hatten **alle vier** ihrer Buff-Felder ohne Wirkungsprüfung — zwölf der
 * damals 26 Lücken. Sie stammen aus #540, wo fünf Verstärkungen auf einmal
 * gebaut wurden; `griplock` und `slag` haben inzwischen Fälle bekommen, diese
 * drei nicht.
 *
 * Ein gemeinsamer Fall ist hier richtig und nicht bequem: Die drei
 * unterscheiden sich in genau einer Sache — worauf ihr Faktor wirkt. Alles
 * andere ist dasselbe, und das macht die **Kreuzprobe** möglich, die diese
 * Datei trägt.
 *
 * # Was je Zauber geprüft wird
 *
 * ```text
 *   factor   die Wirkung verdoppelt sich          gemessen an einer Zahl des Kampfes
 *   kind     der Zwilling ERSETZT, statt zu stapeln
 *   target   der Faktor wirkt auf SEINE Größe — und auf keine andere
 *   gcds     nach N Wartezeiten ist er weg
 * ```
 *
 * # Die Träger sind gemessen, nicht gewählt
 *
 * | Verstärker   | Ziel       | Träger    | Größe                          |
 * | ------------ | ---------- | --------- | ------------------------------ |
 * | `flashpoint` | `resource` | KINDLE    | Hitze-Gewinn: 3 → 6            |
 * | `distil`     | `cdreduce` | PYROCLASM | WISPs Verkürzung: 500 → 1000   |
 * | `virulence`  | `stack`    | FESTER    | Schaden je Stapel: 95 → 114    |
 *
 * **PYROCLASM statt KINDLE bei DISTIL, und das ist der Fund dieser Datei.**
 * Der erste Entwurf maß die Verkürzung an KINDLE — dessen Abklingzeit ist
 * 1500 ms, genau eine Wartezeit. Sie ist damit schon am Boden, und jede
 * Verkürzung läuft ins Leere:
 *
 * ```text
 *   ohne DISTIL   KINDLE-Restzeit 1500 -> 0
 *   mit  DISTIL   KINDLE-Restzeit 1500 -> 0     kein Unterschied
 * ```
 *
 * Genau davor hat der Orchestrator am 2026-08-09 gewarnt (Nachricht 5710),
 * nachdem Worker-2 zweimal darauf hereingefallen war. Die Warnung war richtig,
 * und sie hat mich trotzdem nicht davor bewahrt — erst die Messung hat es
 * gezeigt. **Eine Probe braucht einen Träger, bei dem die Wirkung Platz hat.**
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/** Eine Wartezeit — der Abstand, in dem die Verstärker geworfen werden. */
const GCD_MS = 1500;

function kampf(): Combat {
  // `test-crucible-trial` teilt nicht aus und hat 999999 Leben: Der Lauf endet
  // nicht vorzeitig, und nichts stört die gemessenen Größen.
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  return new Combat({ school: CALCINATION.value, boss, seed: 0 });
}

/** Wirft einen Zauber und meldet, ob er durchkam. */
function wirf(combat: Combat, id: string, timeMs: number): boolean {
  const index = alsSpellIndex(combat.indexOf(id));
  expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
  combat.stepPassive(timeMs);
  return combat.cast(index, timeMs).result === CAST_OK;
}

/**
 * Wirft die Verstärker nacheinander und gibt die Zeit zurück, die danach gilt.
 *
 * **Mit Zusicherung.** Wird einer abgewiesen (Wartezeit, Abklingzeit), misst
 * der Vergleich danach einen Lauf mit weniger Verstärkern — und wäre grün,
 * ohne etwas zu zeigen. Genau das ist die Sorte Fehler, die dieser ganzen
 * Datei zugrunde liegt.
 */
function verstaerken(combat: Combat, ids: readonly string[]): number {
  let zeit = 0;
  for (const id of ids) {
    expect(wirf(combat, id, zeit), `${id} wurde abgewiesen`).toBe(true);
    zeit += GCD_MS;
  }
  return zeit;
}

/** FLASHPOINT wirkt auf `resource`: KINDLEs Hitze-Gewinn. */
function hitzeNachKindle(verstaerker: readonly string[]): number {
  const combat = kampf();
  const zeit = verstaerken(combat, verstaerker);
  const vorher = combat.resource;
  wirf(combat, 'kindle', zeit);
  return combat.resource - vorher;
}

/**
 * DISTIL wirkt auf `cdreduce`: um wie viel WISP eine Abklingzeit verkürzt.
 *
 * Gemessen an PYROCLASM (45 s), nicht an KINDLE (1,5 s) — siehe Kopf. Die
 * `GCD_MS` werden abgezogen, weil zwischen den beiden Ablesungen eine
 * Wartezeit vergeht und die Uhr ohnehin läuft.
 */
function verkuerzungAnPyroclasm(verstaerker: readonly string[]): number {
  const combat = kampf();
  let zeit = verstaerken(combat, verstaerker);
  expect(wirf(combat, 'pyroclasm', zeit), 'PYROCLASM wurde abgewiesen').toBe(true);
  const vorher = combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit);
  zeit += GCD_MS;
  expect(wirf(combat, 'wisp', zeit), 'WISP wurde abgewiesen').toBe(true);
  const nachher = combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit);
  return vorher - nachher - GCD_MS;
}

/** VIRULENCE wirkt auf `stack`: FESTERs Schaden je liegendem Stapel. */
function festerSchaden(verstaerker: readonly string[]): number {
  const combat = kampf();
  let zeit = verstaerken(combat, verstaerker);
  expect(wirf(combat, 'canker', zeit), 'CANKER wurde abgewiesen').toBe(true);
  zeit += GCD_MS;
  const index = alsSpellIndex(combat.indexOf('fester'));
  combat.stepPassive(zeit);
  const ergebnis = combat.cast(index, zeit);
  expect(ergebnis.result, 'FESTER wurde abgewiesen').toBe(CAST_OK);
  return ergebnis.damage;
}

describe('FLASHPOINT — der Faktor auf den Ressourcen-Aufbau', () => {
  it('verdoppelt den Hitze-Gewinn (buff.factor, buff.target)', () => {
    expect(hitzeNachKindle([]), 'KINDLE bringt nicht 3 Hitze').toBe(3);
    expect(hitzeNachKindle(['flashpoint']), 'FLASHPOINT verdoppelt nicht').toBe(6);
  });

  it('ERSETZT seinen Zwilling, statt sich zu stapeln (buff.kind)', () => {
    // `test-heat` trägt dieselbe Art (0) und dasselbe Ziel. Stapelten sie,
    // stünden hier 12 — GAME-DESIGN 11.3 verbietet das ausdrücklich.
    expect(hitzeNachKindle(['flashpoint', 'test-heat'])).toBe(6);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    // Ein vertauschtes `target` fiele sonst nicht auf: Der Faktor wirkte,
    // nur woanders. Diese beiden Zeilen sind der einzige Ort, an dem das
    // sichtbar wird.
    expect(verkuerzungAnPyroclasm(['flashpoint']), 'FLASHPOINT wirkt auf cdreduce').toBe(500);
    expect(festerSchaden(['flashpoint']), 'FLASHPOINT wirkt auf stack').toBe(95);
  });
});

describe('DISTIL — der Faktor auf Abklingzeit-Verkürzungen', () => {
  it('verdoppelt WISPs Verkürzung (buff.factor, buff.target)', () => {
    expect(verkuerzungAnPyroclasm([]), 'WISP verkürzt nicht um 500 ms').toBe(500);
    expect(verkuerzungAnPyroclasm(['distil']), 'DISTIL verdoppelt nicht').toBe(1000);
  });

  it('ERSETZT seinen Zwilling, statt sich zu stapeln (buff.kind)', () => {
    expect(verkuerzungAnPyroclasm(['distil', 'test-cdreduce'])).toBe(1000);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    expect(hitzeNachKindle(['distil']), 'DISTIL wirkt auf resource').toBe(3);
    expect(festerSchaden(['distil']), 'DISTIL wirkt auf stack').toBe(95);
  });
});

describe('VIRULENCE — der Faktor auf die Stapel-Wertigkeit', () => {
  it('erhöht FESTERs Schaden je Stapel (buff.factor, buff.target)', () => {
    expect(festerSchaden([]), 'FESTER richtet ohne Verstärker nicht 95 an').toBe(95);
    expect(festerSchaden(['virulence']), 'VIRULENCE erhöht den Stapel-Wert nicht').toBe(114);
  });

  it('ERSETZT seinen Zwilling, statt sich zu stapeln (buff.kind)', () => {
    expect(festerSchaden(['virulence', 'test-stack'])).toBe(114);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    expect(hitzeNachKindle(['virulence']), 'VIRULENCE wirkt auf resource').toBe(3);
    expect(verkuerzungAnPyroclasm(['virulence']), 'VIRULENCE wirkt auf cdreduce').toBe(500);
  });
});

describe('Das Fenster endet — buff.gcds', () => {
  /**
   * Wirft den Verstärker und lässt danach `wartezeiten` Wartezeiten
   * verstreichen, bevor der Träger drankommt.
   *
   * **Die Wartezeiten müssen VERBRAUCHT werden, nicht nur vergehen.** Ein
   * Fenster in Wartezeiten zählt gewirkte Zauber, nicht die Uhr — deshalb
   * füllt SCAB (1,5 s Abklingzeit) jede davon.
   */
  function hitzeNach(wartezeiten: number): number {
    const combat = kampf();
    expect(wirf(combat, 'flashpoint', 0), 'FLASHPOINT wurde abgewiesen').toBe(true);
    let zeit = GCD_MS;
    for (let i = 0; i < wartezeiten; i++) {
      expect(wirf(combat, 'scab', zeit), `Füller ${i + 1} wurde abgewiesen`).toBe(true);
      zeit += GCD_MS;
    }
    const vorher = combat.resource;
    wirf(combat, 'kindle', zeit);
    return combat.resource - vorher;
  }

  it('FLASHPOINT hält drei Wartezeiten — der DRITTE Zauber ist der letzte', () => {
    // **Gemessen, nicht abgezählt.** Mein erster Entwurf setzte „drei
    // Wartezeiten" mit „drei Füller danach" gleich und war rot. Der Wurf
    // SELBST verbraucht die erste:
    //
    //   Füller 0  KINDLE ist Zauber 2 im Fenster   -> 6
    //   Füller 1  KINDLE ist Zauber 3, der letzte  -> 6
    //   Füller 2  KINDLE ist Zauber 4, draußen     -> 3
    //
    // `buff.gcds: 3` heißt also: der wirkende Zauber und zwei weitere.
    expect(hitzeNach(0), 'unmittelbar nach dem Wurf wirkt er nicht').toBe(6);
    expect(hitzeNach(1), 'der dritte Zauber liegt nicht mehr im Fenster').toBe(6);
    expect(hitzeNach(2), 'das Fenster endet nicht nach dem dritten Zauber').toBe(3);
  });
});
