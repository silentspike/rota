/**
 * Die Credits-Daten als Modul.
 *
 * Getrennt von `credits.ts`, damit die Pruefung ohne die echte Datei getestet
 * werden kann — und damit klar bleibt, dass es genau eine Quelle gibt:
 * `data/credits.json`.
 */
import raw from '../../data/credits.json';

export interface CreditsEntry {
  readonly id: string;
  /** Was benutzt wird — englisch, es steht im Spiel. */
  readonly work: string;
  readonly author: string;
  /** Der Wortlaut, den die Lizenz verlangt. */
  readonly attribution: string;
  readonly license: string;
  readonly licenseUrl: string;
  readonly source: string;
  /**
   * Was am Werk veraendert wurde. CC BY verlangt, Bearbeitungen kenntlich zu
   * machen; fehlt das Feld, wurde die Datei unveraendert uebernommen.
   */
  readonly modified?: string;
  /** true = Namensnennung ist Lizenzauflage, nicht Hoeflichkeit. */
  readonly required: boolean;
  /** Welche Dateien unter assets/ dieser Eintrag deckt (Muster erlaubt). */
  readonly covers: readonly string[];
}

export interface CreditsData {
  readonly note: string;
  readonly entries: readonly CreditsEntry[];
}

export const CREDITS = raw as unknown as CreditsData;
