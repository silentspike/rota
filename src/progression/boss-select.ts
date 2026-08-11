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
import { INSTANCES, instanceById, type BossDef, type ParseResult } from '../core/data.js';

/**
 * Welche Instanzen ein Spieler wählen darf.
 *
 * **Eine Positivliste, keine Ausschlussliste.** Von den Datensätzen sind
 * achtzehn Prüfwerkzeug, und ihre Instanz heißt uneinheitlich mal `testing`,
 * mal `test`. Wer `!== 'testing'` schriebe, ließe die `test`-Gruppe durch —
 * und die nächste Schreibweise ebenso.
 *
 * **Seit #421c aus `data/instances.json`, nicht mehr von Hand.**
 *
 * Hier standen bis dahin zwei Listen: `SPIELBAR` mit den Kennungen und
 * `INSTANZ_TITEL` mit den Überschriften. Wer eine Instanz hinzufügte, musste
 * beide pflegen und konnte eine vergessen — dieselbe doppelte Wahrheit, die
 * am 2026-08-10 den stillen Affix-Verlust erzeugt hat.
 *
 * **Die Positivliste bleibt eine**, nur an einem Ort: Eine Instanz ist
 * wählbar, wenn sie in `instances.json` steht. Prüf-Bosse tragen `testing`
 * oder `test` und stehen dort nicht — sie tauchen also weiterhin nie
 * versehentlich auf, und ein neuer echter muss eingetragen werden.
 */
function spielbareInstanzen(): readonly string[] {
  return INSTANCES.map((i) => i.id);
}

/** Die Überschrift je Instanz — aus den Daten, englisch wie alle Spieltexte. */
function instanzTitel(id: string): string {
  return instanceById(id)?.name ?? id;
}

/**
 * Welche Instanzen der Spieler betreten darf (GAME-DESIGN 12.7).
 *
 * **Nur der Hauptpfad.** 12.7 nennt vier Kriterien nebeneinander; hier zählt
 * „Instanz abgeschlossen": Eine Instanz ist offen, wenn die in `requires`
 * genannte vollständig bezwungen ist. Punkte-Schwelle, Schwierigkeitsstufe
 * und Meisterschafts-Schlüssel fehlen bewusst — die Schwelle hat niemand
 * entschieden, und Schwierigkeitsstufen gibt es im Spiel noch nicht.
 *
 * **Vollständig heißt: jeder Boss.** Eine Instanz, von der drei von vier
 * liegen, ist nicht abgeschlossen — sonst hinge die Freischaltung daran,
 * welchen Boss jemand zuerst wählt.
 */
export function freigeschalteteInstanzen(
  besiegt: readonly string[],
  alle: ReadonlyMap<string, ParseResult<BossDef>>,
): readonly string[] {
  const gefallen = new Set(besiegt);
  const bosseJeInstanz = new Map<string, string[]>();
  for (const eintrag of alle.values()) {
    const liste = bosseJeInstanz.get(eintrag.value.instance) ?? [];
    liste.push(eintrag.value.id);
    bosseJeInstanz.set(eintrag.value.instance, liste);
  }

  const abgeschlossen = (id: string): boolean => {
    const drin = bosseJeInstanz.get(id) ?? [];
    // **Eine Instanz ohne Bosse gilt NICHT als abgeschlossen.** Sonst öffnete
    // ein leerer Eintrag alles Folgende, und zwar lautlos.
    return drin.length > 0 && drin.every((bossId) => gefallen.has(bossId));
  };

  return INSTANCES.filter((i) => i.requires === null || abgeschlossen(i.requires)).map((i) => i.id);
}

/**
 * Die wählbaren Bosse, in der Reihenfolge der Datensätze.
 *
 * # `besiegt` ist der Unterschied zwischen gebaut und angeschlossen
 *
 * Bis #421c filterte diese Funktion nur nach „Instanz existiert". Die
 * Freischaltung aus 12.7 stand daneben in `freigeschalteteInstanzen` —
 * gebaut, mit Prüffällen, **und von niemandem gerufen**. `instance-3` wäre
 * damit sofort sichtbar gewesen, obwohl sie `instance-2` verlangt.
 *
 * Das ist dieselbe Klasse wie der stille Affix-Verlust und wie Topf 4: Eine
 * Sache ist gebaut, geprüft und nicht verdrahtet — und die Prüffälle sind
 * grün, weil sie die Funktion messen und nicht ihren Aufruf.
 *
 * **Ohne Argument bleibt es beim alten Verhalten**, damit ein Aufrufer, der
 * den Spielstand nicht kennt, nicht versehentlich alles sperrt.
 */
export function waehlbareBosse(
  alle: ReadonlyMap<string, ParseResult<BossDef>>,
  besiegt: readonly string[] = [],
): readonly BossDef[] {
  const offen = freigeschalteteInstanzen(besiegt, alle);
  const out: BossDef[] = [];
  for (const eintrag of alle.values()) {
    if (offen.includes(eintrag.value.instance)) out.push(eintrag.value);
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

  for (const instanz of spielbareInstanzen()) {
    const drin = bosse.filter((b) => b.instance === instanz);
    if (drin.length === 0) continue;

    const gruppe = element('div', 'boss-select-group');
    gruppe.dataset.instance = instanz;
    gruppe.appendChild(element('h4', 'boss-select-group-title', instanzTitel(instanz)));

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
