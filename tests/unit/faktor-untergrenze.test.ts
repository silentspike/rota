/**
 * Kein Verstärkungs-Faktor liegt unter 1 (Issue #656).
 *
 * # Warum eine Zahl, die heute niemand verletzt, einen Wächter braucht
 *
 * `Buffs.factor()` bildet ein **Produkt** über alle offenen Arten. Solange
 * jeder Einzelfaktor `>= 1` ist, kann das Produkt nie knapp unter 1 landen.
 * Genau darauf ruht der Kurzschluss in `combat.ts`:
 *
 * ```ts
 * const abstand = faktor === 1 ? dot.tickIntervalMs : Math.round(dot.tickIntervalMs / faktor);
 * ```
 *
 * Das ist ein **Gleichheitsvergleich auf Fließkomma**. Er ist heute sicher —
 * aber aus einem Zufall des Katalogs, nicht aus dem Entwurf. Gemessen über
 * 3999 Paare `a * (1/a)` in IEEE754-double:
 *
 * ```text
 *   568 von 3999 ergeben 0.9999999999999999 statt 1.0     (14 %)
 *   Dreier-Produkte: 406 von 3481
 * ```
 *
 * Der erste Verstärker mit einem Faktor unter 1 macht das erreichbar.
 *
 * # Und warum das nicht folgenlos wäre
 *
 * Der Kurzschluss selbst ist harmlos: `round(x / 0.9999999999999999)` und
 * `round(x / 1)` sind über 100 000 Werte identisch — die Rundung fängt die
 * Abweichung. Folgenreich ist die **eine Stelle, die den Faktor ungerundet
 * weitergibt**: `updateBuffFactor` → `computeDamage`, wo acht Faktoren
 * multipliziert werden und erst am Ende gerundet wird. Der Fall darunter misst
 * das.
 *
 * # Der Wächter steht auf beiden Seiten
 *
 * Hier für das Spiel, und in `sim/build.rs` neben der bestehenden Zusicherung
 * `factor > 0.0` für den Simulator. Zwei Wächter, weil die beiden Seiten die
 * Daten auf verschiedenen Wegen lesen: Das Spiel liest die JSON-Dateien direkt,
 * der Simulator erzeugt daraus beim Bauen einen Rust-Datensatz. Ein Wächter
 * allein sicherte nur einen der beiden Wege.
 *
 * **Ein abschwächender Verstärker ist nicht verboten.** Wer einen bauen will,
 * nimmt beide Zusicherungen heraus und sichert die Klasse ab — der Zweck ist,
 * dass das eine bewusste Handlung wird statt einer stillen.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { Buffs } from '../../src/combat/schools/buffs.js';
import { computeDamage, NEUTRAL_FACTORS } from '../../src/core/damage.js';

const VERZEICHNIS = new URL('../../data/spells/', import.meta.url).pathname;

type Zauber = { id: string; buff?: { kind: number; factor: number; target?: string } };

/** Alle Verstärker aus den Datensätzen — die Quelle, nicht eine Abschrift. */
function verstaerker(): { id: string; datei: string; factor: number }[] {
  const raus: { id: string; datei: string; factor: number }[] = [];
  for (const datei of readdirSync(VERZEICHNIS).filter((d) => d.endsWith('.json'))) {
    const inhalt = JSON.parse(readFileSync(VERZEICHNIS + datei, 'utf8')) as { spells?: Zauber[] };
    for (const s of inhalt.spells ?? []) {
      if (s.buff !== undefined) raus.push({ id: s.id, datei, factor: s.buff.factor });
    }
  }
  return raus;
}

describe('AC2 — kein Verstärkungs-Faktor liegt unter 1', () => {
  it('die Zusicherung vor der Messung: es gibt überhaupt Verstärker zu prüfen', () => {
    // **Ohne diese Zeile wäre der Fall grün, wenn das Verzeichnis leer bliebe
    // oder der Pfad nicht stimmte** — er prüfte dann null Zauber und sagte
    // trotzdem „keiner liegt unter 1".
    const alle = verstaerker();
    expect(alle.length, 'es wurden gar keine Verstärker gefunden').toBeGreaterThan(10);
  });

  it('alle Faktoren im Datensatz sind >= 1', () => {
    const zuKlein = verstaerker().filter((v) => v.factor < 1);
    expect(
      zuKlein.map((v) => `${v.id} (${v.datei}): ${v.factor}`),
      'Ein Faktor unter 1 macht die Fließkomma-Falle aus #656 erreichbar. Das ist ' +
        'nicht verboten — aber dann muss die Klasse abgesichert werden, sonst ' +
        'zeigt sie sich als 1-Schaden-Abweichung, die niemandem auffällt. ' +
        'Lies den Kopf dieser Datei.',
    ).toEqual([]);
  });
});

