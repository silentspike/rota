import { describe, expect, it } from 'vitest';
import { coverageTotals, parseManifest, type Manifest } from '../../src/audio/manifest.js';
import { SOUND_MANIFEST } from '../../src/audio/manifest-data.js';

// Ueber Vite statt ueber das Dateisystem: derselbe Weg, den auch das Spiel
// nimmt (src/audio/asset-urls.ts), und ohne Node-Typen im Browser-Projekt.
const ASSET_FILES = Object.keys(import.meta.glob('/assets/audio/*.ogg'));

describe('Sound-Manifest — das gepflegte Verzeichnis', () => {
  it('laesst sich ohne Beanstandung lesen', () => {
    const parsed = parseManifest(SOUND_MANIFEST);
    expect(parsed.problems).toEqual([]);
    expect(parsed.defs.length).toBe(SOUND_MANIFEST.sounds.length);
  });

  it('vergibt jede Kennung nur einmal', () => {
    const ids = SOUND_MANIFEST.sounds.map((sound) => sound.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('legt jede Datei unter assets/audio ab und benennt sie nach der Kennung', () => {
    for (const sound of SOUND_MANIFEST.sounds) {
      const files = [sound.file, sound.attackFile, sound.bodyFile, sound.releaseFile].filter(
        (file): file is string => typeof file === 'string',
      );
      expect(files.length).toBeGreaterThan(0);
      for (const file of files) {
        expect(file.startsWith('audio/')).toBe(true);
        expect(file.endsWith('.ogg')).toBe(true);
        expect(file).toContain(sound.id);
      }
    }
  });

  it('gibt fuer jeden Klang einen Zweck und einen Prompt an', () => {
    for (const sound of SOUND_MANIFEST.sounds) {
      expect(sound.purpose.length).toBeGreaterThan(10);
      const prompt = sound.kind === 'oneshot' ? sound.prompt : sound.bodyPrompt;
      expect(prompt?.length ?? 0).toBeGreaterThan(10);
    }
  });

  it('haelt sich an die Aufstellung aus GAME-DESIGN 17.1 (115 Klaenge)', () => {
    expect(coverageTotals(SOUND_MANIFEST).target).toBe(115);
  });

  it('begruendet jede Luecke — keine Gruppe bleibt unerklaert offen', () => {
    for (const [group, counts] of Object.entries(SOUND_MANIFEST.coverage.groups)) {
      if (counts.present >= counts.target) continue;
      const reason = SOUND_MANIFEST.coverage.blocked.find((entry) => entry.group === group);
      expect(reason, `Gruppe ${group} ohne Begruendung`).toBeDefined();
      expect(reason!.reason.length).toBeGreaterThan(30);
      expect(reason!.missing).toBe(counts.target - counts.present);
    }
  });

  it('kennt jede Datei, die unter assets/audio liegt — keine Waisen', () => {
    const known = new Set(
      SOUND_MANIFEST.sounds.flatMap((sound) =>
        [sound.file, sound.attackFile, sound.bodyFile, sound.releaseFile]
          .filter((file): file is string => typeof file === 'string')
          .map((file) => `/assets/${file}`),
      ),
    );
    for (const file of ASSET_FILES) {
      expect(known.has(file), `${file} liegt im Ordner, steht aber in keinem Eintrag`).toBe(true);
    }
  });
});

describe('parseManifest — Pruefung fehlerhafter Verzeichnisse', () => {
  const base: Manifest = {
    schema: 1,
    coverage: { groups: { test: { target: 1, present: 1 } }, blocked: [] },
    sounds: [
      {
        id: 'a',
        group: 'test',
        purpose: 'Ein hinreichend langer Zweck.',
        kind: 'oneshot',
        bus: 'sfx',
        file: 'audio/a.ogg',
        targetSeconds: 1,
        prompt: 'dry percussive impact',
      },
    ],
  };

  it('meldet doppelte Kennungen', () => {
    const parsed = parseManifest({
      ...base,
      coverage: { groups: { test: { target: 2, present: 2 } }, blocked: [] },
      sounds: [base.sounds[0]!, base.sounds[0]!],
    });
    expect(parsed.problems.some((p) => p.includes('mehrfach'))).toBe(true);
    expect(parsed.defs).toHaveLength(1);
  });

  it('meldet einen unbekannten Weg', () => {
    const parsed = parseManifest({
      ...base,
      sounds: [{ ...base.sounds[0]!, bus: 'lautsprecher' as never }],
    });
    expect(parsed.problems.some((p) => p.includes('unbekannter Weg'))).toBe(true);
    expect(parsed.defs).toHaveLength(0);
  });

  it('meldet eine fehlende Ziel-Laenge', () => {
    const sound = { ...base.sounds[0]! };
    delete (sound as { targetSeconds?: number }).targetSeconds;
    const parsed = parseManifest({ ...base, sounds: [sound] });
    expect(parsed.problems.some((p) => p.includes('Ziel-Laenge'))).toBe(true);
  });

  it('meldet eine Luecke ohne Begruendung', () => {
    const parsed = parseManifest({
      ...base,
      coverage: { groups: { test: { target: 5, present: 1 } }, blocked: [] },
    });
    expect(parsed.problems.some((p) => p.includes('ohne dass ein Grund'))).toBe(true);
  });

  it('meldet eine Abdeckung, die nicht zum Inhalt passt', () => {
    const parsed = parseManifest({
      ...base,
      coverage: { groups: { test: { target: 3, present: 3 } }, blocked: [] },
    });
    expect(parsed.problems.some((p) => p.includes('tatsaechlich'))).toBe(true);
  });

  it('nimmt einen dreiteiligen Klang samt Schleifen-Grenzen an', () => {
    const parsed = parseManifest({
      ...base,
      sounds: [
        {
          id: 'loop',
          group: 'test',
          purpose: 'Ein hinreichend langer Zweck.',
          kind: 'sustained',
          bus: 'music',
          attackFile: 'audio/loop-attack.ogg',
          bodyFile: 'audio/loop-body.ogg',
          releaseFile: 'audio/loop-release.ogg',
          attackPrompt: 'energy channel start, rising hum',
          bodyPrompt: 'sustained magical channeling hum, seamless loop',
          releasePrompt: 'energy channel release, fading hum',
          loop: true,
          loopStart: 0.2,
          loopEnd: 8,
        },
      ],
    });
    expect(parsed.problems).toEqual([]);
    const def = parsed.defs[0];
    expect(def?.kind).toBe('sustained');
    if (def?.kind === 'sustained') {
      expect(def.loopStart).toBe(0.2);
      expect(def.loopEnd).toBe(8);
      expect(def.attackSrc).toBe('audio/loop-attack.ogg');
    }
  });
});
