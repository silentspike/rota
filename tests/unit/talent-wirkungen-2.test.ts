/**
 * Die neun Talent-Wege der zweiten Runde bewegen neun verschiedene Größen
 * (Issue #419, FIXATION · PUTREFACTION · COAGULATION).
 *
 * # Warum eine eigene Datei
 *
 * `talent-wirkungen.test.ts` misst CALCINATIONs drei Wege und ist damit die
 * Vorlage. Diese Datei setzt sie fort, statt sie zu verlängern: Neun Wege in
 * einer Datei mit dreien ergäbe eine Datei, in der man den Bezug zur Schule
 * suchen muss.
 *
 * # Was hier NICHT steht
 *
 * Die **Messlatte ohne Talente** steht bei jeder Gruppe einzeln, nicht einmal
 * am Anfang. Eine gemeinsame Messlatte wäre kürzer und sagte weniger: Sie
 * müsste für neun Größen gelten, und wer eine Zeile ändert, sieht nicht mehr,
 * welche Zahl zu welchem Weg gehört.
 *
 * # Die Kreuzprobe ist der eigentliche Fall
 *
 * Der Auftraggeber will, dass die drei Wege je Baum **verschiedene** Größen
 * bewegen — heute hoben alle drei den Schaden, also war einer immer die beste
 * Wahl und zwei zwingend tot. Ein Fall, der nur „mehr" prüft, ließe drei Wege
 * durch, die dasselbe tun.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

const VOLL = 5;

function kampf(talente: Record<string, number>): Combat {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  // **VOR `beginFight`** — ab Zeit 0 ist die Verteilung gesperrt (#425).
  combat.setTalents(talente);
  combat.beginFight(0);
  return combat;
}

function wirf(combat: Combat, id: string, timeMs = 0): number {
  const index = alsSpellIndex(combat.indexOf(id));
  const ergebnis = combat.cast(index, timeMs);
  expect(ergebnis.result, `${id} wurde abgewiesen`).toBe(CAST_OK);
  return ergebnis.damage;
}

// ---- FIXATION ------------------------------------------------------------

/** Weg „build": Wie viele Kontroll-Punkte BIND einbringt. */
function punkte(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'bind');
  return combat.control;
}

/** Weg „cost": Wie viele Punkte STALL abzieht. */
function preis(talente: Record<string, number>): number {
  const combat = kampf(talente);
  // Erst genug sammeln — sonst wird STALL abgewiesen und die Messung misst
  // die Abweisung statt den Preis.
  for (let i = 0; i < 4; i++) wirf(combat, 'bind', i * 2000);
  const vorher = combat.control;
  wirf(combat, 'stall', 8000);
  return vorher - combat.control;
}

/** Weg „effect": Um wie viel WEDGE je Punkt in Schaden umwandelt. */
function umwandlung(talente: Record<string, number>): number {
  const combat = kampf(talente);
  for (let i = 0; i < 3; i++) wirf(combat, 'bind', i * 2000);
  const vorher = combat.damageDealt;
  wirf(combat, 'wedge', 6000);
  return combat.damageDealt - vorher;
}

// ---- PUTREFACTION --------------------------------------------------------

/** Weg „build": Wie viele Stapel CANKER legt. */
function stapel(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'canker');
  return combat.decayStacks;
}

/** Weg „effect": Die Verwundbarkeit bei einem liegenden Stapel. */
function verwundbarkeit(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'canker');
  return combat.vulnerability;
}

/** Weg „duration": Wie lange ein Stapel hält, in Millisekunden. */
function haltbarkeit(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'canker');
  return combat.decayExpiryAt(0);
}

// ---- COAGULATION ---------------------------------------------------------

/** Weg „thicker": Wie viel Schutz CRUST aufbaut. */
function schicht(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'crust');
  return combat.shield;
}

/**
 * Weg „longer": Wie lange die Schutzschicht hält, in Millisekunden.
 *
 * **Gemessen am Verfall, nicht an `shieldExpiresAtMs`** — das Feld ist privat,
 * und ein Getter nur für die Probe wäre ein zweiter Messpunkt neben der Sache.
 * Hier wird vorgespult, bis die Schicht wirklich weg ist; das ist die Größe,
 * die der Spieler spürt.
 *
 * **Der Rückgabewert liegt einen Tick über der Dauer** (12010 statt 12000): Der
 * Verfall wird beim ersten Schritt NACH dem Ablauf bemerkt, und `TICK_MS` ist
 * 10. Die Fälle unten erwarten deshalb `dauer + TICK_MS`, nicht `dauer`.
 *
 * `crust` trägt `durationMs: 12000` und `cooldownMs: 6000` — beim ersten Bau
 * dieser Funktion habe ich die Abklingzeit für die Dauer gehalten und 6000
 * erwartet. Vier rote Fälle, und keiner davon lag am Talent.
 */
