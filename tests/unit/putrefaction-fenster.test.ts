/**
 * PUTRID BLOOM und DECAY — vier Größen, zwei Fenster (Issue #479).
 *
 * # Vier von vier ungeschützt
 *
 * `tools/groessen-mutation.py`, Vollmessung aller 116 Paare:
 *
 * ```text
 *   decayCap.bonus                putrid-bloom  2 -> 1         NUR FRISCHE
 *   decayCap.durationMs           putrid-bloom  10000 -> 5000  NUR FRISCHE
 *   decayWard.permillePerStack    decay         40 -> 20       NUR FRISCHE
 *   decayWard.durationMs          decay         6000 -> 3000   NUR FRISCHE
 * ```
 *
 * Beide Zauber sind reine **Fenster**: Sie richten keinen Schaden an
 * (`baseDamage: 0`), sondern heben für eine Weile die Stapel-Obergrenze
 * beziehungsweise mindern den eingehenden Schlag. Ein Fenster hat immer zwei
 * Größen — **wie stark** und **wie lange** —, und keine der vier war gedeckt.
 *
 * # Warum ZWEI Messpunkte, nicht einer
 *
 * Die Stärke und die Dauer treffen sich am ersten Messpunkt: Halbiert man
 * `bonus`, stehen bei 12 s sechs Stapel; halbiert man `durationMs`, ebenfalls
 * sechs. **Ein Messpunkt fängt beide, kann sie aber nicht unterscheiden** —
 * und ein Fall, der bei zwei verschiedenen Ursachen dieselbe Zahl zeigt,
 * schickt den Leser in die falsche Richtung.
 *
 * Gemessen wurde deshalb an zwei Zeitpunkten, jede Zeile ein eigener Lauf mit
 * mutierten Daten:
 *
 * ```text
 *                                  12 s   16 s   Treffer 12000
 *   Bestand                         7      7      1280
 *   decayCap.bonus      2 -> 1      6      6      —
 *   decayCap.durationMs 10 -> 5 s   6      5      —
 *   permillePerStack   40 -> 20     —      —      1440
 *   decayWard.duration  6 -> 3 s    —      —      1600
 * ```
 *
 * Bei 16 s trennen sich die beiden Fälle: `bonus` hält die Obergrenze
 * gesenkt, die kürzere Dauer lässt sie ganz zurückfallen.
 *
 * # Der Träger ist gemessen, nicht gewählt
 *
 * `test-onslaught` schlägt bei 5000, 12000 und 19000 ms zu. Das ist der
 * einzige Prüf-Boss, dessen Treffer so liegen, dass **einer zwischen dem
 * halben und dem vollen Fenster fällt**: Wirft man DECAY bei 6000, reicht das
 * echte Fenster bis 12000, das halbierte nur bis 9000. Der Treffer bei 12000
 * liegt genau dazwischen.
 *
 * An `boss-1-1` — dem ersten Versuch — schlägt es bei 8000 und dann erst
 * wieder bei 15500. Der Treffer bei 8000 liegt in **beiden** Fenstern, der bei
 * 15500 in **keinem**. Der Fall wäre grün geblieben und hätte nichts gezeigt.
 */
import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex, CAST_OK } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

interface Lauf {
  /** Wann der Fenster-Zauber fiel, oder −1. */
  readonly wurfBei: number;
  readonly stapelBei12s: number;
  readonly stapelBei16s: number;
  /** Schaden des Boss-Schlags bei 12000 ms, oder −1. */
  readonly trefferBei12s: number;
}

/**
 * Baut mit CANKER und BLIGHT Stapel auf und wirft den Fenster-Zauber, sobald
 * vier liegen.
 *
 * **Vier, nicht fünf.** Bei fünf wäre die Obergrenze schon erreicht, und
 * PUTRID BLOOM käme zu spät, um noch etwas zu heben — der Lauf zeigte dann
 * die Stärke des Zaubers nicht, sondern nur den Zufall der Reihenfolge.
 */
function lauf(werfe: 'putrid-bloom' | 'decay' | null): Lauf {
  const boss = bossById('test-onslaught') ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss });
  const canker = alsSpellIndex(combat.indexOf('canker'));
  const blight = alsSpellIndex(combat.indexOf('blight'));
  const ziel = werfe ? alsSpellIndex(combat.indexOf(werfe)) : alsSpellIndex(0);

  let wurfBei = -1;
  let stapelBei12s = -1;
  let stapelBei16s = -1;
  let trefferBei12s = -1;
  let hpVor = combat.playerHp;

  for (let tick = 0; tick < 3000; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);

    if (werfe !== null && wurfBei < 0 && combat.decayStacks >= 4 && combat.isReady(ziel, timeMs)) {
      if (combat.cast(ziel, timeMs).result === CAST_OK) wurfBei = timeMs;
    } else if (combat.isReady(canker, timeMs)) {
      combat.cast(canker, timeMs);
    } else if (combat.isReady(blight, timeMs)) {
      combat.cast(blight, timeMs);
    }

    if (timeMs === 12000) stapelBei12s = combat.decayStacks;
    if (timeMs === 16000) stapelBei16s = combat.decayStacks;
    if (combat.playerHp < hpVor) {
      if (timeMs === 12000) trefferBei12s = hpVor - combat.playerHp;
      hpVor = combat.playerHp;
    }
  }
  return { wurfBei, stapelBei12s, stapelBei16s, trefferBei12s };
}

