/**
 * Was am Kampfende festgehalten wird (Issue #179).
 *
 * Es wird WIRKLICH gespielt: Sitzung, Countdown, Tastendruecke, Kampfende. Ein
 * Prueffall mit von Hand gesetzten Zahlen wuerde die Rechnung aus #164 ein
 * zweites Mal messen -- was hier fehlte, war die VERBINDUNG zum Spiel.
 */
import { describe, expect, it } from 'vitest';
import { alsSpellIndex, Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE, type BossDef } from '../../src/core/data.js';
import { spellIndex } from '../helpers/spell-index.js';
import { COUNTDOWN_MS, OUTCOME_VICTORY, OUTCOME_WIPE, PHASE_ENDED } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { PressQueue } from '../../src/input/queue.js';
import { markFor } from '../../src/analysis/progress-mark.js';
import { RunPersistence } from '../../src/analysis/run-persistence.js';
import { findForbiddenFields } from '../../src/analysis/save-format.js';
import { pointsForRun } from '../../src/progression/points.js';
import { itemById } from '../../src/gear/catalog.js';
import {
  countsAsAttempt,
  recordRun,
  COUNTED_OUTCOMES,
  IGNORED_OUTCOMES,
  type RecordedState,
} from '../../src/analysis/run-recorder.js';

const KINDLE = spellIndex('kindle');
const SMOULDER = spellIndex('smoulder');
const LEER: RecordedState = {
  runHistory: [],
  progressMarks: [],
  bestRuns: [],
  talentPoints: {},
  items: [],
  defeatedBosses: [],
  level: 1,
  experience: 0,
};

const KONTEXT = {
  bossId: 'first-crucible',
  difficulty: 1,
  bossLevel: 1,
  gameVersion: '0.1.0',
  at: 1_800_000_000_000,
  seed: 0,
  instance: 1,
  endlessLevel: 0,
};

interface Lauf {
  readonly summary: ReturnType<Session['summary']>;
  readonly outcome: number;
}

/**
 * Einen Durchlauf spielen.
 *
 * `everyMs` steuert, wie gut gespielt wird -- der einzige Hebel, mit dem sich
 * ein besserer und ein schlechterer Versuch unterscheiden.
 */
function play(
  everyMs: number,
  fightMs = 60_000,
  boss: BossDef = FIRST_CRUCIBLE.value,
  loadout: readonly number[] = [KINDLE, SMOULDER],
): Lauf {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss });
  const session = new Session(queue, combat);
  loadout.forEach((spell, slot) => {
    session.bind(slot, alsSpellIndex(spell));
  });
  session.beginCountdown(0);

  // Bis zum ECHTEN Ende laufen lassen, nicht bis zu einer gewaehlten Dauer:
  // Ohne Sieg oder Ueberdruck bleibt der Ausgang 'none', und dann zaehlt der
  // Durchlauf gar nicht -- der Prueffall haette nichts gemessen.
  const ende = COUNTDOWN_MS + fightMs;
  let tick = 0;
  let presses = 0;
  let letzter = 0;
  for (let t = 0; t <= ende; t += 10) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % everyMs === 0) {
      queue.push(presses % loadout.length, tick, t, tick);
      presses++;
    }
    session.tick(t);
    letzter = t;
    tick++;
    if (session.phase === PHASE_ENDED) break;
  }
  return { summary: session.summary(letzter), outcome: session.outcome };
}

describe('AC6 — die Umrechnung laeuft ueber attemptFrom', () => {
  it('schreibt die VERBLIEBENE Gesundheit, nicht den Fortschritt', () => {
    // Der Nachweis, dass an dieser Aufrufstelle nicht verwechselt wurde. Die
    // beiden Zahlen ergaenzen sich zu 100 -- wer sie vertauscht, bekommt eine
    // Zahl, bei der niemand stutzt.
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_VICTORY, KONTEXT);

    const mark = markFor(ergebnis.progressMarks, 'first-crucible', 1);
    expect(mark?.bestRemainingPercent).toBeCloseTo(100 - lauf.summary.progressPercent, 6);
    // Und nicht der Fortschritt selbst -- sonst waere die Pruefung blind.
    expect(lauf.summary.progressPercent).toBeGreaterThan(0);
    expect(mark?.bestRemainingPercent).not.toBeCloseTo(lauf.summary.progressPercent, 6);
  });

  it('legt dieselbe Zahl in Marke UND Verlaufs-Eintrag', () => {
    // Zwei Wege zu ihr waeren zwei Gelegenheiten, sie zu verdrehen.
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_VICTORY, KONTEXT);
    expect(ergebnis.runHistory[0]?.remainingPercent).toBe(
      markFor(ergebnis.progressMarks, 'first-crucible', 1)?.bestRemainingPercent,
    );
  });
});

