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

  /**
   * **Die GRÖSSE, nicht die Existenz** (Issue #547).
   *
   * Der Fall darüber war fünfzehn Tage lang die einzige Absicherung, und er
   * ist gegen eine Balance-Änderung blind: Mit `factor: 1.05` statt `1.5`
   * wird aus fünfzig Prozent Anhebung eine von fünf, der Zauber ist praktisch
   * wirkungslos — und `toBeGreaterThan(CAP)` bleibt grün, weil 21 immer noch
   * größer als 20 ist.
   *
   * # Warum die 1.5 hier HART steht und nicht aus den Daten kommt
   *
   * Das ist der Punkt, an dem dieser Fall entweder wirkt oder Zierde ist.
   * Läse er den Faktor aus `slag.buff.factor`, läse er bei der Gegenprobe die
   * **mutierte** Zahl mit: erwartet würde `round(20 × 1.05) = 21`, gemessen
   * käme 21, und der Fall wäre grün — er prüfte die Formel gegen sich selbst
   * statt gegen das Design.
   *
   * Ein Sollwert, der aus derselben Quelle stammt wie der Istwert, ist kein
   * Sollwert. Die 1.5 ist die Design-Entscheidung aus #490; ändert jemand sie
   * absichtlich, gehört diese Zeile mitgeändert — und genau dann fällt auf,
   * dass eine Entscheidung getroffen wurde.
   */
  it('und zwar um die HÄLFTE, nicht nur irgendwie', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);

    // **Nicht über `hoechsterVorrat`.** Ein erster Entwurf hat es versucht
    // und gemessen: BELLOWS kommt im Fenster auf 22, der angehobene Deckel
    // liegt bei 30. Der Fall hätte also zwei Zahlen verglichen, von denen
    // die eine den Deckel gar nicht berührt — er hätte die Anhebung für zu
    // klein gehalten, obwohl sie stimmt.
    //
    // `setResource` klemmt gegen `resourceCap`, also gegen die eine
    // Rechenstelle selbst (combat.ts, `Math.round(cap * factor)`). Damit
    // misst der Fall den DECKEL und nicht, was BELLOWS zufällig schafft.
    c.setResource(9999);
    expect(
      c.resource,
      'SLAG hebt die Grenze zwar, aber nicht um die 50 % aus dem Design (#490)',
    ).toBe(Math.round(CAP * 1.5));
  });

  /**
   * **Der zweite Fall, und warum er keine Wiederholung ist.**
   *
   * Ein `buff` trägt ZWEI Größen: einen Betrag (`factor`) und eine Dauer
   * (`gcds`). Der Fall darüber misst die Grenze — er sieht den Betrag und
   * ist blind gegen die Dauer. Dieser hier misst, was im Fenster tatsächlich
   * ankommt, und sieht beides.
   *
   * Gemessen, und die erste Vermutung war falsch. Erwartet hatte ich, der
   * Grenz-Fall sei blind gegen `gcds` — er ist es nicht, weil er bei t=1600
   * misst und das Fenster bei `gcds: 1` dann längst abgelaufen ist:
   *
   * ```text
   *                factor 1.05   gcds 1   bellows.heat 8→4
   *   Grenze           rot         rot         GRÜN      <- die Trennung
   *   Erreichbar       rot         rot          rot
   * ```
   *
   * Die Zelle „Grenze / heat → grün" ist der Grund für diesen Fall, und er
   * ist ein anderer, als ich zuerst dachte: Der Grenz-Fall sieht nur SLAG,
   * dieser sieht die ganze Kette bis zum Vorrat.
   *
   * # Und warum beide bleiben, obwohl dieser mehr fängt
   *
   * Nicht wegen der Fang-Kraft — der Erreichbar-Fall fängt nach diesen drei
   * Messungen alles, was der Grenz-Fall fängt. Sondern wegen der DIAGNOSE:
   * Ändert jemand den Verfall oder den GCD, wird dieser Fall rot, obwohl an
   * SLAG nichts falsch ist. Dass der Grenz-Fall dann grün bleibt, ist die
   * Auskunft „der Zauber stimmt, die Umgebung hat sich bewegt".
   *
   * Ein einzelner Fall müsste diese beiden Fälle mit einer Zahl beantworten
   * und könnte es nicht.
   *
   * # Woher die 22 kommt
   *
   * Das Fenster ist 4 GCDs, der GCD 1500 ms — es passen drei BELLOWS-Würfe
   * hinein (je 8), abzüglich Verfall (1 je 2000 ms) bleiben 22. Die oberen
   * acht Punkte der angehobenen Grenze sind heute unerreichbar; das ist ein
   * Balance-Befund (Worker-2 in #547), kein Fehler.
   *
   * Die 22 steht hart, aus demselben Grund wie die 1.5 oben: Ein Sollwert,
   * der aus derselben Quelle stammt wie der Istwert, ist keiner.
   */
  it('und im Fenster kommen davon 22 an', () => {
    const c = neu();
    c.cast(c.indexOf('slag'), 0);
    for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
    expect(
      hoechsterVorrat(c, 1600),
      'im SLAG-Fenster kommt nicht mehr an — entweder ist der Faktor kleiner oder das Fenster kürzer',
    ).toBe(22);
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
