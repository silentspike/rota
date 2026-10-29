/**
 * Der Archiv-Katalog gegen den Plan (Issue #1177, Abschnitt 5 „Archiv").
 *
 * # Was hier gemessen wird
 *
 * Der Masterplan (Geschichte und Ton) legt Umfang und Bindung fest:
 *
 * > Vor jeder Instanz und nach jedem **ersten** Boss-Sieg ein kurzer Text …
 * > **Archiv-Umfang: ~150 Einträge** — acht Instanz-Einleitungen, 32
 * > Boss-Einträge (je erster Sieg), fünf Prozess-Erklärungen,
 * > Endgame-Instanzen, dazu Einträge für Zauber, Gegenstands-Sätze und
 * > versteckte Funde.
 *
 * # Die Sollwerte kommen aus den Daten, nicht aus diesem Kopf
 *
 * Welche Instanzen und Bosse es gibt, steht in `data/instances.json` und
 * `data/bosses/`; welche Zauber, in `data/spells/`. Eine Liste hier wäre beim
 * ersten neuen Boss falsch — und bliebe grün, weil sie den neuen nicht kennt.
 * Deshalb wird jeder Fall über `INSTANCES`, `BOSSES` und die Zauberliste
 * geführt.
 *
 * Gegenprobe zu jeder Zählung: `ZUSICHERUNG`-Fälle, die belegen, dass die
 * Grundmenge nicht leer ist. Eine Schleife über nichts ist immer grün.
 */
import { describe, expect, it } from 'vitest';
import { PLAYABLE_ARCHIVE_ENTRIES } from '../../src/analysis/archive-data.js';
import { ARCHIVE_VOICES, type ArchiveEntry } from '../../src/analysis/archive.js';
import { BOSSES, CALCINATION, INSTANCES, type InstanceDef } from '../../src/core/data.js';
import { SETS } from '../../src/gear/sets.js';

const SCHULEN = ['CALCINATION', 'COAGULATION', 'FIXATION', 'PUTREFACTION', 'SUBLIMATION'];

/** Die Instanzen, die im Spiel stehen — der Prolog zählt dazu. */
const INSTANZ_KENNUNGEN = new Set(INSTANCES.map((i) => i.id));

/** Gebaut heißt: ein Datensatz, dessen Instanz in `instances.json` steht. */
const GEBAUTE_BOSSE = [...BOSSES.values()]
  .map((b) => b.value)
  .filter((b) => INSTANZ_KENNUNGEN.has(b.instance));

/** Die Kennungen, unter denen die kommenden Bosse einer Instanz erscheinen werden. */
function geplantePlaetze(instanz: InstanceDef): string[] {
  const nummer = /^instance-(\d+)$/.exec(instanz.id)?.[1];
  if (nummer === undefined) return [];
  return Array.from({ length: instanz.plannedBosses }, (_, k) => `boss-${nummer}-${k + 1}`);
}

/** Die spielbaren Zauber: alles bis Stufe 100, die Prüfzauber (101) nicht. */
const SPIELBARE_ZAUBER = CALCINATION.value.spells.filter((s) => s.unlockLevel <= 100);

const katalog: readonly ArchiveEntry[] = PLAYABLE_ARCHIVE_ENTRIES;
const vonArt = (trigger: string) => katalog.filter((e) => e.trigger === trigger);

describe('ZUSICHERUNG — die Grundmengen sind nicht leer', () => {
  it('es gibt gebaute Bosse, spielbare Zauber und einen Katalog', () => {
    expect(GEBAUTE_BOSSE.length).toBeGreaterThan(0);
    expect(SPIELBARE_ZAUBER.length).toBeGreaterThan(0);
    expect(katalog.length, 'der ausgelieferte Katalog ist leer').toBeGreaterThan(0);
  });
});

