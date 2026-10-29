// @vitest-environment jsdom

/**
 * Kommen „Press Q", die Einführungen und die Instanz-Texte beim Spieler an?
 * (Issue #1177, Abschnitt 5 „Hilfe und Einführung" und „Archiv")
 *
 * # Der Weg, nicht die Funktion
 *
 * Bei #85 und #811 waren die Funktionen richtig und unerreichbar: `nextIntroduction`
 * hatte keinen Aufrufer, und der Auslöser `beforeInstance` stand bis #1177 in
 * keiner Zeile außerhalb des Parsers. Diese Datei fährt deshalb die echte `App`
 * und sieht nach, was im Dokument steht — und auf der Zeichenfläche, über
 * eine Aufzeichnung der `fillText`-Aufrufe.
 *
 * # Der allererste Start ist die Ausnahme
 *
 * Masterplan, Einstieg: „Sofort drücken, Erklärung nebenbei. Kein Menü, kein
 * Vorspann — nach dem Start steht man vor dem Werktisch, ein Symbol leuchtet,
 * eine Zeile sagt ‚Press Q'." Dort erscheint also KEIN Fenster. Die
 * Einführungen beginnen mit der zweiten Vorbereitung.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { App } from '../../src/core/app.js';
import { BOSSES, CALCINATION, INSTANCES } from '../../src/core/data.js';
import { PLAYABLE_INTRODUCTIONS } from '../../src/analysis/help-data.js';
import {
  bossMechanics,
  firstPressHint,
  FIRST_RUN_MECHANIC,
  loadoutMechanics,
} from '../../src/analysis/introduction-triggers.js';

/** Was die Zeichenfläche geschrieben hat, in der Reihenfolge der Aufrufe. */
let geschrieben: string[] = [];

