/**
 * Der Fund behält Stufe und Qualität (Issue #480).
 *
 * Worker-3 hat gemessen, dass ein mythischer Fund aus Instanz 14 exakt so stark
 * war wie der erste aus Instanz 1 — beide 407/406/406/67/67/67. `rollLoot`
 * würfelte beide Zahlen korrekt, `recordRun` übernahm nur die Kennung, und
 * jeder Fund fiel damit auf die Werte seiner Vorlage zurück.
 *
 * **Warum hier mit `itemLevelOf` gerechnet und nicht mit 1 und 85 getippt wird:**
 * Die zwei Zahlen sind das Ergebnis von `(i-1)*6 + d*3 + e + 1`. Stünden sie als
 * Festwerte im Test, hielte er weiter, nachdem die Formel geändert wurde — und
 * beschriebe dann eine Verteilung, die es nicht mehr gibt.
 */

import { describe, expect, it } from 'vitest';
import { instanceLevelOf, instanceQualityOf } from '../../src/gear/instance.js';
import { isOwnedItem, cleanSave } from '../../src/analysis/entry-shape.js';
import { itemById } from '../../src/gear/catalog.js';
import { itemLevelOf } from '../../src/loot/rates.js';
import { basePermille } from '../../src/gear/sources.js';
import { SAVE_SCHEMA } from '../../src/analysis/save-format.js';

const KENNUNG = 'test-relic';
/** Die Stufe der VORLAGE — der Wert, auf den vorher jeder Fund zurückfiel. */
const VORLAGE = itemById(KENNUNG)?.itemLevel ?? 0;

/** Instanz 1, normal, kein Endlos. */
const NIEDRIG = itemLevelOf(1, 0, 0);
/** Instanz 14, mythisch, kein Endlos. */
const HOCH = itemLevelOf(14, 2, 0);

describe('AC1 — zwei Funde derselben Kennung sind verschieden stark', () => {
  it('trennt die Grundwerte nach der Stufe des Fundes', () => {
    const schwach = { itemId: KENNUNG, itemLevel: NIEDRIG, quality: 0 };
    const stark = { itemId: KENNUNG, itemLevel: HOCH, quality: 0 };

    const wertSchwach = basePermille(instanceLevelOf(schwach), 100);
    const wertStark = basePermille(instanceLevelOf(stark), 100);

    // Die Zahlen, nicht "höher": Stufe 1 gibt 100, Stufe 85 gibt 6024.
    expect(wertSchwach).toBe(100);
    expect(wertStark).toBe(6024);
    expect(wertStark).not.toBe(wertSchwach);

    // ZUSICHERUNG: Beide Stufen weichen von der Vorlage ab. Wäre eine von
    // beiden zufällig 40, prüfte dieser Fall die Auflösung nicht mit — er
    // bekäme dieselbe Zahl auf zwei Wegen und sähe grün aus.
    expect(NIEDRIG).not.toBe(VORLAGE);
    expect(HOCH).not.toBe(VORLAGE);
  });

  it('das Verhältnis folgt den ~5 % je Stufe aus 6.5', () => {
    // 84 Stufen Abstand: 1,05^84 = 60,2. Gemessen, nicht behauptet — die
    // Rundung auf ganze Promille lässt die Einzelschritte zwischen 4,76 % und
    // 5,44 % schwanken, das Ganze aber trifft.
    const faktor = basePermille(HOCH, 100) / basePermille(NIEDRIG, 100);
    expect(HOCH - NIEDRIG).toBe(84);
    expect(faktor).toBeCloseTo(1.05 ** 84, 0);
  });
});

