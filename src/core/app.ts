/**
 * Zusammenbau des laufenden Spiels: Zeichenfläche, Logik-Takt, Zeichenschleife,
 * Eingabe, Kampf und Klang.
 *
 * Hier hängt zusammen, was sonst bewusst getrennt gehalten ist. Die Klasse bleibt
 * frei von Entwicklungs-Werkzeugen — die hängen sich von außen an
 * (`GameLoop.onFrameMeasured`) und fallen im Auslieferungsstand weg.
 */
import { AudioFeedback } from './audio-bridge.js';
import { Combat } from './combat.js';
import {
  BOSSES,
  CALCINATION,
  FIRST_CRUCIBLE,
  bossById,
  instanceById,
  type BossDef,
} from './data.js';
import { buildBossSelect, mechanikenVon, waehlbareBosse } from '../progression/boss-select.js';
import { EV_BOSS_HIT, NO_KEY } from './events.js';
import {
  applyBindings,
  bindSpell,
  clearSlot,
  defaultBindings,
  sanitize,
} from '../progression/bindings.js';
import { buildComposition } from '../progression/composition-view.js';
import { buildGearView } from '../progression/gear-view.js';
import { buildTalentView } from '../progression/talent-view.js';
import { SLOT_COUNT } from '../input/keys.js';
import { GameLoop } from './loop.js';
import { Session } from './session.js';
import { PressQueue } from '../input/queue.js';
import { InputRouter } from '../input/router.js';
import { KeyLabels } from '../input/layout.js';
import { Impact } from '../render/impact.js';
import { Overlay } from '../render/overlay.js';
import { PHASE_COUNTDOWN, PHASE_ENDED, PHASE_FIGHTING, PHASE_PREPARE } from './phases.js';
import { RunPersistence } from '../analysis/run-persistence.js';
import { recordRun, type RecordResult } from '../analysis/run-recorder.js';
import { comparableFromBest, curveFromBest, usableBest } from '../analysis/best-run.js';
import {
  compareRuns,
  sampleDamageCurve,
  type ComparedMetric,
  type RunCurve,
} from '../analysis/run-compare.js';
import { levelUpDisplayFor } from '../analysis/level-up-display.js';
import { buildHistoryView } from '../analysis/history-view.js';
import {
  attachHelp,
  buildArchiveView,
  buildEntryPopup,
  buildIntroduction,
} from '../analysis/archive-view.js';
import { ALL_HELP_ENTRIES, ALL_INTRODUCTIONS } from '../analysis/help-data.js';
import { markSeen, nextIntroduction } from '../analysis/help.js';
import { buildArchive } from '../analysis/archive.js';
import { ALL_ARCHIVE_ENTRIES } from '../analysis/archive-data.js';
import { DEFAULT_DUMMY, buildDummy } from '../analysis/training.js';
import '../analysis/history.css';
import { pointsDisplayFor } from '../analysis/points-display.js';
import type { RunSummary } from './summary.js';
import { markDisplayFor } from '../analysis/mark-display.js';
import type { MarkDisplay } from '../render/overlay.js';
import { PlayerHome } from '../progression/player-home.js';
import { statOverview } from '../progression/stat-overview.js';
import { levelBarFill, levelOverview } from '../progression/level-overview.js';
import { buildLevelOverview } from '../progression/level-overview-view.js';
import { buildStatOverview } from '../progression/stat-overview-view.js';
import { baseStats } from '../progression/base-stats.js';
import '../progression/stats.css';
import '../progression/levels.css';
import '../progression/composition.css';
import '../progression/gear.css';
import '../progression/talents.css';
import '../progression/boss-select.css';
// **Die Gestaltung für Archiv, Erklärfenster und Einführung** — sie war seit
// #85 gebaut und wurde von NIEMANDEM geladen (gemessen an #811: kein einziger
// Import auf `archive.css` im ganzen Baum; sie erschien nur im Bündel der
// Prüfseite `docs/archive-check/`).
//
// Was der Browser daraus machte: `position: static`, kein Hintergrund, kein
// Rahmen, 704 px breit — das Erklärfenster floss als nackter Text in die
// Tabellenzelle. Im DOM alles richtig, auf dem Bildschirm unbrauchbar.
//
// Dieselbe Klasse wie „gebaut, grün, unerreichbar", nur eine Ebene tiefer:
// Kein Prüffall fragt, ob eine Stilvorlage geladen wird, und `tsc` und
// `eslint` erst recht nicht. Gefunden hat es `getComputedStyle` im Browser.
import '../analysis/archive.css';
import { resistanceLines } from '../progression/schools.js';
import { HudRenderer } from '../render/hud.js';
import { Surface } from '../render/surface.js';
import { rotationEffectsFrom, wornFrom } from '../gear/catalog.js';
import { allocationProblem } from '../progression/talents.js';
import { talentPointsAtLevel } from '../progression/unlocks.js';
import { equip, equippedBySlot, unequip } from '../gear/equip.js';
import type { OwnedItem } from '../analysis/save-format.js';

/**
 * Welcher Boss den Kampf traegt — normalerweise der erste Tiegel.
 *
 * # Der Startparameter `?boss=` ist ein Entwicklungs-Werkzeug (19.1)
 *
 * Die Werkzeugkiste nennt „Zustand setzen" mit vier Beispielen — Boss-Leben,
 * Abklingzeiten, Phase, Hitze —, und alle vier haben denselben Zweck: einen
 * Zustand herstellen, den zu erspielen unverhältnismäßig wäre. Einen Boss zu
 * wählen ist derselbe Zweck eine Ebene höher: „zeig mir den, der die Mechanik
 * überhaupt hat."
 *
 * Ohne ihn wäre die Energieleiste (#219) nicht prüfbar. Sechs der sieben Bosse
 * haben keine; der einzige mit einer war im Spiel nicht erreichbar, solange die
 * Instanz-Auswahl (12.7) fehlte — **seit Issue #613 gibt es sie** (`Targets`
 * im Player Home). Der Startparameter bleibt trotzdem: Er trifft auch die
 * Prüf-Bosse, die in der Auswahl bewusst nicht stehen.
 *
 * # Warum ein Startparameter und kein Wechsel zur Laufzeit
 *
 * _Diese Begründung galt bis #613 und stimmt in ihrer Annahme nicht mehr._
 * Sie lautete: Ein `setBoss(id)` müsste `Combat` und `Session` **ersetzen**,
 * beide sind in `App` unveränderlich, und an der Session hängt der Beobachter
 * für den Spielstand — ein verlorener Haken fiele erst auf, wenn ein Lauf
 * nicht gespeichert wird.
 *
 * **Der Schluss war richtig, die Voraussetzung nicht.** Einen Boss zu wechseln
 * heißt nicht zwangsläufig, den Kampf zu ersetzen: `Combat.switchBoss` behält
 * die Identität des Objekts und tauscht nur den Gegner samt allem, was aus ihm
 * folgt. `Session` wird dabei nicht angefasst, der Beobachter also auch nicht.
 *
 * Hier entsteht der Kampf weiterhin **von Anfang an** mit dem richtigen Boss —
 * das ist für ein Werkzeug der kürzere Weg. Und es bleibt der Weg, den 12.7
 * ohnehin nimmt: Der Boss wird **vor** dem Kampf gewählt, nicht in ihm.
 *
 * # Ein Tippfehler scheitert sichtbar
 *
 * `?boss=tipfeler` startet **nicht** stillschweigend den Standard-Boss. Wer
 * dann misst, misst die falsche Sache und hält sie für die richtige — genau
 * die Klasse Fehler, gegen die dieses Werkzeug gebaut wird.
 */
function startBoss(): BossDef {
  if (!__ROTA_DEVTOOLS__) return FIRST_CRUCIBLE.value;
  const wunsch = new URLSearchParams(window.location.search).get('boss');
  if (wunsch === null || wunsch === '') return FIRST_CRUCIBLE.value;

  const gefunden = bossById(wunsch);
  if (gefunden === undefined) {
    throw new Error(
      `Unbekannter Boss "${wunsch}" im Startparameter. ` +
        `Ein Tippfehler darf nicht stillschweigend den Standard-Boss starten — ` +
        `sonst misst man die falsche Sache und hält sie für die richtige.`,
    );
  }
  console.info(`[rota] Boss aus dem Startparameter: ${wunsch}`);
  return gefunden;
}

