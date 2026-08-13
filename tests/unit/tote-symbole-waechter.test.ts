/**
 * Welches exportierte Wertsymbol benutzt niemand? (Issue #809)
 *
 * # Die Lücke, die dieser Wächter schliesst — sie stand schriftlich da
 *
 * `kein-unerreichbarer-code.test.ts` misst **Dateien** und nennt seine Grenze
 * selbst:
 *
 * > „Er sieht Dateien, nicht Funktionen. `buildEfficiencyView` steht in einer
 * > Datei, die aus einem anderen Grund lebt — der Wächter schweigt dazu. Diese
 * > Ebene bräuchte eine Auswertung der Exporte je Datei; sie ist hier bewusst
 * > nicht gebaut."
 *
 * Das ist die Ebene. Dass sie fehlte, hat an einem Tag fünfmal zugeschlagen:
 * `setSetEffects` machte **alle sechs Rotations-Effekte** für jeden Spieler
 * unerreichbar, und die Datei, in der es stand, war die ganze Zeit erreichbar.
 *
 * # Was gemeldet wird — und was ausdrücklich nicht
 *
 * ```text
 *   GEMELDET   exportiert, kein Vorkommen in einer anderen Datei UND
 *              keines in der eigenen ausser der Deklaration
 *
 *   NICHT      nur zu weit exportiert: in der eigenen Datei benutzt.
 *              Der Code läuft, der Bundler behält ihn zu Recht, niemand wird
 *              getäuscht. Sie zu melden hiesse, jeden internen Helfer beim
 *              Export zu überdenken — eine Stilregel, keine Fehlerklasse.
 *              Entscheidung des Orchestrators.
 *
 *   AUCH NICHT was in einer Datei steht, die als Ganzes unerreichbar ist. Die
 *              führt `BEKANNT_UNERREICHBAR` bereits mit Begründung; sonst stünde
 *              dieselbe Sache auf zwei Ebenen in zwei Listen.
 * ```
 *
 * **Keine Zahl in diesem Kopf**, ausser den zwei unten, die einen Vorgang
 * belegen: Wie viele es heute je Gruppe sind, sagt die Fehlermeldung des
 * Wächters aus der Messung — ein Kommentar sagte es aus dem Gedächtnis und
 * würde beim ersten Anschluss falsch, ohne dass ein Fall es merkt.
 *
 * # Warum über den Syntaxbaum und nicht über Zeilenmuster
 *
 * Der erste Entwurf zählte mit einem Zeilenmuster und einer Wortsuche im Text.
 * Er fand **19** Symbole, es sind **21**. Die zwei fehlenden kamen in einer
 * anderen Datei vor — nur eben im **Kommentar**:
 *
 * ```text
 *   runLiveMetrics  kein-unerreichbarer-code.test.ts:82  (Prosa)
 *   STREAM_HEAL     dot-tick-eigener-strom.test.ts:12    (Prosa)
 * ```
 *
 * Eine Textsuche kann Code nicht von Erklärung unterscheiden. Der Syntaxbaum
 * führt Kommentare gar nicht, also zählt hier nur, was wirklich ausgeführt
 * wird. Dieselbe Klasse in der Gegenrichtung: Ein mehrzeilig umbrochener
 * `export` wäre für ein Zeilenmuster unsichtbar gewesen.
 *
 * # Seine Grenzen — beide gemessen, keine geraten
 *
 * **1. „Nur in Tests benutzt" gilt als benutzt.** Das ist die Festlegung aus
 * #784, und sie hat einen Preis: `nextIntroduction` in `help.ts` hat keinen
 * Aufrufer im Spiel und wird trotzdem nicht gemeldet, weil `help.test.ts` sie
 * dreimal ruft. Der Fall ist bekannt — er steht seit dem Anschluss des Archivs
 * im Kopf des Datei-Wächters. Ein Wächter, der ihn fände, müsste Test-Code von
 * Spiel-Code trennen; das ist eine eigene Entscheidung und kein Nebenbei.
 *
 * **2. Ein Re-Export gilt als Nutzung.** `export { x } from './y'` nennt den
 * Namen in einer anderen Datei. Zehn solche Stellen gibt es. Die Richtung des
 * Irrtums ist bewusst gewählt: lieber ein totes Symbol übersehen als eins
 * melden, das über eine Sammelstelle sehr wohl benutzt wird.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join, normalize } from 'node:path';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';
import { BEKANNT_UNERREICHBAR } from './kein-unerreichbarer-code.test.js';

/** Eine Fundstelle: wo das Symbol steht und wie es heisst. */
interface Stelle {
  readonly datei: string;
  readonly art: string;
  readonly name: string;
}

