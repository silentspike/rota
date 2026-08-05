/**
 * `runReference` nimmt einen Boss — und lehnt ab, was es nicht kennt
 * (Issue #338).
 *
 * # Der Fall, aus dem das Issue kommt
 *
 * ```
 * simulate('good', { boss: 'test-resistant' })   → 4240
 * simulate('good', { boss: 'test-vulnerable' })  → 4240
 * simulate('good', { boss: 'first-crucible' })   → 4240
 * ```
 *
 * Das sah aus wie „der Widerstand wirkt nicht" und wäre um ein Haar als
 * Befund an fremder Arbeit gemeldet worden. Tatsächlich gab es den Parameter
 * nicht: Der Kampf war fest auf `first-crucible` gebaut, das Feld kam an und
 * wurde nie gelesen.
 *
 * **Warum der Typ nicht half:** Der Aufruf lief im Browser über
 * `window.__rota`. Dort sind die Typen weg — die Dev-Werkzeuge sind eine
 * Laufzeit-Schnittstelle. Deshalb prüft `runReference` jetzt selbst.
 */
import { describe, expect, it } from 'vitest';
import { runReference } from '../../src/devtools/reference-run.js';

describe('Der Boss kommt an (AC3)', () => {
  it('rechnet ohne Angabe gegen first-crucible', () => {
    const r = runReference('good');
    expect(r.bossId).toBe('first-crucible');
  });

  it('nennt den Boss im ERGEBNIS, nicht nur in der Anfrage', () => {
    // **Das ist die Reparatur der Verwechslungsgefahr.** Die Referenz
    // `4240 @ 40510` gilt für first-crucible; steht der Boss neben der Zahl,
    // ist eine andere Zahl keine Überraschung mehr, sondern eine Angabe.
    const r = runReference('good', { bossId: 'test-metronome' });
    expect(r.bossId).toBe('test-metronome');
  });

  it('rechnet gegen verschiedene Boesse VERSCHIEDEN', () => {
    // Der eigentliche Nachweis: Ohne ihn wäre „der Boss kommt an" die
    // Behauptung, die das Issue schon einmal geglaubt hat.
    const a = runReference('good', { bossId: 'first-crucible' });
    const b = runReference('good', { bossId: 'test-metronome' });
    expect(a.damage).not.toBe(b.damage);
  });

  it('lehnt einen Boss ab, den es nicht gibt', () => {
    expect(() => runReference('good', { bossId: 'gibtesnicht' })).toThrow(/gibtesnicht/);
  });
});

describe('Unbekannte Felder fallen auf (AC1)', () => {
  it('wirft — und nennt den NAMEN des Feldes', () => {
    // „Unbekannte Option" ließe den Aufrufer suchen. Der Name ist der
    // Unterschied zwischen „wo?" und „da".
    expect(() => runReference('good', { boss: 'test-metronome' } as never)).toThrow(/"boss"/);
  });

  it('nennt ALLE unbekannten Felder, nicht nur das erste', () => {
    // Wer zwei Tippfehler hat, soll sie in einem Durchgang sehen.
    let gemeldet = '';
    try {
      runReference('good', { boss: 'x', dauer: 1 } as never);
    } catch (fehler) {
      gemeldet = String(fehler);
    }
    expect(gemeldet).toContain('"boss"');
    expect(gemeldet).toContain('"dauer"');
  });

  it('laesst die gueltigen Felder unveraendert durch (AC2)', () => {
    // Der Gegenfall: Eine Prüfung, die zu viel ablehnt, ist schlimmer als
    // keine — sie macht das Werkzeug unbenutzbar statt ungenau.
    const r = runReference('good', {
      durationMs: 20_000,
      stopOnKill: false,
      bossId: 'first-crucible',
    });
    expect(r.bossId).toBe('first-crucible');
    expect(r.damage).toBeGreaterThan(0);
  });
});

describe('Die Referenz steht (AC4)', () => {
  it('4240 @ 40510 unveraendert', () => {
    const r = runReference('good');
    expect(r.damage).toBe(4240);
    expect(r.killedAtMs).toBe(40_510);
  });
});
