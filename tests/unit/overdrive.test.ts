/**
 * Overdrive im Spiel (#1177, Abschnitt 2, „Overdrive auf G").
 *
 * | Regel | Gruppe |
 * | --- | --- |
 * | einmal pro Kampf, ein zweiter Druck tut nichts | „Einmal pro Kampf" |
 * | Wirkung genau in der Dauer | „Genau in der Dauer" |
 * | Tempo heißt CATALYSIS, nicht globale Wartezeit | „Der Tempo-Schub" |
 * | Leiste und Schulen unberührt | „Außerhalb der Leiste" |
 * | Taste nach Position (`KeyG`), nicht nach Buchstabe | „Die Taste" |
 *
 * Gegenstück im Simulator: `sim/tests/overdrive.rs`; die Parität beider Seiten
 * hält `sim/tests/parity_overdrive.rs` an `recordings/overdrive.json` fest.
 *
 * **Ohne getragene Ausrüstung**: Jedes Tempo hier kommt aus Overdrive, eine
 * verkürzte Zeit kann also nur von ihm stammen.
 */
import { afterEach, describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, PLAYER } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';
import { hastedMs } from '../../src/core/haste.js';
import { EV_INPUT_REJECTED, EV_OVERDRIVE, NO_KEY, type FeedbackSink } from '../../src/core/events.js';
import { Session } from '../../src/core/session.js';
import { PHASE_FIGHTING } from '../../src/core/phases.js';
import {
  OVERDRIVE_ACTIVE,
  OVERDRIVE_READY,
  OVERDRIVE_SPENT,
  Overdrive,
} from '../../src/combat/overdrive.js';
import {
  NO_SLOT,
  OVERDRIVE_CODE,
  OVERDRIVE_PRESS,
  SLOT_CODES,
  SLOT_COUNT,
  positionForCode,
  slotFor,
} from '../../src/input/keys.js';
import { PressQueue } from '../../src/input/queue.js';
import { InputRouter } from '../../src/input/router.js';
import { FALLBACK_OVERDRIVE_LABEL, KeyLabels } from '../../src/input/layout.js';
import { replayRecording } from '../../src/devtools/recording.js';
import { SoundMap } from '../../src/core/sound-map.js';
import { SOUND_MANIFEST } from '../../src/audio/manifest-data.js';
import { parseManifest } from '../../src/audio/manifest.js';
import { maxOf, SOURCE_SPELLS, STAT_CATALYSIS } from '../../src/gear/sources.js';
import { overdriveHint } from '../../src/render/overlay.js';

const DAUER = Overdrive.durationMs;
const BONUS = Overdrive.catalysisBonus;

function kampf(): Combat {
  return new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
}

describe('Die Zahlen', () => {
  it('kommen aus data/player/base.json und tun etwas', () => {
    expect(DAUER).toBe(PLAYER.overdriveDurationMs);
    expect(BONUS).toBe(PLAYER.overdriveCatalysisPermille);
    // Sonst wären alle Fälle unten an einem wirkungslosen Overdrive grün.
    expect(DAUER).toBeGreaterThan(0);
    expect(BONUS).toBeGreaterThan(0);
  });

  it('das Budget der Quellen folgt der gebauten Mechanik', () => {
    // `sources.ts` führte eine eigene 300, bevor es Overdrive gab (#178).
    expect(maxOf(SOURCE_SPELLS, STAT_CATALYSIS)).toBe(BONUS);
  });
});

describe('Einmal pro Kampf', () => {
  it('der erste Druck zündet, jeder weitere nicht', () => {
    const o = new Overdrive();
    expect(o.isReady).toBe(true);
    expect(o.ignite(1000)).toBe(true);
    expect(o.ignite(5000)).toBe(false);
    expect(o.ignite(1000 + DAUER + 10_000)).toBe(false);
    expect(o.ignitedAtMs).toBe(1000);
    expect(o.endsAtMs).toBe(1000 + DAUER);
  });

  it('ein zweiter Druck im Kampf verlängert das Fenster nicht', () => {
    const c = kampf();
    expect(c.igniteOverdrive(0)).toBe(true);
    expect(c.igniteOverdrive(15_000)).toBe(false);
    expect(c.overdrive.isActive(DAUER - TICK_MS)).toBe(true);
    expect(c.overdrive.isActive(DAUER)).toBe(false);
  });

  it('erst ein neuer Kampf gibt ihn zurück', () => {
    const c = kampf();
    c.igniteOverdrive(0);
    c.reset();
    expect(c.overdrive.isReady).toBe(true);
    expect(c.igniteOverdrive(0)).toBe(true);
  });
});

