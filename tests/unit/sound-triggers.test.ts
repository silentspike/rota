import { describe, expect, it } from 'vitest';
import { SOUND_MANIFEST } from '../../src/audio/manifest-data.js';
import { parseManifest, type ManifestEntry } from '../../src/audio/manifest.js';
import { AudioFeedback } from '../../src/core/audio-bridge.js';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import {
  EV_BOSS_DEFEATED,
  EV_BOSS_LOW_HEALTH,
  EV_BOSS_OVERPRESSURE,
  EV_BOSS_OVERPRESSURE_WARNING,
  EV_COUNTDOWN_TICK,
  EV_FIGHT_END,
  EV_FIGHT_START,
  EV_FIGHT_WIPE,
  EV_GCD_END,
  EV_INPUT_REJECTED,
  EV_MENU_ENTER,
  EV_RESULT_BEST_TIME,
  EV_RESULT_DEFEAT,
  EV_RESULT_VICTORY,
  EV_SPELL_CAST,
  EV_SPELL_READY,
  EVENT_NAMES,
  NO_KEY,
  type FeedbackSink,
} from '../../src/core/events.js';
import { TICK_MS } from '../../src/core/loop.js';
import { OVERPRESSURE_WARNING_MS } from '../../src/core/phases.js';
import { Session } from '../../src/core/session.js';
import { SoundMap } from '../../src/core/sound-map.js';
import { PressQueue } from '../../src/input/queue.js';

const SPELL_IDS = CALCINATION.value.spells.map((spell) => spell.id);

function realMap(): SoundMap {
  const map = new SoundMap(parseManifest(SOUND_MANIFEST).entries);
  map.bindSpells(SPELL_IDS);
  return map;
}

class Recorder implements FeedbackSink {
  readonly events: { event: number; key: number; timeMs: number }[] = [];
  onEvent(event: number, key: number, timeMs: number): void {
    this.events.push({ event, key, timeMs });
  }
  countOf(event: number): number {
    return this.events.filter((e) => e.event === event).length;
  }
  timesOf(event: number): number[] {
    return this.events.filter((e) => e.event === event).map((e) => e.timeMs);
  }
  keysOf(event: number): number[] {
    return this.events.filter((e) => e.event === event).map((e) => e.key);
  }
}

interface Harness {
  readonly session: Session;
  readonly combat: Combat;
  readonly queue: PressQueue;
  readonly rec: Recorder;
  readonly run: (ticks: number, fromMs?: number) => void;
}

function setup(): Harness {
  const queue = new PressQueue();
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
  const rec = new Recorder();
  const session = new Session(queue, combat, rec);
  session.bind(0, combat.indexOf('kindle'));
  session.bind(1, combat.indexOf('smoulder'));
  session.bind(2, combat.indexOf('pyre'));
  return {
    session,
    combat,
    queue,
    rec,
    run: (ticks, fromMs = 0) => {
      for (let i = 0; i < ticks; i++) session.tick(fromMs + i * TICK_MS);
    },
  };
}

