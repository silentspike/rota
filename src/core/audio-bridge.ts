/**
 * Anschluss des Kampfes an die Klang-Ausgabe aus Epic E8 (GAME-DESIGN 17.1).
 *
 * Der Kampf meldet Ereignisse; welcher Klang daran hängt, steht im Verzeichnis
 * (`data/sound-manifest.json`, Feld `trigger`) und wird beim Start in
 * `SoundMap` übersetzt. **Hier steht deshalb keine einzige Fallunterscheidung
 * über Zauber oder Ereignisse** — ein neuer Zauber ist ein Datensatz
 * (Leitplanke 6).
 *
 * **Stummheit ist kein Fehler.** Fehlt der Klang-Kontext, schlägt das Laden fehl
 * oder liegt eine Datei noch nicht vor, läuft das Spiel unverändert weiter. Ein
 * stummes Spiel rechnet identisch (GAME-DESIGN 18.5).
 */
import { createGameAudio, knownAssets, type AudioEngine } from '../audio/index.js';
import { SOUND_MANIFEST } from '../audio/manifest-data.js';
import { parseManifest } from '../audio/manifest.js';
import { MusicDirector, type MusicLayer } from '../audio/music.js';
import type { FeedbackSink } from './events.js';
import { SoundMap } from './sound-map.js';

export class AudioFeedback implements FeedbackSink {
  private engine: AudioEngine | null = null;
  private music: MusicDirector | null = null;
  private ready = false;
  private failure: string | null = null;
  private unlocked = false;
  private musicStarted = false;
  private requested = 0;

  /** Das Verzeichnis, einmal gelesen und geprüft. */
  private readonly manifest = parseManifest(SOUND_MANIFEST);

  /** Die Zuordnung aus dem Verzeichnis. */
  readonly map = new SoundMap(this.manifest.entries);

  /** Wie oft jeder Klang angefordert wurde — auch im stummen Zustand. */
  private readonly requestsPerSound = new Int32Array(this.map.ids.length);

  /**
   * Welche Musik-Schicht **jetzt** hörbar sein müsste.
   *
   * Wird auch geführt, solange es noch keine Musik gibt. Der Klang-Kontext wacht
   * erst mit der ersten Nutzergeste auf (Browser-Regel); bis dahin kann der
   * Kampf längst begonnen haben. Ohne diesen Merker liefe danach der
   * Menü-Teppich weiter, obwohl gekämpft wird — im Bild geprüft und genau so
   * aufgetreten.
   */
  private readonly layerWanted = new Uint8Array(this.map.layers.length);

  constructor() {
    // Der Grundklang ist von Anfang an gewollt; alles andere schaltet der Kampf.
    for (let i = 0; i < this.map.layers.length; i++) {
      if (this.map.layers[i]?.base !== true) continue;
      this.layerWanted[i] = 1;
      this.countLayer(i);
    }
  }

  private countLayer(layer: number): void {
    const sound = this.map.soundOfLayer[layer];
    if (sound === undefined) return;
    this.requestsPerSound[sound] = (this.requestsPerSound[sound] ?? 0) + 1;
  }

  /** Was beim Lesen des Verzeichnisses auffiel. Leer heißt: in Ordnung. */
  get manifestProblems(): readonly string[] {
    return this.manifest.problems;
  }

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

  get director(): MusicDirector | null {
    return this.music;
  }

  /** Anforderungen je Klang, nach Kennnummer in `map.ids`. */
  get requestsBySound(): Int32Array {
    return this.requestsPerSound;
  }

  /**
   * Die Zauber des Kampfes nachreichen.
   *
   * Erst damit wird aus „`spell.cast` mit Betreff kindle" eine Zeile, die im
   * Takt mit einer Kennnummer nachgeschlagen werden kann.
   */
  bindSpells(spellIds: readonly string[]): void {
    this.map.bindSpells(spellIds);
  }

