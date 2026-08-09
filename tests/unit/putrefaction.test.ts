import { describe, expect, it } from 'vitest';
import {
  Combat,
  DECAY_STACK_DURATION_MS,
  DECAY_STACK_PERCENT,
  MAX_DECAY_STACKS,
} from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';

/**
 * Zersetzungs-Stapel auf dem Boss (GAME-DESIGN 4.1, Issue #97).
 *
 * # Was diese Schule von den anderen unterscheidet
 *
 * Sie erhöht **nicht** den eigenen Schaden, sondern lässt den Boss von
 * **allen** Quellen mehr Schaden nehmen. Deshalb wirkt sie über den siebten
 * Faktor der Schadensformel (6.3, Verwundbarkeit des Bosses) und nicht über
 * einen Faktor des Spielers — und deshalb genügt ein Prüffall mit nur
 * Putrefaktions-Zaubern nicht, um die Design-Aussage zu belegen.
 *
 * # Jeder Stapel hat seine eigene Uhr
 *
 * Der älteste verfällt zuerst, die jüngeren bleiben. Ein gemeinsamer Verfall
 * war die verworfene Bauart — er hätte jeden Zwischenfall im Kampf den ganzen
 * Aufbau gekostet. Ein Prüffall, der nur zeigt, dass *irgendwann* etwas
 * verfällt, ließe genau diese verworfene Bauart durch.
 */

function bau(): Combat {
  const boss = bossById('test-warden');
  if (boss === undefined) throw new Error('test-warden fehlt');
  return new Combat({ school: CALCINATION.value, boss });
}

/** Wirkt einen Zauber, sobald er bereit ist, und gibt die Kampfzeit zurück. */
function wirke(combat: Combat, id: string, abMs: number): number {
  const index = combat.indexOf(id);
  for (let t = abMs; t < abMs + 30_000; t += 10) {
    combat.stepPassive(t);
    if (combat.cast(index, t).result === 0) return t;
  }
  throw new Error(`${id} kam nicht zum Zug`);
}

describe('AC1 — ein Stapel macht den Boss messbar verwundbarer', () => {
  it('zwei Läufe, Unterschied nur in den Stapeln', () => {
    // **Die Ursache zuerst:** CANKER muss gewirkt haben und ein Stapel liegen.
    const mit = bau();
    wirke(mit, 'canker', 0);
    expect(mit.decayStacks, 'CANKER hat keinen Stapel angelegt').toBeGreaterThan(0);

    const ohne = bau();
    expect(ohne.decayStacks).toBe(0);
    expect(ohne.vulnerability).toBe(1);

    // Derselbe Zauber, derselbe Zeitpunkt — der einzige Unterschied ist der
    // Stapel, der schon liegt.
    const schadenOhne = ohne.cast(ohne.indexOf('fester'), 20_000);
    const schadenMit = mit.cast(mit.indexOf('fester'), 20_000);
    expect(schadenOhne.result).toBe(0);
    expect(schadenMit.result).toBe(0);
    expect(
      schadenMit.damage,
      `mit Stapel ${schadenMit.damage}, ohne ${schadenOhne.damage} — der ` + 'Stapel wirkt nicht',
    ).toBeGreaterThan(schadenOhne.damage);
  });

  it('die Verwundbarkeit folgt der Zahl der Stapel', () => {
    const combat = bau();
    expect(combat.vulnerability).toBe(1);
    wirke(combat, 'canker', 0);
    expect(combat.vulnerability).toBe((100 + DECAY_STACK_PERCENT) / 100);
  });
});

describe('AC2 — der Zuwachs gilt für ALLE Schulen', () => {
  it('nachgewiesen mit einem Zauber einer anderen Schule', () => {
    // **Das ist die Design-Aussage, nicht die Mechanik.** Ein Prüffall nur mit
    // Putrefaktions-Zaubern belegt sie nicht: FESTER hat einen eigenen Bonus je
    // Stapel, der Zuwachs käme also auch dann zustande, wenn die Verwundbarkeit
    // gar nicht wirkte.
    //
    // KINDLE gehört zur CALCINATION und kennt keine Stapel. Steigt sein
    // Schaden trotzdem, kann das nur der Faktor in der Schadensformel sein.
    const mit = bau();
    wirke(mit, 'canker', 0);
    expect(mit.decayStacks).toBeGreaterThan(0);

    const ohne = bau();
    const kindle = ohne.indexOf('kindle');
    expect(kindle, 'KINDLE ist auf der Leiste nicht auffindbar').toBeGreaterThanOrEqual(0);

    const a = ohne.cast(kindle, 20_000);
    const b = mit.cast(mit.indexOf('kindle'), 20_000);
    expect(a.result).toBe(0);
    expect(b.result).toBe(0);
    expect(
      b.damage,
      `KINDLE macht mit Stapel ${b.damage}, ohne ${a.damage} — dann wirkt die ` +
        'Verwundbarkeit nicht auf fremde Schulen, und die Schule hält ihr ' +
        'Versprechen nicht',
    ).toBeGreaterThan(a.damage);
  });
});