/** Der Schlüssel einer Ausnahme: Pfad und Name, damit er eindeutig ist. */
const schluessel = (s: Stelle): string => `${s.datei}#${s.name}`;

/**
 * Was heute niemand benutzt und trotzdem stehen bleibt — mit seinem Grund.
 *
 * **Jede Zeile nennt ihren Vorgang.** Eine Ausnahmeliste ohne Begründung wird
 * zur Halde: Nach dem dritten Eintrag weiss niemand mehr, ob eine Zeile ein
 * offener Rest oder ein vergessener Fehler ist. Und der vorletzte Fall meldet,
 * wenn ein Eintrag überholt ist — sonst überlebt die Liste, was sie beschreibt.
 */
const AUSNAHMEN: Readonly<Record<string, string>> = {
  // ---- OFFEN: sie gehören an #811, nicht in den Papierkorb ----------------
  //
  // Beide stammen aus AC2 und AC4 von #85, die als eigenes Issue #811 laufen.
  // Sie jetzt zu löschen hiesse, sie dort neu zu schreiben. **Die Ausnahme hat
  // ein Ablaufdatum:** Sobald #811 sie anschliesst, wird der Fall „überholte
  // Ausnahme" rot und die Zeilen fallen.
  'src/analysis/archive-view.ts#attachHelp':
    'OFFEN bis #811 (Hilfe-Fenster, AC2 von #85) — die Ansicht hängt, die Hilfe daran fehlt noch',
  'src/analysis/archive-view.ts#buildIntroduction':
    'OFFEN bis #811 (geführte Einführung, AC4 von #85) — baut den Text, den erst #811 aufruft',

  // ---- Absicht: vorgesehene Auslöser, deren Klänge schon im Verzeichnis stehen
  //
  // `events.ts` sagt es im Kopf: das Verzeichnis führt ihre Klänge bereits, und
  // das Issue verlangt ausdrücklich, die Auslöser vorzusehen statt sie zu
  // belegen. Ein Auslöser, der erst mit seinem Klang entsteht, kommt in einer
  // anderen Nummer heraus — und verschiebt jede aufgezeichnete Prüfsumme.
  'src/core/events.ts#EV_SYNERGY_READY': 'vorgesehener Auslöser, Klang im Verzeichnis',
  'src/core/events.ts#EV_BOSS_CAST_START': 'vorgesehener Auslöser, Klang im Verzeichnis',
  'src/core/events.ts#EV_BOSS_CAST_SUCCESS': 'vorgesehener Auslöser, Klang im Verzeichnis',
  'src/core/events.ts#EV_BOSS_PHASE_CHANGE': 'vorgesehener Auslöser, Klang im Verzeichnis',
  'src/core/events.ts#EV_BOSS_REACTION_DESTABILISE': 'vorgesehener Auslöser, Klang im Verzeichnis',
  'src/core/events.ts#EV_PLAYER_LOW_HEALTH': 'vorgesehener Auslöser, Klang im Verzeichnis',

  // ---- Absicht: gemeinsamer Nummernraum mit dem Simulator ------------------
  //
  // GEMESSEN, nicht dem Kommentar geglaubt: `sim/src/fight.rs` schreibt alle
  // drei (`LogKind::Spend`, `::Kill`, `::PlayerDeath`). Die TypeScript-Seite
  // liest sie beim Vergleich der Aufzeichnungen. Wer eine entfernte, verschöbe
  // den Raum und damit jede gespeicherte Prüfsumme.
  'src/core/log.ts#EVENT_SPEND':
    'gemeinsamer Nummernraum — sim/src/fight.rs schreibt LogKind::Spend',
  'src/core/log.ts#EVENT_KILL': 'gemeinsamer Nummernraum — sim/src/fight.rs schreibt LogKind::Kill',
  'src/core/log.ts#EVENT_PLAYER_DEATH':
    'gemeinsamer Nummernraum — sim/src/fight.rs schreibt LogKind::PlayerDeath',
  // Dasselbe eine Ebene tiefer: `sim/src/rng.rs` führt `Heal = 2` und verweist
  // im Kommentar auf `src/core/rng.ts`. Der Strom ist leer, weil alle drei
  // Koagulations-Wege deterministisch sind (Design 21.1) — ihn zu entfernen und
  // später wieder einzufügen verschöbe jede Prüfsumme zweimal.
  'src/core/rng.ts#STREAM_HEAL': 'reservierter Strom — sim/src/rng.rs führt Heal = 2 dazu',

  // ---- Absicht: der Grund steht am Symbol -----------------------------------
  'src/devtools/reference-run.ts#REFERENCE_FIGHT_RUNNING':
    'macht den Kampf-Zustand für Prüfungen lesbar, wenn er nicht mehr läuft',
  'src/gear/pools.ts#POOLS_OF_SPECIAL_STATS':
    'erklärt in der Oberfläche, welche Töpfe einen Faktor ergeben — gerechnet wird über die Kennnummern',
  'src/loot/moment.ts#HIGHEST_PRESENTATION': 'die obere Grenze von presentationFor, für Prüfungen',
  'src/render/vessel.ts#VESSEL_INK': 'Farbe, die das HUD für Beschriftung und Zahlen braucht',
  'src/render/vessel.ts#VESSEL_INK_DIM': 'Farbe, die das HUD für Beschriftung und Zahlen braucht',

  // ---- Absicht: Glied einer vollständigen Reihe -----------------------------
  //
  // Sie stehen neben `SPACE_POSITION`, `DIFFICULTY_COUNT` und `BLOCK_WIDTH`.
  // Eine Reihe, bei der ein Glied fehlt, lädt zum Nachrechnen ein — und das
  // Nachrechnen ist die Fehlerquelle, nicht die ungenutzte Zeile.
  'src/input/keys.ts#ROW_COUNT': 'Glied der Tasten-Anordnung, neben SPACE_POSITION',
  'src/loot/rates.ts#DIFFICULTY_NAMES': 'Glied der Schwierigkeits-Reihe, neben DIFFICULTY_COUNT',
  'src/render/spellbar.ts#BLOCK_HEIGHT': 'Glied der Geometrie-Reihe, neben BLOCK_WIDTH',
};

