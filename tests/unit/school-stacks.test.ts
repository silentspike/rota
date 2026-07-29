import { describe, expect, it } from 'vitest';
import { RULE_DOT_TICK, RULE_SPELL_CAST } from '../../src/combat/events.js';
import { StackMechanic } from '../../src/combat/schools/stacks.js';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

function makeCombat(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('Die Stapel-Mechanik hängt am Ereignis-System (E3-2)', () => {
  it('ist als Empfänger angemeldet, nicht im Kampfcode verdrahtet', () => {
    const combat = makeCombat();
    expect(combat.events.receiverIds(RULE_SPELL_CAST)).toContain('stacks:heat');
    expect(combat.events.receiverIds(RULE_DOT_TICK)).toContain('stacks:heat');
    expect(combat.stacks).toBeInstanceOf(StackMechanic);
  });

  it('ist die einzige Quelle der Hitze', () => {
    // Die Probe darauf, dass wirklich nichts mehr im Kampfcode steht: Ohne die
    // Mechanik darf keine Hitze entstehen.
    const combat = makeCombat();
    combat.events.reset();
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.resource).toBe(0);

    const wieder = makeCombat();
    wieder.cast(wieder.indexOf('kindle'), 0);
    expect(wieder.resource).toBe(3);
  });

  it('zählt jeden Gewinn mit', () => {
    const combat = makeCombat();
    combat.cast(combat.indexOf('kindle'), 0);
    expect(combat.stacks.gainCount).toBe(1);
  });
});

describe('Hitze: sammeln, verfallen, entladen (AC1)', () => {
  it('steigt bei jeder Wirkung um den Wert aus den Daten', () => {
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    // KINDLE bringt +3 (STEP1-SPEC 1.1), Wartezeit 1,5 s.
    for (let i = 0; i < 3; i++) combat.cast(kindle, i * 1500);
    expect(combat.resource).toBe(9);
  });

  it('steigt bei jedem Tick eines Effekts über Zeit', () => {
    const combat = makeCombat();
    // SMOULDER: +1 je Tick, sechs Ticks über 12 s (STEP1-SPEC 1.2).
    combat.cast(combat.indexOf('smoulder'), 0);
    const vorher = combat.stacks.gainCount;
    for (let t = TICK_MS; t <= 12_000; t += TICK_MS) combat.stepPassive(t);
    expect(combat.stacks.gainCount - vorher).toBe(6);
  });

  it('hebt sich beim SMOULDER-Tick genau mit dem Verfall auf', () => {
    // Erst beim Schreiben dieses Prüffalls aufgefallen, und es ist kein
    // Zufall: +1 je 2 s aus dem Tick gegen −1 je 2 s Verfall. Über die
    // gesamte Laufzeit bleibt die Hitze deshalb stehen — SMOULDER hält sie,
    // KINDLE baut sie auf. Das ist die Rechnung aus STEP1-SPEC 1.4, von der
    // anderen Seite gesehen.
    const combat = makeCombat();
    combat.cast(combat.indexOf('smoulder'), 0);
    const nachWirkung = combat.resource;
    for (let t = TICK_MS; t <= 12_000; t += TICK_MS) combat.stepPassive(t);
    expect(combat.resource).toBe(nachWirkung);
  });

  it('verfällt mit einem Punkt alle zwei Sekunden', () => {
    const combat = makeCombat();
    combat.beginFight(0);
    combat.setResource(10);
    // Der Verfall hängt an der Uhr, nicht an einem Ereignis — er läuft auch
    // ohne jede Eingabe weiter.
    for (let t = 0; t <= 6000; t += TICK_MS) combat.stepPassive(t);
    expect(combat.resource).toBe(7);
  });

  it('setzt die Hitze beim Entladen auf null', () => {
    const combat = makeCombat();
    combat.setResource(20);
    combat.cast(combat.indexOf('pyre'), 0);
    expect(combat.resource).toBe(0);
  });

  it('lässt den Entlade-Zauber lesen, bevor sein eigener Gewinn zählt', () => {
    // Die Regel, die `Fight::cast` im Simulator seit E2 festhält — und der
    // Grund, warum das Ereignis erst NACH der Abrechnung ausgetragen wird.
    const combat = makeCombat();
    combat.setResource(15);
    const outcome = combat.cast(combat.indexOf('pyre'), 0);
    // 15 Hitze mit Bonus: 12 × 15 × 2 = 360. Ein eigener Gewinn vorher hätte
    // eine andere Zahl ergeben.
    expect(outcome.damage).toBe(360);
    expect(outcome.resourceBefore).toBe(15);
  });

  it('deckelt bei zwanzig und zählt das Verpuffte mit (AC4-Muster)', () => {
    const combat = makeCombat();
    combat.setResource(19);
    combat.cast(combat.indexOf('kindle'), 0);
    // 19 + 3 = 22, gedeckelt auf 20, zwei verpuffen.
    expect(combat.resource).toBe(20);
    expect(combat.resourceOverflow).toBe(2);
  });

  it('hält die Obergrenze auch bei Dauerbeschuss (AC4)', () => {
    const combat = makeCombat();
    const kindle = combat.indexOf('kindle');
    for (let i = 0; i < 200; i++) combat.gainResource(3);
    expect(combat.resource).toBe(20);
    combat.cast(kindle, 0);
    expect(combat.resource).toBe(20);
  });
});

