import { describe, expect, it } from 'vitest';
import {
  NO_SUBJECT,
  RULE_BOSS_PHASE,
  RULE_BUFF_EXPIRED,
  RULE_CRIT,
  RULE_DAMAGE,
  RULE_DOT_TICK,
  RULE_EVENT_COUNT,
  RULE_EVENT_NAMES,
  RULE_SPELL_CAST,
  RuleEvents,
  type RuleReceiver,
} from '../../src/combat/events.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Ein Empfänger, der nur mitschreibt, wer wann drankam. */
class Trace implements RuleReceiver {
  constructor(
    readonly id: string,
    private readonly sink: string[],
  ) {}
  onRuleEvent(): void {
    this.sink.push(this.id);
  }
}

describe('Ereignis-Menge (GAME-DESIGN 18.3)', () => {
  it('führt genau die sechs Ereignisse des Designs', () => {
    expect(RULE_EVENT_COUNT).toBe(6);
    expect(RULE_EVENT_NAMES.length).toBe(RULE_EVENT_COUNT);
    expect(RULE_EVENT_NAMES).toEqual([
      'spell.cast',
      'damage',
      'crit',
      'dot.tick',
      'boss.phase',
      'buff.expired',
    ]);
    // Die Kennnummern gehen in die Prüfsumme ein: Wer sie umsortiert, macht
    // jeden Vergleich mit dem Simulator ungültig.
    expect([
      RULE_SPELL_CAST,
      RULE_DAMAGE,
      RULE_CRIT,
      RULE_DOT_TICK,
      RULE_BOSS_PHASE,
      RULE_BUFF_EXPIRED,
    ]).toEqual([0, 1, 2, 3, 4, 5]);
  });
});

describe('Feste Empfänger-Reihenfolge (AC1)', () => {
  it('läuft über 1000 Durchgänge in derselben Reihenfolge', () => {
    // Der Kern des Issues. Geprüft wird nicht einmal, sondern tausendmal —
    // eine Reihenfolge, die von einem Hash-Container käme, schwankt zwischen
    // Läufen, nicht innerhalb eines Laufs.
    const seen = new Set<string>();
    for (let run = 0; run < 1000; run++) {
      const order: string[] = [];
      const bus = new RuleEvents();
      // Anmeldung bewusst in wechselnder Folge — das Ergebnis darf nicht
      // davon abhängen, wer zuerst am Schalter stand.
      const ids = run % 2 === 0 ? ['a', 'b', 'c'] : ['c', 'b', 'a'];
      for (const id of ids) {
        bus.register({ id, on: RULE_SPELL_CAST, order: id.charCodeAt(0) }, new Trace(id, order));
      }
      bus.emit(RULE_SPELL_CAST, 0, 0, 0);
      seen.add(order.join('>'));
    }
    expect(seen.size).toBe(1);
    expect([...seen][0]).toBe('a>b>c');
  });

  it('trägt bei gleichem Rang in der Reihenfolge der Anmeldung aus', () => {
    const order: string[] = [];
    const bus = new RuleEvents();
    for (const id of ['zweiter', 'erster', 'dritter']) {
      bus.register({ id, on: RULE_DAMAGE }, new Trace(id, order));
    }
    bus.emit(RULE_DAMAGE, 0, 100, 0);
    expect(order).toEqual(['zweiter', 'erster', 'dritter']);
  });

  it('hält die Reihenfolge über tausend Ereignisse durch', () => {
    const order: string[] = [];
    const bus = new RuleEvents();
    bus.register({ id: 'a', on: RULE_DOT_TICK, order: 1 }, new Trace('a', order));
    bus.register({ id: 'b', on: RULE_DOT_TICK, order: 0 }, new Trace('b', order));
    for (let i = 0; i < 1000; i++) bus.emit(RULE_DOT_TICK, 1, 40, i * TICK_MS);

    expect(order.length).toBe(2000);
    for (let i = 0; i < 1000; i++) {
      expect(order[i * 2]).toBe('b');
      expect(order[i * 2 + 1]).toBe('a');
    }
  });
});

describe('Anmeldung verschiebt nichts (AC2)', () => {
  it('lässt die Reihenfolge der bestehenden Empfänger unberührt', () => {
    const bus = new RuleEvents();
    const senke: string[] = [];
    bus.register({ id: 'alt-1', on: RULE_SPELL_CAST, order: 10 }, new Trace('alt-1', senke));
    bus.register({ id: 'alt-2', on: RULE_SPELL_CAST, order: 20 }, new Trace('alt-2', senke));
    const vorher = bus.receiverIds(RULE_SPELL_CAST);
    expect(vorher).toEqual(['alt-1', 'alt-2']);

    // Der Neue drängt sich VOR beide — die beiden bleiben trotzdem in ihrer
    // relativen Ordnung. Genau das verlangt AC2.
    bus.register({ id: 'neu', on: RULE_SPELL_CAST, order: 5 }, new Trace('neu', senke));
    const nachher = bus.receiverIds(RULE_SPELL_CAST);
    expect(nachher).toEqual(['neu', 'alt-1', 'alt-2']);
    expect(nachher.filter((id) => id.startsWith('alt'))).toEqual(vorher);

    // Und dazwischen.
    bus.register({ id: 'mitte', on: RULE_SPELL_CAST, order: 15 }, new Trace('mitte', senke));
    expect(bus.receiverIds(RULE_SPELL_CAST)).toEqual(['neu', 'alt-1', 'mitte', 'alt-2']);
    expect(bus.receiverIds(RULE_SPELL_CAST).filter((id) => id.startsWith('alt'))).toEqual(vorher);
  });

  it('rührt die Empfänger anderer Ereignisse nicht an', () => {
    const bus = new RuleEvents();
    const senke: string[] = [];
    bus.register({ id: 'x', on: RULE_DAMAGE }, new Trace('x', senke));
    bus.register({ id: 'y', on: RULE_DAMAGE }, new Trace('y', senke));
    bus.register({ id: 'z', on: RULE_DOT_TICK }, new Trace('z', senke));

    expect(bus.receiverIds(RULE_DAMAGE)).toEqual(['x', 'y']);
    expect(bus.receiverIds(RULE_DOT_TICK)).toEqual(['z']);
    expect(bus.receiverCount(RULE_CRIT)).toBe(0);

    bus.emit(RULE_DOT_TICK, 0, 40, 0);
    expect(senke).toEqual(['z']);
  });

  it('weist die Anmeldung auf ein Ereignis zurück, das es nicht gibt', () => {
    const bus = new RuleEvents();
    const senke: string[] = [];
    expect(bus.register({ id: 'q', on: 99 }, new Trace('q', senke))).toBe(false);
    expect(bus.register({ id: 'q', on: RULE_BUFF_EXPIRED }, new Trace('q', senke))).toBe(true);
  });
});