describe('Die Prüfeinträge stehen nicht im ausgelieferten Katalog', () => {
  it('keine Kennung mit dem Präfix test-', () => {
    const pruef = katalog.filter((e) => e.id.startsWith('test-')).map((e) => e.id);
    expect(pruef).toEqual([]);
  });

  it('und kein Text, der sich als Prüftext ausgibt', () => {
    // Die Prüfeinträge sagen von sich „A test record" — ein Text mit diesem
    // Satz wäre einer, der beim Aufräumen in den Katalog gewandert ist.
    const verdaechtig = katalog.filter((e) => /test record|test entry/i.test(e.title + e.body));
    expect(verdaechtig.map((e) => e.id)).toEqual([]);
  });
});

describe('Jede gebaute Instanz und jeder gebaute Boss hat seine Einträge', () => {
  it('jede Instanz mit Bossen hat genau eine Einleitung — außer dem Prolog', () => {
    // **Der Prolog hat keine, und das ist Absicht** (Masterplan, Einstieg):
    // „Kein Menü, kein Vorspann." Ein Text vor dem allerersten Kampf wäre ein
    // Vorspann.
    const mitBossen = new Set(GEBAUTE_BOSSE.map((b) => b.instance));
    for (const id of mitBossen) {
      const davor = vonArt('beforeInstance').filter((e) => e.instanceId === id);
      expect(davor.length, `${id}: Einleitungen`).toBe(id === 'prologue' ? 0 : 1);
    }
  });

  it('jeder gebaute Boss hat genau einen Eintrag zum ersten Sieg', () => {
    for (const boss of GEBAUTE_BOSSE) {
      const treffer = vonArt('afterFirstKill').filter((e) => e.bossId === boss.id);
      expect(treffer.length, `${boss.id}: Einträge zum ersten Sieg`).toBe(1);
      expect(treffer[0]?.instanceId, `${boss.id}: falsche Instanz`).toBe(boss.instance);
    }
  });
});

describe('Die noch nicht gebauten Instanzen sind vorbereitet', () => {
  it('jede Instanz des Aufstiegs und des Endgames hat ihre Einleitung', () => {
    const vierzehn = INSTANCES.filter((i) => i.id !== 'prologue');
    expect(vierzehn.length, 'V1 sind 14 Instanzen (Entscheidung 29.10.)').toBe(14);
    for (const instanz of vierzehn) {
      const davor = vonArt('beforeInstance').filter((e) => e.instanceId === instanz.id);
      expect(davor.length, `${instanz.id}: Einleitungen`).toBe(1);
    }
  });

  it('jeder geplante Boss-Platz hat seinen Eintrag zum ersten Sieg', () => {
    let plaetze = 0;
    for (const instanz of INSTANCES) {
      for (const bossId of geplantePlaetze(instanz)) {
        plaetze++;
        const treffer = vonArt('afterFirstKill').filter((e) => e.bossId === bossId);
        expect(treffer.length, `${bossId}: Einträge`).toBe(1);
        expect(treffer[0]?.instanceId).toBe(instanz.id);
      }
    }
    // 8 Aufstiegs-Instanzen à 4 plus 6 Endgame-Instanzen à 6 (Masterplan
    // „Umfang V1"). Die Zahl steht in der Meldung, nicht als Bedingung über
    // die Daten — die Bedingung ist, dass jeder Platz seinen Eintrag hat.
    expect(plaetze, 'Boss-Plätze laut instances.json').toBe(68);
  });
});

describe('Jeder Verweis trifft etwas, das es gibt oder geben wird', () => {
  const geplant = new Set(INSTANCES.flatMap(geplantePlaetze));
  const gebaut = new Set(GEBAUTE_BOSSE.map((b) => b.id));
  const zauber = new Set(SPIELBARE_ZAUBER.map((s) => s.id));
  const saetze = new Set(SETS.filter((s) => s.id > 0).map((s) => s.id));

  it('Instanz, Boss, Zauber, Schule und Satz', () => {
    for (const e of katalog) {
      if (e.instanceId !== undefined) {
        expect(INSTANZ_KENNUNGEN.has(e.instanceId), `${e.id}: Instanz ${e.instanceId}`).toBe(true);
      }
      if (e.bossId !== undefined) {
        const trifft = gebaut.has(e.bossId) || geplant.has(e.bossId);
        expect(trifft, `${e.id}: Boss ${e.bossId}`).toBe(true);
      }
      if (e.spellId !== undefined) {
        expect(zauber.has(e.spellId), `${e.id}: Zauber ${e.spellId}`).toBe(true);
      }
      if (e.school !== undefined) expect(SCHULEN, `${e.id}`).toContain(e.school);
      if (e.setId !== undefined) expect(saetze.has(e.setId), `${e.id}: Satz ${e.setId}`).toBe(true);
    }
  });
});

