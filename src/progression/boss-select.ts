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
import {
  BOSSES,
  BOSS_STAGES,
  INSTANCES,
  instanceById,
  type BossDef,
  type ParseResult,
} from '../core/data.js';
import { bossIdOfMarkKey, markKey } from '../analysis/progress-mark.js';
import { difficultyName } from '../core/difficulty.js';
import { DIFFICULTY_COUNT, DIFFICULTY_NAMES, DIFFICULTY_NORMAL } from '../loot/rates.js';

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
 * **Zwei der vier Kriterien öffnen eine Instanz**, nebeneinander:
 *
 * | Kriterium | hier |
 * | --- | --- |
 * | Instanz abgeschlossen | die in `requires` genannte ist vollständig bezwungen |
 * | Punkte-Schwelle | `verdient >= pointsThreshold` — das Ventil gegen Feststecken |
 * | Schwierigkeitsstufe | öffnet keine Instanz, sondern ihre Stufen: {@link offeneStufen} |
 * | Meisterschafts-Schlüssel | nur Geheim-Instanzen, nach V1 (Entscheidung 29.10.) |
 *
 * **Vollständig heißt: jeder Boss.** Eine Instanz, von der drei von vier
 * liegen, ist nicht abgeschlossen — sonst hinge die Freischaltung daran,
 * welchen Boss jemand zuerst wählt.
 *
 * **`verdient` sind die VERDIENTEN Punkte, nicht das Guthaben** (Entscheidung
 * des Auftraggebers vom 29.10.: „Instanzen öffnen über die Punkte-Schwelle
 * ohne Ausgeben“). Wer Punkte in eine Aufwertung steckt, schließt sich damit
 * keine Instanz wieder zu. Ohne Angabe 0 — dann zählt allein der Hauptpfad,
 * wie bis #1177.
 */
export function freigeschalteteInstanzen(
  besiegt: readonly string[],
  alle: ReadonlyMap<string, ParseResult<BossDef>>,
  verdient = 0,
): readonly string[] {
  // **Die Kennung aus dem Schlüssel lösen** (Fehlerbehebung 2026-08-11).
  //
  // `defeatedBosses` führt Schlüssel aus Kennung UND Stufe, gebildet über
  // `markKey`. Bis hierher fragte diese Funktion nach der reinen Kennung —
  // die beiden Mengen berührten sich nie, `abgeschlossen()` lieferte IMMER
  // false, und keine Instanz mit `requires` wurde je freigeschaltet.
  //
  // Im Browser gemessen: vier Bosse besiegt, Instance Two blieb unsichtbar.
  // Die Prüffälle waren grün, weil sie reine Kennungen übergaben — sie
  // prüften die Funktion, nicht den Weg.
  const gefallen = new Set(besiegt.map(bossIdOfMarkKey));
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
  // **Das Ventil**: dieselbe Instanz, ohne den Abschluss davor. `null` heißt
  // „hat keins“ — nicht „Schwelle 0“, die jede Instanz sofort öffnete.
  const ueberSchwelle = (schwelle: number | null): boolean =>
    schwelle !== null && verdient >= schwelle;

  return INSTANCES.filter(
    (i) => i.requires === null || abgeschlossen(i.requires) || ueberSchwelle(i.pointsThreshold),
  ).map((i) => i.id);
}

/**
 * Welche Schwierigkeitsstufen einer Instanz offen sind (12.7, Issue #1177).
 *
 * > **Schwierigkeitsstufe** — Heroische Instanz öffnet nach Normal, mythische
 * > nach Heroisch. (Masterplan, Freischaltung)
 *
 * **Je Instanz, nicht je Boss**, wörtlich nach dem Satz: Heroisch öffnet, wenn
 * JEDER Boss der Instanz auf Normal liegt; Mythisch, wenn jeder auf Heroisch
 * liegt. Je Boss wäre die schwächere Regel — ein Spieler könnte den ersten
 * Boss mythisch wiederholen, ohne die Instanz je beendet zu haben, und
 * „Mythisch nach Heroisch“ hiesse dann „nach einem heroischen Sieg“.
 *
 * **Der Schlüssel trägt die Stufe** (`markKey(id, stufe)` in
 * `defeatedBosses`). Anders als in {@link freigeschalteteInstanzen} wird sie
 * hier nicht abgelöst — genau sie ist die Frage.
 *
 * **Eine Instanz mit Bossen ohne Stufen bleibt bei Normal.** Das sind
 * Prüf-Instanzen; eine heroische Fassung, die es für einen ihrer Bosse nicht
 * gibt, ließe sich anbieten und nicht spielen.
 *
 * Offen ist immer mindestens Normal. Ob die Instanz selbst betreten werden
 * darf, entscheidet {@link freigeschalteteInstanzen}, nicht diese Funktion.
 */
