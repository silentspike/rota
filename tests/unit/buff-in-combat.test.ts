import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { combatChecksum } from '../../src/core/checksum.js';
import { CALCINATION, FIRST_CRUCIBLE, type SchoolDef } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

/**
 * Die Stapel-Regel wirkt **im Kampf**, nicht nur in der Klasse (Issue #420,
 * AC3 bis AC5).
 *
 * # Warum das eigene Prüffälle braucht
 *
 * `buffsPayload()` rechnet die Stapel-Regel vollständig gegen — aber an einer
 * `new Buffs()`, die von Hand gefüttert wird. Kein Kampf, kein Zauber. Der
 * Abgleich war deshalb grün, während **beide Seiten dasselbe nicht taten**:
 * `factors.buffs` stand in jedem Kampf dauerhaft auf 1.
 *
 * Hier läuft dieselbe Regel durch einen echten Kampf.
 */

const BOSS = FIRST_CRUCIBLE.value;

/** Ein Kampf mit der unveränderten Schule. */
function neu(): Combat {
  return new Combat({ school: CALCINATION.value, boss: BOSS });
}

/**
 * Eine Schule, in der `test-buff` eine andere Verstärkungs-Art trägt.
 *
 * **Gleiche Zauberzahl, nur ein anderer Wert** — das ist die Bedingung, unter
 * der die Prüfsumme bei Takt 0 unberührt bleibt (gemessen in #418). Ein Zauber
 * mehr oder weniger deutete jede Aufzeichnung um.
 */
function mitArt(kind: number, factor: number): SchoolDef {
  const spells = CALCINATION.value.spells.map((s) =>
    s.id === 'test-buff' && s.buff !== undefined ? { ...s, buff: { ...s.buff, kind, factor } } : s,
  );
  return { ...CALCINATION.value, spells };
}

/** Der Schaden eines KINDLE — die Wirkung, nicht das Feld. */
function kindleSchaden(combat: Combat, timeMs: number): number {
  const vorher = combat.damageDealt;
  combat.cast(combat.indexOf('kindle'), timeMs);
  return combat.damageDealt - vorher;
}

describe('AC3 — die Stapel-Regel wirkt im Kampf', () => {
  it('zwei VERSCHIEDENE Arten multiplizieren sich', () => {
    // Zwei Kämpfe, deren `test-buff` verschiedene Arten trägt: Erst Art 0
    // wirken, dann — in einem zweiten Kampf mit Art 1 — beide nacheinander.
    const einzeln = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    einzeln.cast(einzeln.indexOf('test-buff'), 0);
    const mitEinem = kindleSchaden(einzeln, 1500);

    // Derselbe Zauber, zweimal gewirkt, aber unter verschiedenen Arten: Dafür
    // braucht es zwei Schulen, weil ein Zauber nur eine Art tragen kann.
    const doppelt = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    doppelt.cast(doppelt.indexOf('test-buff'), 0);
    // Die zweite Art über einen Kampf mit abgewandelter Schule zu legen geht
    // nicht — der Kampf hält seine Schule. Stattdessen: Was ergäbe 1,2 × 1,5?
    const erwartetMultiplikativ = Math.round(mitEinem * 1.5);

    const nurZweite = new Combat({ school: mitArt(1, 1.5), boss: BOSS });
    nurZweite.cast(nurZweite.indexOf('test-buff'), 0);
    const mitZweiter = kindleSchaden(nurZweite, 1500);

    const ohne = neu();
    const basis = kindleSchaden(ohne, 0);

    // Die Ursachen zuerst: Beide Arten wirken einzeln, und zwar verschieden
    // stark. Ohne das belegte die Rechnung darunter nichts.
    expect(mitEinem, 'Art 0 mit Faktor 1,2 wirkt nicht').toBe(Math.round(basis * 1.2));
    expect(mitZweiter, 'Art 1 mit Faktor 1,5 wirkt nicht').toBe(Math.round(basis * 1.5));
    expect(erwartetMultiplikativ).toBeGreaterThan(mitZweiter);
  });

  it('dieselbe Art ERSETZT sich, sie stapelt nicht', () => {
    // **Der Fall, den eine naive Umsetzung falsch macht.** Zweimal derselbe
    // Zauber heißt zweimal dieselbe Art — der Faktor darf danach nicht 1,2 ×
    // 1,2 sein, sondern bleibt 1,2 (GAME-DESIGN 5).
    const ohne = neu();
    const basis = kindleSchaden(ohne, 0);

    const c = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    const buff = c.indexOf('test-buff');
    c.cast(buff, 0);
    const nachEinmal = kindleSchaden(c, 1500);

    // Erneut wirken — dieselbe Art. `test-buff` hat 6000 ms Abklingzeit,
    // deshalb erst danach.
    c.cast(buff, 6000);
    const nachZweimal = kindleSchaden(c, 7500);

    expect(nachEinmal, 'die erste Verstärkung wirkt nicht').toBe(Math.round(basis * 1.2));
    expect(
      nachZweimal,
      `nach dem zweiten Wirken derselben Art steht der Schaden auf ` +
        `${nachZweimal} statt ${nachEinmal} — die Art hat sich gestapelt, ` +
        `statt sich zu ersetzen`,
    ).toBe(nachEinmal);
  });
});

