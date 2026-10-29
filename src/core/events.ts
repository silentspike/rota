/**
 * Die Kampf-Ereignisse, auf die Klang und Musik hören (GAME-DESIGN 17.1).
 *
 * **Das Vokabular steht im Code, die Zuordnung in den Daten.** Welche Ereignisse
 * es überhaupt gibt, weiß nur der Kampf — er ist es, der sie auslöst. Welcher
 * Klang daran hängt, steht dagegen in `data/sound-manifest.json` und ist damit
 * ohne Code-Änderung zu ändern (Leitplanke 6: ein neuer Zauber ist ein Datensatz).
 *
 * Ereignisse sind **Zahlen, keine Zeichenketten**: Sie werden im Logik-Takt
 * ausgelöst, und dort entsteht nichts (GAME-DESIGN 18.2). Die Namen dienen dem
 * Verzeichnis und den Werkzeugen.
 *
 * Ereignisse, die es in Schritt 1 noch nicht geben kann — Boss-Zauber,
 * Phasenwechsel, Spieler-Schaden —, stehen **trotzdem hier**: Das Verzeichnis
 * führt ihre Klänge bereits, und das Issue verlangt ausdrücklich, die Auslöser
 * vorzusehen statt sie zu belegen.
 */

/** Kein Betreff — das Ereignis betrifft keinen einzelnen Zauber. */
export const NO_KEY = -1;

export const EV_SPELL_CAST = 0;
export const EV_SPELL_READY = 1;
export const EV_INPUT_REJECTED = 2;
export const EV_GCD_END = 3;
export const EV_SYNERGY_READY = 4;
export const EV_BOSS_HIT = 5;
export const EV_BOSS_CAST_START = 6;
export const EV_BOSS_CAST_SUCCESS = 7;
export const EV_BOSS_PHASE_CHANGE = 8;
export const EV_BOSS_REACTION_DESTABILISE = 9;
export const EV_BOSS_LOW_HEALTH = 10;
export const EV_BOSS_OVERPRESSURE_WARNING = 11;
export const EV_BOSS_OVERPRESSURE = 12;
export const EV_BOSS_DEFEATED = 13;
export const EV_COUNTDOWN_TICK = 14;
export const EV_FIGHT_START = 15;
export const EV_FIGHT_WIPE = 16;
export const EV_FIGHT_END = 17;
export const EV_PLAYER_LOW_HEALTH = 18;
export const EV_PLAYER_DEATH = 19;
export const EV_RESULT_VICTORY = 20;
export const EV_RESULT_DEFEAT = 21;
export const EV_RESULT_BEST_TIME = 22;
export const EV_MENU_ENTER = 23;

/**
 * Die Boss-Ankündigungen (GAME-DESIGN 11.5, Kanal 3; Issue #797) — **angehängt**,
 * damit keine bestehende Nummer wandert.
 *
 * „Eigener Klang je Ereignisart": je Warnart ein Ereignis, gesendet, wenn eine
 * Fähigkeit der Stufe 2 oder 3 in ihre letzten Sekunden eintritt. Dazu der
 * gesprochene Countdown von Stufe 3, je Sekunde ein eigenes Ereignis — fünf
 * Wörter sind fünf Aufnahmen.
 *
 * **Die Klänge dazu gibt es noch nicht** (Stand #797). Das Verzeichnis führt sie
 * als vorläufige Einträge mit Datei und Prompt, aber ohne Auslöser — ein Auslöser
 * ohne Datei verlangte einen Klang, den es nicht gibt. Bis dahin laufen die
 * Ereignisse ins Leere; danach ist der Klang ein Datensatz und keine
 * Code-Änderung (Leitplanke 6).
 */
export const EV_BOSS_WARNING_DEBUFF = 24;
export const EV_BOSS_WARNING_DAMAGE = 25;
export const EV_BOSS_WARNING_LETHAL = 26;
export const EV_BOSS_WARNING_OPPORTUNITY = 27;
export const EV_BOSS_COUNTDOWN_1 = 28;
export const EV_BOSS_COUNTDOWN_2 = 29;
export const EV_BOSS_COUNTDOWN_3 = 30;
export const EV_BOSS_COUNTDOWN_4 = 31;
export const EV_BOSS_COUNTDOWN_5 = 32;

