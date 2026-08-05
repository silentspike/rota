/**
 * Vorbereitung, Esc-Menü und Auswertung (GAME-DESIGN 10, 18.1).
 *
 * **HTML, nicht Canvas.** GAME-DESIGN 18.1 teilt bewusst auf: Canvas für den
 * Kampf, HTML für Menüs und Auswertung. Der Grund steht in 18.2 — jede Änderung
 * am HTML löst eine Neuberechnung des Layouts aus. Deshalb wird hier **nur bei
 * einem Phasenwechsel** angefasst, nie je Bild.
 *
 * Die Maus ist hier erlaubt: Design-Gesetz 2 verbietet das Klicken von **Zaubern**,
 * nicht das Bedienen von Menüs (GAME-DESIGN 8.1: „Die Maus dient nur Menüs,
 * Talentbaum und Zusammenstellung").
 *
 * Alle Texte englisch (Design-Gesetz 5).
 */
import {
  OUTCOME_DEATH,
  OUTCOME_OVERPRESSURE,
  OUTCOME_VICTORY,
  OUTCOME_WIPE,
  PHASE_ENDED,
  PHASE_PREPARE,
} from '../core/phases.js';
import type { RunSummary } from '../core/summary.js';
import type { LossReport } from '../analysis/loss-sources.js';
import { buildLossView } from '../analysis/analysis-view.js';
// Ohne diesen Import erscheint die Rangliste ungestylt: Die Datei wurde bis
// #280 nur vom Pruefblatt geladen (docs/analysis-check/), das sie selbst
// einbindet.
import '../analysis/analysis.css';
import { classOfStat, formatStat, type StatLine } from '../gear/format.js';
import { labelFor, presentationFor, LootMoment } from '../loot/moment.js';
import type { Comparison } from '../loot/compare.js';

export interface OverlayActions {
  onReady(): void;
  onResume(): void;
  onWipe(): void;
  onRestart(): void;
  onBackToMenu(): void;
  /**
   * Das Player Home oeffnen (GAME-DESIGN 16.1, Issue #188).
   *
   * Das Overlay OEFFNET es nicht selbst: Es weiss nicht, was darin steht, und
   * soll es nicht wissen. Es meldet nur den Wunsch.
   */
  onOpenHome(): void;
}

/**
 * Die Fortschrittsmarke, fertig zum Anzeigen (Issue #187).
 *
 * FERTIG heisst: Der Satz ist schon gebildet. Die Ansicht rechnet nicht und
 * formuliert nicht — sonst gaebe es zwei Stellen, die aus denselben Zahlen
 * einen Satz machen, und die zweite wird beim Nachziehen vergessen. Dasselbe
 * Muster wie bei der laufenden Anzeige im Trainingsraum.
 *
 * `null` heisst: **nichts zeigen**. Nicht „0 %", nicht „—". Der erste Kampf
 * eines neuen Spielstands hat keine Marke, und eine Anzeige, die dann eine
 * Null zeigt, behauptet einen Fortschritt, den es nicht gibt.
 */
export interface MarkDisplay {
  /** Ein Satz fuer den Spieler, englisch (Design-Gesetz 5). */
  readonly sentence: string;
  readonly attempts: number;
}

