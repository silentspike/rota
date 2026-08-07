/**
 * Wer den Boss wechselt, zieht die Anzeige mit (Issue #613).
 *
 * # Der Fehler, den das verhindern soll — gemessen, nicht ausgedacht
 *
 * Der Orchestrator hat den Auslieferungs-Bau gespielt und dabei zwei Titel auf
 * einem Bild gefunden, die sich widersprachen:
 *
 *     HUD über dem Ring     INSTANCE TWO, FIRST     richtig
 *     Overlay-Überschrift   THE FIRST CRUCIBLE      falsch
 *
 * Das HUD liest `session.combat.boss.name` bei jedem Bild neu; das Overlay
 * bekam den Namen **einmal im Konstruktor**. Solange der Boss feststand, war
 * das richtig.
 *
 * **Es ist derselbe Fehlertyp wie #229, eine Stufe weiter.** Dort stand der
 * Name zweimal fest im Quelltext, hier steht er einmal fest im Zustand. Der
 * Kopfkommentar von `boss-name-source.test.ts` beschreibt die Gefahr bereits
 * genau: „Wer den Boss wechselt und den vertrauten Namen sieht, hält den
 * Wechsel für gescheitert — oder, schlimmer, hält einen echten Fehlschlag für
 * gelungen, weil alles aussieht wie immer."
 *
 * # WAS DIESE PRÜFUNG NICHT KANN
 *
 * Sie liest **Quelltext, nicht Verhalten**. Das Projekt fährt Vitest mit
 * `environment: 'node'` und hat kein jsdom; einen echten Aufbau des Overlay
 * gibt es hier also nicht zu messen. Wer `setBoss` ruft und darin das falsche
 * Feld setzt, kommt daran vorbei.
 *
 * Was sie kann, ist der Rückfall: Sie hält fest, dass **jede** Stelle, die den
 * Kampf auf einen anderen Boss stellt, auch die Anzeige mitzieht. Genau das
 * war der Fehler — nicht ein falscher Name, sondern ein vergessener Aufruf.
 *
 * Der Nachweis über das laufende Spiel steht im PR (playwright, ausgelieferter
 * Bau). Beides zusammen, keins allein.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const APP = readFileSync(new URL('../../src/core/app.ts', import.meta.url), 'utf8');
const OVERLAY = readFileSync(new URL('../../src/render/overlay.ts', import.meta.url), 'utf8');

describe('der Wechsel zieht die Anzeige mit', () => {
  it('es gibt überhaupt etwas zu lesen', () => {
    // Läse eine Datei leer — verschobener Pfad, umbenannte Datei —, wären alle
    // Fälle darunter trivial erfüllt.
    expect(APP.length, 'app.ts ist leer').toBeGreaterThan(1000);
    expect(OVERLAY.length, 'overlay.ts ist leer').toBeGreaterThan(1000);
  });

  it('`switchBoss` und `overlay.setBoss` stehen zusammen', () => {
    // Der Kern. Beide Aufrufe müssen vorkommen — und zwar so, dass keiner
    // ohne den anderen dasteht.
    expect(APP).toContain('this.combat.switchBoss(');
    expect(APP).toContain('this.overlay.setBoss(');
  });

  it('jeder switchBoss-Aufruf hat einen setBoss-Aufruf', () => {
    // **Die eigentliche Zusicherung**, und sie wächst mit: Käme ein zweiter
    // Wechsel-Weg dazu — etwa aus einem Menü oder einem Tastendruck — und
    // zöge die Anzeige nicht mit, fiele dieser Fall.
    const wechsel = APP.match(/this\.combat\.switchBoss\(/g)?.length ?? 0;
    const anzeige = APP.match(/this\.overlay\.setBoss\(/g)?.length ?? 0;
    expect(wechsel, 'kein switchBoss-Aufruf gefunden — der Fall misst nichts').toBeGreaterThan(0);
    expect(anzeige).toBe(wechsel);
  });

  it('das Overlay setzt seinen Titel NICHT mehr nur im Konstruktor', () => {
    // Die Ursache selbst: Stünde `bossName.toUpperCase()` weiterhin direkt im
    // Aufbau des Bildschirms, wäre der Titel wieder einmalig gesetzt — und
    // `setBoss` änderte einen Knoten, den niemand ansieht.
    expect(OVERLAY).toContain('setBoss(');
    expect(OVERLAY).not.toContain("element('h1', 'rota-title', bossName");
  });

  it('und die Widerstände gehen denselben Weg wie der Name', () => {
    // Sie hängen genauso am Boss. Ein Spieler, der von einem widerstandslosen
    // auf einen widerstehenden wechselt, läse sonst weiter „keine" — und 11.4
    // nennt genau diesen Fall eine böse Überraschung.
    const setBossBlock = OVERLAY.slice(OVERLAY.indexOf('setBoss('));
    expect(setBossBlock.slice(0, 800)).toContain('rota-resist-line');
  });
});
