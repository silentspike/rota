// @vitest-environment jsdom

/**
 * Der Boss-Wechsel im **Dokument** gemessen, nicht im Quelltext (Issue #625).
 *
 * # Warum es diese Datei gibt — der Verweis war älter als die Prüfung
 *
 * `src/render/overlay.ts:296` behauptete über `setBoss`:
 *
 *     `tests/unit/overlay-boss-switch.test.ts` misst beides.
 *
 * Die Datei gab es nicht. Ein einziger `grep` fand genau einen Treffer für
 * den Namen: den Kommentar selbst. **Eine Behauptung über Deckung, die
 * niemand nachprüft** — wer sie liest, hält die Sache für gemessen und schaut
 * nicht weiter. Statt den Verweis zu löschen, ist hier die Prüfung, die er
 * meint.
 *
 * # Was der bestehende Wächter NICHT kann, und warum das keine Kritik ist
 *
 * `boss-switch-anzeige.test.ts` liest Quelltext und sagt das in seinem
 * eigenen Kopf:
 *
 *     "Wer `setBoss` ruft und darin das falsche Feld setzt, kommt daran
 *      vorbei."
 *
 * Er hält fest, dass **jede** Stelle, die den Kampf umstellt, die Anzeige
 * mitzieht — ein vergessener Aufruf fällt ihm auf. Was drinnen passiert,
 * sieht er nicht. Beide Fälle zusammen decken beides ab; keiner allein.
 *
 * # Die DOM-Umgebung gilt NUR für diese Datei
 *
 * Der Docblock in Zeile 1 schaltet jsdom für genau diese Datei ein. Eine
 * globale Umstellung in der Vitest-Konfiguration hätte alle 187 Dateien
 * zahlen lassen, damit eine Handvoll ein Dokument bekommt (Issue #625, AC4).
 */

import { beforeEach, describe, expect, it } from 'vitest';

import { Overlay, type OverlayActions } from '../../src/render/overlay';

/** Nimmt jeden Wunsch entgegen und tut nichts — hier wird die Anzeige gemessen. */
const STILL: OverlayActions = {
  onReady: () => {},
  onResume: () => {},
  onWipe: () => {},
  onRestart: () => {},
  onBackToMenu: () => {},
  onOpenHome: () => {},
};

let parent: HTMLElement;

beforeEach(() => {
  document.body.replaceChildren();
  parent = document.createElement('div');
  document.body.appendChild(parent);
});

/** Liest den Titel so, wie ein Spieler ihn sieht: aus dem Dokument. */
function titelImDokument(): string {
  const knoten = parent.querySelector('.rota-title');
  if (knoten === null) throw new Error('kein .rota-title im Dokument');
  return knoten.textContent ?? '';
}

/** Liest die Widerstands-Zeilen aus dem Dokument. */
function widerstandsZeilen(): string[] {
  return [...parent.querySelectorAll('.rota-resist-line')].map((n) => n.textContent ?? '');
}

function karteVersteckt(): boolean {
  const karte = parent.querySelector('.rota-resistances');
  if (karte === null) throw new Error('keine .rota-resistances im Dokument');
  return (karte as HTMLElement).hidden;
}

describe('setBoss schreibt in das Dokument, nicht nur in ein Feld', () => {
  it('es gibt überhaupt ein Dokument und einen Aufbau', () => {
    // **Die Zusicherung in den Datensatz.** Liefe diese Datei versehentlich
    // wieder unter `environment: 'node'`, gäbe es kein `document` — und ohne
    // diesen Fall bliebe unklar, ob die Prüfungen darunter etwas gemessen
    // haben oder nur nichts gefunden haben.
    expect(typeof document, 'kein document — läuft die Datei ohne jsdom?').toBe('object');

    new Overlay(parent, 'The First Crucible', STILL);
    expect(parent.querySelectorAll('*').length, 'das Overlay hat nichts aufgebaut').toBeGreaterThan(
      10,
    );
  });

  it('der Titel im Dokument folgt dem Wechsel', () => {
    // **Der Kern, und genau die Lücke aus dem Kopfkommentar von
    // boss-switch-anzeige:** Wer hier das falsche Feld setzt, kommt am
    // Quelltext-Wächter vorbei — an dieser Zeile nicht.
    const overlay = new Overlay(parent, 'The First Crucible', STILL);
    expect(titelImDokument()).toBe('THE FIRST CRUCIBLE');

    // Der Fehler aus #613: zwei Titel auf einem Bild, weil das Overlay den
    // Namen einmal im Konstruktor bekam.
    overlay.setBoss('Instance Two, First');
    expect(
      titelImDokument(),
      'der Titel im Dokument hängt am alten Boss — genau der Fehler aus #613',
    ).toBe('INSTANCE TWO, FIRST');
  });

  it('die Widerstände wechseln mit', () => {
    // GAME-DESIGN 11.4: „Vorher sichtbar in der Boss-Übersicht." Eine falsche
    // Auskunft ist hier schlimmer als keine — deshalb steht sie im Dokument.
    const overlay = new Overlay(parent, 'Erster', STILL, ['Fire 50%']);
    expect(widerstandsZeilen()).toEqual(['Fire 50%']);
    expect(karteVersteckt()).toBe(false);

    overlay.setBoss('Zweiter', ['Frost 30%', 'Shadow 10%']);
    expect(titelImDokument()).toBe('ZWEITER');
    expect(widerstandsZeilen(), 'die alten Widerstände stehen noch da').toEqual([
      'Frost 30%',
      'Shadow 10%',
    ]);
  });

  it('ein widerstandsloser Boss versteckt die Karte, statt „keine" zu behaupten', () => {
    const overlay = new Overlay(parent, 'Erster', STILL, ['Fire 50%']);
    expect(karteVersteckt()).toBe(false);

    overlay.setBoss('Ohne');
    expect(widerstandsZeilen(), 'Zeilen eines fremden Bosses stehen noch im Dokument').toEqual([]);
    expect(karteVersteckt(), 'die leere Karte bleibt sichtbar').toBe(true);
  });
});
