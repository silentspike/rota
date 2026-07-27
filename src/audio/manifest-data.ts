/**
 * Das Verzeichnis als Modul.
 *
 * Getrennt von `manifest.ts`, damit die Pruefung ohne die echte Datei getestet
 * werden kann — und damit klar bleibt, dass es genau eine Quelle gibt:
 * `data/sound-manifest.json`.
 */
import raw from '../../data/sound-manifest.json';
import type { Manifest } from './manifest.js';

export const SOUND_MANIFEST = raw as unknown as Manifest;