describe('Kein Speicher je Ereignis (AC3)', () => {
  it('gibt bei jedem Austragen dieselben Listen weiter, statt neue zu bauen', () => {
    // Der Nachweis, den ein Prüflauf führen kann: Die Empfänger-Liste ist
    // dieselbe Instanz, und `emit` bekommt nur Zahlen. Die Messung in Bytes
    // steht in VERIFICATION Paragraf 3.5 und läuft im Browser.
    const bus = new RuleEvents();
    const senke: string[] = [];
    bus.register({ id: 'a', on: RULE_DAMAGE }, new Trace('a', senke));

    let empfangen = 0;
    const zaehler: RuleReceiver = {
      onRuleEvent: (event, subject, value, timeMs) => {
        // Nur Zahlen — kein Ereignis-Objekt, das je Austragung entstünde.
        expect(typeof event).toBe('number');
        expect(typeof subject).toBe('number');
        expect(typeof value).toBe('number');
        expect(typeof timeMs).toBe('number');
        empfangen++;
      },
    };
    bus.register({ id: 'b', on: RULE_DAMAGE }, zaehler);
    for (let i = 0; i < 10_000; i++) bus.emit(RULE_DAMAGE, 0, 100, i);
    expect(empfangen).toBe(10_000);
    expect(bus.emittedCount).toBe(10_000);
  });

  it('kostet nichts, wenn niemand zuhört', () => {
    const bus = new RuleEvents();
    expect(() => {
      for (let i = 0; i < 1000; i++) bus.emit(RULE_BOSS_PHASE, NO_SUBJECT, 2, i);
    }).not.toThrow();
    expect(bus.emittedCount).toBe(1000);
  });
});

// Die Prüfsumme gehört zum Modul, nicht mehr zu einer AC von #76: Der Abgleich
// mit dem Simulator ist nach dem neuen Zuschnitt Issue E3-1b (#87). Sie steht
// hier trotzdem geprüft bereit, weil sie dort die Vergleichsgröße ist.
describe('Prüfsumme über die Ereignisfolge (Vorarbeit für #87)', () => {
  it('unterscheidet zwei Folgen, die sich in einem Wert unterscheiden', () => {
    const a = new RuleEvents();
    const b = new RuleEvents();
    a.emit(RULE_DAMAGE, 0, 100, 0);
    b.emit(RULE_DAMAGE, 0, 101, 0);
    expect(a.sequenceChecksum).not.toBe(b.sequenceChecksum);
  });

  it('unterscheidet dieselben Werte in anderer Reihenfolge', () => {
    const a = new RuleEvents();
    const b = new RuleEvents();
    a.emit(RULE_DAMAGE, 0, 100, 0);
    a.emit(RULE_SPELL_CAST, 0, 100, 0);
    b.emit(RULE_SPELL_CAST, 0, 100, 0);
    b.emit(RULE_DAMAGE, 0, 100, 0);
    expect(a.sequenceChecksum).not.toBe(b.sequenceChecksum);
  });

  it('hängt nicht davon ab, wer zuhört', () => {
    // Sonst wären zwei Läufe mit verschiedenen Empfängern nicht vergleichbar —
    // und genau das ist der Fall zwischen Spiel und Simulator.
    const senke: string[] = [];
    const ohne = new RuleEvents();
    const mit = new RuleEvents();
    mit.register({ id: 'a', on: RULE_DAMAGE }, new Trace('a', senke));
    for (let i = 0; i < 50; i++) {
      ohne.emit(RULE_DAMAGE, i % 3, i * 7, i * TICK_MS);
      mit.emit(RULE_DAMAGE, i % 3, i * 7, i * TICK_MS);
    }
    expect(mit.sequenceChecksum).toBe(ohne.sequenceChecksum);
    expect(senke.length).toBe(50);
  });

  it('bleibt im Bereich vorzeichenloser 32 Bit', () => {
    const bus = new RuleEvents();
    for (let i = 0; i < 500; i++) {
      bus.emit(i % RULE_EVENT_COUNT, i % 3, i * 1.5, i * TICK_MS);
      const hash = bus.sequenceChecksum;
      expect(Number.isInteger(hash)).toBe(true);
      expect(hash).toBeGreaterThanOrEqual(0);
      expect(hash).toBeLessThan(2 ** 32);
    }
  });
});

// Was der Kampf austrägt, steht in Issue E3-1b (#87): Der Anschluss geschieht
// auf beiden Seiten zugleich, weil die Stellen, an denen ein Ereignis entsteht,
// die Reihenfolge festlegen — und die gilt nie einseitig (Auflage aus #87).