/** Was die Auswertung anzeigt. */
export interface ResultView {
  readonly outcome: number;
  /** Verstrichene Kampfzeit in ms. */
  readonly elapsedMs: number;
  /** Restzeit bis zum Überdruck in ms. Bei Sieg die Reserve. */
  readonly remainingMs: number;
  /**
   * Was einen Sieg vom anderen trennt, bei einem Boss mit umgekehrtem Ziel.
   *
   * `null` heißt: gewöhnlicher Boss — dort trennt die Restzeit, die schon
   * dasteht (Issue #256).
   */
  readonly heldPercent: number | null;
  /**
   * Die Verlustquellen dieses Laufs, **fertig gerechnet** (Issue #280).
   *
   * `null` heißt: noch keine gerechnet — nicht „keine Verluste". Die
   * Unterscheidung zählt, weil eine leere Rangliste eine Aussage ist und eine
   * fehlende keine.
   *
   * Sie kommt aus `hud.ts` und nicht von hier: `analyzeRun` braucht den
   * `Combat`, und wer ihn dem Overlay gibt, lädt es ein, selbst zu rechnen.
   * Genau diese zweite Rechenstelle hat #256 hier entfernt.
   */
  readonly analysis: LossReport | null;
  readonly damage: number;
  /**
   * Der Fortschritt in Prozent, **fertig gerechnet**.
   *
   * Hier stand bis zum 2026-08-05 `bossHp`, und die Anzeige rechnete daraus
   * ihren eigenen Fortschritt: `100 * damage / bossHp`. Das war eine zweite
   * Stelle für dieselbe Entscheidung — und sie ist bei #256 stehen geblieben,
   * während `progressOf` die Umkehrung schon kannte. Im Browser zeigte die
   * Niederlage danach 100 %, obwohl die Kurzfassung 85 % meldete.
   *
   * **Die Zutat ist deshalb weg, nicht nur der Aufruf.** Wer die alte Formel
   * hier wieder aufschreiben wollte, fände `bossHp` nicht mehr vor. Das ist
   * derselbe Griff wie bei `applyBindings` (#264): Die Regel steht an EINER
   * Stelle, und die anderen bekommen das Ergebnis.
   */
  readonly progressPercent: number;
  readonly casts: number;
  /** Die Kurzfassung mit den drei Kennzahlen (GAME-DESIGN 15.1). */
  readonly summary: RunSummary | null;
  /**
   * Die Fortschrittsmarke, oder `null`.
   *
   * Sie ist der Grund, warum ein Spieler nach der zwanzigsten Niederlage
   * weitermacht (GAME-DESIGN 10.3) — und bis Issue #187 wurde sie gespeichert,
   * ohne dass er sie je zu sehen bekam.
   */
  readonly mark: MarkDisplay | null;
}

const OUTCOME_HEADLINES: Readonly<Record<number, string>> = {
  [OUTCOME_VICTORY]: 'REACTION CONTAINED',
  [OUTCOME_OVERPRESSURE]: 'OVERPRESSURE',
  [OUTCOME_WIPE]: 'RUN ABANDONED',
  [OUTCOME_DEATH]: 'VESSEL LOST',
};

