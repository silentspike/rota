/**
 * Acht kumulative Grössen kommen im `ReplayResult` an (Issue #630).
 *
 * # Der Befund, aus dem dieses Issue entstand
 *
 * `leitungs-waechter.test.ts` (#612) prüft, ob ein Getter **einen Weg** hat:
 * eine Zuweisung `feld: combat.x` **oder** Deckung über `combatChecksum`. Bei
 * `bossHealed` genügte die zweite Bedingung — und als die Zuweisung in #609
 * versuchsweise entfernt wurde, meldete im ganzen Bestand **kein einziger
 * Fall** den Verlust.
 *
 * Ausgezählt an `origin/main`, in vier Stufen:
 *
 * ```text
 *   42  Grössen in combatChecksum
 *   38  davon ohne eigene Zuweisung   <- sieht nach einem Befund aus
 *   10  davon kumulativ               <- die Zahl, über die sich reden lässt
 *    2  davon heute benutzt
 * ```
 *
 * **Die 38 wären ein Befund gewesen, der Rauschen erzeugt.** Der grösste Teil
 * sind Momentanwerte — `casting`, `gcd`, `resource`, `playerHp` —, die im
 * Ergebnis nichts zu suchen haben. Für sie ist „keine Leitung" richtig.
 *
 * # Warum auch die acht unbenutzten eine Leitung bekommen
 *
 * Von den zehn kumulativen hatten acht ausserhalb von `combat.ts` null
 * Treffer. Das ist **kein** Grund, sie wegzulassen: Bei #605 standen elf
 * Kontroll-Grössen auf 0,00 %, **weil** ihre Leitung fehlte. Eine Grösse ohne
 * Leitung sieht immer unbenutzt aus.
 *
 * # Zwei der zehn fehlen hier, und das ist Absicht
 *
 * `lostDotTicks(index)` und `dotTickCount(index)` sind **Methoden mit
 * Parameter**, keine Getter — sie liefern einen Wert je Zauber. Als
 * Einzelzahl passen sie nicht ins `ReplayResult`; sie bräuchten ein Feld je
 * Zauber, so wie `casts` eines ist. Das ist eine eigene Entscheidung und
 * gehört nicht in dieses Issue. **Gemeldet statt stillschweigend
 * weggelassen** — eine Zählung nach zwei Klassen findet keine dritte.
 */
import { describe, expect, it } from 'vitest';
import { bossById } from '../../src/core/data.js';
import { replayRecording } from '../../src/devtools/recording.js';

/** Ein Lauf mit Rotation gegen den Prüfboss mit Zeitleiste und Fenster. */
function lauf(bossId: string, ticks: number): ReturnType<typeof replayRecording> {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`Der Boss ${bossId} fehlt.`);
  const presses: { tick: number; slot: number }[] = [];
  for (let tick = 0; tick < ticks; tick += 90)
    presses.push({ tick, slot: tick % 180 === 0 ? 0 : 1 });
  return replayRecording({ name: 'kumulativ', seed: 0, ticks, presses }, { boss });
}

const FELDER = [
  'playerDamage',
  'damageLostToRace',
  'afflictionsCleared',
  'resourceOverflow',
  'conversionPercent',
  'phaseHoldPercent',
  'buffWindowSum',
  'synergyWindowSum',
] as const;

describe('die acht Grössen haben einen Weg ins Ergebnis', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // Läse der Lauf leer — falscher Boss, kaputte Aufzeichnung —, wären alle
    // Fälle darunter Aussagen über ein leeres Objekt.
    const r = lauf('first-crucible', 600);
    expect(Object.keys(r).length, 'kein ReplayResult-Feld gefunden').toBeGreaterThan(15);
    expect(r.damage, 'der Lauf macht gar keinen Schaden').toBeGreaterThan(0);
  });

  it('jedes Feld existiert und ist eine Zahl', () => {
    const r = lauf('first-crucible', 600) as unknown as Record<string, unknown>;
    for (const feld of FELDER) {
      expect(r[feld], `${feld} fehlt im ReplayResult`).toBeTypeOf('number');
    }
  });
});

describe('die Werte kommen an — je Grösse eine harte Zahl', () => {
  // **Drei der acht lassen sich mit einer Aufzeichnung bewegen**, und nur für
  // sie steht hier eine Zahl. Gemessen an vier Bossen:
  //
  //     playerDamage       1200      damageLostToRace   300 (nur test-racer)
  //     resourceOverflow      7      die übrigen fünf     0
  //
  // Die fünf anderen brauchen eine Rotation, die den jeweiligen Baustein
  // wirklich bedient — reinigen, umwandeln, eine Phasenschwelle halten, ein
  // Buff- oder Synergiefenster füllen. Mit dem starren Muster hier bleiben sie
  // auf 0, und eine Zusicherung darauf verglichen zwei Nullen.
  //
  // **Das ist gemeldet, nicht kaschiert.** Ihre Leitung ist gelegt und ihr
  // Typ geprüft; was fehlt, ist der Lauf, der sie bewegt. Wer ihn baut, macht
  // aus dem Strukturfall darunter einen mit harter Zahl.

  it('playerDamage — gemessen, und getrennt vom Fortschritt', () => {
    // Die Grösse, die es erlaubt, bei einem `runaway`-Boss zwischen
    // „getroffen" und „zurückgedreht" zu unterscheiden. `damage` zählt dort
    // beide Richtungen zusammen.
    expect(lauf('test-crucible-trial', 2000).playerDamage).toBe(1200);
  });

  it('damageLostToRace — was ein verpasstes Fenster gekostet hat', () => {
    // Nur `test-racer` trägt ein Wettrennen, das dieser Lauf verpasst.
    expect(lauf('test-racer', 2000).damageLostToRace).toBe(300);

    // Die Gegenprobe: Ein Boss ohne Wettrennen verliert nichts. Ohne sie
    // bliebe offen, ob die Leitung eine Konstante durchreicht.
    expect(lauf('test-crucible-trial', 2000).damageLostToRace).toBe(0);
  });

  it('resourceOverflow — der Vorrat, der über den Rand lief', () => {
    expect(lauf('test-crucible-trial', 2000).resourceOverflow).toBe(7);
  });

  it('DIE FÜNF ÜBRIGEN stehen auf 0 — und das ist hier keine Messung', () => {
    // **Der Fall, der seine eigene Grenze festhält.** Er sichert nicht zu,
    // dass die Grössen 0 SIND — er hält fest, dass dieser Lauf sie nicht
    // bewegt, damit niemand die Nullen für einen Befund hält.
    //
    // Er wird rot, sobald jemand eine Rotation baut, die sie doch bewegt. Das
    // ist die richtige Richtung: Dann gehört an dieser Stelle eine harte Zahl
    // hin, und der Fall verlangt sie.
    const r = lauf('test-crucible-trial', 2000) as unknown as Record<string, number>;
    for (const feld of [
      'afflictionsCleared',
      'conversionPercent',
      'phaseHoldPercent',
      'buffWindowSum',
      'synergyWindowSum',
    ]) {
      expect(r[feld], `${feld} bewegt sich jetzt — trag eine harte Zahl ein`).toBe(0);
    }
  });
});
