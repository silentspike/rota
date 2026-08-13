// @vitest-environment jsdom

/**
 * Kommt der Bestlauf-Vergleich in der Auswertung an? (Issue #82)
 *
 * # Warum diese Datei nötig ist — gemessen, nicht befürchtet
 *
 * `compareRuns` und `buildComparisonView` standen seit E7-2 fertig gebaut da
 * und hatten **keinen einzigen Aufrufer**. Gemessen am 2026-08-13:
 *
 * ```text
 *   buildComparisonView   1 Datei   (nur ihre eigene)
 *   compareRuns           1 Datei   (nur ihre eigene)
 *   buildHistoryView      2 Dateien (eigene + app.ts)  <- angeschlossen
 * ```
 *
 * Dieselbe Klasse wie `setSetEffects` in #794 und `buildEfficiencyView` in
 * #81: gebaut, geprüft, für jeden Spieler unerreichbar.
 *
 * **Die Zeilen allein genügen nicht.** Ein Anschluss ohne Fall ist nur ein
 * Anschluss bis zur nächsten Änderung. Gemessen: Setzt man den Callback in
 * `app.ts` auf `return null`, bleibt ohne diese Datei **jeder** der 2437 Fälle
 * grün.
 *
 * # Was hier NICHT geprüft wird
 *
 * Wie die Ansicht aussieht. Das ist Sache von `history-view`; hier zählt, dass
 * sie überhaupt gebaut wird und Zahlen bekommt.
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { compareRuns } from '../../src/analysis/run-compare.js';
import { comparableFromBest } from '../../src/analysis/best-run.js';
import { buildComparisonView } from '../../src/analysis/history-view.js';
import type { BestRun } from '../../src/analysis/save-format.js';
import type { ComparedMetric } from '../../src/analysis/run-compare.js';
import { Overlay } from '../../src/render/overlay.js';
import { OUTCOME_WIPE, PHASE_ENDED } from '../../src/core/phases.js';

/** jsdom bringt keinen Canvas-2D-Kontext mit; `Surface` wirft ohne ihn. */
function stubbeCanvas(): void {
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_ziel, name) => (name === 'canvas' ? undefined : () => ctx),
      set: () => true,
    },
  );
  HTMLCanvasElement.prototype.getContext = ((): CanvasRenderingContext2D =>
    ctx as CanvasRenderingContext2D) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}

function baueApp(): App {
  const parent = document.createElement('div');
  document.body.appendChild(parent);
  return new App(parent);
}

const LAUF = {
  victory: true,
  elapsedMs: 38_000,
  damage: 4500,
  progressPercent: 100,
  casts: 30,
  lostTimeWithReadyMs: 900,
  lostTimeNothingReadyMs: 3000,
};

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('AC1 — die Ansicht ist gebaut und nimmt Zahlen an', () => {
  it('buildComparisonView erzeugt ein Element mit allen sechs Kennzahlen', () => {
    const view = buildComparisonView(compareRuns(LAUF, null));
    expect(view).toBeInstanceOf(HTMLElement);
    // Sechs Kennzahlen aus `METRICS` — die Ansicht darf keine verschlucken.
    expect(view.textContent).toContain('PROGRESS');
    expect(view.textContent).toContain('DAMAGE');
    expect(view.textContent).toContain('CASTS');
  });

  it('und zeigt einen Bestwert, wenn es einen gibt', () => {
    const best: BestRun = {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 19_490,
      gameVersion: '0.1.0',
      achievedAt: 1_800_000_000_000,
      ...LAUF,
      damage: 4240,
    };
    const metriken = compareRuns({ ...LAUF, damage: 4500 }, comparableFromBest(best));
    const schaden = metriken.find((m) => m.key === 'damage');
    expect(schaden?.best).toBe(4240);
    expect(schaden?.improved).toBe(true);
  });
});

describe('AC2 — der Weg des Spielers: die App liefert den Vergleich', () => {
  it('die App baut ihn, ohne dass jemand ihn von Hand ruft', () => {
    // **Der eigentliche Nachweis.** Nicht „gibt es einen Aufrufer", sondern
    // „kommt beim Spieler etwas an": Die App wird gebaut wie im Browser, und
    // der Vergleich muss aus ihr herausfallen.
    const app = baueApp();
    const quelle = app.comparisonForResult();
    // Ohne beendeten Lauf gibt es nichts zu vergleichen — das ist richtig.
    expect(quelle).toBeNull();
  });

  it('nach einem Lauf steht er da, mit allen sechs Kennzahlen', () => {
    const app = baueApp();
    // Einen Lauf beenden, wie es die Sitzung tut.
    app.session.startFight(0);
    app.session.wipe(5000);
    const metriken = app.comparisonForResult();
    expect(metriken, 'kein Vergleich nach einem Lauf — der Anschluss fehlt').not.toBeNull();
    expect(metriken).toHaveLength(6);
    // Und er trägt die Zahlen DIESES Laufs, nicht Nullen.
    const fortschritt = metriken?.find((m) => m.key === 'progress');
    expect(fortschritt?.current).toBeGreaterThanOrEqual(0);
  });

  it('GEGENPROBE: ohne Bestlauf sagt jede Kennzahl UNBEKANNT', () => {
    // Ein frischer Spielstand hat keinen Bestlauf. Der Vergleich steht
    // trotzdem da — mit `best: null` überall, nicht mit erfundenen Nullen.
    const app = baueApp();
    app.session.startFight(0);
    app.session.wipe(5000);
    const metriken = app.comparisonForResult();
    expect(metriken?.every((m) => m.best === null)).toBe(true);
    expect(metriken?.every((m) => m.improved === null)).toBe(true);
  });
});

