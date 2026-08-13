/**
 * Der Bestlauf trägt seine Vergleichszahlen (Issue #82).
 *
 * # Was hier geprüft wird
 *
 * Seit #82 führt ein `BestRun` die sieben Zahlen, die `compareRuns` liest —
 * damit die Auswertung „dieser Lauf gegen deinen besten" überhaupt etwas
 * vergleichen kann. Bis dahin stand im Spielstand **eine** Zahl (`metric` und
 * `value`), und die Vergleichsansicht hatte keinen Aufrufer.
 *
 * # Die Fassungs-Wanderung ist eine NICHT-Wanderung, und das ist Absicht
 *
 * Die sieben Felder sind **optional**. Ein Spielstand aus einer Fassung vor
 * #82 lädt damit unverändert weiter: `isWellFormed` prüft nur die sechs
 * Pflichtfelder, und `comparableFromBest` gibt `null` — dieselbe Aussage wie
 * „es gibt noch keinen Bestlauf".
 *
 * Der Gegenentwurf wäre gewesen, die Felder zur Pflicht zu machen und alte
 * Einträge mit Nullen aufzufüllen. Das wäre schlimmer als kein Vergleich: Der
 * Spieler sähe „0 Schaden" als Bestwert und läge in jeder Kennzahl scheinbar
 * vorn. Ein Fall unten misst genau das.
 */

import { describe, expect, it } from 'vitest';
import {
  comparableFromBest,
  isWellFormed,
  recordBest,
  usableBest,
} from '../../src/analysis/best-run.js';
import { compareRuns } from '../../src/analysis/run-compare.js';
import type { BestRun } from '../../src/analysis/save-format.js';
import { findForbiddenFields } from '../../src/analysis/save-format.js';

/** Ein Eintrag im ALTEN Format — so lag er vor #82 im Spielstand. */
const ALT: BestRun = {
  bossId: 'first-crucible',
  difficulty: 1,
  metric: 'reserve',
  value: 19_490,
  gameVersion: '0.1.0',
  achievedAt: 1_800_000_000_000,
};

/** Derselbe Eintrag, wie ihn #82 schreibt. */
const NEU: BestRun = {
  ...ALT,
  victory: true,
  elapsedMs: 40_510,
  damage: 4240,
  progressPercent: 100,
  casts: 28,
  lostTimeWithReadyMs: 1200,
  lostTimeNothingReadyMs: 3400,
};

const LAUF = {
  victory: true,
  elapsedMs: 38_000,
  damage: 4500,
  progressPercent: 100,
  casts: 30,
  lostTimeWithReadyMs: 900,
  lostTimeNothingReadyMs: 3000,
};

describe('AC1 — die sieben Zahlen kommen im Bestlauf an', () => {
  it('recordBest schreibt sie mit', () => {
    const { runs } = recordBest([], {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 19_490,
      gameVersion: '0.1.0',
      at: 1_800_000_000_000,
      comparison: LAUF,
    });
    expect(runs).toHaveLength(1);
    expect(comparableFromBest(runs[0] ?? null)).toEqual(LAUF);
  });

  it('und sie gehören dem BESTWERT, nicht dem letzten Versuch', () => {
    // Ein schlechterer Lauf darf die Zahlen des Bestlaufs nicht überschreiben
    // — sonst vergliche sich der Spieler gegen seinen letzten statt gegen
    // seinen besten, und der Vergleich zeigte immer eine Null.
    const { runs } = recordBest([NEU], {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 1000, // schlechter als 19 490
      gameVersion: '0.1.0',
      at: 1_800_000_100_000,
      comparison: { ...LAUF, damage: 1 },
    });
    expect(comparableFromBest(runs[0] ?? null)?.damage).toBe(4240);
  });

  it('ein BESSERER Lauf schreibt sie dagegen neu', () => {
    const { runs } = recordBest([NEU], {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 25_000,
      gameVersion: '0.1.0',
      at: 1_800_000_100_000,
      comparison: { ...LAUF, damage: 9999 },
    });
    expect(comparableFromBest(runs[0] ?? null)?.damage).toBe(9999);
  });
});