describe('Entlade-Schaden nach der Formel (AC2)', () => {
  // Die Beispiele aus STEP1-SPEC 1.3, Zeichen für Zeichen.
  const faelle: [number, number][] = [
    [10, 120],
    [14, 168],
    [15, 360],
    [20, 480],
  ];

  for (const [hitze, schaden] of faelle) {
    it(`bei ${hitze} Hitze ${schaden} Schaden`, () => {
      const combat = makeCombat();
      combat.setResource(hitze);
      expect(combat.cast(combat.indexOf('pyre'), 0).damage).toBe(schaden);
    });
  }

  it('nimmt die Schwelle selbst mit — „ab 15", nicht „über 14"', () => {
    const unter = makeCombat();
    unter.setResource(14);
    const ueber = makeCombat();
    ueber.setResource(15);
    // Ein Punkt mehr verdoppelt: 168 gegen 360. Genau diese Kante ist der
    // Kern der Mechanik (STEP1-SPEC 1.3).
    expect(unter.cast(unter.indexOf('pyre'), 0).damage).toBe(168);
    expect(ueber.cast(ueber.indexOf('pyre'), 0).damage).toBe(360);
  });

  it('richtet ohne Hitze keinen Schaden an', () => {
    const combat = makeCombat();
    expect(combat.cast(combat.indexOf('pyre'), 0).damage).toBe(0);
  });
});

describe('Der Umbau hat nichts verschoben', () => {
  it('ergibt dieselbe Hitze wie die Handrechnung aus STEP1-SPEC 1.4', () => {
    // `3 / 1,5 s` (KINDLE) `+ 1 / 2 s` (SMOULDER-Tick) `− 1 / 2 s` (Verfall)
    // = 2,0 Hitze je Sekunde. Über 6 s ohne Entladung: 12.
    const combat = makeCombat();
    combat.beginFight(0);
    const kindle = combat.indexOf('kindle');
    const smoulder = combat.indexOf('smoulder');
    combat.cast(smoulder, 0);
    let heat = combat.resource;
    for (let t = TICK_MS; t <= 6000; t += TICK_MS) {
      combat.stepPassive(t);
      if (combat.gcd.isReady(t) && combat.isReady(kindle, t)) combat.cast(kindle, t);
    }
    // Vier KINDLE (bei 1500, 3000, 4500, 6000) = 12, drei Ticks = 3,
    // drei Verfall-Schritte = −3. SMOULDER selbst bringt keine Hitze.
    expect(combat.resource - heat).toBe(12);
    heat = combat.resource;
    expect(heat).toBeLessThanOrEqual(20);
  });
});