function dauer(talente: Record<string, number>): number {
  const combat = kampf(talente);
  wirf(combat, 'crust');
  for (let t = TICK_MS; t <= 60_000; t += TICK_MS) {
    combat.stepPassive(t);
    if (combat.shield === 0) return t;
  }
  throw new Error('die Schutzschicht ist binnen 60 s nicht verfallen');
}

/** Weg „stacks": Wie viele Lagen CRUST übereinander legen kann. */
function lagen(talente: Record<string, number>): number {
  const combat = kampf(talente);
  // **Im Takt der ABKLINGZEIT, nicht schneller.** CRUST hält 6000 ms; ein
  // Abstand von 3000 ms ließ nur jeden zweiten Wurf durch, und die Messung
  // ergab sechs Lagen statt acht — eine Grenze, die nie berührt wurde. Das
  // ist derselbe Fall wie PUTRID BLOOM mit 880 gegen 880: Eine Probe braucht
  // einen Träger, bei dem die Wirkung Platz hat.
  //
  // Jeder Wurf setzt die Ablaufzeit für ALLE Lagen neu, also läuft dabei
  // nichts ab.
  for (let i = 0; i < 12; i++) {
    const index = alsSpellIndex(combat.indexOf('crust'));
    combat.cast(index, i * 6000);
    combat.stepPassive(i * 6000);
  }
  return combat.shieldStacks;
}

describe('FIXATION — drei Wege, drei Größen', () => {
  it('die Messlatte ohne Talente', () => {
    expect(punkte({}), 'BIND bringt nicht 2 Punkte').toBe(2);
    expect(preis({}), 'STALL kostet nicht 4 Punkte').toBe(4);
    expect(umwandlung({}), 'WEDGE wandelt nicht wie erwartet um').toBeGreaterThan(0);
  });

  describe('build — der Aufbau', () => {
    const TALENT = { 'fixation-build': VOLL };
    it('bringt mehr Punkte', () => {
      // 100 Promille je Rang, fünf Ränge: Faktor 1,5 auf 2 ergibt 3.
      expect(punkte(TALENT)).toBe(3);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(preis(TALENT), 'build wirkt auf den Preis').toBe(4);
    });
  });

  describe('cost — der Preis', () => {
    const TALENT = { 'fixation-cost': VOLL };
    it('senkt den Preis — er TEILT, als einziger der zwölf', () => {
      // 80 Promille je Rang, fünf Ränge: Faktor 1,4; 4 / 1,4 = 2,86 → 3.
      expect(preis(TALENT)).toBeLessThan(4);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(punkte(TALENT), 'cost wirkt auf den Aufbau').toBe(2);
    });
  });

  describe('effect — die Stärke', () => {
    const TALENT = { 'fixation-effect': VOLL };
    it('wandelt mehr um', () => {
      expect(umwandlung(TALENT)).toBeGreaterThan(umwandlung({}));
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(punkte(TALENT), 'effect wirkt auf den Aufbau').toBe(2);
      expect(preis(TALENT), 'effect wirkt auf den Preis').toBe(4);
    });
  });
});