describe('AC1 — ein Produkt aus Faktoren >= 1 bleibt >= 1', () => {
  it('das ist die Eigenschaft, auf der der Kurzschluss ruht', () => {
    // **Gemessen an der echten Klasse, nicht nachgerechnet.** Alle im
    // Datensatz vorkommenden Faktoren, paarweise über zwei Arten kombiniert —
    // das ist die Lage, die es bei `cap` wirklich gibt (SLAG × test-cap).
    const faktoren = [...new Set(verstaerker().map((v) => v.factor))];
    expect(faktoren.length, 'keine Faktoren gefunden').toBeGreaterThan(1);

    for (const a of faktoren) {
      for (const b of faktoren) {
        const buffs = new Buffs();
        buffs.applyForGcds(0, a, 3, 0, 1500);
        buffs.applyForGcds(1, b, 3, 0, 1500);
        const p = buffs.factor(0);
        expect(p, `${a} * ${b} ergibt ${p} — unter 1`).toBeGreaterThanOrEqual(1);
      }
    }
  });

  it('und die Gegenprobe: MIT einem Faktor unter 1 wäre es verletzbar', () => {
    // **Ohne diese Zeile belegte der Fall oben nur, dass die heutigen Zahlen
    // passen** — nicht, dass die Eigenschaft an ihnen hängt. Hier steht,
    // warum der Wächter oben nötig ist.
    const buffs = new Buffs();
    buffs.applyForGcds(0, 0.9, 3, 0, 1500);
    expect(buffs.factor(0), 'ein Faktor unter 1 ergibt kein Produkt unter 1').toBeLessThan(1);
  });
});

describe('AC4 — die Abweichung wäre folgenreich, nicht kosmetisch', () => {
  /** Der größte `double` unterhalb von 1 — das Ergebnis eines Produkts wie `0.09 * 11.11…`. */
  const FAST1 = 0.9999999999999999;

  it('ein Faktor von FAST1 statt 1 verschiebt echten Schaden', () => {
    // **Die Zahl, die das Issue trägt.** `computeDamage` multipliziert acht
    // Faktoren und rundet erst am Ende; `updateBuffFactor` ist die einzige
    // Stelle, die den Buff-Faktor ungerundet dorthin gibt.
    let abweichungen = 0;
    let geprueft = 0;
    for (const vulnerability of [1, 1.15, 1.3]) {
      for (let base = 1; base <= 4000; base++) {
        const f = { ...NEUTRAL_FACTORS, vulnerability };
        if (computeDamage(base, { ...f, buffs: 1 }) !== computeDamage(base, { ...f, buffs: FAST1 }))
          abweichungen++;
        geprueft++;
      }
    }
    expect(geprueft).toBe(12000);
    expect(
      abweichungen,
      'die Abweichung ist folgenlos geworden — dann trägt die Begründung von #656 nicht mehr',
    ).toBeGreaterThan(300);
  });

  it('bei neutralen Begleitfaktoren dagegen NICHT — deshalb fällt sie nicht auf', () => {
    // **Der Grund, warum ein Test sie nicht von selbst fängt.** Ohne einen
    // zweiten krummen Faktor ist die Abweichung unsichtbar; erst
    // `vulnerability` (im Spiel `(100 + decayCount * jeStapel) / 100`) macht
    // sie sichtbar.
    let abweichungen = 0;
    for (let base = 1; base <= 4000; base++) {
      const f = { ...NEUTRAL_FACTORS, vulnerability: 1 };
      if (computeDamage(base, { ...f, buffs: 1 }) !== computeDamage(base, { ...f, buffs: FAST1 }))
        abweichungen++;
    }
    expect(abweichungen, 'ohne krummen Begleitfaktor gibt es doch Abweichungen').toBe(0);
  });

  it('und der Kurzschluss selbst ist harmlos — die Rundung fängt ihn', () => {
    // Belegt, dass der Wächter an der richtigen Stelle sitzt: nicht am
    // Vergleich, sondern am ungerundeten Weg.
    let abweichungen = 0;
    for (let x = 1; x <= 20000; x++) {
      if (Math.round(x / 1) !== Math.round(x / FAST1)) abweichungen++;
      if (Math.round(x * 1) !== Math.round(x * FAST1)) abweichungen++;
    }
    expect(abweichungen, 'die Rundung fängt die Abweichung doch nicht').toBe(0);
  });
});
