import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';

/**
 * Die vier umgebauten Talent-Wege wirken — jeder auf seine eigene Größe (#419).
 *
 * # Warum sie umgebaut wurden
 *
 * Gemessen verloren alle vier gegen ihren Nachbarn im selben Baum, und zwar
 * aus einem Grund, der keine Zahlenfrage war:
 *
 * > Ein Weg auf die SUMME schlägt einen Weg auf einen SUMMANDEN, und zwar in
 * > jeder Rotation, die viel von der Summe hat.
 *
 * `effect` multipliziert alle Zersetzungs-Stapel, der alte `build` legte
 * gelegentlich einen dazu; `threshold` verbreitert den Abzug, der alte
 * `reduction` vertiefte ihn; `release` hebt den Entlader direkt, der alte
 * `global-build` fütterte ihn indirekt. Andere Promille hätten die Grenze
 * verschoben, nicht die Richtung.
 *
 * # Was jeder jetzt tut
 *
 * ```text
 *   putrefaction build      hebt die Stapel-OBERGRENZE
 *   putrefaction duration   hebt den SCHUTZ je Stapel        (defensiv)
 *   sublimation  reduction  hebt EFFLUX' UMWANDLUNG          (gegenläufig)
 *   global       build      hebt den Schaden der AUFBAU-Zauber
 * ```
 *
 * # Was dieser Fall NICHT prüft
 *
 * Ob die Zahlen ausgewogen sind — das misst der Balance-Läufer, und die
 * Kipp-Punkte stehen in `sim/tests/ac3_wege_gewinnen.rs`. Hier steht nur, dass
 * jeder Weg seine Größe überhaupt bewegt und die der anderen nicht.
 */

const VOLL = 5;

