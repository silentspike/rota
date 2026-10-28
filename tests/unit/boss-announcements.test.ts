/**
 * Die Boss-Ankündigungen (GAME-DESIGN 11.5, Issue #797).
 *
 * WAS ES VORHER GAB
 * -----------------
 * Von 11.5 standen im Bild nur der Balken des laufenden Boss-Zaubers (#327) und
 * die Überdruck-Warnung. Was als Nächstes kommt, sah der Spieler nicht — „aus
 * Reagieren Planen" (11.5) ging nicht, weil es nichts zu planen gab.
 *
 * WAS HIER GEPRÜFT WIRD
 * ---------------------
 * 1. **Jedes Zeitleisten-Ereignis jedes Bosses** wird angekündigt — mit dem
 *    Zeitpunkt, zu dem es WIRKLICH feuert (aus dem Kampfprotokoll, nicht aus
 *    einer zweiten Rechnung), mit Stufe und Warnart, über die volle Abklingzeit.
 *    Die Bosse kommen aus dem Verzeichnis, nicht aus einer Liste: Ein neuer Boss
 *    ist ohne Zeile Änderung dabei.
 * 2. Die Einstufung je Fähigkeit, an festen Fällen mit von Hand gesetzter
 *    Erwartung — ein Fall, der die Regel selbst aufriefe, prüfte sie gegen sich.
 * 3. Die Sortierung nach Dringlichkeit, der Countdown-Text, die Gleichheit von
 *    Fenster und Vollbild, Phasen-Bindung und STAY.
 *
 * Die Klänge (Kanal 3) stehen in `boss-cast-sound.test.ts`: Sie hängen an der
 * Sitzung, nicht an der Ankündigung, und brauchen deren Modul nicht.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  BossAnnouncer,
  FINAL_WINDOW_MS,
  KIND_DAMAGE,
  KIND_IDS,
  KIND_LETHAL,
  TIER_BAR,
  TIER_HIGHLIGHT,
  TIER_LETHAL,
  classifyEntry,
  countdownSecond,
  type Announcement,
} from '../../src/combat/boss/announcements.js';
import { Combat } from '../../src/core/combat.js';
import {
  BOSSES,
  CALCINATION,
  bossById,
  type BossDef,
  type TimelineEntryDef,
} from '../../src/core/data.js';
import { EVENT_NAMES } from '../../src/core/events.js';
import { CombatLog, EVENT_BOSS_TIMELINE_FIRED } from '../../src/core/log.js';
import { TICK_MS } from '../../src/core/loop.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { AnnouncementRenderer } from '../../src/render/announcements.js';

const BOSS_DIR = join(__dirname, '../../data/bosses');

function boss(id: string): BossDef {
  const gefunden = bossById(id);
  if (gefunden === undefined) throw new Error(`data/bosses/${id}.json fehlt.`);
  return gefunden;
}

/** Die Kennungen aller Bossdateien — aus dem Verzeichnis gelesen, nicht aus `BOSSES`. */
function bossIdsOnDisk(): string[] {
  return readdirSync(BOSS_DIR)
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((f) => (JSON.parse(readFileSync(join(BOSS_DIR, f), 'utf8')) as { id: string }).id);
}

function find(ann: BossAnnouncer, name: string): Announcement | undefined {
  for (let r = 0; r < ann.count; r++) {
    const a = ann.at(r);
    if (a.name === name) return a;
  }
  return undefined;
}

function playerOf(combat: Combat): { vesselMax: number; gearWardPermille: number } {
  return { vesselMax: combat.playerVesselMax, gearWardPermille: combat.wornValues.ward };
}

/**
 * Der Boss ohne die Bausteine, die einen untätigen Spieler töten, BEVOR die
 * Zeitleiste durch ist (Zähler, Schwächung, Stapel, davonlaufende Reaktion).
 *
 * **Die Zeitleiste bleibt Zeichen für Zeichen dieselbe.** Ohne diesen Schritt
 * endete `boss-1-2` nach 17 s am Zähler, und drei seiner vier Einträge kämen
 * nie an die Reihe — der Fall prüfte dann einen Bruchteil und hieße „alle".
 * Die Ankündigung liest keinen dieser Bausteine (`BossAnnouncer.update`).
 */