export class App {
  readonly surface: Surface;
  readonly loop: GameLoop;
  readonly combat: Combat;
  readonly session: Session;
  readonly input: InputRouter;
  readonly labels = new KeyLabels();
  readonly audio = new AudioFeedback();
  readonly overlay: Overlay;
  readonly impact = new Impact();

  private readonly boundUiKey: (event: KeyboardEvent) => void;

  /** Die Übergabe vom Browser an den Logik-Takt. Öffentlich für die Werkzeuge. */
  readonly pressQueue = new PressQueue();

  /** Was beim Lesen der Spieldaten nicht stimmte. Leer heißt: alles in Ordnung. */
  readonly dataProblems: readonly string[];

  /**
   * Der Spielstand im Browser (Issue #179).
   *
   * Öffentlich, weil das Prüfblatt ihn liest — und weil ein Zustand, den man
   * nur über einen Kampf erreichen kann, nicht nachweisbar wäre.
   */
  readonly persistence: RunPersistence;

  /**
   * Was der letzte Kampf an Marke und Verlauf bewirkt hat, oder `null`.
   *
   * Daraus zeigt die Auswertung dem Spieler, dass er weiterkommt — bei einer
   * **Niederlage** ist das der ganze Zweck (GAME-DESIGN 15.1).
   */
  lastRecord: RecordResult | null = null;

  /**
   * Die Kurzfassung desselben Kampfes (Issue #387).
   *
   * **Neben `lastRecord` und nicht daraus**: Die Erklärung der Punktzahl
   * braucht beides — den Bestwert von VOR dem Lauf aus der Aufzeichnung und
   * das Mass aus dem Lauf selbst (`heldPercent !== null`). Beide zusammen
   * entstehen nur hier, in `onFinish`.
   */
  lastSummary: RunSummary | null = null;

  /**
   * Die Fortschrittsmarke, fertig zum Anzeigen (Issue #187).
   *
   * Hier gebildet und nicht in der Ansicht: Der Satz entsteht an EINER Stelle,
   * aus `markSentence`. Zwei Stellen, die aus denselben Zahlen einen Satz
   * machen, laufen beim naechsten Nachziehen auseinander.
   *
   * `null` heisst: nichts zeigen. Das ist der Fall beim allerersten Kampf
   * eines Spielstands und nach einem Sieg -- dort steht das Ergebnis, nicht
   * die Marke (Issue #187, AC3 und AC4).
   */
  lastMark: MarkDisplay | null = null;

  /**
   * Der Bildschirm zwischen den Kaempfen (GAME-DESIGN 16.1, Issue #188).
   *
   * Oeffentlich, weil das Pruefblatt ihn liest -- und weil ein Bildschirm, den
   * man nur ueber einen Knopf erreicht, ohne Browser nicht nachweisbar waere.
   */
  readonly home: PlayerHome;

