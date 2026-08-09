/**
 * Das Auslaufen einer Verstärkung steht im Verlauf (Issue #682).
 *
 * # Warum dieser Fall neben dem Wächter aus #676 steht
 *
 * Jener prüft, ob eine Ereignisart überhaupt geschrieben wird. Er prüft
 * **nicht**, ob sie an der richtigen Stelle steht oder die richtige Zahl
 * trägt — er wird grün, sobald irgendetwas einen Eintrag der Art 7 erzeugt.
 *
 * Dieselbe Lage wie bei #568: Dort konnte die Parität den Verlaufseintrag
 * nicht fangen, weil der Verlauf in keiner Aufzeichnung dieses Bosses steht.
 * Hier ist es der Wächter statt der Parität, aber die Folgerung ist gleich —
 * die Stelle und die Zahl brauchen einen eigenen Fall.
 *
 * # Was vorher fehlte
 *
 * Art 7 war im Spiel vergeben, in `log.ts` dokumentiert, von `log-kinds`
 * gedeckt und von `freshness` gezählt — und `EVENT_BUFF_EXPIRE` hatte
 * ausserhalb seiner Deklaration keinen einzigen Treffer. Auf der Rust-Seite
 * fehlte die Art sogar ganz: Die Aufzählung sprang von 6 auf 8.
 */
import { describe, expect, it } from 'vitest';

import { Buffs, BUFF_KIND_CAPACITY, BUFF_KIND_MASK_LIMIT } from '../../src/combat/schools/buffs.js';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { CombatLog } from '../../src/core/log.js';
import { TICK_MS } from '../../src/core/loop.js';

describe('die Maske sagt, WELCHE Verstärkung abgelaufen ist', () => {
  it('setzt je abgelaufener Art genau ein Bit', () => {
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 1, 0, GCD_MS);
    buffs.applyForGcds(2, 1.5, 1, 0, GCD_MS);

    // **Die harte Zahl, nicht „mindestens eins".** Zwei Arten laufen im
    // selben Aufruf ab, also stehen genau zwei Bits — und zwar die der
    // Arten 0 und 2, nicht irgendwelche zwei.
    const maske = buffs.expire(GCD_MS);
    expect(maske, 'erwartet sind Bit 0 und Bit 2').toBe(0b101);

    // Die frühere Anzahl ist verlustfrei enthalten: `expiresAt` hält je Art
    // genau einen Eintrag, es kann also höchstens eine je Art ablaufen.
    expect(popcount(maske), 'popcount ist die alte Anzahl').toBe(2);
  });

  it('meldet 0, wenn nichts abläuft — wie die frühere Anzahl', () => {
    // **Der Vergleich, an dem die achte Sammlung hängt.** `combat.ts` prüft
    // `=== 0`, um die Faktoren nicht ohne Anlass neu zu rechnen. Wäre die
    // leere Maske etwas anderes als 0, liefe diese Prüfung ins Leere.
    const buffs = new Buffs();
    buffs.applyForGcds(0, 1.2, 5, 0, GCD_MS);
    expect(buffs.expire(GCD_MS)).toBe(0);
  });

  it('unterscheidet zwei Abläufe, die als Anzahl gleich aussähen', () => {
    // **Der Grund für die Maske in einem Fall.** Vorher trugen beide Abläufe
    // dieselbe 1 — eine Anzahl kann nicht sagen, WELCHE Verstärkung endete.
    const a = new Buffs();
    a.applyForGcds(0, 1.2, 1, 0, GCD_MS);
    const b = new Buffs();
    b.applyForGcds(3, 1.2, 1, 0, GCD_MS);

    const ersteMaske = a.expire(GCD_MS);
    const zweiteMaske = b.expire(GCD_MS);

    expect(popcount(ersteMaske), 'beide lassen genau eine ablaufen').toBe(1);
    expect(popcount(zweiteMaske)).toBe(1);
    expect(
      ersteMaske,
      'als Anzahl wären beide 1 — als Maske müssen sie sich unterscheiden',
    ).not.toBe(zweiteMaske);
  });
});

describe('die Maske trägt die Kapazität', () => {
  it('acht Arten passen in eine JS-Zahl, und die Grenze ist zugesichert', () => {
    // **Ohne diese Zusicherung bricht die Maske still** (#682). Wer
    // `BUFF_KIND_CAPACITY` auf 40 setzt — eine Ein-Zeilen-Änderung an einer
    // harmlos aussehenden Konstante —, bekommt in JavaScript ab Bit 31
    // falsche Ergebnisse: Bit 31 ist das Vorzeichen, ab Bit 32 fällt alles
    // weg. Es gäbe keinen Fehler, keine Warnung und keinen roten Test —
    // nur Einträge, die für die hohen Arten nicht mehr entstehen.
    expect(
      BUFF_KIND_CAPACITY,
      `BUFF_KIND_CAPACITY ist ${BUFF_KIND_CAPACITY} — der Rückgabewert von ` +
        'Buffs.expire() ist eine Bitmaske, und ab 32 Arten passt sie nicht mehr ' +
        'in eine JS-Zahl. Vor dem Hochsetzen muss expire() auf eine andere ' +
        'Form umgestellt werden.',
    ).toBeLessThanOrEqual(BUFF_KIND_MASK_LIMIT);

    // Die Grenze selbst ist keine gegriffene Zahl: Bit 31 ist das
    // Vorzeichenbit, also ist 31 die höchste Anzahl, die sicher trägt.
    expect(BUFF_KIND_MASK_LIMIT).toBe(31);
  });

  it('das höchste Bit der heutigen Kapazität bleibt positiv', () => {
    // Die Zusicherung oben prüft eine Zahl. Diese prüft, dass die Rechnung
    // dahinter stimmt — ein Muster, das bei 8 Arten trägt und bei 32 nicht.
    expect(1 << (BUFF_KIND_CAPACITY - 1)).toBeGreaterThan(0);
    expect(1 << BUFF_KIND_MASK_LIMIT, 'Bit 31 ist bereits negativ').toBeLessThan(0);
  });
});

