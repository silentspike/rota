/**
 * Prüfblatt für Issue #179, AC1 bis AC5.
 *
 * ES LÄUFT ÜBER `App`, NICHT ÜBER DIE EINZELTEILE.
 * Dass `recordRun` richtig rechnet, steht in tests/unit/run-recorder.test.ts.
 * Hier geht es um die VERDRAHTUNG: dass `session.onFinish` überhaupt gerufen
 * wird, dass der Aufrufer die Umrechnung nicht umgeht und dass der Stand im
 * Browser landet. Ein Blatt, das `recordRun` direkt ruft, hätte dieselbe Sache
 * zum zweiten Mal gemessen.
 *
 * DER LAUF WIRD VON HAND GETAKTET, NICHT ÜBER `app.start()`.
 * Der Bild-Takt liefe in Echtzeit — ein Kampf bis zum Überdruck dauerte eine
 * Minute je Durchlauf. Getaktet wird dieselbe `Session` mit denselben
 * Tastendrücken; `onFinish` hängt bereits am Konstruktor von `App` und löst
 * genauso aus.
 */
import { App } from '../../src/core/app.js';
import { COUNTDOWN_MS, PHASE_ENDED } from '../../src/core/phases.js';
import { markFor } from '../../src/analysis/progress-mark.js';
import { historySegments } from '../../src/analysis/run-history.js';
import { RunPersistence } from '../../src/analysis/run-persistence.js';
import { buildHistoryView } from '../../src/analysis/history-view.js';
import '../../src/analysis/history.css';

const KINDLE = 0;
const SMOULDER = 1;

function line(text: string, ok?: boolean): void {
  const el = document.createElement('div');
  el.className = ok === undefined ? 'note' : ok ? 'ok' : 'bad';
  el.textContent = text;
  document.querySelector('#out')?.appendChild(el);
}

/** Die Bühne für die Zeichenfläche — sie liegt außerhalb des Bildes. */
function buehne(): HTMLElement {
  const vorhanden = document.querySelector<HTMLElement>('#buehne');
  if (vorhanden === null) throw new Error('#buehne fehlt');
  return vorhanden;
}

/**
 * Einen echten Kampf durch `App` spielen.
 *
 * `everyMs` steuert, wie gut gespielt wird — der einzige Hebel, mit dem sich
 * ein besserer von einem schlechteren Versuch unterscheidet.
 */
function kampf(app: App, everyMs: number, loadout: readonly number[]): void {
  app.session.reset();
  loadout.forEach((spell, slot) => {
    app.session.bind(slot, spell);
  });
  app.session.beginCountdown(0);

  let tick = 0;
  let presses = 0;
  for (let t = 0; t <= COUNTDOWN_MS + 120_000; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      app.pressQueue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    app.session.tick(t);
    tick++;
    if (app.session.phase === PHASE_ENDED) break;
  }
}

// --------------------------------------------------------------------------
// Zweiter Durchgang: nach dem echten Neuladen
// --------------------------------------------------------------------------

