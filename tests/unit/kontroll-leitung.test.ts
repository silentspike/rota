/**
 * Ein gescheiterter Kontroll-Zauber ist von einem gelungenen unterscheidbar
 * (Issue #605).
 *
 * # Der Befund: es fehlte nicht die Grösse, sondern die Leitung
 *
 * `combat.ts` bricht die Buchung ab, wenn die Punkte nicht reichen, und zählt
 * `controlFailed` hoch. Die Zahl steht im `RunSummary` und wird nach jedem
 * Lauf als „Control wasted" angezeigt — sie war also nie unsichtbar.
 *
 * **Nur kam sie im `ReplayResult` nicht an**, und der Balance-Läufer wie jede
 * Hebel-Messung lesen ausschliesslich das. Damit konnte keine Aufzeichnung je
 * belegen, dass ein Kontroll-Zauber mangels Punkten scheiterte.
 *
 * Gemessen an `fixation.json#1`, `anchor.control.cost` von 6 auf 999:
 *
 * ```text
 *   vor #605    damage 900   casts.anchor 2      (beide Läufe identisch)
 *   seit #605   cost 6    -> controlSpent 23, controlFailed 0
 *               cost 999  -> controlSpent 11, controlFailed 2
 * ```
 *
 * `damage` und `casts.anchor` bleiben in beiden Fällen gleich. Genau deshalb
 * war der Unterschied vorher nicht zu sehen — und deshalb standen in #592 alle
 * elf Kontroll-Grössen auf 0,00 %.
 *
 * # Warum `casts` einen gescheiterten Zauber weiter zählt
 *
 * Ein Druck, der angenommen wurde und einen GCD ausgelöst hat, **ist**
 * gewirkt — nur seine Wirkung blieb aus. Wer ihn nicht zählte, verlöre die
 * Unterscheidung zwischen „nicht gedrückt" und „gedrückt, aber wirkungslos",
 * und die zweite ist die interessantere. `controlFailed` steht daneben, nicht
 * statt dessen. (Entschieden am 2026-08-07, Issue #605 AC3.)
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { replayRecording } from '../../src/devtools/recording.js';

type Lauf = {
  controlSpent: number;
  controlFailed: number;
  anchorWuerfe: number;
  punkteAmEnde: number;
};

/**
 * Wirft ANCHOR entweder sofort (Punkte reichen nicht) oder nach genug BIND.
 *
 * **Ohne Datei-Mutation.** Die Messung, die das Issue nennt, setzt
 * `anchor.control.cost` auf 999 — das geht in einem Prüffall nicht sauber, weil
 * er dann die Datenlage der ganzen Suite verändert. Dieselbe Lage entsteht,
 * wenn man ANCHOR wirft, BEVOR genug Punkte gesammelt sind: Der Zweig, der
 * `controlFailed` hochzählt, ist derselbe.
 */
function lauf(sammelnVorher: boolean): Lauf {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  const anchor = alsSpellIndex(combat.indexOf('anchor'));
  let anchorWuerfe = 0;

  // **ANCHOR wird GENAU EINMAL geworfen.** Der erste Entwurf liess ihn wirken,
  // sooft er bereit war — bei 20 s Abklingzeit und BIND dazwischen gelang der
  // zweite Wurf dann doch, und `controlSpent` stand auf 6 statt 0. Die Aussage
  // war richtig und die Lage unsauber: Sie enthielt beide Fälle zugleich.
  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    // ANCHOR kostet 6, BIND bringt 2 — vor drei BIND-Würfen reicht es nie.
    const bereit = combat.isReady(anchor, timeMs);
    const jetzt = sammelnVorher ? combat.control >= 6 : true;
    if (anchorWuerfe === 0 && bereit && jetzt) {
      if (combat.cast(anchor, timeMs).result === CAST_OK) anchorWuerfe++;
    } else if (combat.isReady(bind, timeMs)) {
      combat.cast(bind, timeMs);
    }
  }

  return {
    controlSpent: combat.controlSpent,
    controlFailed: combat.controlFailed,
    anchorWuerfe,
    punkteAmEnde: combat.control,
  };
}