describe('Genau in der Dauer', () => {
  it('wirkt im Takt des Zündens und im letzten, nicht davor und nicht im Takt des Endes', () => {
    const o = new Overdrive();
    o.ignite(1000);
    expect(o.catalysisPermille(1000 - TICK_MS)).toBe(0);
    expect(o.catalysisPermille(1000)).toBe(BONUS);
    expect(o.catalysisPermille(1000 + DAUER - TICK_MS)).toBe(BONUS);
    expect(o.catalysisPermille(1000 + DAUER)).toBe(0);
    expect(o.remainingMs(1000)).toBe(DAUER);
    expect(o.remainingMs(1000 + DAUER - TICK_MS)).toBe(TICK_MS);
    expect(o.remainingMs(1000 + DAUER)).toBe(0);
    // Genau `Dauer / Takt` Takte — gezählt, nicht gefolgert.
    let wirksam = 0;
    for (let t = 0; t < 100_000; t += TICK_MS) if (o.isActive(t)) wirksam++;
    expect(wirksam).toBe(DAUER / TICK_MS);
  });

  it('die Anzeige kennt bereit, aktiv und verbraucht', () => {
    const o = new Overdrive();
    expect(o.state(0)).toBe(OVERDRIVE_READY);
    o.ignite(1000);
    expect(o.state(1000)).toBe(OVERDRIVE_ACTIVE);
    expect(o.state(1000 + DAUER - TICK_MS)).toBe(OVERDRIVE_ACTIVE);
    expect(o.state(1000 + DAUER)).toBe(OVERDRIVE_SPENT);
  });
});

describe('Der Tempo-Schub', () => {
  it('ein Zauber im Fenster bekommt die kürzere Abklingzeit', () => {
    const c = kampf();
    const crust = alsSpellIndex(c.indexOf('crust'));
    const grund = c.spells[crust]?.cooldownMs ?? 0;
    const erwartet = hastedMs(grund, BONUS);
    expect(erwartet, 'bei dieser Abklingzeit muss Tempo sichtbar sein').toBeLessThan(grund);

    c.igniteOverdrive(0);
    c.cast(crust, 0);
    expect(c.cooldownRemainingMs(crust, 0)).toBe(erwartet);

    const ohne = kampf();
    ohne.cast(crust, 0);
    expect(ohne.cooldownRemainingMs(crust, 0)).toBe(grund);
  });

  it('im letzten Takt noch, im Takt des Endes nicht mehr', () => {
    const drin = kampf();
    const crust = alsSpellIndex(drin.indexOf('crust'));
    const grund = drin.spells[crust]?.cooldownMs ?? 0;
    drin.igniteOverdrive(0);
    drin.cast(crust, DAUER - TICK_MS);
    expect(drin.cooldownRemainingMs(crust, DAUER - TICK_MS)).toBe(hastedMs(grund, BONUS));

    const draussen = kampf();
    draussen.igniteOverdrive(0);
    draussen.cast(crust, DAUER);
    expect(draussen.cooldownRemainingMs(crust, DAUER)).toBe(grund);
  });

  it('die globale Wartezeit bleibt gleich (GAME-DESIGN 6.2)', () => {
    const mit = kampf();
    const ohne = kampf();
    const crust = alsSpellIndex(mit.indexOf('crust'));
    mit.igniteOverdrive(0);
    mit.cast(crust, 0);
    ohne.cast(crust, 0);
    expect(mit.gcd.readyAt).toBeGreaterThan(0);
    expect(mit.gcd.readyAt).toBe(ohne.gcd.readyAt);
  });

  it('das Zünden bricht eine laufende Wirkzeit ab (GAME-DESIGN 7.1)', () => {
    const c = kampf();
    const wirkzeit = alsSpellIndex(c.indexOf('test-cast'));
    c.cast(wirkzeit, 0);
    expect(c.playerCasting).toBe(wirkzeit);
    expect(c.igniteOverdrive(500)).toBe(true);
    expect(c.playerCasting).toBe(-1);
    expect(c.castsCancelled).toBe(1);

    // Ein Druck, der nichts zündet, bricht nichts ab — wie in `cast`.
    c.cast(wirkzeit, 7000);
    expect(c.igniteOverdrive(7500)).toBe(false);
    expect(c.playerCasting).toBe(wirkzeit);
    expect(c.castsCancelled).toBe(1);
  });
});

/** Ein Zuhörer, der die gemeldeten Ereignisse zählt. */
function zuhoerer(): FeedbackSink & { readonly ereignisse: number[] } {
  const ereignisse: number[] = [];
  return {
    ereignisse,
    onEvent: (event: number): void => {
      ereignisse.push(event);
    },
  };
}

