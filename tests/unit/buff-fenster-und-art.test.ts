/**
 * GRIPLOCKs Fenster und QUICKSILVERs Art (Issue #489, #555).
 *
 * # Zwei Lücken, die dieselbe Datei fast schon gedeckt hätte
 *
 * `tools/groessen-mutation.py`, Vollmessung aller 116 Paare:
 *
 * ```text
 *   buff.gcds  griplock     4 -> 2   NUR FRISCHE
 *   buff.kind  quicksilver  0 -> 1   SCHWEIGT
 * ```
 *
 * `buff-ersetzung-ueber-zauber.test.ts` prüft die Ersetzungs-Regel bereits —
 * aber nur für den Speicher `control` (GRIPLOCK gegen TEST-CONTROL). Für
 * `convert` gab es keinen Gegenpart, und `umwandlung-groessen.test.ts` misst
 * dort zwar Faktor, Dauer und Ziel, nie die **Art**.
 *
 * `SCHWEIGT` heißt: Nicht einmal die Vergleichsdatei färbte. Der Grund ist
 * derselbe wie bei `slag.buff.kind` (#555): Die Art wird erst sichtbar, wenn
 * ein **zweiter** Verstärker denselben Speicher trifft. Läuft nur einer, ist
 * es gleichgültig, auf welchem Platz er liegt.
 *
 * # Beide Größen an ihrer Wirkung gemessen
 *
 * ```text
 *   GRIPLOCK, BIND danach       ohne  im Fenster  gcds halbiert
 *     nach 1500 ms                2       4            4
 *     nach 4500 ms                2       4            2      <- der Fänger
 *     nach 6000 ms                2       2            2
 *
 *   QUICKSILVER + TEST-CONVERT, dann TEMPER + KINDLE
 *     ohne Verstärker            25
 *     mit einem                  50
 *     mit beiden, ersetzend      50      <- die Regel
 *     mit beiden, stapelnd      100      <- gemessen mit kind: 1
 * ```
 *
 * Die 100 sind kein Rechenweg, sondern ein Lauf mit mutierter Datei. Ohne ihn
 * bliebe offen, ob die Mutation überhaupt etwas bewegt — und ein Fall, der
 * eine unbewegliche Zahl festnagelt, sieht aus wie Schutz und ist keiner.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * Wirft GRIPLOCK bei 0 und BIND bei `bindBei` — wie viele Punkte bringt BIND?
 *
 * **Gemessen wird die Wirkung, nicht der Faktor.** Der Kontroll-Faktor liegt
 * in einem privaten Speicher; BIND bringt 2 Punkte, und der Verstärker
 * verdoppelt sie. Damit steht das Fenster in einer Zahl, die der Kampf
 * ohnehin führt.
 */
function punkteNachBind(bindBei: number, mitGriplock: boolean): number {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = alsSpellIndex(combat.indexOf('bind'));

  if (mitGriplock) {
    const griplock = alsSpellIndex(combat.indexOf('griplock'));
    combat.stepPassive(0);
    if (combat.cast(griplock, 0).result !== CAST_OK) return -1;
  }

  const vorher = combat.control;
  combat.stepPassive(bindBei);
  combat.cast(bind, bindBei);
  return combat.control - vorher;
}

/**
 * Wirft die Verstärker nacheinander, dann TEMPER und KINDLE.
 *
 * KINDLE richtet fest 100 an; TEMPER wandelt 25 % davon um. Ein Verstärker
 * auf `convert` verdoppelt den Satz — zwei ersetzen einander, statt sich zu
 * multiplizieren.
 */
function umgewandelt(verstaerker: readonly string[]): { converted: number; wuerfe: number } {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  let zeit = 0;
  let wuerfe = 0;

  for (const id of verstaerker) {
    const index = alsSpellIndex(combat.indexOf(id));
    expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
    combat.stepPassive(zeit);
    if (combat.cast(index, zeit).result === CAST_OK) wuerfe++;
    zeit += 1500;
  }

  combat.stepPassive(zeit);
  combat.cast(alsSpellIndex(combat.indexOf('temper')), zeit);
  zeit += 1500;
  combat.stepPassive(zeit);
  combat.cast(alsSpellIndex(combat.indexOf('kindle')), zeit);
  return { converted: combat.converted, wuerfe };
}

describe('GRIPLOCKs Fenster hält vier Wartezeiten (buff.gcds)', () => {
  it('die Zusicherung: BIND bringt ohne Verstärker zwei Punkte', () => {
    // Stünde hier schon 4, sagte jede Zahl darunter nichts.
    expect(punkteNachBind(1500, false), 'BIND bringt nicht zwei Punkte').toBe(2);
    expect(punkteNachBind(4500, false)).toBe(2);
  });

  it('nach 4500 ms gilt der Verstärker noch — vier Punkte', () => {
    // **Das ist der Fänger.** Halbiert man `gcds` auf 2, endet das Fenster bei
    // 3000 ms und hier stünden 2. Gemessen mit mutierter Datei, nicht
    // hergeleitet.
    expect(punkteNachBind(4500, true), 'das Fenster endet vor 4500 ms').toBe(4);
  });

  it('nach 6000 ms nicht mehr — die Grenze liegt bei vier Wartezeiten', () => {
    // 4 × GCD (1500 ms) = 6000 ms, und die Grenze ist ausschließend. Ohne
    // diesen Fall wäre auch ein Fenster von zehn Wartezeiten grün.
    expect(punkteNachBind(6000, true), 'das Fenster reicht über 6000 ms hinaus').toBe(2);
  });

  it('und dazwischen ist es durchgehend offen, nicht nur an einem Punkt', () => {
    expect(punkteNachBind(1500, true)).toBe(4);
    expect(punkteNachBind(3000, true)).toBe(4);
  });
});

describe('QUICKSILVER teilt sich seinen Platz mit TEST-CONVERT (buff.kind)', () => {
  it('die Zusicherungen: beide wirken einzeln, und zwar gleich stark', () => {
    // Wirkte einer der beiden gar nicht, zeigte der Vergleich unten eine
    // Ersetzung, wo nur ein einzelner Verstärker liegt.
    expect(umgewandelt([]).converted, 'TEMPER wandelt nicht 25 % um').toBe(25);
    expect(umgewandelt(['quicksilver']).converted, 'QUICKSILVER verdoppelt nicht').toBe(50);
    expect(umgewandelt(['test-convert']).converted, 'TEST-CONVERT verdoppelt nicht').toBe(50);
  });

  it('beide Würfe kommen durch', () => {
    expect(umgewandelt(['quicksilver', 'test-convert']).wuerfe, 'ein Wurf wurde abgewiesen').toBe(
      2,
    );
  });

  it('der zweite ersetzt den ersten, statt sich mit ihm zu multiplizieren', () => {
    // Beide tragen `kind: 0` und `target: convert`. Multiplikativ wären es
    // Faktor 4 und damit 100 — gemessen mit `kind: 1` in der Datei, also
    // belegt und nicht gerechnet. Ersetzend bleibt es bei 50.
    expect(
      umgewandelt(['quicksilver', 'test-convert']).converted,
      'die beiden Verstärkungen stapeln statt zu ersetzen',
    ).toBe(50);
  });
});