describe('AC1 — ein besserer Versuch verbessert die Marke', () => {
  it('meldet den Fortschritt beim zweiten, besseren Durchlauf', () => {
    const schwach = play(3000);
    const stark = play(1500);
    expect(stark.summary.progressPercent).toBeGreaterThan(schwach.summary.progressPercent);

    let stand: RecordedState = LEER;
    const erster = recordRun(stand, schwach.summary, schwach.outcome, KONTEXT);
    stand = erster;
    const zweiter = recordRun(stand, stark.summary, stark.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });

    expect(erster.mark?.improved).toBe(false); // erster Versuch: nichts zu unterbieten
    expect(zweiter.mark?.improved).toBe(true);
    expect(zweiter.mark?.gainedPercent).toBeGreaterThan(0);
  });
});

describe('AC2 — ein schlechterer Versuch laesst die Marke stehen', () => {
  it('zaehlt trotzdem als Versuch mit', () => {
    // Der Gegenfall. Ohne ihn bestuende auch eine Marke, die sich bei JEDEM
    // Versuch fortschreibt -- und die belohnte das Gegenteil dessen, wofuer sie
    // da ist.
    const stark = play(1500);
    const schwach = play(3000);

    const stand: RecordedState = recordRun(LEER, stark.summary, stark.outcome, KONTEXT);
    const vorher = markFor(stand.progressMarks, 'first-crucible', 1)?.bestRemainingPercent;

    const danach = recordRun(stand, schwach.summary, schwach.outcome, {
      ...KONTEXT,
      at: KONTEXT.at + 1,
    });
    const mark = markFor(danach.progressMarks, 'first-crucible', 1);

    expect(danach.mark?.improved).toBe(false);
    expect(mark?.bestRemainingPercent).toBe(vorher);
    expect(mark?.attempts).toBe(2);
    // Und im Verlauf steht er sehr wohl -- er hat stattgefunden.
    expect(danach.runHistory).toHaveLength(2);
  });
});

describe('AC3 — ein Sieg setzt die Marke auf null', () => {
  it('faellt auf 0 und merkt sich den Sieg', () => {
    // Ein Sieg muss erst einmal moeglich sein. Gemessen: KINDLE und SMOULDER
    // abwechselnd bringen in 60 s nur 2760 Schaden -- test-warden hat 3000 und
    // ueberlebt den Ueberdruck. KINDLE ALLEIN trifft jedes Fenster und faellt
    // ihn nach 43,5 s.
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, 120_000, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    expect(lauf.outcome).toBe(OUTCOME_VICTORY);

    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    const mark = markFor(ergebnis.progressMarks, 'first-crucible', 1);
    expect(mark?.bestRemainingPercent).toBe(0);
    expect(mark?.defeated).toBe(true);
  });
});

describe('Der Abbruch zaehlt nicht', () => {
  it('schreibt weder Marke noch Verlaufs-Eintrag', () => {
    const lauf = play(1500);
    const ergebnis = recordRun(LEER, lauf.summary, OUTCOME_WIPE, KONTEXT);

    expect(ergebnis.counted).toBe(false);
    expect(ergebnis.mark).toBeNull();
    expect(ergebnis.progressMarks).toEqual([]);
    expect(ergebnis.runHistory).toEqual([]);
  });

  it('steht als Liste da, nicht als Bedingung im Rumpf', () => {
    // Die Frage, ob ein Abbruch ein Versuch ist, gehoert dem Design. Als Liste
    // ist die Antwort ein Eintrag, als Bedingung waere sie ein Umbau.
    expect(countsAsAttempt(OUTCOME_VICTORY)).toBe(true);
    expect(countsAsAttempt(OUTCOME_WIPE)).toBe(false);
    expect(COUNTED_OUTCOMES).toHaveLength(3);
    expect(IGNORED_OUTCOMES).toEqual([OUTCOME_WIPE]);
  });
});

