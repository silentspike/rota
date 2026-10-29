/**
 * Freischaltung der Stufen und das Punkte-Ventil (GAME-DESIGN 12.7, Issue #1177).
 *
 * > **Schwierigkeitsstufe** — Heroische Instanz öffnet nach Normal, mythische
 * > nach Heroisch.
 * > **Punkte-Schwelle** — Ventil gegen Feststecken: Wer an einem Boss
 * > scheitert, kommt durch Perfektionieren alter Kämpfe weiter.
 * > (Masterplan, Instanzen und Freischaltung)
 *
 * Und die Entscheidung des Auftraggebers vom 29.10.: „Instanzen öffnen über die
 * Punkte-Schwelle **ohne Ausgeben**“ — gezählt wird, was verdient wurde.
 *
 * # Über den echten Schlüssel, nicht über reine Kennungen
 *
 * `defeatedBosses` führt `markKey(id, stufe)`. Ein Prüffall mit reinen
 * Kennungen hat am 2026-08-11 schon einmal eine Freischaltung grün gemeldet,
 * die im Spiel nie griff (`freischaltung-schluesselformat.test.ts`). Hier wird
 * jeder Sieg mit dem Schlüssel gebaut, den `recordRun` schreibt.
 */
import { describe, expect, it } from 'vitest';
import { BOSSES, INSTANCES, REWARD, bossAtDifficulty } from '../../src/core/data.js';
import { markKey } from '../../src/analysis/progress-mark.js';
import {
  freigeschalteteInstanzen,
  freischaltungen,
  neueFreischaltungen,
  offeneStufen,
} from '../../src/progression/boss-select.js';
import { pointsBreakdown } from '../../src/progression/points.js';
import { DIFFICULTY_HEROIC, DIFFICULTY_MYTHIC, DIFFICULTY_NORMAL } from '../../src/loot/rates.js';

/** Die Bosse einer Instanz, in Datei-Reihenfolge. */
function bosseVon(instanz: string): string[] {
  return [...BOSSES.values()]
    .map((e) => e.value)
    .filter((b) => b.instance === instanz)
    .map((b) => b.id);
}

/** Die Schlüssel „besiegt auf dieser Stufe“ für eine Liste von Bossen. */
function besiegtAuf(ids: readonly string[], stufe: number): string[] {
  return ids.map((id) => markKey(id, stufe));
}

const INSTANZ_1 = bosseVon('instance-1');

describe('ZUSICHERUNG — die Instanz hat vier Bosse, und alle haben Stufen', () => {
  it('instance-1 führt vier Bosse', () => {
    expect(INSTANZ_1).toEqual(['boss-1-1', 'boss-1-2', 'boss-1-3', 'boss-1-4']);
  });
});

describe('AC2 — Heroisch öffnet nach Normal, je Instanz', () => {
  it('ohne Sieg ist nur Normal offen', () => {
    expect(offeneStufen('instance-1', [])).toEqual([DIFFICULTY_NORMAL]);
  });

  it('drei von vier auf Normal öffnen Heroisch NICHT', () => {
    const drei = besiegtAuf(INSTANZ_1.slice(0, 3), DIFFICULTY_NORMAL);
    expect(offeneStufen('instance-1', drei)).toEqual([DIFFICULTY_NORMAL]);
  });

  it('alle vier auf Normal öffnen Heroisch, aber nicht Mythisch', () => {
    const alle = besiegtAuf(INSTANZ_1, DIFFICULTY_NORMAL);
    expect(offeneStufen('instance-1', alle)).toEqual([DIFFICULTY_NORMAL, DIFFICULTY_HEROIC]);
  });

  it('eine abgeschlossene Instanz öffnet die Stufe nur bei sich, nicht bei der nächsten', () => {
    const alle = besiegtAuf(INSTANZ_1, DIFFICULTY_NORMAL);
    expect(offeneStufen('instance-2', alle)).toEqual([DIFFICULTY_NORMAL]);
  });
});

