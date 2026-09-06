/** Real progression fights for #1057, also used to produce the Rust replay input. */
import { Combat, isCastAccepted } from '../../src/core/combat.js';
import { bossById, CALCINATION, type BossDef } from '../../src/core/data.js';
import { GCD_MS } from '../../src/core/gcd.js';
import { TICK_MS } from '../../src/core/loop.js';
import { erwarteterStand } from '../../src/devtools/expected-gear.js';
import { replayRecording, type RecordedPress } from '../../src/devtools/recording.js';
import { wornFrom } from '../../src/gear/catalog.js';
import { type WornValues } from '../../src/gear/worn.js';

export const PRODUCTION_PHASES_NAME = 'production-phase-bindings';
export const PHASE_CASES = [
  { bossId: 'boss-1-2', previous: 1, gearSource: 'prologue, normal, ten mixed items' },
  { bossId: 'boss-3-4', previous: 2, gearSource: 'instance-2, normal, ten mixed items' },
] as const;

const BINDINGS = [
  { slot: 0, spellId: 'kindle' },
  { slot: 1, spellId: 'smoulder' },
  { slot: 2, spellId: 'pyre' },
  { slot: 3, spellId: 'quench' },
  { slot: 4, spellId: 'purge' },
] as const;

/**
 * The established good/bad priorities, with the same QUENCH/PURGE response
 * in both rotations. Every action passes through Combat.cast. Level 1 is
 * explicit: these bosses have no separate player-level calibration yet.
 * No talents, legendary items or set effects (in particular no Long Grip).
 */
export function playPhaseBoss(
  boss: BossDef,
  worn: WornValues,
  rotation: 'good' | 'bad',
  seed: number,
) {
  const combat = new Combat({ school: CALCINATION.value, boss, worn, seed, level: 1 });
  const indices = BINDINGS.map(({ spellId }) => combat.indexOf(spellId));
  const kindle = combat.indexOf('kindle');
  const smoulder = combat.indexOf('smoulder');
  const pyre = combat.indexOf('pyre');
  const quench = combat.indexOf('quench');
  const purge = combat.indexOf('purge');
  const presses: RecordedPress[] = [];
  let killedAtMs = -1;
  let damageAt20s = -1;
  let highestPhase = 0;
  const ticks = boss.overpressureMs / TICK_MS;
  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    highestPhase = Math.max(highestPhase, combat.bossPhase);
    if (timeMs === 20000) damageAt20s = combat.damageDealt;
    if (combat.playerDead) break;
    if (combat.bossHpRemaining <= 0) {
      killedAtMs = timeMs;
      break;
    }

    let slot = -1;
    if (combat.casting !== -1 && combat.isOffCooldown(quench, timeMs)) slot = 3;
    else if (combat.gcd.isReady(timeMs)) {
      const refreshBelow =
        rotation === 'good'
          ? GCD_MS
          : Math.floor((combat.spellAt(smoulder)?.dot?.durationMs ?? 0) / 2);
      if (
        boss.doom !== null &&
        combat.doomCharges > 0 &&
        combat.doom >= boss.doom.clearAmount &&
        combat.isReady(purge, timeMs)
      ) {
        slot = 4;
      } else if (
        combat.dotRemainingMs(smoulder, timeMs) < refreshBelow &&
        combat.isReady(smoulder, timeMs)
      ) {
        slot = 1;
      } else if (
        combat.resource >= (rotation === 'good' ? 15 : 1) &&
        combat.isReady(pyre, timeMs)
      ) {
        slot = 2;
      } else if (combat.isReady(kindle, timeMs)) slot = 0;
    }
    const index = indices[slot];
    if (index !== undefined && isCastAccepted(combat.cast(index, timeMs).result)) {
      presses.push({ tick, slot });
    }
  }
  return {
    recording: { name: boss.id, seed, ticks, bindings: BINDINGS, presses },
    damage: combat.damageDealt,
    damageTaken: combat.damageTaken,
    killedAtMs,
    playerDead: combat.playerDead,
    damageAt20s,
    highestPhase,
    setEffects: combat.activeSetEffects,
  };
}

export function productionPhasesPayload() {
  const runs = PHASE_CASES.flatMap(({ bossId, previous, gearSource }) => {
    const original = bossById(bossId);
    if (original === undefined) throw new Error(`Missing production boss: ${bossId}`);
    const worn = wornFrom(erwarteterStand(previous, 10));
    const played = playPhaseBoss(original, worn, 'good', 0);
    return [true, false].map((phasesEnabled) => {
      const boss = phasesEnabled ? original : { ...original, phases: [] };
      const name = `${bossId}-${phasesEnabled ? 'phases' : 'without-phases'}`;
      const recording = { ...played.recording, name };
      // Identical keys and inputs: the only change is phases: []. The replay
      // supplies all expected values, including the damage omitted by hashes.
      const replay = replayRecording(recording, { boss, worn, level: 1, checksumPerTick: true });
      return {
        ...recording,
        bossId,
        phasesEnabled,
        tickMs: TICK_MS,
        playerLevel: 1,
        gearSource,
        worn,
        longGrip: false,
        expected: {
          damage: replay.damage,
          damageTaken: replay.damageTaken,
          diedAtMs: replay.diedAtMs,
          killedAtMs: replay.killedAtMs,
          accepted: replay.accepted,
          rejected: replay.rejected,
          casts: replay.casts,
          finalChecksum: replay.finalChecksum,
          sequenceChecksum: replay.sequenceChecksum,
        },
        checksums: replay.checksums,
      };
    });
  });
  return { schema: 1, name: PRODUCTION_PHASES_NAME, runs };
}