const OHNE = lauf(null);
const MIT_BLOOM = lauf('putrid-bloom');
const MIT_DECAY = lauf('decay');

describe('der Träger trägt — sonst misst nichts darunter etwas', () => {
  it('beide Zauber kommen überhaupt zustande, und zwar bei 6000 ms', () => {
    // Ein nie geworfener Zauber sähe wie ein wirkungsloser aus: Alle Zahlen
    // unten glichen denen des Kontrolllaufs, und der Fall wäre grün.
    expect(MIT_BLOOM.wurfBei, 'PUTRID BLOOM kam nicht zustande').toBe(6000);
    expect(MIT_DECAY.wurfBei, 'DECAY kam nicht zustande').toBe(6000);
  });

  it('der Boss schlägt bei 12000 ms wirklich zu', () => {
    // Ohne Schlag wäre die ganze DECAY-Gruppe unten trivial erfüllt. Und
    // dieser Fall wird rot, sobald jemand `test-onslaught`s Zeitleiste
    // verschiebt — dann gehören die Messpunkte nachgezogen, nicht die Zahlen.
    expect(OHNE.trefferBei12s, 'kein Schlag bei 12000 ms').toBe(1600);
  });

  it('ohne die beiden Zauber steht die Obergrenze bei fünf', () => {
    expect(OHNE.stapelBei12s).toBe(5);
    expect(OHNE.stapelBei16s).toBe(5);
  });
});

describe('PUTRID BLOOM hebt die Obergrenze (decayCap.bonus)', () => {
  it('zwei Stapel mehr: sieben statt fünf', () => {
    expect(MIT_BLOOM.stapelBei12s, 'die Obergrenze wurde nicht um zwei gehoben').toBe(7);
  });

  it('und der Zuschlag hält bis 16 s an', () => {
    // **Dieser Fall trennt Stärke von Dauer.** Ein halbierter `bonus` zeigt
    // bei 12 s sechs — genau wie eine halbierte Dauer. Erst hier gehen sie
    // auseinander: `bonus 1` hält sechs, das kürzere Fenster fällt auf fünf
    // zurück. Gemessen mit mutierten Daten, nicht hergeleitet.
    expect(MIT_BLOOM.stapelBei16s, 'der Zuschlag hält nicht an').toBe(7);
  });
});

describe('PUTRID BLOOMs Fenster ist zehn Sekunden lang (decayCap.durationMs)', () => {
  it('bei 16 s liegen noch sieben — mit halbem Fenster wären es fünf', () => {
    // Wurf bei 6000, Fenster bis 16000. Halbiert reichte es bis 11000, und
    // der Aufbau auf sieben fiele weg. Der Unterschied zu 12 s ist der Punkt:
    // dort zeigen beide Mutationen sechs, hier sechs gegen fünf.
    expect(MIT_BLOOM.stapelBei16s).toBe(7);
    expect(MIT_BLOOM.stapelBei16s - OHNE.stapelBei16s, 'kein Zuschlag mehr bei 16 s').toBe(2);
  });

  it('liegende Stapel bleiben liegen — die Grenze wirft nichts weg', () => {
    // Das steht so in den Daten begründet: Die Obergrenze entscheidet nur, ob
    // NACHGELEGT werden darf. Liefe sie als zweiter Verfall, wäre der Stand
    // bei 16 s kleiner als bei 12 s.
    expect(MIT_BLOOM.stapelBei16s).toBeGreaterThanOrEqual(MIT_BLOOM.stapelBei12s);
  });
});

describe('DECAY mindert den Schlag nach Stapelzahl (decayWard.permillePerStack)', () => {
  it('fünf Stapel à 40 ‰ nehmen ein Fünftel: 1600 wird zu 1280', () => {
    expect(MIT_DECAY.trefferBei12s, 'der Schlag wurde nicht um 20 % gemindert').toBe(1280);
  });

  it('die Stapelzahl dabei ist fünf — sonst stimmte die Rechnung zufällig', () => {
    // 1280 ist 20 % unter 1600, und 20 % sind 5 × 40 ‰. Ohne diese Zeile
    // bliebe offen, ob die Minderung wirklich pro Stapel rechnet oder eine
    // feste Zahl trifft, die zufällig passt.
    expect(MIT_DECAY.stapelBei12s).toBe(5);
  });
});

describe('DECAYs Fenster ist sechs Sekunden lang (decayWard.durationMs)', () => {
  it('der Schlag bei 12000 ms liegt gerade noch darin', () => {
    // Wurf bei 6000, Fenster bis 12000 — der Schlag liegt auf der Grenze,
    // und `<=` lässt ihn hinein. Halbiert reichte es bis 9000, dann käme der
    // volle Schaden an. Genau dieser Träger wurde dafür gesucht: An
    // `boss-1-1` liegt zwischen 8000 und 15500 kein Schlag, der die beiden
    // Fenster trennt.
    expect(MIT_DECAY.trefferBei12s).toBe(1280);
    expect(OHNE.trefferBei12s - MIT_DECAY.trefferBei12s, 'keine Minderung im Fenster').toBe(320);
  });

  it('GEGENPROBE: der Zauber mindert nicht einfach immer', () => {
    // Ohne DECAY steht derselbe Schlag ungemindert da. Ein Fall, der nur die
    // 1280 prüft, wäre auch dann grün, wenn der Boss von sich aus schwächer
    // zuschlüge.
    expect(OHNE.trefferBei12s).toBe(1600);
  });
});
