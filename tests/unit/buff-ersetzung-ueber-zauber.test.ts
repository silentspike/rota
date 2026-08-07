/**
 * Die Ersetzungs-Regel gilt auch auf dem Weg über einen ZAUBER (Issue #555).
 *
 * # Der Befund war präziser als „die Regel ist ungeprüft"
 *
 * Sie **ist** geprüft — `buffs.test.ts:75` erwartet `replacementCount === 2`,
 * `buffs.json` führt den Wert, `sim/tests/buffs.rs` vergleicht ihn. Und sie
 * steckt in der Bauart: `factors[kind]` ist ein Array je Art, zwei gleiche
 * können gar nicht nebeneinander stehen.
 *
 * Trotzdem schwieg die Mutation `slag.buff.kind 1 -> 0` über die ganze Suite
 * (1901 von 1901 grün). Der Grund:
 *
 * > Alle bestehenden Prüfungen rufen `applyForGcds(kind, …)` **direkt** auf,
 * > mit einer festen Zahl. Kein Fall ging den Weg **Datei → `buff.kind` →
 * > `Buffs`**.
 *
 * Die Mechanik war geprüft, die Leitung dorthin nicht — dieselbe Klasse wie
 * #605 und #619.
 *
 * # Was dieser Fall NICHT belegt, und warum er es nicht kann
 *
 * GAME-DESIGN 11.3 hat zwei Hälften:
 *
 * > Verschiedene Bonus-Arten stapeln multiplikativ, **gleiche Arten ersetzen
 * > einander.**
 *
 * Hier steht nur die zweite. Die erste ist über Zauber heute **nicht baubar** —
 * gemessen, je Speicher:
 *
 * ```text
 *   cap        slag(1)                              1 Art
 *   control    griplock(0) · test-control(0)        1 Art
 *   cdreduce   distil(0) · test-cdreduce(0)         1 Art
 *   convert    quicksilver(0) · test-convert(0)     1 Art
 *   resource   flashpoint(0) · test-heat(0)         1 Art
 *   stack      virulence(0) · test-stack(0)         1 Art
 *
 *   Speicher mit mehr als EINER Art: 0
 * ```
 *
 * Kein Speicher beherbergt zwei verschiedene Arten. Die Stapel-Hälfte ist
 * deshalb nur über `Buffs` direkt geprüft (`buffs.test.ts`) — also auf
 * demselben Weg, dessen Lücke dieser Fall hier schliesst. Solange kein Zauber
 * mit `kind: 1` und einem geteilten `target` existiert, bleibt sie so.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/** Beide tragen `kind: 0` und `target: control` — sie teilen sich einen Platz. */
const ERSTER = 'griplock';
const ZWEITER = 'test-control';

/**
 * **Gemessen wird die WIRKUNG, nicht der Faktor.**
 *
 * Der Kontroll-Faktor liegt in einem privaten Speicher; ihn herauszureichen
 * wäre eine Leitung, die nur dieser Fall braucht. BIND bringt 2 Punkte, und
 * der Faktor multipliziert sie — damit steht die Regel in einer Zahl, die der
 * Kampf ohnehin führt:
 *
 * ```text
 *   ohne Verstärker            +2
 *   mit einem (Faktor 2)       +4
 *   mit beiden, ersetzend      +4     <- die Regel
 *   mit beiden, stapelnd       +8     <- was 11.3 verbietet
 * ```
 */
type Lauf = {
  punkteMitEinem: number;
  punkteMitBeiden: number;
  punkteOhne: number;
  wuerfe: number;
};

/**
 * Wirft beide Verstärker nacheinander und liest den Kontroll-Faktor ab.
 *
 * **Über `cast`, nicht über `Buffs.applyForGcds`.** Genau darin liegt der
 * Unterschied zu `buffs.test.ts`: Dieser Weg liest `buff.kind` aus der
 * Zauber-Datei. Wird das Feld dort geändert, muss sich hier etwas bewegen.
 */
function punkteNachBind(verstaerker: readonly string[]): { punkte: number; wuerfe: number } {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const bind = alsSpellIndex(combat.indexOf('bind'));
  let wuerfe = 0;
  let zeit = 0;

  // Die Verstärker nacheinander, je eine Wartezeit auseinander — beide liegen
  // damit im Fenster des ersten (GRIPLOCK hält vier Wartezeiten).
  for (const id of verstaerker) {
    const index = alsSpellIndex(combat.indexOf(id));
    expect(index, `${id} fehlt in der Schule`).toBeGreaterThanOrEqual(0);
    combat.stepPassive(zeit);
    if (combat.cast(index, zeit).result === CAST_OK) wuerfe++;
    zeit += 1500;
  }

  // Jetzt BIND: sein controlGain von 2 wird mit dem Faktor multipliziert.
  const vorher = combat.control;
  combat.stepPassive(zeit);
  combat.cast(bind, zeit);
  return { punkte: combat.control - vorher, wuerfe };
}

function messen(): Lauf {
  const ohne = punkteNachBind([]);
  const eins = punkteNachBind([ERSTER]);
  const beide = punkteNachBind([ERSTER, ZWEITER]);
  return {
    punkteOhne: ohne.punkte,
    punkteMitEinem: eins.punkte,
    punkteMitBeiden: beide.punkte,
    wuerfe: beide.wuerfe,
  };
}

describe('Gleiche Arten ersetzen einander — auch über einen Zauber gewirkt', () => {
  it('die Voraussetzung: beide Würfe kommen durch und der erste wirkt', () => {
    // **Die Zusicherung vor der Messung.** Wird einer der beiden abgewiesen
    // (Wartezeit, Abklingzeit), misst der Vergleich darunter einen Lauf mit
    // nur einer Verstärkung — und wäre grün, ohne etwas zu zeigen.
    const r = messen();
    expect(r.wuerfe, 'einer der beiden Verstärker wurde abgewiesen').toBe(2);
    expect(r.punkteOhne, 'BIND bringt gar keine Kontroll-Punkte').toBe(2);
    expect(r.punkteMitEinem, `${ERSTER} verstärkt den Punktegewinn nicht`).toBe(4);
  });

  it('der zweite ersetzt den ersten, statt sich mit ihm zu multiplizieren', () => {
    const r = messen();
    // Beide tragen Faktor 2. Multiplikativ wären es 4 und damit 8 Punkte —
    // ersetzend bleibt es bei 2 und 4. Die Zahl trennt die Fälle; eine
    // Richtung (`>= 4`) täte es nicht.
    expect(r.punkteMitBeiden, 'die beiden Verstärkungen stapeln statt zu ersetzen').toBe(4);
    expect(r.punkteMitBeiden).toBe(r.punkteMitEinem);
  });

  it('und das ist keine leere Aussage: ohne Verstärker sind es weniger', () => {
    // **Die Gegenprobe zur Zahl.** Stünde der Gewinn auch ohne Verstärker auf
    // 4, sagte der Vergleich oben nichts — dann wirkte gar nichts.
    const r = messen();
    expect(r.punkteMitEinem).toBeGreaterThan(r.punkteOhne);
    expect(r.punkteMitEinem / r.punkteOhne, 'der Faktor ist nicht 2').toBe(2);
  });
});
