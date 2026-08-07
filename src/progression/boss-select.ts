/**
 * Die Boss-Wahl (GAME-DESIGN 11.5/11.6, Issue #613).
 *
 * # Warum ein Bereich und kein eigener Bildschirm
 *
 * `player-home.ts` nennt die Boss-Wahl in seinem Kopfkommentar ausdrücklich
 * als künftigen Bereich, mit Verweis auf 11.6 — neben Talentbaum,
 * Zusammenstellung und Ausrüstung. Die letzten beiden stehen heute schon so
 * darin.
 *
 * # Wie das mit „fasst den Kampf nicht an" zusammengeht
 *
 * Dieser Bereich fasst ihn auch nicht an. Er meldet über `onSelect` nach oben,
 * **welchen** Boss der Spieler will; was daraufhin geschieht, entscheidet
 * `app.ts`. Das ist derselbe Weg, den `onClose` des Player Home schon geht:
 * „Der Bildschirm entscheidet nicht selbst, wohin — das weiß nur der
 * Aufrufer."
 *
 * # Was hier NICHT steht
 *
 * Die vollständige Übersicht aus 11.6 — Fähigkeiten-Liste, Markierungen,
 * eigene Vorgeschichte — gehört zu E4-6 und ist ein eigenes Stück Arbeit.
 * Hier stehen Name und Instanz, mehr nicht.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import type { BossDef, ParseResult } from '../core/data.js';

/**
 * Welche Instanzen ein Spieler wählen darf.
 *
 * **Eine Positivliste, keine Ausschlussliste.** Von den 23 Datensätzen sind 14
 * Prüfwerkzeug, und ihre Instanz heißt uneinheitlich mal `testing`, mal
 * `test`. Wer `!== 'testing'` schriebe, ließe die `test`-Gruppe durch — und
 * die nächste Schreibweise ebenso. Ein neuer Prüf-Boss taucht so niemals
 * versehentlich in der Auswahl auf; ein neuer echter muss hier eingetragen
 * werden, und das ist die Richtung, in der ein Vergessen auffällt.
 */
const SPIELBAR: readonly string[] = ['prologue', 'instance-1', 'instance-2'];

/** Die Überschrift je Instanz. Englisch, wie alle Spieltexte. */
const INSTANZ_TITEL: Readonly<Record<string, string>> = {
  prologue: 'Prologue',
  'instance-1': 'Instance One',
  'instance-2': 'Instance Two',
};

/** Die wählbaren Bosse, in der Reihenfolge der Datensätze. */
export function waehlbareBosse(
  alle: ReadonlyMap<string, ParseResult<BossDef>>,
): readonly BossDef[] {
  const out: BossDef[] = [];
  for (const eintrag of alle.values()) {
    if (SPIELBAR.includes(eintrag.value.instance)) out.push(eintrag.value);
  }
  return out;
}

function element(tag: string, className?: string, text?: string): HTMLElement {
  const node = document.createElement(tag);
  if (className !== undefined) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Den Bereich bauen.
 *
 * @param bosse Die wählbaren Bosse, bereits gefiltert.
 * @param aktuell Kennung des Bosses, der gerade läuft — er wird markiert und
 *   ist nicht anwählbar. Ohne diese Markierung drückt ein Spieler auf den
 *   Namen, den er schon spielt, und der Kampf startet neu, ohne dass sich
 *   sichtbar etwas ändert.
 * @param onSelect Meldet die Wahl nach oben. Dieser Bereich startet nichts.
 */
export function buildBossSelect(
  bosse: readonly BossDef[],
  aktuell: string,
  onSelect: (bossId: string) => void,
): HTMLElement {
  const wurzel = element('div', 'boss-select');

  for (const instanz of SPIELBAR) {
    const drin = bosse.filter((b) => b.instance === instanz);
    if (drin.length === 0) continue;

    const gruppe = element('div', 'boss-select-group');
    gruppe.dataset.instance = instanz;
    gruppe.appendChild(element('h4', 'boss-select-group-title', INSTANZ_TITEL[instanz] ?? instanz));

    for (const boss of drin) {
      const knopf = document.createElement('button');
      knopf.type = 'button';
      knopf.className = 'boss-select-entry';
      knopf.dataset.bossId = boss.id;
      knopf.textContent = boss.name;
      if (boss.id === aktuell) {
        knopf.disabled = true;
        knopf.dataset.current = 'yes';
      }
      knopf.addEventListener('click', () => {
        onSelect(boss.id);
      });
      gruppe.appendChild(knopf);
    }
    wurzel.appendChild(gruppe);
  }

  return wurzel;
}
