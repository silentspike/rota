/**
 * Der Spielertod beendet den Kampf (GAME-DESIGN A.3, Issue #581).
 *
 * # Was hier schiefgehen kann, ohne dass es jemand merkt
 *
 * `checkDefeat()` hiess so und prüfte den Tod des Spielers nicht. Sie hatte
 * drei Zweige — schon zu Ende, Boss gefallen, Überdruck — und keinen für den
 * Tod. Der wurde in `hitPlayer` gesetzt und von **keiner** Stelle als Ende
 * gelesen: Ein toter Spieler sammelte weiter Schaden und gewann. Gemessen an
 * `boss-2-2`: Tod bei 15000 ms, Boss gefallen bei 52510 — 37 s später.
 *
 * Das Tückische daran war nicht der fehlende Zweig, sondern dass die
 * Auswertung den Tod bereits **kannte**: `OUTCOME_DEATH` und die Überschrift
 * `VESSEL LOST` standen seit jeher da, mit dem Vermerk „In Schritt 1
 * unerreichbar — der Testboss teilt keinen Schaden aus". Der Vermerk stimmte,
 * als er geschrieben wurde. Seit der ersten Instanz stimmt er nicht mehr, und
 * niemand hat es bemerkt, weil nichts kaputtging — es fehlte nur etwas.
 *
 * # Warum die Gegenprobe hier keine Zugabe ist
 *
 * Eine Regel „der Kampf endet beim Tod" lässt sich auch dadurch erfüllen, dass
 * der Kampf IMMER endet. Der Fall über den überlebten Lauf ist deshalb kein
 * Beiwerk, sondern die Hälfte der Aussage.
 */
import { describe, expect, it } from 'vitest';
import {
  alsSpellIndex,
  Combat,
  FIGHT_DIED,
  FIGHT_OVERPRESSURE,
  FIGHT_WON,
} from '../../src/core/combat.js';
import { bossById, CALCINATION } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { readFileSync } from 'node:fs';

/**
 * **6001, nicht 6000.** Der Überdruck von `first-crucible` liegt bei exakt
 * 60000 ms, und Takt 6000 ist der bei 59990 — mit 6000 Takten endete kein
 * überlebter Kampf je, und die Gegenprobe unten meldete `-1` statt eines
 * regulären Endes. Ein Zaunpfahl, der sich als fehlende Regel tarnt.
 */
const TAKTE = 6001;

type Lauf = {
  /** Kampfzeit, zu der `playerHp` auf 0 fiel — unabhängig vom Kampfende. */
  todBeiMs: number;
  /** Kampfzeit, zu der `checkDefeat` das Ende meldete. */
  endeBeiMs: number;
  fightState: number;
  schadenAmEnde: number;
};

/**
 * Fährt einen Kampf, bis `checkDefeat` das Ende meldet — wie `Session.tick`.
 *
 * Die beiden Zeitpunkte werden **getrennt** geführt: `todBeiMs` kommt aus
 * `diedAtMs` (wann die Lebenspunkte fielen), `endeBeiMs` aus dem Rückgabewert
 * von `checkDefeat`. Nur so lässt sich „Tod bei X, Ende bei X" als Aussage
 * prüfen; aus einer einzigen Zahl abgelesen wäre es eine Tautologie.
 */
