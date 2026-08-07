/**
 * Ein DoT-Tick verschiebt die Krit-Würfe der Zauber nicht (GAME-DESIGN 18.4,
 * Issue #602).
 *
 * # Der Befund, aus dem dieser Fall entstand
 *
 * `rollCrit()` zog bis zum 2026-08-07 **immer** aus `STREAM_CRIT` — auch für
 * den Krit-Wurf eines DoT-Ticks. Beide Seiten taten das gleich, die Parität
 * war intakt, und genau deshalb hat es niemand gesehen.
 *
 * Gefunden wurde es nicht an einer Zahl, sondern an einer **fehlenden
 * Erklärung neben einer vorhandenen**: `STREAM_HEAL` trägt im Code die
 * Begründung, warum er leer ist (keine Zufallsheilung). `STREAM_DOT_TICK` trug
 * keine — und dort war der Zufall vorhanden, er wurde nur aus dem falschen
 * Strom gezogen.
 *
 * # Was die Regel schützt, und was sie NICHT schützt
 *
 * Nicht die Parität — die war intakt. Sondern die **Vergleichbarkeit zweier
 * Läufe mit verschiedenen Rotationen**:
 *
 * > Krit, DoT-Ticks und Heilung brauchen je einen eigenen Strom — sonst
 * > verschiebt eine zusätzliche Ziehung alles Nachfolgende.
 *
 * Wer einen DoT hinzufügte, verschob damit jeden nachfolgenden Krit-Wurf. Ein
 * Balance-Vergleich zwischen zwei Zusammenstellungen maß dann teilweise
 * verschobenen Zufall statt der Rotation — und trifft damit den
 * Balance-Läufer, für den das ganze Determinismus-Gebäude gebaut wurde.
 *
 * # Warum der Fall die Ströme direkt fährt und keinen Kampf
 *
 * Ein Kampf mit und ohne DoT unterscheidet sich in mehr als dem DoT: andere
 * GCD-Last, anderer Schaden, andere Dauer. Die Aussage wäre dann von drei
 * Dingen abhängig. Hier wird nur das geprüft, was 18.4 sagt — dass eine
 * Ziehung im einen Strom die Folge im anderen nicht bewegt —, und zwar an
 * genau den beiden Strömen, die der Kampf benutzt.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { STREAM_CRIT, STREAM_DOT_TICK } from '../../src/core/rng.js';
import { critChance, NOTHING_WORN } from '../../src/gear/worn.js';

/** Genug PURITY, damit überhaupt gewürfelt wird — ohne sie schliesst `chance` kurz. */
const MIT_KRIT = { ...NOTHING_WORN, purity: 375 };

function neuerKampf(): Combat {
  const boss = bossById('test-crucible-trial') ?? FIRST_CRUCIBLE.value;
  return new Combat({ school: CALCINATION.value, boss, seed: 0, worn: MIT_KRIT });
}

/** Ziehungen je Strom, über den öffentlichen Weg. */
function ziehungen(combat: Combat): number[] {
  return combat.random.drawCounts();
}