function nurZeitleiste(b: BossDef): BossDef {
  return { ...b, doom: null, affliction: null, stacks: null, runaway: null };
}

interface Feuer {
  readonly atMs: number;
  readonly progress: number;
}

/** Wann welcher Eintrag feuert — aus dem Kampfprotokoll, nicht nachgerechnet. */
function feuerOf(b: BossDef, horizonMs: number): Feuer[] {
  const combat = new Combat({ school: CALCINATION.value, boss: b });
  const log = new CombatLog();
  combat.log = log;
  combat.beginFight(0);
  for (let t = 0; t <= horizonMs; t += TICK_MS) combat.stepPassive(t);
  const out: Feuer[] = [];
  for (let i = 0; i < log.length; i++) {
    if (log.typeAt(i) !== EVENT_BOSS_TIMELINE_FIRED) continue;
    out.push({ atMs: log.timeAt(i), progress: log.valueAt(i) });
  }
  return out;
}

/** Wie weit ein Boss gefahren wird: eine Liste einmal, eine Schleife zweimal. */
function horizonOf(b: BossDef): number {
  const letzter = Math.max(...b.timeline.map((e) => e.atMs));
  const zuschlag = 10_000; // der längste Zauber in den Daten wirkt 3500 ms
  return b.timelineLoopMs === null ? letzter + zuschlag : 2 * b.timelineLoopMs + zuschlag;
}

