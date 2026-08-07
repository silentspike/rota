/**
 * Verschiedene Arten stapeln multiplikativ — auch auf dem Weg über einen
 * ZAUBER (Issue #562).
 *
 * # Was #555 offenlassen musste, und warum
 *
 * GAME-DESIGN 11.3 hat zwei Hälften:
 *
 * > Verschiedene Bonus-Arten stapeln multiplikativ, **gleiche Arten ersetzen
 * > einander.**
 *
 * `buff-ersetzung-ueber-zauber.test.ts` (#555) belegt die zweite. Die erste
 * war über einen Zauber **nicht baubar**, und der Kopf dort nennt den Grund:
 * Kein Speicher beherbergte zwei verschiedene Arten. Gemessen über alle
 * Zauber-Dateien trug von dreizehn Verstärkern genau EINER `kind: 1` — SLAG,
 * allein im Speicher `cap`.
 *
 * `test-cap` ist der erste Partner für SLAG. Damit ist die Lage herstellbar,
 * die das Design vorsieht und der Bestand nicht hergab.
 *
 * # Die Zahl, an der alles hängt
 *
 * ```text
 *   ohne Verstärker                       20     die Grunddeckelung
 *   nur SLAG        (kind 1, 1.5)         30
 *   nur test-cap    (kind 0, 2.0)         40
 *   beide zusammen                        60     <- multiplikativ, 11.3 Satz 1
 *   beide, wenn SLAG auf kind 0 stünde    40     <- ersetzend, der Fehlerfall
 * ```
 *
 * 60 gegen 40 trennt die Regel von ihrer Verletzung. Eine Richtung
 * (`> 40`) täte es nicht — sie bliebe grün, wenn der Deckel aus einem
 * anderen Grund stiege.
 *
 * # Warum dieser Fall über `cast` geht und nicht über `Buffs`
 *
 * `buffs.test.ts` ruft `applyForGcds(kind, …)` direkt mit einer festen Zahl
 * auf und prüft damit die Mechanik. Die LEITUNG — Datei → `buff.kind` →
 * `Buffs` — prüft es nicht. Genau daran schwieg die Mutation
 * `slag.buff.kind 1 -> 0` über die ganze Suite, bis #555 den ersten Fall
 * dieser Art baute. Dieselbe Klasse wie #605 und #619.
 *
 * # Die Sollwerte stehen HART, und das ist der Punkt
 *
 * Läse dieser Fall die Faktoren aus `slag.buff.factor` und
 * `test-cap.buff.factor`, läse er bei einer Gegenprobe die **mutierte** Zahl
 * mit und prüfte die Formel gegen sich selbst. Die Begründung steht
 * ausführlich in `slag.test.ts` über dem 1.5-Fall; sie gilt hier genauso.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

const BOSS = FIRST_CRUCIBLE.value;
/** Die Grunddeckelung aus `calcination.json` — 20. */
const CAP = CALCINATION.value.resource.cap;

/**
 * Wirft die genannten Verstärker und liest den DECKEL ab.
 *
 * **`setResource(9999)` misst die Grenze, nicht was ein Erzeuger schafft.**
 * Der Umweg über BELLOWS wäre hier falsch: Er kommt im Fenster auf 22 und
 * berührte den auf 60 gehobenen Deckel nie — der Fall verglichen zwei Zahlen,
 * von denen die eine nichts mit der Grenze zu tun hat. Dieselbe Falle ist in
 * `slag.test.ts` schon einmal zugeschnappt und dort dokumentiert.
 *
 * Der Wert klemmt gegen `resourceCap`, also gegen die eine Rechenstelle
 * (`combat.ts`, `Math.round(cap * capBuffs.factor(...))`).
 */