describe('AC2 — Gegenprobe: es ist die STUFE, nicht etwas Mitgewandertes', () => {
  it('bei gleicher Stufe und verschiedener Qualität bleibt der Grundwert gleich', () => {
    // Die Grundwerte hängen NUR an der Stufe (13.5). Änderte die Qualität sie
    // mit, wäre der Unterschied oben nicht der Stufe zuzuschreiben — beide
    // Achsen wandern beim Fund gemeinsam.
    const arm = { itemId: KENNUNG, itemLevel: HOCH, quality: 0 };
    const reich = { itemId: KENNUNG, itemLevel: HOCH, quality: 3 };

    expect(basePermille(instanceLevelOf(arm), 100)).toBe(basePermille(instanceLevelOf(reich), 100));
    // ZUSICHERUNG: Die Qualitäten sind wirklich verschieden — sonst prüft der
    // Fall zwei identische Gegenstände und ist immer grün.
    expect(instanceQualityOf(arm)).not.toBe(instanceQualityOf(reich));
  });

  it('bei gleicher Kennung und nur verschiedener Stufe trennt es sich doch', () => {
    // Der Gegenpol zum Fall darüber: Alles gleich AUSSER der Stufe. Bleibt der
    // Unterschied, kann er an nichts anderem liegen.
    const a = { itemId: KENNUNG, itemLevel: NIEDRIG, quality: 2 };
    const b = { itemId: KENNUNG, itemLevel: HOCH, quality: 2 };

    expect(a.itemId).toBe(b.itemId);
    expect(instanceQualityOf(a)).toBe(instanceQualityOf(b));
    expect(basePermille(instanceLevelOf(a), 100)).not.toBe(basePermille(instanceLevelOf(b), 100));
  });
});

describe('AC3 — ein alter Spielstand lädt und verliert nichts', () => {
  const ALT = { itemId: KENNUNG, upgradeSteps: 3, equippedSlot: 0 };

  it('ein Eintrag ohne die neuen Felder besteht die Formprüfung', () => {
    expect(isOwnedItem(ALT)).toBe(true);
    // ZUSICHERUNG: Der Eintrag hat die Felder wirklich nicht. Ein Test gegen
    // ein Objekt, das sie stillschweigend trägt, prüft den alten Fall nicht.
    expect('itemLevel' in ALT).toBe(false);
    expect('quality' in ALT).toBe(false);
  });

  it('cleanSave filtert ihn NICHT weg — der Spieler behält seine Ausrüstung', () => {
    // Das ist der eigentliche Schaden, den die optionalen Felder abwenden:
    // `cleanSave` arbeitet mit `wert.filter(guard)`. Ein strenges `isOwnedItem`
    // hätte hier still die gesamte Ausrüstung entfernt — kein Fehler, keine
    // Meldung, nur ein leerer Beutel.
    const ergebnis = cleanSave({ schema: 1, items: [ALT] });
    expect(ergebnis.rejected).toBeNull();
    expect(ergebnis.dropped['items'] ?? 0).toBe(0);
    expect((ergebnis.data as { items: unknown[] } | null)?.items).toHaveLength(1);
  });

  it('er rechnet danach GENAU wie vorher — die Vorlage ist der Rückfall', () => {
    // Der Standardwert ist bewusst die Vorlage und nicht 1: Bei 1 verlöre ein
    // alter Stand den Faktor 6,7 seiner Grundwerte, ohne dass der Spieler
    // etwas getan hat. Ein Fassungswechsel darf Fortschritt hinzufügen, nicht
    // wegnehmen.
    expect(instanceLevelOf(ALT)).toBe(VORLAGE);
    expect(instanceQualityOf(ALT)).toBe(itemById(KENNUNG)?.quality);
    // ZUSICHERUNG: Die Vorlage ist nicht zufällig 1 — sonst wäre der Fall
    // gegen die falsche Wahl blind.
    expect(VORLAGE).toBeGreaterThan(1);
  });

  it('ein vorhandenes Feld muss trotzdem stimmen', () => {
    // Fehlen ist erlaubt, Unsinn nicht. Eine Zeichenkette käme aus einem von
    // Hand bearbeiteten Stand und verkettete sich in der Rechnung, statt sich
    // zu addieren.
    expect(isOwnedItem({ ...ALT, itemLevel: '85' })).toBe(false);
    expect(isOwnedItem({ ...ALT, quality: null })).toBe(false);
    expect(isOwnedItem({ ...ALT, itemLevel: HOCH, quality: 2 })).toBe(true);
  });

  it('die Fassung ist gestiegen, damit ein neuer Stand sich zu erkennen gibt', () => {
    // Ohne den Anstieg läse eine ältere Fassung des Spiels einen Stand der
    // Fassung 2 als ihren eigenen und verlöre die Instanz-Zahlen still.
    expect(SAVE_SCHEMA).toBe(2);
  });
});
