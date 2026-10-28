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

  // 17.1 nennt seit dem 2026-10-28 135: die 126 bis dahin gezaehlten Ziele plus die 9 Klaenge
  // der Boss-Ankuendigungen (#797). Sie standen erst als vorlaeufige Eintraege ausserhalb der
  // Ziele; seit ihre Dateien liegen, zaehlt sie die Gruppe `announce` — die Zeile
  // „Boss-Ankuendigungen 4 + 5" der Tabelle in 17.1.
  it('haelt sich an die gezaehlte Aufstellung aus GAME-DESIGN 17.1 (135 Ziele)', () => {
    expect(coverageTotals(SOUND_MANIFEST).target).toBe(135);
    expect(SOUND_MANIFEST.coverage.groups.announce).toEqual({ target: 9, present: 9 });
  });

  it('weist jede Luecke als Platzhalter mit Epic und Begruendung aus', () => {
    for (const [group, counts] of Object.entries(SOUND_MANIFEST.coverage.groups)) {
      if (counts.present >= counts.target) continue;
      const placeholder = SOUND_MANIFEST.coverage.placeholders.find(
        (entry) => entry.group === group,
      );
      expect(placeholder, `Gruppe ${group} ohne Platzhalter`).toBeDefined();
      expect(placeholder!.count).toBe(counts.target - counts.present);
      expect(placeholder!.epic.length).toBeGreaterThan(1);
      expect(placeholder!.reason.length).toBeGreaterThan(30);

      // **Der Grund muss die Zahlen NENNEN, die er erklärt** (Issue #512).
      //
      // Die Länge allein hat nicht getragen: Über 30 Zeichen erfüllt jeder
      // Satz, auch der falsche. Gemessen — der Text beschrieb 2026-08-07
      // korrekt Klang-Einträge (98d5563), und heute stand dort wieder
      // „Von den 60 Zaubern sind dreizehn ausgearbeitet".
      //
      // Der Rückfall entstand nicht aus Nachlässigkeit: Jemand hat die ZAHL
      // gepflegt (zehn → dreizehn) und dabei den Satz neu geschrieben. **Eine
      // Korrektur an einem Text, der aus einem anderen Grund gepflegt wird,
      // überlebt die nächste Pflege nicht** — es sei denn, sie ist geprüft.
      //
      // Geprüft wird, was dastehen MUSS, nicht was verboten wäre: Wer
      // umformuliert, darf umformulieren; wer die Größe wechselt, verliert
      // eine der beiden Zahlen und fällt auf.
      //
      // **Was dieser Fall NICHT fängt:** „60 Zauber, davon 32 ausgearbeitet"
      // — beide Zahlen richtig, die Bedeutung falsch. Dagegen hilft nur der
      // Text selbst, und deshalb nennt er die Größe im ersten Satz.
      expect(
        placeholder!.reason,
        `Der Grund für '${group}' nennt die Zielzahl ${counts.target} nicht — ` +
          'dann beschreibt er eine andere Größe als die, die er erklären soll',
      ).toContain(String(counts.target));
      expect(
        placeholder!.reason,
        `Der Grund für '${group}' nennt die vorhandenen ${counts.present} nicht — ` +
          'genau diese Zahl ging beim Rückfall von #512 verloren',
      ).toContain(String(counts.present));
    }
  });

  it('beschreibt alle ableitbaren Slots vollstaendig', () => {
    // 135 minus die 60 Zauber und die 14 Instanz-Grundklaenge (beide E10)
    // ergibt 61: die 47 Klaenge ausserhalb von Musik und Ankuendigungen, die
    // fuenf festen Schichten und die 9 Ankuendigungen. Vorlaeufige Eintraege
    // zaehlen nicht mit.
    const ableitbar = SOUND_MANIFEST.sounds.filter(
      (sound) => sound.group !== 'spells' && !sound.provisional,
    );
    expect(ableitbar).toHaveLength(61);
    for (const sound of ableitbar) {
      if (sound.kind === 'oneshot') {
        expect(sound.file, sound.id).toBeDefined();
        expect(sound.targetSeconds, sound.id).toBeGreaterThan(0);
        expect(sound.prompt, sound.id).toBeDefined();
      } else {
        expect(sound.bodyFile, sound.id).toBeDefined();
        expect(sound.bodyPrompt, sound.id).toBeDefined();
      }
    }
  });

  it('zaehlt vorlaeufige Eintraege nicht zur Abdeckung', () => {
    const provisional = SOUND_MANIFEST.sounds.filter((sound) => sound.provisional);
    expect(provisional.length).toBeGreaterThan(0);
    for (const sound of provisional) {
      expect(sound.provisional!.length).toBeGreaterThan(20);
    }
    // Die Abdeckung der Gruppe zaehlt sie nicht mit.
    const music = SOUND_MANIFEST.sounds.filter(
      (sound) => sound.group === 'music' && !sound.provisional,
    );
    expect(SOUND_MANIFEST.coverage.groups.music!.present).toBe(music.length);
  });

  it('deckt mit Beschriebenem und Platzhaltern zusammen die vollen 135 ab', () => {
    const platzhalter = SOUND_MANIFEST.coverage.placeholders.reduce((a, p) => a + p.count, 0);
    const beschrieben = SOUND_MANIFEST.sounds.filter((sound) => !sound.provisional).length;
    expect(beschrieben + platzhalter).toBe(135);
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

  // ------------------------------------------------------------------
  // Die Gegenrichtung — Issue #204
  //
  // „Keine Waisen" oben fängt die eine Richtung: eine Datei ohne Eintrag.
  // Die andere fiel bis heute nirgends auf. Ein Eintrag ohne Datei ist
  // still in der Prüfung UND still im Spiel: `src/audio/library.ts` fängt
  // den Ladefehler ab, der Zauber wird gewirkt, es passiert kein Klang.
  //
  // Gemessen am 2026-08-05 waren so **neun** Zauber stumm, ohne dass ein
  // einziger Prüffall etwas sagte. Wer COAGULATION oder FIXATION spielte,
  // hörte seine halbe Leiste nicht und hielt es für Absicht.
  // ------------------------------------------------------------------

  /** Welche Einträge verweisen auf eine Datei, die es nicht gibt? */
  function eintraegeOhneDatei(manifest: Manifest, vorhanden: ReadonlySet<string>): string[] {
    const fehlend: string[] = [];
    for (const sound of manifest.sounds) {
      // Vorläufige Einträge sind ausgenommen — sie sagen selbst, dass die
      // Datei fehlt, und zählen nicht zur Abdeckung. Ohne diese Ausnahme
      // schlüge der Wächter beim nächsten Platzhalter an und würde dann
      // entschärft; danach wäre er weg.
      if (sound.provisional) continue;
      for (const file of [sound.file, sound.attackFile, sound.bodyFile, sound.releaseFile]) {
        if (typeof file === 'string' && !vorhanden.has(`/assets/${file}`)) {
          fehlend.push(`${sound.id} -> ${file}`);
        }
      }
    }
    return fehlend;
  }

  it('führt keinen Eintrag ohne Datei', () => {
    expect(eintraegeOhneDatei(SOUND_MANIFEST, new Set(ASSET_FILES))).toEqual([]);
  });

  it('würde einen Eintrag ohne Datei auch finden', () => {
    // Ohne diesen Fall wäre oben nicht zu unterscheiden, ob nichts fehlt
    // oder ob die Prüfung nichts sieht. Eine Datei aus der Menge nehmen
    // ist dasselbe, als wäre sie nie erzeugt worden.
    const eine = SOUND_MANIFEST.sounds.find((sound) => sound.file && !sound.provisional);
    expect(eine, 'kein Eintrag mit Datei — dann prüft der Wächter nichts').toBeDefined();
    const ohne = new Set(ASSET_FILES.filter((file) => file !== `/assets/${eine!.file}`));
    expect(eintraegeOhneDatei(SOUND_MANIFEST, ohne)).toEqual([`${eine!.id} -> ${eine!.file}`]);
  });

  it('nimmt vorläufige Einträge aus — auch das belegt', () => {
    // Der Gegenfall zum Gegenfall: Ein Platzhalter darf den Wächter nicht
    // auslösen, sonst steht er dem nächsten Manifest-Eintrag im Weg.
    const platzhalter: Manifest = {
      ...SOUND_MANIFEST,
      sounds: [
        {
          id: 'spell-noch-nicht',
          group: 'spells',
          purpose: 'Platzhalter für diesen Prüffall',
          kind: 'oneshot',
          bus: 'sfx',
          file: 'audio/spell-noch-nicht.ogg',
          targetSeconds: 0.5,
          prompt: 'placeholder',
          provisional: 'Eintrag steht, Datei fehlt — Beispiel für diesen Prüffall.',
        },
      ],
    };
    expect(eintraegeOhneDatei(platzhalter, new Set(ASSET_FILES))).toEqual([]);

    // Und ohne die Marke schlägt derselbe Eintrag an — sonst wäre nicht
    // belegt, dass die Ausnahme greift und nicht etwas anderes.
    // `provisional` wird entfernt, nicht auf `undefined` gesetzt: Der Typ
    // lässt das nicht zu, und `npm run build` bricht daran ab, während
    // `vitest` grün bleibt — es transpiliert ohne Typprüfung.
    const ohneFeld = { ...platzhalter.sounds[0]! };
    delete (ohneFeld as { provisional?: string }).provisional;
    const ohneMarke: Manifest = { ...platzhalter, sounds: [ohneFeld] };
    expect(eintraegeOhneDatei(ohneMarke, new Set(ASSET_FILES))).toEqual([
      'spell-noch-nicht -> audio/spell-noch-nicht.ogg',
    ]);
  });
});

describe('parseManifest — Pruefung fehlerhafter Verzeichnisse', () => {
  const base: Manifest = {
    schema: 1,
    coverage: { groups: { test: { target: 1, present: 1 } }, placeholders: [] },
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
      coverage: { groups: { test: { target: 2, present: 2 } }, placeholders: [] },
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

  it('meldet eine Luecke ohne Platzhalter', () => {
    const parsed = parseManifest({
      ...base,
      coverage: { groups: { test: { target: 5, present: 1 } }, placeholders: [] },
    });
    expect(parsed.problems.some((p) => p.includes('ohne dass ein Platzhalter'))).toBe(true);
  });

  it('meldet eine Abdeckung, die nicht zum Inhalt passt', () => {
    const parsed = parseManifest({
      ...base,
      coverage: { groups: { test: { target: 3, present: 3 } }, placeholders: [] },
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
