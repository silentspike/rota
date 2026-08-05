/**
 * Talentpunkte (Issue #379, GAME-DESIGN 12.1 und Zeile 111).
 *
 * WAS BIS HIERHER FEHLTE
 * ----------------------
 * `talentPoints` stand seit #83 im Spielstand-Format, und `levels.json` kannte
 * keine Vergabe — dieselben drei Treffer wie bei `bestRuns` vor #377: Feld da,
 * Vergabe nicht. GAME-DESIGN 12.1 nennt vier Dinge, die eine Stufe gibt;
 * gemessen an den Daten gab sie drei.
 *
 * VERDIENT WIRD GERECHNET, AUSGEGEBEN WIRD GESPEICHERT
 * ----------------------------------------------------
 * Das Feld im Spielstand heisst laut seinem eigenen Kommentar „Ausgegebene
 * Talentpunkte je Zweig — die Wahl, nicht ihre Wirkung". Ein Guthaben passt
 * dort nicht hinein, und es soll auch nicht: GAME-DESIGN 18.6 verlangt, dass
 * der Spielstand nur hält, was besessen und erreicht wurde. Wie viele Punkte
 * eine Stufe eingebracht hat, folgt aus ihr — wie die Grundwerte, die
 * Leisten-Plätze und die freigeschalteten Zauber.
 *
 * AC5 IST DER KERN, UND ER HAT EINE FALLE
 * ---------------------------------------
 * `unlocks.ts` warnt davor an seiner eigenen Nachbarfunktion:
 *
 *   „Ein Prüffall, der die Belegung aus der Anzahl selbst herstellt, vergleicht
 *    seine eigene Erwartung mit sich — er ist grün, egal was die Umsetzung tut."
 *
 * Deshalb stehen Verteilung und Zielgrösse als ZWEI Einträge in den Daten, und
 * der Wächter hält sie gegeneinander.
 */
import { describe, expect, it } from 'vitest';
import { LEVELS } from '../../src/core/data.js';
import { barSlotsAtLevel, talentPointsAtLevel } from '../../src/progression/unlocks.js';
import { levelUpDisplayFor } from '../../src/analysis/level-up-display.js';
import { emptySave, RunPersistence } from '../../src/analysis/run-persistence.js';
import { checksumOf } from '../../src/analysis/save-store.js';

const FASSUNG = '0.1.0';
const JETZT = 1_800_000_000_000;

describe('AC1 — ein Aufstieg vergibt Punkte aus einer benannten Grösse', () => {
  it('die Vergabe steht in den Daten, nicht im Code', () => {
    // Der Rumpf von `talentPointsAtLevel` liest `LEVELS.talentPointLevels`.
    // Wäre die Verteilung im Code, änderte eine Datei-Änderung nichts — hier
    // ändert sie alles.
    expect(LEVELS.talentPointLevels.length).toBeGreaterThan(0);
    for (const stufe of LEVELS.talentPointLevels) {
      expect(talentPointsAtLevel(stufe)).toBeGreaterThan(talentPointsAtLevel(stufe - 1));
    }
  });

  it('auf einer Stufe ohne Eintrag kommt nichts dazu', () => {
    // GEGENPROBE zur Zeile darüber: Gäbe es auf jeder Stufe einen Punkt, wäre
    // sie ebenfalls grün — und die Verteilung bedeutungslos.
    const ohne = [];
    for (let stufe = 2; stufe <= LEVELS.maxLevel; stufe++) {
      if (!LEVELS.talentPointLevels.includes(stufe)) ohne.push(stufe);
    }
    expect(ohne.length).toBeGreaterThan(0);
    for (const stufe of ohne.slice(0, 20)) {
      expect(talentPointsAtLevel(stufe)).toBe(talentPointsAtLevel(stufe - 1));
    }
  });

  it('steigt nie und fällt nie zurück', () => {
    let vorher = 0;
    for (let stufe = 1; stufe <= LEVELS.maxLevel; stufe++) {
      const jetzt = talentPointsAtLevel(stufe);
      expect(jetzt).toBeGreaterThanOrEqual(vorher);
      expect(jetzt - vorher).toBeLessThanOrEqual(1);
      vorher = jetzt;
    }
  });

  it('Stufe 1 bringt noch keinen — der Aufstieg gibt ihn, nicht der Anfang', () => {
    expect(talentPointsAtLevel(1)).toBe(0);
  });
});

