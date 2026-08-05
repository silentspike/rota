/**
 * Der Spielstand im Browser — Laden beim Start, Schreiben beim Kampfende
 * (Issue #179).
 *
 * WARUM DAS EINE EIGENE DATEI IST UND NICHT DREI ZEILEN IN app.ts
 * ---------------------------------------------------------------
 * `SaveStore` wurde bis hierher **nirgends** benutzt: Rotation, Prüfsummen und
 * Wiederherstellung standen seit #83 bereit und liefen nie. Diese Datei ist die
 * erste Verwendung, und sie muss drei Dinge können, die in einer Zeile nicht
 * unterzubringen sind: einen fehlenden Stand vertragen, einen beschädigten
 * überstehen und einen Browser ohne Speicher überleben.
 *
 * OHNE SPEICHER LÄUFT DAS SPIEL WEITER
 * ------------------------------------
 * `localStorage` kann fehlen (privates Fenster, abgeschaltete Speicherung) und
 * `setItem` kann werfen (Kontingent voll). Beides darf den Kampf nicht
 * anhalten: Ein Spiel, das nicht startet, weil die Fortschrittsmarke nicht
 * abgelegt werden konnte, hat sein Verhältnis verkehrt. Der Fehler wird
 * gemeldet, nicht verschluckt — aber er beendet nichts.
 */
import { SaveStore, type KeyValueStore } from './save-store.js';
import { SAVE_SCHEMA, type SaveData } from './save-format.js';
import type { RecordedState } from './run-recorder.js';

/**
 * Ein Spielstand, wie er entsteht, wenn es noch keinen gibt.
 *
 * Ausdrücklich vollständig und nicht „teilweise": Die Liste der erlaubten
 * Felder ist abschließend (save-format.ts), und ein Stand, dem eines fehlt,
 * würde beim ersten Schreiben auffallen statt beim ersten Lesen.
 */
export function emptySave(gameVersion: string, now: number): SaveData {
  return {
    schema: SAVE_SCHEMA,
    gameVersion,
    savedAt: now,
    level: 1,
    experience: 0,
    talentPoints: {},
    items: [],
    bindings: {},
    unlockedSpells: [],
    unlockedInstances: [],
    bestRuns: [],
    defeatedBosses: [],
    runHistory: [],
    progressMarks: [],
  };
}

/**
 * Einen geladenen Stand auf die heutige Form bringen.
 *
 * WARUM DAS NOETIG IST, OBWOHL DIE PRUEFSUMME STIMMT
 * -------------------------------------------------
 * Ein Stand, der vor Issue #82 geschrieben wurde, kennt `runHistory` und
 * `progressMarks` nicht — und er ist deswegen nicht **beschädigt**: Prüfsumme
 * und Schema stimmen, `SaveStore` gibt ihn als `ok` zurück, und das ist
 * richtig so. Fehlende Felder sind kein Schaden, sondern ein älterer Stand.
 *
 * Ohne diese Stelle stünde in `runHistory` dann `undefined`, und der erste
 * Zugriff würfe — **beim Kampfende**, also genau dann, wenn der Spieler seine
 * Auswertung sehen will. Gemessen, bevor es jemand erlebt hat.
 *
 * Ergänzt wird nur, was FEHLT. Ein vorhandenes Feld wird nie überschrieben:
 * Das hier ist eine Ergänzung und keine Reparatur.
 */
function ergaenzt(geladen: SaveData | null, gameVersion: string, now: number): SaveData {
  const leer = emptySave(gameVersion, now);
  if (geladen === null) return leer;
  return {
    ...leer,
    ...geladen,
    // Die beiden aus #82. Sie stehen einzeln da, weil `...geladen` ein
    // ausdrückliches `undefined` durchreichen würde — der Streu-Operator
    // unterscheidet nicht zwischen „fehlt" und „ist undefined".
    runHistory: geladen.runHistory ?? leer.runHistory,
    progressMarks: geladen.progressMarks ?? leer.progressMarks,
  };
}

/** Was beim Laden geschah — für die Anzeige und für das Prüfblatt. */
export interface PersistenceStatus {
  /** `ok`, `recovered` (ein jüngerer Stand war beschädigt) oder `empty`. */
  readonly status: string;
  /** Wie viele jüngere Stände übersprungen wurden. */
  readonly skipped: number;
  /** Was beim Schreiben zuletzt schiefging, sonst `null`. */
  readonly lastError: string | null;
}