describe('Die Kontroll-Grössen kommen im Lauf-Ergebnis an', () => {
  it('AC2 — zu früh geworfen zählt controlFailed hoch, rechtzeitig nicht', () => {
    const zuFrueh = lauf(false);
    const rechtzeitig = lauf(true);

    expect(zuFrueh.controlFailed, 'ANCHOR ohne Punkte scheitert nicht').toBeGreaterThan(0);
    expect(zuFrueh.controlSpent, 'ohne Punkte wurde trotzdem etwas ausgegeben').toBe(0);

    expect(rechtzeitig.controlFailed, 'mit genug Punkten scheitert ANCHOR trotzdem').toBe(0);
    expect(rechtzeitig.controlSpent, 'mit genug Punkten wurde nichts ausgegeben').toBeGreaterThan(
      0,
    );
  });

  it('AC3 — casts zählt den gescheiterten Zauber weiter als gewirkt', () => {
    // **Die Entscheidung, nicht ihr Nebeneffekt.** Ein Druck, der einen GCD
    // ausgelöst hat, ist gewirkt; ob seine Wirkung eintrat, sagt
    // `controlFailed`. Zählte `casts` ihn nicht, wäre „nicht gedrückt" von
    // „gedrückt, aber wirkungslos" nicht mehr zu unterscheiden.
    const zuFrueh = lauf(false);
    expect(zuFrueh.anchorWuerfe, 'der Wurf wurde gar nicht angenommen').toBeGreaterThan(0);
    expect(zuFrueh.anchorWuerfe, 'angenommene Würfe und Fehlschläge weichen ab').toBe(
      zuFrueh.controlFailed,
    );
  });

  it('und das ist keine leere Aussage: die beiden Läufe unterscheiden sich wirklich', () => {
    // **Die zweite Zahl, die für die Aussage niemand braucht** (Guide 8.67):
    // Beide Läufe müssen ANCHOR überhaupt geworfen haben. Wäre einer davon
    // leer — falscher Boss, Zauber nicht in der Schule —, wären die Vergleiche
    // oben trivial grün.
    const zuFrueh = lauf(false);
    const rechtzeitig = lauf(true);
    expect(zuFrueh.anchorWuerfe).toBeGreaterThan(0);
    expect(rechtzeitig.anchorWuerfe).toBeGreaterThan(0);
    expect(
      rechtzeitig.controlSpent,
      'beide Läufe geben gleich viel aus — dann trennt die Lage nichts',
    ).not.toBe(zuFrueh.controlSpent);
  });

  it('AC1 — das ReplayResult führt beide Grössen, nicht nur der Kampf', () => {
    // **Der Fall, den die drei darüber NICHT abdecken.** Sie lesen
    // `combat.controlSpent` direkt und blieben grün, wenn die Leitung ins
    // `ReplayResult` wieder verschwände — genau der Zustand vor #605. Gemessen:
    // Wird die Zuweisung in `recording.ts` auf 0 zurückgestellt, meldet dieser
    // Fall es, die anderen drei nicht.
    const datei = new URL('../parity/recordings/fixation.json', import.meta.url).pathname;
    const d = JSON.parse(readFileSync(datei, 'utf8')) as {
      bossId: string;
      runs: {
        ticks: number;
        presses: unknown[];
        bindings: unknown[];
        expected: Record<string, number>;
      }[];
    };
    const mitAusgabe = d.runs[1];
    if (mitAusgabe === undefined) throw new Error('fixation.json hat keinen zweiten Lauf');

    const boss = bossById(d.bossId);
    if (boss === undefined) throw new Error(`Der Boss ${d.bossId} fehlt.`);
    const ergebnis = replayRecording(
      {
        name: 'fixation-mit-ausgabe',
        seed: 0,
        ticks: mitAusgabe.ticks,
        presses: mitAusgabe.presses as never,
        bindings: mitAusgabe.bindings as never,
      },
      { boss },
    );

    // Die Aufzeichnung führt die Zahl seit jeher in `expected`; das
    // ReplayResult tat es nicht. Beide müssen dasselbe sagen.
    expect(ergebnis.controlSpent, 'das ReplayResult führt die Ausgabe nicht').toBe(
      mitAusgabe.expected['controlSpent'],
    );
    expect(ergebnis.controlFailed).toBe(mitAusgabe.expected['controlFailed']);

    // **Die zweite Zahl:** Der Lauf muss überhaupt etwas ausgegeben haben,
    // sonst verglichen beide Zeilen zwei Nullen.
    expect(ergebnis.controlSpent, 'der Lauf gibt gar keine Punkte aus').toBeGreaterThan(0);
  });
});