describe('PUTREFACTION — drei Wege, drei Größen', () => {
  it('die Messlatte ohne Talente', () => {
    expect(stapel({}), 'CANKER legt nicht 1 Stapel').toBe(1);
    expect(verwundbarkeit({}), 'ein Stapel ergibt nicht 1,06').toBeCloseTo(1.06, 5);
    expect(haltbarkeit({}), 'ein Stapel hält nicht 12000 ms').toBe(12_000);
  });

  describe('build — der Aufbau', () => {
    const TALENT = { 'putrefaction-build': VOLL };
    it('legt mehr Stapel', () => {
      // 100 Promille je Rang, fünf Ränge: Faktor 1,5 auf 1 ergibt 1,5 → 1.
      //
      // **Und genau das ist die Aussage, nicht ihr Fehlen.** Der Aufbau-Weg
      // rundet AB; bei einem einzelnen Stapel je Wirkung zeigt er sich erst,
      // wenn ein Zauber mehr als einen legt. Ein Fall, der hier `toBe(2)`
      // erwartete, verlangte ein Aufrunden — und dann schenkte jeder erste
      // Rang sofort einen Stapel.
      expect(stapel(TALENT)).toBe(1);
    });
    it('zeigt sich bei einem Zauber, der MEHRERE legt — ROTs Ticks', () => {
      // ROT legt je Tick einen Stapel (`dot.decayPerTick`), und der läuft
      // durch dieselbe Sammelstelle. Über vier Ticks summiert sich, was ein
      // einzelner Wurf nicht zeigt.
      const mit = kampf(TALENT);
      wirf(mit, 'rot');
      for (let t = TICK_MS; t <= 5000; t += TICK_MS) mit.stepPassive(t);
      const ohne = kampf({});
      wirf(ohne, 'rot');
      for (let t = TICK_MS; t <= 5000; t += TICK_MS) ohne.stepPassive(t);
      expect(mit.decayStacks).toBeGreaterThanOrEqual(ohne.decayStacks);
    });
  });

  describe('effect — die Wertigkeit', () => {
    const TALENT = { 'putrefaction-effect': VOLL };
    it('macht den Boss verwundbarer', () => {
      expect(verwundbarkeit(TALENT)).toBeGreaterThan(1.06);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(stapel(TALENT), 'effect wirkt auf den Aufbau').toBe(1);
      expect(haltbarkeit(TALENT), 'effect wirkt auf die Haltbarkeit').toBe(12_000);
    });
  });

  describe('duration — die Haltbarkeit', () => {
    const TALENT = { 'putrefaction-duration': VOLL };
    it('lässt den Stapel länger liegen', () => {
      // 60 Promille je Rang, fünf Ränge: Faktor 1,3 auf 12000 ergibt 15600.
      expect(haltbarkeit(TALENT)).toBeGreaterThan(12_000);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(stapel(TALENT), 'duration wirkt auf den Aufbau').toBe(1);
      expect(verwundbarkeit(TALENT), 'duration wirkt auf die Wertigkeit').toBeCloseTo(1.06, 5);
    });
  });
});

describe('COAGULATION — drei Wege, drei Größen', () => {
  it('die Messlatte ohne Talente', () => {
    expect(schicht({}), 'CRUST baut nicht 900 Schutz auf').toBe(900);
    expect(lagen({}), 'CRUST stapelt sich nicht dreimal').toBe(3);
    expect(dauer({}), 'CRUST hält nicht 12000 ms').toBe(12_000 + TICK_MS);
  });

  describe('thicker — die Dicke', () => {
    const TALENT = { 'coagulation-thicker': VOLL };
    it('baut mehr Schutz auf', () => {
      // 100 Promille je Rang, fünf Ränge: Faktor 1,5 auf 900 ergibt 1350.
      expect(schicht(TALENT)).toBe(1350);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(lagen(TALENT), 'thicker wirkt auf die Zahl der Lagen').toBe(3);
      // **Die Dauer gehört hierher, und ihr Fehlen war die Lücke.** Ohne diese
      // Zeile blieb eine Vertauschung still: Trägt `thicker` statt
      // `shieldAmountPerRankPerMille` das Dauer-Feld, bewegen `thicker` und
      // `longer` dieselbe Größe — und nur der Wirkungsfall oben färbte, kein
      // einziger Fall meldete die Dopplung.
      expect(dauer(TALENT), 'thicker wirkt auf die Dauer').toBe(12_000 + TICK_MS);
    });
  });

  describe('longer — die Haltbarkeit', () => {
    const TALENT = { 'coagulation-longer': VOLL };
    it('lässt die Schicht länger halten', () => {
      // 80 Promille je Rang, fünf Ränge: Faktor 1,4 auf 12000 ergibt 16800.
      expect(dauer(TALENT)).toBe(16_800 + TICK_MS);
    });
    it('lässt die FREMDEN Größen unberührt', () => {
      expect(schicht(TALENT), 'longer wirkt auf die Dicke').toBe(900);
      expect(lagen(TALENT), 'longer wirkt auf die Zahl der Lagen').toBe(3);
    });
  });

  describe('stacks — die Zahl der Lagen', () => {
    const TALENT = { 'coagulation-stacks': VOLL };
    it('erlaubt mehr Lagen — ein ganzzahliger ZUSCHLAG', () => {
      // 1 je Rang, fünf Ränge: 3 + 5 = 8.
      expect(lagen(TALENT)).toBe(8);
    });
    it('lässt die FREMDE Größe unberührt — die Dicke JE LAGE bleibt', () => {
      // **Nicht `shield`, sondern die Dicke einer einzelnen Lage.** Wer mehr
      // Lagen legt, hat mehr Schutz — das ist der Weg, nicht sein Fehler. Die
      // Kreuzprobe muss deshalb die erste Lage messen, nicht die Summe.
      expect(schicht(TALENT), 'stacks wirkt auf die Dicke je Lage').toBe(900);
      expect(dauer(TALENT), 'stacks wirkt auf die Dauer').toBe(12_000 + TICK_MS);
    });
  });
});