function element(tag: string, className: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(label: string, onClick: () => void): HTMLButtonElement {
  const node = document.createElement('button');
  node.className = 'rota-button';
  node.type = 'button';
  node.textContent = label;
  node.addEventListener('click', onClick);
  return node;
}

function row(label: string): { root: HTMLElement; label: HTMLElement; value: HTMLElement } {
  const root = element('div', 'rota-row');
  // Die Beschriftung wird mitgegeben, weil sie bei einem Boss mit umgekehrtem
  // Ziel eine andere Größe benennt (Issue #256). Zwei Zeilenpaare
  // nebeneinander wären der „zweite Satz Anzeigen", den die Auflage verbietet
  // — dieselbe Zeile mit der richtigen Beschriftung ist die Antwort.
  const beschriftung = element('span', 'rota-row-label', label);
  root.appendChild(beschriftung);
  const value = element('span', 'rota-row-value', '—');
  root.appendChild(value);
  return { root, label: beschriftung, value };
}

export class Overlay {
  private readonly root: HTMLElement;
  private readonly prepare: HTMLElement;
  private readonly pause: HTMLElement;
  private readonly result: HTMLElement;
  private readonly resultHeadline: HTMLElement;
  private readonly markPanel: HTMLElement;
  private readonly markSentence: HTMLElement;
  private readonly markAttempts: HTMLElement;
  private readonly resultRows: Record<string, HTMLElement> = {};
  /** Die Beschriftungen — sie wechseln bei einem umgekehrten Ziel (#256). */
  private readonly resultLabels: Record<string, HTMLElement> = {};
  private readonly detail: HTMLElement;
  /** Die Werte-Karte des Vorbereitungs-Bildschirms (Issue E6-1, AC4). */
  private readonly gearCard: HTMLElement;
  /** Der Beute-Moment (Issue E6-4). */
  private readonly lootPanel: HTMLElement;
  /** Sein Zustand — hier, weil das Überspringen an der Anzeige hängt. */
  readonly loot = new LootMoment();
  private readonly analysisButton: HTMLButtonElement;
  private detailOpen = false;

  private shownPhase = -1;
  private shownOutcome = -1;
  private paused = false;

  /**
   * @param bossName Wie der Boss heißt, aus seinem Datensatz.
   *
   * **Nicht aus einer Konstante.** Hier stand bis #229 der Name des ersten
   * Tiegels fest im Aufruf, und dieselbe Zeichenkette ein zweites Mal in
   * `hud.ts`. Solange nur ein Boss spielbar war, stimmte sie zufällig.
   *
   * Der Name wird bewusst nicht ausgeschrieben: Ein Kommentar, der ihn
   * zitiert, ist selbst eine Abschrift.
   *
   * Das Overlay bekommt den Namen als Wert und nicht den Boss selbst: Es
   * zeichnet einen Vorbereitungs-Bildschirm und hat mit Kampfdaten nichts zu
   * tun. Wer ihm den ganzen Boss gäbe, lüde ihn ein, mehr daraus zu lesen.
   *
   * @param resistances Fertige Zeilen über die Widerstände (Issue #328) —
   *   **nicht** die Abbildung aus dem Datensatz. Dieselbe Regel wie beim
   *   Namen, ein Feld weiter: Das Overlay soll den Text setzen, nicht ihn
   *   herleiten. Wer ihm `Record<string, number>` gäbe, verlagerte die
   *   Entscheidung „ab wann heißt ein Faktor Widerstand" in die Anzeige.
   */
  constructor(
    parent: HTMLElement,
    bossName: string,
    actions: OverlayActions,
    resistances: readonly string[] = [],
  ) {
    this.root = element('div', 'rota-overlay');

    // --- Vorbereitung: keine Uhr, kein Zeitdruck (GAME-DESIGN 10) -----------
    this.prepare = element('div', 'rota-panel');
    this.prepare.appendChild(element('h1', 'rota-title', bossName.toUpperCase()));
    this.prepare.appendChild(
      element('p', 'rota-note', 'Prologue · no target until the run begins'),
    );
    this.prepare.appendChild(
      element(
        'p',
        'rota-note rota-note-dim',
        'Cooldowns run during preparation. The clock does not.',
      ),
    );
    // **Die Widerstände, VOR dem Kampf** (GAME-DESIGN 11.4, Issue #328):
    //
    // > Vorher sichtbar in der Boss-Übersicht — die Zusammenstellung ist
    // > bewusste Vorbereitung, keine böse Überraschung.
    //
    // Der Satz steht in derselben Ecke des Designs wie die Zusammenstellung
    // selbst: Wer erst im Kampf merkt, dass seine Schule abgewehrt wird, kann
    // nichts mehr daran ändern — die Leiste wird zwischen den Kämpfen gelegt.
    //
    // **Nichts, solange nichts anzuzeigen ist.** Ein leerer Kasten für „dieser
    // Boss hat keine Widerstände" wäre eine Zeile, die bei zehn von elf Bössen
    // nur Platz kostet.
    if (resistances.length > 0) {
      const karte = element('div', 'rota-resistances');
      karte.appendChild(element('h3', 'rota-resist-heading', 'Resistances'));
      for (const zeile of resistances) {
        karte.appendChild(element('p', 'rota-resist-line', zeile));
      }
      this.prepare.appendChild(karte);
    }

    // Die Werte-Karte. **Leer, solange es keine Gegenstände gibt** — der
    // Katalog steht noch nicht (GAME-DESIGN 22, vor E6). Sie zeigt sich erst,
    // wenn etwas anzuzeigen ist, und schiebt bis dahin nichts.
    this.gearCard = element('div', 'rota-gear-card');
    this.gearCard.hidden = true;
    this.prepare.appendChild(this.gearCard);

    this.prepare.appendChild(button('Ready', actions.onReady));
    // Das Player Home ist der Ort ZWISCHEN den Kaempfen -- erreichbar von
    // hier und von der Auswertung, nicht waehrend des Kampfes (AC1).
    this.prepare.appendChild(button('Player Home', actions.onOpenHome));
    this.prepare.appendChild(element('p', 'rota-hint', 'Enter'));
    this.root.appendChild(this.prepare);

    // --- Beute-Moment (GAME-DESIGN 13.6) -----------------------------------
    // HTML und nicht Canvas, wie Menüs und Auswertung (18.1). Er wird nur bei
    // einem Fund angefasst, nie je Bild.
    this.lootPanel = element('div', 'rota-loot');
    this.lootPanel.hidden = true;
    this.root.appendChild(this.lootPanel);

    // --- Esc-Menü ----------------------------------------------------------
    this.pause = element('div', 'rota-panel');
    this.pause.appendChild(element('h2', 'rota-title', 'PAUSED'));
    this.pause.appendChild(button('Resume', actions.onResume));
    this.pause.appendChild(button('Wipe Run', actions.onWipe));
    this.pause.appendChild(
      element('p', 'rota-hint', 'A wipe counts as an ending. No penalty either way.'),
    );
    this.root.appendChild(this.pause);

    // --- Auswertung: dieselbe für alle vier Ausgänge ------------------------
    this.result = element('div', 'rota-panel');
    this.resultHeadline = element('h2', 'rota-title', '');
    this.result.appendChild(this.resultHeadline);
    for (const [key, label] of [
      ['time', 'Time'],
      ['remaining', 'Reserve'],
      ['damage', 'Damage'],
      ['progress', 'Progress'],
      ['score', 'Score'],
    ] as const) {
      const made = row(label);
      this.result.appendChild(made.root);
      this.resultRows[key] = made.value;
      this.resultLabels[key] = made.label;
    }

    // Die Fortschrittsmarke steht VOR den Kennzahlen, direkt unter der
    // Ueberschrift (Issue #187). Sie ist die Antwort auf die Frage, mit der
    // ein Spieler nach einer Niederlage vor dem Bildschirm sitzt -- „bringt
    // das ueberhaupt was?". Weiter unten stuende sie unter Zahlen, die diese
    // Frage nicht beantworten.
    this.markPanel = element('div', 'rota-mark');
    this.markSentence = element('p', 'rota-mark-line', '');
    this.markAttempts = element('p', 'rota-mark-attempts', '');
    this.markPanel.appendChild(this.markSentence);
    this.markPanel.appendChild(this.markAttempts);
    this.result.appendChild(this.markPanel);

    // Die drei Kennzahlen aus GAME-DESIGN 15.1.
    this.result.appendChild(element('div', 'rota-divider'));
    for (const [key, label] of [
      ['idleWorst', 'Idle · worst spell'],
      ['lostTime', 'Time lost'],
      ['synergy', 'Synergy yield'],
      // Zwei Größen dazu (Issue #294). Sie beantworten „wie ist es gelaufen",
      // nicht „warum" — das ist die Frage dieser Ebene.
      ['taken', 'Damage taken'],
      // **„Pushed back", nicht „Ring held".** Zwei Zeilen mit „held"
      // nebeneinander — eine in Sekunden, eine in Ringpunkten — liest sich
      // als dieselbe Größe. Der Name nennt deshalb die HANDLUNG.
      ['pushedBack', 'Pushed back'],
    ] as const) {
      const made = row(label);
      this.result.appendChild(made.root);
      this.resultRows[key] = made.value;
      this.resultLabels[key] = made.label;
    }

    // "View Analysis" klappt den Leerlauf je Zauber auf. Die ausführliche
    // Auswertung mit Effizienz-Tabelle und Bestlauf-Vergleich ist Epic E7.
    this.detail = element('div', 'rota-detail');
    this.result.appendChild(this.detail);

    this.result.appendChild(button('Retry', actions.onRestart));
    this.analysisButton = button('View Analysis', () => {
      this.toggleDetail();
    });
    this.result.appendChild(this.analysisButton);
    this.result.appendChild(button('Back to Menu', actions.onBackToMenu));
    this.result.appendChild(button('Player Home', actions.onOpenHome));
    this.root.appendChild(this.result);

    parent.appendChild(this.root);
    this.hideAll();
  }

  dispose(): void {
    this.root.remove();
  }

  /** Ist das Esc-Menü offen? */
  get isPaused(): boolean {
    return this.paused;
  }

  setPaused(paused: boolean): void {
    if (this.paused === paused) return;
    this.paused = paused;
    this.shownPhase = -1; // erzwingt ein Neuzeichnen der Sichtbarkeit
  }

  /**
   * Sichtbarkeit an die Phase anpassen.
   *
   * Wird je Bild aufgerufen, fasst das HTML aber nur an, wenn sich etwas geändert
   * hat — sonst zahlte jedes Bild eine Layout-Neuberechnung (GAME-DESIGN 18.2).
   */
  update(phase: number, outcome: number, view: ResultView): void {
    if (phase === this.shownPhase && outcome === this.shownOutcome) return;
    this.shownPhase = phase;
    this.shownOutcome = outcome;

    this.detailOpen = false;
    this.detail.classList.remove('rota-visible');
    this.analysisButton.textContent = 'View Analysis';
    this.hideAll();
    if (phase === PHASE_PREPARE) {
      this.show(this.prepare);
      return;
    }
    if (phase === PHASE_ENDED) {
      this.fillResult(view);
      this.show(this.result);
      return;
    }
    if (this.paused) this.show(this.pause);
  }

  private fillResult(view: ResultView): void {
    this.resultHeadline.textContent = OUTCOME_HEADLINES[view.outcome] ?? 'RUN ENDED';
    this.resultHeadline.classList.toggle('rota-title-win', view.outcome === OUTCOME_VICTORY);
    const set = (key: string, text: string): void => {
      const node = this.resultRows[key];
      if (node !== undefined) node.textContent = text;
    };
    set('time', `${(view.elapsedMs / 1000).toFixed(2)} s`);
    // **Dieselbe Zeile, zwei Größen** — kein zweiter Satz Anzeigen (Auflage
    // aus #256). Bei einem gewöhnlichen Boss steht dort die Reserve; bei einem
    // mit umgekehrtem Ziel ist die immer 0,00 s, weil der Sieg das Ablaufen
    // der Frist IST. Dort trennt der Ring-Rest die Siege.
    if (view.heldPercent === null) {
      set('remaining', `${(view.remainingMs / 1000).toFixed(2)} s`);
    } else {
      set('remaining', `${view.heldPercent.toFixed(1)} %`);
    }
    set('damage', String(view.damage));
    // **Gerechnet wird woanders.** `progressOf` kennt die Umkehrung; diese
    // Zeile stellt sie nur dar.
    set('progress', `${view.progressPercent.toFixed(1)} %`);
    // Die Punkteformel ist noch nicht ausgearbeitet (GAME-DESIGN 22, vor Epic
    // E5). Eine erfundene Zahl wäre schlimmer als ein ehrlicher Strich.
    set('score', '—');

    // Die Marke: ganz weg, wenn es keine gibt. Ein leerer Block mit Trenner
    // saehe aus wie ein Fehler, eine „0 %" waere eine Luege (AC4).
    const mark = view.mark;
    this.markPanel.classList.toggle('rota-visible', mark !== null);
    if (mark === null) {
      this.markSentence.textContent = '';
      this.markAttempts.textContent = '';
    } else {
      this.markSentence.textContent = mark.sentence;
      // Die Zahl der Versuche gehoert daneben: Sie macht aus „19 %" ein
      // „19 % beim zwoelften Versuch" -- und genau das ist die Geschichte,
      // die der Spieler sich erzaehlt.
      this.markAttempts.textContent =
        mark.attempts === 1 ? '1 attempt' : `${mark.attempts} attempts`;
    }

    const summary = view.summary;
    if (summary === null) return;

    const beschriften = (key: string, text: string): void => {
      const node = this.resultLabels[key];
      if (node !== undefined) node.textContent = text;
    };

    if (summary.reversed) {
      // **BEI EINEM UMGEKEHRTEN ZIEL SAGEN DIESE ZWEI ZEILEN DAS GEGENTEIL.**
      // Gemessen an einem Lauf ohne einen einzigen Tastendruck:
      //
      //   Idle · worst spell   kindle · 17.0 s (100 %)
      //   Time lost            17.0 s (100 %)
      //
      // Beides ist korrekt GEMESSEN und trotzdem verkehrt: Nichtstun war dort
      // das Bestmögliche, und KINDLE ist der Zauber, der den Lauf 9,5 s früher
      // kippt. Die Auswertung riet dem Spieler zu dem, was ihn umbringt.
      //
      // Dieselben zwei Zeilen tragen deshalb die Größen, die dort zählen —
      // kein zweiter Satz Anzeigen.
      beschriften('idleWorst', 'Ring damage by you');
      set(
        'idleWorst',
        summary.playerDamage <= 0
          ? 'none — you held it'
          : `${summary.playerDamage.toLocaleString('en-US')} of ${summary.bossHp.toLocaleString('en-US')}`,
      );
      beschriften('lostTime', 'Time held');
      set('lostTime', `${(summary.elapsedMs / 1000).toFixed(1)} s`);
    } else {
      beschriften('idleWorst', 'Idle · worst spell');
      const worst = summary.idlePerSpell[0];
      set(
        'idleWorst',
        worst === undefined
          ? '—'
          : `${worst.spellId} · ${(worst.idleMs / 1000).toFixed(1)} s (${(worst.share * 100).toFixed(0)} %)`,
      );
      beschriften('lostTime', 'Time lost');
      set(
        'lostTime',
        `${(summary.lostTimeMs / 1000).toFixed(1)} s (${(summary.lostTimeShare * 100).toFixed(0)} %)`,
      );
    }
    set('taken', summary.damageTaken.toLocaleString('en-US'));
    // **Nur bei einem umgekehrten Ziel** (AC5). Bei einem gewöhnlichen Boss
    // gibt es nichts zurückzudrängen — die Zahl stünde dort bei null und
    // sagte nichts.
    const rueck = this.resultRows['pushedBack'];
    const rueckLabel = this.resultLabels['pushedBack'];
    if (rueck !== undefined && rueckLabel !== undefined) {
      const zeile = rueck.parentElement;
      if (zeile !== null) zeile.hidden = !summary.reversed;
      if (summary.reversed) set('pushedBack', summary.stabilised.toLocaleString('en-US'));
    }
    set(
      'synergy',
      summary.synergyAvailable === 0
        ? 'none available'
        : `${summary.synergyUsed} of ${summary.synergyAvailable}`,
    );
    this.fillDetail(summary, view.analysis);
  }

  /**
   * Die Zeile je Zauber, nach dem sortiert, was zählt (GAME-DESIGN 15.1).
   *
   * **BEI EINEM UMGEKEHRTEN ZIEL IST ES NICHT DER LEERLAUF** (Issue #256).
   * Diese Liste stand bis zum 2026-08-05 als „Idle time per spell" auch dort —
   * dieselbe verkehrte Aussage wie die Zeile darüber, nur eine Ebene tiefer und
   * hinter einem Knopf, den ich beim Umbau nicht gedrückt hatte.
   *
   * Gefunden hat es kein Prüffall, sondern ein Screenshot: `overlay.ts` braucht
   * ein Dokument, und die Prüffälle laufen ohne DOM. Dieselbe Stelle, an der
   * schon die eigene Fortschritts-Rechnung saß.
   *
   * Der Schaden je Zauber IST der schädliche Anteil: Stabilisierer drängen den
   * Ring über `undoProgress` zurück, statt Schaden zu buchen, und stehen hier
   * deshalb bei null.
   */
  private fillDetail(summary: RunSummary, analysis: LossReport | null): void {
    this.detail.replaceChildren();

    // **Die Rangliste zuerst** (GAME-DESIGN 15.3, Issue #280). Sie ist die
    // Auskunft, die den Spieler beim nächsten Versuch besser macht — „Versuch
    // 20 ist dann klüger als Versuch 5, nicht nur häufiger".
    //
    // Die Rohliste darunter bleibt: 15.1 nennt sie ausdrücklich, und sie
    // beantwortet eine andere Frage. Eine Rangliste sagt nicht, WELCHER
    // Zauber brachlag — „Idle time 9 %" ohne die Zeile darunter ist eine Zahl
    // ohne Adresse.
    if (analysis !== null) {
      this.detail.appendChild(buildLossView(analysis));
      this.detail.appendChild(element('div', 'rota-divider'));
    }
    const umgekehrt = summary.reversed;
    this.detail.appendChild(
      element(
        'p',
        'rota-hint',
        umgekehrt ? 'Ring damage per spell, worst first' : 'Idle time per spell, worst first',
      ),
    );
    // Die Kurzfassung sortiert nach Leerlauf. Bei einem umgekehrten Ziel ist
    // das die falsche Reihenfolge — hier steht der oben, der am meisten Ring
    // gekostet hat. Eine Kopie, damit die Kurzfassung unberührt bleibt.
    const zeilen = umgekehrt
      ? [...summary.idlePerSpell].sort(
          (a, b) => b.damage - a.damage || a.spellId.localeCompare(b.spellId),
        )
      : summary.idlePerSpell;
    for (const entry of zeilen) {
      const line = element('div', 'rota-row');
      line.appendChild(
        element('span', 'rota-row-label', `${entry.spellId} · ${entry.casts} casts`),
      );
      line.appendChild(
        element(
          'span',
          'rota-row-value',
          umgekehrt
            ? entry.damage === 0
              ? '—'
              : `${entry.damage.toLocaleString('en-US')} ring damage`
            : `${(entry.idleMs / 1000).toFixed(1)} s (${(entry.share * 100).toFixed(0)} %)`,
        ),
      );
      this.detail.appendChild(line);
    }
    // --- Vier Auskünfte über einzelne Mechaniken (Issue #294) -------------
    //
    // Sie stehen im aufgeklappten Bereich und nicht bei den Kennzahlen: Wer
    // keinen Unterbrecher mitführt, will die Zeile nicht nach jedem Lauf
    // sehen; wer einen mitführt, schlägt sie nach.
    //
    // **Nur, was sich bewegt hat.** Eine Zeile mit null bei jeder Mechanik,
    // die der Spieler gar nicht dabeihat, wäre Rauschen — und bei vier Zeilen
    // wäre es viermal Rauschen.
    const auskuenfte: readonly [string, number][] = [
      ['Interrupts landed', summary.interruptsLanded],
      ['Control wasted', summary.controlFailed],
      ['Shield absorbed', summary.shieldAbsorbed],
      ['Converted', summary.converted],
    ];
    for (const [label, wert] of auskuenfte) {
      if (wert <= 0) continue;
      const zeile = element('div', 'rota-row');
      zeile.appendChild(element('span', 'rota-row-label', label));
      zeile.appendChild(element('span', 'rota-row-value', wert.toLocaleString('en-US')));
      this.detail.appendChild(zeile);
    }

    const log = element('div', 'rota-row');
    log.appendChild(element('span', 'rota-row-label', 'Log entries'));
    log.appendChild(
      element(
        'span',
        'rota-row-value',
        `${summary.logEntries} / ${summary.logCapacity}${summary.logOverflowed ? ` · ${summary.logDropped} dropped` : ''}`,
      ),
    );
    this.detail.appendChild(log);
  }

  private toggleDetail(): void {
    this.detailOpen = !this.detailOpen;
    this.detail.classList.toggle('rota-visible', this.detailOpen);
    this.analysisButton.textContent = this.detailOpen ? 'Hide Analysis' : 'View Analysis';
  }

  /**
   * Die Werte eines Gegenstands anzeigen (Issue E6-1, AC4).
   *
   * **Additive und multiplikative Werte sind an zwei Merkmalen zu
   * unterscheiden:** an der Schreibweise (`+20%` gegen `x1.20`) und an der
   * Farbe. Zwei Merkmale, weil eine Farbe allein nicht trägt — etwa jeder
   * zwölfte Mann sieht Rot und Grün nicht auseinander.
   *
   * Wird **nur bei einer Änderung** angefasst, nie je Bild: Jede Änderung am
   * HTML löst eine Neuberechnung des Layouts aus (GAME-DESIGN 18.2).
   */
  showGear(title: string, lines: readonly StatLine[]): void {
    this.gearCard.replaceChildren();
    if (lines.length === 0) {
      this.gearCard.hidden = true;
      return;
    }
    this.gearCard.appendChild(element('h3', 'rota-gear-title', title));
    for (const line of lines) {
      const root = element('div', 'rota-row');
      root.appendChild(element('span', 'rota-row-label', line.label));
      root.appendChild(
        element('span', `rota-row-value ${classOfStat(line.kind)}`, formatStat(line)),
      );
      this.gearCard.appendChild(root);
    }
    this.gearCard.hidden = false;
  }

  /**
   * Einen Fund zeigen (Issue E6-4, AC3 und AC5).
   *
   * Die Inszenierung staffelt sich nach Qualität: Common beiläufig, ein
   * Legendary hält alles an. Der Vergleich trägt **immer** beide Merkmale der
   * Näherung — Zeichen und Wort.
   */
  showLoot(quality: number, itemName: string, comparison: Comparison, nowMs: number): void {
    const inszenierung = presentationFor(quality);
    this.loot.show(quality, nowMs);

    this.lootPanel.replaceChildren();
    this.lootPanel.className = `rota-loot ${inszenierung.className}`;
    this.lootPanel.style.setProperty('--rota-loot-dim', String(inszenierung.dim));
    this.lootPanel.style.setProperty('--rota-loot-ms', `${inszenierung.durationMs}ms`);

    this.lootPanel.appendChild(element('div', 'rota-loot-quality', labelFor(quality)));
    this.lootPanel.appendChild(element('div', 'rota-loot-name', itemName));

    const wert = element('div', 'rota-loot-delta', comparison.text);
    wert.classList.add(comparison.verdict > 0 ? 'rota-loot-better' : 'rota-loot-worse');
    this.lootPanel.appendChild(wert);

    // **Die Näherung in Worten.** Ohne sie liest sich die Zahl wie eine
    // Messung — und ein Spieler, der sie für exakt hält, trifft falsche
    // Entscheidungen (Auflage zu AC5).
    this.lootPanel.appendChild(element('div', 'rota-loot-caveat', comparison.caveat));
    this.lootPanel.appendChild(element('div', 'rota-loot-hint', 'Space to skip'));

    this.lootPanel.hidden = false;
  }

  /**
   * Die Inszenierung überspringen (AC4) — **auch die des Legendary**.
   *
   * Gibt zurück, ob überhaupt etwas lief.
   */
  skipLoot(): boolean {
    const lief = this.loot.skip();
    this.lootPanel.hidden = true;
    return lief;
  }

  /** Läuft der Beute-Moment noch? Sonst ausblenden. */
  stepLoot(nowMs: number): void {
    if (!this.loot.isRunning) return;
    this.loot.step(nowMs);
    if (!this.loot.isRunning) this.lootPanel.hidden = true;
  }

  private show(panel: HTMLElement): void {
    panel.classList.add('rota-visible');
    this.root.classList.add('rota-overlay-active');
  }

  private hideAll(): void {
    for (const panel of [this.prepare, this.pause, this.result]) {
      panel.classList.remove('rota-visible');
    }
    this.root.classList.remove('rota-overlay-active');
  }
}
