import { describe, expect, it } from 'vitest';
// Direkt statt per `import.meta.glob`: Das Glob-Muster überspringt
// Punkt-Verzeichnisse, `.github/` bliebe also unsichtbar — und eine Prüfung,
// die ihre Quelle nicht findet, belegt nichts.
import ciYaml from '../../.github/workflows/ci.yml?raw';

/**
 * Der Pfad-Filter der `sim`-Stufe kennt alle Eingaben der Rust-Tests (#173).
 *
 * # Warum diese Datei existiert
 *
 * Das Muster in `.github/workflows/ci.yml` ist eine **von Hand geführte
 * Abschrift** dessen, was die Rust-Tests lesen. Sie war unvollständig: Die
 * dreizehn Vergleichsdateien unter `tests/parity/recordings/` werden per
 * `include_str!` eingebunden, fielen aber durch das Muster. Ein Zweig, der nur
 * eine davon neu erzeugt — ein alltäglicher Vorgang —, übersprang genau die
 * Stufe, die diese Datei prüft.
 *
 * Gemessen: Eine einzelne verfälschte Prüfsumme in `fixation.json` lässt
 * `cargo test --test parity_fixation` mit Exit 101 scheitern. Die Stufe hätte
 * also etwas zu prüfen gehabt.
 *
 * # Warum die Prüfung aus den `include_str!`-Zeilen folgt
 *
 * Eine zweite Liste hier wäre dieselbe Abschrift noch einmal — und die nächste
 * Datei, die jemand einbindet, fehlte in beiden. **Eine Abschrift kann nicht
 * bemerken, dass ihr ein Eintrag fehlt.** Dieselbe Klasse wie #134
 * (Protokoll-Nummernraum), #138 (Spielseite) und #159 (`FACTOR_NAMES`).
 *
 * Deshalb wird hier nichts aufgezählt: Das Muster kommt aus dem Workflow, die
 * Pfade kommen aus den Rust-Quellen, und geprüft wird nur, dass das eine das
 * andere abdeckt.
 *
 * # Warum `import.meta.glob` und nicht `node:fs`
 *
 * Das Projekt führt `@types/node` nicht. Ein Prüffall, der dafür eine
 * Abhängigkeit nachzieht oder mit `@ts-expect-error` am Übersetzer vorbeigeht,
 * wäre teuer bezahlt — Letzteres ist genau die Abschaltung, die in #167 den
 * Schaden angerichtet hat. Vite liest die Dateien roh ein, ist bereits
 * typisiert (`types: ['vite/client']`) und liefert die Pfade als Schlüssel.
 *
 * # Warum ein TypeScript-Prüffall und kein eigener CI-Schritt
 *
 * Der `unit`-Job läuft bei jeder Änderung außer reiner Dokumentation, also
 * auch bei jeder `.rs`- und jeder Workflow-Änderung. Ein eigener Schritt
 * kostete Gate-Zeit und liefe genau dann, wenn dieser hier ohnehin läuft.
 */