  constructor(parent: HTMLElement) {
    const boss = startBoss();
    this.dataProblems = [...CALCINATION.problems, ...FIRST_CRUCIBLE.problems];
    this.surface = new Surface(parent);
    this.combat = new Combat({ school: CALCINATION.value, boss });
    this.session = new Session(this.pressQueue, this.combat, this.audio);

    // Erste Verwendung des Spielstands ueberhaupt: Rotation, Pruefsummen und
    // Wiederherstellung stehen seit #83 bereit und liefen nie.
    this.persistence = new RunPersistence(__ROTA_VERSION__, () => Date.now());

    // **Die Talente NACH dem Spielstand** (Issue #417): Der Kampf entsteht
    // weiter oben, weil die Sitzung ihn braucht — die Verteilung kann er also
    // nicht schon beim Aufbau bekommen. Sie hier nachzuziehen ist dieselbe
    // Zeile wie bei einem Umskillen zwischen zwei Kaempfen, und es gibt keine
    // zweite Rechenstelle: `setTalents` bildet die Faktoren neu.
    this.combat.setTalents(this.persistence.state.talentPoints);

    // **Die getragenen Werte** (Issue #421a). Bis hierher bekam der einzige
    // Produktions-Kampf gar kein `worn` und fiel auf `NOTHING_WORN` zurück —
    // die Kette `worn -> essenceFactor -> factors` war seit #282 verdrahtet
    // und begann bei einem Wert, den niemand füllte.
    this.combat.setWorn(wornFrom(this.persistence.state.items));

    // **Die Rotations-Effekte der Sätze** (13.5, Issue #480). Zwei Quellen,
    // zwei Wege in den Kampf: `wornFrom` bringt die Zahlen, `rotationEffectsFrom`
    // die Kennungen der sechsten Satzstufen.
    //
    // **Diese Zeile fehlte seit #774**, und mit ihr die Wirkung von sechs
    // Issues: Ashen Regimen, The Whole Work, Slow Rot, The Sealed Vessel,
    // The Long Grip und Perpetual Draft waren gebaut, geprüft und
    // paritätsgesichert — `hasSetEffect` gab für jeden Spieler immer `false`,
    // weil `setEffects` nie befüllt wurde. Die Leitung war bis
    // `rotationEffectsFrom` gebaut und endete dort an der Wand.
    //
    // Jeder Prüffall dazu war grün, weil jeder `setSetEffects` selbst rief —
    // ein Fall, der den Setter bedient, IST die fehlende Speisung und kann sie
    // nicht sehen. Gefunden hat es erst die Frage „wer ruft das im Betrieb?",
    // gestellt an `setWorn` (2 Aufrufer) neben `setSetEffects` (0).
    this.combat.setSetEffects(rotationEffectsFrom(this.persistence.state.items));

    // Am Kampfende festhalten, was der Versuch eingebracht hat. Der Beobachter
    // greift nicht ein -- ein Spiel ohne Spielstand rechnet identisch
    // (GAME-DESIGN 18.5), es merkt sich nur nichts.
    this.session.onFinish = (summary, outcome): void => {
      this.lastRecord = recordRun(this.persistence.state, summary, outcome, {
        bossId: this.combat.boss.id,
        // Schritt 1 kennt eine Stufe. Die Zahl steht hier und nicht als
        // Vorgabe im Rechner: Sobald es mehrere gibt, ist es diese Zeile.
        difficulty: 1,
        // Die Stufe des BOSSES, aus seinem Datensatz — nicht die des
        // Spielers und nicht die Schwierigkeit (Issue #248).
        bossLevel: this.combat.boss.level,
        gameVersion: __ROTA_VERSION__,
        at: Date.now(),
        // **Der Startwert dieses Kampfes** (Issue #447): Aus ihm leitet
        // `lootStreamFor` den Beute-Strom ab — getrennt von den Kampfströmen,
        // aber aus derselben Wurzel, damit eine aufgezeichnete Runde ihre
        // Beute wiederholt.
        seed: 0,
        // **Die Instanz des Bosses, nicht mehr die feste 1** (#421c).
        //
        // Hier stand bis dahin `instance: 1` mit dem Vermerk „sobald es die
        // Zuordnung gibt, ist es diese Zeile". `data/instances.json` gibt es
        // jetzt, und damit greifen zwei Dinge, die vorher nirgends griffen:
        //
        //   `itemLevelOf(i, …) = (i-1)*6 + d*3 + e + 1`   die Beute-Stufe
        //   `qualityCapOf(i)`                             die Obergrenze (13.4)
        //
        // Mit der festen 1 ließ ein Boss aus instance-2 dieselbe Beute fallen
        // wie der Prolog-Boss, und die „Obergrenze durch Inhalt" aus 13.4 war
        // überall dieselbe.
        //
        // **Unbekannte Instanz fällt auf 1 zurück, nicht auf 0.** Ein Boss,
        // dessen Kennung in `instances.json` fehlt, ist ein Datenfehler — aber
        // ein Kampf, der deswegen keine Beute wirft, wäre die schlechtere
        // Antwort darauf. `itemLevelOf` klemmt ohnehin auf `[1, INSTANCE_COUNT]`.
        instance: instanceById(this.combat.boss.instance)?.number ?? 1,
        endlessLevel: 0,
        // **Der Schadensverlauf dieses Laufs** (Issue #82, AC1 „zwei Kurven
        // übereinander"). HIER und nicht im Rechner: Dies ist die Stelle, an
        // der Kampfprotokoll und Spielstand zusammenkommen — `recordRun` sieht
        // das Protokoll nie, und `RunSummary` führt keines.
        //
        // **Dieselbe Rechnung wie in `hud.ts`, und trotzdem keine Doppelung
        // im schädlichen Sinn**: Dort entsteht die Kurve des GERADE
        // gelaufenen Versuchs für die Anzeige, bei jedem Ausgang. Hier wird
        // sie abgelegt, und nur ein Sieg legt etwas ab. Zusammengelegt müsste
        // eine der beiden Stellen die andere fragen — die Anzeige den
        // Spielstand oder der Spielstand die Anzeige.
        curve: sampleDamageCurve(this.session.log, summary.elapsedMs),
      });
      // Ein misslungenes Ablegen ist ein Verlust, aber kein Grund, den Ablauf
      // anzuhalten -- der Rueckgabewert wird bewusst nicht geprueft.
      if (this.lastRecord.counted) this.persistence.persist(this.lastRecord);
      this.lastMark = markDisplayFor(this.lastRecord, summary, this.combat.boss.id);
      this.lastSummary = summary;
      this.showUnlockedEntry(this.lastRecord);
    };

    this.applyBindings();
    this.audio.bindSpells(this.combat.spells.map((spell) => spell.id));

    this.overlay = new Overlay(
      parent,
      this.combat.boss.name,
      {
        onReady: () => this.ready(),
        onResume: () => this.resume(),
        onWipe: () => this.wipe(),
        onRestart: () => this.restart(),
        onBackToMenu: () => this.backToMenu(),
        onOpenHome: () => {
          this.openHome();
        },
      },
      // **Fertige Zeilen, nicht der Datensatz** (Issue #328): Ob ein Faktor
      // „hält ab" oder „ist schwach" heißt, entscheidet `resistanceLines` an
      // einer Stelle — nicht jeder Bildschirm für sich.
      resistanceLines(this.combat.boss.resistances),
    );

    // Das Player Home. Es fasst den Kampf NICHT an -- kein Anhalten, kein
    // Zuruecksetzen, kein Zugriff auf die Sitzung. Es zeigt und verbirgt sich.
    this.home = new PlayerHome(() => {
      // Zurueck heisst zurueck zum Bildschirm davor. Das Overlay steht
      // ohnehin dort, wo es stand -- es wurde nie angefasst.
    });
    // **Der Aufstieg steht vor den Werten.** Er ist das, was sich zwischen
    // zwei Besuchen ändert; die Grundwerte folgen daraus (Issue #248, AC6).
    this.home.addSection({
      id: 'levels',
      title: 'Progression',
      content: buildLevelOverview(
        levelOverview({
          level: this.persistence.state.level,
          xp: this.persistence.state.experience,
        }),
        levelBarFill({
          level: this.persistence.state.level,
          xp: this.persistence.state.experience,
        }),
      ),
    });
    // **Die Leiste vor den Werten, nach dem Aufstieg.** Sie ist das, was der
    // Spieler hier ändert; die Werte darunter folgen daraus (Issue #264).
    // **Die Boss-Wahl vor der Zusammenstellung** (Issue #613): Erst steht
    // fest, gegen WEN man antritt — die Leiste stellt man danach darauf ein.
    // Umgekehrt stellte man zusammen, ohne den Gegner zu kennen.
    //
    // Der Bereich meldet nur die Wahl; dass daraufhin ein Kampf beginnt,
    // entscheidet diese Datei. So bleibt die Zusage von `player-home.ts`
    // wörtlich gültig, dass jener Bildschirm den Kampf nicht anfasst.
    this.home.addSection({
      id: 'targets',
      title: 'Targets',
      content: this.buildBossSection(),
    });
    this.home.addSection({
      id: 'loadout',
      // Der Bereich heißt jetzt nach dem, was er ist: Die Ansicht aus
      // GAME-DESIGN 16 zeigt beide Seiten — die eigenen Zauber und die
      // Leiste. „Spell bar" nannte nur die halbe Ansicht (Issue #270).
      title: 'Composition',
      content: this.buildLoadoutSection(),
    });
    // **Die Ausrüstung zwischen Leiste und Werten.** Sie ist wie die Leiste
    // etwas, das der Spieler hier ändert; die Werte darunter folgen aus beidem
    // (Issue #453).
    this.home.addSection({
      id: 'gear',
      title: 'Equipment',
      content: this.buildGearSection(),
    });
    // **Die Talente nach der Ausruestung, vor den Werten** (Issue #802): Sie
    // sind wie Leiste und Ausruestung etwas, das der Spieler HIER aendert, und
    // die Werte darunter folgen aus allen dreien.
    //
    // Bis zu diesem Abschnitt hatte `App.setTalents` genau einen Aufrufer
    // ausserhalb der Pruefstuecke — in den Entwicklungswerkzeugen. Sechs
    // Baeume und achtzehn Wege waren gebaut und fuer jeden Spieler
    // unerreichbar.
    this.home.addSection({
      id: 'talents',
      title: 'Talents',
      content: this.buildTalentSection(),
    });
    this.home.addSection({
      id: 'stats',
      title: 'Stats',
      // Die Werte, wie sie heute gelten. Ohne Ausruestung sind es die
      // Grundwerte -- ein neuer Spielstand hat nichts angelegt (AC7).
      content: buildStatOverview(
        statOverview(baseStats(this.persistence.state.level), this.combat.wornValues),
      ),
    });

    // **Der Trainingsraum** (E7-4, Issue #84). `buildDummy` stand fertig da
    // und hatte keinen Aufrufer — eine Puppe mit einer Milliarde Leben und
    // 24 Stunden Ueberdruck, die niemand betreten konnte.
    //
    // Der Knopf startet mit `DEFAULT_DUMMY`. Das Einstellungs-Formular aus
    // 19.2 (Widerstand, Gegenschlag, Wirkzauber) ist NICHT gebaut: Es gibt
    // dafuer keine Ansicht, nur den Datentyp. Ein Knopf ohne Formular macht
    // den Raum betretbar; ein Formular ohne Raum haette gar nichts gebracht.
    this.home.addSection({
      id: 'training',
      title: 'Training',
      content: this.buildTrainingSection(),
    });

    // **Der Verlauf ueber ALLE Durchlaeufe** (E7-2, Issue #82). Er stand
    // fertig gebaut da und hatte keinen Aufrufer — der Bundler hat die Datei
    // deshalb aus dem Spiel geworfen.
    //
    // HIER und nicht in der Auswertung: Was sich auf EINEN Lauf bezieht,
    // gehoert zu dessen Zahlen; der Verlauf ueber alle ist Rueckschau und
    // haette nach jedem einzelnen Kampf nichts zu suchen.
    this.home.addSection({
      id: 'history',
      title: 'History',
      content: buildHistoryView(this.persistence.state.runHistory),
    });

    // **Das Archiv** (GAME-DESIGN 17.4, Issue #85). `buildArchiveView` und
    // `buildArchive` standen fertig gebaut da und hatten keinen Aufrufer —
    // dieselbe Klasse wie `buildHistoryView` vor #82 und `setSetEffects` vor
    // #794.
    //
    // HIER und nicht in der Auswertung: Das Archiv ist Rückschau über alles,
    // was der Spieler gesehen hat. Nach einem einzelnen Kampf hätte es nichts
    // zu suchen — dieselbe Begründung wie beim Verlauf darüber.
    //
    // **Die Einträge kommen aus `ALL_ARCHIVE_ENTRIES`, nicht aus
    // `PLAYABLE_ARCHIVE_ENTRIES`**: Heute sind das ausschliesslich
    // Prüfeinträge, und die ausgelieferte Liste wäre leer. Die Ansicht zeigte
    // dann dauerhaft „Nothing recorded yet" — sichtbar gebaut, aber nicht
    // belegbar. Sobald E10 echte Texte liefert, gehört hier die gefilterte
    // Liste hin; der Vermerk steht in `archive-data.ts`.
    this.home.addSection({
      id: 'archive',
      title: 'Archive',
      content: this.buildArchiveSection(),
    });
    parent.appendChild(this.home.root);

    // **Die Erklärfenster** (AC2 von #811). `attachHelp` stand fertig gebaut
    // da und war DOPPELT tot: kein Aufrufer, und kein einziger Knoten trug
    // `data-help` — es hätte auch mit Aufruf geschwiegen. Das Issue nannte nur
    // die erste Hälfte.
    //
    // **Über das Merkmal, nicht über eine Liste von Bereichen**: So bekommt
    // eine Boss-Fähigkeit ihr Fenster durch dasselbe Zutun wie ein
    // Zauber-Symbol oder ein Wert, und keine Gruppe kann vergessen werden,
    // weil sie woanders im Baum steht.
    //
    // **Die Einträge kommen aus `ALL_HELP_ENTRIES`, nicht aus
    // `PLAYABLE_HELP_ENTRIES`** — heute sind das ausschliesslich Prüfeinträge,
    // und die ausgelieferte Liste wäre leer. Ohne Einträge erschiene kein
    // einziges Fenster: sichtbar gebaut, aber nicht belegbar. Dieselbe
    // Entscheidung wie beim Archiv (#85); der Vermerk steht in `help-data.ts`.
    this.attachHelpToHome();

    // Treffer gehen an die Anzeige und an den Klang. Beide lesen nur mit — ein
    // stummes, unsichtbares Spiel rechnet identisch (GAME-DESIGN 18.5).
    this.combat.onHit = (damage, kind, isCrit, big, timeMs): void => {
      this.impact.onHit(damage, kind, isCrit, big, timeMs);
      // **Jeder Treffer klingt** — so beschreibt das Verzeichnis den Klang:
      // „der häufigste Klang im Spiel überhaupt, jeder Zauber und jeder
      // DoT-Tick löst ihn aus. Deshalb sehr kurz, trocken und mit Streuung."
      // Gegen den Maschinengewehr-Effekt steht die Tonhöhen-Streuung, nicht das
      // Weglassen (GAME-DESIGN 17.1). In E1-6 war das noch offen und deshalb auf
      // die großen Treffer beschränkt; E8 hat es entschieden.
      this.audio.onEvent(EV_BOSS_HIT, NO_KEY, timeMs);
    };

    const renderer = new HudRenderer(
      this.surface,
      this.session,
      this.labels,
      this.overlay,
      this.impact,
      () => this.lastMark,
      // **Nur bei einem gezählten Lauf.** `recordRun` gibt bei einem Abbruch
      // `counted: false` und 0 Punkte zurück; eine 0 in der Anzeige hiesse
      // aber „dieser Lauf hat nichts eingebracht", und ein Abbruch hat gar
      // nicht stattgefunden. Der Strich ist die richtige Aussage.
      // **Die Struktur statt der nackten Zahl** (Issue #387): Eine Punktzahl
      // ohne ihren Grund erklärt sich nicht — 87 statt 4.373 sieht aus wie ein
      // Fehler. `pointsDisplayFor` entscheidet, OB es etwas zu erklären gibt;
      // wie es dasteht, entscheidet die Anzeige.
      () => pointsDisplayFor(this.lastRecord, this.lastSummary),
      // Erfahrung und Aufstieg (Issue #371), aus demselben Grund nur bei
      // einem gezaehlten Lauf: Ein Abbruch hat nicht stattgefunden, und eine
      // 0 hiesse „hat nichts eingebracht".
      () => {
        const record = this.lastRecord;
        if (record?.counted !== true) return { xpGained: null, levelUp: null };
        return {
          xpGained: record.xpGained,
          levelUp: levelUpDisplayFor(
            record.leveledUp,
            this.persistence.state.level,
            this.combat.spells,
          ),
        };
      },
      // **Der Vergleich mit dem Bestlauf** (E7-2, Issue #82).
      //
      // `compareRuns` und `buildComparisonView` standen seit E7-2 fertig
      // gebaut da und hatten **keinen einzigen Aufrufer** — dieselbe Klasse
      // wie `buildEfficiencyView` (#81) und `setSetEffects` (#794): gebaut,
      // geprüft, für jeden Spieler unerreichbar. Diese Zeilen sind der ganze
      // Unterschied.
      //
      // Der Bestlauf wird über Boss UND Schwierigkeit gesucht, mit demselben
      // Maß, das `recordRun` schreiben würde. Passt es nicht — der Boss hat
      // `runaway` bekommen oder verloren —, verwirft `usableBest` den Eintrag,
      // und der Vergleich sagt UNBEKANNT statt Sekunden gegen Prozent zu
      // halten.
      () => this.comparisonForResult(),
      // **Und die zweite Kurve** (Issue #82, AC1). Aus demselben Eintrag wie
      // der Vergleich eine Zeile hoeher, damit Linie und Tabelle denselben
      // Bestlauf zeigen.
      () => this.bestCurveForResult(),
    );
    this.loop = new GameLoop({
      simulation: this.session,
      renderer,
      requestFrame: (callback) => window.requestAnimationFrame(callback),
      cancelFrame: (handle) => {
        window.cancelAnimationFrame(handle);
      },
      now: () => performance.now(),
    });
    this.input = new InputRouter({
      queue: this.pressQueue,
      // Der Druck wirkt im nächsten Takt, der gerechnet wird. Damit ist er im
      // nächsten Bild sichtbar — die kürzeste Strecke, die das Zeitgerüst zulässt.
      nextTickIndex: () => this.loop.tickCount,
      frameIndex: () => this.loop.frameCount,
      onFirstInput: () => {
        this.audio.unlock();
      },
    });

    // Die Bedienung des Ablaufs läuft über eigene Tasten, nicht über die
    // Zauberleiste: Esc öffnet das Menü, Eingabe startet den Kampf. Beide sind
    // bewusst keine Belegungen (siehe `src/input/keys.ts`).
    this.boundUiKey = (event: KeyboardEvent): void => {
      this.handleUiKey(event);
    };
  }

