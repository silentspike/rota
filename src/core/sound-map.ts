/**
 * Welcher Klang an welchem Kampf-Ereignis hängt — **gelesen, nicht programmiert**.
 *
 * Die Zuordnung steht in `data/sound-manifest.json` (Feld `trigger`) und wird
 * hier einmal beim Start in Nachschlagetabellen übersetzt. Ein neuer Zauber
 * braucht deshalb keinen Code, sondern einen Eintrag im Verzeichnis
 * (Leitplanke 6, GAME-DESIGN 18.3).
 *
 * **Warum die Tabelle im Kampf liegt und nicht in der Klang-Schicht:** Welches
 * Ereignis welchen Klang auslöst, ist eine Entscheidung des Kampfsystems. Die
 * Klang-Schicht aus Epic E8 bleibt eine Bibliothek — sie weiß nichts von Kämpfen.
 *
 * **Zahlen statt Zeichenketten:** Im Logik-Takt wird nur in vorab gebauten
 * Zahlen-Listen nachgeschlagen. Es entsteht dort nichts (GAME-DESIGN 18.2).
 */
import type { ManifestEntry } from '../audio/manifest.js';
import { EVENT_COUNT, eventIndex, isKeyedEvent } from './events.js';

/** Eine Musik-Schicht, wie das Verzeichnis sie beschreibt. */
export interface LayerBinding {
  readonly id: string;
  /** Von Anfang an hörbar. */
  readonly base: boolean;
  /** Sofort blenden statt an der nächsten Taktgrenze. */
  readonly immediate: boolean;
}

const NO_SOUNDS: readonly number[] = [];

/** Ein Klang, der auf einen bestimmten Zauber wartet. */
interface KeyedBinding {
  readonly key: string;
  readonly sound: number;
}

export class SoundMap {
  /** Klang-Kennungen, nach Kennnummer. Die Nummer ist der Schlüssel überall sonst. */
  readonly ids: string[] = [];

  /** Was am Verzeichnis nicht stimmte. Leer heißt: alles zugeordnet. */
  readonly problems: string[] = [];

  /** Einträge mit Auslöser, nach Klang-Kennnummer — für den Abdeckungs-Bericht. */
  readonly boundEvents: number[] = [];

  private readonly keyless: number[][] = [];
  private readonly keyedRaw: KeyedBinding[][] = [];
  /** Ereignis → Zauber-Kennnummer → Klänge. Steht erst nach `bindSpells`. */
  private keyed: (readonly number[])[][] = [];

  readonly layers: LayerBinding[] = [];
  /** Schicht-Nummer → Klang-Kennnummer. Für die Zählung im Bericht. */
  readonly soundOfLayer: number[] = [];
  private readonly enableOn: number[][] = [];
  private readonly disableOn: number[][] = [];

  constructor(entries: readonly ManifestEntry[]) {
    for (let i = 0; i < EVENT_COUNT; i++) {
      this.keyless.push([]);
      this.keyedRaw.push([]);
      this.keyed.push([]);
      this.enableOn.push([]);
      this.disableOn.push([]);
    }

    for (const entry of entries) {
      const trigger = entry.trigger;
      if (trigger === undefined) continue;

      const where = `Klang "${entry.id}"`;
      const on = trigger.on === undefined ? -1 : eventIndex(trigger.on);
      if (trigger.on !== undefined && on < 0) {
        this.problems.push(`${where}: unbekanntes Ereignis "${trigger.on}".`);
        continue;
      }
      if (trigger.key !== undefined && on >= 0 && !isKeyedEvent(on)) {
        this.problems.push(`${where}: Ereignis "${trigger.on}" kennt keinen Betreff.`);
        continue;
      }

      const sound = this.ids.length;
      this.ids.push(entry.id);
      this.boundEvents.push(on);

      if (trigger.layer === true) {
        const layer = this.layers.length;
        this.soundOfLayer.push(sound);
        this.layers.push({
          id: entry.id,
          base: trigger.base === true,
          immediate: trigger.immediate === true,
        });
        if (on >= 0) this.enableOn[on]?.push(layer);
        if (trigger.off !== undefined) {
          const off = eventIndex(trigger.off);
          if (off < 0) this.problems.push(`${where}: unbekanntes Abschalt-Ereignis.`);
          else this.disableOn[off]?.push(layer);
        }
        continue;
      }

      if (trigger.key !== undefined) this.keyedRaw[on]?.push({ key: trigger.key, sound });
      else this.keyless[on]?.push(sound);
    }
  }

  /**
   * Die Zauber-Kennungen nachreichen, sobald der Kampf steht.
   *
   * Erst hier wird aus „`spell.cast` mit Betreff kindle" eine Zeile, die im Takt
   * mit einer Kennnummer nachgeschlagen werden kann.
   */
  bindSpells(spellIds: readonly string[]): void {
    this.keyed = [];
    for (let event = 0; event < EVENT_COUNT; event++) {
      const perSpell: (readonly number[])[] = [];
      const bindings = this.keyedRaw[event] ?? [];
      // **Ein Eintrag ohne Betreff gilt für jeden Zauber.** So kommt „Zauber
      // wieder bereit" mit einem einzigen Klang aus, statt sechzigmal im
      // Verzeichnis zu stehen. Ohne diese Zusammenführung bliebe er stumm —
      // im Bild aufgefallen, nicht im Prüflauf.
      const forAll = this.keyless[event] ?? [];
      for (const spellId of spellIds) {
        const sounds = [
          ...bindings.filter((b) => b.key === spellId).map((b) => b.sound),
          ...forAll,
        ];
        perSpell.push(sounds.length > 0 ? sounds : NO_SOUNDS);
      }
      for (const binding of bindings) {
        if (!spellIds.includes(binding.key)) {
          this.problems.push(
            `Klang "${this.ids[binding.sound] ?? '?'}": Zauber "${binding.key}" gibt es nicht.`,
          );
        }
      }
      this.keyed.push(perSpell);
    }
  }

  /**
   * Welche Klänge dieses Ereignis auslöst.
   *
   * Der heiße Pfad: nur zwei Feldzugriffe, kein `Map`, keine Zeichenkette.
   */
  soundsFor(event: number, key: number): readonly number[] {
    if (key >= 0) return this.keyed[event]?.[key] ?? NO_SOUNDS;
    return this.keyless[event] ?? NO_SOUNDS;
  }

  layersOn(event: number): readonly number[] {
    return this.enableOn[event] ?? NO_SOUNDS;
  }

  layersOff(event: number): readonly number[] {
    return this.disableOn[event] ?? NO_SOUNDS;
  }

  /** Auf welches Ereignis dieser Klang wartet, oder -1. */
  eventOf(sound: number): number {
    return this.boundEvents[sound] ?? -1;
  }

  indexOf(soundId: string): number {
    return this.ids.indexOf(soundId);
  }
}