/** Die Rust-Quellen, roh. Schlüssel sind Pfade ab der Projektwurzel. */
const RUST_QUELLEN = import.meta.glob('/sim/**/*.rs', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/**
 * Das Muster der `sim`-Stufe, gelesen aus dem Workflow.
 *
 * **Nicht abgeschrieben.** Wird es dort geändert, ändert sich diese Prüfung
 * mit — sonst prüfte sie eine Fassung, die nicht mehr läuft.
 */
function simMuster(): RegExp {
  expect(ciYaml, 'ci.yml ist nicht lesbar — dann belegt diese Datei nichts').toBeTypeOf('string');
  const treffer = [...ciYaml.matchAll(/echo "sim=\$\(match '([^']+)'\)"/g)];
  // **Ohne diese Zusicherung prüft die Datei nichts.** Findet die Suche das
  // Muster nicht mehr — weil jemand den Workflow umformatiert hat —, wäre
  // `treffer` leer, und ein Prüffall, der über eine leere Liste läuft, ist
  // grün, ohne etwas gesehen zu haben.
  expect(
    treffer,
    `im Workflow steht keine eindeutige sim-Filterzeile mehr (gefunden: ${treffer.length}) — ` +
      'dann kann diese Prüfung das Muster nicht lesen und belegt nichts',
  ).toHaveLength(1);
  return new RegExp(treffer[0]![1]!);
}

/**
 * Löst einen Pfad mit `../`-Anteilen auf. Ohne `node:path`, siehe oben.
 *
 * Die Pfade im Makro sind **relativ zur einbindenden Datei** — `sim/src/apl.rs`
 * mit `../apl/x` meint `sim/apl/x`, `sim/tests/y.rs` mit `../../tests/z` meint
 * `tests/z`. Wer das nicht auflöst, vergleicht Zeichenketten und irrt sich
 * genau dort, wo es darauf ankommt.
 */
function aufloesen(verzeichnis: string, relativ: string): string {
  const teile = verzeichnis.split('/').filter(Boolean);
  for (const stueck of relativ.split('/')) {
    if (stueck === '..') teile.pop();
    else if (stueck !== '.' && stueck !== '') teile.push(stueck);
  }
  return teile.join('/');
}

/**
 * Der Abschnitt eines Jobs, roh — von seiner Kopfzeile bis zum nächsten Job.
 *
 * Text statt YAML-Baum: Das Projekt führt keinen YAML-Leser. Die Jobs stehen in
 * `ci.yml` als Schlüssel mit genau zwei Leerzeichen Einzug, ihre Schritte
 * tiefer; die Trennkommentare zwischen den Jobs beginnen mit `#`.
 */
function jobAbschnitt(name: string): string {
  const zeilen = ciYaml.split('\n');
  const anfang = zeilen.findIndex((z) => z === `  ${name}:`);
  expect(
    anfang,
    `im Workflow steht kein Job "${name}" mehr — dann belegt diese Prüfung nichts`,
  ).toBeGreaterThan(-1);
  const rest = zeilen.slice(anfang + 1);
  const ende = rest.findIndex((z) => /^ {2}[a-z][\w-]*:\s*$/.test(z));
  return (ende === -1 ? rest : rest.slice(0, ende)).join('\n');
}

/**
 * Die Zeilen eines Abschnitts, die GitHub ausführt: ohne Kommentarzeilen.
 *
 * **Eine Regel für alle Leser dieser Abschnitte.** Ein auskommentierter Schritt
 * läuft nicht; wer ihn als Text findet, hält ihn für angeschlossen. Bis #892
 * filterte nur die Werkzeugsuche Kommentare heraus, die Prüfung des Schritts
 * nicht — und blieb grün, als der Schritt auskommentiert war.
 */
function ohneKommentarzeilen(abschnitt: string): string[] {
  return abschnitt.split('\n').filter((zeile) => !/^\s*#/.test(zeile));
}

/** Die Zeilen eines Jobs, die GitHub ausführt. */
function jobZeilen(name: string): string[] {
  return ohneKommentarzeilen(jobAbschnitt(name));
}

/** Der Abschnitt des `sim`-Jobs, roh. */
function simJob(): string {
  const abschnitt = jobAbschnitt('sim');
  // **Zusicherung in den Datensatz.** Träfe die Suche den falschen Abschnitt,
  // fände sie dort keine Werkzeuge, und der Fall darunter wäre grün.
  expect(abschnitt, 'der gelesene Abschnitt ist nicht der Rust-Job').toContain('cargo test');
  return abschnitt;
}

/** Die Zeilen des `sim`-Jobs, die GitHub ausführt. */
function simZeilen(): string[] {
  return ohneKommentarzeilen(simJob());
}

/**
 * Die Schlüssel auf der obersten Ebene eines Jobs (`needs`, `if`, `steps` …):
 * vier Leerzeichen Einzug. Tiefer Eingerücktes gehört zu ihrem Wert.
 */
function jobSchluessel(name: string): { name: string; wert: string }[] {
  return jobZeilen(name).flatMap((zeile) => {
    const treffer = /^ {4}([\w-]+):(.*)$/.exec(zeile);
    return treffer ? [{ name: treffer[1]!, wert: treffer[2]!.trim() }] : [];
  });
}

/** Die Unterschlüssel eines Job-Schlüssels mit Block-Wert, etwa `outputs:` — sechs Leerzeichen Einzug. */
function unterSchluessel(job: string, schluessel: string): { name: string; wert: string }[] {
  const zeilen = jobZeilen(job);
  const anfang = zeilen.findIndex((z) => z === `    ${schluessel}:`);
  if (anfang === -1) return [];
  const rest = zeilen.slice(anfang + 1);
  const ende = rest.findIndex((z) => z.trim() !== '' && !/^ {6}/.test(z));
  return (ende === -1 ? rest : rest.slice(0, ende)).flatMap((zeile) => {
    const treffer = /^ {6}([\w-]+):(.*)$/.exec(zeile);
    return treffer ? [{ name: treffer[1]!, wert: treffer[2]!.trim() }] : [];
  });
}

/** Die Einträge eines `needs`-Werts, ob als einzelner Name oder als `[a, b]`. */
function needsListe(wert: string): string[] {
  const innen = /^\[(.*)\]$/.exec(wert)?.[1] ?? wert;
  return innen
    .split(',')
    .map((teil) => teil.trim())
    .filter((teil) => teil !== '');
}

/** Die Werkzeuge unter `tools/`, die der `sim`-Job selbst aufruft — Kommentare zählen nicht. */
function simWerkzeuge(): string[] {
  const gefunden = new Set<string>();
  for (const zeile of simZeilen()) {
    for (const treffer of zeile.matchAll(/\btools\/[\w./-]+\.(?:py|sh|mjs)\b/g)) {
      gefunden.add(treffer[0]);
    }
  }
  return [...gefunden].sort();
}

/**
 * Die Schritte eines Jobs, je als Block ihrer Zeilen: von `- ` bis vor den
 * nächsten Schritt.
 *
 * Text statt YAML-Baum, siehe `jobAbschnitt()`. Die Schritte stehen mit sechs
 * Leerzeichen Einzug und `- `, ihre Schlüssel mit acht.
 */
function schritteAus(zeilen: string[]): string[][] {
  const schritte: string[][] = [];
  for (const zeile of zeilen) {
    if (zeile.trim() === '') continue;
    if (/^ {6}- /.test(zeile)) schritte.push([zeile]);
    else if (schritte.length > 0) schritte[schritte.length - 1]!.push(zeile);
  }
  return schritte;
}

/** Die Schritte des `sim`-Jobs. */
function simSchritte(): string[][] {
  return schritteAus(simZeilen());
}

/** Schlüssel und Wert je Schlüsselzeile eines Schritts — tiefer Eingerücktes ist Wert, kein Schlüssel. */
function schluessel(schritt: string[]): { name: string; wert: string }[] {
  return schritt.flatMap((zeile) => {
    const treffer = /^(?: {6}- | {8})([\w-]+):(.*)$/.exec(zeile);
    return treffer ? [{ name: treffer[1]!, wert: treffer[2]!.trim() }] : [];
  });
}

/**
 * Schlüssel, die einen Schritt weder überspringen noch seinen Ausgang
 * verschlucken können. **Eine Zulassung, keine Sperrliste:** `if` und
 * `continue-on-error` schalten ab, eine eigene `shell` kann den Ausgang
 * verschlucken — und eine Sperrliste fängt nur, woran ihr Verfasser gedacht
 * hat. Ein neuer Schlüssel, der nichts davon kann, gehört hier dazu.
 */
const HARMLOSE_SCHLUESSEL = new Set(['name', 'id', 'run', 'timeout-minutes']);

/**
 * Dieselbe Zulassung eine Ebene höher, für den Kopf des `sim`-Jobs. `if` steht
 * darin, weil sein Wert eigens und wörtlich geprüft wird; `continue-on-error`
 * fehlt, weil es das Scheitern des ganzen Jobs durchgehen lässt.
 */
const HARMLOSE_JOB_SCHLUESSEL = new Set(['needs', 'if', 'runs-on', 'steps', 'timeout-minutes']);

/** Was die Rust-Quellen per `include_str!` einbinden, als Pfade ab der Wurzel. */
function eingebundeneDateien(): string[] {
  const gefunden = new Set<string>();
  for (const [datei, quelle] of Object.entries(RUST_QUELLEN)) {
    const verzeichnis = datei.slice(0, datei.lastIndexOf('/'));
    for (const treffer of quelle.matchAll(/include_str!\("([^"]+)"\)/g)) {
      gefunden.add(aufloesen(verzeichnis, treffer[1]!));
    }
  }
  return [...gefunden].sort();
}

describe('Der Pfad-Filter der sim-Stufe (#173)', () => {
  it('deckt jede Datei ab, die die Rust-Tests einlesen', () => {
    const muster = simMuster();
    const dateien = eingebundeneDateien();

    // **Die Ursache zuerst.** Fände die Suche keine einzige `include_str!`-
    // Zeile — weil sich das Makro oder die Verzeichnisstruktur geändert hat —,
    // liefe die Schleife unten nicht und der Fall bliebe grün.
    expect(
      dateien.length,
      'in sim/ steht kein einziges include_str! mehr — dann sucht diese ' +
        'Prüfung an der falschen Stelle und belegt nichts',
    ).toBeGreaterThan(5);

    const fehlend = dateien.filter((d) => !muster.test(d));
    expect(
      fehlend,
      'diese Dateien werden von den Rust-Tests gelesen, fallen aber durch den ' +
        'sim-Filter — eine Änderung an ihnen ließe die Rust-Tests nicht laufen:\n  ' +
        fehlend.join('\n  '),
    ).toEqual([]);
  });

  it('lässt reine Dokumentation weiterhin durchfallen', () => {
    // **Der Gegenfall, und er ist der Grund für den Filter überhaupt.** Eine
    // Lösung, die einfach immer läuft, deckt alles ab und zerstört den Zweck:
    // Eine Korrektur an einer Markdown-Datei zahlte dann Rust-Bauzeit.
    const muster = simMuster();
    for (const doku of ['docs/GAME-DESIGN.md', 'docs/WORKER-GUIDE.md', 'README.md', 'CLAUDE.md']) {
      expect(muster.test(doku), `${doku} löst die sim-Stufe aus`).toBe(false);
    }
  });

  it('fasst eine neu erzeugte Vergleichsdatei als Rust-Eingabe auf', () => {
    // Der Fall aus dem Befund: Vergleichsdateien werden nach jeder
    // Regeländerung neu erzeugt, an einem einzigen Tag mehrfach.
    const muster = simMuster();
    expect(muster.test('tests/parity/recordings/fixation.json')).toBe(true);
    expect(muster.test('tests/parity/recordings/log-parity.json')).toBe(true);
  });

  it('fasst den Compiler-Pin als Rust-Eingabe auf', () => {
    // Seit cargo remote v2 (2026-10-23) bestimmt rust-toolchain.toml den
    // Compiler der sim-Stufe. Ein Versionswechsel, der die Stufe nicht
    // auslöst, wäre ein grüner PR für einen Compiler, auf dem nie geprüft
    // wurde. Der Anker zählt: Eine gleichnamige Datei tiefer im Baum ist nicht
    // der Pin des Workspace.
    const muster = simMuster();
    expect(muster.test('rust-toolchain.toml')).toBe(true);
    expect(muster.test('docs/rust-toolchain.toml.md')).toBe(false);
  });

  it('fasst jedes Werkzeug, das die sim-Stufe selbst ausführt, als Eingabe auf (#892)', () => {
    // Dieselbe Klasse wie `ci.yml` im Muster, eine Ebene tiefer: Ein Werkzeug,
    // das der sim-Job ausführt, IST Teil der Prüfung. Fällt es durch den Filter,
    // ändert jemand die Prüfung, die Stufe läuft nicht, und ci-pass meldet grün.
    // Die Liste kommt aus dem Job selbst, nicht aus einer Abschrift.
    const muster = simMuster();
    const werkzeuge = simWerkzeuge();
    expect(
      werkzeuge.length,
      'der sim-Job ruft kein einziges Werkzeug unter tools/ auf — dann sucht diese Prüfung an der falschen Stelle',
    ).toBeGreaterThan(0);
    const fehlend = werkzeuge.filter((w) => !muster.test(w));
    expect(
      fehlend,
      'diese Werkzeuge fährt der sim-Job selbst, sie fallen aber durch den sim-Filter — ' +
        'eine Änderung an ihnen ließe genau die Stufe nicht laufen, die sie benutzt:\n  ' +
        fehlend.join('\n  '),
    ).toEqual([]);
  });

  it('führt die Baubruch-Gegenprobe der Wirkungsbeschreibung im sim-Job aus (#892)', () => {
    // Die Abnahme aus Epic #892: Ein neuer Effekt-Baustein bricht den Bau an
    // jeder registrierten Analyse — nachgewiesen bei JEDER sim-Änderung mit dem
    // echten Übersetzer, nicht einmal im PR-Text. Ohne diesen Schritt wäre das
    // Werkzeug eine Leitung ohne Anschluss.
    //
    // **Angeschlossen heißt: Er läuft, und sein Scheitern bricht den Job.** Ein
    // Text im Abschnitt belegt das nicht. Die erste Fassung dieses Falls suchte
    // nur die Befehlszeile und blieb grün, als der Schritt auskommentiert, mit
    // `if: false` übersprungen, mit `continue-on-error: true` entschärft oder
    // sein Ausgang mit `|| true` verschluckt war (gemessen bei der Prüfung von
    // PR #1136). Ein Abschalten des GANZEN Jobs fängt dieser Fall nicht — es
    // träfe jeden Schritt der Stufe gleich —, das tun die beiden Fälle darunter.
    const NAME = 'Wirkungs-Baubruch-Gegenprobe';
    const BEFEHL = 'python3 -B tools/wirkung-baubruch-gegenprobe.py --befehl cargo';
    const schritte = simSchritte();
    // **Zusicherung in den Datensatz.** Zerlegte die Suche den Job falsch, fände
    // sie den Schritt nicht und meldete „fehlt“ statt „falsch gelesen“.
    expect(
      schritte.filter((s) =>
        schluessel(s).some((k) => k.name === 'run' && k.wert === 'cargo test'),
      ),
      'die Zerlegung in Schritte findet nicht einmal `run: cargo test` — dann liest sie den Job falsch',
    ).toHaveLength(1);

    const treffer = schritte.filter((s) =>
      schluessel(s).some((k) => k.name === 'name' && k.wert === NAME),
    );
    expect(
      treffer.length,
      `im sim-Job steht der Schritt „${NAME}“ ${treffer.length}-mal (Kommentarzeilen nicht ` +
        'mitgezählt), erwartet genau einmal — auskommentiert läuft er nicht, und bei zweien ' +
        'belegt dieser Fall nicht, welcher läuft',
    ).toBe(1);
    const schritt = schluessel(treffer[0]!);

    const unzulaessig = schritt.map((k) => k.name).filter((k) => !HARMLOSE_SCHLUESSEL.has(k));
    expect(
      unzulaessig,
      `„${NAME}“ trägt Schlüssel außerhalb der Zulassung — \`if\` und \`continue-on-error\` ` +
        'schalten ihn ab, `shell` kann sein Scheitern verschlucken; zugelassen sind nur ' +
        `${[...HARMLOSE_SCHLUESSEL].join(', ')}. Ein Schlüssel, der nichts davon kann, gehört ` +
        'in HARMLOSE_SCHLUESSEL',
    ).toEqual([]);

    // Der Wert verankert bis zum Zeilenende: `… --befehl cargo || true` ist ein
    // anderer Befehl, und ein Block-Skalar (`run: |`) könnte einen solchen Zusatz tragen.
    const befehle = schritt.filter((k) => k.name === 'run').map((k) => k.wert);
    expect(
      befehle,
      `„${NAME}“ muss genau \`${BEFEHL}\` ausführen, als eine Zeile ohne Zusatz — ` +
        `gefunden: ${JSON.stringify(befehle)}`,
    ).toEqual([BEFEHL]);
  });

  it('bindet den ganzen sim-Job an den sim-Ausgang des Pfadfilters (#892)', () => {
    // Der Fall darüber hält den Schritt am Laufen — aber nur, wenn der Job selbst
    // läuft. Mit `if: false` am Job blieben alle Fälle dieser Datei grün, und
    // `ci-pass` wertet `skipped` als bestanden (gemessen bei der Prüfung von
    // PR #1136). Ein abgeschalteter Job sieht dort aus wie eine Änderung, die
    // Rust nicht berührt.
    //
    // Die Kette hat drei Glieder, und jedes wird hier geprüft: die Bedingung des
    // Jobs, der Ausgang `sim` des Jobs `changes` und der Filterschritt, der ihn
    // schreibt. Bricht eines, läuft die Stufe nie oder immer — und „nie“ ist von
    // „nichts zu tun“ nicht zu unterscheiden.
    const kopf = jobSchluessel('sim');
    // **Zusicherung in den Datensatz.** Läse die Zerlegung den Kopf falsch, fände
    // sie kein `if` und meldete „fehlt“ statt „falsch gelesen“.
    expect(
      kopf.map((k) => k.name),
      'im Kopf des sim-Jobs steht kein `steps:` — dann liest diese Zerlegung den Job falsch',
    ).toContain('steps');

    const unzulaessig = kopf.map((k) => k.name).filter((k) => !HARMLOSE_JOB_SCHLUESSEL.has(k));
    expect(
      unzulaessig,
      'der sim-Job trägt Schlüssel außerhalb der Zulassung — `continue-on-error` lässt sein ' +
        'Scheitern durchgehen; zugelassen sind nur ' +
        `${[...HARMLOSE_JOB_SCHLUESSEL].join(', ')}. Ein Schlüssel, der den Job weder ` +
        'überspringen noch sein Scheitern verschlucken kann, gehört in HARMLOSE_JOB_SCHLUESSEL',
    ).toEqual([]);

    // Genau diese Bedingung und keine andere: `false`, `always()`, ein Zusatz
    // `&& …` oder der Ausgang eines anderen Filters lösen den Job vom sim-Muster.
    expect(
      kopf.filter((k) => k.name === 'if').map((k) => k.wert),
      'der sim-Job muss genau an `needs.changes.outputs.sim` hängen — sonst läuft er nie, ' +
        'immer oder nach dem falschen Muster',
    ).toEqual(["needs.changes.outputs.sim == 'true'"]);
    const needs = kopf.filter((k) => k.name === 'needs').map((k) => needsListe(k.wert));
    expect(
      needs.length === 1 && needs[0]!.includes('changes'),
      `der sim-Job muss auf \`changes\` warten, sonst ist sein \`needs.changes\` leer — gefunden: ${JSON.stringify(needs)}`,
    ).toBe(true);

    expect(
      unterSchluessel('changes', 'outputs')
        .filter((k) => k.name === 'sim')
        .map((k) => k.wert),
      'der Ausgang `sim` des Jobs `changes` muss aus dem Filterschritt kommen',
    ).toEqual(['${{ steps.filter.outputs.sim }}']);
    const filter = schritteAus(jobZeilen('changes')).filter((s) =>
      schluessel(s).some((k) => k.name === 'id' && k.wert === 'filter'),
    );
    expect(filter, 'im Job `changes` steht kein Schritt `id: filter` genau einmal').toHaveLength(1);
    const text = filter[0]!.join('\n');
    expect(
      text,
      'der Filterschritt schreibt die sim-Zeile nicht — das Muster, das `simMuster()` liest, ' +
        'stünde dann woanders und entschiede nichts',
    ).toMatch(/^\s*echo "sim=\$\(match '[^']+'\)"$/m);
    expect(text, 'der Filterschritt schreibt nicht nach $GITHUB_OUTPUT').toMatch(
      /^\s*\} >> "\$GITHUB_OUTPUT"$/m,
    );
  });

  it('lässt ci-pass auf den sim-Job warten und sein Ergebnis werten (#892)', () => {
    // Das andere Ende derselben Kette: Wartet `ci-pass` nicht auf den sim-Job
    // oder liest sein Ergebnis nicht, bricht ein roter sim-Job nichts — der
    // einzige Required Check bliebe grün.
    const kopf = jobSchluessel('ci-pass');
    const needs = kopf.filter((k) => k.name === 'needs').map((k) => needsListe(k.wert));
    expect(needs, 'ci-pass hat nicht genau einen `needs`-Schlüssel').toHaveLength(1);
    expect(needs[0], 'ci-pass wartet nicht auf den sim-Job').toContain('sim');
    expect(
      kopf.filter((k) => k.name === 'if').map((k) => k.wert),
      'ci-pass muss auch nach übersprungenen oder roten Jobs laufen',
    ).toEqual(['always()']);
    const ergebnis = jobZeilen('ci-pass').filter((z) =>
      /^\s*\[sim\]="\$\{\{ needs\.sim\.result \}\}"\s*$/.test(z),
    );
    expect(
      ergebnis,
      'ci-pass liest das Ergebnis des sim-Jobs nicht genau einmal — dann wertet es ihn nicht',
    ).toHaveLength(1);
  });
});