  // ------------------------------------------------------------ Kampfablauf

  /** „Ready" — der Countdown beginnt (GAME-DESIGN 10). */
  /**
   * Die Belegung aus dem Spielstand auf die Sitzung übertragen (Issue #264).
   *
   * **Bis hierher stand sie fest** (`STEP1_BINDINGS`), seit Schritt 1. Die
   * Plätze wuchsen mit der Stufe (#248), belegen konnte sie niemand.
   *
   * Ein leerer Spielstand bekommt die Startbelegung — und zwar **nicht
   * gespeichert**, sondern gerechnet: Solange der Spieler nichts geändert hat,
   * gibt es nichts zu speichern. Erst seine Entscheidung wird abgelegt.
   *
   * `sanitize` läuft davor: Ein Stand kann Einträge tragen, die heute nicht
   * mehr gelten — aus einer Fassung mit anderen Freischaltstufen etwa. Sie
   * stillschweigend zu übernehmen hieße, dass der Kampf einen Zauber wirkt,
   * den der Spieler gar nicht haben dürfte.
   *
   * **Jeder Platz wird gesetzt, auch die leeren.** Wer nur die belegten
   * überträgt, lässt einen geräumten Platz auf seinem alten Zauber stehen —
   * und genau das ist der Fall, der still bricht: Der Spieler merkt es erst im
   * Kampf und hält es für einen Fehler in seiner Rotation.
   */
  private applyBindings(): void {
    const stand = this.persistence.state;
    const gespeichert = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const belegung =
      Object.keys(gespeichert).length > 0
        ? gespeichert
        : defaultBindings(stand.level, this.combat.spells);

    applyBindings(
      belegung,
      SLOT_COUNT,
      (spellId) => this.combat.indexOf(spellId),
      (slot, index) => {
        this.session.bind(slot, index);
      },
    );
  }