describe('Der uebergebene Stand bleibt unangetastet', () => {
  it('gibt einen neuen zurueck', () => {
    // Ein halb geschriebener Spielstand darf keine Marke verfaelschen.
    const lauf = play(1500);
    const stand: RecordedState = {
      runHistory: [],
      progressMarks: [],
      bestRuns: [],
      talentPoints: {},
      items: [],
      defeatedBosses: [],
      level: 1,
      experience: 0,
    };
    const kopie = JSON.stringify(stand);
    recordRun(stand, lauf.summary, lauf.outcome, KONTEXT);
    expect(JSON.stringify(stand)).toBe(kopie);
  });
});

describe('Punkte am echten Lauf (Issue #276)', () => {
  /**
   * Ein Sieg, der wirklich gespielt wird.
   *
   * Dieselbe Zusammenstellung wie beim Marken-Prueffall oben: KINDLE allein
   * faellt test-warden nach rund 43,5 s und laesst damit echte Restzeit
   * uebrig -- ohne die zahlten die Punkte null, und die Pruefung haette
   * nichts gemessen.
   */
  function sieg(): Lauf {
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, 120_000, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    return lauf;
  }

  it('zahlt fuer einen echten Sieg echte Punkte', () => {
    // Die Verbindung zwischen Formel und Spiel. Alles andere prueft
    // points.test.ts an gesetzten Zahlen -- hier kommt die Restzeit aus einem
    // gespielten Kampf.
    const lauf = sieg();
    expect(lauf.summary.remainingMs).toBeGreaterThan(0);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(ergebnis.pointsGained).toBeGreaterThan(0);
  });

  it('zahlt fuer einen gescheiterten Versuch nichts', () => {
    const lauf = play(3000);
    expect(lauf.summary.victory).toBe(false);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(ergebnis.pointsGained).toBe(0);
  });

  it('AC4 GEGENPROBE am echten Lauf: derselbe Sieg zahlt bei Abstand weniger', () => {
    const lauf = sieg();
    const passend = recordRun(LEER, lauf.summary, lauf.outcome, { ...KONTEXT, bossLevel: 20 });
    const entwachsen = recordRun({ ...LEER, level: 30 }, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      bossLevel: 20,
    });
    expect(passend.pointsGained).toBeGreaterThan(0);
    expect(entwachsen.pointsGained).toBeLessThan(passend.pointsGained);
    // Und die Erfahrung faellt mitsamt -- eine Waehrung, eine Kurve.
    expect(entwachsen.xpGained).toBeLessThan(passend.xpGained);
  });

  it('AC6 — der abgelegte Spielstand haelt keine Punktzahl', () => {
    // Die Auszahlung reist zur Anzeige und wird nicht abgelegt. Geprueft wird
    // am geschriebenen JSON, nicht am Typ: Ein Typ sagt, was gemeint war, der
    // Speicher sagt, was drinsteht.
    const speicher = new Map<string, string>();
    const persistenz = new RunPersistence('0.1.0', () => KONTEXT.at, {
      getItem: (k) => speicher.get(k) ?? null,
      setItem: (k, v) => {
        speicher.set(k, v);
      },
      removeItem: (k) => {
        speicher.delete(k);
      },
    });

    const lauf = sieg();
    const ergebnis = recordRun(persistenz.state, lauf.summary, lauf.outcome, KONTEXT);
    expect(ergebnis.pointsGained).toBeGreaterThan(0);
    expect(persistenz.persist(ergebnis)).toBe(true);

    const geschrieben = [...speicher.values()].join('\n');
    expect(geschrieben.length).toBeGreaterThan(0);
    // Der Betrag selbst taucht nirgends auf, und kein Feld traegt seinen Namen.
    expect(geschrieben).not.toContain('pointsGained');
    expect(geschrieben).not.toMatch(/"points"/);
    expect(findForbiddenFields(JSON.parse([...speicher.values()][0] as string))).toEqual([]);
  });
});

