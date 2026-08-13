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
import { describe, expect, it } from 'vitest';
import { attachHelp, buildIntroduction } from '../../src/analysis/archive-view.js';
import { ALL_HELP_ENTRIES, ALL_INTRODUCTIONS } from '../../src/analysis/help-data.js';
import { markSeen, nextIntroduction } from '../../src/analysis/help.js';
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
