import { describe, expect, it } from 'vitest';
import { CALCINATION, FIRST_CRUCIBLE, parseBoss, parseSchool } from '../../src/core/data.js';
import { computeDamage, NEUTRAL_FACTORS } from '../../src/core/damage.js';
import { RandomStreams, STREAM_COUNT, STREAM_CRIT, STREAM_DOT_TICK } from '../../src/core/rng.js';
import { STEP1_BINDINGS } from '../../src/core/loadout.js';
import { SOUND_MANIFEST } from '../../src/audio/manifest-data.js';
import { parseManifest } from '../../src/audio/manifest.js';
import { EV_SPELL_CAST } from '../../src/core/events.js';
import { SoundMap } from '../../src/core/sound-map.js';

describe('Zauber-Datensätze', () => {
  it('liest CALCINATION ohne Beanstandung', () => {
    expect(CALCINATION.problems).toEqual([]);
    expect(CALCINATION.value.school).toBe('CALCINATION');
    // Drei Kampfzauber, der Unterbrecher QUENCH (#110) und die drei Wege der
    // Koagulation (#79).
    expect(CALCINATION.value.spells.length).toBe(11);
  });

  it('trägt alle Zahlen aus STEP1-SPEC § 1', () => {
    const byId = new Map(CALCINATION.value.spells.map((s) => [s.id, s]));
    const kindle = byId.get('kindle');
    expect(kindle?.baseDamage).toBe(100);
    expect(kindle?.cooldownMs).toBe(1500);
    expect(kindle?.resourceGain?.['heat']).toBe(3);

    const smoulder = byId.get('smoulder');
    expect(smoulder?.dot?.tickDamage).toBe(40);
    expect(smoulder?.dot?.tickIntervalMs).toBe(2000);
    expect(smoulder?.dot?.durationMs).toBe(12_000);
    expect(smoulder?.dot?.refreshMode).toBe('reset');

    const pyre = byId.get('pyre');
    expect(pyre?.cooldownMs).toBe(5000);
    expect(pyre?.spender?.damagePerPoint).toBe(12);
    expect(pyre?.spender?.bonusThreshold).toBe(15);
    expect(pyre?.spender?.bonusMultiplier).toBe(2);
    expect(pyre?.unlockLevel).toBe(2); // STEP1-SPEC § 7
  });

  it('kennt die Hitze mit Obergrenze, Verfall und Markierung', () => {
    const resource = CALCINATION.value.resource;
    expect(resource.id).toBe('heat');
    expect(resource.cap).toBe(20);
    expect(resource.decayPerStepMs).toBe(2000);
    expect(resource.decayAmount).toBe(1);
    expect(resource.highlightAt).toBe(15);
  });

  it('weist einen anderen refreshMode als reset zurück', () => {
    // GAME-DESIGN 7.2: In V1 ist "reset" der einzige zulässige Wert. Ein anderer
    // wäre eine stille Design-Änderung.
    const result = parseSchool({
      schema: 1,
      school: 'X',
      resource: { id: 'heat', name: 'Heat', cap: 10, decayPerStepMs: 1000, decayAmount: 1 },
      spells: [
        {
          id: 'a',
          cooldownMs: 1000,
          dot: { tickDamage: 1, tickIntervalMs: 1000, durationMs: 2000, refreshMode: 'extend' },
        },
      ],
    });
    expect(result.problems.some((p) => p.includes('refreshMode'))).toBe(true);
    expect(result.value.spells[0]?.dot).toBeUndefined();
  });

  it('meldet eine unbekannte Fassung des Formats', () => {
    expect(parseSchool({ schema: 2, spells: [] }).problems[0]).toContain('Unbekannte Fassung');
  });

  it('meldet doppelte Kennungen und überspringt sie', () => {
    const result = parseSchool({
      schema: 1,
      school: 'X',
      resource: { id: 'r', name: 'R', cap: 1, decayPerStepMs: 1000, decayAmount: 1 },
      spells: [
        { id: 'a', cooldownMs: 0 },
        { id: 'a', cooldownMs: 0 },
      ],
    });
    expect(result.value.spells.length).toBe(1);
    expect(result.problems.some((p) => p.includes('mehrfach'))).toBe(true);
  });

  it('meldet eine Dauer, die kein Vielfaches des Tick-Abstands ist', () => {
    const result = parseSchool({
      schema: 1,
      school: 'X',
      resource: { id: 'r', name: 'R', cap: 1, decayPerStepMs: 1000, decayAmount: 1 },
      spells: [
        {
          id: 'a',
          cooldownMs: 0,
          dot: { tickDamage: 1, tickIntervalMs: 2000, durationMs: 5000, refreshMode: 'reset' },
        },
      ],
    });
    expect(result.problems.some((p) => p.includes('Vielfaches'))).toBe(true);
  });

  it('meldet eine krumme Abklingzeit', () => {
    const result = parseSchool({
      schema: 1,
      school: 'X',
      resource: { id: 'r', name: 'R', cap: 1, decayPerStepMs: 1000, decayAmount: 1 },
      spells: [{ id: 'a', cooldownMs: 1500.5 }],
    });
    expect(result.problems.some((p) => p.includes('ganzzahlig'))).toBe(true);
  });
});

describe('Boss-Datensatz', () => {
  it('liest THE FIRST CRUCIBLE ohne Beanstandung', () => {
    expect(FIRST_CRUCIBLE.problems).toEqual([]);
    expect(FIRST_CRUCIBLE.value.name).toBe('The First Crucible');
    expect(FIRST_CRUCIBLE.value.hp).toBe(4200);
    expect(FIRST_CRUCIBLE.value.overpressureMs).toBe(60_000);
    expect(FIRST_CRUCIBLE.value.resistances).toEqual({});
  });

  it('meldet einen Boss ohne Lebenspunkte oder Zeitlimit', () => {
    const result = parseBoss({ schema: 1, hp: 0, overpressureMs: 0 });
    expect(result.problems.length).toBe(2);
  });
});

