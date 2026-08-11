/**
 * AETHERs zwei Buff-Größen (Issue #480).
 *
 * # Warum dieser Fall entsteht
 *
 * `tools/groessen-mutation.py` hat am 2026-08-11 gemessen:
 *
 * ```text
 *   buff.factor  aether   2 -> 1    NUR FRISCHE -> faktisch ungeschützt
 *   buff.gcds    aether   8 -> 4    NUR FRISCHE -> faktisch ungeschützt
 * ```
 *
 * „Nur frische" heißt: Es färbt allein die Vergleichsdatei, kein einziger
 * Prüffall. **Und eine veraltete Aufzeichnung zieht man nach, ohne zu fragen
 * warum** — der Unterschied zwischen einer Sperre und einer Bremse.
 *
 * # AETHER ist DISTILs längere Schwester, und das ist der Grund
 *
 * Sein eigener Kommentar in `data/spells/sublimation.json` sagt es: „dieselbe
 * Bauart (buff auf cdreduce), doppelte Dauer, viermal die Abklingzeit."
 *
 * | | `factor` | `gcds` | `target` | `kind` | Abklingzeit |
 * | ------ | -: | -: | --- | -: | -: |
 * | DISTIL | 2 | 4 | cdreduce | 0 | 15 s |
 * | AETHER | 2 | 8 | cdreduce | 0 | 60 s |
 *
 * DISTIL wurde in #719 abgesichert (`buff-groessen-540.test.ts`), AETHER
 * nicht — **derselbe Bau, dieselben Felder, kein Netz.** Die Prüffälle, die
 * ihn erwähnen, sind Struktur-Tests (`spell-school`, `combat`,
 * `sound-triggers`); keiner misst seine Wirkung.
 *
 * # Der Träger ist gemessen, nicht gewählt
 *
 * Gemessen wird an **PYROCLASM** (45 s Abklingzeit), nicht an KINDLE. KINDLE
 * liegt bei 1,5 s — genau einer Wartezeit — und ist damit schon am Boden;
 * jede Verkürzung liefe ins Leere. Genau darauf ist der DISTIL-Fall bei
 * seinem ersten Entwurf hereingefallen.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/** Eine Wartezeit — der Abstand, in dem geworfen wird. */
const GCD_MS = 1500;

