/**
 * Jeder ausgelieferte Boss kommt in einer Aufzeichnung vor.
 *
 * # Die Lücke, die es bis heute gab
 *
 * Zwei Wächter standen schon da, und beide gingen von der Aufzeichnung aus:
 *
 * ```text
 *   freshness.test.ts    Aufzeichnung -> ist sie noch aktuell?
 *   gegenstueck.test.ts  Aufzeichnung -> liest die Rust-Seite sie?
 * ```
 *
 * Eine Aufzeichnung, die **gar nicht erst entsteht**, fällt durch beide. Genau
 * das war bei der dritten Instanz der Fall: vier ausgelieferte Bosse, null
 * Aufzeichnungen, kein einziger roter Fall. Spiel und Simulator konnten bei
 * ihnen beliebig auseinanderlaufen.
 *
 * Aufgefallen ist es nicht durch eine Prüfung, sondern durch eine Mutation:
 * `energy.resetTo` von `boss-3-2` auf einen anderen Wert gesetzt — nachweislich
 * wirksam (drei Energie-Auslöser wurden fünf, der Spieler verlor 384
 * Lebenspunkte mehr) — und die volle Suite blieb mit 2332 grünen Fällen stumm.
 * Dieselbe Klasse Änderung an einem Boss der ersten Instanz schlug sofort an.
 *
 * # Warum dieser Wächter und nicht ein dritter vom selben Typ
 *
 * In `freshness.test.ts` steht der Satz: *„Derselbe Fehler zweimal auf dieselbe
 * Weise gefunden heißt: Der nächste wird nicht gefunden."* Er stammt aus dem
 * vierten Mal, bei dem eine Aufzeichnung ohne Frische-Fall zufällig auffiel.
 *
 * Dieser Wächter dreht deshalb die Richtung um: Er geht **vom Bestand aus**,
 * nicht von den Dateien. Ein neuer Boss — instance-4 ist geplant — bringt ihn
 * am Tag seiner Einführung zum Sprechen, und niemand muss daran denken.
 *
 * # Was er NICHT leistet
 *
 * Er prüft die Existenz, nicht die Güte. Eine Aufzeichnung, die den Boss zwar
 * fährt, seine Signatur aber nie auslöst, erfüllt ihn — dafür stehen die
 * Zusicherungen in den Rust-Dateien (`purge` genau zweimal, `quench` genau
 * einmal), die eine still verschobene Rotation melden.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const HIER = new URL('.', import.meta.url).pathname;
const AUFZEICHNUNGEN = join(HIER, 'recordings');
const DATEN = join(HIER, '..', '..', 'data');

/**
 * Die ausgelieferten Bosse ohne Aufzeichnung.
 *
 * **Als reine Funktion mit Argumenten**, damit sie sich mit erfundenen Daten
 * füttern lässt — ein Wächter, der immer grün ist, weil er nichts findet, ist
 * von einem funktionierenden nicht zu unterscheiden (dieselbe Begründung wie
 * bei `ungedeckte()` in `gegenstueck.test.ts`).
 */
export function ohneAufzeichnung(
  bosse: readonly string[],
  genannte: ReadonlySet<string>,
): string[] {
  return bosse.filter((id) => !genannte.has(id)).sort();
}

/**
 * Ausgeliefert heißt: Die Instanz des Bosses steht in `instances.json`.
 *
 * **Eine Positivliste, kein Namensmuster.** Ein Filter auf `test-` im
 * Bezeichner ginge, solange sich alle daran halten — und `boss-select.ts`
 * dokumentiert, dass genau das schiefging: Prüf-Bosse tragen ihre Instanz mal
 * als `testing`, mal als `test`. Wer nach Namen filtert, übersieht die nächste
 * Schreibweise.
 */
function ausgelieferteBosse(): string[] {
  const instanzen = new Set(
    (
      JSON.parse(readFileSync(join(DATEN, 'instances.json'), 'utf8')) as {
        instances: { id: string }[];
      }
    ).instances.map((i) => i.id),
  );
  const out: string[] = [];
  for (const datei of readdirSync(join(DATEN, 'bosses')).filter((d) => d.endsWith('.json'))) {
    const boss = JSON.parse(readFileSync(join(DATEN, 'bosses', datei), 'utf8')) as {
      id: string;
      instance: string;
    };
    if (instanzen.has(boss.instance)) out.push(boss.id);
  }
  return out.sort();
}

/** Welche Boss-Kennungen in irgendeiner Aufzeichnung stehen. */
function genannteBosse(): Set<string> {
  const out = new Set<string>();
  for (const datei of readdirSync(AUFZEICHNUNGEN).filter((d) => d.endsWith('.json'))) {
    const inhalt = JSON.parse(readFileSync(join(AUFZEICHNUNGEN, datei), 'utf8')) as unknown;
    if (typeof inhalt !== 'object' || inhalt === null) continue;
    const id = (inhalt as Record<string, unknown>)['bossId'];
    if (typeof id === 'string') out.add(id);
  }
  return out;
}

describe('Jeder ausgelieferte Boss steht in einer Aufzeichnung', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // **Die Zusicherung vor der Messung.** Läse eine der beiden Seiten leer —
    // falscher Pfad, umbenannter Ordner —, wäre der Fall darunter grün und
    // prüfte nichts.
    expect(ausgelieferteBosse().length, 'keine ausgelieferten Bosse gefunden').toBeGreaterThan(10);
    expect(genannteBosse().size, 'keine Aufzeichnung nennt einen Boss').toBeGreaterThan(10);
  });

  it('keiner steht ohne da', () => {
    const fehlen = ohneAufzeichnung(ausgelieferteBosse(), genannteBosse());

    expect(
      fehlen,
      `Diese Bosse werden ausgeliefert, kommen aber in keiner Aufzeichnung vor. ` +
        `Spiel und Simulator werden bei ihnen NICHT verglichen — eine Änderung an ` +
        `ihren Daten oder an der Mechanik, die sie tragen, bleibt unbemerkt.\n\n` +
        `  Aufzeichnung nach dem Muster von \`instance3Payload\` in payloads.ts ergänzen,\n` +
        `  in export-parity.mjs schreiben lassen, Frische-Fall und Rust-Gegenstück nachziehen.\n\n` +
        `Geprüft wurden ${ausgelieferteBosse().length} ausgelieferte Bosse gegen ` +
        `${genannteBosse().size} in Aufzeichnungen genannte.`,
    ).toEqual([]);
  });

  it('und der Wächter kann rot werden', () => {
    // Ohne diesen Fall belegte der obige nur, dass heute nichts fehlt — nicht,
    // dass ein Fehlen auffiele.
    expect(ohneAufzeichnung(['boss-9-9', 'boss-1-1'], new Set(['boss-1-1']))).toEqual(['boss-9-9']);
    expect(ohneAufzeichnung(['boss-1-1'], new Set(['boss-1-1']))).toEqual([]);
  });
});