describe('AC5 — der Wächter: Verteilung gegen Zielgrösse', () => {
  it('die Summe über alle Stufen trifft die benannte Zielgrösse', () => {
    // **Zwei Quellen, nicht eine.** `talentPointTarget` steht unabhängig in den
    // Daten; wer die Liste ändert, ohne das Ziel anzufassen, wird hier bemerkt.
    expect(talentPointsAtLevel(LEVELS.maxLevel)).toBe(LEVELS.talentPointTarget);
  });

  it('GEGENPROBE: eine veränderte Verteilung fällt', () => {
    // Die Fehlerart nachgebaut, nicht behauptet: eine Liste mit einem Eintrag
    // weniger. Der Prüffall darüber vergliche dann 49 gegen 50.
    const gekuerzt = LEVELS.talentPointLevels.slice(0, -1);
    const zaehle = (stufe: number): number => gekuerzt.filter((ab) => stufe >= ab).length;
    expect(zaehle(LEVELS.maxLevel)).not.toBe(LEVELS.talentPointTarget);
  });

  it('GEGENPROBE ZUR GEGENPROBE: die echte Liste besteht dieselbe Rechnung', () => {
    // Sonst bliebe offen, ob die Kürzung den Unterschied macht oder die
    // Rechnung selbst immer danebenliegt.
    const zaehle = (stufe: number): number =>
      LEVELS.talentPointLevels.filter((ab) => stufe >= ab).length;
    expect(zaehle(LEVELS.maxLevel)).toBe(LEVELS.talentPointTarget);
  });

  it('die Zielgrösse ist die aus dem Design', () => {
    // GAME-DESIGN Zeile 111: „Talentpunkte — ~50 (etwa jede zweite Stufe
    // einer)". Die Zahl ist entschieden, die Verteilung vorläufig.
    expect(LEVELS.talentPointTarget).toBe(50);
  });

  it('AC2 — und sie kommen über die ganzen 100 Stufen zusammen, nicht früher', () => {
    // Eine Verteilung, die bei Stufe 60 fertig ist, gäbe dem Endspiel nichts
    // mehr. Gemessen statt behauptet: Der letzte Punkt fällt auf der
    // Höchststufe.
    expect(LEVELS.talentPointLevels.at(-1)).toBe(LEVELS.maxLevel);
    expect(talentPointsAtLevel(LEVELS.maxLevel - 1)).toBeLessThan(LEVELS.talentPointTarget);
  });
});

describe('AC3 — die Stufe übersteht Speichern und Laden, die Punkte folgen', () => {
  it('nach dem Neuladen ergibt dieselbe Stufe dieselbe Punktzahl', () => {
    // **Kein zweites Spielstand-Feld** (GAME-DESIGN 18.6): Ein gespeichertes
    // Guthaben ginge nicht kaputt, es würde nur alt — eine geänderte
    // Verteilung erreichte bestehende Spielstände nie mehr.
    const daten = new Map<string, string>();
    const store = {
      getItem: (k: string) => daten.get(k) ?? null,
      setItem: (k: string, v: string) => {
        daten.set(k, v);
      },
      removeItem: (k: string) => {
        daten.delete(k);
      },
    };

    const erste = new RunPersistence(FASSUNG, () => JETZT, store);
    erste.persistLevel(42);
    const erwartet = talentPointsAtLevel(42);
    expect(erwartet).toBeGreaterThan(0);

    const zweite = new RunPersistence(FASSUNG, () => JETZT, store);
    expect(zweite.state.level).toBe(42);
    expect(talentPointsAtLevel(zweite.state.level)).toBe(erwartet);
  });

  it('das Spielstand-Feld bleibt für die AUSGABEN, und die sind leer', () => {
    // Was der Spieler entschieden hat, steht im Stand; was daraus folgt, nicht.
    // Der Talentbaum, der die Ausgaben füllt, ist #249.
    const leer = emptySave(FASSUNG, JETZT);
    expect(leer.talentPoints).toEqual({});
  });

  it('AC4 (erfüllt durch #378) — ein Stand mit `null` bekommt sie ergänzt', () => {
    // Nachgewiesen, nicht neu gebaut: Die Zeile steht seit #378 in `ergaenzt()`
    // (`run-persistence.ts`), und `save-shape.test.ts` bewacht sie für alle
    // neun zusammengesetzten Felder. Hier nur der Beleg für dieses eine.
    const daten = new Map<string, string>();
    const stand = { ...(emptySave(FASSUNG, JETZT) as unknown as Record<string, unknown>) };
    stand['talentPoints'] = null;
    const payload = JSON.stringify(stand);
    daten.set('rota.save.0', JSON.stringify({ checksum: checksumOf(payload), payload }));
    daten.set('rota.save.index', '0');

    const geladen = new RunPersistence(FASSUNG, () => JETZT, {
      getItem: (k) => daten.get(k) ?? null,
      setItem: (k, v) => {
        daten.set(k, v);
      },
      removeItem: (k) => {
        daten.delete(k);
      },
    });
    const roh = geladen as unknown as { data: Record<string, unknown> };
    expect(roh.data['talentPoints']).toEqual({});
  });
});

describe('Der Aufstieg zeigt sie an', () => {
  it('eine Stufe mit Punkt nennt ihn, eine ohne nicht', () => {
    // Ohne diese Verbindung wäre es „gerechnet und nirgends angekommen" — die
    // teuerste Klasse in diesem Projekt.
    const mit = LEVELS.talentPointLevels[3] ?? 8;
    const ohne = mit + 1;
    expect(LEVELS.talentPointLevels).not.toContain(ohne);

    expect(levelUpDisplayFor(true, mit, [])?.newTalentPoints).toBe(1);
    expect(levelUpDisplayFor(true, ohne, [])?.newTalentPoints).toBe(0);
  });

  it('kein Aufstieg, keine Anzeige', () => {
    expect(levelUpDisplayFor(false, 10, [])).toBeNull();
  });

  it('die Punkte stehen neben den anderen drei Dingen aus 12.1', () => {
    // Auf einer Stufe, die alles gibt, muss alles dastehen. `barSlotLevels`
    // und `talentPointLevels` überschneiden sich bei 30.
    const anzeige = levelUpDisplayFor(true, 30, []);
    expect(anzeige?.statPercent).toBe(LEVELS.growthPerLevelPercent);
    expect(anzeige?.newBarSlots).toBe(barSlotsAtLevel(30) - barSlotsAtLevel(29));
    expect(anzeige?.newTalentPoints).toBe(1);
  });
});