describe('Jedes Zeitleisten-Ereignis jedes Bosses wird angekündigt (AC: alle Bossdateien)', () => {
  const ids = bossIdsOnDisk();

  it('liest dieselbe Menge Bosse wie das Spiel', () => {
    // Zwei Quellen, damit ein Fehler im Einlesen nicht beide Seiten gleich trifft.
    expect(ids.length, 'Bossdateien im Verzeichnis').toBe(BOSSES.size);
    for (const id of ids) expect(BOSSES.has(id), id).toBe(true);
  });

  for (const id of ids) {
    const original = boss(id);
    if (original.timeline.length === 0) continue;

    it(`${id}: jedes Ereignis mit Zeitpunkt, Stufe, Art und voller Abklingzeit`, () => {
      const b = nurZeitleiste(original);
      const horizon = horizonOf(b);
      const feuer = feuerOf(b, horizon);
      const laenge = b.timeline.length;
      const schleife = b.timelineLoopMs !== null;
      const platzOf = (progress: number): number => (schleife ? progress % laenge : progress);

      const combat = new Combat({ school: CALCINATION.value, boss: b });
      combat.beginFight(0);
      const ann = new BossAnnouncer();
      const geprueft = new Set<number>();
      /** Wann die vorige Wiederholung je Name endete — Beginn der vollen Abklingzeit. */
      const endeVorher = new Map<string, number>();
      let laufendVorher = -1;

      for (let t = 0; t <= horizon; t += TICK_MS) {
        combat.stepPassive(t);
        ann.update(combat, t);
        if (combat.playerDead) break;

        // Ende eines laufenden Zaubers: Ab hier beginnt die nächste Abklingzeit.
        if (laufendVorher >= 0 && combat.casting !== laufendVorher) {
          const name = b.timeline[platzOf(laufendVorher)]!.name;
          endeVorher.set(name, t);
        }
        // Ein Schlag, der in diesem Takt gefeuert hat, beendet seine Wiederholung sofort.
        for (const f of feuer) {
          if (f.atMs !== t) continue;
          const e = b.timeline[platzOf(f.progress)]!;
          if (e.mechanic !== 'interruptibleCast') endeVorher.set(e.name, t);
        }
        laufendVorher = combat.casting;

        // 1. Der laufende Zauber zählt auf seinen Treffer.
        const laufendName = combat.casting >= 0 ? b.timeline[platzOf(combat.casting)]!.name : null;
        if (laufendName !== null) {
          const a = find(ann, laufendName);
          expect(a, `${id} t=${t}: laufender Zauber ${laufendName} fehlt`).toBeDefined();
          expect(a!.casting, `${id} t=${t}: ${laufendName} als laufend`).toBe(true);
          expect(a!.dueAtMs, `${id} t=${t}: ${laufendName} zählt auf den Treffer`).toBe(
            combat.castLandsAtMs,
          );
        }

        // 2. Je Name die nächste Wiederholung — genau die, die das Protokoll feuern sieht.
        const naechste = new Map<string, Feuer>();
        for (const f of feuer) {
          if (f.atMs <= t) continue;
          const name = b.timeline[platzOf(f.progress)]!.name;
          if (!naechste.has(name)) naechste.set(name, f);
        }
        for (const [name, f] of naechste) {
          if (name === laufendName) continue;
          const platz = platzOf(f.progress);
          const eintrag = b.timeline[platz]!;
          const a = find(ann, name);
          const wo = `${id} t=${t}: ${name} (Eintrag ${platz}, feuert ${f.atMs})`;
          expect(a, `${wo} wird nicht angekündigt`).toBeDefined();
          expect(a!.casting, wo).toBe(false);
          expect(a!.dueAtMs, `${wo}: Zeitpunkt`).toBe(f.atMs);
          expect(a!.remainingMs, `${wo}: Restzeit`).toBe(f.atMs - t);
          const soll = classifyEntry(eintrag, playerOf(combat));
          expect(a!.tier, `${wo}: Stufe`).toBe(soll.tier);
          expect(KIND_IDS[a!.kind], `${wo}: Art`).toBe(KIND_IDS[soll.kind]);
          expect(a!.sinceMs, `${wo}: Balken über die volle Abklingzeit`).toBe(
            endeVorher.get(name) ?? 0,
          );
          if (f.atMs - t === TICK_MS) geprueft.add(platz);
        }

        // 3. Nichts angekündigt, was nicht kommt.
        for (let r = 0; r < ann.count; r++) {
          const a = ann.at(r);
          if (a.casting) continue;
          const f = naechste.get(a.name);
          if (f === undefined) {
            expect(
              a.dueAtMs,
              `${id} t=${t}: ${a.name} angekündigt für ${a.dueAtMs}, feuert aber nicht mehr`,
            ).toBeGreaterThan(horizon);
          }
        }
      }

      // Abdeckung: jeder Eintrag der Daten, außer den an eine nie erreichte Phase
      // gebundenen (die verfallen; eigener Fall unten).
      const erwartet: number[] = [];
      b.timeline.forEach((e, i) => {
        if (e.phase === undefined || e.phase <= 0) erwartet.push(i);
      });
      expect(
        [...geprueft].sort((x, y) => x - y),
        `${id}: ${geprueft.size} von ${erwartet.length} Einträgen bis zum Feuern angekündigt`,
      ).toEqual(erwartet);
    });
  }
});