  /**
   * Einen Zauber auf einen Platz legen (Issue #264, AC1 und AC5).
   *
   * **Drei Dinge in einer Reihenfolge, und die Reihenfolge zählt:**
   *
   * 1. Die Belegung ändern — abgelehnt wird sie, wenn Zauber oder Platz nicht
   *    frei sind; dann geschieht gar nichts.
   * 2. Ablegen, damit sie den Neustart überlebt (AC1).
   * 3. **Auf die Sitzung übertragen.** Ohne diesen Schritt wirkt der Kampf
   *    weiter den alten Zauber — der Fall, der still bricht: Der Spieler
   *    merkt es erst im Kampf und hält es für einen Fehler in seiner
   *    Rotation.
   *
   * @returns ob die Belegung wirklich geändert wurde
   */
  bindSpellToSlot(spellId: string, slot: number): boolean {
    const stand = this.persistence.state;
    const vorher = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const gefuellt =
      Object.keys(vorher).length > 0 ? vorher : defaultBindings(stand.level, this.combat.spells);
    const nachher = bindSpell(gefuellt, spellId, slot, stand.level, this.combat.spells);
    if (nachher === gefuellt) return false;

    this.persistence.persistBindings(nachher);
    this.applyBindings();
    return true;
  }

  /**
   * Den Belegungs-Bereich bauen (Issue #264).
   *
   * **Nach jeder Änderung neu**, wie die Aufstiegs-Anzeige: Ein einmal
   * gebauter Bereich zeigte, was beim Aufbau galt. Bei #248 hat genau das eine
   * falsche Zahl auf dem Bildschirm stehen lassen, während alle Prüffälle grün
   * waren.
   */
  /**
   * Die Boss-Wahl (Issue #613).
   *
   * Der Bereich bekommt die Liste fertig gefiltert und meldet nur zurück,
   * **welchen** Boss der Spieler will. Was daraufhin geschieht, steht in
   * {@link selectBoss} — hier, nicht dort.
   */
  private buildBossSection(): HTMLElement {
    // **Der Spielstand entscheidet mit** (12.7): Eine Instanz ist erst offen,
    // wenn die davor vollständig bezwungen ist. Ohne dieses zweite Argument
    // stünde `instance-3` sofort in der Auswahl — die Freischaltung wäre
    // gebaut, geprüft und nicht angeschlossen.
    const bosse = waehlbareBosse(BOSSES, this.persistence.state.defeatedBosses);
    return buildBossSelect(bosse, this.combat.boss.id, (bossId) => {
      this.selectBoss(bossId);
    });
  }

  /**
   * Einen anderen Boss übernehmen und in die Vorbereitung gehen (Issue #613).
   *
   * Derselbe Weg wie bei {@link setItems}: Der laufende Kampf wird
   * mitgeführt, nicht erst beim nächsten Neuladen. Zusätzlich muss die Sitzung
   * zurück — ein Wechsel mitten im Kampf ließe Zeit, Punkte und Phase des
   * alten Gegners stehen.
   *
   * **Die Reihenfolge ist nicht beliebig.** `switchBoss` vor `backToMenu`:
   * Ersteres setzt den Kampf zurück und leitet Zeitgeber und Widerstände aus
   * dem NEUEN Boss ab, letzteres die Sitzung darum herum. Umgekehrt liefe die
   * Sitzung kurz gegen den neuen Boss mit den Zählern des alten.
   *
   * **Was hier BEWUSST fehlt:** `dataProblems` bleibt stehen. Das Feld ist
   * `readonly` und wird beim Aufbau aus Schule und Start-Boss gefüllt; nach
   * einem Wechsel nennt es also weiterhin die Probleme des vorigen. Für die
   * kleinste Fassung ist das hinnehmbar — kein ausgelieferter Boss hat welche
   * (die Prüfung in `parseBoss` liefe sonst rot). Wer den Wechsel um eigene
   * Bosse erweitert, muss das Feld beweglich machen.
   *
   * Unbekannte Kennungen werden still übergangen statt zu werfen: Die Liste
   * entsteht aus `BOSSES`, ein Treffer daneben ist also kein Spielerfehler,
   * sondern ein Programmfehler — und ein abgestürzter Bildschirm wäre die
   * schlechtere Antwort darauf als ein Knopf, der nichts tut.
   */
  selectBoss(bossId: string): void {
    const gewaehlt = bossById(bossId);
    if (gewaehlt === undefined) return;
    this.combat.switchBoss(gewaehlt);
    // **Der Vorbereitungs-Bildschirm muss mit** (im Browser gefunden, nicht in
    // einem Prüffall): Er nannte weiter den Boss, den der Spieler eben
    // abgewählt hatte, während das HUD über dem Ring schon den neuen führte.
    // Die Widerstandszeilen entstehen dabei über `resistanceLines` — dieselbe
    // Stelle wie beim Aufbau, damit „hält ab" nicht zweimal entschieden wird.
    this.overlay.setBoss(gewaehlt.name, resistanceLines(gewaehlt.resistances));
    this.backToMenu();
    this.refreshHome();
    this.home.hide();
    // **Die geführte Einführung** (AC4 von #811, GAME-DESIGN 17.3). Zuletzt,
    // nach `home.hide()`: Sie liegt über dem Vorbereitungs-Bildschirm, den der
    // Spieler jetzt sieht — „an Ort und Stelle" heisst dort, wo die Mechanik
    // gleich auftritt.
    this.showIntroductionFor(gewaehlt);
  }

  /**
   * Die Einführung zur ersten noch unbekannten Mechanik dieses Bosses.
   *
   * # Der Auslöser ist die WAHL, nicht der Takt im Kampf
   *
   * 17.3 sagt „beim ersten Auftreten einer Mechanik hält das Spiel kurz an".
   * Hier hält nichts an, weil noch nichts läuft: Zwischen Wahl und erstem Takt
   * liegt der Vorbereitungs-Bildschirm, und der Spieler begegnet der Mechanik
   * genau hier zum ersten Mal.
   *
   * **Warum nicht mitten im Kampf:** Der Kampf ist der Gegenstand der Parität.
   * Der Simulator kennt kein Anhalten; ein Halt im Spiel, den `sim/` nicht
   * nachvollzieht, wäre eine Regel, die nur eine Seite kennt — und die bricht
   * nicht sofort, sondern beim ersten Lauf, in dem jemand pausiert. Diese
   * Auslegung liegt dem Auftraggeber vor; wandert der Auslöser, wandert diese
   * Methode und sonst nichts.
   *
   * # Höchstens eine, und Überspringen zählt
   *
   * `nextIntroduction` liefert die erste ungesehene — mehrere Fenster
   * übereinander wären keine Führung, sondern ein Stapel. Beide Knöpfe
   * vermerken sie als gesehen, weil `buildIntroduction` beide gleich behandelt:
   * Wer überspringt, will sie nicht wiedersehen.
   */
  /**
   * Den frisch freigeschalteten Archiv-Eintrag zeigen (Issue #85, AC1).
   *
   * **Als benannte Methode und nicht als Rückruf im Konstruktor.** Dieselbe
   * Lehre wie bei `comparisonForResult` (#82), `openHome` (#85) und
   * `parseSets` (#780): Was namenlos ist, kann kein Fall fassen — die Mutation
   * „zeig nie etwas" bliebe dann grün.
   *
   * # `null` heisst dreierlei und für den Spieler dasselbe
   *
   * Kein Sieg, kein Eintrag für diesen Boss, oder er war schon freigeschaltet.
   * In allen drei Fällen erscheint nichts, und der dritte IST der zweite Teil
   * von AC1: beim zweiten Sieg nicht mehr. Die Entscheidung darüber fällt in
   * `nextAutomaticEntry` und nicht hier — zwei Stellen, die befinden, wären
   * eine zu viel.
   *
   * # Die Reihenfolge gegen die Mechanik-Einführung
   *
   * Sie ist gegenstandslos, und das ist gemessen statt behauptet: Die
   * Einführung erscheint in `selectBoss` (ihrem einzigen Aufrufer), also bei
   * der WAHL des Bosses und auf dem Vorbereitungs-Bildschirm. Dieses Fenster
   * erscheint in `session.onFinish`, also am KAMPFENDE und auf der
   * Auswertung. Zwischen beiden liegt ein ganzer Kampf, und sie hängen an
   * verschiedenen Bildschirmen (`prepare` gegen `result`).
   *
   * **Wenn dieser Absatz eines Tages nicht mehr stimmt** — etwa weil eine
   * Einführung mitten im Kampf ausgelöst wird —, dann ist die Reihenfolge zu
   * entscheiden, und zwar für den Archiv-Text zuerst: Er gehört zum eben
   * Erreichten, die Einführung zum Kommenden.
   */
  private showUnlockedEntry(record: RecordResult): void {
    const eintrag = record.unlockedEntry;
    if (eintrag === null) return;
    this.overlay.showArchiveEntry(buildEntryPopup(eintrag, () => undefined));
  }