describe('Bestwert am echten Lauf (Issue #377)', () => {
  function warden(): Lauf {
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, 120_000, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    return lauf;
  }

  it('AC1 — ein gespielter Sieg schreibt den Bestwert, der vorher NIE geschrieben wurde', () => {
    const lauf = warden();
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);

    expect(LEER.bestRuns).toEqual([]);
    expect(ergebnis.bestRuns).toHaveLength(1);
    expect(ergebnis.bestRuns[0]?.metric).toBe('reserve');
    expect(ergebnis.bestRuns[0]?.value).toBe(lauf.summary.remainingMs);
    expect(ergebnis.best?.previous).toBeNull();
  });

  it('ein gescheiterter Versuch setzt KEINEN Bestwert', () => {
    // Er hat keine Reserve und keinen gehaltenen Ring. Eine 0 einzutragen
    // hiesse, eine Marke zu behaupten, die es nicht gibt.
    const lauf = play(3000);
    expect(lauf.summary.victory).toBe(false);
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(ergebnis.bestRuns).toEqual([]);
    expect(ergebnis.best).toBeNull();
  });

  it('AC5 — der zweite gleiche Lauf zahlt weniger als der erste', () => {
    const lauf = warden();
    const erster = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    const zweiter = recordRun(erster, lauf.summary, lauf.outcome, KONTEXT);

    expect(erster.pointsGained).toBeGreaterThan(0);
    expect(zweiter.pointsGained).toBeGreaterThan(0);
    expect(zweiter.pointsGained).toBeLessThan(erster.pointsGained);
    // Der Bestwert steht danach unveraendert da — derselbe Lauf verbessert nichts.
    expect(zweiter.bestRuns[0]?.value).toBe(erster.bestRuns[0]?.value);
    expect(zweiter.best?.improved).toBe(false);
  });

  it('DIE REIHENFOLGE-FALLE: die Differenz rechnet gegen den Wert VOR dem Lauf', () => {
    // Wer den Bestwert fortschreibt und danach die Differenz rechnet, bekommt
    // immer null — jeder Sieg zahlte dann nur den Rest-Ertrag, auch der erste.
    const lauf = warden();
    const erster = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);
    expect(erster.pointsGained).toBe(
      pointsForRun(lauf.summary, { bossLevel: KONTEXT.bossLevel, playerLevel: LEER.level }, null),
    );
    // Und die Gegenprobe: gegen den schon geschriebenen Wert waere es weniger.
    expect(erster.pointsGained).toBeGreaterThan(
      pointsForRun(
        lauf.summary,
        { bossLevel: KONTEXT.bossLevel, playerLevel: LEER.level },
        erster.bestRuns[0]?.value ?? null,
      ),
    );
  });

  it('AC4 — der Bestwert steht im abgelegten Spielstand, die Punkte nicht', () => {
    const speicher = new Map<string, string>();
    const persistenz = new RunPersistence('0.1.0', () => KONTEXT.at, {
      getItem: (k) => speicher.get(k) ?? null,
      setItem: (k, v) => {
        speicher.set(k, v);
      },
      removeItem: (k) => {
        speicher.delete(k);
      },
    });

    const lauf = warden();
    const ergebnis = recordRun(persistenz.state, lauf.summary, lauf.outcome, KONTEXT);
    expect(persistenz.persist(ergebnis)).toBe(true);

    // **Am geparsten Stand, nicht am Rohtext**: Der Spielstand liegt doppelt
    // serialisiert im Speicher (Nutzlast als Zeichenkette in einer Huelle mit
    // Pruefsumme). Eine Suche im Rohtext haette hier auf escapte
    // Anfuehrungszeichen geprueft — und waere rot geworden, obwohl der Wert
    // korrekt drinstand.
    const huelle = JSON.parse([...speicher.values()][0] as string) as { payload: string };
    const stand = JSON.parse(huelle.payload) as Record<string, unknown>;
    const beste = (stand['bestRuns'] as { metric: string; value: number }[])[0];

    // Der Rohwert ist da — ERREICHT, nicht gerechnet.
    expect(beste?.metric).toBe('reserve');
    expect(beste?.value).toBe(lauf.summary.remainingMs);
    // Die Punkte sind es nicht, und der Waechter bleibt still.
    expect(huelle.payload).not.toContain('pointsGained');
    expect(Object.keys(beste ?? {})).not.toContain('points');
    expect(findForbiddenFields(stand)).toEqual([]);
  });

  it('der Bestwert ueberlebt einen Neustart und wird wiedergelesen', () => {
    // Der eigentliche Zweck: Ohne das Lesen waere jeder Lauf ein erster, und
    // die Differenz-Auszahlung zahlte fuer immer den vollen Ertrag.
    const speicher = new Map<string, string>();
    const laden = (): RunPersistence =>
      new RunPersistence('0.1.0', () => KONTEXT.at, {
        getItem: (k) => speicher.get(k) ?? null,
        setItem: (k, v) => {
          speicher.set(k, v);
        },
        removeItem: (k) => {
          speicher.delete(k);
        },
      });

    const lauf = warden();
    const erste = laden();
    erste.persist(recordRun(erste.state, lauf.summary, lauf.outcome, KONTEXT));

    const zweite = laden();
    expect(zweite.state.bestRuns).toHaveLength(1);
    expect(zweite.state.bestRuns[0]?.value).toBe(lauf.summary.remainingMs);

    // Und der naechste Lauf zahlt darum nur noch den Rest-Ertrag.
    const nachher = recordRun(zweite.state, lauf.summary, lauf.outcome, KONTEXT);
    expect(nachher.best?.previous).toBe(lauf.summary.remainingMs);
    expect(nachher.pointsGained).toBeLessThan(
      recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT).pointsGained,
    );
  });
});