  /**
   * Klang-Kontext anlegen und alle Dateien laden, die es tatsächlich gibt.
   *
   * Das Verzeichnis führt alle 126 vorgesehenen Klänge, von denen 56 erzeugt
   * sind (GAME-DESIGN 22). Würden alle angemeldet, liefe beim Start ein Schwall
   * vergeblicher Abrufe — deshalb wird gegen die Liste der wirklich gebauten
   * Dateien gefiltert.
   */
  async init(): Promise<void> {
    try {
      const engine = createGameAudio();
      const available = new Set(knownAssets());
      const defs = this.manifest.defs.filter((def) =>
        def.kind === 'oneshot' ? available.has(def.src) : available.has(def.bodySrc),
      );
      engine.library.register(defs);
      const report = await engine.load();
      this.engine = engine;
      this.music = new MusicDirector(engine);
      this.ready = report.loaded > 0;
      if (report.failures.length > 0) {
        this.failure = `${report.failures.length} Datei(en) nicht ladbar`;
      }
      this.startMusic();
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
    this.startMusic();
  }

  /**
   * Alle Musik-Schichten mitlaufen lassen.
   *
   * Erst wenn beides steht — geladen **und** aufgeweckt. Vorher liefe die
   * Schleife in einem angehaltenen Kontext, und alle Schichten stünden nachher
   * um die Wartezeit zueinander verschoben (siehe `music.ts`).
   */
  private startMusic(): void {
    if (this.musicStarted || !this.ready || !this.unlocked || this.music === null) return;
    this.musicStarted = true;
    // Hörbar ist, was **jetzt** hörbar sein müsste — nicht, was beim Laden der
    // Seite gegolten hätte.
    const layers: MusicLayer[] = this.map.layers.map((layer, i) =>
      this.layerWanted[i] === 1 ? { id: layer.id, base: true } : { id: layer.id },
    );
    this.music.start(layers);
  }

  get isMusicRunning(): boolean {
    return this.music?.isRunning() ?? false;
  }

  /**
   * Ein Kampf-Ereignis vertonen — der heiße Pfad.
   *
   * Nachschlagen in vorab gebauten Zahlen-Listen; es entsteht nichts
   * (GAME-DESIGN 18.2). Was der Browser je Wiedergabe an Speicher verlangt,
   * hängt an der Zahl der Klänge, nicht an der Bildzahl (VERIFICATION § 3.5).
   */
  onEvent(event: number, key: number, _timeMs: number): void {
    const sounds = this.map.soundsFor(event, key);
    for (let i = 0; i < sounds.length; i++) {
      const sound = sounds[i] as number;
      this.requested++;
      this.requestsPerSound[sound] = (this.requestsPerSound[sound] ?? 0) + 1;
      if (this.engine !== null) this.engine.play(this.map.ids[sound] as string);
    }

    const director = this.music;
    const on = this.map.layersOn(event);
    for (let i = 0; i < on.length; i++) {
      const index = on[i] as number;
      const layer = this.map.layers[index];
      if (layer === undefined) continue;
      // Das Zuschalten einer Schicht zählt wie eine Anforderung — sonst stünde
      // im Bericht „nie ausgelöst" bei einer Schicht, die hörbar geschaltet hat.
      if (this.layerWanted[index] !== 1) this.countLayer(index);
      this.layerWanted[index] = 1;
      if (director !== null) director.enable(layer.id, layer.immediate);
    }
    const off = this.map.layersOff(event);
    for (let i = 0; i < off.length; i++) {
      const index = off[i] as number;
      const layer = this.map.layers[index];
      if (layer === undefined) continue;
      this.layerWanted[index] = 0;
      if (director !== null) director.disable(layer.id, layer.immediate);
    }
  }

  /** Welche Schichten hörbar sein müssten — auch bevor es Musik gibt. */
  get wantedLayers(): string[] {
    const out: string[] = [];
    for (let i = 0; i < this.layerWanted.length; i++) {
      if (this.layerWanted[i] === 1) out.push(this.map.layers[i]?.id ?? '?');
    }
    return out;
  }
}