describe('Die Einstufung je Fähigkeit', () => {
  const basis = { vesselMax: 5000, gearWardPermille: 0 };
  const eintrag = (id: string, name: string): TimelineEntryDef => {
    const e = boss(id).timeline.find((x) => x.name === name);
    if (e === undefined) throw new Error(`${id}: ${name} fehlt`);
    return e;
  };

  // Von Hand gesetzt: Stufe und Art je Fall, aus 11.5 gelesen, nicht aus der Regel.
  const faelle: ReadonlyArray<[string, string, number, number, string]> = [
    ['test-lethal-strike', 'Sunder', TIER_LETHAL, KIND_LETHAL, '18400 nach WARD 14720 >= 5000'],
    ['test-survivable-strike', 'Sunder', TIER_BAR, KIND_DAMAGE, '4600 nach WARD 3680 < 5000'],
    ['boss-1-1', 'Kindling Roar', TIER_HIGHLIGHT, KIND_DAMAGE, 'unterbrechbar, überlebbar'],
    ['boss-1-1', 'Backlash', TIER_BAR, KIND_DAMAGE, 'Schlag, überlebbar'],
    ['boss-2-2', 'Collapse', TIER_HIGHLIGHT, KIND_DAMAGE, 'der schwerste Zauber der Spiel-Bosse'],
    ['boss-1-2', 'Cinder Sweep', TIER_BAR, KIND_DAMAGE, 'an Phase 1 gebunden — gleiche Regel'],
  ];
  for (const [id, name, tier, kind, warum] of faelle) {
    it(`${id} / ${name}: Stufe ${tier}, ${KIND_IDS[kind]} (${warum})`, () => {
      const c = classifyEntry(eintrag(id, name), basis);
      expect(c.tier).toBe(tier);
      expect(KIND_IDS[c.kind]).toBe(KIND_IDS[kind]);
    });
  }

  it('bemisst „tödlich" am Spieler DIESES Kampfes, nicht an einer festen Zahl', () => {
    const sunder = eintrag('test-lethal-strike', 'Sunder');
    // 18400 * 0,8 = 14720: Mit 14720 Leben tötet er genau, mit 14721 nicht mehr.
    expect(classifyEntry(sunder, { vesselMax: 14720, gearWardPermille: 0 }).tier).toBe(TIER_LETHAL);
    expect(classifyEntry(sunder, { vesselMax: 14721, gearWardPermille: 0 }).tier).toBe(TIER_BAR);
    // WARD aus der Ausrüstung zählt mit: 0,2 + 0,1 = 0,3 → 12880.
    expect(classifyEntry(sunder, { vesselMax: 13000, gearWardPermille: 100 }).tier).toBe(TIER_BAR);
  });

  it('die Begründung für Stufe 2 gilt an den Daten: der Zauber ist der schwerste Eintrag', () => {
    // classifyEntry gibt dem unterbrechbaren Zauber Stufe 2, weil er in jedem
    // Spiel-Boss härter trifft als jeder Schlag. Kommt ein Boss, bei dem das
    // nicht mehr gilt, muss die Regel neu entschieden werden — nicht still weiter.
    let mitZauber = 0;
    for (const id of bossIdsOnDisk().filter((x) => /^boss-\d+-\d+$/.test(x))) {
      const t = boss(id).timeline;
      const zauber = t.filter((e) => e.mechanic === 'interruptibleCast').map((e) => e.damage);
      const schlaege = t.filter((e) => e.mechanic === 'strike').map((e) => e.damage);
      if (zauber.length === 0) continue;
      mitZauber++;
      if (schlaege.length === 0) continue;
      expect(
        Math.min(...zauber),
        `${id}: leichtester Zauber gegen schwersten Schlag`,
      ).toBeGreaterThan(Math.max(...schlaege));
    }
    expect(mitZauber, 'sonst prüft der Fall nichts').toBeGreaterThan(0);
  });
});

/** Ein Boss mit genau den Einträgen, die ein Fall braucht. */
function mitZeitleiste(timeline: TimelineEntryDef[], phases: BossDef['phases'] = []): BossDef {
  return {
    ...boss('test-warden'),
    id: 'test-announcements',
    timeline,
    phases,
    timelineLoopMs: null,
  };
}

function schlag(atMs: number, name: string, damage: number): TimelineEntryDef {
  return { atMs, mechanic: 'strike', name, damage, castTimeMs: 0, interruptsPlayer: false };
}

function zauber(atMs: number, name: string, damage: number, castTimeMs: number): TimelineEntryDef {
  return { atMs, mechanic: 'interruptibleCast', name, damage, castTimeMs, interruptsPlayer: false };
}