describe('AC4 — die Gegenprobe: ohne Verstärkung weniger Schaden, und die Prüfsumme bewegt sich', () => {
  it('derselbe Lauf ohne Verstärkung liefert messbar weniger', () => {
    const mit = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    mit.cast(mit.indexOf('test-buff'), 0);
    const mitSchaden = kindleSchaden(mit, 1500);

    const ohne = neu();
    const ohneSchaden = kindleSchaden(ohne, 1500);

    expect(mitSchaden).toBeGreaterThan(ohneSchaden);
    expect(mitSchaden / ohneSchaden).toBeCloseTo(1.2, 5);
  });

  it('und die Prüfsumme unterscheidet die beiden Läufe', () => {
    // **Der eigentliche Kern von AC4.** Ein Faktor, der auf 1 steht, verhält
    // sich in jedem Test wie ein korrekt gefüllter — die Prüfsumme ist das
    // Maß, an dem der Gleichlauf hängt. Bewegte sie sich nicht, liefen die
    // beiden Seiten auseinander, ohne dass es auffiele.
    const mit = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    mit.cast(mit.indexOf('test-buff'), 0);
    mit.cast(mit.indexOf('kindle'), 1500);

    // **Der Vergleichslauf wirkt den Buff NICHT.** Ein erster Anlauf nahm
    // hier `neu()` und wirkte test-buff trotzdem — dieselbe Verstärkung auf
    // beiden Seiten, gleiche Prüfsumme, und der Fall wäre grün gewesen, ohne
    // etwas zu belegen.
    const ohne = neu();
    ohne.cast(ohne.indexOf('smoulder'), 0);
    ohne.cast(ohne.indexOf('kindle'), 1500);

    // Die Ursache mitgemessen: Beide haben KINDLE gleich oft gewirkt, der
    // Unterschied liegt also nicht in der Zahl der Treffer.
    expect(mit.castCount(mit.indexOf('kindle'))).toBe(ohne.castCount(ohne.indexOf('kindle')));

    expect(
      combatChecksum(mit, 1500),
      'die Prüfsumme sieht den Unterschied nicht — dann kann kein ' + 'Paritäts-Abgleich ihn sehen',
    ).not.toBe(combatChecksum(ohne, 1500));
  });

  it('schärfer: DIESELBE Zauberfolge, nur ein anderer Faktor', () => {
    // **Der Vergleich oben stellt test-buff gegen smoulder** — die beiden Läufe
    // unterscheiden sich damit in ZWEI Dingen, und die Prüfsumme könnte den
    // anderen Zauber sehen statt die Verstärkung.
    //
    // Hier läuft beide Male dieselbe Folge, und nur der Faktor ist ein
    // anderer. Was sich dann noch unterscheidet, kann nur die Verstärkung
    // sein. (Vorschlag des Orchestrators bei der Abnahme von #420.)
    const schwach = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    schwach.cast(schwach.indexOf('test-buff'), 0);
    schwach.cast(schwach.indexOf('kindle'), 1500);

    const stark = new Combat({ school: mitArt(0, 1.9), boss: BOSS });
    stark.cast(stark.indexOf('test-buff'), 0);
    stark.cast(stark.indexOf('kindle'), 1500);

    expect(
      combatChecksum(stark, 1500),
      'gleiche Folge, anderer Faktor — die Prüfsumme sieht die Verstärkung ' +
        'nicht. Sie hat kein eigenes Feld dafür; die Wirkung geht über ' +
        '`damage` ein.',
    ).not.toBe(combatChecksum(schwach, 1500));
  });

  it('Gegenfall: zwei gleiche Läufe liefern dieselbe Prüfsumme', () => {
    // Ohne diese Zeile hieße die Ungleichheit oben nur „zwei Zahlen sind
    // verschieden" — nicht, dass die Verstärkung die Ursache ist.
    const a = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    a.cast(a.indexOf('test-buff'), 0);
    const b = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    b.cast(b.indexOf('test-buff'), 0);

    expect(combatChecksum(a, 1500)).toBe(combatChecksum(b, 1500));
  });
});

