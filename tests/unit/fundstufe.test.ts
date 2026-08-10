/**
 * Ein Stück ist so stark wie SEINE Stufe (Issue #480).
 *
 * Zwei Achsen wurden erhoben und nie gelesen: die gewürfelte Fundstufe
 * (`rollLoot` erhob sie, die Verbuchung liess sie fallen) und die Aufwertung
 * (`upgradeSteps` wurde gespeichert und als „+14" angezeigt, ging aber in keine
 * Wertrechnung ein). Gemessen an `wornFrom` waren zwei Stücke derselben
 * Kennung darum exakt gleich stark — 407/406/406/67/67/67, egal woher sie kamen.
 *
 * **Warum hier `itemLevelOf` gerechnet und nicht 1 und 85 getippt wird:** Die
 * zwei Zahlen sind das Ergebnis von `(i-1)*6 + d*3 + e + 1`. Stünden sie als
 * Festwerte im Test, hielte er weiter, nachdem die Formel geändert wurde — und
 * beschriebe dann eine Verteilung, die es nicht mehr gibt.
 */

import { describe, expect, it } from 'vitest';
import { effectiveLevelOf, instanceLevelOf, instanceQualityOf } from '../../src/gear/instance.js';
import { isOwnedItem, cleanSave } from '../../src/analysis/entry-shape.js';
import { itemById, wornFrom, type OwnedItemLike } from '../../src/gear/catalog.js';
import { MAX_ITEM_LEVEL } from '../../src/gear/budget.js';
import { itemLevelOf } from '../../src/loot/rates.js';
import { SAVE_SCHEMA } from '../../src/analysis/save-format.js';

const KENNUNG = 'test-relic';
/** Stufe der VORLAGE — der Wert, auf den vorher JEDES Stück zurückfiel. */
const VORLAGE = itemById(KENNUNG)?.itemLevel ?? 0;
const VORLAGE_GUETE = itemById(KENNUNG)?.quality ?? 0;

/** Instanz 1, normal, kein Endlos. */
const NIEDRIG = itemLevelOf(1, 0, 0);
/** Instanz 14, mythisch, kein Endlos. */
const HOCH = itemLevelOf(14, 2, 0);

/** Ein getragenes Stück — auf einem Platz, sonst trägt es nichts bei. */
function getragen(felder: Partial<OwnedItemLike> = {}): OwnedItemLike {
  return { itemId: KENNUNG, upgradeSteps: 0, equippedSlot: 0, ...felder };
}

describe('AC1 — zwei Stücke derselben Kennung sind verschieden stark', () => {
  it('die Fundstufe trennt sie, an wornFrom gemessen', () => {
    const schwach = wornFrom([getragen({ itemLevel: NIEDRIG })]);
    const stark = wornFrom([getragen({ itemLevel: HOCH })]);

    // Die Zahlen, nicht „höher": Stufe 1 gegen Stufe 85.
    expect(schwach.essence).toBe(10);
    expect(stark.essence).toBe(602);
    expect(schwach.catalysis).toBe(64);
    expect(stark.catalysis).toBe(3622);

    // ZUSICHERUNG: Beide Stufen weichen von der Vorlage ab. Wäre eine zufällig
    // 40, prüfte der Fall die Auflösung nicht mit — er bekäme dieselbe Zahl auf
    // zwei Wegen und sähe grün aus.
    expect(NIEDRIG).not.toBe(VORLAGE);
    expect(HOCH).not.toBe(VORLAGE);
  });

  it('die AUFWERTUNG trennt sie ebenso — 0 gegen 14 Schritte', () => {
    // Der Fall, an dem der Orchestrator den Auftrag festgemacht hat.
    const roh = wornFrom([getragen({ itemLevel: NIEDRIG, upgradeSteps: 0 })]);
    const aufgewertet = wornFrom([getragen({ itemLevel: NIEDRIG, upgradeSteps: 14 })]);

    // Vierzehn Schritte auf Stufe 1 sind Stufe 15: 1,05^14 = 1,98, und beide
    // Grössen folgen dem — die Grundwerte von 10 auf 19, die Affixe von 64
    // auf 122.
    expect(roh.essence).toBe(10);
    expect(aufgewertet.essence).toBe(19);
    expect(roh.catalysis).toBe(64);
    expect(aufgewertet.catalysis).toBe(122);
  });

  it('Aufwertung ist dasselbe wie eine höhere Fundstufe — sie HEBT sie', () => {
    // 13.4: „Material und Punkte heben die GEGENSTANDSSTUFE an." Also nicht
    // eine zweite Grösse daneben, sondern dieselbe Achse. Ein Stück auf Stufe 1
    // mit 14 Schritten muss so stark sein wie eines, das auf 15 gefunden wurde.
    const gehoben = wornFrom([getragen({ itemLevel: NIEDRIG, upgradeSteps: 14 })]);
    const gefunden = wornFrom([getragen({ itemLevel: NIEDRIG + 14, upgradeSteps: 0 })]);
    expect(gehoben).toEqual(gefunden);
  });
});

