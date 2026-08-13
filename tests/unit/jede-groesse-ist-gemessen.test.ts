import { describe, expect, it } from 'vitest';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Der Wächter zu Issue #547, AC5 — **er prüft die Vollständigkeit der
 * MESSUNG, nicht den Schutz.**
 *
 * # Was er bewacht
 *
 * `docs/groessen-abdeckung-messung.txt` sagt für jedes Paar (Zauber, Größe),
 * ob eine Änderung seines Wertes irgendeine Prüfung auslöst. Die Messung ist
 * teuer — 68 volle Suiten-Läufe, gut vierzig Minuten — und sie veraltet in dem
 * Moment, in dem jemand einen Zauber hinzufügt.
 *
 * Dieser Fall meldet genau das: **ein Paar, über das niemand etwas weiß.**
 *
 * # Warum nicht der naheliegende Wächter
 *
 * Das Kriterium schlug vor, ein `buff.factor` zu melden, das in keinem
 * Prüffall mit seinem Wert vorkommt. Gemessen am 2026-08-07 wären das **45 von
 * 68 Paaren** — alle echt ungeschützt, kein einziges falsch positiv. Ein
 * Wächter, der 45 Ausnahmen mitführt, ist aber keine Prüfung mehr, sondern
 * eine zweite Liste neben der ersten. Und er wäre an dem Tag rot, an dem
 * jemand eine der 45 behebt, ohne die Ausnahmeliste zu pflegen.
 *
 * Die Auflage aus dem Kriterium lautete: *Ein Wächter, der 13 Falsch-Positive
 * liefert, ist keiner.* Der hier liefert keine — er vergleicht zwei Mengen,
 * die beide im Repository stehen.
 *
 * # Was er NICHT leistet
 *
 * Er misst nichts. Er stellt fest, dass etwas **ungemessen** ist.
 *
 * **Ein Eintrag „NUR FRISCHE" erfüllt ihn genauso wie „geschützt".** Das Paar
 * ist dann gemessen UND ungeschützt — der Wächter ist zufrieden, weil die
 * Lücke bekannt und aufgeschrieben ist.
 *
 * Deshalb heißt die Datei nicht `groessen-abdeckung`: Wer den Namen liest,
 * schlösse „die Größen sind abgedeckt", und das wäre genau die Sorte
 * Täuschung, gegen die dieses Issue angetreten ist — ein Name, der Mechanik
 * verspricht, die dahinter nicht steht.
 *
 * Das ist Absicht. Die 45 Lücken zu schließen ist Arbeit für mehrere Issues;
 * zu verhindern, dass unbemerkt eine sechsundvierzigste entsteht, ist dieser
 * Fall.
 */

const MESSUNG = 'docs/groessen-abdeckung-messung.txt';
const SPELL_DIR = 'data/spells';

/**
 * Alle Zahlenfelder unter einem Unterknoten — dieselbe Regel wie im Werkzeug.
 *
 * **Aus den JSON-Dateien, nicht aus `data.ts`.** Der Wächter vergleicht eine
 * Messung über die Datendateien mit ebendiesen Dateien; ginge er über die
 * geladene Struktur, vergliche er zwei verschiedene Dinge und wäre bei jeder
 * Umbenennung im Ladepfad grundlos rot.
 */
function gemesseneGroessen(): Set<string> {
  const paare = new Set<string>();
  for (const datei of readdirSync(SPELL_DIR).filter((d) => d.endsWith('.json'))) {
    const inhalt = JSON.parse(readFileSync(`${SPELL_DIR}/${datei}`, 'utf8')) as {
      spells: Record<string, unknown>[];
    };
    for (const zauber of inhalt.spells) {
      if (((zauber['unlockLevel'] as number) ?? 0) > 100) continue; // Prüf-Zauber
      for (const [gruppe, knoten] of Object.entries(zauber)) {
        if (typeof knoten !== 'object' || knoten === null || Array.isArray(knoten)) continue;
        for (const [feld, wert] of Object.entries(knoten as Record<string, unknown>)) {
          // **Dieselbe Regel wie `messbar()` in `tools/groessen-mutation.py`.**
          // Bis #633 stand hier `typeof wert !== 'number'` — Werkzeug und
          // Wächter hatten damit zwei Begriffe von „Größe", und der des
          // Wächters war der engere. Er hätte jede Textmessung als „Größe, die
          // es nicht mehr gibt" gemeldet; genau das ist beim ersten Lauf
          // passiert, mit 22 Zeilen auf einmal.
          //
          // `_comment` bleibt draußen: Freitext für Leser, ohne Wirkung.
          if (feld.startsWith('_')) continue;
          if (!['number', 'string', 'boolean'].includes(typeof wert)) continue;
          paare.add(`${gruppe}.${feld} ${zauber['id'] as string}`);
        }
      }
    }
  }
  return paare;
}

function ausDerDatei(): Set<string> {
  const zeilen = readFileSync(MESSUNG, 'utf8').split('\n');
  const paare = new Set<string>();
  for (const zeile of zeilen) {
    const teile = zeile.trim().split(/\s+/);
    if (teile.length < 2 || zeile.startsWith('#')) continue;
    paare.add(`${teile[0]} ${teile[1]}`);
  }
  return paare;
}