if (location.hash === '#nachher') {
  line('AC4 — was das Neuladen überstanden hat');
  line('Die Seite wurde wirklich neu geladen; alles unten kommt aus dem Browser-Speicher.');

  // Ein FRISCHER Leser, der nichts weiss ausser dem, was abgelegt ist.
  const frisch = new RunPersistence(__ROTA_VERSION__, () => Date.now());
  const stand = frisch.state;

  line(
    `Verlaufs-Einträge nach dem Neuladen: ${stand.runHistory.length}`,
    stand.runHistory.length === 3,
  );
  const mark = markFor(stand.progressMarks, 'first-crucible', 1);
  line(
    `Marke: beste verbliebene Gesundheit ${mark?.bestRemainingPercent.toFixed(1)} %, ${mark?.attempts} Versuche`,
    mark !== null && mark.attempts === 3,
  );

  // AC1 zweite Haelfte: Die Marke ist die des BESTEN Versuchs, nicht die des
  // letzten. Der dritte Durchlauf war absichtlich schlechter.
  const bester = Math.min(...stand.runHistory.map((e) => e.remainingPercent));
  const letzter = stand.runHistory.at(-1)?.remainingPercent ?? -1;
  line(
    `Sie steht beim BESTEN Versuch (${bester.toFixed(1)} %), nicht beim letzten (${letzter.toFixed(1)} %)`,
    mark?.bestRemainingPercent === bester && bester < letzter,
  );

  // AC5: Fassung an JEDEM Eintrag.
  const ohneFassung = stand.runHistory.filter((e) => e.gameVersion === '');
  line(
    `Jeder Eintrag trägt die Fassung (${stand.runHistory[0]?.gameVersion ?? '—'})`,
    ohneFassung.length === 0 && (stand.runHistory[0]?.gameVersion ?? '') !== '',
  );

  // Und der Schnitt: ein untergeschobener Eintrag einer anderen Fassung teilt
  // den Verlauf. Er wird NICHT abgelegt -- geprueft wird die Ansicht.
  const mitWechsel = [
    ...stand.runHistory,
    { ...stand.runHistory[0]!, gameVersion: '9.9.9', at: Date.now() },
  ];
  line(
    `Ein Fassungswechsel erzeugt einen Schnitt: ${historySegments(mitWechsel).length} Abschnitte`,
    historySegments(mitWechsel).length === 2,
  );
  line(
    `Ohne Wechsel keiner: ${historySegments(stand.runHistory).length} Abschnitt`,
    historySegments(stand.runHistory).length === 1,
  );

  document.querySelector('#view')?.appendChild(buildHistoryView(stand.runHistory));
} else {
  // ------------------------------------------------------------------------
  // Erster Durchgang: spielen, ablegen, neu laden
  // ------------------------------------------------------------------------

  line('AC1 bis AC3 — drei echte Kämpfe durch App');

  // Sauber anfangen: Das Blatt ist ein Werkzeug, kein Spiel.
  for (let i = 0; i < 5; i++) localStorage.removeItem(`rota.save.${i}`);
  localStorage.removeItem('rota.save.index');

  const app = new App(buehne());
  line(
    `Spielstand beim Start: ${app.persistence.status.status}`,
    app.persistence.status.status === 'empty',
  );
  line(`Fassung im Programm: ${__ROTA_VERSION__}`, __ROTA_VERSION__.length > 0);

  // 1. schwach — die erste Marke, ohne dass „verbessert" behauptet wird
  kampf(app, 3000, [KINDLE, SMOULDER]);
  const erster = app.lastRecord;
  line(
    `1. Kampf (schwach): ${erster?.mark?.previousPercent === null ? 'erste Marke' : 'unerwartet'} ` +
      `bei ${erster?.runHistory.at(-1)?.remainingPercent.toFixed(1)} %`,
    erster?.counted === true && erster.mark?.improved === false,
  );

  // 2. stark — AC1: die Marke verbessert sich und der Fortschritt wird gemeldet
  kampf(app, 1500, [KINDLE, SMOULDER]);
  const zweiter = app.lastRecord;
  line(
    `2. Kampf (stark): verbessert um ${zweiter?.mark?.gainedPercent.toFixed(1)} Punkte`,
    zweiter?.mark?.improved === true && (zweiter.mark.gainedPercent ?? 0) > 0,
  );

  // 3. schwächer — AC2: die Marke bleibt stehen, der Versuch zählt trotzdem
  const markeVorher = markFor(app.persistence.state.progressMarks, 'first-crucible', 1);
  kampf(app, 4500, [KINDLE, SMOULDER]);
  const dritter = app.lastRecord;
  const markeNachher = markFor(app.persistence.state.progressMarks, 'first-crucible', 1);
  line(
    `3. Kampf (schwächer): Marke bleibt bei ${markeNachher?.bestRemainingPercent.toFixed(1)} %`,
    dritter?.mark?.improved === false &&
      markeNachher?.bestRemainingPercent === markeVorher?.bestRemainingPercent,
  );
  line(
    `…zählt aber als Versuch mit: ${markeNachher?.attempts} Versuche, ${app.persistence.state.runHistory.length} Einträge`,
    markeNachher?.attempts === 3 && app.persistence.state.runHistory.length === 3,
  );

  // Der Abbruch zählt nicht — entschieden am 2026-08-04 (GAME-DESIGN 12.8).
  const vorAbbruch = app.persistence.state.runHistory.length;
  // ERST ZURUECKSETZEN. Nach dem dritten Kampf steht die Sitzung in
  // PHASE_ENDED, und dort greift beginCountdown nicht -- der Abbruch faende
  // gar nicht statt, und lastRecord truege noch das Ergebnis des dritten
  // Kampfes. Der Prueffall haette dann "3 Einträge, vorher 3" gemeldet und
  // ausgesehen, als habe er den Abbruch geprueft.
  app.session.reset();
  app.session.bind(0, KINDLE);
  app.session.beginCountdown(0);
  for (let t = 0; t <= COUNTDOWN_MS + 5000; t += 10) app.session.tick(t);
  const abgebrochen = app.session.wipe(COUNTDOWN_MS + 5000);
  line(
    `Der Abbruch hat stattgefunden: ${abgebrochen}, Ausgang ${app.session.outcome}`,
    abgebrochen && app.session.phase === PHASE_ENDED,
  );
  line(
    `…und schreibt nichts: ${app.persistence.state.runHistory.length} Einträge (vorher ${vorAbbruch})`,
    app.persistence.state.runHistory.length === vorAbbruch && app.lastRecord?.counted === false,
  );

  line('');
  line('Lade die Seite jetzt WIRKLICH neu — AC4 wird danach gemessen.');
  location.hash = '#nachher';
  // Ein zweiter Aufruf im selben Seitenzustand wuerde nur zeigen, dass etwas im
  // Speicher ueberlebt. Erst das Neuladen zeigt, dass es gesichert ist.
  setTimeout(() => {
    location.reload();
  }, 300);
}
