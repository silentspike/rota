// @vitest-environment jsdom

/**
 * Kommt ein Archiv-Eintrag beim Spieler an? (Issue #85, GAME-DESIGN 17.4)
 *
 * # Der Befund, aus dem diese Datei entstand
 *
 * `archive.ts`, `archive-view.ts` und `help.ts` standen fertig gebaut da und
 * hatten **keinen Aufrufer** — sie waren namentlich in
 * `kein-unerreichbarer-code.test.ts` als bekannt-unerreichbar geführt, mit der
 * Begründung „es fehlt nicht der Aufruf, sondern die Sache".
 *
 * Das stimmte: Es gab keinen einzigen `ArchiveEntry` und keinen Ort im
 * Spielstand für `unlockedEntryIds`.
 *
 * # Was hier NICHT entsteht
 *
 * Die ~150 echten Texte. Sie sind ausdrücklich E10 (Issue-Text und
 * GAME-DESIGN 2620). Die Einträge in `data/archive/test-entries.json` sind
 * **Prüfdaten** in der Bauart von `test-set` und `test-channel`: erkennbar
 * Prüfung, über das Präfix `test-` von der ausgelieferten Liste getrennt.
 *
 * # Ein flüchtiger roter Lauf, und was die Nachmessung ergab
 *
 * Am 2026-08-13 fiel EIN Fall aus der vollen Suite (1 von 2491), ohne dass
 * der Name erfasst wurde — der Lauf lief ohne Umleitung, die Ausgabe war weg.
 * Systemlast zu dem Zeitpunkt: **36,31**.
 *
 * Nachgemessen wurde diese Datei zusammen mit den zwei anderen jsdom-Dateien,
 * zwanzig Läufe: **alle 32 grün, rc=0**, Last 12,0 bis 15,4. Der Orchestrator
 * fuhr fünf eigene Läufe bei Last 20,7, ebenfalls sauber.
 *
 * **Das entlastet nicht, es grenzt nur ein.** Niemand hat bei Last 36
 * gemessen; zwanzig grüne Läufe bei einem Drittel davon belegen nicht, dass
 * es dort hält. Wer hier wieder einen roten Fall sieht: Er ist der Verdacht,
 * und diesmal gehört der NAME in die Meldung.
 *
 * Diese Datei ist die schwerste der drei (3,8 s gegen 3,4 und 3,3) — sie baut
 * in mehreren Fällen eine ganze `App` unter jsdom.
 *
 * # Die Regel aus 17.4, die hier gemessen wird
 *
 * „Vor jeder Instanz und nach jedem **ersten** Boss-Sieg ein kurzer Text —
 * danach nie wieder automatisch, aber jederzeit im Archiv nachlesbar."
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { recordRun, type RecordedState } from '../../src/analysis/run-recorder.js';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { OUTCOME_VICTORY, OUTCOME_WIPE } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { ALL_ARCHIVE_ENTRIES, PLAYABLE_ARCHIVE_ENTRIES } from '../../src/analysis/archive-data.js';
import {
  ARCHIVE_VOICES,
  buildArchive,
  nextAutomaticEntry,
  unlock,
} from '../../src/analysis/archive.js';

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

beforeEach(() => {
  stubbeCanvas();
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('AC0 — ZUSICHERUNG: es gibt Einträge, und sie sind Prüfdaten', () => {
  it('vier Einträge, alle mit dem Präfix `test-`', () => {
    expect(ALL_ARCHIVE_ENTRIES.length).toBeGreaterThanOrEqual(4);
    expect(ALL_ARCHIVE_ENTRIES.every((e) => e.id.startsWith('test-'))).toBe(true);
  });

  it('und die ausgelieferte Liste ist deshalb LEER', () => {
    // Kein Fehler, sondern der Stand: Der Katalog-Inhalt gehört zu E10. Diese
    // Zeile fällt, sobald er kommt — und dann ist sie der Anlass, den
    // Vermerk in `archive-data.ts` zu prüfen.
    expect(PLAYABLE_ARCHIVE_ENTRIES).toHaveLength(0);
  });

  it('alle vier Stimmen aus 17.4 sind vertreten', () => {
    // Sonst prüfte die Ansicht nur eine Sorte, und die Gliederung nach Stimme
    // wäre unbelegt.
    const stimmen = new Set(ALL_ARCHIVE_ENTRIES.map((e) => e.voice));
    for (const stimme of ARCHIVE_VOICES) {
      expect(stimmen, `die Stimme ${stimme} fehlt`).toContain(stimme);
    }
  });
});

describe('AC1 — ein Eintrag erscheint EINMAL, danach nie wieder', () => {
  const leer = { unlockedEntryIds: [] as readonly string[] };
  const kontext = {
    trigger: 'afterFirstKill' as const,
    instanceId: 'instance-1',
    bossId: 'boss-1-1',
  };

  it('beim ersten Sieg steht er da', () => {
    const eintrag = nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, leer, kontext);
    expect(eintrag, 'kein Eintrag beim ersten Sieg').not.toBeNull();
    expect(eintrag?.bossId).toBe('boss-1-1');
  });

  it('beim ZWEITEN nicht mehr', () => {
    const erster = nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, leer, kontext);
    const danach = unlock(leer, erster?.id ?? '');
    expect(nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, danach, kontext)).toBeNull();
  });

  it('aber im Archiv ist er weiterhin auffindbar', () => {
    // Der zweite Halbsatz aus 17.4: „danach nie wieder automatisch, aber
    // jederzeit im Archiv nachlesbar." Ohne diesen Fall bewiese der obige
    // nur, dass der Eintrag verschwindet.
    const erster = nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, leer, kontext);
    const danach = unlock(leer, erster?.id ?? '');
    const abschnitte = buildArchive(ALL_ARCHIVE_ENTRIES, danach);
    // `buildArchive` zeigt NUR Freigeschaltetes — ein gesperrter Eintrag
    // erscheint nicht einmal als graue Zeile, weil das eine Ankündigung wäre.
    const gefunden = abschnitte.flatMap((x) => x.entries).some((e) => e.id === erster?.id);
    expect(gefunden, 'der freigeschaltete Eintrag fehlt im Archiv').toBe(true);
  });

  it('ein Sieg über einen ANDEREN Boss gibt einen anderen Eintrag', () => {
    // Die Zuordnung: `nextAutomaticEntry` vergleicht `bossId`. Ohne diesen
    // Fall wäre nicht belegt, dass sie greift — ein Eintrag für jeden Boss
    // sähe genauso aus.
    const eins = nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, leer, kontext);
    const zwei = nextAutomaticEntry(ALL_ARCHIVE_ENTRIES, leer, {
      ...kontext,
      bossId: 'boss-1-2',
    });
    expect(zwei).not.toBeNull();
    expect(zwei?.id).not.toBe(eins?.id);
  });
});

describe('AC2 — die Ansicht ist im Spiel erreichbar', () => {
  it('das Player Home führt einen Abschnitt „Archive"', () => {
    // **Der Anschluss.** `buildArchiveView` hatte keinen Aufrufer; diese Zeile
    // ist der Unterschied zwischen gebaut und erreichbar.
    const app = baueApp();
    expect(app).toBeDefined();
    const text = document.body.textContent ?? '';
    expect(text, 'der Abschnitt Archive fehlt im Player Home').toContain('ARCHIVE');
  });

  it('bei frischem Stand steht dort der leere Hinweis — nicht ein Eintrag', () => {
    // **Und das ist richtig so**: Das Archiv zeigt nur Freigeschaltetes. Ein
    // gesperrter Eintrag als graue Zeile wäre eine Ankündigung von Inhalten,
    // die der Spieler noch nicht erreicht hat.
    const app = baueApp();
    expect(app).toBeDefined();
    const text = document.body.textContent ?? '';
    expect(text).toContain('Nothing recorded yet');
    expect(text).not.toContain('Test Entry');
  });

  it('nach einer Freischaltung steht der Eintrag im Dokument', () => {
    // Der eigentliche Nachweis: Die Ansicht liest den Spielstand und zeigt,
    // was darin steht. Ohne den Anschluss in `app.ts` bliebe es beim Hinweis.
    const erster = ALL_ARCHIVE_ENTRIES[0];
    expect(erster).toBeDefined();
    const app = baueApp();
    app.persistence.persistUnlockedEntries([erster?.id ?? '']);

    // Neu laden heisst: neue App auf demselben Spielstand.
    document.body.innerHTML = '';
    const zweite = baueApp();
    expect(zweite).toBeDefined();
    const text = document.body.textContent ?? '';
    expect(text, 'der freigeschaltete Eintrag steht nicht im Dokument').toContain('Test Entry');
  });
});

describe('AC3 — der Spielstand führt die Kennungen und wandert', () => {
  it('ein frischer Stand beginnt mit einer leeren Liste', () => {
    const app = baueApp();
    expect(app.persistence.state.unlockedEntryIds).toEqual([]);
  });

  it('und ein Stand OHNE das Feld lädt trotzdem', () => {
    // Die Fassungs-Wanderung. Ein Spielstand von vor #85 hat das Feld nicht;
    // ohne die Zeile in `run-persistence.ts` stünde dort `undefined`, und der
    // erste Zugriff auf `.includes` bräche den Ladevorgang.
    //
    // Der Fall steht hier zusätzlich zu `save-shape.test.ts`, der ihn
    // mechanisch für JEDES Feld fährt: Dieser hier nennt die Sache beim
    // Namen, jener fängt das nächste Feld ohne Pflege.
    const app = baueApp();
    expect(() => app.persistence.state.unlockedEntryIds.includes('x')).not.toThrow();
  });
});

describe('AC4 — der SIEG schaltet frei, nicht die Ansicht', () => {
  /**
   * Die Stelle, die zuerst ungeschützt war.
   *
   * `recordRun` schreibt `unlockedEntryIds` fort. Gemessen: Reicht man dort
   * statt des neuen Standes den alten durch, blieb jeder Fall grün — die
   * Freischaltung war von nichts gehalten. Dieselbe Klasse wie
   * `setSetEffects` (#794) und `view.comparison` (#82), zum dritten Mal an
   * diesem Tag.
   */
  const LEER: RecordedState = {
    runHistory: [],
    progressMarks: [],
    bestRuns: [],
    talentPoints: {},
    items: [],
    defeatedBosses: [],
    unlockedEntryIds: [],
    seenMechanicIds: [],
    level: 1,
    experience: 0,
  };

  /** Der Boss, an dem ein Prüfeintrag hängt. */
  const KONTEXT = {
    bossId: 'boss-1-1',
    difficulty: 1,
    bossLevel: 1,
    gameVersion: '0.1.0',
    at: 1_800_000_000_000,
    seed: 0,
    instance: 1,
    endlessLevel: 0,
  };

  /**
   * Eine echte Kurzfassung, deren `victory` der Fall selbst setzt.
   *
   * Einen Sieg wirklich zu erspielen wäre hier Beiwerk: Geprüft wird, was
   * `recordRun` mit dem Ausgang macht, nicht wie der Ausgang entsteht.
   */
  function kurzfassung() {
    const boss = bossById('boss-1-1') as BossDef;
    const combat = new Combat({ school: CALCINATION.value, boss });
    const session = new Session(new PressQueue(), combat);
    session.startFight(0);
    return session.summary(1000);
  }

  it('ein SIEG schaltet den Eintrag des Bosses frei', () => {
    const summary = { ...kurzfassung(), victory: true };
    const ergebnis = recordRun(LEER, summary, OUTCOME_VICTORY, KONTEXT);
    expect(ergebnis.unlockedEntryIds, 'der Sieg hat nichts freigeschaltet').toHaveLength(1);
    expect(ergebnis.unlockedEntryIds[0]).toBe('test-after-kill');
  });

  it('eine NIEDERLAGE schaltet nichts frei', () => {
    // 17.4 sagt „nach jedem ersten Boss-SIEG". Eine Niederlage ist keiner.
    const summary = { ...kurzfassung(), victory: false };
    const ergebnis = recordRun(LEER, summary, OUTCOME_WIPE, KONTEXT);
    expect(ergebnis.unlockedEntryIds).toEqual([]);
  });

  it('der ZWEITE Sieg schaltet nichts Neues frei', () => {
    // Der Kern von 17.4: einmal automatisch, danach nie wieder.
    const summary = { ...kurzfassung(), victory: true };
    const erster = recordRun(LEER, summary, OUTCOME_VICTORY, KONTEXT);
    const zweiter = recordRun(
      { ...LEER, unlockedEntryIds: erster.unlockedEntryIds },
      summary,
      OUTCOME_VICTORY,
      KONTEXT,
    );
    expect(zweiter.unlockedEntryIds).toEqual(erster.unlockedEntryIds);
  });

  it('ein Sieg über einen ANDEREN Boss schaltet dessen Eintrag frei', () => {
    // Die Zuordnung über `bossId` — ohne sie schaltete jeder Sieg denselben
    // Eintrag frei, und der Fall oben bliebe trotzdem grün.
    const summary = { ...kurzfassung(), victory: true };
    const ergebnis = recordRun(LEER, summary, OUTCOME_VICTORY, {
      ...KONTEXT,
      bossId: 'boss-1-2',
    });
    expect(ergebnis.unlockedEntryIds).toEqual(['test-device-log']);
  });
});

