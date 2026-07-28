/**
 * Das Klang-Verzeichnis lesen und in Datensaetze der Ausgabe verwandeln.
 *
 * `data/sound-manifest.json` ist die eine Wahrheit darueber, welche Klaenge es
 * gibt, wofuer sie da sind, wie lang sie werden sollen und mit welchem Prompt
 * sie erzeugt wurden. Die Erzeugung (`tools/suno_api.py`) und das Spiel lesen
 * dieselbe Datei — sonst driften Prompt-Sammlung und Auslieferung auseinander.
 *
 * Die Datei wird von Hand gepflegt. Deshalb wird sie hier geprueft, statt ihr
 * zu vertrauen: Ein Tippfehler in einem Weg-Namen soll beim Laden auffallen und
 * nicht erst dann, wenn ein Klang im Kampf stumm bleibt.
 */
import type { Bus, OneShotDef, SoundDef, SustainedDef } from './types.js';

export interface ManifestEntry {
  readonly id: string;
  readonly group: string;
  readonly purpose: string;
  readonly kind: 'oneshot' | 'sustained';
  readonly bus: Bus;
  readonly file?: string;
  readonly attackFile?: string;
  readonly bodyFile?: string;
  readonly releaseFile?: string;
  readonly targetSeconds?: number;
  readonly loop?: boolean;
  readonly pitchJitterCents?: number;
  readonly gain?: number;
  readonly prompt?: string;
  readonly attackPrompt?: string;
  readonly bodyPrompt?: string;
  readonly releasePrompt?: string;
  readonly fadeInSeconds?: number;
  readonly fadeOutSeconds?: number;
  readonly loopStart?: number;
  readonly loopEnd?: number;
  /**
   * Gesetzt, wenn der Klang ein Ersatz ist, bis seine Vorlage ausgearbeitet
   * wurde. Solche Eintraege zaehlen **nicht** zur Abdeckung — sonst sae he das
   * Verzeichnis vollstaendiger aus, als es ist. Der Wert nennt den Grund.
   */
  readonly provisional?: string;
}

/**
 * Reservierter Platz fuer Klaenge, die es geben wird, sobald ihre Vorlage
 * ausgearbeitet ist. Der Verweis auf das Epic sagt, wer sie fuellt.
 */
export interface Placeholder {
  readonly group: string;
  readonly count: number;
  readonly epic: string;
  readonly reason: string;
}

export interface Manifest {
  readonly schema: number;
  readonly coverage: {
    readonly groups: Record<string, { target: number; present: number }>;
    readonly placeholders: readonly Placeholder[];
  };
  readonly sounds: readonly ManifestEntry[];
}

export interface ParsedManifest {
  readonly defs: readonly SoundDef[];
  readonly entries: readonly ManifestEntry[];
  /** Was im Verzeichnis nicht stimmt. Leer heisst: alles in Ordnung. */
  readonly problems: readonly string[];
}

const BUSES = new Set<string>(['music', 'sfx', 'ui']);

/**
 * Verzeichnis pruefen und in Datensaetze wandeln.
 *
 * Fehlerhafte Eintraege werden gemeldet **und** uebersprungen: Ein einzelner
 * kaputter Eintrag darf nicht das ganze Spiel stumm schalten.
 */
