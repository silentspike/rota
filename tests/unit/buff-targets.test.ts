import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Eine Verstärkung kann auf andere Größen wirken als den Schaden
 * (Issue #489), und die vier Speicher sind wirklich getrennt.
 *
 * # Warum die zweite Spalte die wichtigere ist
 *
 * Jeder Fall unten prüft zweierlei: dass die Verstärkung **ihre** Größe
 * bewegt — und dass sie die **anderen** in Ruhe lässt. Die zweite Hälfte ist
 * der eigentliche Nachweis: Sie belegt, dass die vier `Buffs`-Instanzen
 * getrennt sind.
 *
 * Wären sie es nicht, müsste GAME-DESIGN 328 („gleiche Art ersetzt einander")
 * über Zielgrößen hinweg gelten — und eine Verstärkung auf den Punktegewinn
 * verdrängte eine auf den Schaden, obwohl das niemand beschlossen hat.
 *
 * # Vier Prüf-Zauber, nicht einer
 *
 * Vier Zauber, die alle dasselbe Fenster öffnen, belegten vier Mal dasselbe.
 * Deshalb trägt jeder eine andere `target`-Größe, und die Fälle vergleichen
 * über Kreuz.
 */

const BOSS = FIRST_CRUCIBLE.value;

function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/**
 * Wirkt den Prüf-Zauber und lässt den Takt bis kurz VOR das Fensterende
 * laufen.
 *
 * **1600 ms, nicht 100.** Der Prüf-Zauber löst den GCD aus (1500 ms); ein
 * zweiter Druck davor wird abgewiesen, und beide Läufe sähen gleich aus. Ein
 * erster Anlauf maß bei 200 ms und bekam 0 gegen 2 Punkte — nicht weil die
 * Verstärkung fehlte, sondern weil BIND gar nicht wirkte.
 *
 * Das Fenster ist drei Wartezeiten lang (4500 ms), der zweite Druck liegt
 * also noch darin.
 */
function mitVerstaerkung(id: string): Combat {
  const c = neu();
  c.cast(c.indexOf(id), 0);
  for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
  return c;
}

/** Derselbe Zeitlauf ohne Verstärkung — damit die Uhr nicht der Unterschied ist. */
function ohneVerstaerkung(): Combat {
  const c = neu();
  for (let t = 0; t <= 1600; t += 10) c.stepPassive(t);
  return c;
}

describe('control — der Punktegewinn steigt, der Schaden nicht', () => {
  it('BIND gibt mehr Kontroll-Punkte', () => {
    const ohne = ohneVerstaerkung();
    ohne.cast(ohne.indexOf('bind'), 1600);

    const mit = mitVerstaerkung('test-control');
    mit.cast(mit.indexOf('bind'), 1600);

    expect(ohne.control, 'ohne Verstärkung gibt BIND keine Punkte').toBeGreaterThan(0);
    expect(mit.control, 'die Verstärkung wirkt nicht auf den Punktegewinn').toBeGreaterThan(
      ohne.control,
    );
  });

  it('und der Schaden bleibt gleich — die Speicher sind getrennt', () => {
    // **Der eigentliche Nachweis.** Wäre es ein gemeinsamer Speicher, träfe
    // die Verstärkung auch den Schaden.
    const ohne = ohneVerstaerkung();
    const a = ohne.damageDealt;
    ohne.cast(ohne.indexOf('kindle'), 1600);

    const mit = mitVerstaerkung('test-control');
    const b = mit.damageDealt;
    mit.cast(mit.indexOf('kindle'), 1600);

    expect(mit.damageDealt - b).toBe(ohne.damageDealt - a);
  });
});

describe('resource — der Hitze-Aufbau steigt, der Punktegewinn nicht', () => {
  it('KINDLE lädt mehr Hitze auf', () => {
    const ohne = ohneVerstaerkung();
    ohne.cast(ohne.indexOf('kindle'), 1600);

    const mit = mitVerstaerkung('test-heat');
    mit.cast(mit.indexOf('kindle'), 1600);

    expect(ohne.resource, 'ohne Verstärkung lädt KINDLE nichts').toBeGreaterThan(0);
    expect(mit.resource).toBeGreaterThan(ohne.resource);
  });

  it('und die Kontroll-Punkte bleiben gleich', () => {
    const ohne = ohneVerstaerkung();
    ohne.cast(ohne.indexOf('bind'), 1600);

    const mit = mitVerstaerkung('test-heat');
    mit.cast(mit.indexOf('bind'), 1600);

    expect(mit.control).toBe(ohne.control);
  });
});

describe('stack — der Stapel-Wert steigt, die Stapelzahl nicht', () => {
  it('dieselbe Zahl Stapel wiegt schwerer', () => {
    const ohne = ohneVerstaerkung();
    ohne.cast(ohne.indexOf('canker'), 1600);
    for (let t = 1600; t <= 1900; t += 10) ohne.stepPassive(t);

    const mit = mitVerstaerkung('test-stack');
    mit.cast(mit.indexOf('canker'), 1600);
    for (let t = 1600; t <= 1900; t += 10) mit.stepPassive(t);

    // Die Ursache zuerst: gleich viele Stapel.
    expect(mit.decayStacks, 'die Stapelzahl unterscheidet sich').toBe(ohne.decayStacks);
    expect(ohne.decayStacks, 'ohne Stapel misst der Fall nichts').toBeGreaterThan(0);

    expect(
      mit.damageFactors.vulnerability,
      'derselbe Stapel wiegt nicht schwerer — dann wirkt die Verstärkung nicht',
    ).toBeGreaterThan(ohne.damageFactors.vulnerability);
  });
});

describe('Die Trennung über Kreuz — vier Speicher, vier Größen', () => {
  it('keine der vier Verstärkungen berührt den Schadens-Faktor', () => {
    // **Sonst wäre die ganze Konstruktion umsonst.** Alle vier tragen
    // `kind: 0` — in einem gemeinsamen Speicher würden sie einander und den
    // Schadens-Buff verdrängen.
    for (const id of ['test-control', 'test-heat', 'test-tick', 'test-stack']) {
      const c = mitVerstaerkung(id);
      expect(c.damageFactors.buffs, `${id} hat den Schadens-Faktor bewegt`).toBe(1);
    }
  });

  it('der Schadens-Buff dagegen schon — die Gegenprobe', () => {
    // Ohne diese Zeile zeigte der Fall oben nur, dass `factors.buffs`
    // überhaupt nie etwas anderes als 1 wird.
    const c = mitVerstaerkung('test-buff');
    expect(c.damageFactors.buffs).toBeGreaterThan(1);
  });

  it('und jede der vier bewegt die Fenstersumme', () => {
    // Damit steht auch fest, dass sie in der Prüfsumme ankommen (#493).
    for (const id of ['test-control', 'test-heat', 'test-tick', 'test-stack']) {
      const c = mitVerstaerkung(id);
      expect(c.buffWindowSum, `${id} öffnet kein Fenster`).toBeGreaterThan(0);
    }
  });
});

/**
 * **Die Grenze dieses Abschnitts: er prüft die RICHTUNG, nicht die Zahl**
 * (Issue #640).
 *
 * `toBeGreaterThan(a.length)` sagt „dichter als ohne". Eine Richtung hat zwei
 * Werte, eine Rundung hat viele — welcher Abstand herauskommt, steht hier
 * nirgends. `dotInterval` verrechnet aber drei Dinge: den Buff-Faktor, eine
 * Rundung und `hasted()`.
 *
 * Bis #640 hatte jede Seite genau so einen Richtungs-Wächter (hier und
 * `sim/tests/buff_wirkung.rs`, `tick_wirkt`) und **kein Paritäts-Fall verglich
 * die Zahlen** — `tick` war die einzige Zielgröße ohne Träger in einer
 * Aufzeichnung. Die Zahl steht jetzt in `tests/parity/recordings/tick-rundung.json`
 * und wird Takt für Takt gegen den Simulator geprüft.
 *
 * Dieser Abschnitt bleibt trotzdem: Er ist die Diagnose. Wird er rot und der
 * Paritäts-Fall nicht, hat sich die Wirkung geändert und nicht der Gleichlauf.
 */
describe('tick — die Ticks werden dichter, der Tick-Schaden bleibt', () => {
  /**
   * Zeichnet die Sprünge im ausgeteilten Schaden auf: wann ein Tick fiel und
   * wie hoch er war.
   *
   * **Beides zusammen ist der Nachweis.** Nur die Zahl der Sprünge zu zählen
   * ließe offen, ob die Verstärkung die Ticks dichter macht oder jeden
   * einzelnen stärker — die Gesamtsumme stiege in beiden Fällen.
   */
  function sprunge(c: Combat, vonMs: number, bisMs: number): number[] {
    const hoehen: number[] = [];
    let letzter = c.damageDealt;
    for (let t = vonMs; t <= bisMs; t += 10) {
      c.stepPassive(t);
      if (c.damageDealt !== letzter) {
        hoehen.push(c.damageDealt - letzter);
        letzter = c.damageDealt;
      }
    }
    return hoehen;
  }

  it('SMOULDER tickt öfter, und jeder Tick trägt dasselbe', () => {
    const ohne = ohneVerstaerkung();
    ohne.cast(ohne.indexOf('smoulder'), 1600);
    const a = sprunge(ohne, 1610, 4400);

    const mit = mitVerstaerkung('test-tick');
    mit.cast(mit.indexOf('smoulder'), 1600);
    const b = sprunge(mit, 1610, 4400);

    expect(a.length, 'ohne Verstärkung tickt nichts — dann misst der Fall nichts').toBeGreaterThan(
      0,
    );
    expect(b.length, 'die Ticks werden nicht dichter').toBeGreaterThan(a.length);

    // **Die zweite Spalte.** Der Faktor teilt hier den Abstand, statt zu
    // multiplizieren: Ein Faktor 2 heißt doppelt so oft, nicht doppelt so
    // stark. Wäre der Speicher mit dem Schadens-Speicher geteilt, stiege auch
    // die Höhe.
    expect(new Set(b).size, 'die Ticks sind unterschiedlich hoch').toBe(1);
    expect(b[0], 'ein Tick trägt mehr als ohne Verstärkung').toBe(a[0]);
  });
});