export function offeneStufen(
  instanz: string,
  besiegt: readonly string[],
  alle: ReadonlyMap<string, ParseResult<BossDef>> = BOSSES,
  stufen: ReadonlyMap<string, readonly ParseResult<BossDef>[]> = BOSS_STAGES,
): readonly number[] {
  const bosse = [...alle.values()]
    .map((eintrag) => eintrag.value)
    .filter((boss) => boss.instance === instanz)
    .map((boss) => boss.id);
  const offen: number[] = [DIFFICULTY_NORMAL];
  if (bosse.length === 0) return offen;
  const gefallen = new Set(besiegt);
  for (let stufe = DIFFICULTY_NORMAL + 1; stufe < DIFFICULTY_COUNT; stufe++) {
    const darunterGelegt = bosse.every((id) => gefallen.has(markKey(id, stufe - 1)));
    const spielbar = bosse.every((id) => stufen.get(id)?.[stufe] !== undefined);
    if (!darunterGelegt || !spielbar) break;
    offen.push(stufe);
  }
  return offen;
}

/**
 * Alles, was gerade offen ist, als Kennungen: jede Instanz und jede Stufe über
 * Normal (`instance-1@heroic`), in der Reihenfolge der Auswahl.
 *
 * Der Vergleichsgegenstand für {@link neueFreischaltungen}: Der Klang
 * „Freischaltung“ (`menu-unlock`) gehört zu dem Sieg, der etwas öffnet — nicht
 * zu jedem Öffnen der Auswahl, in der es danach steht.
 */
export function freischaltungen(
  besiegt: readonly string[],
  alle: ReadonlyMap<string, ParseResult<BossDef>> = BOSSES,
  verdient = 0,
): readonly string[] {
  const out: string[] = [];
  for (const instanz of freigeschalteteInstanzen(besiegt, alle, verdient)) {
    out.push(instanz);
    for (const stufe of offeneStufen(instanz, besiegt, alle)) {
      if (stufe !== DIFFICULTY_NORMAL) out.push(`${instanz}@${DIFFICULTY_NAMES[stufe] ?? stufe}`);
    }
  }
  return out;
}

/**
 * Was zwischen zwei Ständen NEU offen ist (Issue #1177).
 *
 * **Eine Differenz, kein Zustand**: Wer den Klang an „ist offen“ hängte, hörte
 * ihn bei jedem Sieg danach wieder. Gerechnet wird über die Kennungen aus
 * {@link freischaltungen}, also über beide Wege zu einer Instanz — Abschluss
 * und Punkte-Schwelle — und über jede neue Stufe.
 */
