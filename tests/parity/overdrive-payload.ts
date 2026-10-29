/**
 * Overdrive in der Parität (#1177, Abschnitt 2, „Overdrive auf G").
 *
 * Eine aufgezeichnete Runde — die Rotation des Spiels, mitgeschrieben, nicht
 * abgetippt (GAME-DESIGN 18.4) — mit einem Druck auf G und zwei verworfenen.
 * Gegenstück: `sim/tests/parity_overdrive.rs`.
 *
 * # Was der Lauf berührt, und wo
 *
 * | Takt | Kampfzeit | Was |
 * | ---: | ---: | --- |
 * | 300 | 3000 ms | `test-cast` beginnt (2000 ms Wirkzeit) |
 * | 400 | 4000 ms | **G** — zündet, und bricht die Wirkzeit ab (GAME-DESIGN 7.1) |
 * | 400–2399 | 4000–23 990 ms | `crust` und `smoulder` im Fenster: kürzere Abklingzeit, dichtere Ticks |
 * | 1500 | 15 000 ms | **G** zum zweiten Mal — verworfen, verlängert nichts |
 * | 2399 | 23 990 ms | `quench` im letzten Takt des Fensters — noch mit Tempo |
 * | 2400 | 24 000 ms | `crust` im Takt des Endes — wieder ohne |
 * | 2600 | 26 000 ms | **G** zum dritten Mal — verworfen |
 *
 * **Die beiden Grenz-Takte sind gerechnet, nicht gewählt.** `crust` (6000 ms)
 * kommt unter Overdrive auf 4620 ms und wird an der nächsten Grenze der
 * Wartezeit geworfen: 0, 6000, 12 000, 18 000, 24 000. Der letzte Wurf fällt
 * genau auf das Ende des Fensters. `quench` löst keine Wartezeit aus und steht
 * damit an jedem Takt zur Wahl — er setzt die andere Seite der Grenze.
 *
 * **Ohne getragene Ausrüstung**: Jedes Tempo in diesem Lauf kommt aus
 * Overdrive.
 */
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';
import { Overdrive } from '../../src/combat/overdrive.js';
import { OVERDRIVE_PRESS } from '../../src/input/keys.js';
import type { Recording, RecordedPress } from '../../src/devtools/recording.js';
import { replayRecording } from '../../src/devtools/recording.js';
import { pressRecorder } from './press-recorder.js';

export const OVERDRIVE_NAME = 'overdrive';

const SCHEMA = 1;
const TICKS = 3000;
const ZUENDUNG_TAKT = 400;
const ZWEITER_DRUCK_TAKT = 1500;
const DRITTER_DRUCK_TAKT = 2600;
const WIRKZEIT_TAKT = 300;

/** Die Leiste dieser Aufzeichnung — VOR der Funktion (Issue #405). */
const OVERDRIVE_BINDINGS = [
  { slot: 0, spellId: 'crust' },
  { slot: 1, spellId: 'smoulder' },
  { slot: 2, spellId: 'kindle' },
  { slot: 3, spellId: 'quench' },
  { slot: 4, spellId: 'test-cast' },
] as const;

/**
 * Die Rotation einmal spielen und dabei mitschreiben.
 *
 * Vorrang: `test-cast` einmal bei 3000 ms, `crust`, sobald bereit, `smoulder`,
 * wenn der Brand ausläuft, sonst `kindle`. `quench` nur im letzten Takt des
 * Fensters.
 */