function fahre(bossId: string): Lauf {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`Der Boss ${bossId} fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
  const kindle = alsSpellIndex(combat.indexOf('kindle'));

  for (let tick = 0; tick < TAKTE; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.checkDefeat(timeMs)) {
      return {
        todBeiMs: combat.diedAtMs,
        endeBeiMs: timeMs,
        fightState: combat.fightState,
        schadenAmEnde: combat.damageDealt,
      };
    }
    if (combat.isReady(kindle, timeMs)) combat.cast(kindle, timeMs);
  }
  return {
    todBeiMs: combat.diedAtMs,
    endeBeiMs: -1,
    fightState: combat.fightState,
    schadenAmEnde: combat.damageDealt,
  };
}

describe('AC2 — Tod bei X heisst Ende bei X', () => {
  it('boss-1-2: der Spieler stirbt nach 17 s und der Kampf endet dort', () => {
    const lauf = fahre('boss-1-2');

    // **Harte Zahlen, aus zwei verschiedenen Quellen gelesen.** 17000 ist die
    // Zahl, die Worker-3 unabhängig gemessen hat; sie steht hier ausgeschrieben
    // und nicht als Variable, damit eine Verschiebung auffällt statt sich
    // mitzubewegen.
    expect(lauf.todBeiMs).toBe(17000);
    expect(lauf.endeBeiMs).toBe(17000);
    expect(lauf.fightState).toBe(FIGHT_DIED);
  });

  it('boss-1-1: derselbe Zusammenhang bei einer ganz anderen Zahl', () => {
    // Zwei Bosse statt einem — sonst bliebe offen, ob die Gleichheit an der
    // Regel hängt oder daran, dass 17000 zufällig beides ist.
    const lauf = fahre('boss-1-1');
    expect(lauf.todBeiMs).toBe(44700);
    expect(lauf.endeBeiMs).toBe(44700);
    expect(lauf.fightState).toBe(FIGHT_DIED);
  });

  it('der Kampf endet NICHT als Überdruck oder Sieg, wenn der Spieler stirbt', () => {
    // Vor #581 wäre `boss-1-1` mit FIGHT_WON geendet: Der tote Spieler sammelte
    // weiter Schaden, bis der Boss fiel. Genau diese Verwechslung schliesst der
    // Fall aus — ein Ende allein genügt nicht, es muss das richtige sein.
    const lauf = fahre('boss-1-1');
    expect(lauf.fightState).not.toBe(FIGHT_WON);
    // 5200 ist die hp von boss-1-1, aus data/bosses/boss-1-1.json abgelesen.
    // (Der erste Entwurf schrieb 4900 — das ist boss-1-4. Der Vergleich war
    // trotzdem grün, weil 4900 unter 5200 liegt: eine richtige Zusicherung mit
    // falscher Begründung.)
    expect(lauf.schadenAmEnde).toBeLessThan(5200);
  });
});

describe('AC3 — ein überlebter Kampf endet weiterhin regulär', () => {
  it('boss-1-3 und boss-1-4 sterben nicht und laufen normal', () => {
    // **Die Hälfte der Aussage.** Eine Regel, die jeden Kampf beendet, erfüllte
    // AC2 ebenso — dieser Fall schliesst sie aus.
    for (const id of ['boss-1-3', 'boss-1-4']) {
      const lauf = fahre(id);
      expect(lauf.todBeiMs, `${id}: der Spieler stirbt unerwartet`).toBeLessThan(0);
      expect(lauf.fightState, `${id}: endet als Tod, obwohl niemand starb`).not.toBe(FIGHT_DIED);
    }
  });

  it('der Überdruck bleibt der Überdruck', () => {
    // Drei Bosse, die niemanden töten: Alle enden bei 60000 ms mit
    // FIGHT_OVERPRESSURE. Gemessen, nicht angenommen — mit KINDLE allein fällt
    // keiner von ihnen.
    for (const id of ['first-crucible', 'boss-1-3', 'boss-1-4']) {
      const lauf = fahre(id);
      expect(lauf.todBeiMs, `${id}: stirbt unerwartet`).toBe(-1);
      expect(lauf.endeBeiMs, `${id}: endet nicht beim Überdruck`).toBe(60000);
      expect(lauf.fightState, `${id}: falsches Ende`).toBe(FIGHT_OVERPRESSURE);
    }
  });

  it('und ein Sieg bleibt ein Sieg — an der Referenz des Projekts', () => {
    // **Der schärfste verfügbare Beleg.** `4240 @ 40510` ist die
    // Referenz-Rechnung, gegen die seit #96 gemessen wird (zuletzt in #493 als
    // AC4 zitiert). Ein Eingriff ins Kampfende, der sie verschiebt, fällt hier
    // auf — und zwar an einer Zahl, die niemand für diesen Fall erfunden hat.
    //
    // Die Rotation ist die schlichte aus `recordGoodRotation`: SMOULDER halten,
    // sonst PYRE ab 15 Vorrat, sonst KINDLE. Mit KINDLE allein siegt hier
    // niemand — der Boss fällt nicht, und der Fall prüfte den Überdruck ein
    // zweites Mal statt den Sieg.
    const boss = bossById('first-crucible');
    if (boss === undefined) {
      throw new Error('first-crucible fehlt — die Gegenprobe misst nichts');
    }
    const combat = new Combat({ school: CALCINATION.value, boss, seed: 0 });
    const kindle = alsSpellIndex(combat.indexOf('kindle'));
    const smoulder = alsSpellIndex(combat.indexOf('smoulder'));
    const pyre = alsSpellIndex(combat.indexOf('pyre'));

    let ende = -1;
    for (let tick = 0; tick < TAKTE && ende < 0; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      if (combat.checkDefeat(timeMs)) {
        ende = timeMs;
        break;
      }
      if (combat.dotRemainingMs(smoulder, timeMs) < 1500 && combat.isReady(smoulder, timeMs)) {
        combat.cast(smoulder, timeMs);
      } else if (combat.isReady(pyre, timeMs) && combat.resource >= 15) {
        combat.cast(pyre, timeMs);
      } else if (combat.isReady(kindle, timeMs)) {
        combat.cast(kindle, timeMs);
      }
    }

    expect(combat.diedAtMs, 'der Spieler ist gestorben — falscher Boss für diesen Fall').toBe(-1);
    expect(ende, 'die Referenz-Zeit hat sich verschoben').toBe(40510);
    expect(combat.damageDealt, 'die Referenz-Rechnung hat sich verschoben').toBe(4240);
    expect(combat.fightState).toBe(FIGHT_WON);
  });
});

describe('AC4 — die vier Siege der ersten Instanz haben lebende Sieger', () => {
  /**
   * Spielt eine eingecheckte Aufzeichnung mit `checkDefeat` nach und meldet,
   * womit der Kampf endete.
   *
   * **Mit der Rotation aus der Datei, nicht mit einer eigenen.** Der Verdacht
   * in #581 stützte sich auf einen Lauf mit KINDLE allein — dort stirbt der
   * Spieler an `boss-1-1` bei 44700 ms, und der gemeldete Sieg lag bei 51010.
   * Mit der Rotation, die tatsächlich aufgezeichnet ist, überlebt er. Eine
   * ärmere Rotation beantwortet die Frage nicht, sie stellt eine andere.
   */
  function spieleAufzeichnung(nummer: number): {
    endeBeiMs: number;
    todBeiMs: number;
    fightState: number;
  } {
    const datei = new URL(`../parity/recordings/instance1-boss-${nummer}.json`, import.meta.url)
      .pathname;
    const d = JSON.parse(readFileSync(datei, 'utf8')) as {
      bossId: string;
      seed?: number;
      ticks: number;
      bindings: { slot: number; spellId: string }[];
      presses: { tick: number; slot: number }[];
    };
    const boss = bossById(d.bossId);
    if (boss === undefined) throw new Error(`Der Boss ${d.bossId} fehlt.`);
    const combat = new Combat({ school: CALCINATION.value, boss, seed: d.seed ?? 0 });
    const platz = new Map<number, number>();
    for (const b of d.bindings) platz.set(b.slot, combat.indexOf(b.spellId));

    let pressIndex = 0;
    for (let tick = 0; tick < d.ticks; tick++) {
      const timeMs = tick * TICK_MS;
      combat.stepPassive(timeMs);
      if (combat.checkDefeat(timeMs)) {
        return { endeBeiMs: timeMs, todBeiMs: combat.diedAtMs, fightState: combat.fightState };
      }
      while (pressIndex < d.presses.length) {
        const p = d.presses[pressIndex];
        if (p === undefined || p.tick > tick) break;
        pressIndex++;
        const idx = platz.get(p.slot);
        if (idx !== undefined && idx >= 0) combat.cast(alsSpellIndex(idx), timeMs);
      }
    }
    return { endeBeiMs: -1, todBeiMs: combat.diedAtMs, fightState: combat.fightState };
  }

  it('boss-1-1, -3 und -4 gewinnen, ohne dass der Spieler stirbt', () => {
    // **Der Verdacht aus #581 ist damit widerlegt**, nicht bestätigt: Er lautete,
    // der gemeldete Sieg an boss-1-1 bei 51010 ms könnte einem Toten gehören,
    // weil der Tod bei 44700 gemessen worden war. Mit der aufgezeichneten
    // Rotation stirbt dort niemand.
    for (const [n, sieg] of [
      [1, 51070],
      [3, 57010],
      [4, 55520],
    ] as const) {
      const lauf = spieleAufzeichnung(n);
      expect(lauf.fightState, `boss-1-${n}: kein Sieg`).toBe(FIGHT_WON);
      expect(lauf.todBeiMs, `boss-1-${n}: TOTER SIEGER`).toBe(-1);
      expect(lauf.endeBeiMs, `boss-1-${n}: der Sieg liegt woanders`).toBe(sieg);
    }
  });

  it('boss-1-2 ist der eine, der stirbt — und gewinnt deshalb nicht', () => {
    // Die Gegenprobe zur Zeile darüber: Fände der Wächter nirgends einen Tod,
    // bliebe offen, ob er einen erkennen KANN.
    const lauf = spieleAufzeichnung(2);
    expect(lauf.fightState).toBe(FIGHT_DIED);
    expect(lauf.todBeiMs).toBe(17000);
    expect(lauf.endeBeiMs).toBe(17000);
  });
});