describe('Zuordnung aus dem Verzeichnis (AC4)', () => {
  it('liest das echte Verzeichnis ohne Beanstandung', () => {
    expect(realMap().problems).toEqual([]);
    expect(parseManifest(SOUND_MANIFEST).problems).toEqual([]);
  });

  /**
   * Zauber, die **noch keinen Klang haben** — ausdrücklich und namentlich.
   *
   * Die Stummheit steht damit schwarz auf weiß im Quelltext statt in einer
   * übersprungenen Prüfung. Der Fall unten wirkt in **beide** Richtungen: Ein
   * Zauber ohne Klang, der hier fehlt, macht ihn rot — und ein Zauber, der hier
   * steht und einen Klang hat, ebenso.
   *
   * **Ist die Liste leer, fällt sie weg.** Genau das leisten #198 und #204.
   */
  // `test-cast` gehört NICHT zum Spiel (Issue #304): Er ist der Prüf-Zauber
  // zur Wirkzeit, liegt über der Stufengrenze und ist für den Spieler weder
  // sichtbar noch belegbar. Ein Klang für einen Zauber, den niemand wirken
  // kann, wäre eine Datei ohne Fall — und die Liste hier ist genau der Ort,
  // an dem das schwarz auf weiß steht statt in einer übersprungenen Prüfung.
  // `bellows` und `searing` sind neu (Issue #268) und SPIELBAR — anders als
  // `test-cast`. Sie stehen hier aus dem Grund, den der Kopf nennt: Sie haben
  // noch keinen Klang, und das gehoert namentlich in den Quelltext statt in
  // eine uebersprungene Pruefung.
  //
  // Klaenge entstehen ueber Suno (docs/suno-sfx.md) und werden vom
  // Auftraggeber abgenommen — sie in einem Zauber-PR mitzubauen ginge nicht.
  // DIE LISTE HAT ZWEI ZWECKE, und sie sind verschieden lange gültig:
  //
  //   DAUERHAFT   die beiden Prüf-Zauber über der Stufengrenze
  //               (`unlockLevel: 101`), die kein Spieler je auf die Leiste
  //               legt: `test-cast` aus #304 und `test-buff` aus #420.
  //
  //   VORLÄUFIG   `harden` und `sepsis` aus #523. Beide sind gebaut und
  //               spielbar, ihre Klänge fehlen noch.
  //
  // **Der Klang entsteht AUSSERHALB dieses Zweigs** — über Suno
  // (`docs/suno-sfx.md`), mit anschliessender Abnahme durch den Auftraggeber.
  // Deshalb steht der Bedarf hier namentlich statt in einer übersprungenen
  // Prüfung.
  //
  // Die Begründung lautet bewusst NICHT „lässt sich in keiner Sitzung
  // erzeugen". Genau dieser Satz stand hier einmal und war nacheinander wahr,
  // falsch (in einer Nacht entstanden sieben Klänge) und wieder wahr (Suno
  // verlangt seither einen Captcha-Token aus einer eingeloggten Seite) — ohne
  // dass sich ein Wort geändert hätte. Wer ihn liest, zieht den richtigen
  // Schluss aus einer überholten Begründung.
  //
  // **Ein vorläufiger Eintrag ist kein Schulden-Vermerk, sondern eine Prüfung,
  // die auf ihren Anlass wartet:** Die Gegenrichtung unten holt ihn von selbst
  // wieder heraus, sobald der Klang da ist.
  const STUMM = new Set(['test-cast', 'test-buff', 'harden', 'sepsis']);

  it('findet für jeden Zauber genau einen Klang', () => {
    const map = realMap();
    for (let i = 0; i < SPELL_IDS.length; i++) {
      const id = SPELL_IDS[i] ?? '';
      const sounds = map.soundsFor(EV_SPELL_CAST, i);
      if (STUMM.has(id)) {
        // **Die zweite Richtung ist die wichtigere.** Ohne sie verrottet die
        // Liste: Sobald der Klang kommt, bliebe der Eintrag stehen, und beim
        // nächsten stummen Zauber fiele es niemandem auf, weil „da stehen ja
        // schon welche drin".
        expect(
          sounds.length,
          `${id} steht in STUMM, hat aber einen Klang — nimm ihn aus der Liste`,
        ).toBe(0);
        continue;
      }
      expect(sounds.length, `${id} hat keinen Klang und steht nicht in STUMM`).toBe(1);
    }

    // **Auf den Namen wird ausdrücklich NICHT geprüft.** Bis E1-9 stand hier
    // die Regel `spell-` + Kennung; sie ist entfallen, weil die Zuordnung ins
    // Verzeichnis gehört und nicht in eine Namenskonvention (siehe
    // `src/core/loadout.ts`). QUENCH ist der erste Fall, in dem beides
    // auseinanderfällt: Sein Klang heißt `special-interrupt`, weil
    // GAME-DESIGN 17.1 Unterbrechen unter den sieben SONDERZAUBERN führt und
    // nicht unter den 60 Zaubern.
    //
    // Ein Test, der den Namen weiter erzwingt, hielte eine abgeschaffte Regel
    // am Leben — und zwar an der Stelle, an der sie zum ersten Mal falsch wird.
    const quench = SPELL_IDS.indexOf('quench');
    expect(quench).toBeGreaterThanOrEqual(0);
    const quenchSounds = map.soundsFor(EV_SPELL_CAST, quench);
    expect(map.ids[quenchSounds[0] as number]).toBe('special-interrupt');
  });

  it('nimmt einen zusätzlichen Eintrag ohne Code-Änderung auf', () => {
    // Der Kern von AC4: Ein neuer Zauber ist ein Datensatz, kein Code
    // (Leitplanke 6). Hier kommt ein zweiter Klang an denselben Zauber — im
    // Code steht dafür keine Zeile.
    const extra: ManifestEntry = {
      id: 'spell-testflame',
      group: 'spells',
      purpose: 'Prüfeintrag',
      kind: 'oneshot',
      bus: 'sfx',
      file: 'audio/spell-testflame.ogg',
      trigger: { on: 'spell.cast', key: 'kindle' },
    };
    const map = new SoundMap([...parseManifest(SOUND_MANIFEST).entries, extra]);
    map.bindSpells(SPELL_IDS);

    const kindle = SPELL_IDS.indexOf('kindle');
    const ids = map.soundsFor(EV_SPELL_CAST, kindle).map((s) => map.ids[s]);
    expect(ids).toEqual(['spell-kindle', 'spell-testflame']);
    expect(map.problems).toEqual([]);
  });

  it('lässt einen Eintrag ohne Betreff für jeden Zauber gelten', () => {
    // „Zauber wieder bereit" ist **ein** Klang für alle sechzig Zauber. Ohne
    // diese Regel bräuchte er sechzig Einträge — oder bliebe stumm, was er im
    // ersten Bau tatsächlich war.
    const map = realMap();
    const ready = map.indexOf('feedback-spell-ready');
    expect(ready).toBeGreaterThanOrEqual(0);
    for (let i = 0; i < SPELL_IDS.length; i++) {
      expect(map.soundsFor(EV_SPELL_READY, i)).toContain(ready);
    }
  });

  it('meldet ein Ereignis, das es nicht gibt', () => {
    const entry: ManifestEntry = {
      id: 'x',
      group: 'spells',
      purpose: 'p',
      kind: 'oneshot',
      bus: 'sfx',
      file: 'audio/x.ogg',
      trigger: { on: 'spell.explodes' },
    };
    const map = new SoundMap([entry]);
    expect(map.problems).toEqual(['Klang "x": unbekanntes Ereignis "spell.explodes".']);
    expect(map.ids).toEqual([]);
  });

  it('meldet einen Betreff an einem Ereignis, das keinen kennt', () => {
    const entry: ManifestEntry = {
      id: 'x',
      group: 'state',
      purpose: 'p',
      kind: 'oneshot',
      bus: 'sfx',
      file: 'audio/x.ogg',
      trigger: { on: 'fight.start', key: 'kindle' },
    };
    expect(new SoundMap([entry]).problems).toEqual([
      'Klang "x": Ereignis "fight.start" kennt keinen Betreff.',
    ]);
  });

  it('meldet einen Zauber, den es nicht gibt', () => {
    const entry: ManifestEntry = {
      id: 'spell-ghost',
      group: 'spells',
      purpose: 'p',
      kind: 'oneshot',
      bus: 'sfx',
      file: 'audio/x.ogg',
      trigger: { on: 'spell.cast', key: 'ghost' },
    };
    const map = new SoundMap([entry]);
    map.bindSpells(SPELL_IDS);
    expect(map.problems).toEqual(['Klang "spell-ghost": Zauber "ghost" gibt es nicht.']);
  });

  it('beanstandet Schicht-Angaben an einem einzelnen Klang', () => {
    const entry: ManifestEntry = {
      id: 'x',
      group: 'state',
      purpose: 'p',
      kind: 'oneshot',
      bus: 'sfx',
      file: 'audio/x.ogg',
      targetSeconds: 1,
      prompt: 'p',
      trigger: { on: 'fight.start', off: 'fight.end' },
    };
    const problems = parseManifest({
      schema: 1,
      coverage: { groups: {}, placeholders: [] },
      sounds: [entry],
    }).problems;
    expect(problems).toContain('Klang "x": Schicht-Angaben an einem Klang, der keine Schicht ist.');
  });

  it('kennt zu jedem Ereignis einen eindeutigen Namen', () => {
    expect(new Set(EVENT_NAMES).size).toBe(EVENT_NAMES.length);
  });
});