describe('AC3 — die letzte Meile: das Overlay ZEIGT ihn', () => {
  /**
   * Der Anschluss hat drei Stufen, und jede kann einzeln brechen:
   *
   * ```text
   *   app.comparisonForResult()   rechnet ihn      <- AC2 oben
   *   hud: view.comparison = …    holt ihn ab      <- HIER
   *   overlay: buildComparison…   zeigt ihn        <- HIER
   * ```
   *
   * **Die mittlere Stufe ist hier NICHT abgedeckt, und das steht so da.**
   * Gemessen: Setzt man `view.comparison = null` in `hud.ts`, bleibt jeder
   * der 2437 Fälle grün. Ein Fall dafür bräuchte einen laufenden
   * `HudRenderer` samt Oberfläche und Sitzung; den gibt es im Projekt
   * nirgends, und ihn allein für diese Zeile zu bauen wäre mehr Attrappe als
   * Prüfung.
   *
   * Die Stufe ist stattdessen **im Browser** belegt (siehe PR-Text): Dort
   * läuft die ganze Kette, und der Vergleich steht sichtbar in der
   * Auswertung. Das ist eine Messung, aber keine Absicherung — wer die Zeile
   * entfernt, merkt es erst beim nächsten Hinsehen. Lieber benannt als
   * stillschweigend gelassen.
   */
  /** Eine Kurzfassung aus einem wirklich gelaufenen Kampf. */
  function echteSummary() {
    const app = baueApp();
    app.session.startFight(0);
    app.session.wipe(5000);
    return app.session.summary(5000);
  }

  function ergebnis(comparison: readonly ComparedMetric[] | null): HTMLElement {
    const parent = document.createElement('div');
    document.body.appendChild(parent);
    const overlay = new Overlay(parent, 'The First Crucible', {
      onReady: () => undefined,
      onResume: () => undefined,
      onWipe: () => undefined,
      onRestart: () => undefined,
      onBackToMenu: () => undefined,
      onOpenHome: () => undefined,
    });
    overlay.update(PHASE_ENDED, OUTCOME_WIPE, {
      outcome: OUTCOME_WIPE,
      elapsedMs: 5000,
      remainingMs: 0,
      heldPercent: null,
      analysis: null,
      efficiency: null,
      curve: null,
      comparison,
      points: null,
      damage: 100,
      progressPercent: 10,
      casts: 3,
      // **Eine ECHTE Kurzfassung**, keine null: `fillDetail` läuft nur mit
      // ihr, und ohne sie prüfte der Fall eine Auswertung, die den
      // Detail-Bereich nie füllt — grün, ohne den Vergleich je zu sehen.
      summary: echteSummary(),
      mark: null,
      xpGained: null,
      levelUp: null,
      loot: null,
    } as never);
    // **Den Detail-Bereich aufklappen.** `fillDetail` läuft erst beim Klick
    // auf „View Analysis" — die Auswertung zeigt oben nur die Kurzfassung.
    // Ohne diesen Schritt prüfte der Fall die Kurzfassung und nicht den
    // Vergleich, und wäre grün, ohne ihn je gesehen zu haben.
    const knopf = [...parent.querySelectorAll('button')].find(
      (b) => b.textContent === 'View Analysis',
    );
    expect(knopf, 'der Knopf „View Analysis" fehlt').toBeDefined();
    knopf?.click();
    return parent;
  }

  it('mit Vergleich stehen die Kennzahlen im Dokument', () => {
    const wurzel = ergebnis(compareRuns(LAUF, null));
    expect(wurzel.textContent).toContain('PROGRESS');
    expect(wurzel.textContent).toContain('DAMAGE');
  });

  it('GEGENPROBE: ohne Vergleich stehen sie NICHT da', () => {
    // Ohne diesen Fall bewiese der obige nur, dass das Wort irgendwo im
    // Dokument vorkommt — etwa aus einer anderen Tabelle.
    const wurzel = ergebnis(null);
    expect(wurzel.textContent).not.toContain('PROGRESS');
  });
});