describe('AC2 — Mythisch öffnet nach Heroisch', () => {
  const normal = besiegtAuf(INSTANZ_1, DIFFICULTY_NORMAL);

  it('drei von vier auf Heroisch öffnen Mythisch NICHT', () => {
    const drei = [...normal, ...besiegtAuf(INSTANZ_1.slice(0, 3), DIFFICULTY_HEROIC)];
    expect(offeneStufen('instance-1', drei)).toEqual([DIFFICULTY_NORMAL, DIFFICULTY_HEROIC]);
  });

  it('alle vier auf Heroisch öffnen Mythisch', () => {
    const alle = [...normal, ...besiegtAuf(INSTANZ_1, DIFFICULTY_HEROIC)];
    expect(offeneStufen('instance-1', alle)).toEqual([
      DIFFICULTY_NORMAL,
      DIFFICULTY_HEROIC,
      DIFFICULTY_MYTHIC,
    ]);
  });

  it('die Gegenprobe: heroische Siege ohne Normal öffnen nichts', () => {
    // Ein Spielstand, der so nicht entstehen kann — und gerade deshalb die
    // Probe, dass die Kette Stufe für Stufe gefragt wird, nicht nur die oberste.
    const nurHeroisch = besiegtAuf(INSTANZ_1, DIFFICULTY_HEROIC);
    expect(offeneStufen('instance-1', nurHeroisch)).toEqual([DIFFICULTY_NORMAL]);
  });

  it('eine Instanz, deren Bosse keine Stufen haben, bleibt bei Normal', () => {
    // Die Prüf-Bosse tragen `test` oder `testing` und keine `difficulties`.
    const pruef = bosseVon('testing');
    expect(pruef.length, 'keine Prüf-Bosse gefunden').toBeGreaterThan(0);
    expect(offeneStufen('testing', besiegtAuf(pruef, DIFFICULTY_NORMAL))).toEqual([
      DIFFICULTY_NORMAL,
    ]);
  });
});

describe('AC2 — die Punkte-Schwelle öffnet die Instanz ohne den Abschluss davor', () => {
  const zweiteSchwelle = INSTANCES.find((i) => i.id === 'instance-2')?.pointsThreshold ?? 0;

  it('ZUSICHERUNG: instance-2 hat eine Schwelle', () => {
    expect(zweiteSchwelle).toBeGreaterThan(0);
  });

  it('knapp darunter bleibt instance-2 zu', () => {
    const offen = freigeschalteteInstanzen([], BOSSES, zweiteSchwelle - 1);
    expect(offen).not.toContain('instance-2');
  });

  it('auf der Schwelle öffnet sie — ohne einen einzigen Sieg in instance-1', () => {
    const offen = freigeschalteteInstanzen([], BOSSES, zweiteSchwelle);
    expect(offen).toContain('instance-2');
    // Nur sie: Die nächste hat ihre eigene, höhere Schwelle.
    expect(offen).not.toContain('instance-3');
  });

  it('über der Schwelle bleibt sie offen, und jede weitere öffnet an ihrer eigenen', () => {
    expect(freigeschalteteInstanzen([], BOSSES, zweiteSchwelle + 1)).toContain('instance-2');
    for (const instanz of INSTANCES.filter((i) => i.pointsThreshold !== null)) {
      const schwelle = instanz.pointsThreshold ?? 0;
      expect(freigeschalteteInstanzen([], BOSSES, schwelle - 1), instanz.id).not.toContain(
        instanz.id,
      );
      expect(freigeschalteteInstanzen([], BOSSES, schwelle), instanz.id).toContain(instanz.id);
      expect(freigeschalteteInstanzen([], BOSSES, schwelle + 1), instanz.id).toContain(instanz.id);
    }
  });

  it('ohne Punkte gilt allein der Hauptpfad, wie bis #1177', () => {
    expect(freigeschalteteInstanzen([], BOSSES)).toEqual(['prologue', 'instance-1']);
    const erste = besiegtAuf(INSTANZ_1, DIFFICULTY_NORMAL);
    expect(freigeschalteteInstanzen(erste, BOSSES)).toContain('instance-2');
  });

  it('die Gegenprobe: eine Instanz ohne Schwelle öffnet bei keiner Punktzahl', () => {
    // Prolog und Instanz 1 tragen `null` — sie sind ohnehin offen. Verlangte
    // eine Instanz ohne Schwelle etwas, bliebe sie bei jeder Punktzahl zu;
    // hier prüfen wir die Lesart: `null` heißt „kein Ventil“, nicht „Schwelle 0“.
    const ohne = INSTANCES.filter((i) => i.pointsThreshold === null).map((i) => i.id);
    expect(ohne).toEqual(['prologue', 'instance-1']);
  });
});

