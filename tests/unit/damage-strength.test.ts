/**
 * Wirkt jeder Faktor der Schadensformel gleich stark? (Issue #356)
 *
 * WAS ES AUF DIESER SEITE NICHT GAB
 * ---------------------------------
 * Der Simulator hat den Wächter seit #346 (`alle_faktoren_wirken_gleich_stark`).
 * Die Spielseite hatte **keinen** — und zwar für keinen der acht Faktoren.
 * Gemessen von Worker-1:
 *
 * ```
 * grep "gleich stark" über tests/   ein Treffer, aus base-stats.test.ts (#282),
 *                                   etwas anderes
 * ```
 *
 * `damage-order.test.ts` prüft die **Reihenfolge** der Multiplikationen, nicht
 * die Stärke. Die Aussage „jeder Faktor wirkt gleich stark" gab es nur auf der
 * Rust-Seite — und die Spielseite ist die, die der Spieler erlebt.
 *
 * WARUM DIE NAMEN AUS ZWEI QUELLEN KOMMEN
 * ---------------------------------------
 * Eine Liste im Prüffall wäre die, die jemand vergisst. Also nimmt der Wächter,
 * was ohnehin dasteht:
 *
 * | Quelle | Was sie trägt |
 * | --- | --- |
 * | `Object.keys(NEUTRAL_FACTORS)` | was das **Literal** führt |
 * | der Quelltext von `computeDamage` | was die **Formel** benutzt |
 *
 * **Beide, weil eine allein eine Lücke lässt.** Gemessen am 2026-08-05:
 *
 * ```
 * A) readonly probe: number    an DamageFactors
 *    -> tsc: "Property 'probe' is missing ... but required"   der Typ erzwingt
 *
 * B) readonly probe?: number   plus `value *= factors.probe ?? 1`
 *    -> tsc SCHWEIGT                                          der Typ erzwingt nichts
 * ```
 *
 * Im Fall B stünde ein Faktor in der Formel und nicht im Literal;
 * `Object.keys` sähe ihn nicht, und der Wächter wäre still blind. Der
 * Namensvergleich fängt ihn.
 *
 * Das ist Worker-1s Unterscheidung aus #346, eine Ebene tiefer: Der Typ
 * erzwingt, dass jemand die Stelle **anfasst** — aber nur bei einem
 * Pflichtfeld.
 *
 * WAS ER NICHT KANN
 * -----------------
 * Er prüft die Formel in `computeDamage`. Wer eine **zweite** Schadensstelle
 * baut, ist nicht erfasst. Auf der Rust-Seite gilt dasselbe; hier steht es.
 */
import { describe, expect, it } from 'vitest';
import { NEUTRAL_FACTORS, computeDamage, type DamageFactors } from '../../src/core/damage.js';

const QUELLE = import.meta.glob('/src/core/damage.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function damageQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length < 500) {
    throw new Error('damage.ts nicht geladen — die Prüfung würde nichts messen');
  }
  return text;
}

/**
 * Die Faktoren, die `computeDamage` wirklich multipliziert — aus dem Quelltext.
 *
 * **Nur der Rumpf dieser einen Funktion**, nicht die ganze Datei: `factors.`
 * kommt auch in Kommentaren und in anderen Funktionen vor, und eine Erwähnung
 * ist keine Multiplikation.
 */
export function faktorenInDerFormel(quelle: string): string[] {
  const beginn = quelle.indexOf('export function computeDamage');
  if (beginn < 0) throw new Error('computeDamage nicht gefunden');
  const ende = quelle.indexOf('\n}', beginn);
  const rumpf = quelle.slice(beginn, ende);
  return [...new Set([...rumpf.matchAll(/value \*= factors\.(\w+)/g)].map((t) => t[1] as string))];
}

/** Ein Satz Faktoren, in dem genau einer von 1 abweicht. */
function nurEinerAnders(name: string, wert: number): DamageFactors {
  return { ...NEUTRAL_FACTORS, [name]: wert };
}

const PROBEWERT = 1.25;
const GRUNDSCHADEN = 1000;