describe('Musik-Schichten (AC3)', () => {
  const map = realMap();
  const idsOf = (list: readonly number[]): string[] =>
    list.map((i) => map.layers[i]?.id ?? '?').sort();

  it('führt fünf Schichten, davon eine von Anfang an hörbar', () => {
    expect(map.layers.map((l) => l.id).sort()).toEqual([
      'music-bed-placeholder',
      'music-final-push',
      'music-menu',
      'music-overpressure',
      'music-phase-change',
    ]);
    expect(map.layers.filter((l) => l.base).map((l) => l.id)).toEqual(['music-menu']);
  });

  it('tauscht Menü- gegen Kampf-Teppich am Kampfbeginn', () => {
    expect(idsOf(map.layersOn(EV_FIGHT_START))).toEqual(['music-bed-placeholder']);
    expect(idsOf(map.layersOff(EV_FIGHT_START))).toEqual(['music-menu']);
  });

  it('schaltet den Endspurt unter 20 % zu (GAME-DESIGN 10.1)', () => {
    expect(idsOf(map.layersOn(EV_BOSS_LOW_HEALTH))).toEqual(['music-final-push']);
  });

  it('schaltet den Überdruck mit der Warnung zu, und zwar sofort', () => {
    expect(idsOf(map.layersOn(EV_BOSS_OVERPRESSURE_WARNING))).toEqual(['music-overpressure']);
    const layer = map.layers.find((l) => l.id === 'music-overpressure');
    expect(layer?.immediate).toBe(true);
  });

  it('nimmt am Kampfende alle Kampf-Schichten zurück und den Menü-Teppich auf', () => {
    expect(idsOf(map.layersOff(EV_FIGHT_END))).toEqual([
      'music-bed-placeholder',
      'music-final-push',
      'music-overpressure',
      'music-phase-change',
    ]);
    expect(idsOf(map.layersOn(EV_MENU_ENTER))).toEqual(['music-menu']);
  });
});

