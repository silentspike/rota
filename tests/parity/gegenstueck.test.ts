/**
 * Jede Aufzeichnung hat ein Rust-Gegenstück (Issue #533).
 *
 * # Was hier schiefgehen kann, ohne dass es jemand merkt
 *
 * Eine Aufzeichnung unter `recordings/` sieht nach Absicherung aus. Sie wird
 * exportiert, `freshness.test.ts` hält sie aktuell, `coverage.test.ts` zählt
 * die Zauber darin als gedeckt. Nur: **Wenn keine Rust-Datei sie liest,
 * vergleicht niemand die beiden Seiten.** Der ganze Aufwand erzeugt dann eine
 * gepflegte Datei und keine einzige Aussage über Bitgleichheit.
 *
 * Genau das war der Befund aus #456 — `synergy-window.json` stand seit #318
 * da und in keinem Frische-Fall. Beim Bau dieses Wächters fanden sich zwei
 * weitere: `fixation-spells` und `kern-zauber`.
 *
 * # Warum die Prüfung eine eigene Funktion ist
 *
 * `ungedeckte()` bekommt die beiden Listen als Argumente, statt sie selbst zu
 * lesen. Nur so lässt sich der Wächter mit erfundenen Daten füttern und
 * zeigen, dass er **rot werden kann** — ein Wächter, der immer grün ist, weil
 * er nichts findet, ist von einem funktionierenden nicht zu unterscheiden.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const HIER = new URL('.', import.meta.url).pathname;
const AUFZEICHNUNGEN = join(HIER, 'recordings');
const RUST_TESTS = join(HIER, '..', '..', 'sim', 'tests');

/**
 * Die Namen der Aufzeichnungen, die in keinem der Rust-Texte vorkommen.
 *
 * Gesucht wird nach dem Dateinamen, nicht nach `include_str!` — ein
 * Gegenstück darf die Datei auch anders einbinden, solange es sie nennt.
 *
 * **Der Schrägstrich davor ist nicht Kosmetik.** Ohne ihn gälte `lifetime`
 * als gedeckt, sobald irgendwo `dot-lifetime.json` steht: `includes` findet
 * den kürzeren Namen im längeren. Eine Aufzeichnung liefe dann als geprüft,
 * weil eine ANDERE ein Gegenstück hat — der Fehler, den dieser Wächter
 * gerade verhindern soll, nur eine Ebene tiefer.
 */
export function ungedeckte(namen: readonly string[], rustTexte: readonly string[]): string[] {
  return namen.filter((name) => !rustTexte.some((text) => text.includes(`/${name}.json`)));
}

const namen = readdirSync(AUFZEICHNUNGEN)
  .filter((d) => d.endsWith('.json'))
  .map((d) => d.replace('.json', ''));

const rustTexte = readdirSync(RUST_TESTS)
  .filter((d) => d.endsWith('.rs'))
  .map((d) => readFileSync(join(RUST_TESTS, d), 'utf8'));

describe('Jede Aufzeichnung wird von der Rust-Seite gelesen', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // **Die Zusicherung vor der Messung.** Läse das Verzeichnis leer — falscher
    // Pfad, umbenannter Ordner —, wäre der Fall darunter grün und prüfte
    // nichts. Genau die Klasse Fehler, die dieser Wächter aufdecken soll.
    expect(namen.length, 'keine Aufzeichnungen gefunden').toBeGreaterThan(30);
    expect(rustTexte.length, 'keine Rust-Testdateien gefunden').toBeGreaterThan(30);
  });

  it('keine Aufzeichnung steht ohne Gegenstück da', () => {
    expect(ungedeckte(namen, rustTexte)).toEqual([]);
  });
});

describe('Der Wächter kann rot werden — die Gegenprobe', () => {
  it('eine Aufzeichnung, die keine Rust-Datei nennt, wird gemeldet', () => {
    // Ohne diesen Fall bliebe offen, ob `ungedeckte()` überhaupt je etwas
    // zurückgibt oder nur zufällig immer die leere Liste liefert.
    expect(ungedeckte(['erfunden'], ['include_str!("…/cinder-ticks.json")'])).toEqual(['erfunden']);
  });

  it('und der echte Bestand fiele durch, wäre eine Datei leer', () => {
    // Derselbe Bestand, dem man die Rust-Seite wegnimmt: Jetzt müssen ALLE
    // gemeldet werden. Das belegt, dass der grüne Fall oben an den Rust-Texten
    // hängt und nicht an einer leeren Namensliste.
    expect(ungedeckte(namen, []).length).toBe(namen.length);
  });

  it('ein Namens-Ende zählt NICHT als Deckung', () => {
    // Der erste Entwurf dieses Falls behauptete das Gegenteil dessen, was der
    // Code tat, und war trotzdem grün: `includes('lifetime.json')` findet den
    // Namen in `dot-lifetime.json`. Erst der Schrägstrich trennt die beiden.
    expect(ungedeckte(['lifetime'], ['recordings/dot-lifetime.json'])).toEqual(['lifetime']);
    expect(ungedeckte(['dot-lifetime'], ['recordings/dot-lifetime.json'])).toEqual([]);
  });
});
