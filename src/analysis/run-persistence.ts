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
import { SaveStore, SLOT_COUNT, type KeyValueStore } from './save-store.js';
import { SAVE_SCHEMA, type OwnedItem, type SaveData } from './save-format.js';
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
    // **JEDES nicht primitive Feld steht hier einzeln**, weil der Streu-Operator
    // ein `null` durchreicht.
    //
    // **Der Grund ist `null`, nicht `undefined`** — bis Issue #378 stand hier
    // das Gegenteil, und es war beim Bau eines Wächters aufgefallen, der den
    // falschen Fall nachbaute und deshalb grün blieb, ohne etwas zu prüfen:
    //
    //   Feld FEHLT           `{ ...leer, ...geladen }` nimmt den leeren Wert.
    //                        Harmlos, diese Zeile wäre dafür nicht nötig.
    //   Feld ist `null`      wird durchgereicht. DAS fängt die Zeile — und es
    //                        ist der einzige Fall, den ein geladener Stand
    //                        erzeugen kann.
    //   Feld ist `undefined` würde auch durchgereicht, kommt aber nie an:
    //                        `JSON.stringify({a: undefined})` ergibt `{}`.
    //
    // Die ersten beiden stammen aus #82, `bestRuns` kam in #377 dazu, als sie
    // zum ersten Mal beschrieben wurde. Die übrigen sechs fehlten bis #378 —
    // folgenlos, solange niemand sie las, und genau deshalb unbemerkt.
    //
    // **Was diese Liste vollständig hält, ist kein Vorsatz, sondern ein
    // Wächter**: `tests/unit/save-shape.test.ts` baut für jedes nicht primitive
    // Feld aus `emptySave()` einen Stand, in dem genau dieses fehlt, und nennt
    // es beim Namen, wenn danach `undefined` herauskommt. Eine neue Liste ohne
    // ihre Zeile macht ihn rot — der Anlass kommt mit, statt daran zu hängen,
    // dass jemand diesen Kommentar liest.
    runHistory: geladen.runHistory ?? leer.runHistory,
    progressMarks: geladen.progressMarks ?? leer.progressMarks,
    bestRuns: geladen.bestRuns ?? leer.bestRuns,
    items: geladen.items ?? leer.items,
    unlockedSpells: geladen.unlockedSpells ?? leer.unlockedSpells,
    unlockedInstances: geladen.unlockedInstances ?? leer.unlockedInstances,
    defeatedBosses: geladen.defeatedBosses ?? leer.defeatedBosses,
    // **Die beiden sind keine Listen, haben aber dieselbe Krankheit.** Ein
    // Wächter, der nur Arrays prüft, liesse sie stehen.
    talentPoints: geladen.talentPoints ?? leer.talentPoints,
    bindings: geladen.bindings ?? leer.bindings,
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
  /**
   * Der Platz im Ringpuffer, der GILT — `-1`, solange keiner geschrieben wurde.
   *
   * Beim Start der Platz, aus dem gelesen wurde; danach der zuletzt
   * geschriebene. Beides beantwortet dieselbe Frage: wo der Stand steht,
   * der zählt.
   *
   * **Gemerkt und nicht nachgeschlagen** (Issue #378): Wer ihn im Browser von
   * Hand sucht, liest `rota.save.index` und muss wissen, dass es fünf Plätze
   * gibt. Genau dabei sind an einem Abend zwei Sitzungen unabhängig
   * voneinander auf den falschen Platz geraten und hielten einen ausgebliebenen
   * Fortschritt für einen Befund.
   */
  private slotValue = -1;

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
    this.slotValue = gelesen.slot;
    this.data = ergaenzt(gelesen.data, gameVersion, now());
  }

  get state(): RecordedState {
    return {
      runHistory: this.data.runHistory,
      progressMarks: this.data.progressMarks,
      bestRuns: this.data.bestRuns,
      talentPoints: this.data.talentPoints,
      items: this.data.items,
      level: this.data.level,
      experience: this.data.experience,
    };
  }

  get save(): SaveData {
    return this.data;
  }

  /**
   * Wo dieser Stand herkommt — für Dev-Werkzeuge und Prüffälle (Issue #378).
   *
   * `skippedSlots` nennt die übersprungenen Plätze beim Namen. Eine Zahl allein
   * („2 übersprungen") ist eine Angabe ohne Bezugspunkt: Sie sagt nicht,
   * WELCHE, und genau danach sucht man, wenn etwas fehlt.
   */
  get origin(): {
    readonly slot: number;
    readonly status: string;
    readonly skipped: number;
    readonly skippedSlots: readonly number[];
    readonly healthy: number;
  } {
    const uebersprungen: number[] = [];
    if (this.slotValue >= 0) {
      // Rückwärts vom jüngsten Platz bis zu dem, der genommen wurde — dieselbe
      // Richtung, in der `load()` gesucht hat.
      for (let back = this.skippedValue; back > 0; back--) {
        uebersprungen.push((this.slotValue + back) % SLOT_COUNT);
      }
    }
    return {
      slot: this.slotValue,
      status: this.statusValue,
      skipped: this.skippedValue,
      skippedSlots: uebersprungen,
      healthy: this.store?.healthySlots() ?? 0,
    };
  }

  get status(): PersistenceStatus {
    return {
      status: this.statusValue,
      skipped: this.skippedValue,
      lastError: this.lastErrorValue,
    };
  }

  /**
   * Nur die Stufe ablegen — fuer Nachweise (Issue #264).
   *
   * Setzt die Erfahrung auf null: Ein Sprung auf Stufe 5 mit der Erfahrung von
   * Stufe 1 waere ein Zustand, den das Spiel nie erreicht.
   */
  /**
   * Nur die Talent-Verteilung ablegen (Issue #417).
   *
   * **Getrennt von `persist`**, aus demselben Grund wie die Belegung (#264):
   * Sie entsteht zu einem anderen Zeitpunkt — `persist` schreibt nach einem
   * Kampf, verteilt wird dazwischen.
   */
  /** Nur die Gegenstände ablegen (Issue #421a) — wie `persistTalents`. */
  persistItems(items: readonly OwnedItem[]): boolean {
    this.data = { ...this.data, savedAt: this.now(), items };
    if (this.store === null) return false;
    try {
      this.slotValue = this.store.save(this.data);
      return true;
    } catch {
      return false;
    }
  }

  persistTalents(allocation: Readonly<Record<string, number>>): boolean {
    this.data = { ...this.data, savedAt: this.now(), talentPoints: allocation };
    if (this.store === null) return false;
    try {
      this.slotValue = this.store.save(this.data);
      return true;
    } catch {
      return false;
    }
  }

  persistLevel(level: number): boolean {
    this.data = {
      ...this.data,
      savedAt: this.now(),
      level: Math.max(Math.trunc(level), 1),
      experience: 0,
    };
    if (this.store === null) return false;
    try {
      // **Der geschriebene Platz zieht mit** (Issue #378): Sonst nennte
      // `origin.slot` den Platz, aus dem beim Start GELESEN wurde — und wer
      // danach nachsieht, liest einen alten Stand. Genau der Fehler, gegen den
      // dieses Werkzeug gebaut ist, nur eine Ebene tiefer.
      this.slotValue = this.store.save(this.data);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Nur die Belegung ablegen (Issue #264).
   *
   * **Getrennt von `persist`**, weil sie zu einem anderen Zeitpunkt entsteht:
   * `persist` schreibt nach einem Kampf, die Belegung ändert der Spieler
   * dazwischen. Beides in einen Aufruf zu legen hieße, dass eine Änderung an
   * der Leiste erst nach dem nächsten Kampf ankäme — und ein Absturz davor
   * nähme sie mit.
   *
   * Gibt zurück, ob es gelungen ist; der Aufrufer darf das ignorieren.
   */
  persistBindings(bindings: Readonly<Record<string, string>>): boolean {
    this.data = { ...this.data, savedAt: this.now(), bindings };
    if (this.store === null) {
      this.lastErrorValue = 'Kein Browser-Speicher verfuegbar.';
      return false;
    }
    try {
      // **Der geschriebene Platz zieht mit** (Issue #378): Sonst nennte
      // `origin.slot` den Platz, aus dem beim Start GELESEN wurde — und wer
      // danach nachsieht, liest einen alten Stand. Genau der Fehler, gegen den
      // dieses Werkzeug gebaut ist, nur eine Ebene tiefer.
      this.slotValue = this.store.save(this.data);
      this.lastErrorValue = null;
      return true;
    } catch (fehler) {
      this.lastErrorValue = fehler instanceof Error ? fehler.message : String(fehler);
      return false;
    }
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
      // **Was ERREICHT wurde, nicht was gerechnet wurde** (Issue #377): Der
      // Eintrag hält die Reserve bzw. den Ring-Rest als Rohwert, nicht die
      // Punkte daraus. Punkte wären ein berechneter Wert, der auf kommende
      // Kämpfe zurückwirkt — genau das, was 18.6 verbietet.
      bestRuns: next.bestRuns,
      // **Nur die Verteilung** (Issue #417, GAME-DESIGN 18.6): Der Faktor, den
      // der Kampf daraus bildet, ist ein berechneter Wert und gehoert nicht
      // hierher.
      talentPoints: next.talentPoints,
      items: next.items,
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
      // **Der geschriebene Platz zieht mit** (Issue #378): Sonst nennte
      // `origin.slot` den Platz, aus dem beim Start GELESEN wurde — und wer
      // danach nachsieht, liest einen alten Stand. Genau der Fehler, gegen den
      // dieses Werkzeug gebaut ist, nur eine Ebene tiefer.
      this.slotValue = this.store.save(this.data);
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