function alleQuellen(ordner: string): string[] {
  const out: string[] = [];
  const gehe = (wo: string): void => {
    for (const eintrag of readdirSync(wo, { withFileTypes: true })) {
      const pfad = join(wo, eintrag.name);
      if (eintrag.isDirectory()) gehe(pfad);
      else if (eintrag.name.endsWith('.ts') && !eintrag.name.endsWith('.d.ts')) out.push(pfad);
    }
  };
  gehe(ordner);
  return out.map((p) => normalize(p));
}

function baumVon(datei: string): ts.SourceFile {
  return ts.createSourceFile(datei, readFileSync(datei, 'utf8'), ts.ScriptTarget.Latest, true);
}

const istExportiert = (knoten: ts.Node): boolean =>
  ts.canHaveModifiers(knoten) &&
  (ts.getModifiers(knoten) ?? []).some((m) => m.kind === ts.SyntaxKind.ExportKeyword);

/**
 * Die exportierten **Wert**symbole einer Datei.
 *
 * Interfaces und Typ-Aliase bleiben draussen: Sie verschwinden im Bau und
 * kosten zur Laufzeit nichts. Von 800 Exporten sind 85 solche Typen — sie
 * mitzumelden hiesse, eine Liste zu führen, in der nichts kaputt sein kann.
 */