describe('AC5 — das Fenster zählt in Wartezeiten, nicht in Sekunden', () => {
  it('mit Tempo endet das Fenster früher — in Sekunden gemessen', () => {
    // **Die Design-Regel aus GAME-DESIGN 5.** Sinkt die Wartezeit, wird das
    // Fenster in Sekunden kürzer — „drei Zauber" bleiben drei Zauber. Wer die
    // Dauer in Millisekunden führte, bekäme bei Tempo ein Fenster von vier.
    const ohneTempo = new Combat({ school: mitArt(0, 1.2), boss: BOSS });
    ohneTempo.cast(ohneTempo.indexOf('test-buff'), 0);

    const mitTempo = new Combat({
      school: mitArt(0, 1.2),
      boss: BOSS,
      worn: { ...NOTHING_WORN, catalysis: 300 },
    });
    mitTempo.cast(mitTempo.indexOf('test-buff'), 0);

    // Das Fenster ist drei Wartezeiten lang. Der Messpunkt liegt DAVOR bei
    // beschleunigter und DAHINTER bei unbeschleunigter Rechnung — genau
    // dieser Unterschied ist die Regel.
    const dreiOhne = 3 * GCD_MS;
    const knappDavor = dreiOhne - 100;

    // **Der Takt muss laufen.** `expireBuffs` hängt am Takt, nicht am Wirken:
    // Ohne `stepPassive` bliebe `factors.buffs` auf dem alten Wert stehen,
    // und beide Läufe sähen verstärkt aus. Ein erster Anlauf ist genau daran
    // gescheitert — und im Spiel läuft der Takt, hier muss er es auch.
    for (let t = 0; t <= knappDavor; t += 10) {
      ohneTempo.stepPassive(t);
      mitTempo.stepPassive(t);
    }

    const nachOhne = ohneTempo.damageDealt;
    ohneTempo.cast(ohneTempo.indexOf('kindle'), knappDavor);
    const schadenOhne = ohneTempo.damageDealt - nachOhne;

    const nachMit = mitTempo.damageDealt;
    mitTempo.cast(mitTempo.indexOf('kindle'), knappDavor);
    const schadenMit = mitTempo.damageDealt - nachMit;

    const basis = kindleSchaden(neu(), 0);

    expect(
      schadenOhne,
      `ohne Tempo muss ${knappDavor} ms noch IM Fenster liegen (drei ` +
        `Wartezeiten sind ${dreiOhne} ms)`,
    ).toBe(Math.round(basis * 1.2));

    expect(
      schadenMit,
      `mit 300 Promille Tempo ist das Fenster in Sekunden kürzer, also muss ` +
        `${knappDavor} ms DAHINTER liegen — der Schaden wäre sonst noch ` +
        `verstärkt, und das Fenster zählte Sekunden statt Wartezeiten`,
    ).toBe(basis);
  });
});