function aufzeichnen(): { recording: Recording; overdriveRemaining: number[] } {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const presses: RecordedPress[] = [];
  const bindings = OVERDRIVE_BINDINGS.map(({ slot, spellId }) => ({ slot, spellId }));
  const druecke = pressRecorder(combat, bindings, presses);
  const crust = alsSpellIndex(combat.indexOf('crust'));
  const smoulder = alsSpellIndex(combat.indexOf('smoulder'));
  const kindle = alsSpellIndex(combat.indexOf('kindle'));
  const quench = alsSpellIndex(combat.indexOf('quench'));
  const wirkzeit = alsSpellIndex(combat.indexOf('test-cast'));
  const letzterTakt = ZUENDUNG_TAKT + Overdrive.durationMs / TICK_MS - 1;

  const overdriveRemaining: number[] = [];
  for (let tick = 0; tick < TICKS; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp || combat.playerDead) break;

    // Der Druck auf G — mitgeschrieben auch dann, wenn er verworfen wird:
    // Genau das ist der zweite und dritte Fall.
    if (tick === ZUENDUNG_TAKT || tick === ZWEITER_DRUCK_TAKT || tick === DRITTER_DRUCK_TAKT) {
      combat.igniteOverdrive(timeMs);
      presses.push({ tick, slot: OVERDRIVE_PRESS });
    }
    if (tick === letzterTakt && combat.isReady(quench, timeMs)) druecke(quench, tick, timeMs);

    if (combat.gcd.isReady(timeMs) && combat.playerCasting < 0) {
      const wahl =
        tick === WIRKZEIT_TAKT && combat.isReady(wirkzeit, timeMs)
          ? wirkzeit
          : combat.isReady(crust, timeMs)
            ? crust
            : combat.dotRemainingMs(smoulder, timeMs) < GCD_MS && combat.isReady(smoulder, timeMs)
              ? smoulder
              : combat.isReady(kindle, timeMs)
                ? kindle
                : null;
      if (wahl !== null) druecke(wahl, tick, timeMs);
    }
    overdriveRemaining.push(combat.overdrive.remainingMs(timeMs));
  }

  return {
    recording: { name: OVERDRIVE_NAME, seed: 0, ticks: TICKS, presses, bindings },
    overdriveRemaining,
  };
}

export function overdrivePayload(): Record<string, unknown> {
  const { recording, overdriveRemaining } = aufzeichnen();
  const result = replayRecording(recording, { checksumPerTick: true });

  // Ein zweiter Durchgang über dieselben Drücke für die Zahlen, die das
  // Ergebnis nicht führt — dieselbe Bauart wie in `castTimePayload`.
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const wirk = new Map(recording.bindings?.map((b) => [b.slot, combat.indexOf(b.spellId)]) ?? []);
  let p = 0;
  for (let tick = 0; tick < recording.ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    if (combat.damageDealt >= combat.boss.hp || combat.playerDead) break;
    while (p < recording.presses.length) {
      const press = recording.presses[p];
      if (press === undefined || press.tick > tick) break;
      p++;
      if (press.slot === OVERDRIVE_PRESS) combat.igniteOverdrive(timeMs);
      else combat.cast(alsSpellIndex(wirk.get(press.slot) ?? -1), timeMs);
    }
  }

  return {
    schema: SCHEMA,
    name: recording.name,
    bossId: FIRST_CRUCIBLE.value.id,
    seed: recording.seed,
    ticks: recording.ticks,
    tickMs: TICK_MS,
    // Die Regel-Zahlen, wie das Spiel sie liest — der Rust-Fall vergleicht
    // sie mit seinen, statt sie abzuschreiben.
    overdrive: {
      press: OVERDRIVE_PRESS,
      durationMs: Overdrive.durationMs,
      catalysisPermille: Overdrive.catalysisBonus,
    },
    bindings: recording.bindings,
    presses: recording.presses.map((x) => ({ tick: x.tick, slot: x.slot })),
    expected: {
      damage: result.damage,
      accepted: result.accepted,
      rejected: result.rejected,
      castsCancelled: result.castsCancelled,
      killedAtMs: result.killedAtMs,
      finalChecksum: result.finalChecksum,
      sequenceChecksum: result.sequenceChecksum,
      // **Die Zahlen, die belegen, dass der Lauf die Sache berührt hat.**
      // Ohne sie wäre die Datei auch dann gültig, wenn nie gezündet würde.
      overdriveIgnitedAtMs: combat.overdriveIgnitedAtMs,
      crustCasts: combat.castCount(alsSpellIndex(combat.indexOf('crust'))),
      quenchCasts: combat.castCount(alsSpellIndex(combat.indexOf('quench'))),
      smoulderTicks: combat.dotTickCount(combat.indexOf('smoulder')),
    },
    checksums: result.checksums,
    overdriveRemaining,
  };
}
