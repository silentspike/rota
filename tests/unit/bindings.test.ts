/**
 * Die Zauberleiste ist belegbar (GAME-DESIGN 14.4; Issue #264, AC2 bis AC4).
 *
 * AC4 HAT EINEN GEMESSENEN HINTERGRUND — MEINEN
 * ---------------------------------------------
 * Bei #248 war dieselbe Prüfung zuerst eine **Tautologie**: Sie stellte beide
 * Zustände selbst her und war grün, egal was die Umsetzung tat. Die Auflage
 * dieses Issues zieht die Lehre daraus, und der Fall unten hält sie ein: Beide
 * verglichenen Belegungen kommen aus `bindSpell`, keine wird hier gebaut.
 *
 * Die Frage, an der es hängt: **Woraus kommt die Zahl, gegen die verglichen
 * wird?** Aus dem Prüflauf selbst — dann ist es eine Tautologie.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import {
  bindProblem,
  bindSpell,
  canBind,
  clearSlot,
  defaultBindings,
  sanitize,
  spellAtSlot,
  type Bindings,
} from '../../src/progression/bindings.js';
import { barSlotsAtLevel } from '../../src/progression/unlocks.js';

const SPELLS = CALCINATION.value.spells;
const LEER: Bindings = {};

describe('AC2 — was nicht darf, geht nicht', () => {
  it('nennt getrennte Gründe, statt nur nein zu sagen', () => {
    // Der Spieler soll lesen können, WARUM es nicht geht. Ein `false` ließe
    // die Anzeige raten, und sie riete beim nächsten Umbau anders.
    expect(bindProblem('pyre', 0, 1, SPELLS, {}), 'PYRE ist Stufe 2').toBe('spell-locked');
    expect(bindProblem('kindle', 5, 1, SPELLS, {}), 'Stufe 1 hat zwei Plätze').toBe('slot-locked');
    expect(bindProblem('gibtsnicht', 0, 1, SPELLS, {})).toBe('unknown-spell');
    expect(bindProblem('kindle', 0, 1, SPELLS, {}), 'das geht').toBeNull();
  });

  it('lässt einen Zauber unterhalb seiner Stufe nicht auf die Leiste', () => {
    expect(canBind('pyre', 0, 1, SPELLS, {})).toBe(false);
    expect(canBind('pyre', 0, 2, SPELLS, {})).toBe(true);
  });

  it('lässt einen nicht freigeschalteten Platz nicht belegen', () => {
    // Auf Stufe 1 gibt es zwei Plätze: 0 und 1.
    expect(barSlotsAtLevel(1)).toBe(2);
    expect(canBind('kindle', 1, 1, SPELLS, {})).toBe(true);
    expect(canBind('kindle', 2, 1, SPELLS, {})).toBe(false);
    // Auf Stufe 3 kommt der dritte dazu.
    expect(canBind('kindle', 2, 3, SPELLS, {})).toBe(true);
  });

  it('gibt bei einem verbotenen Zug die UNVERÄNDERTE Belegung zurück', () => {
    // Eine halb geänderte wäre schlimmer als gar keine: Der Spieler sähe einen
    // Platz leer, den er nie geräumt hat.
    const vorher = bindSpell(LEER, 'kindle', 0, 1, SPELLS);
    const nachher = bindSpell(vorher, 'pyre', 1, 1, SPELLS);
    expect(nachher).toEqual(vorher);
    expect(spellAtSlot(nachher, 1)).toBeNull();
  });
});

describe('AC3 — die Belegung ist eine Zuordnung, kein berechneter Zustand', () => {
  it('führt Kennungen, keine Indizes', () => {
    // **Der Kern der Umstellung.** Ein Index ist die Position in einer Liste;
    // kommt ein Zauber dazu, zeigt jede gespeicherte Zahl auf einen anderen
    // Zauber. Der Spieler fände seine Leiste vertauscht, ohne dass etwas
    // kaputtgegangen wäre.
    const belegt = bindSpell(LEER, 'kindle', 0, 1, SPELLS);
    expect(belegt['0']).toBe('kindle');
    expect(typeof belegt['0']).toBe('string');
  });

  it('lässt den übergebenen Stand unberührt', () => {
    const a = bindSpell(LEER, 'kindle', 0, 1, SPELLS);
    bindSpell(a, 'smoulder', 1, 1, SPELLS);
    expect(a).toEqual({ '0': 'kindle' });
  });

  it('räumt einen Platz, statt ihn auf leeren Text zeigen zu lassen', () => {
    const belegt = bindSpell(LEER, 'kindle', 0, 1, SPELLS);
    const geraeumt = clearSlot(belegt, 0);
    expect('0' in geraeumt, 'der Eintrag ist weg, nicht leer').toBe(false);
    expect(spellAtSlot(geraeumt, 0)).toBeNull();
  });
});

describe('AC4 — bestehende Belegungen verschieben sich nicht', () => {
  it('behält jeden Platz, wenn ein neuer dazukommt', () => {
    // **Beide Belegungen kommen aus der Umsetzung.** Keine wird hier gebaut —
    // die Lehre aus #248, wo dieselbe Prüfung zuerst ihre eigene Erwartung mit
    // sich selbst verglich.
    //
    // Auf Stufe 1 sind zwei Plätze offen, auf Stufe 3 drei.
    let auf1 = bindSpell(LEER, 'kindle', 0, 1, SPELLS);
    auf1 = bindSpell(auf1, 'smoulder', 1, 1, SPELLS);
    expect(Object.keys(auf1)).toHaveLength(2);

    // Derselbe Stand, eine Stufe später, plus ein neuer Zauber auf dem neuen
    // Platz. Der Aufstieg selbst fasst die Belegung nicht an.
    const auf3 = bindSpell(auf1, 'pyre', 2, 3, SPELLS);

    expect(spellAtSlot(auf3, 0), 'Platz 0 bleibt KINDLE').toBe(spellAtSlot(auf1, 0));
    expect(spellAtSlot(auf3, 1), 'Platz 1 bleibt SMOULDER').toBe(spellAtSlot(auf1, 1));
    expect(spellAtSlot(auf3, 2)).toBe('pyre');
  });

  it('rückt beim Aufräumen NICHT nach', () => {
    // Ein ungültiger Eintrag fällt weg, die übrigen bleiben, wo sie sind.
    // Nachrücken wäre genau das Verschieben, das 14.4 verbietet — und es
    // geschähe hinter dem Rücken des Spielers.
    // **Der ERSTE Eintrag muss wegfallen, nicht der zweite.** Mein erster
    // Entwurf ließ Platz 0 gültig und Platz 1 wegfallen — dabei landet ein
    // nachrückender Eintrag zufällig auf demselben Platz, und die Gegenprobe
    // blieb grün. Erst wenn der vordere Platz frei wird, zeigt sich, ob der
    // hintere seine Nummer behält.
    let belegt = bindSpell(LEER, 'pyre', 0, 2, SPELLS);
    belegt = bindSpell(belegt, 'kindle', 1, 2, SPELLS);
    expect(Object.keys(belegt)).toHaveLength(2);

    // Auf Stufe 1 ist PYRE nicht mehr erlaubt. KINDLE bleibt auf Platz 1.
    const heil = sanitize(belegt, 1, SPELLS);
    expect(spellAtSlot(heil, 0), 'PYRE fällt weg, der Platz bleibt leer').toBeNull();
    expect(spellAtSlot(heil, 1), 'KINDLE rückt NICHT auf Platz 0 nach').toBe('kindle');
  });

  it('wirft einen Eintrag weg, dessen Platz es auf dieser Stufe nicht gibt', () => {
    // Ein Spielstand aus einer Fassung mit mehr Plätzen. Ihn stillschweigend
    // zu behalten hieße, dass der Kampf einen Platz kennt, den die Anzeige
    // nicht zeigt.
    const zuVieleAlt: Bindings = { '0': 'kindle', '9': 'smoulder' };
    const heil = sanitize(zuVieleAlt, 1, SPELLS);
    expect(spellAtSlot(heil, 0)).toBe('kindle');
    expect(spellAtSlot(heil, 9)).toBeNull();
  });
});

describe('Die Startbelegung ist ein Vorschlag, keine Empfehlung', () => {
  it('füllt nur so viele Plätze, wie offen sind', () => {
    expect(Object.keys(defaultBindings(1, SPELLS))).toHaveLength(barSlotsAtLevel(1));
    expect(Object.keys(defaultBindings(10, SPELLS)).length).toBeLessThanOrEqual(
      barSlotsAtLevel(10),
    );
  });

  it('nimmt keinen Zauber, den es auf dieser Stufe nicht gibt', () => {
    // **Zwei Bedingungen, nicht eine — und ich hatte nur an die erste
    // gedacht.** PYRE ist ab Stufe 2 freigeschaltet, aber die Startbelegung
    // nimmt ihn erst ab Stufe 3 auf: Vorher sind nur zwei Plätze offen, und
    // die tragen schon KINDLE und SMOULDER. Der erste Entwurf erwartete ihn
    // auf Stufe 2 und war rot, ohne dass an der Umsetzung etwas falsch war.
    expect(Object.values(defaultBindings(1, SPELLS))).not.toContain('pyre');
    expect(Object.values(defaultBindings(2, SPELLS)), 'nur zwei Plätze').not.toContain('pyre');
    expect(barSlotsAtLevel(3)).toBe(3);
    expect(Object.values(defaultBindings(3, SPELLS)), 'jetzt passt er').toContain('pyre');
  });

  it('folgt der Reihenfolge der Daten, nicht einer Bewertung', () => {
    // GAME-DESIGN sagt zweimal, dass das Zusammenstellen eine der beiden
    // Kernaufgaben ist. Eine Sortierung nach „bester Zauber" nähme sie weg —
    // und sie wäre schwer wieder herauszubekommen, weil der Spieler sich
    // darauf verlässt.
    const auf3 = defaultBindings(3, SPELLS);
    const erwartet = SPELLS.filter((s) => s.unlockLevel <= 3)
      .slice(0, barSlotsAtLevel(3))
      .map((s) => s.id);
    expect(Object.values(auf3)).toEqual(erwartet);
  });
});