function kampf(talents?: Record<string, number>, bossId = 'first-crucible'): Combat {
  return new Combat({
    school: CALCINATION.value,
    boss: bossById(bossId) ?? FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
}

describe('putrefaction-build — die Obergrenze', () => {
  it('erlaubt mehr Stapel, ohne mehr zu legen', () => {
    const ohne = kampf();
    const mit = kampf({ 'putrefaction-build': VOLL });
    expect(ohne.decayStackCap, 'die Grundgrenze liegt nicht bei fünf').toBe(5);
    // 5 Ränge à 400 Promille = 2000 -> floor(2) zusätzliche Plätze.
    expect(mit.decayStackCap, 'der Weg hebt die Grenze nicht').toBe(7);
  });

  it('und legt selbst KEINEN Stapel mehr — das war der alte Weg', () => {
    // **Die Trennschärfe zum Vorgänger.** Legte er weiter nach, stünde er
    // wieder gegen `effect` statt daneben.
    const einWurf = (talents?: Record<string, number>) => {
      const combat = kampf(talents);
      combat.cast(alsSpellIndex(combat.indexOf('canker')), 0);
      return combat.decayStacks;
    };
    expect(einWurf()).toBe(1);
    expect(einWurf({ 'putrefaction-build': VOLL }), 'der Weg legt wieder Stapel').toBe(1);
  });
});

describe('putrefaction-duration — der Schutz', () => {
  /** Was der Boss durchbringt, während Stapel liegen und DECAY läuft. */
  function erlitten(talents?: Record<string, number>): number {
    const combat = kampf(talents, 'boss-1-1');
    const canker = alsSpellIndex(combat.indexOf('canker'));
    const blight = alsSpellIndex(combat.indexOf('blight'));
    const decay = alsSpellIndex(combat.indexOf('decay'));
    const hpVorher = combat.playerHp;
    // **Bis 20000 ms, nicht länger** — gemessen stirbt der Spieler danach, und
    // dann ist der erlittene Schaden bei seinem Leben gedeckelt. Ein Vergleich
    // zweier gedeckelter Werte sieht aus wie „keine Wirkung"; genau das hat
    // meinen ersten Messversuch verdorben.
    for (let t = 0; t <= 20_000; t += 100) {
      combat.stepPassive(t);
      if (!combat.gcd.isReady(t)) continue;
      if (combat.decayStacks >= 3 && combat.isOffCooldown(decay, t)) combat.cast(decay, t);
      else if (combat.isOffCooldown(canker, t)) combat.cast(canker, t);
      else if (combat.isOffCooldown(blight, t)) combat.cast(blight, t);
    }
    expect(combat.playerHp, 'der Spieler ist gestorben — der Wert wäre gedeckelt').toBeGreaterThan(
      0,
    );
    return hpVorher - combat.playerHp;
  }

  it('lässt weniger durch', () => {
    const ohne = erlitten();
    expect(ohne, 'der Boss trifft gar nicht — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(
      erlitten({ 'putrefaction-duration': VOLL }),
      'der Schutz-Weg mindert nichts',
    ).toBeLessThan(ohne);
  });

  it('und teilt selbst keinen Schaden aus', () => {
    // **Der defensive Weg bleibt defensiv.** Hübe er auch den Schaden, wäre er
    // wieder eine schwächere Fassung von `effect`.
    const schaden = (talents?: Record<string, number>) => {
      const combat = kampf(talents);
      combat.cast(alsSpellIndex(combat.indexOf('canker')), 0);
      combat.cast(alsSpellIndex(combat.indexOf('fester')), 1500);
      return combat.damageDealt;
    };
    expect(schaden({ 'putrefaction-duration': VOLL })).toBe(schaden());
  });
});

describe('sublimation-reduction — die Umwandlung', () => {
  /** Was EFFLUX aus der liegenden Restzeit macht. */
  function umgewandelt(talents?: Record<string, number>): number {
    const combat = kampf(talents);
    // Erst lange Abklingzeiten auslösen, damit EFFLUX etwas vorfindet.
    for (const [i, id] of ['pyroclasm', 'slag', 'flashpoint'].entries()) {
      combat.cast(alsSpellIndex(combat.indexOf(id)), i * 1500);
    }
    const vorher = combat.damageDealt;
    combat.cast(alsSpellIndex(combat.indexOf('efflux')), 4500);
    return combat.damageDealt - vorher;
  }

  it('macht liegende Restzeit mehr wert', () => {
    const ohne = umgewandelt();
    expect(ohne, 'EFFLUX wandelt gar nichts um — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(
      umgewandelt({ 'sublimation-reduction': VOLL }),
      'der Umwandlungs-Weg hebt nichts',
    ).toBeGreaterThan(ohne);
  });
});

describe('global-build — der Aufbau-Schaden', () => {
  function schadenVon(zauber: string, talents?: Record<string, number>): number {
    const combat = kampf(talents);
    for (let i = 0; i < 8; i++) combat.cast(alsSpellIndex(combat.indexOf('kindle')), i * 1500);
    const vorher = combat.damageDealt;
    combat.cast(alsSpellIndex(combat.indexOf(zauber)), 12_000);
    return combat.damageDealt - vorher;
  }

  it('KINDLE schlägt härter zu', () => {
    const ohne = schadenVon('kindle');
    expect(ohne).toBeGreaterThan(0);
    expect(schadenVon('kindle', { 'global-build': VOLL })).toBeGreaterThan(ohne);
  });

  it('PYRE nicht — er sammelt nichts an', () => {
    const ohne = schadenVon('pyre');
    expect(ohne).toBeGreaterThan(0);
    expect(schadenVon('pyre', { 'global-build': VOLL }), 'der Weg trifft auch Entlader').toBe(ohne);
  });

  it('und BELLOWS bekommt BEIDE Zuschläge — er sammelt UND schlägt zu', () => {
    // **Der Fall, den die Bauart nahelegt und den niemand verlangt hat.**
    // BELLOWS trägt `resourceGain` UND `baseDamage`. Zwei Summanden im selben
    // Topf (13.1b) — wer sie multiplizierte, machte aus einem Topf zwei.
    const nur = schadenVon('bellows', { 'global-build': VOLL });
    const beide = schadenVon('bellows', { 'global-build': VOLL, 'global-release': VOLL });
    expect(schadenVon('bellows'), 'BELLOWS teilt keinen Schaden aus').toBeGreaterThan(0);
    expect(nur, 'der Aufbau-Zuschlag greift bei BELLOWS nicht').toBeGreaterThan(
      schadenVon('bellows'),
    );
    // BELLOWS ist kein Entlader — der zweite Weg darf nichts ändern.
    expect(beide, 'der Entlader-Zuschlag greift bei einem Aufbau-Zauber').toBe(nur);
  });
});
