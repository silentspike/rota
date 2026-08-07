import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import ERWARTET from './spell-order.json' with { type: 'json' };

/**
 * Die Zauberreihenfolge ändert sich nur **bewusst** (Issue #402, berichtigt).
 *
 * # Die ursprüngliche Begründung hielt nicht — sie ist hier stehengeblieben
 *
 * Hier stand: die Indizes stünden in jeder aufgezeichneten Runde und in jedem
 * gespeicherten Spielstand, ein Einschub deute sie deshalb **still** um.
 *
 * **Beide Hälften waren zum Zeitpunkt dieses Wächters bereits falsch**, und
 * zwar durch Issue #264, das drei Wochen früher lag:
 *
 * ```
 * save-format.ts    bindings: Readonly<Record<string, string>>   Kennung
 * Aufzeichnungen    "spellId": "kindle"                          Kennung
 * ```
 *
 * `bindings.ts` sagt es im selben Atemzug: *„Die Vergleichsdateien der Parität
 * führen die Belegung längst über `spellId`."* Ein `grep` über `SpellIndex`
 * findet in `src/` drei Treffer, alle drei Typdefinitionen — **nirgends wird
 * ein Zauber-Index serialisiert.**
 *
 * # Was gemessen wurde (2026-08-06)
 *
 * Derselbe Zauber zweimal eingefügt, einmal mitten in die Gesamtliste, einmal
 * ganz hinten. Beide Male `parity:export`, beide Male die volle Suite:
 *
 * ```
 *                            MITTE      HINTEN
 * bewegte Aufzeichnungen        23          23
 * rote Tests nach Export         4           3
 * Referenz 4240 @ 40510   unverändert  unverändert
 * ```
 *
 * **23 gegen 23.** Der Grund steht in `checksum.ts`: Die Schleife läuft über
 * `combat.spells.length` und rührt fünf Felder je Zauber ein. Ein Zauber mehr
 * sind fünf Felder mehr — an welcher Stelle, ist gleichgültig. Jede Prüfsumme
 * bewegt sich so oder so.
 *
 * > **Die Regel „nur hinten anhängen" sparte keinen einzigen Neu-Export.** Sie
 * > kostete den fachlich richtigen Ort: Ein Kalzinations-Zauber gehört in
 * > `calcination.json`, nicht in eine Zweitdatei ganz hinten.
 *
 * Der einzige Test, der zwischen „Mitte" und „hinten" unterschied, war dieser
 * hier. **Ein Wächter, der nur noch sich selbst schützt.**
 *
 * # Wozu er trotzdem bleibt
 *
 * Nicht als Verbot, sondern als Meldung: Er sagt, **dass** sich die Liste
 * geändert hat, und zwingt dazu, `spell-order.json` bewusst nachzuziehen —
 * im selben Commit wie der Export. Eine unbeabsichtigte Umsortierung fällt
 * damit weiterhin auf; eine beabsichtigte kostet eine Zeile.
 *
 * # Warum ein Präfix-Vergleich und keine Gleichheit
 *
 * Gleichheit bräche auch dann, wenn hinten ein Zauber dazukommt. Der Wächter
 * prüft deshalb, dass die heutige Liste mit der eingecheckten **beginnt** —
 * Anhängen bleibt ohne Nachziehen möglich, jede andere Änderung meldet sich.
 *
 * # Warum die erwartete Liste in einer eigenen Datei steht
 *
 * Weil sie sonst aus derselben Quelle käme, die sie bewacht. Eine Liste, die
 * sich beim Laufen selbst herstellt, ist immer grün — sie belegte nur, dass
 * `map` funktioniert. Die JSON ist eingecheckt und ändert sich nur, wenn
 * jemand sie bewusst anfasst.
 */
describe('Zauberreihenfolge (Issue #402)', () => {
  const heute = CALCINATION.value.spells.map((s) => s.id);

  it('beginnt unverändert — die Reihenfolge hat sich nicht unbemerkt verschoben', () => {
    // Position für Position, nicht als Ganzes: Die Meldung soll sagen, WO es
    // auseinanderläuft und WELCHE zwei Namen dort stehen. `toEqual` über zwei
    // Listen nennt bei achtzehn Einträgen nur „nicht gleich".
    for (let i = 0; i < ERWARTET.length; i++) {
      expect(
        heute[i],
        `Position ${i}: erwartet "${ERWARTET[i]}", gefunden "${heute[i] ?? '(fehlt)'}". ` +
          `Die Zauberreihenfolge hat sich geändert. War das ABSICHT — etwa ein neuer ` +
          `Zauber in seiner Schuldatei —, dann ziehe "spell-order.json" nach und fahre ` +
          `"npm run parity:export" im SELBEN Commit. War es keine Absicht, hast du ` +
          `gerade unbemerkt umsortiert.`,
      ).toBe(ERWARTET[i]);
    }
  });

  it('darf hinten gewachsen sein, aber nicht geschrumpft', () => {
    expect(
      heute.length,
      `Die Liste ist kürzer geworden (${heute.length} statt ${ERWARTET.length}). ` +
        `Ein entfernter Zauber verschiebt alle Indizes dahinter.`,
    ).toBeGreaterThanOrEqual(ERWARTET.length);
  });

  it('enthält keinen Namen doppelt', () => {
    // Sonst zeigten zwei Indizes auf denselben Zauber, und der Präfix-Vergleich
    // oben bliebe grün, während `indexOf` etwas anderes liefert als gedacht.
    expect(new Set(heute).size, `Doppelte Zauber-ID in SPELLS`).toBe(heute.length);
  });
});
