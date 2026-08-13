/**
 * Kommt die Hilfe beim Spieler an? (Issue #811)
 *
 * # Warum eine eigene Datei neben `help.test.ts`
 *
 * Jene prüft die Mechanik: `helpFor` findet, `markSeen` vermerkt,
 * `nextIntroduction` liefert höchstens eine. Alles davon war grün, während
 * **niemand** `attachHelp` rief und **kein einziger Knoten** `data-help` trug.
 * Die Funktionen waren richtig und unerreichbar.
 *
 * Diese Datei fragt das andere: Setzt die Ansicht das Merkmal, an dem das
 * Fenster hängt? Trägt der Fachbegriff seinen Klartext? Kennt der Spielstand
 * die gesehene Mechanik?
 *
 * # Was sie NICHT leisten kann, und was deshalb im Browser gemessen wurde
 *
 * Ob das Fenster **sichtbar** ist. Bei #811 lag es unter jsdom vollständig
 * korrekt im Baum — mit Text, Grösse und `z-index` — und im Browser bei
 * y = 900 in einem 900 px hohen Fenster, also exakt ausserhalb des Bildes.
 * Ursache war eine fehlende Zeile CSS. jsdom rechnet keine Layouts; was hier
 * grün ist, kann dort unsichtbar sein.
 *
 * @vitest-environment jsdom
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { attachHelp, buildIntroduction } from '../../src/analysis/archive-view.js';
import { ALL_HELP_ENTRIES, ALL_INTRODUCTIONS } from '../../src/analysis/help-data.js';
import { helpFor, markSeen, nextIntroduction } from '../../src/analysis/help.js';
import { buildStatOverview } from '../../src/progression/stat-overview-view.js';
import { statOverview } from '../../src/progression/stat-overview.js';
import { baseStats } from '../../src/progression/base-stats.js';
import { buildComposition } from '../../src/progression/composition-view.js';
import {
  buildBossSelect,
  faehigkeitenVon,
  mechanikenVon,
} from '../../src/progression/boss-select.js';
import { CALCINATION, bossById } from '../../src/core/data.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';

const ERSTER = bossById('boss-1-1');

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

describe('AC3 — der Fachbegriff trägt seinen Klartext', () => {
  it('die Werte-Übersicht schreibt „ESSENCE (Strength of the substance)"', () => {
    const wurzel = buildStatOverview(statOverview(baseStats(1), NOTHING_WORN));
    const zeile = wurzel.querySelector('.stats-label');
    // Beide Hälften einzeln: Der Fachbegriff MUSS stehen bleiben — der
    // Klartext tritt daneben, nicht an seine Stelle (17.3).
    expect(zeile?.textContent).toContain('ESSENCE');
    expect(zeile?.textContent).toContain('Strength of the substance');
  });

  it('die Zusammenstellung setzt den Klartext an den Schulnamen', () => {
    const wurzel = buildComposition({
      bindings: {},
      level: 40,
      spells: CALCINATION.value.spells,
      onBind: () => true,
    });
    const titel = [...wurzel.querySelectorAll('.composition-group-title')].map(
      (n) => n.textContent ?? '',
    );
    // **Beide Hälften, und der Klartext aus den Daten**: `withPlainText` setzt
    // ihn nur, wenn es ihn kennt — eine erfundene Erklärung wäre schlimmer als
    // keine, und ein Tippfehler in `SCHOOL_PLAIN` bliebe sonst unsichtbar.
    expect(titel, `keine Schule mit Klartext: ${titel.join(' | ')}`).toContain(
      'CALCINATION (Damage over time)',
    );
  });
});

describe('AC2 — die Ansicht setzt das Merkmal, an dem das Fenster hängt', () => {
  it('Werte tragen `data-help` mit ihrer Kennung', () => {
    const wurzel = buildStatOverview(statOverview(baseStats(1), NOTHING_WORN));
    const ziele = [...wurzel.querySelectorAll<HTMLElement>('[data-help]')];
    expect(ziele.length, 'kein einziger Wert ist ausgezeichnet').toBeGreaterThan(0);
    // **Die Kennung muss treffen**, sonst erscheint kein Fenster: Der Wert
    // heisst im Datensatz `essence`, und genau das muss am Knoten stehen.
    expect(ziele.map((n) => n.dataset.help)).toContain('essence');
  });

  it('Boss-Fähigkeiten tragen `data-help` — die Gruppe, die man vergisst', () => {
    const wurzel = buildBossSelect([ERSTER as never], 'anderer', () => undefined);
    const namen = [...wurzel.querySelectorAll<HTMLElement>('.boss-select-ability')];
    expect(namen.length, 'keine Fähigkeit steht da').toBeGreaterThan(0);
    expect(namen.map((n) => n.dataset.help)).toContain('Kindling Roar');
  });

  it('und das Fenster erscheint mit der Erklärung darin', () => {
    const wurzel = document.createElement('div');
    const ziel = document.createElement('span');
    ziel.dataset.help = 'essence';
    wurzel.appendChild(ziel);
    attachHelp(wurzel, ALL_HELP_ENTRIES);

    ziel.dispatchEvent(new MouseEvent('mouseenter'));
    const fenster = ziel.querySelector('.help-popover');
    expect(fenster, 'kein Fenster nach mouseenter').not.toBeNull();
    expect(fenster?.querySelector('.help-effect')?.textContent).toContain('damage');

    ziel.dispatchEvent(new MouseEvent('mouseleave'));
    expect(ziel.querySelector('.help-popover'), 'das Fenster bleibt stehen').toBeNull();
  });

  it('ein zweiter Aufruf hängt keine zweiten Zuhörer an', () => {
    // **Das Player Home ersetzt einzelne Bereiche und lässt andere stehen.**
    // Ohne die Sperre bekäme jeder stehen gebliebene Knoten bei jedem
    // Auffrischen ein weiteres Paar Zuhörer — und niemand sähe es, weil das
    // Ergebnis gleich AUSSIEHT.
    const wurzel = document.createElement('div');
    const ziel = document.createElement('span');
    ziel.dataset.help = 'essence';
    wurzel.appendChild(ziel);

    attachHelp(wurzel, ALL_HELP_ENTRIES);
    expect(ziel.dataset.helpAttached).toBe('yes');
    attachHelp(wurzel, ALL_HELP_ENTRIES);

    ziel.dispatchEvent(new MouseEvent('mouseenter'));
    expect(ziel.querySelectorAll('.help-popover'), 'zwei Fenster übereinander').toHaveLength(1);
  });

  it('ohne passenden Eintrag erscheint KEIN Fenster', () => {
    // Kein Fenster ist besser als ein leeres — und ein leeres wäre der Beweis,
    // dass die Kupplung nicht trägt.
    const wurzel = document.createElement('div');
    const ziel = document.createElement('span');
    ziel.dataset.help = 'gibt-es-nicht';
    wurzel.appendChild(ziel);
    attachHelp(wurzel, ALL_HELP_ENTRIES);
    ziel.dispatchEvent(new MouseEvent('mouseenter'));
    expect(ziel.querySelector('.help-popover')).toBeNull();
  });
});

describe('Der Anschluss selbst — nicht die Teile, sondern die Stelle, die sie verbindet', () => {
  /**
   * Ob ein Knoten wirklich einen Zuhörer hat, prüft nur der Zuhörer selbst.
   *
   * `dataset.helpAttached` wäre die bequeme Antwort und die falsche: Es ist
   * eine Markierung, die `attachHelp` setzt — sie belegt, dass die Funktion
   * lief, nicht dass ein Fenster erscheint. Hier wird das Ereignis ausgelöst
   * und nachgesehen, was ankommt.
   */
  function zeigtFenster(ziel: HTMLElement): boolean {
    ziel.dispatchEvent(new MouseEvent('mouseenter'));
    const da = ziel.querySelector('.help-popover') !== null;
    ziel.dispatchEvent(new MouseEvent('mouseleave'));
    return da;
  }

  it('die App hängt die Fenster ans Player Home — beim Aufbau', () => {
    // **Dieser Fall fehlte, und der Orchestrator hat es gemessen:**
    // `attachHelpToHome` mit `return;` als erster Zeile liess alle 2556 Fälle
    // grün. Die Methode ist der EINZIGE Anschluss der Erklärfenster; wird sie
    // leer, hat kein Fenster mehr einen Zuhörer, und nichts merkt es.
    //
    // Die zehn Fälle darüber prüfen `attachHelp` an einem Baum, den sie selbst
    // bauen — sie überspringen genau die Stelle, an der die App entscheidet,
    // ob es überhaupt passiert. Dieselbe Klasse wie `comparisonForResult`
    // (#82) und `setSetEffects` (#794), diesmal in meiner eigenen Arbeit.
    const app = baueApp();
    app.openHome();

    const ziele = [...document.querySelectorAll<HTMLElement>('[data-help]')];
    expect(ziele.length, 'kein einziges Ziel im Player Home').toBeGreaterThan(0);

    const mitFenster = ziele.filter((z) => zeigtFenster(z));
    expect(
      mitFenster.length,
      `${ziele.length} Ziele tragen data-help, aber keines öffnet ein Fenster — ` +
        `der Anschluss in App.attachHelpToHome fehlt oder tut nichts`,
    ).toBeGreaterThan(0);
  });

  // **Was dieser Fall NICHT leistet, gemessen statt vermutet:** Kappt man nur
  // den Aufruf in `refreshHome` und lässt den beim Aufbau stehen, bleibt er
  // GRÜN. `refreshHome` ersetzt nicht alle Bereiche — die Zusammenstellung
  // etwa behält ihre Zuhörer, und „mindestens eines" ist dann erfüllt, während
  // Werte und Boss-Wahl tot sind.
  //
  // Genau dafür steht der Fall darunter. Zwei Fälle, weil es zwei Wege gibt,
  // und einer, der beide zugleich prüfte, prüfte in Wahrheit nur den
  // bequemeren.

  it('und nach jedem Auffrischen wieder — sonst sind sie nach dem ersten weg', () => {
    // `refreshHome` ersetzt ganze Bereiche; der alte Teilbaum geht mitsamt
    // seinen Zuhörern verloren. Ohne den zweiten Aufruf hätten die Werte nach
    // dem ersten Aufstieg kein Fenster mehr und die Boss-Wahl nach dem ersten
    // Wechsel keins — im Browser sichtbar, in einem Fall über `attachHelp`
    // allein unsichtbar.
    const app = baueApp();
    app.openHome();
    // Ein zweites Öffnen läuft durch `refreshHome` und tauscht die Bereiche.
    app.openHome();

    const ziel = document.querySelector<HTMLElement>('.stats-label[data-help]');
    expect(ziel, 'kein Wert mit data-help nach dem Auffrischen').not.toBeNull();
    expect(
      zeigtFenster(ziel as HTMLElement),
      'nach dem Auffrischen öffnet der Wert kein Fenster mehr',
    ).toBe(true);
  });

  it('die Fenster tragen die Erklärung aus den Daten, nicht irgendeine', () => {
    // Ohne diese Zeile bliebe offen, ob die App dieselbe Liste einhängt, die
    // `help-data.ts` prüft — ein leeres Array wäre ebenso „angeschlossen".
    const app = baueApp();
    app.openHome();
    const ziel = document.querySelector<HTMLElement>('.stats-label[data-help]') as HTMLElement;
    ziel.dispatchEvent(new MouseEvent('mouseenter'));
    expect(ziel.querySelector('.help-effect')?.textContent).toBe(
      helpFor(ALL_HELP_ENTRIES, ziel.dataset.help ?? '')?.effect,
    );
  });
});

