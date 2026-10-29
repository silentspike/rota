/**
 * Fehlt ein Pflichtfeld, nennt das Spiel es und verwirft den Datensatz
 * (Issue #1055) — je Feld an echten Daten.
 *
 * # Warum
 *
 * Ohne `dot.tickIntervalMs` lehnte der Rust-Bau den Zauber ab
 * (`calcination.json/spells[smoulder]/dot: Feld "tickIntervalMs" fehlt`), das
 * Spiel las ihn mit Tick-Abstand 0 weiter und zeichnete endlos Tick-Marken.
 * Gemessen am 2026-10-29: Von 25 Feldern, die den Rust-Bau brechen, wies die
 * TypeScript-Seite eines ab.
 *
 * # Wie
 *
 * Für jeden Eintrag der Tafeln in `src/core/pflichtfelder.ts` wird jeder
 * Datensatz gesucht, der das Feld trägt, das Feld darin gestrichen und der
 * Datensatz durch den echten Lader geschickt. Erwartet: ein Problem, das das
 * Feld beim Namen nennt, und der Datensatz fehlt im Ergebnis. Ohne Streichung
 * lädt derselbe Datensatz ohne Problem — sonst bewiese das Verwerfen nichts.
 *
 * Ein Eintrag ohne einen einzigen Träger in den Daten ist ein Fehler dieses
 * Prüffalls, kein Bestehen: Er bliebe sonst ungeprüft und sähe geprüft aus.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { bossVerzeichnis, parseSchool } from '../../src/core/data.js';
import { BOSS_PFLICHT, ZAUBER_PFLICHT, type Pflichtfeld } from '../../src/core/pflichtfelder.js';

type Roh = Record<string, unknown>;

function lies(pfad: string): Roh {
  return JSON.parse(readFileSync(new URL(`../../${pfad}`, import.meta.url), 'utf8')) as Roh;
}

const RAHMEN = lies('data/spells/calcination.json');
const ZAUBERDATEIEN = readdirSync(new URL('../../data/spells/', import.meta.url))
  .filter((name) => name.endsWith('.json'))
  .sort()
  .map((name) => ({ name, roh: lies(`data/spells/${name}`) }));
const BOSSDATEIEN = Object.fromEntries(
  readdirSync(new URL('../../data/bosses/', import.meta.url))
    .filter((name) => name.endsWith('.json'))
    .sort()
    .map((name) => [name, lies(`data/bosses/${name}`)]),
);

/** Die Elternobjekte, an denen `pfad` sein letztes Glied trägt, samt Weg. */
function traeger(datensatz: unknown, eintrag: Pflichtfeld): { eltern: Roh; weg: string }[] {
  const teile = eintrag.pfad.split('.');
  const blatt = teile.at(-1) as string;
  let ebene: { eltern: unknown; weg: string }[] = [{ eltern: datensatz, weg: '' }];
  for (const teil of teile.slice(0, -1)) {
    const name = teil.replace(/\[\]$/, '');
    const naechste: { eltern: unknown; weg: string }[] = [];
    for (const { eltern, weg } of ebene) {
      const kind = (eltern as Roh | null)?.[name];
      if (kind === undefined || kind === null) continue;
      if (teil.endsWith('[]')) {
        if (Array.isArray(kind))
          kind.forEach((k, i) => naechste.push({ eltern: k, weg: `${weg}${name}[${i}].` }));
      } else naechste.push({ eltern: kind, weg: `${weg}${name}.` });
    }
    ebene = naechste;
  }
  return ebene
    .filter(
      ({ eltern }) => typeof eltern === 'object' && eltern !== null && blatt in (eltern as Roh),
    )
    .filter(({ eltern }) => {
      const wenn = eintrag.wenn;
      if (wenn === undefined) return true;
      const wert = (eltern as Roh)[wenn.feld];
      return typeof wert === 'string' && wenn.werte.includes(wert);
    })
    .map(({ eltern, weg }) => ({ eltern: eltern as Roh, weg }));
}

function kopie<T>(wert: T): T {
  return JSON.parse(JSON.stringify(wert)) as T;
}

/** Den Zauber in seiner Schule lesen, wie `CALCINATION` es mit jeder Datei tut. */
function leseZauber(datei: Roh, zauber: Roh) {
  const resource = datei['resource'] ?? RAHMEN['resource'];
  return parseSchool({ ...RAHMEN, resource, spells: [zauber] });
}

const blattVon = (eintrag: Pflichtfeld): string => eintrag.pfad.split('.').at(-1) as string;

