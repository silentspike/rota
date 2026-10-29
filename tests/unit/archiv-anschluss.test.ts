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
 * # Katalog und Prüfdaten
 *
 * Bis #1177 entstanden hier keine echten Texte; die Fälle liefen über die
 * Prüfeinträge in `data/archive/test-entries.json`. Seitdem liest das Spiel
 * den Katalog `data/archive/archive.json`, und diese Fälle mit ihm — die
 * Prüfeinträge stehen nur noch im Entwicklungs-Bau (`TEST_ARCHIVE_ENTRIES`).
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
import { RunPersistence } from '../../src/analysis/run-persistence.js';
import { PLAYABLE_ARCHIVE_ENTRIES, TEST_ARCHIVE_ENTRIES } from '../../src/analysis/archive-data.js';
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

describe('AC0 — ZUSICHERUNG: es gibt Einträge, und Prüfdaten stehen getrennt', () => {
  it('die Prüfeinträge: mindestens vier, alle mit dem Präfix `test-`', () => {
    expect(TEST_ARCHIVE_ENTRIES.length).toBeGreaterThanOrEqual(4);
    expect(TEST_ARCHIVE_ENTRIES.every((e) => e.id.startsWith('test-'))).toBe(true);
  });

  it('die ausgelieferte Liste ist der Katalog — gefüllt und ohne Prüfeintrag', () => {
    // **Bis #1177 stand hier „und die ausgelieferte Liste ist deshalb LEER"**,
    // mit dem Vermerk, die Zeile falle, sobald der Katalog kommt, und sei dann
    // der Anlass, `archive-data.ts` zu prüfen. Er ist gekommen; Aufzeichnung
    // und Ansicht lesen seitdem `PLAYABLE_ARCHIVE_ENTRIES`. Was der Katalog
    // gegen den Plan trägt, misst `archiv-katalog.test.ts`.
    expect(PLAYABLE_ARCHIVE_ENTRIES.length).toBeGreaterThan(0);
    expect(PLAYABLE_ARCHIVE_ENTRIES.filter((e) => e.id.startsWith('test-'))).toEqual([]);
  });

  it('alle vier Stimmen aus 17.4 sind vertreten', () => {
    // Sonst prüfte die Ansicht nur eine Sorte, und die Gliederung nach Stimme
    // wäre unbelegt.
    const stimmen = new Set(PLAYABLE_ARCHIVE_ENTRIES.map((e) => e.voice));
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
    const eintrag = nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, leer, kontext);
    expect(eintrag, 'kein Eintrag beim ersten Sieg').not.toBeNull();
    expect(eintrag?.bossId).toBe('boss-1-1');
  });

  it('beim ZWEITEN nicht mehr', () => {
    const erster = nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, leer, kontext);
    const danach = unlock(leer, erster?.id ?? '');
    expect(nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, danach, kontext)).toBeNull();
  });

  it('aber im Archiv ist er weiterhin auffindbar', () => {
    // Der zweite Halbsatz aus 17.4: „danach nie wieder automatisch, aber
    // jederzeit im Archiv nachlesbar." Ohne diesen Fall bewiese der obige
    // nur, dass der Eintrag verschwindet.
    const erster = nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, leer, kontext);
    const danach = unlock(leer, erster?.id ?? '');
    const abschnitte = buildArchive(PLAYABLE_ARCHIVE_ENTRIES, danach);
    // `buildArchive` zeigt NUR Freigeschaltetes — ein gesperrter Eintrag
    // erscheint nicht einmal als graue Zeile, weil das eine Ankündigung wäre.
    const gefunden = abschnitte.flatMap((x) => x.entries).some((e) => e.id === erster?.id);
    expect(gefunden, 'der freigeschaltete Eintrag fehlt im Archiv').toBe(true);
  });

  it('ein Sieg über einen ANDEREN Boss gibt einen anderen Eintrag', () => {
    // Die Zuordnung: `nextAutomaticEntry` vergleicht `bossId`. Ohne diesen
    // Fall wäre nicht belegt, dass sie greift — ein Eintrag für jeden Boss
    // sähe genauso aus.
    const eins = nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, leer, kontext);
    const zwei = nextAutomaticEntry(PLAYABLE_ARCHIVE_ENTRIES, leer, {
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

  it('bei frischem Stand steht dort nur, was der Spieler kennt — kein Boss-Eintrag', () => {
    // **Und das ist richtig so**: Das Archiv zeigt nur Freigeschaltetes. Ein
    // gesperrter Eintrag als graue Zeile wäre eine Ankündigung von Inhalten,
    // die der Spieler noch nicht erreicht hat.
    //
    // Bis #1177 stand hier „der leere Hinweis". Seitdem kommen die Einträge
    // der Start-Zauber still ins Archiv (`archiv-funde.test.ts`) — leer ist
    // ein frisches Archiv also nicht mehr. Die Aussage bleibt dieselbe und
    // wird schärfer: kein Eintrag zu einem Boss, den niemand besiegt hat.
    const app = baueApp();
    expect(app).toBeDefined();
    const archiv = document.querySelector('.archive')?.textContent ?? '';
    const zumSieg = PLAYABLE_ARCHIVE_ENTRIES.filter((e) => e.trigger === 'afterFirstKill');
    expect(zumSieg.length).toBeGreaterThan(0);
    for (const eintrag of zumSieg) expect(archiv, eintrag.id).not.toContain(eintrag.body);
    expect(archiv).not.toContain('Test Entry');
  });

  it('nach einer Freischaltung steht der Eintrag im Dokument', () => {
    // Der eigentliche Nachweis: Die Ansicht liest den Spielstand und zeigt,
    // was darin steht. Ohne den Anschluss in `app.ts` bliebe es beim Hinweis.
    const erster = PLAYABLE_ARCHIVE_ENTRIES.find((e) => e.trigger === 'afterFirstKill');
    expect(erster).toBeDefined();
    const app = baueApp();
    app.persistence.persistUnlockedEntries([erster?.id ?? '']);

    // Neu laden heisst: neue App auf demselben Spielstand.
    document.body.innerHTML = '';
    const zweite = baueApp();
    expect(zweite).toBeDefined();
    const text = document.querySelector('.archive')?.textContent ?? '';
    expect(text, 'der freigeschaltete Eintrag steht nicht im Dokument').toContain(
      erster?.title ?? '<kein Eintrag>',
    );
  });
});

describe('AC3 — der Spielstand führt die Kennungen und wandert', () => {
  it('ein frischer Stand beginnt mit einer leeren Liste', () => {
    // **Der Spielstand selbst**, ohne App darum: leer. Bis #1177 stand hier
    // die App, und beides war dasselbe.
    const stand = new RunPersistence('0.0.0', () => 0);
    expect(stand.state.unlockedEntryIds).toEqual([]);
  });

  it('und die App legt darin nur ab, was der Spieler schon kennt', () => {
    // Seit #1177 kommen die Einträge der Start-Zauber und ihrer Schule still
    // ins Archiv, sobald die App steht (`archiv-funde.test.ts`). Die Aussage
    // von vorher bleibt erhalten und wird schärfer: Kein Eintrag zu einem Sieg,
    // einer Instanz oder einem Fund, den es nicht gab.
    const app = baueApp();
    const frei = new Set(app.persistence.state.unlockedEntryIds);
    const nichtErreicht = PLAYABLE_ARCHIVE_ENTRIES.filter(
      (e) => e.trigger !== 'spellLearned' && e.trigger !== 'schoolLearned' && frei.has(e.id),
    );
    expect(nichtErreicht.map((e) => e.id)).toEqual([]);
    expect(frei.size, 'ZUSICHERUNG: die Start-Zauber stehen drin').toBeGreaterThan(0);
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
    expect(ergebnis.unlockedEntryIds[0]).toBe('boss-1-1-first-kill');
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
    expect(ergebnis.unlockedEntryIds).toEqual(['boss-1-2-first-kill']);
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
  const EINTRAG = 'first-crucible-first-kill';
  const TITEL = 'A Crack in the Crucible';

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
    // Vorher steht er dort nicht.
    expect(document.querySelector('.archive')?.textContent ?? '').not.toContain(TITEL);

    app.persistence.persistUnlockedEntries([...app.persistence.state.unlockedEntryIds, EINTRAG]);
    app.openHome();

    const text = document.querySelector('.archive')?.textContent ?? '';
    expect(text, 'das Archiv wurde beim Öffnen nicht aufgefrischt').toContain(TITEL);
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

describe('AC1 von #85 — der Eintrag ERSCHEINT beim ersten Sieg, beim zweiten nicht', () => {
  /**
   * Die Hälfte von AC1, die bis zum 2026-08-14 fehlte.
   *
   * Gemessen im Browser: Ein Sieg schaltete frei (`unlockedEntryIds` gesetzt,
   * der Eintrag stand im Player Home) — aber es **erschien nichts**. AC1 sagt
   * „erscheint beim ersten Sieg automatisch".
   *
   * Der Grund war keine tote Ansicht (`buildEntry` wird von `buildArchiveView`
   * gerufen), sondern eine fehlende Auskunft: `nextAutomaticEntry` rechnete
   * den neuen Eintrag aus, `recordRun` benutzte ihn zum Fortschreiben der
   * Liste und **verwarf ihn dann**. Die Anzeige konnte nicht wissen, welcher
   * Eintrag gerade dazukam — `unlockedEntryIds` ist der Zustand, nicht die
   * Differenz.
   *
   * # Über einen ECHTEN Sieg, nicht über eine gesetzte Kurzfassung
   *
   * `setBossHpRemaining(0)` und ein Takt: Damit läuft der Weg, den auch der
   * Spieler nimmt — `session.finish(OUTCOME_VICTORY)` ruft `onFinish`, und
   * dort hängt der Aufruf. Eine von Hand gebaute `RunSummary` mit
   * `victory: true` prüfte die Rechnung, nicht den Anschluss.
   */
  function siegFahren(app: App): void {
    app.session.startFight(0);
    // Der Countdown muss durch sein, bevor der Kampf zählt.
    app.session.tick(5_000);
    app.combat.setBossHpRemaining(0);
    app.session.tick(6_000);
  }

  it('der erste Sieg zeigt das Fenster', () => {
    const app = baueApp();
    siegFahren(app);

    // ZUSICHERUNG: Es war wirklich ein Sieg. Ohne sie prüfte der Fall darunter
    // eine Niederlage und wäre grün, weil dann zu Recht nichts erscheint.
    expect(app.lastRecord?.counted, 'der Lauf wurde nicht gezählt').toBe(true);
    expect(
      app.lastRecord?.unlockedEntry,
      'der Sieg hat keinen Eintrag freigeschaltet — dann sagt der Fall nichts über die Anzeige',
    ).not.toBeNull();

    const fenster = document.querySelector('.archive-popup');
    expect(fenster, 'kein Fenster nach dem ersten Sieg').not.toBeNull();
    expect(fenster?.textContent).toContain('A Crack in the Crucible');
    // Die Knöpfe aus der Entscheidung (GAME-DESIGN 20.5 Punkt 6).
    expect(fenster?.querySelector('.intro-continue')).not.toBeNull();
    expect(fenster?.querySelector('.intro-skip')).not.toBeNull();
  });

  it('der ZWEITE Sieg zeigt es NICHT mehr', () => {
    // Der Teil von AC1, der erst jetzt prüfbar ist: Solange gar nichts
    // erschien, war er trivial erfüllt und sagte nichts.
    const app = baueApp();
    siegFahren(app);
    document.querySelector('.archive-popup')?.remove();

    // Zweiter Lauf auf demselben Spielstand.
    app.restart();
    siegFahren(app);

    expect(app.lastRecord?.counted, 'der zweite Lauf wurde nicht gezählt').toBe(true);
    expect(
      app.lastRecord?.unlockedEntry,
      'der zweite Sieg hat erneut freigeschaltet — dann greift „einmal automatisch" nicht',
    ).toBeNull();
    expect(document.querySelector('.archive-popup'), 'das Fenster kam ein zweites Mal').toBeNull();
  });

  it('ein ABBRUCH zeigt nichts', () => {
    // Die dritte Seite: `IGNORED_OUTCOMES` führt den Abbruch, er ist kein
    // erster Sieg. Ohne diesen Fall wäre eine Umsetzung grün, die nach jedem
    // Kampfende irgendetwas zeigt.
    const app = baueApp();
    app.session.startFight(0);
    app.session.tick(5_000);
    app.session.wipe(6_000);
    expect(document.querySelector('.archive-popup')).toBeNull();
  });

  it('der Knopf schliesst es', () => {
    const app = baueApp();
    siegFahren(app);
    const fenster = document.querySelector('.archive-popup');
    expect(fenster).not.toBeNull();
    (fenster?.querySelector('.intro-skip') as HTMLButtonElement | null)?.click();
    expect(document.querySelector('.archive-popup'), 'SKIP hat nicht geschlossen').toBeNull();
  });
});
