/**
 * Ein Boss-Wechsel lässt nichts vom alten Kampf stehen (Issue #613).
 *
 * # Die Stelle, an der es schiefgeht
 *
 * `reset()` gab es schon, und es leitet die fünf Zeitgeber (`energy`, `doom`,
 * `stacks`, `runaway`) aus `this.boss` neu ab. Wer den Boss davor umsetzt,
 * bekommt sie also richtig — das verleitet zu dem Schluss, ein Setter genüge.
 *
 * **Er genügt nicht.** `resistanceOfSpell` wird ausschließlich im Konstruktor
 * gefüllt. Ohne die Schleife in `switchBoss` rechnete jeder Zauber im neuen
 * Kampf mit dem Widerstand des alten Bosses — eine Zahl, die zu einem Gegner
 * gehört, den der Spieler verlassen hat, und die kein Bildschirm anzeigt.
 *
 * Der erste Fall unten misst genau diese Schleife: Er wählt zwei Bosse mit
 * **verschiedenem** Widerstand gegen dieselbe Schule und vergleicht den
 * Schaden desselben Zaubers. Nimmt man die Schleife heraus, sind beide Zahlen
 * gleich und der Fall fällt.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { BOSSES, CALCINATION, type BossDef } from '../../src/core/data.js';

function boss(id: string): BossDef {
  const eintrag = BOSSES.get(id);
  expect(eintrag, `${id} steht nicht in BOSSES`).toBeDefined();
  return eintrag!.value;
}

/** Der Schaden des ersten Zaubers der Schule, einmal gewirkt. */
function ersterTreffer(c: Combat): number {
  const i = c.indexOf('kindle');
  c.cast(i, 0);
  return c.damageDealt;
}

describe('switchBoss übernimmt die Widerstände des NEUEN Bosses', () => {
  // Zwei Bosse, die sich im Widerstand gegen CALCINATION unterscheiden.
  // Ohne diesen Unterschied misst der Fall darunter nichts — deshalb steht er
  // als eigene Zusicherung davor und nicht als Kommentar.
  const weich = boss('boss-2-1');
  const zaeh = boss('test-resistant');

  it('die beiden Bosse unterscheiden sich überhaupt im Widerstand', () => {
    // **Der Schlüssel ist GROSSgeschrieben**, nicht die Schul-Kennung: In den
    // Bossdateien steht `"resistances": {"CALCINATION": 0.75}`, und `combat.ts`
    // liest ihn über `spells[i].school`. Mit `CALCINATION.value.id` kam hier
    // `undefined` gegen `undefined` heraus — zwei Nichtwerte, die verglichen
    // wurden, als wären sie Messungen.
    //
    // Ein falscher Schlüssel macht diesen Fall ROT (1 gegen 1), nicht still
    // grün. Das ist die Richtung, in die ein Irrtum fallen soll.
    const faktor = (b: BossDef): number => b.resistances['CALCINATION'] ?? 1;
    expect(
      faktor(weich),
      'beide Bosse widerstehen gleich — der Fall darunter misst dann nichts',
    ).not.toBe(faktor(zaeh));
  });

  it('derselbe Zauber trifft nach dem Wechsel anders', () => {
    const direkt = new Combat({ school: CALCINATION.value, boss: zaeh });
    const gewechselt = new Combat({ school: CALCINATION.value, boss: weich });
    gewechselt.switchBoss(zaeh);

    // **Der Kern.** Ein Kampf, der auf `zaeh` gewechselt hat, muss rechnen wie
    // einer, der mit `zaeh` angefangen hat. Ohne die Schleife in `switchBoss`
    // stünde hier der Wert von `weich`.
    expect(ersterTreffer(gewechselt)).toBe(ersterTreffer(direkt));
  });

  it('und die Gegenprobe: ohne Wechsel rechnet er anders', () => {
    // Ohne diesen Fall bliebe offen, ob der Vergleich oben überhaupt etwas
    // unterscheidet oder beide Bosse zufällig gleich rechnen.
    const ohne = new Combat({ school: CALCINATION.value, boss: weich });
    const mit = new Combat({ school: CALCINATION.value, boss: zaeh });
    expect(ersterTreffer(ohne)).not.toBe(ersterTreffer(mit));
  });
});

describe('switchBoss lässt keinen Rest des alten Kampfes stehen', () => {
  it('Fortschritt, Leben und Zauber-Zähler stehen wieder am Anfang', () => {
    const c = new Combat({ school: CALCINATION.value, boss: boss('boss-2-1') });
    c.cast(c.indexOf('kindle'), 0);
    c.stepPassive(3000);

    // Die Zusicherung, die den Fall vor sich selbst schützt: Hätte der Lauf
    // oben nichts bewegt, wäre alles darunter trivial erfüllt.
    expect(c.damageDealt, 'der Vorlauf hat gar keinen Schaden gemacht').toBeGreaterThan(0);

    c.switchBoss(boss('boss-2-4'));

    expect(c.damageDealt).toBe(0);
    expect(c.castCount(c.indexOf('kindle'))).toBe(0);
    expect(c.boss.id).toBe('boss-2-4');
  });

  it('die Zeitgeber stammen vom neuen Boss, nicht vom alten', () => {
    // `boss-2-1` und `boss-2-4` haben verschiedene Zeitleisten-Takte. Nach dem
    // Wechsel muss der Kampf den des neuen führen — `reset()` leitet sie aus
    // `this.boss` ab, also erst nach dem Umsetzen.
    const c = new Combat({ school: CALCINATION.value, boss: boss('boss-2-1') });
    c.switchBoss(boss('boss-2-4'));
    expect(c.boss.timelineLoopMs).toBe(boss('boss-2-4').timelineLoopMs);
  });
});
