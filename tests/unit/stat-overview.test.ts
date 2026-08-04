/**
 * Die Werte-Uebersicht (Issue #188).
 *
 * DER KERN IST DER DECKEL, DEN ES NOCH NICHT GIBT.
 * PURITY ist bei 100 % gedeckelt, CATALYSIS nicht -- der Deckel wird erst
 * gemessen (Issue #118). Die Uebersicht muss beides tragen, ohne dass die
 * spaetere Messung eine Codeaenderung ausloest.
 */
import { describe, expect, it } from 'vitest';
import { CRIT_BASE_PERMILLE, NOTHING_WORN, type WornValues } from '../../src/gear/worn.js';
import {
  formatValue,
  statOverview,
  STAT_CAPS,
  STAT_ORDER,
  type StatRow,
} from '../../src/progression/stat-overview.js';

const GRUND = { essence: 1000, vessel: 5000, ward: 20 };

function zeile(rows: readonly StatRow[], id: string): StatRow {
  const gefunden = rows.find((r) => r.id === id);
  if (gefunden === undefined) throw new Error(`Zeile ${id} fehlt`);
  return gefunden;
}

function getragen(over: Partial<WornValues> = {}): WornValues {
  return { ...NOTHING_WORN, ...over };
}

describe('AC1 — alle sechs Werte stehen da', () => {
  it('fuehrt sie in der Reihenfolge des Designs', () => {
    const rows = statOverview(GRUND, NOTHING_WORN);
    expect(rows.map((r) => r.id)).toEqual([
      'essence',
      'vessel',
      'ward',
      'catalysis',
      'purity',
      'yield',
    ]);
    expect(rows).toHaveLength(STAT_ORDER.length);
  });

  it('rechnet Promille in Prozent um', () => {
    // Im Kampf Promille, auf dem Bildschirm Prozent. Ohne die Umrechnung
    // stuende dort '520 %' statt '52 %'.
    const rows = statOverview(GRUND, getragen({ catalysis: 520 }));
    expect(zeile(rows, 'catalysis').value).toBe(52);
    expect(zeile(rows, 'catalysis').unit).toBe('percent');
  });

  it('zeigt bei YIELD den GESAMTEN Kritschaden, nicht nur den Zuschlag', () => {
    // Der Spieler vergleicht ihn mit dem, was ein Treffer austeilt -- der
    // Grundwert von 150 % gehoert dazu. Nur den Zuschlag zu zeigen waere eine
    // Zahl, die er nirgends wiederfindet.
    const ohne = statOverview(GRUND, NOTHING_WORN);
    expect(zeile(ohne, 'yield').value).toBe(CRIT_BASE_PERMILLE / 10);
    const mit = statOverview(GRUND, getragen({ yieldBonus: 300 }));
    expect(zeile(mit, 'yield').value).toBe((CRIT_BASE_PERMILLE + 300) / 10);
  });
});

describe('AC2 und AC3 — die Warnung und ihr Gegenfall', () => {
  it('warnt bei PURITY UEBER dem Deckel, mit Zahl', () => {
    const rows = statOverview(GRUND, getragen({ purity: 1020 }));
    const p = zeile(rows, 'purity');
    expect(p.value).toBe(102);
    expect(p.cap).toBe(100);
    expect(p.overCap).toBe(2);
    expect(p.warning).toBe('2 % over the limit, no effect');
  });

  it('warnt NICHT unter dem Deckel', () => {
    // Der Gegenfall, und er ist nicht verhandelbar: Eine Warnung, die immer
    // erscheint, ist keine Warnung.
    const rows = statOverview(GRUND, getragen({ purity: 800 }));
    const p = zeile(rows, 'purity');
    expect(p.value).toBe(80);
    expect(p.cap).toBe(100);
    expect(p.overCap).toBeNull();
    expect(p.warning).toBeNull();
  });

  it('warnt auch GENAU auf dem Deckel nicht', () => {
    // 100 ist erreicht, nicht ueberschritten. Ein 'over the limit' bei genau
    // 100 waere falsch und wuerde den Spieler zum Umbauen bringen, obwohl er
    // genau richtig liegt.
    const rows = statOverview(GRUND, getragen({ purity: 1000 }));
    expect(zeile(rows, 'purity').overCap).toBeNull();
    expect(zeile(rows, 'purity').warning).toBeNull();
  });
});