function stubbeCanvas(): void {
  const ctx: unknown = new Proxy(
    {},
    {
      get: (_ziel, name) => {
        if (name === 'canvas') return undefined;
        if (name === 'fillText') {
          return (text: unknown) => {
            geschrieben.push(String(text));
            return ctx;
          };
        }
        return () => ctx;
      },
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

/** Ein echter Sieg über den laufenden Boss — derselbe Weg wie in archiv-anschluss. */
function siegFahren(app: App): void {
  app.session.startFight(0);
  app.session.tick(5_000);
  app.combat.setBossHpRemaining(0);
  app.session.tick(6_000);
}

function knopf(text: string): HTMLButtonElement | undefined {
  return [...document.querySelectorAll('button')].find((b) => b.textContent === text);
}

beforeEach(() => {
  stubbeCanvas();
  geschrieben = [];
  localStorage.clear();
  document.body.innerHTML = '';
});

describe('Der erste Start: „Press Q" und kein Fenster', () => {
  it('es erscheint keine Einführung und kein Archiv-Fenster', () => {
    baueApp();
    expect(document.querySelector('.intro')).toBeNull();
  });

  it('im ersten Kampf steht „Press Q" auf der Zeichenfläche', () => {
    const app = baueApp();
    app.session.startFight(0);
    app.session.tick(5_000);
    app.hud.render(5_000, 0);
    expect(geschrieben, 'die Zeile fehlt auf der Zeichenfläche').toContain('Press Q');
  });

  it('nach dem ersten gezählten Lauf nicht mehr', () => {
    const app = baueApp();
    siegFahren(app);
    expect(app.lastRecord?.counted, 'ZUSICHERUNG: der Lauf zählte').toBe(true);
    app.backToMenu();
    geschrieben = [];
    app.session.startFight(0);
    app.session.tick(5_000);
    app.hud.render(5_000, 0);
    expect(geschrieben).not.toContain('Press Q');
  });

  it('die Regel: nur vor dem ersten Lauf und nur bis zum ersten Druck', () => {
    expect(firstPressHint(false, 0, 'Q')).toBe('Press Q');
    // Die Beschriftung kommt aus dem Layout des Spielers, nicht aus dem Text.
    expect(firstPressHint(false, 0, 'A')).toBe('Press A');
    expect(firstPressHint(false, 1, 'Q')).toBeNull();
    expect(firstPressHint(true, 0, 'Q')).toBeNull();
  });
});

describe('Die Einführungen beginnen mit der zweiten Vorbereitung', () => {
  it('nach dem ersten Kampf erklärt „Back to Menu" die Reaktion', () => {
    const app = baueApp();
    siegFahren(app);
    document.querySelector('.archive-popup')?.remove();
    knopf('Back to Menu')?.click();
    const intro = document.querySelector<HTMLElement>('.intro[data-mechanic-id]');
    expect(intro?.dataset.mechanicId).toBe(FIRST_RUN_MECHANIC);
    expect(intro?.textContent).toContain('REACTION');
  });

  it('höchstens eine auf einmal — auch beim zweiten Aufruf', () => {
    const app = baueApp();
    siegFahren(app);
    knopf('Back to Menu')?.click();
    knopf('Back to Menu')?.click();
    expect(document.querySelectorAll('.intro[data-mechanic-id]').length).toBeLessThanOrEqual(1);
  });

  it('CONTINUE vermerkt sie, und die nächste kommt beim nächsten Besuch', () => {
    const app = baueApp();
    siegFahren(app);
    knopf('Back to Menu')?.click();
    document.querySelector<HTMLButtonElement>('.intro .intro-continue')?.click();
    expect(app.persistence.state.seenMechanicIds).toContain(FIRST_RUN_MECHANIC);
  });
});

describe('Vor jeder Instanz ein Text — bei der ersten Wahl ihres Bosses', () => {
  it('die erste Wahl eines Bosses aus Instanz 1 zeigt deren Einleitung', () => {
    const app = baueApp();
    siegFahren(app);
    document.querySelector('.archive-popup')?.remove();
    app.selectBoss('boss-1-1');
    const fenster = document.querySelector<HTMLElement>('.archive-popup');
    expect(fenster?.dataset.entryId).toBe('instance-1-before');
    expect(app.persistence.state.unlockedEntryIds).toContain('instance-1-before');
  });

  it('nach CONTINUE folgt die Einführung, nicht beides übereinander', () => {
    const app = baueApp();
    siegFahren(app);
    document.querySelector('.archive-popup')?.remove();
    app.selectBoss('boss-1-1');
    expect(document.querySelector('.intro[data-mechanic-id]'), 'beides auf einmal').toBeNull();
    document.querySelector<HTMLButtonElement>('.archive-popup .intro-continue')?.click();
    expect(document.querySelector('.archive-popup')).toBeNull();
    expect(document.querySelector('.intro[data-mechanic-id]')).not.toBeNull();
  });

  it('ein zweiter Boss derselben Instanz zeigt sie nicht noch einmal', () => {
    const app = baueApp();
    siegFahren(app);
    document.querySelector('.archive-popup')?.remove();
    app.selectBoss('boss-1-1');
    document.querySelector<HTMLButtonElement>('.archive-popup .intro-skip')?.click();
    app.selectBoss('boss-1-2');
    expect(document.querySelector('.archive-popup')).toBeNull();
  });

  it('das erste Unterbrechen wird bei boss-1-1 erklärt', () => {
    const app = baueApp();
    siegFahren(app);
    app.persistence.persistSeenMechanics([FIRST_RUN_MECHANIC]);
    app.persistence.persistUnlockedEntries([
      ...app.persistence.state.unlockedEntryIds,
      'instance-1-before',
    ]);
    document.querySelector('.archive-popup')?.remove();
    app.selectBoss('boss-1-1');
    const intro = document.querySelector<HTMLElement>('.intro[data-mechanic-id]');
    expect(intro?.dataset.mechanicId).toBe('interruptibleCast');
  });
});

describe('Jede Einführung ist erreichbar, und jede Mechanik hat eine', () => {
  const instanzen = new Set(INSTANCES.map((i) => i.id));
  const gebaut = [...BOSSES.values()].map((b) => b.value).filter((b) => instanzen.has(b.instance));
  const spielbar = CALCINATION.value.spells.filter((s) => s.unlockLevel <= 100);

  /** Alles, was die Auslöser für den ausgelieferten Inhalt liefern können. */
  const erreichbar = new Set<string>([
    FIRST_RUN_MECHANIC,
    ...gebaut.flatMap((b) => bossMechanics(b)),
    ...spielbar.flatMap((s) => loadoutMechanics([s])),
    ...loadoutMechanics(spielbar),
  ]);

  it('ZUSICHERUNG: die Auslöser liefern etwas', () => {
    expect(erreichbar.size).toBeGreaterThanOrEqual(4);
  });

  it('keine Einführung, die nie an die Reihe käme', () => {
    const tot = PLAYABLE_INTRODUCTIONS.map((i) => i.mechanicId).filter((id) => !erreichbar.has(id));
    expect(tot).toEqual([]);
  });

  it('keine Mechanik ohne Einführung', () => {
    const ids = new Set(PLAYABLE_INTRODUCTIONS.map((i) => i.mechanicId));
    expect([...erreichbar].filter((id) => !ids.has(id))).toEqual([]);
  });
});