describe('AC5 — die zwei Stellen, die der Browser gefunden hat', () => {
  /**
   * Beide waren von KEINEM Fall gehalten, und beide fielen erst im Browser auf.
   *
   * ```text
   *   recordRun rechnet die Freischaltung   ->  war geprüft
   *   persist legt sie ab                   ->  fehlte, 2489 Fälle grün
   *   refreshHome zeigt sie                 ->  fehlte, 2489 Fälle grün
   * ```
   *
   * Der Befund im Browser war eindeutig: Boss gefallen, `defeatedBosses`
   * gesetzt, `unlockedEntryIds` **leer**. Und nachdem die Ablage stand: im
   * Stand gesetzt, Archiv trotzdem leer — bis zum Neuladen.
   */
  const EINTRAG = 'test-prologue-kill';

  it('persist legt die freigeschalteten Kennungen wirklich ab', () => {
    const app = baueApp();
    const summary = { ...kurzfassungFuer('first-crucible'), victory: true };
    const ergebnis = recordRun(app.persistence.state, summary, OUTCOME_VICTORY, {
      bossId: 'first-crucible',
      difficulty: 1,
      bossLevel: 1,
      gameVersion: '0.1.0',
      at: 1_800_000_000_000,
      seed: 0,
      instance: 1,
      endlessLevel: 0,
    });
    expect(ergebnis.unlockedEntryIds, 'recordRun hat nichts freigeschaltet').toContain(EINTRAG);

    app.persistence.persist(ergebnis);
    // **Aus dem abgelegten Stand gelesen, nicht aus dem Rückgabewert.** Genau
    // hier lag die Lücke: `recordRun` rechnete richtig, und `persist` nahm das
    // Feld nicht an.
    expect(app.persistence.state.unlockedEntryIds).toContain(EINTRAG);
  });

  it('und das Player Home zeigt ihn ohne Neuladen', () => {
    const app = baueApp();
    // Vorher steht dort der leere Hinweis.
    expect(document.body.textContent ?? '').toContain('Nothing recorded yet');

    app.persistence.persistUnlockedEntries([EINTRAG]);
    app.openHome();

    const text = document.body.textContent ?? '';
    expect(text, 'das Archiv wurde beim Öffnen nicht aufgefrischt').toContain('Prologue Victory');
  });
});

/** Eine Kurzfassung für einen bestimmten Boss. */
function kurzfassungFuer(bossId: string) {
  const boss = bossById(bossId) as BossDef;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(new PressQueue(), combat);
  session.startFight(0);
  return session.summary(1000);
}