  private showIntroductionFor(boss: BossDef): void {
    const stand = { seenMechanicIds: this.persistence.state.seenMechanicIds };
    for (const mechanik of mechanikenVon(boss)) {
      const faellig = nextIntroduction(ALL_INTRODUCTIONS, stand, mechanik);
      if (faellig === null) continue;
      this.overlay.showIntroduction(
        buildIntroduction(faellig, (mechanicId) => {
          // **Der Spielstand muss es annehmen, nicht nur die Rechnung.** Genau
          // hier ist bei #85 die Kette gerissen: `recordRun` schaltete den
          // Eintrag frei, und `persist` übertrug ihn nicht — ohne Fehler und
          // ohne Hinweis, bei 2489 grünen Fällen.
          this.persistence.persistSeenMechanics(
            markSeen({ seenMechanicIds: this.persistence.state.seenMechanicIds }, mechanicId)
              .seenMechanicIds,
          );
        }),
      );
      return;
    }
  }

  /**
   * Der Zugang zum Trainingsraum (E7-4, GAME-DESIGN 19.2).
   *
   * **Wie die Boss-Wahl gebaut, nicht daneben:** Der Bereich meldet nur den
   * Wunsch; dass daraufhin ein Kampf beginnt, entscheidet diese Datei. Der
   * Unterschied zu `selectBoss` ist die Herkunft des Gegners — die Puppe steht
   * in keiner Datendatei, sie wird gerechnet.
   */
  private buildTrainingSection(): HTMLElement {
    const wurzel = document.createElement('div');
    wurzel.className = 'training-section';
    const knopf = document.createElement('button');
    knopf.type = 'button';
    knopf.className = 'training-start';
    // Spieltexte englisch (Design-Gesetz 5).
    knopf.textContent = 'Enter Training Room';
    knopf.addEventListener('click', () => {
      this.startTraining();
    });
    wurzel.appendChild(knopf);
    return wurzel;
  }

  /** Einen Uebungsdurchgang gegen die Puppe beginnen. */
  private startTraining(): void {
    const puppe = buildDummy(DEFAULT_DUMMY, this.combat.school.school);
    this.combat.switchBoss(puppe);
    this.overlay.setBoss(puppe.name, resistanceLines(puppe.resistances));
    this.backToMenu();
    this.refreshHome();
    this.home.hide();
  }

  private buildLoadoutSection(): HTMLElement {
    const stand = this.persistence.state;
    const gespeichert = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const belegung =
      Object.keys(gespeichert).length > 0
        ? gespeichert
        : defaultBindings(stand.level, this.combat.spells);

    return buildComposition({
      bindings: belegung,
      level: stand.level,
      spells: this.combat.spells,
      onBind: (spellId, slot) => {
        const ok = spellId === null ? this.clearSlotAt(slot) : this.bindSpellToSlot(spellId, slot);
        // Der Bereich wird neu gebaut, damit die Anzeige und der Kampf
        // dasselbe sagen — auch die Plätze, die sich mitverändert haben.
        if (ok) this.refreshLoadout();
        return ok;
      },
    });
  }

  /**
   * Die Ausrüst-Ansicht (Issue #453).
   *
   * **Das fehlende Stück der Kette.** `setItems` gab es seit #421a, aber nur
   * das Dev-Werkzeug rief es — und das wird im Auslieferungs-Bau
   * herausgeschnitten (GAME-DESIGN 19, D.3). Ein Fund landete im Beutel und
   * blieb dort, weil es keinen Weg gab, ihn anzulegen.
   *
   * Der Rückruf zeichnet **nicht selbst** neu: `setItems` ruft `refreshHome`,
   * und das baut diesen Bereich mit. Zweimal neu zu bauen wäre nicht falsch,
   * aber es liesse offen, welcher der beiden Wege der gemeinte ist.
   */
  private buildGearSection(): HTMLElement {
    return buildGearView({
      items: this.persistence.state.items,
      onEquip: (index, slot) => {
        const items = this.persistence.state.items;
        // `slot` kommt aus der Zeile, in der gewählt wurde — beim Ablegen ist
        // er die einzige Angabe, denn dort gibt es kein gewähltes Stück.
        const neu =
          index === null ? unequip(items, equippedBySlot(items)[slot] ?? -1) : equip(items, index);
        if (neu === null) return false;
        this.setItems(neu);
        return true;
      },
    });
  }

  /**
   * Die Stufe setzen — nur fuer Nachweise (GAME-DESIGN 19.2; Issue #264).
   *
   * Schreibt **nur, was erreicht wurde**: Stufe und Erfahrung auf null. Alles
   * andere — Grundwerte, freigeschaltete Zauber, Plaetze — wird daraus
   * gerechnet, wie im Spiel (#248, AC5). Ein Werkzeug, das die Folgen setzt
   * statt der Ursache, zeigte einen Zustand, den das Spiel nie erreicht.
   *
   * Danach wird die Belegung neu uebertragen: Auf einer hoeheren Stufe sind
   * mehr Plaetze offen, und die muessen gebunden werden, sonst zeigt die
   * Anzeige mehr, als der Kampf kennt.
   */
  /**
   * Talente verteilen (Issue #417).
   *
   * **Der Weg vom Spielstand in den Kampf, in beide Richtungen.** Wer verteilt,
   * schreibt in den Stand UND setzt den laufenden Kampf nach — sonst rechnete
   * der naechste Kampf mit dem alten Stand, bis die Seite neu laedt.
   *
   * Die Pruefung, ob die Verteilung zulaessig ist, liegt beim Aufrufer
   * (`allocationProblem`): Hier wird ausgefuehrt, nicht entschieden.
   */
  /**
   * Ausrüstung anlegen oder ablegen (Issue #421a).
   *
   * Derselbe Weg wie bei den Talenten: in den Spielstand UND in den laufenden
   * Kampf. Wer nur den Stand schriebe, rechnete bis zum nächsten Neuladen mit
   * den alten Werten weiter.
   */
  setItems(items: readonly OwnedItem[]): void {
    this.persistence.persistItems(items);
    this.combat.setWorn(wornFrom(items));
    // **Beide Wege, sonst hinkt einer nach.** Wer hier nur `setWorn` ruft,
    // zieht die Zahlen nach und lässt die Effekte auf dem Stand des letzten
    // Neuladens stehen — genau der Fehler, vor dem der Kommentar über dieser
    // Methode für den Spielstand warnt, nur eine Ebene tiefer.
    this.combat.setSetEffects(rotationEffectsFrom(items));
    this.refreshHome();
  }

  setTalents(allocation: Readonly<Record<string, number>>): void {
    this.persistence.persistTalents(allocation);
    this.combat.setTalents(allocation);
    this.refreshHome();
  }

  /**
   * Einen Talent-Rang setzen — der Weg, den die Ansicht nimmt (Issue #802).
   *
   * **Die Regel steht nicht hier**, sondern in `allocationProblem`. Diese
   * Methode fragt sie und führt aus; eine zweite Fassung der Regel in der
   * Oberfläche wäre die Stelle, an der beide auseinanderlaufen.
   *
   * Gibt `false` zurück, wenn die Verteilung abgelehnt wurde — dann bleibt der
   * Stand, wie er war, und die Ansicht zeigt weiter die alte Zahl.
   */
  private applyTalentRank(talentId: string, ranks: number): boolean {
    const neu = { ...this.persistence.state.talentPoints, [talentId]: ranks };
    // Ein Talent mit null Rängen steht nicht im Spielstand — sonst wüchse er
    // mit jedem Zurücknehmen um einen Eintrag, der nichts aussagt.
    if (ranks <= 0) delete neu[talentId];
    if (allocationProblem(neu, talentPointsAtLevel(this.persistence.state.level)) !== null) {
      return false;
    }
    this.setTalents(neu);
    return true;
  }

