/**
 * Anschluss des Kampfes an die Klang-Ausgabe aus Epic E8.
 *
 * Der Kampf kennt nur Kennungen von Klängen; alles Weitere — Puffer, Plätze,
 * Lautstärke-Wege — liegt hinter `AudioEngine` (GAME-DESIGN 17.1). Diese Datei ist
 * die einzige Stelle, an der beide Seiten sich berühren.
 *
 * **Stummheit ist kein Fehler.** Fehlt der Klang-Kontext, schlägt das Laden fehl
 * oder liegt eine Datei noch nicht vor, läuft das Spiel unverändert weiter. Ein
 * stummes Spiel rechnet identisch.
 */
import { createGameAudio, knownAssets, type AudioEngine } from '../audio/index.js';
import { SOUND_MANIFEST } from '../audio/manifest-data.js';
import { parseManifest } from '../audio/manifest.js';
import type { FeedbackSink } from './session.js';

/** Fehleingabe: dumpfer Ton neben dem roten Blinken (GAME-DESIGN 8.2). */
export const SOUND_INPUT_ERROR = 'feedback-input-error';

export class AudioFeedback implements FeedbackSink {
  private engine: AudioEngine | null = null;
  private ready = false;
  private failure: string | null = null;
  private unlocked = false;
  private requested = 0;
  private spellSounds: readonly string[] = [];

  /** Ist ein Klang-Kontext vorhanden und geladen? */
  get isReady(): boolean {
    return this.ready;
  }

  get lastFailure(): string | null {
    return this.failure;
  }

  /** Wie oft der Kampf einen Klang angefordert hat — auch im stummen Zustand. */
  get requestCount(): number {
    return this.requested;
  }

  get audio(): AudioEngine | null {
    return this.engine;
  }

  /**
   * Klang-Kontext anlegen und alle Dateien laden, die es tatsächlich gibt.
   *
   * Das Verzeichnis führt alle ~115 vorgesehenen Klänge, von denen erst wenige
   * erzeugt sind (GAME-DESIGN 22, Sound-Manifest). Würden alle angemeldet, liefe
   * beim Start ein Schwall vergeblicher Abrufe — deshalb wird gegen die Liste der
   * wirklich gebauten Dateien gefiltert.
   */
  async init(): Promise<void> {
    try {
      const engine = createGameAudio();
      const available = new Set(knownAssets());
      const defs = parseManifest(SOUND_MANIFEST).defs.filter((def) =>
        def.kind === 'oneshot' ? available.has(def.src) : available.has(def.bodySrc),
      );
      engine.library.register(defs);
      const report = await engine.load();
      this.engine = engine;
      this.ready = report.loaded > 0;
      if (report.failures.length > 0) {
        this.failure = `${report.failures.length} Datei(en) nicht ladbar`;
      }
    } catch (error) {
      this.failure = String(error);
      this.ready = false;
    }
  }

  /**
   * Nach der ersten Nutzergeste aufrufen. Browser halten den Klang-Kontext bis
   * dahin an; ohne dieses Aufwecken bleibt alles stumm.
   */
  unlock(): void {
    if (this.unlocked || this.engine === null) return;
    this.unlocked = true;
    void this.engine.unlock();
  }

  /**
   * Klang je Zauber, nach dessen Kennnummer im Kampf.
   *
   * Ein Zauber ohne eigenen Klang bleibt stumm, statt einen fremden zu leihen —
   * das Verzeichnis führt erst drei der sechzig (GAME-DESIGN 22).
   */
  setSpellSounds(soundIds: readonly string[]): void {
    this.spellSounds = soundIds;
  }

  onPressAccepted(_slot: number, spellIndex: number, _timeMs: number): void {
    const id = this.spellSounds[spellIndex];
    if (id === undefined) return;
    this.requested++;
    if (this.engine === null) return;
    this.engine.play(id);
  }

  onPressRejected(_slot: number, _reason: number, _timeMs: number): void {
    this.requested++;
    if (this.engine === null) return;
    this.engine.play(SOUND_INPUT_ERROR);
  }
}