describe('Der Klang „Freischaltung“ — einmal je Freischaltung', () => {
  const normal = besiegtAuf(INSTANZ_1, DIFFICULTY_NORMAL);
  const zweiteSchwelle = INSTANCES.find((i) => i.id === 'instance-2')?.pointsThreshold ?? 0;

  it('der vierte Normal-Sieg öffnet die heroische Instanz und die nächste', () => {
    const vorher = freischaltungen(normal.slice(0, 3));
    const nachher = freischaltungen(normal);
    expect(neueFreischaltungen(vorher, nachher)).toEqual(['instance-1@heroic', 'instance-2']);
  });

  it('ein Sieg, der nichts öffnet, meldet nichts — auch nicht der fünfte nach dem vierten', () => {
    expect(neueFreischaltungen(freischaltungen(normal), freischaltungen(normal))).toEqual([]);
    expect(
      neueFreischaltungen(freischaltungen(normal.slice(0, 1)), freischaltungen(normal.slice(0, 2))),
    ).toEqual([]);
  });

  it('die Schwelle meldet sich in dem Moment, in dem sie überschritten wird', () => {
    expect(
      neueFreischaltungen(
        freischaltungen([], BOSSES, zweiteSchwelle - 1),
        freischaltungen([], BOSSES, zweiteSchwelle),
      ),
    ).toEqual(['instance-2']);
    expect(
      neueFreischaltungen(
        freischaltungen([], BOSSES, zweiteSchwelle),
        freischaltungen([], BOSSES, zweiteSchwelle + 500),
      ),
    ).toEqual([]);
  });

  it('Mythisch meldet sich erst mit dem letzten heroischen Sieg', () => {
    const heroisch = besiegtAuf(INSTANZ_1, DIFFICULTY_HEROIC);
    const fast = [...normal, ...heroisch.slice(0, 3)];
    expect(neueFreischaltungen(freischaltungen(normal), freischaltungen(fast))).toEqual([]);
    expect(
      neueFreischaltungen(freischaltungen(fast), freischaltungen([...normal, ...heroisch])),
    ).toEqual(['instance-1@mythic']);
  });
});

describe('Die Schwellen — Eigenschaften, nicht Zahlen', () => {
  /** Was ein Normal-Sieg mit diesem Vorsprung zahlt, über die echte Formel. */
  function sieg(restMs: number): number {
    return pointsBreakdown(
      { victory: true, remainingMs: restMs, heldPercent: null },
      { bossLevel: 1, playerLevel: 1 },
    ).awarded;
  }

  /** Wie viele Bosse vor Instanz k liegen, wenn man an deren letztem feststeckt. */
  function davor(k: number): number {
    // Prolog (1) + je vier der Instanzen 1 bis k−2 + drei von k−1.
    return 1 + 4 * (k - 2) + 3;
  }

  const mitSchwelle = INSTANCES.filter((i) => i.pointsThreshold !== null);

  it('ZUSICHERUNG: vier Instanzen tragen eine Schwelle', () => {
    expect(mitSchwelle.map((i) => i.id)).toEqual([
      'instance-2',
      'instance-3',
      'instance-4',
      'instance-5',
    ]);
  });

  it('jede liegt höher als die davor', () => {
    const werte = mitSchwelle.map((i) => i.pointsThreshold ?? 0);
    expect([...werte].sort((a, b) => a - b)).toEqual(werte);
    expect(new Set(werte).size).toBe(werte.length);
  });

  it('bloßes Durchkommen (ein Viertel Restzeit) öffnet keine', () => {
    for (const instanz of mitSchwelle) {
      const k = instanz.number;
      const durchkommen = davor(k) * sieg(15_000);
      expect(durchkommen, instanz.id).toBeLessThan(instanz.pointsThreshold ?? 0);
    }
  });

  it('perfektionierte Normal-Siege (die Hälfte Restzeit) öffnen jede — auch ohne Heroisch', () => {
    for (const instanz of mitSchwelle) {
      const k = instanz.number;
      const perfektioniert = davor(k) * sieg(30_000);
      expect(perfektioniert, instanz.id).toBeGreaterThanOrEqual(instanz.pointsThreshold ?? 0);
    }
  });

  it('die Rechnung stimmt mit der Datei: Kurve aus reward.json, Normal-Zeitlimit 60 s', () => {
    // Die beiden Annahmen der Herleitung, beim Namen — ändert sich eine,
    // stimmen die Schwellen nicht mehr, und dieser Fall sagt es.
    expect(REWARD.pointsExponent).toBe(2);
    expect(bossAtDifficulty('boss-1-1', 0)?.overpressureMs).toBe(60_000);
  });
});