describe('Was der Kampf meldet (AC1, AC2)', () => {
  it('schlägt den Countdown je Sekunde und beginnt danach den Kampf', () => {
    const h = setup();
    h.session.beginCountdown(0);
    h.run(400);

    expect(h.rec.timesOf(EV_COUNTDOWN_TICK)).toEqual([0, 1000, 2000]);
    expect(h.rec.timesOf(EV_FIGHT_START)).toEqual([3000]);
  });

  it('meldet jeden angenommenen Druck mit dem Zauber, jeden verworfenen ohne', () => {
    const h = setup();
    h.session.startFight(0);
    h.queue.push(0, 0, 0, 0); // KINDLE
    h.queue.push(1, 1, 0, 0); // SMOULDER, zu früh: Wartezeit läuft
    h.run(3);

    expect(h.rec.keysOf(EV_SPELL_CAST)).toEqual([h.combat.indexOf('kindle')]);
    expect(h.rec.countOf(EV_INPUT_REJECTED)).toBe(1);
  });

  it('meldet das Ende der Wartezeit genau einmal je Zyklus', () => {
    const h = setup();
    h.session.startFight(0);
    h.queue.push(0, 0, 0, 0);
    h.run(200);

    // Wirken bei 0, Wartezeit bis 1500 — ein einziger Übergang.
    expect(h.rec.timesOf(EV_GCD_END)).toEqual([1500]);
  });

  it('meldet einen Zauber, der wieder bereit ist, nur an der Flanke', () => {
    const h = setup();
    h.session.startFight(0);
    h.combat.setResource(20);
    h.queue.push(2, 0, 0, 0); // PYRE, 5 s Abklingzeit
    h.run(700);

    const pyre = h.combat.indexOf('pyre');
    const ready = h.rec.events.filter((e) => e.event === EV_SPELL_READY && e.key === pyre);
    expect(ready.map((e) => e.timeMs)).toEqual([5000]);
  });

  it('meldet keinen bereiten Zauber im ersten Takt des Kampfes', () => {
    const h = setup();
    h.session.startFight(0);
    h.run(5);
    expect(h.rec.countOf(EV_SPELL_READY)).toBe(0);
  });

  it('meldet den Endspurt beim Unterschreiten von 20 %', () => {
    const h = setup();
    h.session.startFight(0);
    // Ein Prozent über der Schwelle: noch nichts.
    h.combat.setBossHpRemaining(h.combat.boss.hp * 0.21);
    h.run(2);
    expect(h.rec.countOf(EV_BOSS_LOW_HEALTH)).toBe(0);

    h.combat.setBossHpRemaining(h.combat.boss.hp * 0.19);
    h.run(2, 100);
    expect(h.rec.countOf(EV_BOSS_LOW_HEALTH)).toBe(1);

    // Und danach nicht wieder — es ist ein Übergang, kein Zustand.
    h.run(50, 200);
    expect(h.rec.countOf(EV_BOSS_LOW_HEALTH)).toBe(1);
  });

  it('warnt vor dem Überdruck zur selben Zeit wie der Ring', () => {
    const h = setup();
    h.session.startFight(0);
    const warnAt = h.combat.boss.overpressureMs - OVERPRESSURE_WARNING_MS;
    h.run(2, warnAt - 100);
    expect(h.rec.countOf(EV_BOSS_OVERPRESSURE_WARNING)).toBe(0);

    h.run(2, warnAt);
    expect(h.rec.timesOf(EV_BOSS_OVERPRESSURE_WARNING)).toEqual([warnAt]);
  });

  it('meldet beim Sieg Boss, Auswertung, Bestzeit und den Wechsel des Teppichs', () => {
    const h = setup();
    h.session.startFight(0);
    h.combat.setBossHpRemaining(1);
    h.queue.push(0, 0, 0, 0);
    h.run(5);

    const order = h.rec.events.map((e) => EVENT_NAMES[e.event]);
    expect(order).toContain('boss.defeated');
    expect(order.slice(order.indexOf('boss.defeated'))).toEqual([
      'boss.defeated',
      'result.victory',
      'result.best-time',
      'fight.end',
      'menu.enter',
    ]);
  });

  it('meldet die Bestzeit nur, wenn sie besser ist', () => {
    const win = (h: Harness, atMs: number): void => {
      h.session.startFight(0);
      h.combat.setBossHpRemaining(1);
      h.queue.push(0, atMs / TICK_MS, 0, 0);
      h.run(atMs / TICK_MS + 5);
    };
    const h = setup();
    win(h, 1000);
    expect(h.rec.countOf(EV_RESULT_BEST_TIME)).toBe(1);

    h.session.reset();
    win(h, 2000); // langsamer
    expect(h.rec.countOf(EV_RESULT_BEST_TIME)).toBe(1);

    h.session.reset();
    win(h, 500); // schneller
    expect(h.rec.countOf(EV_RESULT_BEST_TIME)).toBe(2);
  });

  it('meldet den Überdruck als Niederlage', () => {
    const h = setup();
    h.session.startFight(0);
    h.run(3, h.combat.boss.overpressureMs);

    expect(h.rec.countOf(EV_BOSS_OVERPRESSURE)).toBe(1);
    expect(h.rec.countOf(EV_RESULT_DEFEAT)).toBe(1);
    expect(h.rec.countOf(EV_BOSS_DEFEATED)).toBe(0);
  });

  it('meldet den Abbruch mit eigenem Klang, nicht als Niederlage', () => {
    // GAME-DESIGN 10: keine Strafe. 12.8: der häufigste Übergang im Spiel.
    const h = setup();
    h.session.startFight(0);
    h.run(10);
    h.session.wipe(100);

    expect(h.rec.countOf(EV_FIGHT_WIPE)).toBe(1);
    expect(h.rec.countOf(EV_RESULT_DEFEAT)).toBe(0);
    expect(h.rec.countOf(EV_RESULT_VICTORY)).toBe(0);
  });

  it('kehrt beim Zurücksetzen in den Menü-Teppich zurück', () => {
    const h = setup();
    h.session.startFight(0);
    h.session.reset();
    expect(h.rec.countOf(EV_MENU_ENTER)).toBe(1);
  });
});

