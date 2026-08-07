import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, type SpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * SLAG hebt die Hitze-Grenze befristet an (Issue #490).
 *
 * # Warum das eine Entscheidung erzeugt
 *
 * Die Obergrenze war fest: Wer sie erreicht, verschwendet jeden weiteren
 * Aufbau. SLAG macht daraus eine Wahl — vor einem Entlader die Grenze heben
 * und mehr sammeln, statt am Deckel zu verlieren.
 *
 * # Die eine Rechenstelle
 *
 * `resource.cap` wurde an **zwei** Orten gelesen: beim Auffüllen und beim
 * Setzen (Entwickler-Werkzeug). Ein Faktor an nur einem hätte eine Grenze
 * ergeben, die beim Auffüllen anders ist als beim Setzen — und der
 * Unterschied fiele erst auf, wenn jemand das Werkzeug benutzt.
 */

const BOSS = FIRST_CRUCIBLE.value;
const CAP = CALCINATION.value.resource.cap;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/**
 * Füllt den Vorrat bis an die Grenze und liefert den HÖCHSTSTAND.
 *
 * **BELLOWS, nicht KINDLE.** KINDLE gibt 3 Hitze je Wurf, der Verfall nimmt
 * 1 je 2000 ms — netto etwa 2 pro Wartezeit. Bis 20 wären das über zehn
 * Würfe, und ein erster Entwurf mit acht kam nur auf 19.
 *
 * Der Fall hätte damit zwei Zahlen UNTER der Grenze verglichen und wäre
 * grün gewesen mit der Aussage "SLAG wirkt nicht" — ein erfundener Fehler.
 * BELLOWS gibt 8 und erreicht die Grenze in drei Würfen.
 *
 * Der Höchststand, nicht der Endstand: Der Vorrat verfällt zwischen den
 * Würfen, und die Frage ist, wie hoch er KOMMT.
 */
function hoechsterVorrat(c: Combat, abMs: number): number {
  const bellows = c.indexOf('bellows');
  let hoch = c.resource;
  let t = abMs;
  for (let n = 0; n < 6; n++) {
    c.cast(bellows, t);
    hoch = Math.max(hoch, c.resource);
    t += 1500;
    for (let s = t - 1500; s <= t; s += 10) {
      c.stepPassive(s);
      hoch = Math.max(hoch, c.resource);
    }
  }
  return hoch;
}

describe('AC1 — die Grenze steigt, solange das Fenster offen ist', () => {
  it('ohne SLAG bleibt der Vorrat bei der Grenze stehen', () => {
    const c = neu();
    expect(hoechsterVorrat(c, 0), 'ohne SLAG über die Grenze — dann misst der Fall nichts').toBe(
      CAP,
    );
  });

  it('mit SLAG steigt er darüber', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(hoechsterVorrat(c, 1600), 'SLAG hebt die Grenze nicht').toBeGreaterThan(CAP);
  });
});

describe('AC3 — die Gegenprobe: ein anderer Zauber hebt sie nicht', () => {
  it('KINDLE allein lässt die Grenze, wo sie ist', () => {
    // **Ohne diese Zeile belegte AC1 nur, dass irgendein Druck etwas tut.**
    const c = neu();
    c.cast(c.indexOf('kindle'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(
      hoechsterVorrat(c, 1600),
      'die Grenze steigt ohne SLAG — dann kommt es nicht von ihm',
    ).toBe(CAP);
  });
});

describe('Die anderen Zielgrößen bleiben unberührt', () => {
  it('SLAG bewegt den Schadens-Faktor nicht', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(c.damageFactors.buffs, 'SLAG hat den Schaden bewegt').toBe(1);
  });
});

/**
 * Ein Erzeuger nimmt nie weg (Issue #549).
 *
 * # Der Zustand ist mit vier Tastendrücken erreichbar
 *
 * SLAG hebt die Grenze von 20 auf 30. Wer im Fenster über 20 sammelt und es
 * ablaufen lässt, steht über dem dann wieder geltenden Deckel. Ein Wurf
 * senkte den Vorrat dort — acht erzeugt, vier verloren, netto minus vier.
 *
 * Im Simulator stürzte dieselbe Stelle ab (`fight.rs`, u16-Subtraktion),
 * während das Spiel still falsch zählte. Beide Fälle sind dieselbe Zeile.
 */
describe('AC2/AC3 (#549) — der Vorrat sinkt durch eine Erzeugung nicht', () => {
  /** Bringt den Vorrat über den Deckel und lässt das Fenster ablaufen. */
  function ueberDemDeckel(): { c: Combat; bellows: SpellIndex; vorher: number } {
    const c = neu();
    const bellows = alsSpellIndex(c.indexOf('bellows'));
    c.cast(alsSpellIndex(c.indexOf('slag')), 0);
    c.cast(bellows, 1600);
    c.cast(bellows, 3100);
    c.cast(bellows, 4600);
    // Genau auf das Fensterende, bevor der Verfall viel getan hat: SLAG hält
    // vier GCDs, also 6000 ms.
    c.stepPassive(6000);
    return { c, bellows, vorher: c.resource };
  }

  it('der Zustand ist überhaupt erreichbar — sonst prüfte der Fall nichts', () => {
    const { c, vorher } = ueberDemDeckel();
    expect(vorher, 'der Vorrat steht nicht über dem Grund-Deckel').toBeGreaterThan(CAP);
    // Und das Fenster ist wirklich zu: Der Deckel gilt wieder in seiner
    // Grundgröße. Ohne diese Zeile könnte der Fall bei offenem Fenster laufen
    // und nichts belegen.
    c.cast(alsSpellIndex(c.indexOf('kindle')), 6000);
    expect(c.resource, 'der Deckel steht noch erhöht').toBeGreaterThanOrEqual(vorher);
  });

  it('ein Wurf über dem Deckel senkt den Vorrat NICHT', () => {
    const { c, bellows, vorher } = ueberDemDeckel();
    c.cast(bellows, 6100);
    expect(
      c.resource,
      'der Erzeuger hat WEGGENOMMEN — mit min(before + amount, cap) fällt der ' +
        'Vorrat auf den Deckel, und der Spieler hat nach einem Aufbau-Zauber ' +
        'weniger als vorher',
    ).toBeGreaterThanOrEqual(vorher);
  });

  it('und der Überlauf-Zähler nennt nur, was von der Erzeugung nicht ankam', () => {
    const { c, bellows, vorher } = ueberDemDeckel();
    const vorZaehler = c.resourceOverflow;
    c.cast(bellows, 6100);
    const erzeugt = c.resource - vorher;
    const menge = c.resourceOverflow - vorZaehler + erzeugt;
    expect(
      c.resourceOverflow - vorZaehler,
      `der Zähler nahm mehr als die ${menge} erzeugten Punkte — dann zählt er ` +
        'die vom fallenden Deckel gekappten mit, und die waren schon angekommen',
    ).toBe(menge);
  });
});
