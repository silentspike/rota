/**
 * Grundtypen der Klang-Ausgabe (GAME-DESIGN 17.1).
 *
 * Bewusst nur Daten, kein Verhalten: Ein Klang ist ein Datensatz, genau wie ein
 * Zauber ein Datensatz ist (GAME-DESIGN 18.3). Das Sound-Manifest aus Issue
 * E8-2 wird direkt in diese Strukturen geladen.
 */

/**
 * Die drei getrennt regelbaren Wege (GAME-DESIGN 17.1).
 *
 * Getrennt, weil viele nebenbei eigene Musik hoeren und sonst den Ton ganz
 * abschalten wuerden — und damit die klanglichen Hinweise verlieren.
 */
export type Bus = 'music' | 'sfx' | 'ui';

export const BUSES: readonly Bus[] = ['music', 'sfx', 'ui'] as const;

/** Ein einzeln abspielbarer Klang mit fester Laenge. */
export interface OneShotDef {
  readonly kind: 'oneshot';
  readonly id: string;
  readonly bus: Bus;
  /** Datei relativ zum Wurzelverzeichnis der Auslieferung. */
  readonly src: string;
  /** Grundlautstaerke des Klangs, linear. 1 = wie aufgenommen. */
  readonly gain?: number;
  /**
   * Tonhoehen-Streuung in Cent (100 Cent = ein Halbton), je Wiedergabe zufaellig
   * aus [-wert, +wert]. Verhindert den Maschinengewehr-Effekt.
   *
   * Bei tonalen Klaengen klein halten oder auf 0 setzen, sonst wirkt es
   * verstimmt (GAME-DESIGN 17.1).
   */
  readonly pitchJitterCents?: number;
}

/**
 * Ein Klang, der so lange laeuft wie der Spieleffekt — dreiteilig aufgebaut
 * (GAME-DESIGN 17.1, suno-sfx.md 4.2):
 *
 * | Teil    | Quelle   | Abspielweise                          |
 * | ------- | -------- | ------------------------------------- |
 * | Ansatz  | One-Shot | einmal beim Start                     |
 * | Koerper | Loop     | in Schleife, solange der Effekt laeuft |
 * | Ausklang| One-Shot | einmal beim Enden oder Abbrechen      |
 *
 * Eine feste Datei waere hier grundsaetzlich falsch: Der Spieler kann
 * abbrechen, ein Verstaerkungs-Fenster kann frueher enden.
 */
export interface SustainedDef {
  readonly kind: 'sustained';
  readonly id: string;
  readonly bus: Bus;
  /** Ansatz — entfaellt bei Klaengen, die einfach einblenden (z. B. Ambiente). */
  readonly attackSrc?: string;
  /** Koerper — die Schleife. Pflicht. */
  readonly bodySrc: string;
  /** Ausklang — entfaellt, wenn der Klang nur ausblenden soll. */
  readonly releaseSrc?: string;
  /**
   * Schleifen-Grenzen in Sekunden innerhalb der Koerper-Datei. Ohne Angabe
   * laeuft die Schleife ueber die ganze Datei.
   *
   * Loop-Dateien werden in der Nachbearbeitung nie beschnitten (suno-sfx.md
   * 4.2) — die Naht liegt also exakt an den Dateigrenzen.
   */
  readonly loopStart?: number;
  readonly loopEnd?: number;
  readonly gain?: number;
  /** Einblendung des Koerpers in Sekunden. 0 = hart einsetzen. */
  readonly fadeInSeconds?: number;
  /** Ausblendung des Koerpers beim Beenden in Sekunden. */
  readonly fadeOutSeconds?: number;
  readonly pitchJitterCents?: number;
}

export type SoundDef = OneShotDef | SustainedDef;

/**
 * Bezeichner einer laufenden Stimme. Bewusst eine Zahl und kein Objekt: Im
 * heissen Pfad wird nichts erzeugt (GAME-DESIGN 18.2).
 *
 * Aufbau: `slotIndex | (generation << 8)`. Die Generation verhindert, dass ein
 * alter Bezeichner eine inzwischen neu vergebene Stimme trifft.
 */
export type VoiceHandle = number;

/** Ergebnis eines fehlgeschlagenen Starts. */
export const NO_VOICE: VoiceHandle = -1;