export function parseManifest(raw: Manifest): ParsedManifest {
  const problems: string[] = [];
  const defs: SoundDef[] = [];
  const seen = new Set<string>();

  for (const entry of raw.sounds) {
    const where = `Klang "${entry.id}"`;
    if (!entry.id) {
      problems.push('Ein Eintrag hat keine Kennung.');
      continue;
    }
    if (seen.has(entry.id)) {
      problems.push(`${where}: Kennung kommt mehrfach vor.`);
      continue;
    }
    seen.add(entry.id);

    if (!BUSES.has(entry.bus)) {
      problems.push(`${where}: unbekannter Weg "${entry.bus}".`);
      continue;
    }
    if (!entry.purpose) problems.push(`${where}: kein Zweck angegeben.`);
    if (entry.pitchJitterCents !== undefined && entry.pitchJitterCents < 0) {
      problems.push(`${where}: Streuung darf nicht negativ sein.`);
      continue;
    }

    if (entry.kind === 'oneshot') {
      if (!entry.file) {
        problems.push(`${where}: keine Datei angegeben.`);
        continue;
      }
      if (!entry.prompt) problems.push(`${where}: kein Prompt hinterlegt.`);
      if (entry.targetSeconds === undefined || entry.targetSeconds <= 0) {
        problems.push(`${where}: Ziel-Laenge fehlt oder ist nicht positiv.`);
      }
      const def: OneShotDef = {
        kind: 'oneshot',
        id: entry.id,
        bus: entry.bus,
        src: entry.file,
        ...(entry.gain !== undefined ? { gain: entry.gain } : {}),
        ...(entry.pitchJitterCents !== undefined
          ? { pitchJitterCents: entry.pitchJitterCents }
          : {}),
      };
      defs.push(def);
      continue;
    }

    if (entry.kind === 'sustained') {
      if (!entry.bodyFile) {
        problems.push(`${where}: dreiteiliger Klang ohne Koerper-Datei.`);
        continue;
      }
      if (!entry.bodyPrompt) problems.push(`${where}: kein Prompt fuer den Koerper hinterlegt.`);
      if (entry.attackFile && !entry.attackPrompt) {
        problems.push(`${where}: Ansatz-Datei ohne Prompt.`);
      }
      if (entry.releaseFile && !entry.releasePrompt) {
        problems.push(`${where}: Ausklang-Datei ohne Prompt.`);
      }
      const def: SustainedDef = {
        kind: 'sustained',
        id: entry.id,
        bus: entry.bus,
        bodySrc: entry.bodyFile,
        ...(entry.attackFile !== undefined ? { attackSrc: entry.attackFile } : {}),
        ...(entry.releaseFile !== undefined ? { releaseSrc: entry.releaseFile } : {}),
        ...(entry.loopStart !== undefined ? { loopStart: entry.loopStart } : {}),
        ...(entry.loopEnd !== undefined ? { loopEnd: entry.loopEnd } : {}),
        ...(entry.gain !== undefined ? { gain: entry.gain } : {}),
        ...(entry.fadeInSeconds !== undefined ? { fadeInSeconds: entry.fadeInSeconds } : {}),
        ...(entry.fadeOutSeconds !== undefined ? { fadeOutSeconds: entry.fadeOutSeconds } : {}),
        ...(entry.pitchJitterCents !== undefined
          ? { pitchJitterCents: entry.pitchJitterCents }
          : {}),
      };
      defs.push(def);
      continue;
    }

    problems.push(`${where}: unbekannte Art "${String(entry.kind)}".`);
  }

  // Die gefuehrte Abdeckung muss zum Inhalt passen, sonst faellt eine Luecke
  // niemandem auf.
  for (const [group, counts] of Object.entries(raw.coverage.groups)) {
    const actual = raw.sounds.filter((sound) => sound.group === group && !sound.provisional).length;
    if (actual !== counts.present) {
      problems.push(
        `Gruppe "${group}": Abdeckung sagt ${counts.present} Eintraege, tatsaechlich sind es ${actual}.`,
      );
    }
    const missing = counts.target - counts.present;
    if (missing === 0) continue;

    const placeholder = raw.coverage.placeholders.find((entry) => entry.group === group);
    if (!placeholder) {
      problems.push(
        `Gruppe "${group}": ${missing} Klaenge fehlen, ohne dass ein Platzhalter sie ausweist.`,
      );
    } else if (placeholder.count !== missing) {
      problems.push(
        `Gruppe "${group}": Platzhalter nennt ${placeholder.count} Klaenge, es fehlen aber ${missing}.`,
      );
    } else if (!placeholder.epic) {
      problems.push(`Gruppe "${group}": Platzhalter ohne Verweis auf ein Epic.`);
    }
  }

  return { defs, entries: raw.sounds, problems };
}

/** Wie viele Klaenge das Design vorsieht und wie viele beschrieben sind. */
export function coverageTotals(raw: Manifest): { target: number; present: number } {
  let target = 0;
  let present = 0;
  for (const counts of Object.values(raw.coverage.groups)) {
    target += counts.target;
    present += counts.present;
  }
  return { target, present };
}