describe('AC2 — Gegenprobe: es ist die STUFE, nicht etwas Mitgewandertes', () => {
  it('bei gleicher Stufe und verschiedener Qualität bleiben die Grundwerte gleich', () => {
    // Die drei Grundwerte hängen NUR an der Stufe (13.5). Änderte die Qualität
    // sie mit, wäre der Unterschied oben nicht der Stufe zuzuschreiben — beide
    // Achsen wandern beim Fund gemeinsam.
    const arm = wornFrom([getragen({ itemLevel: HOCH, quality: 0 })]);
    const reich = wornFrom([getragen({ itemLevel: HOCH, quality: 3 })]);

    expect(arm.essence).toBe(reich.essence);
    expect(arm.vessel).toBe(reich.vessel);
    expect(arm.ward).toBe(reich.ward);

    // ZUSICHERUNG: Die Qualität wirkt sehr wohl — nur eben auf die Affixe.
    // Ohne diese Zeile wäre der Fall auch grün, wenn `quality` gar nicht
    // ankommt, und belegte dann das Gegenteil dessen, was er behauptet.
    expect(arm.catalysis).not.toBe(reich.catalysis);
  });

  it('bei gleicher Qualität und nur verschiedener Stufe trennt es sich doch', () => {
    // Der Gegenpol: Alles gleich AUSSER der Stufe. Bleibt der Unterschied,
    // kann er an nichts anderem liegen.
    const a = getragen({ itemLevel: NIEDRIG, quality: 2 });
    const b = getragen({ itemLevel: HOCH, quality: 2 });

    expect(a.itemId).toBe(b.itemId);
    expect(instanceQualityOf(a, VORLAGE_GUETE)).toBe(instanceQualityOf(b, VORLAGE_GUETE));
    expect(wornFrom([a]).essence).not.toBe(wornFrom([b]).essence);
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

  it('die fehlende Fundstufe fällt auf die Vorlage zurück, nicht auf 1', () => {
    // Bei 1 verlöre ein alter Stand den Faktor 6,7 seiner Grundwerte, ohne dass
    // der Spieler etwas getan hat. Ein Fassungswechsel darf Fortschritt
    // hinzufügen, nicht wegnehmen.
    expect(instanceLevelOf(ALT, VORLAGE)).toBe(VORLAGE);
    expect(instanceQualityOf(ALT, VORLAGE_GUETE)).toBe(VORLAGE_GUETE);
    // ZUSICHERUNG: Die Vorlage ist nicht zufällig 1 — sonst wäre der Fall
    // gegen die falsche Wahl blind.
    expect(VORLAGE).toBeGreaterThan(1);
  });

  it('seine Aufwertung wirkt trotzdem — sie war ja schon gespeichert', () => {
    // Der alte Stand trägt `upgradeSteps: 3`. Nach dem Wechsel wirken sie zum
    // ersten Mal. Das ist ein Zugewinn, kein Verlust, und deshalb erlaubt.
    const ohne = wornFrom([{ ...ALT, upgradeSteps: 0 }]);
    const mit = wornFrom([ALT]);
    expect(mit.essence).toBeGreaterThan(ohne.essence);
    expect(effectiveLevelOf(ALT, VORLAGE)).toBe(VORLAGE + 3);
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
    expect(SAVE_SCHEMA).toBe(2);
  });
});

describe('Die Klemme — Aufwertung macht ein Stück nicht wertlos', () => {
  it('über dem Höchstwert bleibt es beim Höchstwert', () => {
    // `basePermille` und `budgetOf` weisen jede Stufe über MAX_ITEM_LEVEL mit
    // **0** ab, nicht mit dem Höchstwert. Ohne Klemme verlöre ein Fund auf 85
    // bei der sechzehnten Aufwertung sämtliche Werte — still, weil 0 eine
    // gültige Zahl ist.
    const knapp = getragen({ itemLevel: HOCH, upgradeSteps: MAX_ITEM_LEVEL - HOCH });
    const drueber = getragen({ itemLevel: HOCH, upgradeSteps: MAX_ITEM_LEVEL - HOCH + 40 });

    expect(effectiveLevelOf(knapp, VORLAGE)).toBe(MAX_ITEM_LEVEL);
    expect(effectiveLevelOf(drueber, VORLAGE)).toBe(MAX_ITEM_LEVEL);
    expect(wornFrom([drueber])).toEqual(wornFrom([knapp]));

    // ZUSICHERUNG: Es steht wirklich etwas auf dem Spiel — der geklemmte Wert
    // ist nicht null. Ohne diese Zeile wäre der Fall auch dann grün, wenn
    // beide Seiten zu 0 zusammenfielen, also im Schadensfall selbst.
    expect(wornFrom([drueber]).essence).toBeGreaterThan(0);
  });

  it('ohne Aufwertung ändert sich gegenüber vorher nichts', () => {
    // Der Rückfall-Fall in seiner reinsten Form: keine Fundstufe, keine
    // Aufwertung. So rechnete das Spiel bisher, und so muss es weiter rechnen.
    const heute = wornFrom([{ itemId: KENNUNG, upgradeSteps: 0, equippedSlot: 0 }]);
    expect(heute.catalysis).toBe(407);
    expect(heute.purity).toBe(406);
    expect(heute.essence).toBe(67);
  });
});