describe('AC4 — die geführte Einführung', () => {
  it('ein Boss nennt seine Mechaniken, jede einmal', () => {
    const mechaniken = mechanikenVon(ERSTER as never);
    expect(mechaniken).toContain('interruptibleCast');
    // Ein Boss, der dreimal zuschlägt, kann eine Sache, nicht drei.
    expect(new Set(mechaniken).size).toBe(mechaniken.length);
  });

  it('Fähigkeiten und Mechaniken sind zwei Ebenen, nicht dieselbe', () => {
    // Die Einführung erklärt die MECHANIK, das Fenster die FÄHIGKEIT. Fielen
    // beide zusammen, bekäme der Spieler dieselbe Sache unter sechzig Namen.
    expect(faehigkeitenVon(ERSTER as never)).toContain('Kindling Roar');
    expect(mechanikenVon(ERSTER as never)).not.toContain('Kindling Roar');
  });

  it('sie kommt einmal, und Überspringen zählt als gesehen', () => {
    let stand = { seenMechanicIds: [] as readonly string[] };
    const faellig = nextIntroduction(ALL_INTRODUCTIONS, stand, 'interruptibleCast');
    expect(faellig, 'keine Einführung zu interruptibleCast').not.toBeNull();

    const fenster = buildIntroduction(faellig as never, (id) => {
      stand = markSeen(stand, id);
    });
    const skip = [...fenster.querySelectorAll('button')].find((b) => b.textContent === 'SKIP');
    expect(skip, 'kein Knopf zum Überspringen — AC4 verlangt ihn').toBeDefined();
    skip?.click();

    expect(stand.seenMechanicIds).toContain('interruptibleCast');
    // Und danach kommt sie nicht wieder — das ist der Sinn des Knopfes.
    expect(nextIntroduction(ALL_INTRODUCTIONS, stand, 'interruptibleCast')).toBeNull();
  });
});
