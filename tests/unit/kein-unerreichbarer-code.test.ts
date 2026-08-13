import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, normalize } from 'node:path';

/**
 * Kein neuer Code, den der Bundler wegwirft.
 *
 * # Der Anlass
 *
 * In der Nacht zum 2026-08-12 fanden sich **fünf abgenommene Issues**, deren
 * Code im ausgelieferten Spiel nicht vorkommt:
 *
 * ```text
 *   E7-1  #81  Effizienz-Tabelle    buildEfficiencyView wird nirgends gerufen
 *   E7-2  #82  Verlauf/Vergleich    history-view.ts erreicht niemand
 *   E7-4  #84  Trainingsraum        training-view.ts erreicht niemand
 *   E7-5  #85  Archiv und Hilfe     archive-view.ts erreicht niemand
 *   E6-4        Aufwertung           loot/upgrade.ts erreicht niemand
 * ```
 *
 * Alle vier E7-Issues wurden am 04.08. geschlossen. Die Tests waren grün, der
 * Bau lief, `tsc` und `eslint` schwiegen — **denn keines dieser Werkzeuge
 * fragt, ob der Code jemals erreicht wird.**
 *
 * # Warum Erreichbarkeit und nicht „wird importiert"
 *
 * Der erste Anlauf fragte: Welche Datei importiert niemand? Antwort: drei.
 * Der zweite: Welche Datei importiert nur eine tote? Antwort: sechs. Erst der
 * dritte Anlauf stellte die richtige Frage — **welche Datei erreicht man vom
 * Einstiegspunkt aus nicht?** Antwort: acht.
 *
 * Die ersten beiden Antworten waren nicht falsch, sie waren zu flach. Eine
 * Kette stirbt nicht Glied für Glied, sondern ab dem ersten toten Glied. Genau
 * das tut der Bundler, und genau das tut dieser Wächter.
 *
 * # Was er NICHT leistet
 *
 * Er sieht Dateien, nicht Funktionen. `buildEfficiencyView` steht in einer
 * Datei, die aus einem anderen Grund lebt (`buildLossView` wird gerufen) — der
 * Wächter schweigt dazu. Diese Ebene bräuchte eine Auswertung der Exporte je
 * Datei; sie ist hier bewusst nicht gebaut, weil ein Wächter, der zu viel
 * behauptet, schlimmer ist als einer, der seine Grenze nennt.
 */

const SRC = 'src';

/**
 * Die Einstiegspunkte — von hier aus beginnt jede Erreichbarkeit.
 *
 * **Sie stehen in HTML, nicht in TypeScript**, und werden deshalb von keinem
 * `import` genannt. Wer sie vergäße, bekäme sie als „unerreichbar" gemeldet und
 * hielte den Wächter für kaputt.
 */
const EINSTIEGE = [
  'src/main.ts', // index.html
  'src/audio/soundboard.ts', // soundboard.html, vite.config.ts
] as const;

/**
 * Was heute unerreichbar ist und bekannt bleibt, bis darüber entschieden wurde.
 *
 * **Jede Zeile nennt ihren Vorgang.** Eine Ausnahmeliste ohne Begründung wird
 * zur Halde: Nach dem dritten Eintrag weiß niemand mehr, ob eine Zeile ein
 * offener Rest oder ein vergessener Fehler ist.
 *
 * Die Liste ist ausdrücklich **kein Freibrief**. Sie hält den Zustand fest,
 * damit ein NEUNTER Fall auffällt — und der Fall darunter meldet, wenn ein
 * Eintrag verschwindet, damit sie nicht überlebt, was sie beschreibt.
 */
const BEKANNT_UNERREICHBAR: readonly string[] = [
  'src/analysis/archive-view.ts', // E7-5, Issue #85
  'src/analysis/archive.ts', // E7-5, nur von archive-view gebraucht
  'src/analysis/help.ts', // E7-5, nur von archive-view gebraucht
  'src/analysis/history-view.ts', // E7-2, Issue #82
  'src/analysis/run-compare.ts', // E7-2, nur von history-view gebraucht
  'src/analysis/training-view.ts', // E7-4, Issue #84
  'src/analysis/training.ts', // E7-4, nur von training-view gebraucht
  'src/loot/upgrade.ts', // E6-4, Aufwertung nach GAME-DESIGN 13.6
];

function alleQuellen(): string[] {
  const out: string[] = [];
  const gehe = (ordner: string): void => {
    for (const eintrag of readdirSync(ordner, { withFileTypes: true })) {
      const pfad = join(ordner, eintrag.name);
      if (eintrag.isDirectory()) gehe(pfad);
      else if (eintrag.name.endsWith('.ts') && !eintrag.name.endsWith('.d.ts')) out.push(pfad);
    }
  };
  gehe(SRC);
  return out.map((p) => normalize(p));
}

/**
 * Welche Quelldateien eine Datei importiert.
 *
 * Erfasst wird jede Form mit `from '…'` — das schließt `export … from` und
 * `import type … from` ein. **Typ-Importe zählen mit**, obwohl sie im Bau
 * verschwinden: Ein Wächter, der sie ausließe, meldete Dateien als tot, die
 * für die Typprüfung gebraucht werden. Die Richtung des Irrtums ist hier
 * bewusst gewählt — lieber eine Datei zu viel als erreichbar zählen als einen
 * Fehlalarm erzeugen.
 */