describe('Zauber: ein fehlendes Pflichtfeld wird genannt, der Zauber verworfen (#1055)', () => {
  const zwingend = ZAUBER_PFLICHT.filter((e) => e.bewacht !== true);

  it.each(zwingend.map((e) => [e.pfad, e] as const))('%s', (_pfad, eintrag) => {
    const blatt = blattVon(eintrag);
    let faelle = 0;
    for (const { name, roh } of ZAUBERDATEIEN) {
      const liste = (roh['spells'] ?? []) as Roh[];
      liste.forEach((zauber, index) => {
        for (const { weg } of traeger(zauber, eintrag)) {
          const id = String(zauber['id']);
          const vorher = leseZauber(roh, zauber);
          expect(vorher.problems, `${name}/${id} ohne Streichung`).toEqual([]);
          expect(vorher.value.spells.map((s) => s.id)).toEqual([id]);

          const gestrichen = kopie(zauber);
          const eltern = traeger(gestrichen, eintrag).find((t) => t.weg === weg)?.eltern as Roh;
          delete eltern[blatt];
          const nachher = leseZauber(roh, gestrichen);
          expect(
            nachher.problems.some((p) => p.includes(`Feld "${blatt}" fehlt`)),
            `${name}/spells[${index}] (${id}): ${weg}${blatt} gestrichen — ${JSON.stringify(nachher.problems)}`,
          ).toBe(true);
          expect(
            nachher.value.spells.map((s) => s.id),
            `${name}/${id} verworfen`,
          ).toEqual([]);
          faelle++;
        }
      });
    }
    expect(faelle, `kein Zauber trägt ${eintrag.pfad}`).toBeGreaterThan(0);
  });

  it.each(ZAUBER_PFLICHT.filter((e) => e.bewacht === true).map((e) => [e.pfad, e] as const))(
    '%s darf fehlen; steht es, muss es ein Betrag mit genau einer Ressource sein',
    (_pfad, eintrag) => {
      const blatt = blattVon(eintrag);
      let faelle = 0;
      for (const { name, roh } of ZAUBERDATEIEN) {
        for (const zauber of (roh['spells'] ?? []) as Roh[]) {
          for (const { weg } of traeger(zauber, eintrag)) {
            const id = String(zauber['id']);
            const ohne = kopie(zauber);
            delete (traeger(ohne, eintrag).find((t) => t.weg === weg)?.eltern as Roh)[blatt];
            const gelesen = leseZauber(roh, ohne);
            expect(gelesen.problems, `${name}/${id}: ${weg}${blatt} gestrichen`).toEqual([]);
            expect(gelesen.value.spells.map((s) => s.id)).toEqual([id]);

            if (eintrag.art === 'betrag') {
              // Die Streichung, die der Durchgang fährt: der Eintrag im Betrag
              // (`resourceGain.heat`) — dann steht `{}` da, und das bricht den Bau.
              const leer = kopie(zauber);
              (traeger(leer, eintrag).find((t) => t.weg === weg)?.eltern as Roh)[blatt] = {};
              const verworfen = leseZauber(roh, leer);
              expect(
                verworfen.problems.some((p) => p.includes(`"${blatt}" nennt 0 Ressourcen`)),
                `${name}/${id}: ${weg}${blatt} = {} — ${JSON.stringify(verworfen.problems)}`,
              ).toBe(true);
              expect(verworfen.value.spells).toEqual([]);
            }
            faelle++;
          }
        }
      }
      expect(faelle, `kein Zauber trägt ${eintrag.pfad}`).toBeGreaterThan(0);
    },
  );
});

describe('Bosse: ein fehlendes Pflichtfeld wird genannt, der Boss verworfen (#1055)', () => {
  it('ohne Streichung verwirft das Verzeichnis keinen Boss', () => {
    const verzeichnis = bossVerzeichnis(kopie(BOSSDATEIEN));
    expect(verzeichnis.verworfen).toEqual([]);
    expect(verzeichnis.bosse.size).toBe(Object.keys(BOSSDATEIEN).length);
  });

  it.each(BOSS_PFLICHT.map((e) => [e.pfad, e] as const))('%s', (_pfad, eintrag) => {
    const blatt = blattVon(eintrag);
    let faelle = 0;
    for (const [datei, boss] of Object.entries(BOSSDATEIEN)) {
      // Je Datei die erste Stelle genügt — bei Listen also ein Eintrag, nicht alle.
      const erste = traeger(boss, eintrag)[0];
      if (erste === undefined) continue;
      const gestrichen = kopie(boss);
      delete (traeger(gestrichen, eintrag).find((t) => t.weg === erste.weg)?.eltern as Roh)[blatt];
      const verzeichnis = bossVerzeichnis({ ...kopie(BOSSDATEIEN), [datei]: gestrichen });
      expect(
        verzeichnis.verworfen.some((p) => p.includes(`Feld "${blatt}" fehlt`)),
        `${datei}: ${erste.weg}${blatt} gestrichen — ${JSON.stringify(verzeichnis.verworfen)}`,
      ).toBe(true);
      expect(verzeichnis.bosse.size, `${datei} verworfen`).toBe(
        Object.keys(BOSSDATEIEN).length - 1,
      );
      expect(verzeichnis.bosse.has(String(boss['id'])), `${datei} nicht im Verzeichnis`).toBe(
        false,
      );
      faelle++;
    }
    expect(faelle, `kein Boss trägt ${eintrag.pfad}`).toBeGreaterThan(0);
  });
});