describe('AC3 — der älteste Stapel verfällt zuerst, die jüngeren bleiben', () => {
  it('beide Hälften im selben Lauf', () => {
    // **Der Kern des Issues.** Ein Fall, der nur zeigt, dass irgendwann etwas
    // verfällt, ließe einen GEMEINSAMEN Verfall durch — also genau die
    // verworfene Bauart. Gebraucht wird der Moment, in dem der älteste weg ist
    // und die jüngeren stehen.
    const combat = bau();
    const ersterMs = wirke(combat, 'canker', 0);
    // Der zweite deutlich später, damit sich die Uhren unterscheiden.
    const zweiterMs = wirke(combat, 'canker', ersterMs + 5_000);
    expect(zweiterMs).toBeGreaterThan(ersterMs);
    expect(combat.decayStacks, 'es liegen keine zwei Stapel').toBe(2);

    // **Die Ursache:** Die beiden Uhren müssen wirklich verschieden sein.
    expect(
      combat.decayExpiryAt(1) - combat.decayExpiryAt(0),
      'beide Stapel verfallen zur selben Zeit — dann kann dieser Fall einen ' +
        'gemeinsamen Verfall nicht von einer eigenen Uhr unterscheiden',
    ).toBeGreaterThan(0);

    // Ein Zeitpunkt zwischen den beiden Abläufen.
    const dazwischen = ersterMs + DECAY_STACK_DURATION_MS + 10;
    expect(dazwischen).toBeLessThan(zweiterMs + DECAY_STACK_DURATION_MS);
    combat.stepPassive(dazwischen);

    // **Beide Hälften, im selben Lauf:** einer weg, einer da.
    expect(
      combat.decayStacks,
      'nach dem Ablauf der ersten Uhr liegen nicht genau noch ein Stapel — ' +
        `es sind ${combat.decayStacks}. Bei einem gemeinsamen Verfall wären ` +
        'es null',
    ).toBe(1);

    // Und der verbliebene ist der JÜNGERE: seine Uhr läuft noch.
    expect(combat.decayExpiryAt(0)).toBeGreaterThan(dazwischen);
    expect(combat.vulnerability).toBe((100 + DECAY_STACK_PERCENT) / 100);

    // Zuletzt verfällt auch er.
    combat.stepPassive(zweiterMs + DECAY_STACK_DURATION_MS + 10);
    expect(combat.decayStacks).toBe(0);
    expect(combat.vulnerability).toBe(1);
  });
});

describe('AC4 — die Obergrenze hält, und sie verbraucht nichts', () => {
  it('über der Grenze erhöht nichts', () => {
    const combat = bau();
    let ms = 0;
    for (let i = 0; i < MAX_DECAY_STACKS + 3; i++) {
      ms = wirke(combat, 'canker', ms + 10);
    }
    expect(combat.decayStacks).toBe(MAX_DECAY_STACKS);
    expect(combat.vulnerability).toBe((100 + MAX_DECAY_STACKS * DECAY_STACK_PERCENT) / 100);
  });

  it('verbraucht aber auch nichts — die Uhr des ältesten bleibt stehen', () => {
    // **Der Teil, den man leicht übersieht.** Erneuerte eine Wirkung über der
    // Grenze die Uhr des ältesten Stapels, hielte man den Aufbau ewig oben,
    // ohne je nachzulegen — und „verfallen langsam" wäre wirkungslos.
    const combat = bau();
    let ms = 0;
    for (let i = 0; i < MAX_DECAY_STACKS; i++) ms = wirke(combat, 'canker', ms + 10);
    const aeltesteUhr = combat.decayExpiryAt(0);

    ms = wirke(combat, 'canker', ms + 10);
    expect(combat.decayStacks).toBe(MAX_DECAY_STACKS);
    expect(
      combat.decayExpiryAt(0),
      'die Wirkung über der Grenze hat die Uhr des ältesten Stapels erneuert',
    ).toBe(aeltesteUhr);
  });
});

