/**
 * SCAB — der erste COAGULATION-Zauber mit Schaden (Issue #481, Raster 1 von 9).
 *
 * # Warum er zwei Größen zugleich bewegen muss
 *
 * Seine Rolle im Raster ist *„Füller mit Schaden, legt eine dünne Schicht an"*.
 * Das sind **zwei** Aussagen, und ein Prüffall, der nur eine davon misst, ließe
 * die andere unbelegt:
 *
 * | | |
 * | --- | --- |
 * | Schaden | die Schule hatte bis hierher **keinen** Zauber mit Schaden |
 * | Schicht | dünn (200 gegen CRUSTs 900) — nebenher, nicht statt CRUST |
 *
 * # AC3 IST DER KERN UND WIRD LEICHT ZU WEICH GEBAUT
 *
 * „SCAB macht Schaden" ist trivial und wäre auch wahr, wenn der Schaden aus
 * einer ganz anderen Quelle käme. Was belegt, dass **er** es tut, ist die
 * Gegenprobe: derselbe Lauf ohne ihn, und die gemessene Größe bewegt sich
 * nicht.
 *
 * Das ist wörtlich die Klasse „gebaut, geprüft, nicht angeschlossen", die
 * dieses Projekt neun Mal getroffen hat.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

/**
 * Einen Lauf fahren und die beiden Größen zurückgeben, um die es geht.
 *
 * `wirken` entscheidet, ob SCAB gedrückt wird — **derselbe Lauf**, ein Schalter.
 * Zwei getrennt geschriebene Läufe wären die Stelle, an der sich ein
 * Unterschied einschleicht, der nichts mit dem Zauber zu tun hat.
 */
function lauf(wirken: boolean, bisMs = 6000): { schaden: number; schild: number } {
  const combat = kampf();
  const scab = combat.indexOf('scab');
  for (let t = 0; t <= bisMs; t += 10) {
    combat.stepPassive(t);
    if (wirken && combat.gcd.isReady(t) && combat.isOffCooldown(scab, t)) combat.cast(scab, t);
  }
  return { schaden: combat.damageDealt, schild: combat.shield };
}

describe('AC1 — der Datensatz wird angenommen', () => {
  it('SCAB steht im Index-Raum und ist wirkbar', () => {
    // `indexOf` liefert **-1 statt zu werfen** (siehe `Combat.indexOf`, #372).
    // Ohne diese Zeile liefen alle folgenden Fälle mit `cast(-1)` — still,
    // wirkungslos und grün.
    expect(kampf().indexOf('scab')).toBeGreaterThanOrEqual(0);
  });
});

describe('AC2 — SCAB ist im Kampf wirksam, an BEIDEN Größen', () => {
  it('er macht Schaden — als erster Zauber dieser Schule', () => {
    expect(lauf(true).schaden).toBeGreaterThan(0);
  });

  it('und legt gleichzeitig eine Schicht an', () => {
    expect(lauf(true).schild).toBeGreaterThan(0);
  });

  it('die Schicht ist DÜNN — sie ersetzt CRUST nicht', () => {
    // Die Aussage des Rasters ist nicht „SCAB gibt Schutz", sondern „eine
    // dünne Schicht". Ohne diese Schranke wäre ein SCAB mit 900 Schutz
    // ebenfalls grün — und dann gäbe es keinen Grund mehr, CRUST zu drücken.
    const combat = kampf();
    combat.cast(combat.indexOf('scab'), 0);
    const duenn = combat.shield;
    const anderer = kampf();
    anderer.cast(anderer.indexOf('crust'), 0);
    // **Beide Schranken, und die untere ist die wichtigere.** Gemessen: Ohne
    // den Datensatz liefert `indexOf` -1, `cast` tut still nichts, `duenn`
    // ist 0 — und `0 < 900` wäre trivial wahr. Der Fall wäre grün geblieben,
    // ohne dass es SCAB überhaupt gibt.
    expect(duenn, 'ohne Schicht ist der Vergleich mit CRUST bedeutungslos').toBeGreaterThan(0);
    expect(duenn).toBeLessThan(anderer.shield);
  });
});

describe('AC3 — Gegenprobe: ohne ihn bewegt sich nichts', () => {
  it('kein Schaden, wenn er nicht gedrückt wird', () => {
    // **Derselbe Lauf, nur der Schalter umgelegt.** Bewegt sich die Größe
    // trotzdem, kommt sie nicht von SCAB — dann prüft der Fall darüber etwas
    // ganz anderes.
    expect(lauf(false).schaden).toBe(0);
  });

  it('und keine Schicht', () => {
    expect(lauf(false).schild).toBe(0);
  });

  it('ZUSICHERUNG: der Lauf drückt ihn auch wirklich mehrfach', () => {
    // Ohne diese Zahl wäre nicht zu sehen, ob der Lauf SCAB überhaupt
    // erreicht. Bei 1500 ms Abklingzeit sind über 6 s mindestens drei Drücke
    // möglich — er ist ein FÜLLER, und das ist die Aussage.
    const combat = kampf();
    const scab = combat.indexOf('scab');
    let wirksam = 0;
    let vorher = 0;
    for (let t = 0; t <= 6000; t += 10) {
      combat.stepPassive(t);
      if (combat.gcd.isReady(t) && combat.isOffCooldown(scab, t)) {
        combat.cast(scab, t);
        // **Gezählt wird die WIRKUNG, nicht der Druck.** Gemessen: Ohne den
        // Datensatz drückt dieselbe Schleife munter weiter — `cast(-1)` tut
        // still nichts, und ein Zähler über Drücke stünde trotzdem bei vier.
        // Dieselbe Falle wie bei `casts[stall] > 0` in `parity_fixation`.
        if (combat.shield > vorher) wirksam++;
        vorher = combat.shield;
      }
    }
    expect(
      wirksam,
      'SCAB kam kaum zum Zug — dann misst dieser Fall keinen Füller',
    ).toBeGreaterThanOrEqual(3);
  });
});