/**
 * Overdrive gezündet (#1177, Abschnitt 2) — **angehängt**, damit keine
 * bestehende Nummer wandert. Kein Zauber-Ereignis: Overdrive liegt auf keinem
 * Platz und hat keinen Index unter den Zaubern. Der Klang dazu ist
 * `special-overdrive` im Verzeichnis.
 */
export const EV_OVERDRIVE = 33;

export const EVENT_COUNT = 34;

/**
 * Warnart → Ereignis. Die Reihenfolge ist die der Arten in
 * `src/combat/boss/announcements.ts` (`KIND_DEBUFF` = 0 bis `KIND_OPPORTUNITY` = 3).
 */
export const EV_BOSS_WARNING_BY_KIND: readonly number[] = [
  EV_BOSS_WARNING_DEBUFF,
  EV_BOSS_WARNING_DAMAGE,
  EV_BOSS_WARNING_LETHAL,
  EV_BOSS_WARNING_OPPORTUNITY,
];

/** Countdown-Sekunde → Ereignis; Platz 0 ist leer, es gibt kein „null". */
export const EV_BOSS_COUNTDOWN_BY_SECOND: readonly number[] = [
  -1,
  EV_BOSS_COUNTDOWN_1,
  EV_BOSS_COUNTDOWN_2,
  EV_BOSS_COUNTDOWN_3,
  EV_BOSS_COUNTDOWN_4,
  EV_BOSS_COUNTDOWN_5,
];

/**
 * Die Namen, unter denen das Verzeichnis die Ereignisse anspricht.
 *
 * **Diese Zeichenketten sind Schnittstelle**: Wer sie ändert, macht jeden
 * Eintrag im Verzeichnis ungültig, der darauf zeigt.
 */
export const EVENT_NAMES: readonly string[] = [
  'spell.cast',
  'spell.ready',
  'input.rejected',
  'gcd.end',
  'synergy.ready',
  'boss.hit',
  'boss.cast.start',
  'boss.cast.success',
  'boss.phase.change',
  'boss.reaction.destabilise',
  'boss.low-health',
  'boss.overpressure.warning',
  'boss.overpressure',
  'boss.defeated',
  'fight.countdown.tick',
  'fight.start',
  'fight.wipe',
  'fight.end',
  'player.low-health',
  'player.death',
  'result.victory',
  'result.defeat',
  'result.best-time',
  'menu.enter',
  'boss.warning.debuff',
  'boss.warning.damage',
  'boss.warning.lethal',
  'boss.warning.opportunity',
  'boss.countdown.1',
  'boss.countdown.2',
  'boss.countdown.3',
  'boss.countdown.4',
  'boss.countdown.5',
  'overdrive.ignite',
];

/**
 * Ereignisse, die einen einzelnen Zauber betreffen.
 *
 * Nur bei ihnen darf ein Eintrag im Verzeichnis einen Betreff (`key`) nennen —
 * bei allen anderen wäre er sinnlos und deshalb ein Fehler im Verzeichnis.
 */
export function isKeyedEvent(event: number): boolean {
  return event === EV_SPELL_CAST || event === EV_SPELL_READY;
}

/** Name → Kennnummer, oder -1. Läuft nur beim Einlesen des Verzeichnisses. */
export function eventIndex(name: string): number {
  return EVENT_NAMES.indexOf(name);
}

/**
 * Wohin die Rückmeldung geht, die nicht im Bild stattfindet — derzeit der Klang.
 *
 * Der Kampf kennt weder Puffer noch Knoten; er sagt nur, **was** geschehen ist.
 * Ein stummes Spiel rechnet identisch (GAME-DESIGN 18.5).
 */
export interface FeedbackSink {
  /**
   * @param key Kennnummer des betroffenen Zaubers, sonst `NO_KEY`.
   * @param timeMs Zeit auf der durchlaufenden Uhr.
   */
  onEvent(event: number, key: number, timeMs: number): void;
}