function deckelMit(verstaerker: readonly string[]): { deckel: number; wuerfe: number } {
  const combat = new Combat({ school: CALCINATION.value, boss: BOSS });
  let wuerfe = 0;
  let zeit = 0;

  // Je eine Wartezeit auseinander, damit der GCD keinen abweist. SLAG hält
  // vier Wartezeiten (bis 6000), test-cap drei ab seinem Wurf (1500 + 4500 =
  // 6000) — bei der Messung um 3100 sind beide offen.
  for (const id of verstaerker) {
    const index = alsSpellIndex(combat.indexOf(id));
    expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
    if (combat.cast(index, zeit).result === 0) wuerfe++;
    zeit += 1500;
  }

  // Der Faktor wird zu `nowMsForBuffs` gelesen — ohne die Takte stünde die
  // Zeit noch beim letzten Wurf.
  for (let t = 0; t <= 3100; t += 10) combat.stepPassive(t);

  combat.setResource(9999);
  return { deckel: combat.resource, wuerfe };
}

describe('AC1/AC2 — verschiedene Arten stapeln multiplikativ, über Zauber gewirkt', () => {
  it('die Voraussetzung: der Bestand deckelt bei 20 und beide Würfe kommen durch', () => {
    // **Die Zusicherung vor der Messung.** Wird einer der beiden abgewiesen
    // (Wartezeit, Abklingzeit, Zauber fehlt), misst der Vergleich darunter
    // einen Lauf mit nur einer Verstärkung — und wäre grün, ohne etwas zu
    // zeigen. Genau das ist mir bei #555 im ersten Entwurf passiert.
    expect(deckelMit([]).deckel, 'die Grunddeckelung ist nicht 20').toBe(CAP);
    const beide = deckelMit(['slag', 'test-cap']);
    expect(beide.wuerfe, 'einer der beiden Verstärker wurde abgewiesen').toBe(2);
  });

  it('jeder für sich hebt den Deckel um seinen eigenen Faktor', () => {
    // Die beiden Einzelwerte, ohne die der Vergleich darunter nicht zu deuten
    // wäre: 60 könnte sonst auch aus einem einzelnen Faktor 3 stammen.
    expect(deckelMit(['slag']).deckel, 'SLAG hebt nicht um die Hälfte').toBe(30);
    expect(deckelMit(['test-cap']).deckel, 'test-cap verdoppelt nicht').toBe(40);
  });

  it('zusammen multiplizieren sie sich, statt einander zu ersetzen', () => {
    // **Die Aussage.** 20 × 1.5 × 2.0 = 60. Ersetzten sie einander, bliebe
    // der stärkere übrig und es wären 40 — genau der Stand, den die Mutation
    // `slag.buff.kind 1 -> 0` herstellt.
    expect(
      deckelMit(['slag', 'test-cap']).deckel,
      'die beiden Verstärkungen ersetzen einander, statt zu stapeln — dann ' +
        'tragen sie dieselbe Art, und 11.3 Satz 1 gilt nicht mehr',
    ).toBe(60);
  });

  it('die Reihenfolge der beiden Würfe ändert nichts', () => {
    // **Multiplikation ist vertauschbar, die Umsetzung muss es auch sein.**
    // Trüge der zweite Wurf den ersten aus dem Vorrat, käme je nach
    // Reihenfolge 30 oder 40 heraus — und ein Fall, der nur eine Reihenfolge
    // prüft, sähe davon die Hälfte.
    expect(deckelMit(['test-cap', 'slag']).deckel).toBe(60);
  });
});

describe('Die Gegenprobe: es liegt an den ARTEN, nicht an der Zahl der Würfe', () => {
  it('zwei Verstärker DERSELBEN Art ersetzen einander weiterhin', () => {
    // **Ohne diese Zeile belegte der Fall oben nur, dass zwei Würfe mehr
    // bewirken als einer.** GRIPLOCK und test-control tragen beide `kind: 0`
    // und `target: control` — sie teilen sich einen Platz. Dass dort NICHT
    // multipliziert wird, ist die andere Hälfte von 11.3 und der Beleg, dass
    // die Stapelung oben aus den verschiedenen Arten kommt.
    //
    // Gemessen wird sie in `buff-ersetzung-ueber-zauber.test.ts` (#555); hier
    // steht nur der Verweis, damit niemand beide Hälften in einem Fall sucht.
    const nurSlag = deckelMit(['slag']).deckel;
    const beide = deckelMit(['slag', 'test-cap']).deckel;
    expect(beide / nurSlag, 'der zweite Verstärker trägt nicht seinen vollen Faktor bei').toBe(2);
  });
});