  private buildTalentSection(): HTMLElement {
    return buildTalentView({
      allocation: this.persistence.state.talentPoints,
      punkte: talentPointsAtLevel(this.persistence.state.level),
      // **Im Kampf wird nicht verteilt** (#425): `Combat.setTalents` sperrt
      // sich, sobald der Kampf begonnen hat. Ein Knopf, der dann noch
      // anbietet, verspräche etwas, das nicht ankommt.
      //
      // **Gefragt wird die SITZUNG, nicht der Kampf.** `combat.fightState`
      // steht schon vor dem ersten Takt auf `FIGHT_RUNNING` (der Kommentar an
      // dem Feld sagt es ausdrücklich) — eine Sperre darauf griffe immer, und
      // die Ansicht wäre tot, bevor sie einmal benutzt wurde.
      gesperrt: this.session.phase === PHASE_FIGHTING || this.session.phase === PHASE_COUNTDOWN,
      onChange: (talentId, ranks) => this.applyTalentRank(talentId, ranks),
    });
  }

  setPlayerLevel(level: number): void {
    this.persistence.persistLevel(level);
    // Der Kampf rechnet ab jetzt mit der neuen Stufe (Issue #289).
    this.combat.setPlayerLevel(level);
    this.applyBindings();
    this.refreshLoadout();
    this.refreshHome();
  }

  /** Den Belegungs-Bereich neu bauen. */
  private refreshLoadout(): void {
    this.home.replaceSection('loadout', this.buildLoadoutSection());
  }

  /**
   * Einen Platz räumen (Issue #264).
   *
   * Getrennt von `bindSpellToSlot`, weil es etwas anderes ist: Ein leerer
   * Platz ist kein Zauber mit leerem Namen.
   */
  clearSlotAt(slot: number): boolean {
    const stand = this.persistence.state;
    const vorher = sanitize(this.persistence.save.bindings, stand.level, this.combat.spells);
    const gefuellt =
      Object.keys(vorher).length > 0 ? vorher : defaultBindings(stand.level, this.combat.spells);
    const nachher = clearSlot(gefuellt, slot);
    if (Object.keys(nachher).length === Object.keys(gefuellt).length) return false;

    this.persistence.persistBindings(nachher);
    this.applyBindings();
    return true;
  }

  /**
   * Die Aufstiegs-Anzeige neu rechnen (Issue #248, AC6).
   *
   * **Gerechnet, nicht gespeichert — auch auf dem Bildschirm.** Der Spielstand
   * trägt Stufe und Erfahrung; alles andere entsteht hier bei jedem Öffnen
   * neu. Ein einmal gebauter Bereich zeigte sonst, was beim Start galt.
   */
  /**
   * Das Player Home öffnen (Issue #188).
   *
   * **Vor dem Zeigen neu rechnen.** Der Bildschirm entsteht einmal beim Start;
   * ohne das stünde dort, was damals galt. Im Browser gesehen: nach einem
   * gewonnenen Kampf weiterhin „0 / 200" — und bei #85 ein leeres Archiv,
   * obwohl der Eintrag im Spielstand stand.
   *
   * **Als benannte Methode und nicht als anonymer Rückruf im Overlay-Aufbau.**
   * Was namenlos ist, kann kein Fall fassen: Die Mutation „frische nicht auf"
   * bliebe sonst grün. Dieselbe Lehre wie bei `comparisonForResult` (#82) und
   * `parseSets` (#780), zum dritten Mal an diesem Tag.
   */
  openHome(): void {
    this.refreshHome();
    this.home.show();
  }

  private refreshHome(): void {
    const stand = { level: this.persistence.state.level, xp: this.persistence.state.experience };
    this.home.replaceSection(
      'levels',
      buildLevelOverview(levelOverview(stand), levelBarFill(stand)),
    );
    // **Die Boss-Wahl gehört dazu** (Issue #613): Sie markiert den laufenden
    // Boss und sperrt seinen Knopf. Ohne das Neuzeichnen zeigte die Markierung
    // nach einem Wechsel weiter auf den vorigen — derselbe Fehler, den die
    // beiden Kommentare darunter für Stufe und Werte beschreiben.
    this.home.replaceSection('targets', this.buildBossSection());
    // **Die Werte-Übersicht gehört dazu** (Issue #289). Sie zeigt seit dieser
    // Änderung die Werte DER STUFE — ein einmal gebauter Bereich zeigte, was
    // beim Aufbau galt, und im Browser stand nach einem Aufstieg weiter der
    // Grundwert.
    //
    // Genau dieser Fehler stand schon in #248 auf dem Bildschirm, während
    // alle Prüffälle grün waren: Was ein Dokument braucht, sieht Vitest nicht.
    this.home.replaceSection(
      'stats',
      buildStatOverview(
        statOverview(baseStats(this.persistence.state.level), this.combat.wornValues),
      ),
    );
    // **Die Ausrüstung gehört dazu** (Issue #453). Sie ist die einzige
    // Ansicht, die den Spielstand ändert, während sie sichtbar ist — nach
    // jedem Anlegen muss dastehen, was gilt, und der Beutel daneben muss das
    // Stück verloren haben. Ohne diese Zeile zeigte die Liste weiter die alte
    // Wahl, und die Werte darüber die neue.
    this.home.replaceSection('gear', this.buildGearSection());
    // **Die Talente gehören dazu** (Issue #802), und zwar aus zwei Gründen:
    // Nach einem Stufenaufstieg stehen mehr Punkte zur Verfügung, und nach
    // jedem gesetzten Punkt ändert sich, was noch anwählbar ist.
    //
    // Ohne diese Zeile war die Ansicht beim Aufbau eingefroren: Auf Stufe 1
    // gibt es null Punkte, also waren alle Knöpfe gesperrt — und sie blieben
    // es, auch nachdem der Spieler Stufe 2 erreicht hatte. Gefunden hat es
    // `talent-oberflaeche-anschluss.test.ts`, nicht das Lesen.
    this.home.replaceSection('talents', this.buildTalentSection());
    // **Und der Verlauf** (Issue #82) — dieselbe Klasse wie alles darüber, und
    // sie hatte den Bereich erwischt, der von Haus aus über die Zeit spricht.
    //
    // Im Browser gemessen: Spielstand 64 Läufe, Ansicht „64 runs". Einen
    // weiteren Kampf gefahren, dann ins Player Home — Spielstand 65, Ansicht
    // weiter „64 runs". Nach einem Neuladen stimmte es wieder. Der Grund ist
    // der aus den Kommentaren darüber: `buildHistoryView` lief EINMAL beim
    // Aufbau, und ohne diese Zeile zeigte der Verlauf für immer den Stand vom
    // Seitenaufruf.
    //
    // Der zuletzt gefahrene Lauf fehlte damit IMMER — ausgerechnet der, wegen
    // dem man nachsieht.
    this.home.replaceSection('history', buildHistoryView(this.persistence.state.runHistory));
    // **Und das Archiv** (Issue #85), aus demselben Grund wie alles darüber:
    // Ein Eintrag wird nach einem Sieg frei, und der Bildschirm entstand beim
    // Start. Ohne diese Zeile stünde dort weiter „Nothing recorded yet", bis
    // der Spieler die Seite neu lädt.
    //
    // Im Browser gesehen: Boss gefallen, `unlockedEntryIds` im Stand gesetzt,
    // Archiv leer — und nach einem Neuladen war der Eintrag da. Genau der
    // Fehler, den die Kommentare darüber für Stufe, Werte und Ausrüstung
    // beschreiben.
    this.home.replaceSection('archive', this.buildArchiveSection());
    // **Und die Erklärfenster, aus demselben Grund wie alles darüber.** Jeder
    // `replaceSection`-Aufruf wirft den alten Teilbaum weg — mitsamt der
    // Zuhörer, die `attachHelp` daran gehängt hatte. Ohne diese Zeile hätten
    // die Werte nach dem ersten Aufstieg kein Fenster mehr, und die Boss-Wahl
    // nach dem ersten Wechsel keins.
    //
    // Genau die Klasse, die bei #85 zweimal 2489 grüne Fälle durchgelassen hat
    // und erst im Browser aufgefallen ist.
    this.attachHelpToHome();
  }

