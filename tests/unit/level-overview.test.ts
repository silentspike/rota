/**
 * Was der Aufstieg im Player Home zeigt (Issue #248, AC6 — der rechnende Teil).
 *
 * Die Ansicht selbst prüft nur der Browser: Die Prüffälle laufen ohne DOM. Was
 * hier geprüft wird, ist die Rechnung dahinter — und genau deshalb steht sie in
 * einer eigenen Datei. Eine Zahl, die im DOM entsteht, ließe sich ohne Browser
 * gar nicht messen.
 */
import { describe, expect, it } from 'vitest';
import { LEVELS } from '../../src/core/data.js';
import { levelBarFill, levelOverview } from '../../src/progression/level-overview.js';

function zeile(state: { level: number; xp: number }, id: string) {
  return levelOverview(state).find((row) => row.id === id);
}

describe('AC6 — Stufe, Erfahrung und Plätze stehen da', () => {
  it('zeigt alle vier Zeilen, jede mit ihrer Kennung', () => {
    // Die Kennungen tragen die Prüfung im Browser: Dort lässt sich messen, was
    // dasteht, ohne die Gestaltung zu befragen.
    const ids = levelOverview({ level: 1, xp: 0 }).map((row) => row.id);
    expect(ids).toEqual(['level', 'experience', 'bar-slots', 'stat-growth']);
  });

  it('nennt die Erfahrung als Bruch und was noch fehlt', () => {
    const row = zeile({ level: 1, xp: 50 }, 'experience');
    expect(row?.value).toBe('50 / 200');
    expect(row?.note).toBe('150 to next level');
  });

  it('rechnet den Balken, statt ihn zu führen', () => {
    expect(levelBarFill({ level: 1, xp: 0 })).toBe(0);
    expect(levelBarFill({ level: 1, xp: 100 })).toBeCloseTo(0.5, 5);
    expect(levelBarFill({ level: 1, xp: 200 })).toBe(1);
  });

  it('zeigt den WACHSTUMSFAKTOR, nicht noch einmal einen Grundwert', () => {
    // **Im Bild gefunden, nicht im Code.** Hier stand einen Entwurf lang
    // „Vessel 5000" — und die Werte-Übersicht direkt darunter zeigt dieselbe
    // Zahl unter demselben Namen. Zwei Zahlen mit einem Namen auf einem
    // Bildschirm ist der Fall, der in #247 den ganzen Baustein umgeworfen hat.
    //
    // Der Faktor steht nirgends sonst und beantwortet die Frage, die hierher
    // gehört: Was hat der Aufstieg gebracht?
    const auf1 = zeile({ level: 1, xp: 0 }, 'stat-growth');
    const auf50 = zeile({ level: 50, xp: 0 }, 'stat-growth');
    expect(auf1?.value).toBe('x1');
    expect(auf50?.value).toBe(`x${(LEVELS.factorPerMille[49] as number) / 1000}`);
    expect(auf1?.note).toBe(`+${LEVELS.growthPerLevelPercent}% per level`);
  });

  it('nennt die nächste Freischalt-Stufe, nicht die nächste Stufe', () => {
    // Der Spieler soll wissen, worauf er zusteuert. „Next level" stünde schon
    // in der Zeile darüber.
    const row = zeile({ level: 1, xp: 0 }, 'bar-slots');
    expect(row?.value).toBe('2');
    expect(row?.note).toBe(`next at level ${LEVELS.barSlotLevels[0]}`);
  });

  it('sagt „all unlocked", wenn kein Platz mehr kommt', () => {
    const row = zeile({ level: LEVELS.maxLevel, xp: 0 }, 'bar-slots');
    expect(row?.value).toBe('12');
    expect(row?.note).toBe('all unlocked');
  });
});

describe('Auf der Höchststufe steht MAX statt einer unerreichbaren Zahl', () => {
  it('zeigt keinen Bedarf, den niemand einlöst', () => {
    // „Noch 197008 bis Stufe 101" wäre ein Versprechen ohne Deckung, und ein
    // Balken, der bei einer unerreichbaren Zahl hängt, sähe aus wie ein Fehler.
    const row = zeile({ level: LEVELS.maxLevel, xp: 0 }, 'experience');
    expect(row?.value).toBe('MAX');
    expect(row?.note).toBeNull();
    expect(levelBarFill({ level: LEVELS.maxLevel, xp: 0 })).toBe(1);
  });

  it('merkt die Höchststufe auch in der Stufen-Zeile an', () => {
    expect(zeile({ level: LEVELS.maxLevel, xp: 0 }, 'level')?.note).toBe('maximum reached');
    expect(zeile({ level: 1, xp: 0 }, 'level')?.note).toBeNull();
  });

  it('klemmt eine Stufe über der Höchststufe, statt sie anzuzeigen', () => {
    // Ein Spielstand aus einer Fassung mit höherer Höchststufe soll lesbar
    // bleiben — dieselbe Regel wie bei den Grundwerten.
    const row = zeile({ level: 500, xp: 0 }, 'level');
    expect(row?.value).toBe(String(LEVELS.maxLevel));
  });
});
