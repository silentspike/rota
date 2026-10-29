// @vitest-environment jsdom

/**
 * Hat jeder Begriff, den das Spiel zeigt, seine Erklärung? (Issue #1177,
 * Abschnitt 5 „Hilfe und Einführung")
 *
 * # Die Liste kommt aus dem Code, nicht von Hand
 *
 * Auftrag: „jeder Begriff, den das Spiel zeigt, hat einen Hilfe-Eintrag —
 * Liste aus dem Code abgeleitet". Eine Liste in diesem Fall wäre beim nächsten
 * Zauber falsch und bliebe grün, weil sie ihn nicht kennt. Deshalb zwei
 * Quellen, die das Spiel selbst liefert:
 *
 * ```text
 *   HTML-Ansichten   die echten Bauer (Zusammenstellung, Werte, Boss-Wahl)
 *                    mit dem GANZEN ausgelieferten Inhalt; gesammelt wird
 *                    jedes `data-help`, das sie setzen
 *   Zeichenfläche    jede exportierte `LABEL_*`-Konstante aus hud.ts — der
 *                    Kampf ist Canvas und trägt kein Merkmal, also erklärt
 *                    ihn die geführte Einführung, und dort muss der Begriff
 *                    stehen
 * ```
 *
 * Gegenprobe zu jeder Sammlung: Sie ist nicht leer und trifft die Grundmenge
 * aus den Daten. Eine Schleife über nichts ist immer grün.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import * as hud from '../../src/render/hud.js';
import { PLAYABLE_HELP_ENTRIES, PLAYABLE_INTRODUCTIONS } from '../../src/analysis/help-data.js';
import { helpFor } from '../../src/analysis/help.js';
import { buildComposition } from '../../src/progression/composition-view.js';
import { buildStatOverview } from '../../src/progression/stat-overview-view.js';
import { STAT_ORDER, statOverview } from '../../src/progression/stat-overview.js';
import { baseStats } from '../../src/progression/base-stats.js';
import { buildBossSelect, faehigkeitenVon } from '../../src/progression/boss-select.js';
import { BOSSES, CALCINATION, INSTANCES } from '../../src/core/data.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';
import { App } from '../../src/core/app.js';

const SPIELBAR = CALCINATION.value.spells.filter((s) => s.unlockLevel <= 100);
const INSTANZEN = new Set(INSTANCES.map((i) => i.id));
const GEBAUT = [...BOSSES.values()].map((b) => b.value).filter((b) => INSTANZEN.has(b.instance));

/** Jede Kennung, die eine Ansicht als `data-help` setzt. */
function hilfeMerkmale(wurzel: ParentNode): string[] {
  return [...wurzel.querySelectorAll<HTMLElement>('[data-help]')].map((n) => n.dataset.help ?? '');
}

/** Alle drei HTML-Ansichten mit dem ganzen Inhalt. */
function alleMerkmale(): string[] {
  // Jeder spielbare Zauber auf einem eigenen Platz: Dann steht er links in
  // der Liste UND rechts in der Vorschau seiner Schule.
  const bindings: Record<string, string> = {};
  SPIELBAR.slice(0, 12).forEach((spell, platz) => {
    bindings[String(platz)] = spell.id;
  });
  const zusammenstellung = buildComposition({
    bindings,
    level: 100,
    spells: SPIELBAR,
    onBind: () => true,
  });
  const werte = buildStatOverview(statOverview(baseStats(1), NOTHING_WORN));
  const wahl = buildBossSelect(GEBAUT, '', () => undefined);
  return [...hilfeMerkmale(zusammenstellung), ...hilfeMerkmale(werte), ...hilfeMerkmale(wahl)];
}

describe('Jedes data-help der Ansichten hat seinen Eintrag', () => {
  it('ZUSICHERUNG: die Sammlung trifft die Grundmenge aus den Daten', () => {
    const merkmale = new Set(alleMerkmale());
    for (const spell of SPIELBAR) expect(merkmale.has(spell.id), spell.id).toBe(true);
    for (const wert of STAT_ORDER) expect(merkmale.has(wert), wert).toBe(true);
    for (const boss of GEBAUT) {
      for (const name of faehigkeitenVon(boss)) expect(merkmale.has(name), name).toBe(true);
    }
  });

  it('jede gesammelte Kennung findet ihre Erklärung im ausgelieferten Katalog', () => {
    const fehlt = [...new Set(alleMerkmale())].filter(
      (id) => helpFor(PLAYABLE_HELP_ENTRIES, id) === null,
    );
    expect(fehlt).toEqual([]);
  });

  it('jede Erklärung trägt eine Wirkung — kein leeres Fenster', () => {
    for (const entry of PLAYABLE_HELP_ENTRIES) {
      expect(entry.effect.length, entry.id).toBeGreaterThan(0);
    }
  });

  it('die Werte eines Zaubers kommen aus seinen Daten', () => {
    // Keine Zahl im Text der Datei — sie stünde nach der nächsten Messung
    // falsch da. Die Abklingzeit muss trotzdem im Fenster stehen.
    for (const spell of SPIELBAR) {
      const entry = helpFor(PLAYABLE_HELP_ENTRIES, spell.id);
      const sekunden = `${String(spell.cooldownMs / 1000)} s`;
      expect(entry?.values ?? '', `${spell.id}: Abklingzeit fehlt`).toContain(sekunden);
    }
  });
});