describe('AC2 — ein Spielstand im ALTEN Format lädt ohne Verlust', () => {
  it('die Formprüfung nimmt ihn an', () => {
    // Die eigentliche „Wanderung": Sie besteht darin, dass es keine braucht.
    expect(isWellFormed(ALT)).toBe(true);
  });

  it('seine sechs Pflichtfelder bleiben unangetastet', () => {
    // „Ohne Verlust" heisst wörtlich: Nichts fällt weg.
    expect(ALT.bossId).toBe('first-crucible');
    expect(ALT.metric).toBe('reserve');
    expect(ALT.value).toBe(19_490);
    expect(ALT.gameVersion).toBe('0.1.0');
    expect(ALT.achievedAt).toBe(1_800_000_000_000);
  });

  it('und der Vergleich sagt UNBEKANNT statt zu raten', () => {
    expect(comparableFromBest(ALT)).toBeNull();
    const metriken = compareRuns(LAUF, comparableFromBest(ALT));
    // Alle Kennzahlen ohne Bestwert, keine auf `false`.
    expect(metriken.every((m) => m.best === null)).toBe(true);
    expect(metriken.every((m) => m.improved === null)).toBe(true);
  });

  it('GEGENPROBE: mit Nullen aufgefüllt läge der Spieler überall vorn', () => {
    // Der verworfene Gegenentwurf, ausgeschrieben. Ein Vergleich gegen
    // erfundene Nullen meldet in jeder Kennzahl eine Verbesserung — der
    // Spieler bekäme Fortschritt gemeldet, wo nichts zu vergleichen war.
    const genullt = {
      victory: false,
      elapsedMs: 0,
      damage: 0,
      progressPercent: 0,
      casts: 0,
      lostTimeWithReadyMs: 0,
      lostTimeNothingReadyMs: 0,
    };
    const mitNullen = compareRuns(LAUF, genullt);
    const gemeldet = mitNullen.filter((m) => m.improved === true).length;
    expect(gemeldet).toBeGreaterThan(0);
    // Und genau das passiert mit `null` NICHT.
    const mitNull = compareRuns(LAUF, comparableFromBest(ALT));
    expect(mitNull.filter((m) => m.improved === true)).toHaveLength(0);
  });
});

describe('AC3 — ein halber Eintrag zählt als keiner', () => {
  it('fehlt EINE der sieben, gibt es keinen Vergleich', () => {
    // Kann heute nicht entstehen (sie werden gemeinsam geschrieben), aber ein
    // Vergleich mit einer Lücke mitten in der Tabelle wäre nicht erklärbar.
    const halb = { ...NEU } as Record<string, unknown>;
    delete halb['casts'];
    expect(comparableFromBest(halb as unknown as BestRun)).toBeNull();
  });

  it('und das gilt für jede einzelne der sieben', () => {
    const felder = [
      'victory',
      'elapsedMs',
      'damage',
      'progressPercent',
      'casts',
      'lostTimeWithReadyMs',
      'lostTimeNothingReadyMs',
    ] as const;
    for (const feld of felder) {
      // **Das Feld ENTFERNEN, nicht auf `undefined` setzen.** Bei
      // `exactOptionalPropertyTypes` ist „fehlt" etwas anderes als „ist
      // undefined" — und im Spielstand fehlt es wirklich, es steht nicht als
      // `undefined` darin.
      const ohne = { ...NEU } as Record<string, unknown>;
      delete ohne[feld];
      expect(comparableFromBest(ohne as unknown as BestRun), `ohne ${feld}`).toBeNull();
    }
    // ZUSICHERUNG: Mit allen sieben geht es durch — sonst prüfte die Schleife
    // nur, dass die Funktion immer `null` gibt.
    expect(comparableFromBest(NEU)).not.toBeNull();
  });
});