describe('Die zwei Quellen der Faktornamen stimmen überein', () => {
  it('liest überhaupt Namen aus beiden', () => {
    // Ohne diese Zusicherung liefe alles Weitere über leere Mengen — und zwei
    // leere Mengen sind gleich.
    expect(Object.keys(NEUTRAL_FACTORS).length).toBeGreaterThanOrEqual(8);
    expect(faktorenInDerFormel(damageQuelle()).length).toBeGreaterThanOrEqual(8);
  });

  it('führt die Formel keinen Faktor, den das Literal nicht kennt', () => {
    // **Der Fall, den der Übersetzer NICHT fängt** (gemessen: ein optionales
    // Feld plus seine Verwendung in der Formel lässt `tsc` vollständig
    // schweigen). Dann stünde der Faktor in der Rechnung und in keiner
    // Prüfung.
    const imLiteral = new Set(Object.keys(NEUTRAL_FACTORS));
    const fehlend = faktorenInDerFormel(damageQuelle()).filter((n) => !imLiteral.has(n));
    expect(
      fehlend,
      `diese Faktoren multipliziert computeDamage, ohne dass sie in ` +
        `NEUTRAL_FACTORS stehen: ${fehlend.join(', ')}. Ein optionales Feld am ` +
        `Typ erzwingt keinen Eintrag im Literal — dann sieht der Wächter unten ` +
        `den Faktor nicht.`,
    ).toEqual([]);
  });

  it('führt das Literal keinen Faktor, den die Formel nicht benutzt', () => {
    // Die Gegenrichtung. Sie fiele beim Stärke-Fall unten ohnehin auf — ein
    // Faktor, den niemand multipliziert, bewegt den Schaden nicht. Hier
    // bekommt sie einen NAMEN statt eines „wirkt nicht gleich stark".
    const inDerFormel = new Set(faktorenInDerFormel(damageQuelle()));
    const ungenutzt = Object.keys(NEUTRAL_FACTORS).filter((n) => !inDerFormel.has(n));
    expect(ungenutzt, `im Literal, aber nicht in der Formel: ${ungenutzt.join(', ')}`).toEqual([]);
  });
});

describe('Jeder Faktor wirkt gleich stark', () => {
  const namen = Object.keys(NEUTRAL_FACTORS);

  it('rechnet neutral genau den Grundschaden', () => {
    // Der Bezugspunkt. Ohne ihn misst der Vergleich unten zwei Zahlen, von
    // denen keine bekannt ist.
    expect(computeDamage(GRUNDSCHADEN, NEUTRAL_FACTORS)).toBe(GRUNDSCHADEN);
  });

  // **Jeder einzeln.** Ein Fall über alle acht zusammen wäre schon erfüllt,
  // wenn EINER wirkt — und genau der, der beim nächsten Umbau verrutscht,
  // wäre dann nicht der geprüfte.
  for (const name of namen) {
    it(`multipliziert über ${name} wie über jeden anderen`, () => {
      const erwartet = Math.round(GRUNDSCHADEN * PROBEWERT);
      expect(computeDamage(GRUNDSCHADEN, nurEinerAnders(name, PROBEWERT))).toBe(erwartet);
    });
  }

  it('nennt den Namen, wenn einer abweicht — die Gegenprobe von innen', () => {
    // Ohne diesen Fall wüsste niemand, ob die Fälle oben überhaupt anschlagen
    // KÖNNEN. Hier wird ein Faktor absichtlich verstümmelt: Der Vergleich muss
    // ihn finden UND benennen.
    const verstuemmelt = namen.map((name) => {
      const wert = computeDamage(
        GRUNDSCHADEN,
        // `synergy` wird halb so stark angesetzt — so verhielte sich ein
        // Faktor, den jemand an der falschen Stelle addiert statt multipliziert.
        nurEinerAnders(name, name === 'synergy' ? 1.125 : PROBEWERT),
      );
      return { name, wert };
    });
    const erwartet = Math.round(GRUNDSCHADEN * PROBEWERT);
    const abweichend = verstuemmelt.filter((f) => f.wert !== erwartet).map((f) => f.name);
    expect(abweichend, 'die Prüfung findet den abweichenden Faktor und nennt ihn').toEqual([
      'synergy',
    ]);
  });
});