describe('Jeder Begriff der Zeichenfläche steht in einer Einführung', () => {
  /** Die Beschriftungen des HUD, so wie hud.ts sie exportiert. */
  const begriffe = Object.entries(hud)
    .filter(([name, wert]) => name.startsWith('LABEL_') && typeof wert === 'string')
    .map(([, wert]) => wert as string);

  it('ZUSICHERUNG: hud.ts exportiert seine Beschriftungen', () => {
    // Fehlt der Export, wäre die Liste leer und jeder Fall darunter grün.
    expect(begriffe).toContain('HEAT');
    expect(begriffe).toContain('OVERPRESSURE IN');
  });

  it('jeder Begriff steht im Titel oder Text einer ausgelieferten Einführung', () => {
    const texte = PLAYABLE_INTRODUCTIONS.map((i) => `${i.title} ${i.body}`.toLowerCase()).join(' ');
    const fehlt = begriffe.filter((b) => !texte.includes(b.toLowerCase()));
    expect(fehlt).toEqual([]);
  });
});

describe('Die Einführungen', () => {
  it('mindestens vier, darunter die vier aus dem Plan', () => {
    // Masterplan, Hilfe-System: „erster Zauber, erste Abklingzeit, erste
    // Synergie, erstes Unterbrechen". Der erste Zauber ist „Press Q" auf der
    // Zeichenfläche (Einstieg), die drei anderen sind Einführungen.
    const ids = PLAYABLE_INTRODUCTIONS.map((i) => i.mechanicId);
    expect(ids.length).toBeGreaterThanOrEqual(4);
    for (const id of ['cooldown', 'synergy', 'interruptibleCast']) expect(ids, id).toContain(id);
  });

  it('keine Rotations-Empfehlung, auch nicht im Katalog (GAME-DESIGN 21.2)', () => {
    const texte = [
      ...PLAYABLE_HELP_ENTRIES.map(
        (e) => `${e.title} ${e.effect} ${e.values ?? ''} ${e.synergies ?? ''}`,
      ),
      ...PLAYABLE_INTRODUCTIONS.map((i) => `${i.title} ${i.body}`),
    ]
      .join(' ')
      .toLowerCase();
    for (const wort of [
      'should',
      'optimal',
      'better',
      'instead',
      'rotation',
      'before',
      'after you',
      'always cast',
    ]) {
      expect(texte, `"${wort}" darf nicht vorkommen`).not.toContain(wort);
    }
  });

  it('Spieltexte englisch', () => {
    const deutsch = [...PLAYABLE_HELP_ENTRIES, ...PLAYABLE_INTRODUCTIONS].filter((e) =>
      /[äöüßÄÖÜ„“]/.test(JSON.stringify(e)),
    );
    expect(deutsch).toEqual([]);
  });
});

describe('Das Player Home zeigt den ausgelieferten Katalog', () => {
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

  beforeEach(() => {
    stubbeCanvas();
    localStorage.clear();
    document.body.innerHTML = '';
  });

  it('das Fenster über Kindle trägt Wirkung und Werte aus dem Katalog', () => {
    const parent = document.createElement('div');
    document.body.appendChild(parent);
    const app = new App(parent);
    app.openHome();
    const ziel = document.querySelector<HTMLElement>('[data-help="kindle"]');
    expect(ziel, 'kein Merkmal an Kindle').not.toBeNull();
    ziel?.dispatchEvent(new Event('mouseenter'));
    const fenster = ziel?.querySelector('.help-popover');
    const katalog = helpFor(PLAYABLE_HELP_ENTRIES, 'kindle');
    expect(katalog, 'kein Eintrag im Katalog').not.toBeNull();
    expect(fenster?.textContent).toContain(katalog?.effect ?? '<kein Katalog>');
    expect(fenster?.textContent).toContain('Cooldown');
  });
});