describe('Der Krit-Wurf eines DoT-Ticks zieht aus dem eigenen Strom', () => {
  it('die Voraussetzung: es wird überhaupt gewürfelt', () => {
    // **Die Zusicherung vor der Messung.** Ohne PURITY schliesst `chance` bei
    // Wahrscheinlichkeit 0 kurz, ohne den Strom zu drehen — dann wären alle
    // Zählungen unten null und jeder Vergleich trivial grün.
    expect(
      critChance(MIT_KRIT),
      'die Kritchance ist null — es wird gar nicht gewürfelt',
    ).toBeGreaterThan(0);
    expect(critChance(NOTHING_WORN), 'ohne Ausrüstung wird schon gewürfelt').toBe(0);
    const combat = neuerKampf();
    const kindle = alsSpellIndex(combat.indexOf('kindle'));
    combat.stepPassive(0);
    combat.cast(kindle, 0);
    expect(ziehungen(combat)[STREAM_CRIT], 'ein gewirkter Zauber würfelt nicht').toBe(1);
  });

  it('ein gewirkter Zauber zieht aus STREAM_CRIT, ein DoT-Tick nicht', () => {
    const combat = neuerKampf();
    const kindle = alsSpellIndex(combat.indexOf('kindle'));
    combat.stepPassive(0);
    combat.cast(kindle, 0);

    expect(ziehungen(combat)[STREAM_CRIT]).toBe(1);
    expect(ziehungen(combat)[STREAM_DOT_TICK], 'ein Zauber zieht aus dem Tick-Strom').toBe(0);
  });

  it('ein DoT-Tick zieht aus STREAM_DOT_TICK, nicht aus STREAM_CRIT', () => {
    const combat = neuerKampf();
    const smoulder = alsSpellIndex(combat.indexOf('smoulder'));
    combat.stepPassive(0);
    combat.cast(smoulder, 0);

    // **SMOULDER selbst würfelt NICHT**, und das ist kein Versehen: Er hat
    // `baseDamage: 0` und trägt nur den DoT. `resolvePlayerCast` würfelt nur,
    // wenn der Zauber Schaden austeilt — der Wurf käme sonst zu einem Treffer,
    // den es nicht gibt. Der erste Entwurf dieses Falls erwartete hier 1.
    const nachDemWirken = [...ziehungen(combat)];
    expect(
      nachDemWirken[STREAM_CRIT],
      'SMOULDER würfelt beim Wirken — er teilt doch Schaden aus?',
    ).toBe(0);

    // Bis über den ersten Tick hinaus laufen (tickIntervalMs 2000).
    for (let tick = 1; tick <= 300; tick++) combat.stepPassive(tick * TICK_MS);

    const danach = ziehungen(combat);
    expect(danach[STREAM_DOT_TICK], 'die Ticks haben nicht gewürfelt').toBeGreaterThan(0);
    expect(
      danach[STREAM_CRIT],
      'ein DoT-Tick hat aus STREAM_CRIT gezogen — das verschiebt die Zauber-Würfe',
    ).toBe(nachDemWirken[STREAM_CRIT]);
  });

  it('AC2 — ein zusätzlicher DoT verschiebt die Krit-Würfe der Zauber NICHT', () => {
    // **Die Aussage aus 18.4, mit harten Zahlen.** Zwei Läufe, die sich in
    // genau einer Sache unterscheiden: ob ein DoT läuft. Die Folge der
    // Krit-Würfe für die gewirkten Zauber muss dieselbe sein.
    const folgeVon = (mitDot: boolean): number[] => {
      const combat = neuerKampf();
      const kindle = alsSpellIndex(combat.indexOf('kindle'));
      const smoulder = alsSpellIndex(combat.indexOf('smoulder'));
      // **Der Schaden je Wurf steht für den Krit-Wurf.** `CastOutcome` führt
      // kein Krit-Feld; ein Krit macht aber mehr Schaden, und die FOLGE der
      // Schadenswerte ist damit die Folge der Würfe.
      const schaeden: number[] = [];

      combat.stepPassive(0);
      if (mitDot) combat.cast(smoulder, 0);

      // **6000 Takte, nicht 600.** Bei 1500 ms Abklingzeit ergeben 600 Takte
      // (= 6 s) genau vier Würfe — zu wenige, um eine Verschiebung sichtbar zu
      // machen. Der erste Entwurf hatte das, und die Zusicherung darunter hat
      // es gefangen.
      for (let tick = 1; tick <= 6000; tick++) {
        const timeMs = tick * TICK_MS;
        combat.stepPassive(timeMs);
        if (combat.isReady(kindle, timeMs)) {
          const ergebnis = combat.cast(kindle, timeMs);
          if (ergebnis.result === 0) schaeden.push(ergebnis.damage);
        }
      }
      return schaeden;
    };

    const ohne = folgeVon(false);
    const mit = folgeVon(true);

    // **Die zweite Zahl, die für die Aussage niemand braucht** (Guide 8.67):
    // Beide Läufe müssen überhaupt gewirkt haben, und beide gleich oft. Wären
    // die Listen leer oder verschieden lang, wäre der Vergleich darunter
    // trivial oder schief.
    expect(ohne.length, 'im Lauf ohne DoT wurde nichts gewirkt').toBeGreaterThan(5);
    expect(
      mit.length,
      'die Läufe wirken verschieden oft — dann vergleicht die Zeile darunter zwei verschiedene Rotationen',
    ).toBe(ohne.length);

    expect(mit, 'der DoT hat die Krit-Würfe der Zauber verschoben').toEqual(ohne);
  });
});