function zieleVon(datei: string, bekannt: ReadonlySet<string>): string[] {
  const text = readFileSync(datei, 'utf8');
  const out: string[] = [];
  for (const treffer of text.matchAll(/from\s+'([^']+)'/g)) {
    const ziel = treffer[1];
    if (ziel === undefined || !ziel.startsWith('.')) continue;
    const roh = normalize(join(dirname(datei), ziel));
    const kandidat = roh.endsWith('.js') ? `${roh.slice(0, -3)}.ts` : `${roh}.ts`;
    if (bekannt.has(kandidat)) out.push(kandidat);
    else if (bekannt.has(roh)) out.push(roh);
  }
  return out;
}

/**
 * Die reine Rechnung, mit Argumenten statt Dateisystem — nur so lässt sich
 * zeigen, dass der Wächter rot werden KANN (siehe letzter Fall).
 */
export function unerreichbare(
  alle: readonly string[],
  kanten: ReadonlyMap<string, readonly string[]>,
  einstiege: readonly string[],
): string[] {
  const erreicht = new Set<string>(einstiege.filter((e) => alle.includes(e)));
  const rand = [...erreicht];
  while (rand.length > 0) {
    const jetzt = rand.pop() as string;
    for (const ziel of kanten.get(jetzt) ?? []) {
      if (!erreicht.has(ziel)) {
        erreicht.add(ziel);
        rand.push(ziel);
      }
    }
  }
  return alle.filter((d) => !erreicht.has(d)).sort();
}

function gemessen(): string[] {
  const alle = alleQuellen();
  const bekannt = new Set(alle);
  const kanten = new Map<string, readonly string[]>();
  for (const datei of alle) kanten.set(datei, zieleVon(datei, bekannt));
  return unerreichbare(
    alle,
    kanten,
    EINSTIEGE.map((e) => normalize(e)),
  );
}

describe('Kein Code, den der Bundler wegwirft', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // **Die Zusicherung vor der Messung.** Läse das Verzeichnis leer oder
    // fänden die Einstiegspunkte nichts, wäre der Fall darunter grün und
    // prüfte nichts.
    const alle = alleQuellen();
    expect(alle.length, 'keine Quelldateien gefunden').toBeGreaterThan(50);
    for (const e of EINSTIEGE) {
      expect(alle, `Einstiegspunkt ${e} fehlt — der Wächter misst gegen nichts`).toContain(
        normalize(e),
      );
    }
  });

  it('keine NEUE unerreichbare Datei', () => {
    const neu = gemessen().filter((d) => !BEKANNT_UNERREICHBAR.includes(d));

    expect(
      neu,
      `Diese Dateien erreicht man von keinem Einstiegspunkt aus. Der Bundler wirft ` +
        `sie weg — sie sind im ausgelieferten Spiel NICHT vorhanden, auch wenn ihre ` +
        `Tests grün sind.\n\n` +
        `Entweder fehlt der Aufruf, oder die Datei gehört entfernt. Ist es ein ` +
        `bewusster offener Rest, gehört sie mit ihrem Vorgang in BEKANNT_UNERREICHBAR ` +
        `— dann steht dort, WARUM sie liegen bleibt.\n\n` +
        `Geprüft wurden ${alleQuellen().length} Quelldateien von ${EINSTIEGE.length} ` +
        `Einstiegspunkten aus; ${BEKANNT_UNERREICHBAR.length} bekannte Fälle sind ausgenommen.`,
    ).toEqual([]);
  });

  it('und die Ausnahmeliste überlebt nicht, was sie beschreibt', () => {
    // **Die Gegenrichtung**, damit die Liste keine Halde wird: Wird eine der
    // acht angeschlossen oder gelöscht, verschwindet sie hier — und die Zeile
    // behauptete sonst weiter, es gäbe einen offenen Rest.
    const verwaist = BEKANNT_UNERREICHBAR.filter((d) => !gemessen().includes(d));

    expect(
      verwaist,
      `Diese Einträge in BEKANNT_UNERREICHBAR sind erledigt — die Dateien sind ` +
        `entweder angeschlossen oder entfernt. Die Zeilen gehören gelöscht, sonst ` +
        `beschreibt die Liste einen Zustand, den es nicht mehr gibt.`,
    ).toEqual([]);
  });

  it('und der Wächter kann rot werden', () => {
    // Ohne diesen Fall belegte der obige nur, dass heute nichts Neues da ist —
    // nicht, dass ein neuer Fall auffiele.
    const kanten = new Map<string, readonly string[]>([
      ['src/main.ts', ['src/a.ts']],
      ['src/a.ts', []],
      ['src/tot.ts', ['src/auch-tot.ts']],
      ['src/auch-tot.ts', []],
    ]);
    const alle = ['src/main.ts', 'src/a.ts', 'src/tot.ts', 'src/auch-tot.ts'];

    // Die Kette stirbt ab dem ersten toten Glied — beide werden gemeldet, nicht
    // nur das erste. Genau dieser Schritt hat beim Fund drei Anläufe gebraucht.
    expect(unerreichbare(alle, kanten, ['src/main.ts'])).toEqual(['src/auch-tot.ts', 'src/tot.ts']);
    expect(unerreichbare(alle, kanten, ['src/main.ts', 'src/tot.ts'])).toEqual([]);
  });
});
