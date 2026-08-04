/**
 * Die fuenf Verlustquellen (Issue #81, Teil 1).
 *
 * Zwei davon sind heute nicht rechenbar (#151). Der Kern dieser Pruefung ist
 * deshalb nicht nur, dass die drei uebrigen stimmen, sondern dass die fehlenden
 * SICHTBAR fehlen — eine Quelle, die als Null erscheint, sieht aus wie
 * "kein Verlust" und ist die gefaehrlichere Luecke.
 */
import { describe, expect, it } from 'vitest';
import {
  dotTickSource,
  idleSource,
  interruptSource,
  missedCastSource,
  rankLosses,
  synergySource,
} from '../../src/analysis/loss-sources.js';

describe('Fehlende Datengrundlage wird ausgewiesen, nicht als Null gezeigt', () => {
  it('meldet die zwei Quellen, die #151 braucht, als unavailable', () => {
    const report = rankLosses(10_000, [
      missedCastSource(null, 500),
      idleSource(null, 1500, 500),
      interruptSource(null, 7000),
      dotTickSource([]),
    ]);
    expect(report.unavailable).toEqual(['missedCasts', 'idle', 'missedInterrupts']);
    // Und sie tauchen NICHT als Quelle mit Verlust 0 auf.
    expect(report.sources.map((s) => s.kind)).not.toContain('missedCasts');
  });

  it('rechnet sie ein, sobald die Zahl da ist — der Anschluss ist eine Zeile', () => {
    const report = rankLosses(10_000, [missedCastSource(6, 500), idleSource(2, 1500, 500)]);
    expect(report.unavailable).toEqual([]);
    expect(report.totalLost).toBe(6 * 500 + 2 * 500);
  });
});

describe('Verlorene DoT-Ticks — heute rechenbar', () => {
  it('rechnet Ticks mal Schaden je Tick', () => {
    const source = dotTickSource([{ spellId: 'smoulder', lostTicks: 14, damagePerTick: 40 }]);
    const value = source.compute();
    expect(value?.damage).toBe(560);
    expect(value?.detail).toContain('14 ticks lost');
  });

  it('meldet null Verlust, aber MISST — das ist der Unterschied zu unavailable', () => {
    const report = rankLosses(1000, [dotTickSource([])]);
    expect(report.unavailable).toEqual([]);
    expect(report.totalLost).toBe(0);
  });
});

describe('Verpasste Synergien', () => {
  it('rechnet verfuegbar minus genutzt', () => {
    expect(synergySource(7, 10, 900).compute()?.damage).toBe(2700);
  });

  it('ist unavailable, wenn es keine moeglichen Synergien gibt', () => {
    // In Schritt 1 gibt es nur eine Schule. Eine Ausbeute zu zeigen hiesse,
    // ueber etwas zu urteilen, das es nicht gibt.
    const report = rankLosses(1000, [synergySource(0, 0, 900)]);
    expect(report.unavailable).toEqual(['missedSynergies']);
  });
});

describe('Rangfolge und Bezugsgroesse', () => {
  it('bezieht alle Prozente auf erzielt plus verloren', () => {
    const report = rankLosses(63_000, [missedCastSource(42, 500)]);
    expect(report.potential).toBe(63_000 + 21_000);
    expect(report.totalShare).toBeCloseTo(21_000 / 84_000, 6);
  });

  it('hebt hoechstens drei hervor, groesste zuerst', () => {
    const report = rankLosses(10_000, [
      missedCastSource(10, 500),
      idleSource(2, 1500, 500),
      synergySource(7, 10, 900),
      dotTickSource([{ spellId: 'x', lostTicks: 14, damagePerTick: 40 }]),
    ]);
    expect(report.top).toHaveLength(3);
    for (let i = 1; i < report.top.length; i++) {
      expect(report.top[i - 1]!.damage).toBeGreaterThanOrEqual(report.top[i]!.damage);
    }
  });
});

describe('Die Gelegenheits-Regel — Struktur steht, Daten fehlen', () => {
  it('zaehlt Unterbrecher ueber die Gelegenheit, nicht ueber die Bereitschaft', () => {
    // AC5: keine Gelegenheit -> kein Verlust. Die Quelle bekommt die Zahl der
    // GELEGENHEITEN, nicht die Bereitschaftsdauer — deshalb ist 0 hier eine
    // Aussage und keine fehlende Messung.
    const ohne = rankLosses(10_000, [interruptSource(0, 7000)]);
    expect(ohne.sources).toEqual([]);
    expect(ohne.unavailable).toEqual([]);

    // AC6: mit ungenutzter Gelegenheit erscheint er.
    const mit = rankLosses(10_000, [interruptSource(2, 7000)]);
    expect(mit.sources.map((s) => s.kind)).toEqual(['missedInterrupts']);
    expect(mit.totalLost).toBe(14_000);
  });

  it('zaehlt ihn nicht zusaetzlich als verpasste Wirkung', () => {
    // AC6, zweite Haelfte: nicht doppelt. Die Fenster-Zahl fuer verpasste
    // Wirkungen und die Gelegenheits-Zahl sind getrennte Eingaben — es gibt
    // keinen Weg, auf dem derselbe Fehler in beide geraet.
    const report = rankLosses(10_000, [missedCastSource(0, 500), interruptSource(2, 7000)]);
    expect(report.sources.map((s) => s.kind)).toEqual(['missedInterrupts']);
    expect(report.totalLost).toBe(14_000);
  });
});

describe('Keine Rotations-Empfehlung', () => {
  it('nennt in keiner Ausgabe eine Reihenfolge', () => {
    const report = rankLosses(10_000, [
      missedCastSource(6, 500),
      idleSource(2, 1500, 500),
      synergySource(7, 10, 900),
      dotTickSource([{ spellId: 'x', lostTicks: 3, damagePerTick: 40 }]),
      interruptSource(1, 7000),
    ]);
    const texte = report.sources.map((s) => `${s.label} ${s.detail}`.toLowerCase()).join(' ');
    for (const wort of [
      'should',
      'optimal',
      'better',
      'instead',
      'rotation',
      'order',
      'try',
      'before',
    ]) {
      expect(texte, `"${wort}" darf nicht vorkommen`).not.toContain(wort);
    }
  });
});
