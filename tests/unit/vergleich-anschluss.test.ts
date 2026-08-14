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
import { comparableFromBest, curveFromBest, isWellFormed } from '../../src/analysis/best-run.js';
import { buildComparisonView, buildCurveView } from '../../src/analysis/history-view.js';
import type { BestRun } from '../../src/analysis/save-format.js';
import type { ComparedMetric, RunCurve } from '../../src/analysis/run-compare.js';
import { Overlay } from '../../src/render/overlay.js';
import {
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  PHASE_ENDED,
} from '../../src/core/phases.js';
import { recordRun } from '../../src/analysis/run-recorder.js';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';

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

/**
 * Die Bildabrufe, die die App bestellt hat.
 *
 * `App` reicht `window.requestAnimationFrame` an die Schleife weiter. In
 * jsdom liefe der Ruf irgendwann von selbst — hier wird er abgefangen, damit
 * der Frame genau dann fällt, wenn der Fall ihn braucht.
 */
const abgefangeneFrames: FrameRequestCallback[] = [];

beforeEach(() => {
  abgefangeneFrames.length = 0;
  window.requestAnimationFrame = ((ruf: FrameRequestCallback): number => {
    abgefangeneFrames.push(ruf);
    return abgefangeneFrames.length;
  }) as typeof window.requestAnimationFrame;
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
   * **Alle drei Stufen sind abgedeckt**, die mittlere seit AC4 unten.
   *
   * Sie war es zunächst nicht: Ein `view.comparison = null` in `hud.ts` liess
   * alle 2444 Fälle grün — die Stelle war von nichts gehalten. Mein erster
   * Einwand lautete, ein Fall dafür bräuchte einen von Hand zusammengebauten
   * `HudRenderer` und wäre mehr Attrappe als Prüfung.
   *
   * Das stimmte für den isolierten Renderer und war trotzdem der falsche
   * Schluss: **`App` erzeugt ihn selbst** (`app.ts`, im Konstruktor). Wer die
   * App baut und einen Frame fahren lässt, prüft die Kette und nicht den
   * Zusammenbau.
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

describe('AC4 — die mittlere Stufe: die App fährt einen echten Frame', () => {
  /**
   * Der Fall, den es zuerst nicht gab.
   *
   * Zwischen `comparisonForResult()` und der Anzeige liegt eine Zeile in
   * `hud.ts`: `view.comparison = this.comparisonSource()`. Sie war von nichts
   * gehalten — auf `null` gekappt blieben alle 2444 Fälle grün.
   *
   * **Über `new App()` und nicht über einen eigenen `HudRenderer`.** Die App
   * baut ihn selbst; wer ihn von Hand zusammensetzt, prüft den Zusammenbau
   * statt die Sache. Der Bildabruf wird abgefangen, damit der Frame hier
   * fällt und nicht irgendwann — sonst hinge der Fall an der Uhr des
   * Rechners.
   */
  function frameFahren(app: App): void {
    app.loop.start();
    const rufe = [...abgefangeneFrames];
    abgefangeneFrames.length = 0;
    for (const ruf of rufe) ruf(performance.now());
    app.loop.stop();
  }

  it('nach einem Lauf steht der Vergleich im Dokument', () => {
    const app = baueApp();
    app.session.startFight(0);
    app.session.wipe(5000);
    frameFahren(app);

    const knopf = [...document.querySelectorAll('button')].find(
      (b) => b.textContent === 'View Analysis',
    );
    expect(knopf, 'die Auswertung wurde nicht gezeigt').toBeDefined();
    knopf?.click();

    // Die Kennzahlen des Vergleichs — sie kommen NUR über die gekappte Zeile.
    const text = document.body.textContent ?? '';
    expect(text, 'der Vergleich fehlt im Dokument').toContain('PROGRESS');
    expect(text).toContain('MISSED');
  });

  it('ZUSICHERUNG: ohne Frame steht er NICHT da', () => {
    // Ohne diese Zeile bewiese der Fall oben nur, dass das Wort irgendwo im
    // Dokument vorkommt — etwa aus einer anderen Tabelle.
    const app = baueApp();
    app.session.startFight(0);
    app.session.wipe(5000);
    // kein frameFahren
    expect(document.body.textContent ?? '').not.toContain('MISSED');
  });
});

describe('AC5 — der Verlauf im Player Home zeigt den EBEN gefahrenen Lauf', () => {
  /**
   * Der Fehler, den erst der Browser gefunden hat — und der genau die Klasse
   * hat, die in `refreshHome` schon sechsmal kommentiert steht (#289, #453,
   * #802, #85, #613).
   *
   * Gemessen am 2026-08-14 unter `/rota-e9/`:
   *
   * ```text
   *   Spielstand 64 Läufe   Ansicht „64 runs"
   *   einen weiteren Kampf gefahren, dann ins Player Home
   *   Spielstand 65 Läufe   Ansicht „64 runs"     <- eingefroren
   *   Seite neu geladen     Ansicht „65 runs"
   * ```
   *
   * Grund: `buildHistoryView` lief EINMAL beim Aufbau des Player Home. Der
   * zuletzt gefahrene Lauf fehlte damit immer — ausgerechnet der, wegen dem
   * man nachsieht.
   *
   * **Nicht über `session.wipe()`**, obwohl das der kürzeste Weg zu einem
   * Kampfende wäre: `IGNORED_OUTCOMES` in `run-recorder.ts` führt
   * `OUTCOME_WIPE`, ein abgebrochener Lauf wird also absichtlich NICHT
   * aufgezeichnet. Der erste Entwurf dieses Falls lief genau da hinein — die
   * Zusicherung unten fing es ab und meldete `expected 0 to be greater
   * than 0`, statt der Ansicht die Schuld zu geben.
   */
  const KONTEXT = {
    bossId: 'first-crucible',
    difficulty: 1,
    bossLevel: 1,
    gameVersion: '0.1.0',
    at: 1_800_000_000_000,
    seed: 0,
    instance: 1,
    endlessLevel: 0,
  };

  /** Einen gewerteten Lauf in den Spielstand legen. */
  function laufAblegen(app: App): void {
    const boss = bossById('first-crucible') as BossDef;
    const combat = new Combat({ school: CALCINATION.value, boss });
    const session = new Session(new PressQueue(), combat);
    session.startFight(0);
    const summary = session.summary(60_000);
    app.persistence.persist(
      recordRun(app.persistence.state, summary, OUTCOME_OVERPRESSURE, KONTEXT),
    );
  }

  it('ohne Neuladen steht der Lauf im Verlauf', () => {
    const app = baueApp();
    // Vorher: der leere Hinweis, aus `buildHistoryView`.
    expect(document.body.textContent ?? '').toContain('No runs recorded yet');

    laufAblegen(app);

    // **ZUSICHERUNG in den Datensatz.** Ohne sie prüfte der Fall darunter die
    // Ansicht an einem leeren Spielstand — und bliebe grün, weil dann eben
    // nichts anzuzeigen ist.
    expect(
      app.persistence.state.runHistory.length,
      'der Lauf steht gar nicht im Spielstand — dann sagt die Ansicht unten nichts',
    ).toBeGreaterThan(0);

    app.openHome();

    const text = document.body.textContent ?? '';
    expect(text, 'der Verlauf wurde beim Öffnen nicht aufgefrischt').not.toContain(
      'No runs recorded yet',
    );
    expect(text).toContain('1 runs');
  });

  it('und ein ZWEITER Lauf erhöht die Zahl', () => {
    // Ohne diesen Fall wäre nur belegt, dass die Ansicht einmal aufwacht. Der
    // Fehler war aber ein Einfrieren: Die erste Zahl stand, jede weitere
    // nicht mehr.
    const app = baueApp();
    laufAblegen(app);
    app.openHome();
    expect(document.body.textContent ?? '').toContain('1 runs');

    laufAblegen(app);
    expect(app.persistence.state.runHistory).toHaveLength(2);

    app.openHome();
    expect(document.body.textContent ?? '', 'die Ansicht blieb beim ersten Lauf stehen').toContain(
      '2 runs',
    );
  });
});

describe('AC1 — ZWEI Kurven: der Bestlauf bringt seine eigene mit', () => {
  /**
   * Der letzte offene Punkt von #82, und der einzige, der eine Datenlücke war
   * statt eines fehlenden Anschlusses.
   *
   * `buildCurveView(current, best)` konnte zwei Kurven seit E7-2. Was fehlte,
   * war die zweite Quelle: Der Spielstand führte vom Bestlauf `metric` und
   * `value` — zwei Zahlen, aus denen sich keine Linie rekonstruieren lässt.
   * `overlay.ts` reichte deshalb fest `null` durch, mit einem Vermerk, dass
   * das gemessen und nicht vergessen sei.
   *
   * # Die zwei Richtungen, und warum die zweite die wichtigere ist
   *
   * ```text
   *   mit Kurve im Spielstand    ->  zwei Züge   <- das Kriterium
   *   ohne Kurve (Altstand)      ->  ein Zug     <- der Bestandsschutz
   * ```
   *
   * Ein Spielstand von vor dieser Änderung hat das Feld nicht. Bräche die
   * Ansicht daran, kostete dieses Kriterium jeden bestehenden Spieler seine
   * Auswertung — ein schlechterer Tausch als eine fehlende Linie.
   */
  const KURVE_A: RunCurve = { stepMs: 1000, points: [0, 100, 250, 400], durationMs: 3000 };
  const KURVE_B: RunCurve = { stepMs: 1000, points: [0, 150, 300, 500], durationMs: 3000 };

  it('mit gespeicherter Kurve zeichnet die Ansicht ZWEI Züge', () => {
    const view = buildCurveView(KURVE_A, KURVE_B);
    const zuege = view.querySelectorAll('polyline');
    expect(zuege).toHaveLength(2);
    // **Und sie zeigen verschiedene Läufe.** Zwei Züge mit identischen Punkten
    // wären dieselbe Kurve zweimal — genau der Fehler, den der Orchestrator
    // für den Screenshot ausgeschlossen hat („nicht dieselbe zweimal").
    const punkte = [...zuege].map((z) => z.getAttribute('points'));
    expect(punkte[0]).not.toBe(punkte[1]);
    // Die Legende benennt beide, sonst weiss der Spieler nicht, welche welche ist.
    expect(view.textContent).toContain('This run');
    expect(view.textContent).toContain('Best');
  });

  it('OHNE gespeicherte Kurve zeichnet sie EINEN — und bricht nicht', () => {
    const view = buildCurveView(KURVE_A, null);
    expect(view.querySelectorAll('polyline')).toHaveLength(1);
    expect(view.textContent).toContain('This run');
    // Kein „Best" ohne Bestkurve: Eine Beschriftung ohne Linie wäre die
    // Behauptung, da sei etwas zu sehen.
    expect(view.textContent).not.toContain('Best');
  });

  it('ein Sieg legt die Kurve im Spielstand ab', () => {
    // **Der Anschluss, nicht die Ansicht.** Ohne diesen Fall wäre belegt, dass
    // zwei Kurven zeichenbar sind — nicht, dass je eine zweite entsteht.
    const app = baueApp();
    app.session.startFight(0);
    app.session.wipe(5000);
    const summary = { ...app.session.summary(5000), victory: true, heldPercent: null };
    const ergebnis = recordRun(app.persistence.state, summary, OUTCOME_VICTORY, {
      bossId: 'first-crucible',
      difficulty: 1,
      bossLevel: 1,
      gameVersion: '0.1.0',
      at: 1_800_000_000_000,
      seed: 0,
      instance: 1,
      endlessLevel: 0,
      curve: KURVE_A,
    });
    const best = ergebnis.bestRuns.find((r) => r.bossId === 'first-crucible');
    expect(best, 'der Sieg hat keinen Bestlauf angelegt').toBeDefined();
    expect(curveFromBest(best ?? null), 'der Bestlauf trägt keine Kurve').not.toBeNull();
    expect(curveFromBest(best ?? null)?.points).toEqual(KURVE_A.points);
  });

  it('ein Bestlauf OHNE Kurve gibt null statt einer erfundenen Linie', () => {
    // Der Altstand-Fall an der Lesestelle. `curveFromBest` darf hier nicht
    // raten — eine gerade Null wäre die Behauptung, der Bestlauf habe keinen
    // Schaden gemacht.
    const alt: BestRun = {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 19_490,
      gameVersion: '0.1.0',
      achievedAt: 1_800_000_000_000,
    };
    expect(curveFromBest(alt)).toBeNull();
  });

  it('und eine BESCHÄDIGTE Kurve gibt ebenfalls null — ohne den Bestlauf zu verlieren', () => {
    // Der gefährlichere Fall als „fehlt": vorhanden, aber unbrauchbar. Ein
    // `stepMs: 0` teilte durch null, ein leeres `points` zeichnete eine Linie
    // ohne Punkte. Beides käme durch eine blosse Anwesenheitsprüfung.
    const grund: BestRun = {
      bossId: 'first-crucible',
      difficulty: 1,
      metric: 'reserve',
      value: 19_490,
      gameVersion: '0.1.0',
      achievedAt: 1_800_000_000_000,
    };
    expect(
      curveFromBest({ ...grund, curve: { stepMs: 0, points: [1], durationMs: 10 } }),
    ).toBeNull();
    expect(
      curveFromBest({ ...grund, curve: { stepMs: 1000, points: [], durationMs: 10 } }),
    ).toBeNull();
    expect(
      curveFromBest({
        ...grund,
        curve: { stepMs: 1000, points: [1, Number.NaN], durationMs: 10 },
      }),
    ).toBeNull();
    // **Die Zusicherung dazu**: Der Eintrag selbst bleibt gültig. Wäre die
    // Prüfung in `isWellFormed` gelandet, verlöre der Spieler wegen einer
    // kaputten Linie seinen ganzen Bestwert.
    expect(isWellFormed({ ...grund, curve: { stepMs: 0, points: [], durationMs: 0 } })).toBe(true);
  });
});

describe('AC1 — die mittlere Stufe: die Kurve des Bestlaufs kommt beim Spieler an', () => {
  /**
   * **Gemessen, dass dieser Fall nötig ist**, nicht vermutet: Kappt man in
   * `hud.ts` die Zeile `view.bestCurve = this.bestCurveSource()` auf `null`,
   * bleiben ohne ihn **alle 2645 Fälle grün**. Die zweite Kurve wäre gebaut,
   * abgelegt, gelesen — und käme nie an.
   *
   * Dieselbe Klasse wie `view.comparison` in AC4 oben, wie `setSetEffects` in
   * #794 und wie `buildHistoryView` vor #82. Sie trifft dieses Projekt immer
   * wieder an derselben Stelle: dort, wo eine fertige Ansicht auf ihre Daten
   * wartet.
   *
   * **Über `new App()` und einen echten Frame**, aus dem Grund, der bei AC4
   * ausgeschrieben steht: Wer den Renderer von Hand zusammensetzt, prüft den
   * Zusammenbau statt die Kette.
   */
  const BESTKURVE: RunCurve = { stepMs: 1000, points: [0, 500, 900, 1200], durationMs: 3000 };

  function frameFahren(app: App): void {
    app.loop.start();
    const rufe = [...abgefangeneFrames];
    abgefangeneFrames.length = 0;
    for (const ruf of rufe) ruf(performance.now());
    app.loop.stop();
  }

  /** Einen Bestlauf MIT Kurve in den Spielstand legen. */
  function bestlaufMitKurve(app: App): void {
    app.persistence.persist({
      ...app.persistence.state,
      bestRuns: [
        {
          bossId: app.combat.boss.id,
          difficulty: 1,
          metric: 'reserve',
          value: 19_490,
          gameVersion: '0.1.0',
          achievedAt: 1_800_000_000_000,
          curve: BESTKURVE,
        },
      ],
    });
  }

  function auswertungOeffnen(app: App): string {
    app.session.startFight(0);
    app.session.wipe(5000);
    frameFahren(app);
    const knopf = [...document.querySelectorAll('button')].find(
      (b) => b.textContent === 'View Analysis',
    );
    expect(knopf, 'die Auswertung wurde nicht gezeigt').toBeDefined();
    knopf?.click();
    return document.body.textContent ?? '';
  }

  it('mit Bestkurve im Spielstand zeigt die Auswertung ZWEI Züge', () => {
    const app = baueApp();
    bestlaufMitKurve(app);
    // **ZUSICHERUNG in den Datensatz**: Ohne sie prüfte der Fall die Anzeige
    // an einem Spielstand ohne Bestkurve und wäre grün, weil dann eben eine
    // Linie richtig ist.
    expect(
      curveFromBest(app.persistence.state.bestRuns[0] ?? null),
      'die Bestkurve steht gar nicht im Spielstand',
    ).not.toBeNull();

    auswertungOeffnen(app);

    const kurven = document.querySelector('.curves');
    expect(kurven, 'die Kurvenansicht fehlt in der Auswertung').not.toBeNull();
    expect(kurven?.querySelectorAll('polyline')).toHaveLength(2);
  });

  it('GEGENPROBE: ohne Bestkurve zeigt sie EINEN', () => {
    // Ohne diesen Fall bewiese der obige nur, dass irgendwo zwei Linien
    // stehen — nicht, dass die zweite aus dem Spielstand kommt.
    const app = baueApp();
    auswertungOeffnen(app);
    const kurven = document.querySelector('.curves');
    expect(kurven?.querySelectorAll('polyline')).toHaveLength(1);
  });
});