export function neueFreischaltungen(
  vorher: readonly string[],
  nachher: readonly string[],
): readonly string[] {
  const alt = new Set(vorher);
  return nachher.filter((kennung) => !alt.has(kennung));
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
  verdient = 0,
): readonly BossDef[] {
  const offen = freigeschalteteInstanzen(besiegt, alle, verdient);
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
 * Die Fähigkeiten eines Bosses, jede einmal.
 *
 * # Warum aus der Zeitleiste und nicht aus einem Feld `abilities`
 *
 * Weil es dieses Feld nicht gibt — gemessen an `BossDef`. Was ein Boss an
 * Handlungen kann, steht in `timeline`, und jeder Eintrag trägt seinen
 * englischen Anzeigenamen. Eine zweite Liste danebenzustellen hiesse, dieselbe
 * Sache zweimal zu pflegen; die zweite wäre nach dem ersten neuen Boss falsch.
 *
 * # Der Name ist die Kennung, und das ist eine Feststellung, keine Wahl
 *
 * `TimelineEntryDef` hat kein Kennungsfeld. Der Name ist das Einzige, woran
 * eine Erklärung hängen kann — und er ist auch das, was der Spieler liest.
 *
 * Mehrfach genannte Fähigkeiten erscheinen einmal: Ein Boss, der dreimal
 * zuschlägt, kann eine Sache, nicht drei.
 */
/**
 * Die Mechanik-Bausteine eines Bosses, jeder einmal, in ihrer Reihenfolge.
 *
 * Das ist die gröbere Ebene über [`faehigkeitenVon`]: „Kindling Roar" ist eine
 * Fähigkeit, `interruptibleCast` die Mechanik dahinter. Die Einführung erklärt
 * die Mechanik (17.3 sagt „beim ersten Auftreten einer Mechanik") — sonst
 * bekäme der Spieler dieselbe Sache unter sechzig Namen erklärt.
 *
 * Die Reihenfolge ist die der Zeitleiste und damit die, in der er ihnen
 * begegnet. Wer zuerst kommt, wird zuerst erklärt.
 */
export function mechanikenVon(boss: BossDef): readonly string[] {
  const gesehen = new Set<string>();
  const out: string[] = [];
  for (const eintrag of boss.timeline) {
    if (eintrag.mechanic === '' || gesehen.has(eintrag.mechanic)) continue;
    gesehen.add(eintrag.mechanic);
    out.push(eintrag.mechanic);
  }
  return out;
}

export function faehigkeitenVon(boss: BossDef): readonly string[] {
  const gesehen = new Set<string>();
  const out: string[] = [];
  for (const eintrag of boss.timeline) {
    if (eintrag.name === '' || gesehen.has(eintrag.name)) continue;
    gesehen.add(eintrag.name);
    out.push(eintrag.name);
  }
  return out;
}

/**
 * Den Bereich bauen.
 *
 * @param bosse Die wählbaren Bosse, bereits gefiltert.
 * @param aktuell Kennung des Bosses, der gerade läuft — er wird markiert und
 *   ist nicht anwählbar. Ohne diese Markierung drückt ein Spieler auf den
 *   Namen, den er schon spielt, und der Kampf startet neu, ohne dass sich
 *   sichtbar etwas ändert.
 * @param onSelect Meldet die Wahl nach oben — Boss UND Stufe. Dieser Bereich
 *   startet nichts.
 * @param stufen Welche Stufen der Spieler je Boss wählen darf (Issue #1177).
 *   Ohne Angabe nur Normal: Ein Aufrufer, der den Spielstand nicht kennt,
 *   bietet nichts an, was er nicht prüfen kann.
 * @param aktuelleStufe Die Stufe des laufenden Bosses. Gesperrt ist nur das
 *   PAAR aus Boss und Stufe — wer Heroisch spielt, kann auf Normal zurück.
 */
export function buildBossSelect(
  bosse: readonly BossDef[],
  aktuell: string,
  onSelect: (bossId: string, difficulty: number) => void,
  stufen: (bossId: string) => readonly number[] = () => [DIFFICULTY_NORMAL],
  aktuelleStufe: number = DIFFICULTY_NORMAL,
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
      if (boss.id === aktuell && aktuelleStufe === DIFFICULTY_NORMAL) {
        knopf.disabled = true;
        knopf.dataset.current = 'yes';
      }
      knopf.addEventListener('click', () => {
        onSelect(boss.id, DIFFICULTY_NORMAL);
      });
      gruppe.appendChild(knopf);

      // **Die Stufen über Normal, nur die offenen** (12.7, Issue #1177).
      //
      // Der Name bleibt der Knopf für Normal — so liest sich die Liste wie
      // vorher, und ein Spieler ohne geöffnete Stufe sieht keine Änderung.
      // Darunter je offener Stufe ein eigener Knopf: Ein Knopf ist über die
      // Tabulatortaste erreichbar und mit Eingabe auszulösen, wie jeder andere
      // in diesem Bereich — ein Umschalter daneben wäre eine zweite Bedienart.
      const oben = stufen(boss.id).filter((s) => s !== DIFFICULTY_NORMAL);
      if (oben.length > 0) {
        const zeile = element('div', 'boss-select-stages');
        for (const stufe of oben) {
          const stufenKnopf = document.createElement('button');
          stufenKnopf.type = 'button';
          stufenKnopf.className = 'boss-select-stage';
          stufenKnopf.dataset.bossId = boss.id;
          stufenKnopf.dataset.difficulty = String(stufe);
          stufenKnopf.textContent = difficultyName(stufe);
          if (boss.id === aktuell && stufe === aktuelleStufe) {
            stufenKnopf.disabled = true;
            stufenKnopf.dataset.current = 'yes';
          }
          stufenKnopf.addEventListener('click', () => {
            onSelect(boss.id, stufe);
          });
          zeile.appendChild(stufenKnopf);
        }
        gruppe.appendChild(zeile);
      }

      // **Die Fähigkeiten unter dem Boss** (AC2 von #811, GAME-DESIGN 17.3:
      // „Muss auch für Boss-Fähigkeiten funktionieren"). Sie stehen NEBEN dem
      // Knopf und nicht darin: Ein Erklärfenster in einem Knopf wäre beim
      // laufenden Boss unerreichbar, weil der Knopf dann abgeschaltet ist —
      // und gerade dessen Fähigkeiten will man nachschlagen.
      //
      // Hier und nicht im Kampf: Die Leiste und der Ring sind Zeichenfläche
      // (GAME-DESIGN 18.1) und tragen kein Merkmal, an dem ein Fenster hängen
      // könnte. Die Wahl ist ausserdem der Ort, an dem 11.6 zeigen will,
      // worauf man sich einlässt.
      const koennen = faehigkeitenVon(boss);
      if (koennen.length > 0) {
        const liste = element('ul', 'boss-select-abilities');
        for (const name of koennen) {
          const zeile = element('li', 'boss-select-ability', name);
          zeile.dataset.help = name;
          // Über die Tastatur erreichbar, sonst bekäme die Erklärung nur, wer
          // eine Maus benutzt — `attachHelp` hört auf `focus` genau dafür.
          zeile.tabIndex = 0;
          liste.appendChild(zeile);
        }
        gruppe.appendChild(liste);
      }
    }
    wurzel.appendChild(gruppe);
  }

  return wurzel;
}