describe('AC4 — der Deckel, den es noch nicht gibt', () => {
  it('zeigt CATALYSIS ohne Deckel und ohne Warnung', () => {
    // Ein '2 % ueber dem Limit' ohne Limit ist nicht darstellbar. Bis Issue
    // #118 gemessen hat, gibt es keinen -- und das ist eine Aussage, kein
    // vergessener Eintrag.
    const rows = statOverview(GRUND, getragen({ catalysis: 9999 }));
    const c = zeile(rows, 'catalysis');
    expect(c.value).toBeGreaterThan(100);
    expect(c.cap).toBeNull();
    expect(c.overCap).toBeNull();
    expect(c.warning).toBeNull();
  });

  it('traegt einen spaeteren Deckel als DATENWERT, nicht als Codeaenderung', () => {
    // Die Zusicherung hinter AC4. Wenn #118 einen Deckel misst, wird er in
    // STAT_CAPS eingetragen -- die Anzeige braucht keine Fallunterscheidung.
    //
    // Geprueft wird das an einem Wert, der HEUTE schon einen Deckel hat: Die
    // Warnung entsteht allein daraus, dass STAT_CAPS eine Zahl fuehrt. Waere
    // sie an den Namen 'purity' gebunden, gaebe es sie fuer catalysis auch
    // nach der Messung nicht.
    expect(STAT_CAPS['purity']).toBe(100);
    expect(STAT_CAPS['catalysis']).toBeNull();

    // Derselbe Wert, einmal mit und einmal ohne Deckel in der Tabelle --
    // dasselbe Ergebnis wie bei purity/catalysis, ohne dass ein Name geprueft
    // wird.
    const ueber = statOverview(GRUND, getragen({ purity: 1020 }));
    const ohne = statOverview(GRUND, getragen({ catalysis: 1020 }));
    expect(zeile(ueber, 'purity').warning).not.toBeNull();
    expect(zeile(ohne, 'catalysis').warning).toBeNull();
    // Beide tragen denselben Wert -- der Unterschied kommt AUSSCHLIESSLICH
    // aus der Deckel-Tabelle.
    expect(zeile(ueber, 'purity').value).toBe(zeile(ohne, 'catalysis').value);
  });
});

describe('AC5 — ohne Ausruestung stehen die Grundwerte da', () => {
  it('zeigt sie, statt leer zu bleiben', () => {
    // Ein neuer Spielstand hat keine Ausruestung. Eine leere Uebersicht saehe
    // aus wie ein Fehler.
    const rows = statOverview(GRUND, NOTHING_WORN);
    expect(zeile(rows, 'essence').value).toBe(1000);
    expect(zeile(rows, 'vessel').value).toBe(5000);
    expect(zeile(rows, 'ward').value).toBe(20);
    for (const r of rows) {
      expect(Number.isFinite(r.value), r.id).toBe(true);
      expect(r.label.length, r.id).toBeGreaterThan(0);
    }
  });

  it('wirft nicht und zeigt keine Warnung', () => {
    const rows = statOverview(GRUND, NOTHING_WORN);
    expect(rows.filter((r) => r.warning !== null)).toEqual([]);
  });
});

describe('Die Schreibweise', () => {
  it('schreibt Prozent mit Zeichen und Rohzahlen ohne', () => {
    expect(formatValue(52, 'percent')).toBe('52 %');
    expect(formatValue(5000, 'flat')).toBe('5000');
  });

  it('schreibt Wert und Deckel gleich, damit man sie vergleichen kann', () => {
    const p = zeile(statOverview(GRUND, getragen({ purity: 1020 })), 'purity');
    expect(formatValue(p.value, p.unit)).toBe('102 %');
    expect(formatValue(p.cap ?? 0, p.unit)).toBe('100 %');
  });
});