export function exportierteWerte(quelle: ts.SourceFile, datei: string): Stelle[] {
  const out: Stelle[] = [];
  for (const s of quelle.statements) {
    if (!istExportiert(s)) continue;
    if (ts.isFunctionDeclaration(s) && s.name)
      out.push({ datei, art: 'function', name: s.name.text });
    else if (ts.isClassDeclaration(s) && s.name)
      out.push({ datei, art: 'class', name: s.name.text });
    else if (ts.isEnumDeclaration(s)) out.push({ datei, art: 'enum', name: s.name.text });
    else if (ts.isVariableStatement(s)) {
      for (const dek of s.declarationList.declarations)
        if (ts.isIdentifier(dek.name)) out.push({ datei, art: 'const', name: dek.name.text });
    }
  }
  return out;
}

/**
 * Wie oft jeder Bezeichner in einer Datei vorkommt — **ohne** die Namen der
 * Deklarationen auf oberster Ebene.
 *
 * Der Abzug ist der Kern: Ohne ihn zählte jedes Symbol sich selbst und niemand
 * wäre je tot. Kommentare fehlen von allein, weil der Syntaxbaum sie nicht
 * führt — genau das trennt diese Zählung von einer Textsuche.
 */
export function bezeichnerIn(quelle: ts.SourceFile): Map<string, number> {
  const eigene = new Set<ts.Node>();
  for (const s of quelle.statements) {
    if (ts.isFunctionDeclaration(s) && s.name) eigene.add(s.name);
    else if (ts.isClassDeclaration(s) && s.name) eigene.add(s.name);
    else if (ts.isEnumDeclaration(s)) eigene.add(s.name);
    else if (ts.isVariableStatement(s))
      for (const dek of s.declarationList.declarations)
        if (ts.isIdentifier(dek.name)) eigene.add(dek.name);
  }
  const zaehler = new Map<string, number>();
  const gehe = (k: ts.Node): void => {
    if (ts.isIdentifier(k) && !eigene.has(k)) zaehler.set(k.text, (zaehler.get(k.text) ?? 0) + 1);
    ts.forEachChild(k, gehe);
  };
  gehe(quelle);
  return zaehler;
}

/**
 * Die reine Rechnung, mit Argumenten statt Dateisystem.
 *
 * **Sie ist nicht nur für die Gegenprobe da** — `gemessen()` ruft genau diese
 * Funktion. Eine Rechnung, die nur im Prüffall vorkommt, belegt sich selbst und
 * sonst nichts; das ist dieselbe Klasse wie `buildEfficiencyView`, das gebaut,
 * grün und unerreichbar war.
 */
export function einordnen(
  kandidaten: readonly Stelle[],
  vorkommen: ReadonlyMap<string, ReadonlyMap<string, number>>,
): { tot: Stelle[]; nurZuWeit: Stelle[] } {
  const tot: Stelle[] = [];
  const nurZuWeit: Stelle[] = [];
  for (const s of kandidaten) {
    let woanders = 0;
    for (const [d, zaehler] of vorkommen) if (d !== s.datei) woanders += zaehler.get(s.name) ?? 0;
    if (woanders > 0) continue;
    if ((vorkommen.get(s.datei)?.get(s.name) ?? 0) > 0) nurZuWeit.push(s);
    else tot.push(s);
  }
  return { tot, nurZuWeit };
}

