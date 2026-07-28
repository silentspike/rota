/**
 * Abdeckungs-Bericht für Klang und Musik (GAME-DESIGN 19.1).
 *
 * Beantwortet die zwei Fragen, die man einer Klang-Anbindung stellen muss:
 *
 * 1. **Läuft eine Anforderung ins Leere?** — ein Klang wird verlangt, den es
 *    nicht gibt. Das wäre ein Fehler und heißt hier `klangFehlend`.
 * 2. **Bleibt ein Eintrag stumm?** — ein Klang liegt im Verzeichnis, aber kein
 *    Ereignis holt ihn. Das ist **kein** Fehler, solange der Grund benannt ist:
 *    Sein Ereignis wird erst von einem späteren Epic gebaut.
 *
 * Der Bericht gehört zu den Werkzeugen und fällt in der Auslieferung weg (19.3).
 */
import type { App } from '../core/app.js';
import { EVENT_NAMES } from '../core/events.js';
import { SOUND_MANIFEST } from '../audio/manifest-data.js';

/**
 * Warum ein Ereignis in Schritt 1 nicht ausgelöst werden **kann**.
 *
 * Steht hier und nicht im Verzeichnis: Es ist eine Aussage über den Stand des
 * Spiels, nicht über den Klang. Fällt die Zeile weg, sobald das Epic da ist.
 */
const NOT_YET: Readonly<Record<string, string>> = {
  'synergy.ready': 'E3 — Synergien brauchen eine zweite Schule (GAME-DESIGN 5)',
  'boss.cast.start': 'E4 — der Testboss wirkt keine Zauber',
  'boss.cast.success': 'E4 — der Testboss wirkt keine Zauber',
  'boss.phase.change': 'E4 — der Testboss hat eine Phase',
  'boss.reaction.destabilise': 'E4 — Mechanik-Baustein 6 fehlt',
  'player.low-health': 'E4 — der Testboss teilt keinen Schaden aus',
  'player.death': 'E4 — der Testboss teilt keinen Schaden aus',
};

export interface SoundReport {
  /** Angeforderte Klänge, die es nicht gibt. **Muss 0 sein.** */
  readonly klangFehlend: number;
  /** Wie oft der Kampf einen Klang verlangt hat. */
  readonly anforderungen: number;
  /** Wie oft wirklich abgespielt wurde. */
  readonly wiedergaben: number;
  readonly geladen: boolean;
  /** Zustand des Klang-Kontexts. Vor der ersten Nutzergeste `suspended`. */
  readonly kontext: string;
  /** Wie viele Stimmen gerade klingen. */
  readonly stimmen: number;
  /** Zuletzt verwendete Abspielgeschwindigkeiten — die Tonhöhen-Streuung. */
  readonly streuung: number[];
  /** Klänge im Verzeichnis mit einem Auslöser. */
  readonly zugeordnet: number;
  /** Davon in diesem Lauf mindestens einmal erklungen. */
  readonly ausgeloest: number;
  /** Zuordnungen, die noch nie ausgelöst wurden, mit Grund. */
  readonly nochNicht: { id: string; ereignis: string; grund: string }[];
  /** Einträge im Verzeichnis ganz ohne Auslöser, nach Gruppe. */
  readonly ohneAusloeser: Readonly<Record<string, string[]>>;
  /** Was am Verzeichnis nicht stimmt — Zuordnung und Form. Muss leer sein. */
  readonly probleme: string[];
  /**
   * Anforderungen je Klang, absteigend. Bei Musik-Schichten zählt jedes
   * **Zuschalten** — eine Schicht wird nicht abgespielt, sondern aufgeblendet.
   */
  readonly proKlang: { id: string; anforderungen: number }[];
  readonly musik: {
    readonly laeuft: boolean;
    readonly schichten: string[];
    readonly hoerbar: string[];
  };
}

export function buildSoundReport(app: App): SoundReport {
  const audio = app.audio;
  const map = audio.map;
  const metrics = audio.audio?.metrics();
  const counts = audio.requestsBySound;

  const proKlang: { id: string; anforderungen: number }[] = [];
  const nochNicht: { id: string; ereignis: string; grund: string }[] = [];
  let ausgeloest = 0;

  for (let i = 0; i < map.ids.length; i++) {
    const id = map.ids[i] as string;
    const anforderungen = counts[i] ?? 0;
    proKlang.push({ id, anforderungen });
    if (anforderungen > 0) {
      ausgeloest++;
      continue;
    }
    const event = map.eventOf(i);
    const ereignis = event >= 0 ? (EVENT_NAMES[event] ?? '?') : '(nur Schicht)';
    nochNicht.push({
      id,
      ereignis,
      grund: NOT_YET[ereignis] ?? 'in diesem Lauf nicht eingetreten',
    });
  }
  proKlang.sort((a, b) => b.anforderungen - a.anforderungen || a.id.localeCompare(b.id));

  const ohneAusloeser: Record<string, string[]> = {};
  for (const entry of SOUND_MANIFEST.sounds) {
    if (entry.trigger !== undefined) continue;
    (ohneAusloeser[entry.group] ??= []).push(entry.id);
  }

  return {
    klangFehlend: metrics?.missing ?? 0,
    anforderungen: audio.requestCount,
    wiedergaben: metrics?.plays ?? 0,
    geladen: audio.isReady,
    kontext: audio.audio?.ctx.state ?? 'kein Kontext',
    stimmen: metrics?.activeVoices ?? 0,
    streuung: audio.audio?.recentRates(8) ?? [],
    zugeordnet: map.ids.length,
    ausgeloest,
    nochNicht,
    ohneAusloeser,
    probleme: [...audio.manifestProblems, ...map.problems],
    proKlang,
    musik: {
      laeuft: audio.isMusicRunning,
      schichten: audio.director?.runningLayers() ?? [],
      hoerbar: audio.director?.activeLayers() ?? [],
    },
  };
}