/**
 * Der Spielstand über die Zeit eines Seitenaufrufs.
 *
 * Hält den zuletzt geladenen Stand und schreibt ihn fort. Er hält ihn, damit
 * ein Kampfende nicht erst lesen muss — und weil ein Lesefehler mitten im
 * Schreiben den Stand sonst zurücksetzen könnte.
 */
export class RunPersistence {
  private readonly store: SaveStore | null;
  private data: SaveData;
  private statusValue: string;
  private skippedValue: number;
  private lastErrorValue: string | null = null;

  constructor(
    private readonly gameVersion: string,
    private readonly now: () => number,
    store?: KeyValueStore | null,
  ) {
    const gewählt = store === undefined ? safeLocalStorage() : store;
    this.store = gewählt === null ? null : new SaveStore(gewählt);

    if (this.store === null) {
      this.data = emptySave(gameVersion, now());
      this.statusValue = 'unavailable';
      this.skippedValue = 0;
      return;
    }

    const gelesen = this.store.load();
    this.statusValue = gelesen.status;
    this.skippedValue = gelesen.skipped;
    this.data = ergaenzt(gelesen.data, gameVersion, now());
  }

  get state(): RecordedState {
    return {
      runHistory: this.data.runHistory,
      progressMarks: this.data.progressMarks,
      level: this.data.level,
      experience: this.data.experience,
    };
  }

  get save(): SaveData {
    return this.data;
  }

  get status(): PersistenceStatus {
    return {
      status: this.statusValue,
      skipped: this.skippedValue,
      lastError: this.lastErrorValue,
    };
  }

  /**
   * Den fortgeschriebenen Stand ablegen.
   *
   * Gibt zurück, ob es gelungen ist. **Der Aufrufer darf das ignorieren** — ein
   * misslungenes Schreiben ist ein Verlust, aber kein Grund, den Kampf
   * anzuhalten.
   */
  persist(next: RecordedState): boolean {
    this.data = {
      ...this.data,
      // Die Fassung des Standes zieht mit: Er ist unter dieser hier
      // geschrieben. Die Fassung je EINTRAG steht daneben und bleibt, was sie
      // war (GAME-DESIGN 18.6).
      gameVersion: this.gameVersion,
      savedAt: this.now(),
      runHistory: next.runHistory,
      progressMarks: next.progressMarks,
      // **Stufe und Erfahrung, sonst nichts vom Aufstieg** (Issue #248, AC5).
      // Was daraus folgt — Grundwerte, freigeschaltete Zauber, Leisten-Plätze
      // — wird bei jedem Aufruf neu gerechnet. Ein gespeicherter Wert ginge
      // nicht kaputt, er würde nur alt, und eine Balance-Änderung erreichte
      // bestehende Spielstände nie mehr.
      level: next.level,
      experience: next.experience,
    };
    if (this.store === null) {
      this.lastErrorValue = 'Kein Browser-Speicher verfuegbar.';
      return false;
    }
    try {
      this.store.save(this.data);
      this.lastErrorValue = null;
      return true;
    } catch (fehler) {
      // Gemeldet, nicht verschluckt -- und der Kampf laeuft weiter.
      this.lastErrorValue = fehler instanceof Error ? fehler.message : String(fehler);
      console.error('[rota] Spielstand konnte nicht abgelegt werden:', fehler);
      return false;
    }
  }
}

/**
 * `localStorage`, sofern es ihn gibt und er benutzbar ist.
 *
 * Der Zugriff selbst kann werfen — in manchen Browsern wirft schon das Lesen
 * der Eigenschaft, wenn Speicherung abgeschaltet ist. Deshalb der Versuch mit
 * einem echten Schreibvorgang: Vorhandensein allein sagt nicht, dass es geht.
 */
function safeLocalStorage(): KeyValueStore | null {
  try {
    const kandidat = globalThis.localStorage;
    if (kandidat === undefined || kandidat === null) return null;
    const probe = '__rota.probe__';
    kandidat.setItem(probe, '1');
    kandidat.removeItem(probe);
    return kandidat;
  } catch {
    return null;
  }
}