describe('der Ablauf steht im Verlauf des Kampfes', () => {
  it('SLAG läuft ab und schreibt genau einen Eintrag der Art 7, mit seiner Art als Wert', () => {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    const log = new CombatLog(4096);
    combat.log = log;
    combat.beginFight(0);

    // **Die Zusicherung vor der Messung.** Käme SLAG nicht durch, liefe der
    // Fall über einen Kampf ohne Verstärkung — und wäre grün, weil dann
    // nichts abläuft und nichts geschrieben wird.
    const slag = alsSpellIndex(combat.indexOf('slag'));
    expect(slag, 'SLAG fehlt in der Schule').toBeGreaterThanOrEqual(0);
    expect(combat.cast(slag, 0).result, 'SLAG wurde abgewiesen').toBe(CAST_OK);

    // SLAG hält vier Wartezeiten (6000 ms). Der Lauf geht darüber hinaus,
    // damit der Ablauf sicher fällt.
    for (let t = 0; t <= 9000; t += TICK_MS) combat.stepPassive(t);

    const eintraege = log.toArray().filter((e) => e.type === 'buffExpire');
    expect(eintraege.length, 'kein Eintrag der Art 7 — der Ablauf ist stumm').toBe(1);

    // **Der Wert ist die ART, nicht die Anzahl.** SLAG setzt `kind: 1`
    // (calcination.json). Trüge der Eintrag stattdessen eine Anzahl, stünde
    // hier eine 1, weil genau einer ablief — die beiden wären an dieser
    // Stelle nicht zu unterscheiden. Deshalb prüft der Fall darüber die
    // Maske mit den Arten 0 und 2.
    expect(eintraege[0]!.value, 'SLAG setzt kind 1').toBe(1);
  });
});

describe('alle acht Sammlungen schreiben', () => {
  /**
   * Je Sammlung ein Prüf-Zauber aus `data/spells/test-buff.json`. Die acht
   * decken genau die acht `target`-Werte ab, die `expireBuffs` abräumt —
   * `test-buff` selbst trägt kein `target` und bedient damit die
   * Schadens-Verstärkungen, die achte Sammlung.
   *
   * **Ohne diese Datei wäre die Auflage nicht baubar:** In `calcination.json`
   * tragen nur zwei Zauber ein `buff`-Feld (SLAG → cap, FLASHPOINT →
   * resource). Sechs Sammlungen liefen dort nie ab, und ihr Ausbau fiele
   * nirgends auf.
   */
  const PRUEF_ZAUBER = [
    'test-buff',
    'test-control',
    'test-heat',
    'test-tick',
    'test-stack',
    'test-cdreduce',
    'test-convert',
    'test-cap',
  ] as const;

  it('jeder der acht Prüf-Zauber schreibt beim Ablauf seinen Eintrag', () => {
    for (const id of PRUEF_ZAUBER) {
      const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
      const log = new CombatLog(4096);
      combat.log = log;
      combat.beginFight(0);

      // **Zusicherung je Durchgang.** Käme der Zauber nicht durch, liefe der
      // Durchgang über einen Kampf ohne Verstärkung — und wäre grün, weil
      // dann nichts abläuft. Bei acht Durchgängen fiele ein stiller Ausfall
      // sonst gar nicht auf.
      const index = alsSpellIndex(combat.indexOf(id));
      expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
      expect(combat.cast(index, 0).result, `${id} wurde abgewiesen`).toBe(CAST_OK);

      // Drei Wartezeiten Wirkung, der Lauf geht deutlich darüber hinaus.
      for (let t = 0; t <= 9000; t += TICK_MS) combat.stepPassive(t);

      const eintraege = log.toArray().filter((e) => e.type === 'buffExpire');
      expect(
        eintraege.length,
        `${id} läuft ab, ohne einen Eintrag zu schreiben — seine Sammlung ` +
          'ist von der Schreibstelle nicht erfasst',
      ).toBe(1);
    }
  });
});

/** Zählt die gesetzten Bits — die Anzahl, die `expire` früher zurückgab. */
function popcount(maske: number): number {
  let n = 0;
  for (let bit = 0; bit < 32; bit++) if ((maske & (1 << bit)) !== 0) n++;
  return n;
}