/**
 * Der Fund behält seine gewürfelten Zahlen (Issue #480).
 *
 * `rollLoot` würfelt Stufe und Qualität pro Fund — bis hierher übernahm die
 * Verbuchung nur `drop.itemId`, und beide Zahlen starben in dieser einen Zeile.
 * Der Verlust war unsichtbar: Es fehlte kein Feld, es wurde nur keines
 * geschrieben. Die Prüffälle zur WIRKUNG stehen in `fundstufe.test.ts`; hier
 * geht es allein darum, dass der Wurf überhaupt ankommt.
 */
describe('#480 — die Verbuchung verliert Stufe und Qualität nicht mehr', () => {
  const SIEG = () => {
    const boss = bossById('test-warden') as BossDef;
    const lauf = play(1500, 120_000, boss, [KINDLE]);
    expect(lauf.summary.victory).toBe(true);
    return lauf;
  };

  it('schreibt die Zahlen des Wurfs in den Besitz', () => {
    const lauf = SIEG();
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, KONTEXT);

    // ZUSICHERUNG: Es gab überhaupt einen Fund. Ohne sie wäre der Fall grün,
    // sobald der Wurf ausbleibt — und prüfte dann nichts.
    expect(ergebnis.drop).not.toBeNull();
    expect(ergebnis.items).toHaveLength(1);

    expect(ergebnis.items[0]?.itemLevel).toBe(ergebnis.drop?.itemLevel);
    expect(ergebnis.items[0]?.quality).toBe(ergebnis.drop?.quality);
  });

  it('und die Stufe ist die des Wurfs, nicht die der Vorlage', () => {
    // Der eigentliche Befund. Vorher fiel jeder Fund über die Kennung auf die
    // Vorlage zurück; stimmte die Stufe des Wurfs zufällig mit ihr überein,
    // sähe man keinen Unterschied. Instanz 1 wirft Stufe 1, die Vorlage steht
    // auf 40 — sie können sich hier nicht decken.
    const lauf = SIEG();
    const ergebnis = recordRun(LEER, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      instance: 1,
      difficulty: 0,
    });

    const vorlage = itemById(ergebnis.drop?.itemId ?? '')?.itemLevel;
    expect(ergebnis.items[0]?.itemLevel).toBe(1);
    expect(ergebnis.items[0]?.itemLevel).not.toBe(vorlage);
  });

  it('eine höhere Instanz wirft eine höhere Stufe — mit Zahlen', () => {
    // Instanz 14 mythisch gegen Instanz 1 normal: 85 gegen 1. Dieselben zwei
    // Zahlen, an denen Worker-3 den Verlust gemessen hat.
    const lauf = SIEG();
    const niedrig = recordRun(LEER, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      instance: 1,
      difficulty: 0,
    });
    const hoch = recordRun(LEER, lauf.summary, lauf.outcome, {
      ...KONTEXT,
      instance: 14,
      difficulty: 2,
    });

    expect(niedrig.items[0]?.itemLevel).toBe(1);
    expect(hoch.items[0]?.itemLevel).toBe(85);
  });
});