describe('AC4 — der Wächter gegen berechnete Werte bleibt scharf', () => {
  it('`damage` im Bestlauf ist erlaubt — als vergangenes Ergebnis', () => {
    const stand = { bestRuns: [NEU] } as never;
    expect(findForbiddenFields(stand)).not.toContain('damage');
  });

  it('aber NICHT eine Ebene tiefer', () => {
    // Die Ausnahme hängt am Pfad `^bestRuns\.\d+$`. Griffe sie am Namen,
    // wäre der Wächter aufgehoben, ohne dass eine Zeile danach aussieht.
    const stand = { bestRuns: [{ ...NEU, detail: { damage: 40 } }] } as never;
    expect(findForbiddenFields(stand)).toContain('damage');
  });

  it('und nicht auf einem Gegenstand', () => {
    const stand = {
      items: [{ itemId: 'x', upgradeSteps: 0, equippedSlot: null, damage: 40 }],
    } as never;
    expect(findForbiddenFields(stand)).toContain('damage');
  });
});

describe('AC5 — metric und value bleiben wirksam, gerade beim runaway-Boss', () => {
  /**
   * Die Ergänzung aus 6717, und sie ist der eigentliche Grund, warum die
   * sieben Zahlen DAZUKOMMEN statt zu ersetzen.
   *
   * Bei einem `runaway`-Boss ist die Frist immer voll ausgeschöpft — die
   * Reserve ist dort konstant null und trennt keine zwei Siege (#256).
   * Getrennt wird über `heldPercent`, und genau dafür trägt der Eintrag sein
   * Mass. Hätte ich `metric` und `value` ersetzt, wäre bei diesem Boss **gar
   * kein Bestlauf mehr bestimmbar**.
   */
  const HELD_LAUF = { ...LAUF, elapsedMs: 60_000 };

  function eintrag(metric: 'reserve' | 'held', value: number): BestRun {
    return {
      bossId: 'test-runaway',
      difficulty: 1,
      metric,
      value,
      gameVersion: '0.1.0',
      achievedAt: 1_800_000_000_000,
      ...HELD_LAUF,
    };
  }

  it('usableBest wählt nach `held` aus, wenn danach gefragt wird', () => {
    const runs = [eintrag('held', 42)];
    const { entry, discarded } = usableBest(runs, 'test-runaway', 1, 'held');
    expect(discarded).toBeNull();
    expect(entry?.value).toBe(42);
  });

  it('und VERWIRFT denselben Eintrag, wenn nach `reserve` gefragt wird', () => {
    // Der teure Fall aus #256: Der Boss hat `runaway` bekommen oder verloren.
    // Ohne `metric` liesse sich das nicht unterscheiden — der Eintrag trüge
    // eine plausible Zahl der falschen Sorte, und 42 % stünden gegen
    // Millisekunden.
    const runs = [eintrag('held', 42)];
    const { entry, discarded } = usableBest(runs, 'test-runaway', 1, 'reserve');
    expect(entry).toBeNull();
    expect(discarded).toContain('Mass');
  });

  it('`value` entscheidet weiterhin, was besser ist', () => {
    // Grösser ist besser, bei beiden Massen (BEST_METRICS). Ein schlechterer
    // Ring-Rest darf den Bestwert nicht überschreiben.
    const { runs, improved } = recordBest([eintrag('held', 42)], {
      bossId: 'test-runaway',
      difficulty: 1,
      metric: 'held',
      value: 30,
      gameVersion: '0.1.0',
      at: 1_800_000_100_000,
      comparison: HELD_LAUF,
    });
    expect(improved).toBe(false);
    expect(runs[0]?.value).toBe(42);
  });

  it('ZUSICHERUNG: die sieben stehen daneben und stören nichts', () => {
    // Beide Datensätze am selben Eintrag — der eine wählt den Bestlauf, der
    // andere beschreibt ihn. Ohne diese Zeile bewiese der Block nur, dass
    // `metric` weiterhin geht, nicht dass beides nebeneinander besteht.
    const eintragMitBeidem = eintrag('held', 42);
    expect(eintragMitBeidem.metric).toBe('held');
    expect(eintragMitBeidem.value).toBe(42);
    expect(comparableFromBest(eintragMitBeidem)?.elapsedMs).toBe(60_000);
  });
});