describe('Der Umfang gegen den Plan (~150)', () => {
  it('fünf Prozess-Erklärungen, eine je Schule', () => {
    expect(
      vonArt('schoolLearned')
        .map((e) => e.school)
        .sort(),
    ).toEqual([...SCHULEN].sort());
  });

  it('jeder spielbare Zauber hat seinen Eintrag', () => {
    const mitEintrag = new Set(vonArt('spellLearned').map((e) => e.spellId));
    const ohne = SPIELBARE_ZAUBER.filter((s) => !mitEintrag.has(s.id)).map((s) => s.id);
    expect(ohne).toEqual([]);
  });

  it('jeder Gegenstands-Satz ausser dem Prüfsatz hat seinen Eintrag', () => {
    const mitEintrag = new Set(vonArt('setFound').map((e) => e.setId));
    const echte = SETS.filter((s) => s.id > 0);
    expect(echte.length).toBeGreaterThan(0);
    for (const satz of echte) expect(mitEintrag.has(satz.id), `Satz ${satz.id}`).toBe(true);
  });

  it('es gibt versteckte Funde', () => {
    expect(vonArt('firstOutcome').length).toBeGreaterThan(0);
  });

  it('zusammen rund 150 — die Zahl steht in der Meldung', () => {
    const je = Object.fromEntries(
      [...new Set(katalog.map((e) => e.trigger))].map((t) => [t, vonArt(t).length]),
    );
    expect(
      katalog.length,
      `Katalog: ${katalog.length} Einträge, ${JSON.stringify(je)}`,
    ).toBeGreaterThanOrEqual(140);
    expect(katalog.length, `Katalog: ${katalog.length} Einträge`).toBeLessThanOrEqual(160);
  });
});

describe('Stimmen und Sprache', () => {
  it('alle vier Stimmen aus dem Plan sind vertreten', () => {
    const stimmen = new Set(katalog.map((e) => e.voice));
    for (const stimme of ARCHIVE_VOICES) expect(stimmen, stimme).toContain(stimme);
  });

  it('Spieltexte englisch — keine Umlaute, keine deutschen Anführungszeichen', () => {
    const deutsch = katalog.filter((e) => /[äöüßÄÖÜ„“]/.test(e.title + e.body)).map((e) => e.id);
    expect(deutsch).toEqual([]);
  });

  it('jede Instanz mit Schwerpunkt nennt ihn in ihrer Einleitung', () => {
    // Entscheidung f-instanz5-schwerpunkt-schule (28.10.): Der Schwerpunkt
    // wird über Mechanik und **Archiv-Texte** ausgedrückt. Eine Einleitung,
    // die ihre Schule nicht nennt, drückte ihn nicht aus.
    const mitSchwerpunkt = INSTANCES.filter((i) => typeof i.focus === 'string');
    expect(mitSchwerpunkt.length, 'kein Schwerpunkt in den Daten').toBeGreaterThan(0);
    for (const instanz of mitSchwerpunkt) {
      const focus = instanz.focus ?? '';
      const text = vonArt('beforeInstance')
        .filter((e) => e.instanceId === instanz.id)
        .map((e) => `${e.title} ${e.body}`)
        .join(' ')
        .toLowerCase();
      const erwartet = focus === 'ALL' ? 'five' : focus.toLowerCase();
      expect(text, `${instanz.id} nennt ${focus} nicht`).toContain(erwartet);
    }
  });
});