function sitzung(): {
  readonly queue: PressQueue;
  readonly combat: Combat;
  readonly session: Session;
  readonly ohr: ReturnType<typeof zuhoerer>;
} {
  const queue = new PressQueue();
  const combat = kampf();
  const ohr = zuhoerer();
  const session = new Session(queue, combat, ohr);
  session.bind(0, combat.indexOf('kindle'));
  return { queue, combat, session, ohr };
}

describe('Außerhalb der Leiste', () => {
  it('der Druck ist kein Platz: 26 Plätze, die Kennung liegt dahinter', () => {
    expect(SLOT_COUNT).toBe(26);
    expect(OVERDRIVE_PRESS).toBe(SLOT_COUNT);
    expect(SLOT_CODES).not.toContain(OVERDRIVE_CODE);
  });

  it('im Kampf zündet er, ohne als Zauber zu zählen, mit eigenem Klang', () => {
    const h = sitzung();
    h.session.startFight(0);
    expect(h.session.phase).toBe(PHASE_FIGHTING);
    const belegt = h.session.boundSlotCount();
    h.queue.push(OVERDRIVE_PRESS, 1, 0, 0);
    h.session.tick(TICK_MS);
    expect(h.combat.overdriveIgnitedAtMs).toBe(TICK_MS);
    expect(h.session.acceptedCount, 'Overdrive zählte als Zauber').toBe(0);
    expect(h.session.rejectedCount).toBe(0);
    expect(h.ohr.ereignisse).toContain(EV_OVERDRIVE);
    expect(h.session.boundSlotCount()).toBe(belegt);
    // Kein Platz der Leiste wurde berührt — weder Aufleuchten noch Blinken.
    expect([...h.session.acceptedAtMs].every((t) => t < 0)).toBe(true);
    expect([...h.session.rejectedAtMs].every((t) => t < 0)).toBe(true);
    expect(h.session.overdriveAcceptedAtMs).toBe(TICK_MS);
    // Und keine Wartezeit: KINDLE geht im selben Takt.
    h.queue.push(OVERDRIVE_PRESS, 2, 0, 0);
    h.queue.push(0, 2, 0, 0);
    h.session.tick(2 * TICK_MS);
    expect(h.session.acceptedCount).toBe(1);
  });

  it('ein zweiter Druck ist eine Fehleingabe und tut sonst nichts', () => {
    const h = sitzung();
    h.session.startFight(0);
    h.queue.push(OVERDRIVE_PRESS, 1, 0, 0);
    h.session.tick(TICK_MS);
    h.ohr.ereignisse.length = 0;
    h.queue.push(OVERDRIVE_PRESS, 2, 0, 0);
    h.session.tick(2 * TICK_MS);
    expect(h.combat.overdriveIgnitedAtMs).toBe(TICK_MS);
    expect(h.session.rejectedCount).toBe(1);
    expect(h.ohr.ereignisse).toContain(EV_INPUT_REJECTED);
    expect(h.ohr.ereignisse).not.toContain(EV_OVERDRIVE);
  });

  it('in der Vorbereitung zündet er nicht', () => {
    const h = sitzung();
    h.queue.push(OVERDRIVE_PRESS, 1, 0, 0);
    h.session.tick(TICK_MS);
    expect(h.combat.overdrive.isReady).toBe(true);
    expect(h.session.rejectedCount).toBe(1);
  });

  it('der Klang hängt im Verzeichnis, nicht im Code', () => {
    const map = new SoundMap(parseManifest(SOUND_MANIFEST).entries);
    expect(map.problems).toEqual([]);
    const klaenge = map.soundsFor(EV_OVERDRIVE, NO_KEY);
    expect(klaenge).toHaveLength(1);
    expect(map.ids[klaenge[0] ?? -1]).toBe('special-overdrive');
  });

  it('eine Aufzeichnung zündet ihn über dieselbe Kennung', () => {
    const r = replayRecording(
      {
        name: 'overdrive-test',
        seed: 0,
        ticks: 3,
        presses: [
          { tick: 0, slot: OVERDRIVE_PRESS },
          { tick: 0, slot: 0 },
          { tick: 2, slot: OVERDRIVE_PRESS },
        ],
        bindings: [{ slot: 0, spellId: 'kindle' }],
      },
      { checksumPerTick: true },
    );
    expect(r.accepted).toBe(1);
    expect(r.rejected).toBe(1);
  });
});