describe('Die Brücke zur Klang-Ausgabe (AC2, AC5)', () => {
  it('zählt Anforderungen auch ohne Klang-Kontext, ohne zu scheitern', () => {
    // Ein stummes Spiel rechnet identisch (GAME-DESIGN 18.5): Im Prüflauf gibt
    // es keinen AudioContext, und genau das darf nichts kosten.
    const audio = new AudioFeedback();
    audio.bindSpells(SPELL_IDS);
    expect(audio.map.problems).toEqual([]);

    const kindle = SPELL_IDS.indexOf('kindle');
    audio.onEvent(EV_SPELL_CAST, kindle, 0);
    audio.onEvent(EV_INPUT_REJECTED, NO_KEY, 10);
    audio.onEvent(EV_FIGHT_START, NO_KEY, 20);

    expect(audio.requestCount).toBe(3);
    expect(audio.isReady).toBe(false);
    const kindleSound = audio.map.indexOf('spell-kindle');
    expect(audio.requestsBySound[kindleSound]).toBe(1);
  });

  it('bleibt bei einem Ereignis ohne Klang still, statt zu scheitern', () => {
    const audio = new AudioFeedback();
    audio.bindSpells(SPELL_IDS);
    const before = audio.requestCount;
    // `menu.enter` hat nur eine Schicht, keinen Einzelklang.
    audio.onEvent(EV_MENU_ENTER, NO_KEY, 0);
    expect(audio.requestCount).toBe(before);
  });

  it('erzeugt beim Vertonen keine Objekte je Ereignis', () => {
    // Der Nachweis für AC5 auf der Seite des Kampfes: Die Suche nach dem Klang
    // ist ein Feldzugriff. Was der Browser je Wiedergabe verlangt, hängt an der
    // Zahl der Klänge — nicht an der Bildzahl (VERIFICATION § 3.5).
    const audio = new AudioFeedback();
    audio.bindSpells(SPELL_IDS);
    const sounds = audio.map.soundsFor(EV_SPELL_CAST, 0);
    expect(audio.map.soundsFor(EV_SPELL_CAST, 0)).toBe(sounds);
    expect(audio.map.soundsFor(EV_FIGHT_START, NO_KEY)).toBe(
      audio.map.soundsFor(EV_FIGHT_START, NO_KEY),
    );
  });
});