/**
 * Der Baum aller Quelldateien, **einmal** geparst.
 *
 * Ohne diesen Speicher lief der Wächter allein 1,5 s je Aufruf; bei sechs
 * Fällen und der vollen Suite daneben riss er die 5-s-Grenze und meldete einen
 * Zeitablauf statt eines Befundes. Der Baum ändert sich während eines Laufs
 * nicht — ihn sechsmal zu bauen war reine Wiederholung.
 */
let speicher: { kandidaten: Stelle[]; tot: Stelle[]; nurZuWeit: Stelle[] } | undefined;

function gemessen(): { kandidaten: Stelle[]; tot: Stelle[]; nurZuWeit: Stelle[] } {
  return (speicher ??= messen());
}

function messen(): { kandidaten: Stelle[]; tot: Stelle[]; nurZuWeit: Stelle[] } {
  // Dateien, die schon als Ganzes unerreichbar sind, bleiben draussen: Der
  // Datei-Wächter führt sie bereits mit Begründung.
  const gedeckteDateien = new Set(BEKANNT_UNERREICHBAR.map((d) => normalize(d)));
  const quellDateien = alleQuellen('src').filter(
    (d) => !d.endsWith('.test.ts') && !gedeckteDateien.has(d),
  );
  const alle = [...alleQuellen('src'), ...alleQuellen('tests')];

  const vorkommen = new Map<string, Map<string, number>>();
  const baeume = new Map<string, ts.SourceFile>();
  for (const d of alle) {
    const baum = baumVon(d);
    baeume.set(d, baum);
    vorkommen.set(d, bezeichnerIn(baum));
  }

  const kandidaten: Stelle[] = [];
  for (const d of quellDateien)
    kandidaten.push(...exportierteWerte(baeume.get(d) as ts.SourceFile, d));

  return { kandidaten, ...einordnen(kandidaten, vorkommen) };
}

