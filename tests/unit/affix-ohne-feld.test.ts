/**
 * Ein Affix-Name, der keinen Wert füllt, muss laut scheitern (Issue #421b).
 *
 * # Der stille Verlust, den dieser Fall sperrt
 *
 * `POOL_NAMES` (`src/gear/pools.ts`) führt die **Töpfe** aus GAME-DESIGN
 * 13.1b, `WornValues` (`src/gear/worn.ts`) die **Werte**, die der Kampf liest.
 * Das ist fast dasselbe, aber nicht ganz: `active` ist ein Topf ohne Wert,
 * weil aktive Verstärkungen im Kampf entstehen — Overdrive, Synergien,
 * Zersetzungs-Stapel — und nicht auf einem Gegenstand stehen.
 *
 * Bis hierher benutzte `wornFrom` die Art **direkt** als Schlüssel. Gemessen
 * am 2026-08-10 an einem Epic der Stufe 40 mit
 * `["gearEffects","essence","active"]`:
 *
 * ```text
 * {… "essence":473, "gearEffects":407, "active":406}
 *                                       ^^^^^^^^^^^^ kennt der Kampf nicht
 * ```
 *
 * **406 von 1219 Promille verfielen** — ohne Ausnahme, ohne roten Fall, ohne
 * Log. Der Gegenstand sah vollwertig aus und war ein Drittel schwächer.
 *
 * # Warum das unabhängig von `active` gilt
 *
 * Der Auftraggeber hat diese Sperre der Frage **vorgeordnet**, ob `active`
 * überhaupt aus Ausrüstung kommen soll (entschieden am 2026-08-10). Der Grund:
 * Sie trifft **jeden** künftigen Tippfehler in einer Affix-Liste, nicht nur
 * diesen einen Namen. Solange `active` in `POOL_NAMES` steht, fällt es beim
 * Ladeversuch auf statt im Kampf zu verschwinden.
 */
import { describe, expect, it } from 'vitest';

import { ITEMS, pruefeAffixArten, wornFrom } from '../../src/gear/catalog.js';
import { POOL_NAMES } from '../../src/gear/pools.js';
import { feldFuerAffixArt, NOTHING_WORN } from '../../src/gear/worn.js';

describe('Ein Topf ohne Wert bricht den Ladeversuch', () => {
  it('nennt den Namen, an dem es liegt', () => {
    // **Der Name muss in der Meldung stehen.** Eine Ausnahme ohne ihn sagt
    // „irgendwo im Katalog", und der Katalog wird Hunderte Einträge haben.
    expect(() => pruefeAffixArten('test-x', ['active'])).toThrow(/active/);
  });

  it('sagt auch, WARUM — sonst sucht der Leser den Tippfehler', () => {
    // `active` ist richtig geschrieben und steht in POOL_NAMES. Ohne die
    // Begründung suchte man an der falschen Stelle.
    expect(() => pruefeAffixArten('test-x', ['active'])).toThrow(/keinen Wert am Spieler füllt/);
  });

  it('unterscheidet den Tippfehler vom Topf ohne Wert', () => {
    // Zwei verschiedene Fehler, zwei verschiedene Meldungen — wer „catalyis"
    // schreibt, braucht eine andere Auskunft als wer `active` wählt.
    expect(() => pruefeAffixArten('test-x', ['catalyis'])).toThrow(/die es nicht gibt/);
  });

  it('GEGENPROBE: gültige Arten bleiben grün', () => {
    // Ohne diese Zeile wäre eine Prüfung, die IMMER wirft, ebenfalls grün.
    expect(() => pruefeAffixArten('test-x', ['catalysis', 'purity', 'yield'])).not.toThrow();
    expect(() => pruefeAffixArten('test-x', ['essence', 'gearEffects'])).not.toThrow();
    expect(() => pruefeAffixArten('test-x', [])).not.toThrow();
  });
});

describe('Die Abbildung von Art auf Feld', () => {
  it('führt jede wertbare Art auf ein Feld, das es wirklich gibt', () => {
    for (const kind of POOL_NAMES) {
      const feld = feldFuerAffixArt(kind);
      if (feld === null) continue;
      expect(feld in NOTHING_WORN, `${kind} zeigt auf das Feld "${feld}", das es nicht gibt`).toBe(
        true,
      );
    }
  });

  it('kennt genau einen Topf ohne Wert — und der heißt `active`', () => {
    // **Als Liste festgenagelt, nicht als Zahl.** Käme ein zweiter Topf ohne
    // Wert dazu, sagte „eine von sechs" nichts darüber, WELCHE — und ein
    // vertauschtes Paar bliebe grün.
    const ohneFeld = POOL_NAMES.filter((kind) => feldFuerAffixArt(kind) === null);
    expect(ohneFeld).toEqual(['active']);
  });

  it('bildet `yield` auf `yieldBonus` ab — das Wort ist in JavaScript belegt', () => {
    expect(feldFuerAffixArt('yield')).toBe('yieldBonus');
  });
});

describe('Der ausgelieferte Katalog hält die Regel ein', () => {
  it('nennt nirgends eine Art ohne Wert', () => {
    // Der Ladeweg prüft das bereits — dieser Fall hält fest, dass er es für
    // JEDEN Eintrag tut und nicht nur für den ersten.
    for (const item of ITEMS) {
      expect(() => pruefeAffixArten(item.id, item.affixKinds)).not.toThrow();
    }
  });

  it('und die Werte kommen vollständig an', () => {
    // **Die Summe der Affix-Anteile darf nicht verfallen.** Genau das war der
    // Schaden: Der Gegenstand trug sein Budget, und ein Teil landete nirgends.
    const worn = wornFrom([{ itemId: 'test-relic', upgradeSteps: 0, equippedSlot: 0 }]);
    const fremde = Object.keys(worn).filter((k) => !(k in NOTHING_WORN));
    expect(fremde, `wornFrom hat Felder erfunden: ${fremde.join(', ')}`).toEqual([]);
  });
});