function stand(b: BossDef, bisMs: number): { combat: Combat; ann: BossAnnouncer } {
  const combat = new Combat({ school: CALCINATION.value, boss: b });
  combat.beginFight(0);
  const ann = new BossAnnouncer();
  for (let t = 0; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
  ann.update(combat, bisMs);
  return { combat, ann };
}

function reihenfolge(ann: BossAnnouncer): string[] {
  const out: string[] = [];
  for (let r = 0; r < ann.count; r++) out.push(ann.at(r).name);
  return out;
}

describe('Mehrere gleichzeitige Ereignisse: nach Dringlichkeit sortiert', () => {
  const b = mitZeitleiste([
    schlag(10_000, 'Alpha', 100),
    zauber(10_500, 'Beta', 600, 3000),
    schlag(10_800, 'Gamma', 99_999),
    schlag(30_000, 'Delta', 100),
  ]);

  it('außerhalb der letzten Sekunden liest sich die Liste in der Zeit', () => {
    expect(reihenfolge(stand(b, 0).ann)).toEqual(['Alpha', 'Beta', 'Gamma', 'Delta']);
  });

  it('in den letzten Sekunden steht das Tödliche oben, dann die Hervorhebung', () => {
    const { ann } = stand(b, 6000);
    expect(find(ann, 'Alpha')!.remainingMs).toBeLessThanOrEqual(FINAL_WINDOW_MS);
    expect(find(ann, 'Delta')!.remainingMs).toBeGreaterThan(FINAL_WINDOW_MS);
    expect(reihenfolge(ann)).toEqual(['Gamma', 'Beta', 'Alpha', 'Delta']);
  });
});

describe('Der Countdown-Text', () => {
  it('rundet auf wie jeder Countdown: 5-4-3-2-1', () => {
    expect([5000, 4999, 4001, 4000, 3001, 1001, 1000, 1, 0].map(countdownSecond)).toEqual([
      5, 5, 5, 4, 4, 2, 1, 1, 0,
    ]);
  });

  it('nennt die Fähigkeit und die Sekunde — „BETA" und „IN 5"', () => {
    const b = mitZeitleiste([zauber(10_000, 'Beta', 600, 3000)]);
    const combat = new Combat({ school: CALCINATION.value, boss: b });
    combat.beginFight(0);
    for (let t = 0; t <= 5800; t += TICK_MS) combat.stepPassive(t);
    const r = new AnnouncementRenderer();
    r.update(combat, 5800, true);
    const ctx = recordingContext();
    r.drawCenter(ctx);
    const texte = ctx.ops.filter((o) => o.op === 'fillText').map((o) => o.args[0]);
    expect(texte).toContain('BETA');
    expect(texte).toContain('IN ');
    expect(texte, '4200 ms Rest zeigen 5').toContain('5');
  });

  it('erscheint nicht für Stufe 1 — nur der Balken', () => {
    const b = mitZeitleiste([schlag(10_000, 'Alpha', 100)]);
    const combat = new Combat({ school: CALCINATION.value, boss: b });
    combat.beginFight(0);
    for (let t = 0; t <= 8000; t += TICK_MS) combat.stepPassive(t);
    const r = new AnnouncementRenderer();
    r.update(combat, 8000, true);
    const ctx = recordingContext();
    r.drawCenter(ctx);
    expect(ctx.ops.filter((o) => o.op === 'fillText')).toEqual([]);
  });
});

interface Op {
  readonly op: string;
  readonly args: unknown[];
}

/** Zeichenfläche, die jeden Aufruf mitschreibt — Entwurfs-Koordinaten, keine Pixel. */
function recordingContext(): CanvasRenderingContext2D & { ops: Op[] } {
  const ops: Op[] = [];
  const state: Record<string, unknown> = {};
  return new Proxy({ ops } as unknown as CanvasRenderingContext2D & { ops: Op[] }, {
    get(_target, prop: string) {
      if (prop === 'ops') return ops;
      if (prop in state) return state[prop];
      if (prop === 'measureText') return () => ({ width: 10 });
      return (...args: unknown[]) => {
        ops.push({ op: prop, args });
      };
    },
    set(_target, prop: string, value: unknown) {
      state[prop] = value;
      ops.push({ op: `set ${prop}`, args: [value] });
      return true;
    },
  });
}

describe('Fenster und Vollbild zeichnen dieselben Ankündigungen (Design-Gesetz 3)', () => {
  function zeichne(fenster: { fullscreen: boolean; w: number; h: number }): Op[] {
    const g = globalThis as Record<string, unknown>;
    g['document'] = { fullscreenElement: fenster.fullscreen ? {} : null };
    g['innerWidth'] = fenster.w;
    g['innerHeight'] = fenster.h;
    try {
      const b = mitZeitleiste([
        schlag(10_000, 'Alpha', 100),
        zauber(10_500, 'Beta', 600, 3000),
        schlag(10_800, 'Gamma', 99_999),
      ]);
      const combat = new Combat({ school: CALCINATION.value, boss: b });
      combat.beginFight(0);
      for (let t = 0; t <= 7000; t += TICK_MS) combat.stepPassive(t);
      const r = new AnnouncementRenderer();
      const ctx = recordingContext();
      r.measure(ctx);
      r.update(combat, 7000, true);
      r.drawVesselHighlight(ctx, 7000);
      r.drawCenter(ctx);
      r.drawList(ctx, 7000);
      r.drawFlash(ctx);
      return ctx.ops.concat([{ op: 'shake', args: [r.shakeOffset()] }]);
    } finally {
      delete g['document'];
      delete g['innerWidth'];
      delete g['innerHeight'];
    }
  }

  it('gleiche Aufrufe in Fenster 1280×720 und Vollbild 1920×1080', () => {
    const fenster = zeichne({ fullscreen: false, w: 1280, h: 720 });
    const vollbild = zeichne({ fullscreen: true, w: 1920, h: 1080 });
    expect(fenster.length, 'sonst vergleicht der Fall zwei leere Bilder').toBeGreaterThan(20);
    expect(fenster.some((o) => o.op === 'fillText' && o.args[0] === 'GAMMA')).toBe(true);
    expect(vollbild).toEqual(fenster);
  });
});

describe('Kanal 3: Warnklang je Art, Countdown für Stufe 3', () => {
  /** Die Warn- und Countdown-Ereignisse eines Laufs, als „Name@Zeit". */
  function warnungen(b: BossDef, bisMs: number): string[] {
    const out: string[] = [];
    const combat = new Combat({ school: CALCINATION.value, boss: b });
    const session = new Session(new PressQueue(), combat, {
      onEvent: (event, _key, timeMs) => {
        const name = EVENT_NAMES[event] ?? '?';
        if (name.startsWith('boss.warning.') || name.startsWith('boss.countdown.')) {
          out.push(`${name}@${timeMs}`);
        }
      },
    });
    session.startFight(0);
    for (let t = 0; t <= bisMs; t += TICK_MS) session.tick(t);
    return out;
  }

  it('Stufe 3: einmal „lethal", dann je Sekunde 5-4-3-2-1 bis zum Schlag', () => {
    expect(warnungen(boss('test-lethal-strike'), 12_000)).toEqual([
      'boss.warning.lethal@7000',
      'boss.countdown.5@7000',
      'boss.countdown.4@8000',
      'boss.countdown.3@9000',
      'boss.countdown.2@10000',
      'boss.countdown.1@11000',
    ]);
  });

  it('Stufe 2: einmal je Wiederholung — nicht noch einmal, wenn der Zauber zu wirken beginnt', () => {
    // Kindling Roar bei 3000 und 13000; der Schlag dazwischen ist Stufe 1 und
    // schweigt. Die Schleife (15000) bringt die dritte bei 18000 — sie liegt
    // schon im Fenster, wenn die zweite bei 15500 landet, und warnt genau dann.
    // Mein erster Entwurf dieses Falls hatte sie vergessen; der Lauf nicht.
    expect(warnungen(boss('boss-1-1'), 16_000)).toEqual([
      'boss.warning.damage@0',
      'boss.warning.damage@8000',
      'boss.warning.damage@15500',
    ]);
  });

  it('das Nebensächliche schweigt, solange das Tödliche in seinem Fenster ist', () => {
    const erstTot = mitZeitleiste([
      schlag(10_000, 'Gamma', 99_999),
      zauber(10_500, 'Beta', 600, 3000),
    ]);
    expect(warnungen(erstTot, 9000)).toEqual([
      'boss.warning.lethal@5000',
      'boss.countdown.5@5000',
      'boss.countdown.4@6000',
      'boss.countdown.3@7000',
      'boss.countdown.2@8000',
      'boss.countdown.1@9000',
    ]);
    // Umgekehrt meldet sich das Tödliche, auch wenn Leiseres schon läuft.
    const erstLeise = mitZeitleiste([
      zauber(10_000, 'Beta', 600, 3000),
      schlag(10_500, 'Gamma', 99_999),
    ]);
    expect(warnungen(erstLeise, 5500)).toEqual([
      'boss.warning.damage@5000',
      'boss.warning.lethal@5500',
      'boss.countdown.5@5500',
    ]);
  });
});

describe('Phasen-Bindung: angekündigt wird nur, was sicher kommt', () => {
  it('Rupture erscheint erst, wenn Phase 1 erreicht ist — und feuert dann pünktlich', () => {
    const b = { ...boss('test-phasenbindung'), hp: 600 };
    const combat = new Combat({ school: CALCINATION.value, boss: b });
    const log = new CombatLog();
    combat.log = log;
    combat.beginFight(0);
    const ann = new BossAnnouncer();
    const kindle = combat.indexOf('kindle');
    let erreichtMs = -1;
    for (let t = 0; t <= 36_000; t += TICK_MS) {
      combat.stepPassive(t);
      if (t >= 1000 && combat.bossPhase < 1 && combat.gcd.isReady(t)) combat.cast(kindle, t);
      ann.update(combat, t);
      const rupture = find(ann, 'Rupture');
      if (combat.bossPhase < 1) {
        expect(rupture, `t=${t}: vor der Phase angekündigt`).toBeUndefined();
      } else if (t < 36_000) {
        if (erreichtMs < 0) erreichtMs = t;
        expect(rupture, `t=${t}: nach der Phase fehlt Rupture`).toBeDefined();
        expect(rupture!.dueAtMs).toBe(36_000);
      }
    }
    expect(erreichtMs, 'sonst prüft der Fall keine erreichte Phase').toBeGreaterThan(0);
    let feuerte = false;
    for (let i = 0; i < log.length; i++) {
      if (log.typeAt(i) === EVENT_BOSS_TIMELINE_FIRED && log.valueAt(i) === 2) {
        feuerte = log.timeAt(i) === 36_000;
      }
    }
    expect(feuerte, 'Rupture feuert bei 36000').toBe(true);
  });
});

describe('STAY verschiebt den anstehenden Schlag — die Ankündigung zieht mit', () => {
  it('Sealing Word rückt von 4000 auf 7000, und genau dann feuert es', () => {
    const combat = new Combat({ school: CALCINATION.value, boss: boss('test-warden') });
    const log = new CombatLog();
    combat.log = log;
    combat.beginFight(0);
    const ann = new BossAnnouncer();
    const bind = combat.indexOf('bind');
    const stay = combat.indexOf('stay');
    for (let t = 0; t <= 3000; t += TICK_MS) {
      combat.stepPassive(t);
      if (t === 0 || t === 1500) combat.cast(bind, t);
      if (t === 3000) combat.cast(stay, t);
      ann.update(combat, t);
      if (t === 2990) expect(find(ann, 'Sealing Word')!.dueAtMs).toBe(4000);
    }
    expect(find(ann, 'Sealing Word')!.dueAtMs, 'nach STAY').toBe(7000);
    for (let t = 3010; t <= 7000; t += TICK_MS) combat.stepPassive(t);
    let feuer = -1;
    for (let i = 0; i < log.length; i++) {
      if (log.typeAt(i) === EVENT_BOSS_TIMELINE_FIRED && log.valueAt(i) === 0)
        feuer = log.timeAt(i);
    }
    expect(feuer).toBe(7000);
  });
});