function kampf(): Combat {
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
 * **Mit Zusicherung.** Wird einer abgewiesen, misst der Vergleich danach einen
 * Lauf mit weniger Verstärkern — und wäre grün, ohne etwas zu zeigen.
 */
function verstaerken(combat: Combat, ids: readonly string[]): number {
  let zeit = 0;
  for (const id of ids) {
    expect(wirf(combat, id, zeit), `${id} wurde abgewiesen`).toBe(true);
    zeit += GCD_MS;
  }
  return zeit;
}

/** Um wie viel WISP die Abklingzeit von PYROCLASM verkürzt. */
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

/** KINDLEs Hitze-Gewinn — die fremde Größe für die Kreuzprobe. */
function hitzeNachKindle(verstaerker: readonly string[]): number {
  const combat = kampf();
  const zeit = verstaerken(combat, verstaerker);
  const vorher = combat.resource;
  wirf(combat, 'kindle', zeit);
  return combat.resource - vorher;
}

describe('AETHER — der Faktor auf Abklingzeit-Verkürzungen (buff.factor)', () => {
  it('verdoppelt WISPs Verkürzung', () => {
    expect(verkuerzungAnPyroclasm([]), 'WISP verkürzt ohne Verstärker nicht um 500 ms').toBe(500);
    expect(verkuerzungAnPyroclasm(['aether']), 'AETHER verdoppelt nicht').toBe(1000);
  });

  it('lässt die FREMDEN Größen unberührt — die Kreuzprobe', () => {
    // Ein vertauschtes `target` fiele sonst nicht auf: Der Faktor wirkte, nur
    // woanders. Diese Zeile ist der einzige Ort, an dem das sichtbar wird.
    expect(hitzeNachKindle(['aether']), 'AETHER wirkt auf resource').toBe(3);
  });

  it('ERSETZT DISTIL, statt sich zu stapeln (buff.kind)', () => {
    // **Beide tragen `kind: 0` und dasselbe Ziel.** Stapelten sie, stünde hier
    // 2000 — GAME-DESIGN 11.3 verbietet das ausdrücklich, und die beiden sind
    // dieselbe Bauart in zwei Längen.
    expect(verkuerzungAnPyroclasm(['distil', 'aether'])).toBe(1000);
    expect(verkuerzungAnPyroclasm(['aether', 'distil'])).toBe(1000);
  });
});

describe('AETHER — das Fenster hält acht Wartezeiten (buff.gcds)', () => {
  /**
   * Wirft AETHER und lässt danach `wartezeiten` Wartezeiten verstreichen,
   * bevor WISP an PYROCLASM gemessen wird.
   *
   * **Die Wartezeiten müssen VERBRAUCHT werden, nicht nur vergehen.** Ein
   * Fenster in Wartezeiten zählt gewirkte Zauber, nicht die Uhr — deshalb
   * füllt SCAB (1,5 s Abklingzeit) jede davon.
   */
  function verkuerzungNach(wartezeiten: number): number {
    const combat = kampf();
    expect(wirf(combat, 'aether', 0), 'AETHER wurde abgewiesen').toBe(true);
    let zeit = GCD_MS;
    for (let i = 0; i < wartezeiten; i++) {
      expect(wirf(combat, 'scab', zeit), `Füller ${i + 1} wurde abgewiesen`).toBe(true);
      zeit += GCD_MS;
    }
    expect(wirf(combat, 'pyroclasm', zeit), 'PYROCLASM wurde abgewiesen').toBe(true);
    const vorher = combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit);
    zeit += GCD_MS;
    expect(wirf(combat, 'wisp', zeit), 'WISP wurde abgewiesen').toBe(true);
    const nachher = combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit);
    return vorher - nachher - GCD_MS;
  }

  it('AN DER GRENZE gemessen: fünf Füller halten, sechs nicht mehr', () => {
    // **Die Grenze, nicht zwei weit entfernte Punkte.** Meine erste Fassung
    // prüfte 5 (hält) und 9 (weg) — damit wäre ein Fenster von 6 oder 7
    // ebenfalls grün gewesen. Gemessen kippt es zwischen 5 und 6:
    //
    // ```text
    //   AETHER  0:1000 1:1000 2:1000 3:1000 4:1000 5:1000 6:500 7:500 …
    // ```
    //
    // **`buff.gcds: 8` heißt: der wirkende Zauber und sieben weitere.** Mit
    // fünf Füllern sind PYROCLASM und WISP die Zauber sieben und acht, also
    // gerade noch im Fenster; mit sechs fällt WISP heraus. Dieselbe Zählung
    // wie bei FLASHPOINT (`gcds: 3` = der Wurf und zwei weitere).
    expect(verkuerzungNach(5), 'nach fünf Füllern schon weg — Fenster zu kurz').toBe(1000);
    expect(verkuerzungNach(6), 'nach sechs Füllern noch da — Fenster zu lang').toBe(500);
  });

  it('und wirkt unmittelbar nach dem Wurf', () => {
    // Ohne diese Zeile wäre ein Fenster, das gar nicht erst aufgeht, an der
    // Grenze oben ebenfalls unauffällig.
    expect(verkuerzungNach(0)).toBe(1000);
  });

  it('DISTIL kippt bei ZWEI — der Längenunterschied ist die Aussage', () => {
    // AETHER unterscheidet sich von DISTIL in genau einer Zahl: `gcds` 8
    // statt 4. Wäre der Unterschied nicht messbar, wäre AETHER ein zweiter
    // DISTIL mit viermal so langer Abklingzeit — also strikt schlechter.
    //
    // ```text
    //   DISTIL  0:1000 1:1000 2:500 3:500 …
    //   AETHER  0:1000 … 5:1000 6:500 …
    // ```
    function distilNach(fueller: number): number {
      const combat = kampf();
      expect(wirf(combat, 'distil', 0), 'DISTIL wurde abgewiesen').toBe(true);
      let zeit = GCD_MS;
      for (let i = 0; i < fueller; i++) {
        expect(wirf(combat, 'scab', zeit), `Füller ${i + 1} wurde abgewiesen`).toBe(true);
        zeit += GCD_MS;
      }
      expect(wirf(combat, 'pyroclasm', zeit), 'PYROCLASM wurde abgewiesen').toBe(true);
      const vorher = combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit);
      zeit += GCD_MS;
      expect(wirf(combat, 'wisp', zeit), 'WISP wurde abgewiesen').toBe(true);
      return vorher - combat.cooldownRemainingMs(combat.indexOf('pyroclasm'), zeit) - GCD_MS;
    }
    expect(distilNach(1), 'DISTIL hält nach einem Füller nicht mehr').toBe(1000);
    expect(distilNach(2), 'DISTIL hält nach zwei Füllern noch').toBe(500);
    // Und der eigentliche Punkt: An derselben Stelle trägt AETHER noch.
    expect(verkuerzungNach(2), 'AETHER ist so kurz wie DISTIL').toBe(1000);
  });
});