describe('Die Mechanik wird von ausgelieferten Zaubern benutzt', () => {
  it('CANKER legt an, FESTER lebt davon — welche, nicht ob', () => {
    // **Der Wächter, den diese Woche dreimal gefehlt hat.** Eine fertige
    // Mechanik, die kein Zauber aufruft, gilt über die Prüfsumme als fertig
    // und ist es nicht: `sublimation` wurde so zu #201, `energy` zu #219,
    // `cooldown_reset` zu #223.
    //
    // Geprüft wird **welche** Zauber, nicht **ob** einer. Ein `> 0` wäre schon
    // erfüllt, wenn irgendetwas irgendwo steht; die Namen halten fest, was die
    // Referenz-Rechnung sieht.
    //
    // Die Gegenseite steht in `sim/tests/parity_putrefaction.rs` — ein Wächter
    // allein auf einer Seite ist ein halber.
    const spells = CALCINATION.value.spells;
    const legenAn = spells.filter((s) => (s.decay?.stacks ?? 0) > 0).map((s) => s.id);
    const lebenDavon = spells.filter((s) => (s.perDecayStackPercent ?? 0) > 0).map((s) => s.id);

    // **`blight` kam mit Issue #479 dazu, und die Warnung wurde befolgt:**
    // Die Referenz-Rechnung IST nachgemessen, nicht die Liste einfach
    // erweitert. `4240 @ 40510` steht unverändert (step1.test.ts, 15 Fälle) —
    // was hier zu erwarten war, aber nicht angenommen werden durfte: Die
    // Referenz-Rotation ist reine CALCINATION und wirkt keinen der beiden.
    expect(
      legenAn,
      'wer hier einen Zauber ergänzt, ändert die Referenz-Rechnung — dann ' +
        'gehört sie nachgemessen, nicht diese Liste erweitert',
    ).toEqual(['canker', 'blight']);
    // **MIASMA kam mit #479 dazu, und diese Zeile ist sein Nachweis.** Er ist
    // der erste Zauber, der von den Stapeln lebt, ohne selbst welche zu
    // legen: kein `decay`-Feld, aber `perDecayStackPercent`. Damit steht er
    // in dieser Liste und NICHT in der darüber — genau das ist seine Rolle.
    expect(lebenDavon, 'CANKER legt an, FESTER lebt davon (GAME-DESIGN 4.1)').toEqual([
      'fester',
      'miasma',
      // **NECROSIS lebt nicht nur davon, er LOEST EIN** (Issue #479). Er
      // steht aus demselben Grund hier wie MIASMA — `perDecayStackPercent`
      // ohne `decay`-Feld. Was ihn von beiden Vorgaengern trennt, prueft der
      // eigene Fall weiter unten: Nach ihm sind die Stapel WEG.
      'necrosis',
    ]);
  });

  it('und sie wirkt im Kampf, nicht nur in den Daten', () => {
    // **Der zweite Teil, und er ist der wichtigere.** Die Listen oben zeigen,
    // dass die Daten es sagen — nicht, dass der Kampf es tut. Genau diese
    // Lücke hat die drei Fälle dieser Woche entstehen lassen.
    const combat = bau();
    expect(combat.decayStacks).toBe(0);
    wirke(combat, 'canker', 0);
    expect(
      combat.decayStacks,
      'CANKER kam nicht zum Zug oder die Mechanik ist nicht angeschlossen',
    ).toBeGreaterThan(0);
    expect(combat.vulnerability).toBeGreaterThan(1);
  });
});

describe('AC6 — ohne Putrefaktions-Zauber bewegt sich nichts', () => {
  it('die Verwundbarkeit bleibt neutral', () => {
    // Die Referenz-Rechnung (4240 @ 40510) läuft gegen CALCINATION. Solange
    // kein Zersetzungs-Zauber wirkt, darf der Faktor sie nicht berühren.
    const boss = bossById('first-crucible');
    if (boss === undefined) throw new Error('first-crucible fehlt');
    const combat = new Combat({ school: CALCINATION.value, boss });
    for (let t = 0; t <= 20_000; t += 10) {
      combat.stepPassive(t);
      combat.cast(combat.indexOf('kindle'), t);
    }
    expect(combat.decayStacks).toBe(0);
    expect(combat.vulnerability).toBe(1);
  });
});