  /**
   * Die Erklärfenster an alles hängen, was im Player Home `data-help` trägt.
   *
   * Als eigene Methode, weil sie an zwei Stellen gebraucht wird — beim Aufbau
   * und nach jedem Auffrischen. Und als **benannte** Methode, damit ein
   * Prüffall sie fassen kann: Was in einem anonymen Rückruf steckt, ist von
   * aussen weder aufrufbar noch beobachtbar, und die Mutation läuft grün durch
   * (gemessen bei `comparisonForResult` in #82 und `parseSets` in #780).
   */
  private attachHelpToHome(): void {
    attachHelp(this.home.root, ALL_HELP_ENTRIES);
  }

  /**
   * Der Archiv-Abschnitt, aus dem aktuellen Spielstand (Issue #85).
   *
   * Als eigene Methode, weil er an zwei Stellen gebraucht wird — beim Aufbau
   * und bei jedem Auffrischen. Zwei Aufrufe von `buildArchive` mit denselben
   * Zutaten wären zwei Stellen, die auseinanderlaufen können.
   */
  private buildArchiveSection(): HTMLElement {
    return buildArchiveView(
      buildArchive(ALL_ARCHIVE_ENTRIES, {
        unlockedEntryIds: this.persistence.state.unlockedEntryIds,
      }),
    );
  }

  ready(): boolean {
    // „Ready" ist eine Nutzergeste, und zwar die erste des Kampfes. Erst danach
    // gibt der Browser den Klang-Kontext frei — geschähe das erst beim ersten
    // Zauber, bliebe der Countdown stumm.
    this.audio.unlock();
    // **Die Stufe gilt ab dem nächsten Kampf** (Issue #289). Der Kampf
    // entsteht einmal beim Start des Spiels; ohne diese Zeile rechnete er mit
    // der Stufe, die beim Programmstart galt, und der Aufstieg wäre erst nach
    // einem Neuladen spürbar.
    this.combat.setPlayerLevel(this.persistence.state.level);
    return this.session.beginCountdown(this.loop.timeMs);
  }

  /** Esc-Menü öffnen oder schließen. Nur während des Kampfes sinnvoll. */
  togglePause(): void {
    const phase = this.session.phase;
    if (phase !== PHASE_FIGHTING && phase !== PHASE_COUNTDOWN) return;
    this.overlay.setPaused(!this.overlay.isPaused);
  }

  resume(): void {
    this.overlay.setPaused(false);
  }

  /** „Wipe Run" — gilt als Ende und führt zur selben Auswertung wie ein Sieg. */
  wipe(): boolean {
    const wiped = this.session.wipe(this.loop.timeMs);
    if (wiped) this.overlay.setPaused(false);
    return wiped;
  }

  /** Neuer Versuch: zurück in die Vorbereitung und sofort bereit machen. */
  /**
   * Der Vergleich des letzten Laufs mit dem Bestlauf (Issue #82).
   *
   * **Als benannte Methode und nicht als anonymer Rückruf.** Der Unterschied
   * ist nicht Stil, sondern Prüfbarkeit: Steckt die Entscheidung in einer
   * Funktion ohne Namen, kann kein Fall sie fassen — dann ist zwar
   * angeschlossen, aber nichts hält den Anschluss fest. Gemessen: Mit dem
   * anonymen Rückruf blieb die Mutation „gib immer `null` zurück" durch die
   * ganze Suite grün. Dieselbe Lehre wie bei `parseSets` in #780.
   *
   * `null` heisst: kein Vergleich möglich. Drei Fälle, für den Spieler
   * derselbe — es gab noch keinen Lauf, es gibt keinen Bestlauf für diesen
   * Boss, oder der gespeicherte stammt aus einer Fassung vor #82.
   */
  comparisonForResult(): readonly ComparedMetric[] | null {
    const summary = this.lastSummary;
    if (summary === null) return null;
    // Mit demselben Mass suchen, das `recordRun` schreiben würde. Passt es
    // nicht — der Boss hat `runaway` bekommen oder verloren —, verwirft
    // `usableBest` den Eintrag, statt Sekunden gegen Prozent zu halten.
    const { entry } = usableBest(
      this.persistence.state.bestRuns,
      this.combat.boss.id,
      1,
      summary.heldPercent === null ? 'reserve' : 'held',
    );
    return compareRuns(summary, comparableFromBest(entry));
  }

  /**
   * Die Schadenskurve des Bestlaufs — die zweite Kurve aus AC1 (Issue #82).
   *
   * **Als benannte Methode und nicht als anonymer Rückruf**, aus demselben
   * Grund wie `comparisonForResult` darüber: Was namenlos ist, kann kein Fall
   * fassen. Bei `comparisonForResult` blieb die Mutation „gib immer `null`
   * zurück" durch die ganze Suite grün, solange sie namenlos war.
   *
   * **Denselben Eintrag wie der Vergleich, über `usableBest`**: Die zweite
   * Kurve muss zu den Zahlen daneben gehören. Wer sie aus einem anders
   * gewählten Bestlauf holte, zeichnete eine Linie, die zu keiner Zeile der
   * Tabelle passt.
   *
   * `null` heisst: keine zweite Kurve — kein Bestlauf, ein Spielstand von vor
   * dieser Änderung, oder eine Kurve, deren Form nicht trägt. Für den Spieler
   * ist das dasselbe: Es wird eine Kurve gezeichnet statt zwei.
   */
  bestCurveForResult(): RunCurve | null {
    const summary = this.lastSummary;
    if (summary === null) return null;
    const { entry } = usableBest(
      this.persistence.state.bestRuns,
      this.combat.boss.id,
      1,
      summary.heldPercent === null ? 'reserve' : 'held',
    );
    return curveFromBest(entry);
  }

  restart(): void {
    this.backToMenu();
    this.ready();
  }

  /**
   * Zurück in die Vorbereitung.
   *
   * Ein Menü im eigentlichen Sinn gibt es noch nicht — es entsteht mit einem
   * späteren Epic. Der Knopf trägt den Namen aus GAME-DESIGN 10 und führt
   * dorthin, wo der Spieler wieder anfangen kann.
   */
  backToMenu(): void {
    this.overlay.setPaused(false);
    this.session.reset();
    this.impact.reset();
    this.loop.reset();
  }

  private handleUiKey(event: KeyboardEvent): void {
    if (event.ctrlKey || event.altKey || event.metaKey) return;
    // **Der Beute-Moment ist vollständig überspringbar** (GAME-DESIGN 13.6),
    // auch der des Legendary. Er wird ZUERST behandelt: Wer die Taste drückt,
    // will weiter — und ein Moment, der eine Eingabe schluckt, wäre beim
    // zweiten Mal eine Zumutung.
    if (event.code === 'Space' && this.overlay.skipLoot()) {
      event.preventDefault();
      return;
    }
    if (event.code === 'Escape') {
      event.preventDefault();
      this.togglePause();
      return;
    }
    if (event.code === 'Enter' || event.code === 'NumpadEnter') {
      event.preventDefault();
      if (this.session.phase === PHASE_PREPARE) this.ready();
      else if (this.session.phase === PHASE_ENDED) this.restart();
    }
  }

  start(): void {
    if (this.dataProblems.length > 0) {
      // Nicht abbrechen: Ein einzelner kaputter Datensatz darf nicht das ganze
      // Spiel verhindern — aber sichtbar sein muss es.
      console.error('[rota] Spieldaten fehlerhaft:', this.dataProblems);
    }
    this.input.attach();
    window.addEventListener('keydown', this.boundUiKey);
    this.loop.start();
    // Beides läuft nebenher: Das Spiel ist sofort spielbar, die Beschriftung
    // rückt nach, sobald der Browser Auskunft gibt, und der Klang, sobald geladen.
    void this.labels.refresh();
    void this.audio.init();
  }

  stop(): void {
    this.loop.stop();
    this.input.detach();
    window.removeEventListener('keydown', this.boundUiKey);
  }
}