describe('Schadensformel', () => {
  it('lässt den Grundschaden unverändert, wenn alle Faktoren neutral sind', () => {
    expect(computeDamage(100, NEUTRAL_FACTORS)).toBe(100);
    expect(computeDamage(360, NEUTRAL_FACTORS)).toBe(360);
  });

  it('wendet jeden Faktor an', () => {
    expect(computeDamage(100, { ...NEUTRAL_FACTORS, essence: 2 })).toBe(200);
    expect(computeDamage(100, { ...NEUTRAL_FACTORS, resistance: 0.75 })).toBe(75);
    expect(computeDamage(100, { ...NEUTRAL_FACTORS, crit: 2, talents: 1.1 })).toBe(220);
  });

  it('rundet einmal am Ende', () => {
    // Zwischenrundungen wären eine zweite Stelle, an der Spiel und Simulator
    // auseinanderlaufen könnten (GAME-DESIGN 18.4).
    expect(computeDamage(10, { ...NEUTRAL_FACTORS, essence: 1.04, talents: 1.04 })).toBe(11);
  });
});

describe('Zufallsströme', () => {
  it('legt für jede Verwendung einen eigenen Strom an', () => {
    const streams = new RandomStreams(1234);
    expect(streams.drawCounts().length).toBe(STREAM_COUNT);
    expect(streams.totalDraws).toBe(0);
  });

  it('liefert je Strom eine andere Folge', () => {
    // Aus einem gemeinsamen Strom würde eine zusätzliche Ziehung alles
    // Nachfolgende verschieben (GAME-DESIGN 18.4).
    const streams = new RandomStreams(1234);
    const a = streams.stream(STREAM_CRIT).next();
    const b = streams.stream(STREAM_DOT_TICK).next();
    expect(a).not.toBe(b);
  });

  it('liefert bei gleichem Startwert dieselbe Folge', () => {
    const first = new RandomStreams(99);
    const second = new RandomStreams(99);
    for (let i = 0; i < 100; i++) {
      expect(second.stream(STREAM_CRIT).next()).toBe(first.stream(STREAM_CRIT).next());
    }
  });

  it('bleibt im Bereich null bis eins', () => {
    const stream = new RandomStreams(7).stream(STREAM_CRIT);
    for (let i = 0; i < 10_000; i++) {
      const value = stream.next();
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThan(1);
    }
  });

  it('zählt Ziehungen und stellt sich wieder her', () => {
    const streams = new RandomStreams(5);
    const first = streams.stream(STREAM_CRIT).next();
    streams.stream(STREAM_CRIT).next();
    expect(streams.totalDraws).toBe(2);
    streams.reset();
    expect(streams.totalDraws).toBe(0);
    expect(streams.stream(STREAM_CRIT).next()).toBe(first);
  });

  it('behandelt sichere und unmögliche Wahrscheinlichkeiten ohne Ziehung', () => {
    const stream = new RandomStreams(3).stream(STREAM_CRIT);
    expect(stream.chance(0)).toBe(false);
    expect(stream.chance(1)).toBe(true);
    expect(stream.drawCount).toBe(0);
  });
});

describe('Anfangsbelegung', () => {
  it('legt die drei Zauber auf die obere Reihe von links', () => {
    // QUENCH steht seit #110 auf Platz 3 — HINTER den dreien, damit jede
    // bestehende Aufzeichnung unverändert gilt: Ein Tastendruck nennt einen
    // Platz, und die Plätze 0 bis 2 bedeuten weiterhin dasselbe.
    expect(STEP1_BINDINGS.map((b) => b.slot)).toEqual([0, 1, 2, 3]);
    expect(STEP1_BINDINGS.map((b) => b.spellId)).toEqual(['kindle', 'smoulder', 'pyre', 'quench']);
  });

  it('verweist auf Zauber, die es wirklich gibt', () => {
    const ids = new Set(CALCINATION.value.spells.map((s) => s.id));
    for (const binding of STEP1_BINDINGS) expect(ids.has(binding.spellId)).toBe(true);
  });

  it('hat für jeden Zauber einen Klang im Verzeichnis', () => {
    // Seit E1-9 steht die Zuordnung im Verzeichnis statt als Namensregel im
    // Code — geprüft wird deshalb das Verzeichnis, nicht eine Funktion.
    const map = new SoundMap(parseManifest(SOUND_MANIFEST).entries);
    map.bindSpells(CALCINATION.value.spells.map((s) => s.id));
    expect(map.problems).toEqual([]);
    for (const binding of STEP1_BINDINGS) {
      const index = CALCINATION.value.spells.findIndex((s) => s.id === binding.spellId);
      const sounds = map.soundsFor(EV_SPELL_CAST, index);
      expect(sounds.length).toBe(1);
      // Der Name folgt KEINER Regel — genau das war der Punkt von E1-9. QUENCH
      // ist der erste Fall, in dem es auffällt: Sein Klang heißt
      // `special-interrupt`, weil GAME-DESIGN 17.1 Unterbrechen unter den
      // sieben Sonderzaubern führt und nicht unter den 60 Zaubern.
      expect(map.ids[sounds[0] as number]).toBe(
        binding.spellId === 'quench' ? 'special-interrupt' : `spell-${binding.spellId}`,
      );
    }
  });
});