describe('Kein exportiertes Wertsymbol ohne Nutzer', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // **Die Zusicherung vor der Messung.** Fände die Suche keine Exporte oder
    // keine Bezeichner, wäre alles darunter grün und prüfte nichts.
    const { kandidaten, tot, nurZuWeit } = gemessen();
    expect(kandidaten.length, 'keine exportierten Wertsymbole gefunden').toBeGreaterThan(400);
    expect(alleQuellen('src').length, 'keine Quelldateien gefunden').toBeGreaterThan(50);
    expect(
      tot.length + nurZuWeit.length,
      'kein einziges Symbol ohne Nutzer ausserhalb seiner Datei — die Zählung greift nicht',
    ).toBeGreaterThan(10);
  });

  it('Träger-Zusicherung — ein lebendes Symbol wird NICHT gemeldet', () => {
    // Ohne diesen Fall bliebe offen, ob der Wächter überhaupt zwischen lebend
    // und tot unterscheidet: Eine Zählung, die alles für benutzt hält, wäre
    // ebenso grün wie eine richtige.
    const { tot, nurZuWeit } = gemessen();
    const gemeldet = new Set([...tot, ...nurZuWeit].map((s) => s.name));
    for (const lebend of ['Combat', 'replayRecording', 'parseSets', 'parseEntries', 'SETS'])
      expect(gemeldet.has(lebend), `${lebend} lebt und wird trotzdem gemeldet`).toBe(false);
  });

  it('AC — kein Wertsymbol ohne Nutzung und ohne Ausnahme', () => {
    const { tot } = gemessen();
    const neu = tot.filter((s) => AUSNAHMEN[schluessel(s)] === undefined);

    expect(
      neu.map(schluessel),
      `Diese Symbole benutzt NIEMAND: nicht ausserhalb ihrer Datei, nicht darin. ` +
        `Sie sind exportiert, also wirft der Bundler sie nicht weg — sie liegen im ` +
        `ausgelieferten Spiel und tun nichts.\n\n` +
        `Entweder anschliessen oder löschen. Ist es ein bewusster offener Rest, ` +
        `gehört das Symbol mit seinem Vorgang in AUSNAHMEN — dann steht dort, WARUM ` +
        `es liegen bleibt.\n\n` +
        `Nur zu weit exportierte Symbole (in ihrer eigenen Datei benutzt) sind hier ` +
        `bewusst NICHT erfasst; ${gemessen().nurZuWeit.length} solche gibt es heute.`,
    ).toEqual([]);
  });

  it('und die Ausnahmeliste überlebt nicht, was sie beschreibt', () => {
    // **Die Gegenrichtung.** Wird ein Symbol angeschlossen oder gelöscht,
    // verschwindet es aus der Messung — die Zeile behauptete sonst weiter, es
    // gebe einen offenen Rest. Genau daran hängt das Ablaufdatum der beiden
    // #811-Einträge: Schliesst #811 sie an, wird dieser Fall rot.
    const jetztTot = new Set(gemessen().tot.map(schluessel));
    const ueberholt = Object.keys(AUSNAHMEN).filter((k) => !jetztTot.has(k));

    expect(
      ueberholt,
      `Diese Einträge in AUSNAHMEN sind erledigt — die Symbole sind angeschlossen ` +
        `oder entfernt. Die Zeilen gehören gelöscht, sonst beschreibt die Liste einen ` +
        `Zustand, den es nicht mehr gibt.`,
    ).toEqual([]);
  });

  it('jeder offene Eintrag nennt seine Ablaufbedingung', () => {
    // **Damit die grüne Liste nicht als „alles in Ordnung" gelesen wird.** Die
    // Zusicherung hängt an der EIGENSCHAFT, nicht an einem Namen: Ein Name
    // altert mit seiner Behebung und meldet dann einen Fehler, der keiner ist.
    const offen = Object.entries(AUSNAHMEN).filter(([, grund]) => grund.startsWith('OFFEN'));
    expect(
      offen.length,
      'kein einziger offener Befund — dann ist die Liste geschönt',
    ).toBeGreaterThan(0);
    for (const [name, grund] of offen)
      expect(grund, `${name} ist OFFEN, nennt aber kein Issue, das die Ausnahme beendet`).toMatch(
        /#\d+/,
      );
  });

  it('und der Wächter kann rot werden', () => {
    // Ohne diesen Fall belegte der obige nur, dass heute nichts Neues da ist —
    // nicht, dass ein neuer Fall auffiele. Mit Argumenten statt Dateisystem,
    // damit die Rechnung selbst geprüft wird und nicht der heutige Baum.
    const stelle = (datei: string, name: string): Stelle => ({ datei, art: 'const', name });
    const kandidaten = [
      stelle('src/a.ts', 'LEBT'),
      stelle('src/a.ts', 'TOT'),
      stelle('src/a.ts', 'ZU_WEIT'),
    ];
    const vorkommen = new Map<string, Map<string, number>>([
      ['src/a.ts', new Map([['ZU_WEIT', 3]])],
      ['src/b.ts', new Map([['LEBT', 2]])],
    ]);

    const erst = einordnen(kandidaten, vorkommen);
    expect(
      erst.tot.map((s) => s.name),
      'TOT muss auffallen',
    ).toEqual(['TOT']);
    // **Die Trennung ist der Kern des Wächters**, nicht ein Nebenprodukt: Ein
    // nur zu weit exportiertes Symbol darf NICHT in derselben Liste landen,
    // sonst trüge die Ausnahmeliste 25 Zeilen, bei denen nichts kaputt ist.
    expect(erst.nurZuWeit.map((s) => s.name)).toEqual(['ZU_WEIT']);

    // Und nach dem Anschliessen ist es still — die Gegenprobe zur Gegenprobe.
    const angeschlossen = new Map<string, Map<string, number>>([
      ['src/a.ts', new Map([['ZU_WEIT', 3]])],
      [
        'src/b.ts',
        new Map([
          ['LEBT', 2],
          ['TOT', 1],
        ]),
      ],
    ]);
    expect(einordnen(kandidaten, angeschlossen).tot).toEqual([]);
  });
});