/** Ein `keydown`/`keyup` mit Stelle UND Zeichen — der Fall unterscheidet beide. */
function taste(code: string, key: string, extra: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    code,
    key,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    repeat: false,
    timeStamp: 0,
    preventDefault: () => undefined,
    ...extra,
  } as KeyboardEvent;
}

class FensterAttrappe {
  private readonly zuhoerer = new Map<string, ((e: KeyboardEvent) => void)[]>();
  addEventListener(typ: string, fn: (e: KeyboardEvent) => void): void {
    this.zuhoerer.set(typ, [...(this.zuhoerer.get(typ) ?? []), fn]);
  }
  removeEventListener(): void {}
  sende(typ: string, e: KeyboardEvent): void {
    for (const fn of this.zuhoerer.get(typ) ?? []) fn(e);
  }
}

function router(): { readonly queue: PressQueue; readonly fenster: FensterAttrappe; readonly r: InputRouter } {
  const queue = new PressQueue();
  const fenster = new FensterAttrappe();
  const r = new InputRouter({ queue, nextTickIndex: () => 0, frameIndex: () => 0 });
  r.attach(fenster as unknown as Window);
  return { queue, fenster, r };
}

describe('Die Taste', () => {
  it('KeyG ist keine Stelle der Leiste', () => {
    expect(positionForCode(OVERDRIVE_CODE)).toBe(NO_SLOT);
    expect(slotFor(OVERDRIVE_CODE, false)).toBe(NO_SLOT);
    expect(slotFor(OVERDRIVE_CODE, true)).toBe(NO_SLOT);
  });

  it('zündet über die Stelle, nicht über den Buchstaben', () => {
    // Dvorak: an der Stelle rechts neben F steht ein „i".
    const { queue, fenster } = router();
    fenster.sende('keydown', taste('KeyG', 'i'));
    expect(queue.size).toBe(1);
    expect(queue.frontSlot()).toBe(OVERDRIVE_PRESS);

    // Und umgekehrt: ein „g" an anderer Stelle zündet nicht.
    const anders = router();
    anders.fenster.sende('keydown', taste('KeyH', 'g'));
    expect(anders.queue.size).toBe(0);
  });

  it('Halten zündet nicht nach (Design-Gesetz 1)', () => {
    const { queue, fenster, r } = router();
    fenster.sende('keydown', taste('KeyG', 'g'));
    fenster.sende('keydown', taste('KeyG', 'g', { repeat: true }));
    // Ein Browser, der `repeat` nicht setzt: die eigene Sperre greift.
    fenster.sende('keydown', taste('KeyG', 'g'));
    expect(queue.size).toBe(1);
    expect(r.suppressedRepeats).toBe(2);
    fenster.sende('keyup', taste('KeyG', 'g'));
    fenster.sende('keydown', taste('KeyG', 'g'));
    expect(queue.size).toBe(2);
  });

  it('mit Umschalt dieselbe Taste, mit Strg, Alt oder Meta keine', () => {
    const { queue, fenster, r } = router();
    fenster.sende('keydown', taste('KeyG', 'G', { shiftKey: true }));
    expect(queue.frontSlot()).toBe(OVERDRIVE_PRESS);
    fenster.sende('keyup', taste('KeyG', 'G'));
    fenster.sende('keydown', taste('KeyG', 'g', { ctrlKey: true }));
    fenster.sende('keydown', taste('KeyG', 'g', { altKey: true }));
    expect(queue.size).toBe(1);
    expect(r.modifierIgnores).toBe(2);
  });
});

describe('Die Beschriftung', () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
  afterEach(() => {
    if (original === undefined) Reflect.deleteProperty(globalThis, 'navigator');
    else Object.defineProperty(globalThis, 'navigator', original);
  });

  it('nennt, was auf der Taste steht', async () => {
    const labels = new KeyLabels();
    expect(labels.overdriveLabel).toBe(FALLBACK_OVERDRIVE_LABEL);
    Object.defineProperty(globalThis, 'navigator', {
      value: {
        keyboard: {
          getLayoutMap: () =>
            Promise.resolve({ get: (code: string) => (code === 'KeyG' ? 'i' : undefined) }),
        },
      },
      configurable: true,
      writable: true,
    });
    await labels.refresh();
    expect(labels.overdriveLabel).toBe('I');
  });

  it('der Hinweis in der Vorbereitung nennt Taste und Zahlen aus den Daten', () => {
    const text = overdriveHint('I');
    expect(text.startsWith('I — Overdrive')).toBe(true);
    expect(text).toContain(`+${String(BONUS / 10)}% CATALYSIS`);
    expect(text).toContain(`${String(DAUER / 1000)} s`);
  });
});