describe('AC5 — keine Größe ohne Messung', () => {
  /**
   * **Die Zusicherung vor der Messung.** Fände das Einlesen nichts, wäre der
   * Fall unten grün, weil beide Mengen leer sind — und niemand merkte, dass
   * der Wächter gar nichts vergleicht.
   */
  it('die Messdatei ist lesbar und nicht leer', () => {
    // **Der Umfang gehört in die Ausgabe.** Bei der Abnahme hat der
    // Orchestrator einen Prüf-Zauber (`unlockLevel: 101`) angehängt, drei
    // grüne Fälle bekommen und beinahe „AC5 nicht erfüllt" gemeldet. Der
    // Filter unten ist Absicht — aber der Wächter sagte nirgends, dass er
    // filtert, und wie viel.
    //
    // Ein Wächter, dessen geprüfte Menge unsichtbar ist, lädt genau zu dieser
    // Fehldeutung ein: Sein Schweigen sieht aus wie Blindheit.
    // `console.log` taugt dafür nicht: vitest unterdrückt es bei grünen
    // Fällen — gemessen, der erste Entwurf gab nichts aus. Die Zahl steht
    // deshalb in den FEHLERMELDUNGEN, denn dort sieht sie, wer sie braucht.

    const gemessen = ausDerDatei();
    expect(
      gemessen.size,
      `${MESSUNG} liefert keine Paare — der Wächter misst nichts`,
    ).toBeGreaterThan(50);
    expect(
      gemesseneGroessen().size,
      `Kein Größen-Feld im Bestand gefunden. Der Wächter überspringt Prüf-Zauber ` +
        `(unlockLevel > 100) — wer einen solchen anhängt und Schweigen erwartet, ` +
        `misst den Filter statt den Wächter.`,
    ).toBeGreaterThan(50);
  });

  it('jede Größe im Zauber-Bestand steht in der Messung', () => {
    const fehlen = [...gemesseneGroessen()].filter((p) => !ausDerDatei().has(p)).sort();

    expect(
      fehlen,
      `Diese Größen sind nie gemessen worden — es ist unbekannt, ob eine Änderung ` +
        `ihres Wertes irgendeine Prüfung auslöst.\n\n` +
        `  python3 tools/groessen-mutation.py 0 10   (in Blöcken, siehe Dateikopf)\n\n` +
        `Das Ergebnis gehört in ${MESSUNG}. Ohne die Messung ist der Wert dieser ` +
        `Größen so ungeschützt wie SLAGs Anhebung vor #547 — nur weiß es dann niemand.\n\n` +
        `Geprüft wurden ${gemesseneGroessen().size} Paare; Prüf-Zauber ` +
        `(unlockLevel > 100) sind bewusst ausgenommen.`,
    ).toEqual([]);
  });

  it('die Messung nennt keine Größen, die es nicht mehr gibt', () => {
    // Die Gegenrichtung, damit die Datei nicht zur Halde wird: Ein
    // entfernter Zauber hinterlässt sonst eine Zeile, die für immer behauptet,
    // etwas sei gemessen.
    const verwaist = [...ausDerDatei()].filter((p) => !gemesseneGroessen().has(p)).sort();

    expect(
      verwaist,
      `Diese Zeilen in ${MESSUNG} gehören zu Größen, die es im Bestand nicht ` +
        `mehr gibt. Sie gehören entfernt — eine Messung ohne Gegenstück sieht aus ` +
        `wie Abdeckung und ist keine.`,
    ).toEqual([]);
  });

  /**
   * Die dritte Richtung: Existieren die Prüffälle, auf die sich die Urteile
   * berufen?
   *
   * # Was dieser Fall fängt — und was ausdrücklich nicht
   *
   * Ein Urteil lautet `geschuetzt [2 Prueffaelle] (pin-kontrolle.test.ts)`.
   * Wird diese Datei gelöscht oder umbenannt, behauptet die Zeile weiter
   * Schutz, und niemand merkt es: Die beiden Fälle oben prüfen nur, ob das
   * **Paar** dasteht, nicht ob sein Urteil noch trägt.
   *
   * **Er fängt NICHT, dass ein Urteil inhaltlich veraltet.** Am 2026-08-11
   * standen zwanzig Zeilen auf `NUR FRISCHE`, obwohl sie längst gedeckt
   * waren — jede nannte gar keine Datei, also hätte dieser Fall geschwiegen.
   * Dafür gibt es nur einen echten Weg, und er ist teuer: den vollen
   * Mutationslauf über alle Paare (`tools/groessen-mutation.py`), rund eine
   * Viertelstunde mit `--bail`. Das gehört nicht in jeden CI-Lauf, und die
   * Frage, wohin dann, ist keine, die ein Prüffall beantwortet.
   *
   * Diese Grenze steht hier, damit niemand den billigen Wächter für den
   * teuren hält.
   */
  it('jede genannte Prüffall-Datei gibt es wirklich', () => {
    const text = readFileSync(MESSUNG, 'utf8');
    const genannt = [...new Set([...text.matchAll(/([a-z0-9-]+\.test\.ts)/g)].map((t) => t[1]))];

    // **Die Zusicherung zuerst.** Findet das Muster nichts — geändertes
    // Format, umbenannte Endung —, wäre die Prüfung darunter trivial erfüllt.
    expect(genannt.length, `${MESSUNG} nennt keine einzige Prüffall-Datei`).toBeGreaterThan(20);

    const fehlend = genannt
      .filter(
        (name) =>
          !existsSync(join('tests/unit', name as string)) &&
          !existsSync(join('tests/parity', name as string)),
      )
      .sort();

    expect(
      fehlend,
      `Diese Prüffälle werden in ${MESSUNG} als Schutz genannt, es gibt sie aber ` +
        `nicht mehr: ${fehlend.join(', ')}\n\n` +
        `Eine Zeile, die auf eine gelöschte Datei zeigt, behauptet Schutz, den ` +
        `niemand mehr leistet.`,
    ).toEqual([]);
  });
});
