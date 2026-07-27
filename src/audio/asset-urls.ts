/**
 * Aufloesung der Klang-Dateien zu Adressen, die der Browser abrufen kann.
 *
 * Warum nicht einfach der Pfad aus dem Manifest: Vite gibt jeder Datei beim
 * Bauen einen Namen mit Inhalts-Pruefsumme. Ohne diese Zuordnung zeigt das
 * Manifest auf Quelldateien, die in der Auslieferung so nicht liegen — und die
 * Klaenge fehlten genau dort, wo es niemandem mehr auffaellt.
 *
 * Der Pfad im Manifest ist relativ zum Projekt, z. B. `audio/spell-kindle.ogg`.
 */
// Bewusst nur `assets/audio/`: Dort liegen die Klaenge des Spiels. Die uebrigen
// Ordner unter `assets/sfx/` sind Messreihen der Werkzeugkette (suno-sfx.md 7)
// und haetten in der Auslieferung nichts verloren.
const urls = import.meta.glob<string>('/assets/audio/**/*.{ogg,mp3,wav}', {
  eager: true,
  query: '?url',
  import: 'default',
});

/** `audio/x.ogg` -> gebaute Adresse, oder der Pfad selbst, wenn nichts passt. */
export function resolveAssetUrl(src: string): string {
  const key = src.startsWith('/') ? src : `/assets/${src}`;
  return urls[key] ?? src;
}

/** Alle bekannten Klang-Dateien, Projekt-Pfad ohne `/assets/`. */
export function knownAssets(): string[] {
  return Object.keys(urls)
    .map((key) => key.replace(/^\/assets\//, ''))
    .sort();
}
