import { TALENTS, type TalentDef } from '../core/data.js';
import { effectiveRanks, spentPoints, type TalentAllocation } from './talents.js';
import { withPlainText } from '../analysis/help.js';

/**
 * Die Talent-Ansicht im Player Home (Issue #802).
 *
 * # Warum es diese Datei gibt
 *
 * Sechs Bäume, achtzehn Wege, vier Schlüsseltalente und ~50 Punkte auf 100
 * Stufen waren gebaut, geprüft und paritätsgesichert — und **kein Spieler
 * konnte einen einzigen Punkt setzen**. `App.setTalents` hatte genau einen
 * Aufrufer ausserhalb der Prüffälle, in den Entwicklungswerkzeugen.
 *
 * # Eine Liste, keine Karte
 *
 * GAME-DESIGN 12.5 zeigt Bäume, und #249 will sie frei verschiebbar und
 * zoombar auf einem Bildschirm. Das ist ein eigenes Vorhaben. Hier geht es um
 * **Erreichbarkeit**: Was nützt die schönste Karte, wenn der Punkt nicht
 * ankommt. Eine Liste je Baum genügt dafür und steht der Gestaltung nicht im
 * Weg — sie ersetzt sie nicht, sie geht ihr voraus.
 *
 * # Die Vorbedingung wird HIER geprüft, nicht beim Rechnen
 *
 * `talents.json` sagt es wörtlich: „Die Bedingung steht als `requires` am
 * Talent und wird beim Verteilen geprüft, nicht beim Rechnen — ein Spielstand
 * aus einer älteren Fassung könnte eine Verteilung tragen, die heute nicht
 * mehr erlaubt wäre, und ein Kampf, der deswegen abbricht, wäre schlimmer als
 * einer, der die alte Verteilung zu Ende rechnet."
 *
 * Verteilt wird an dieser Stelle. Also steht die Prüfung hier.
 */

export interface TalentViewOptions {
  /** Was heute verteilt ist. */
  readonly allocation: TalentAllocation;
  /** Wie viele Punkte die Stufe hergibt. */
  readonly punkte: number;
  /**
   * Meldet den gewünschten neuen Stand eines Talents.
   *
   * Gibt `false` zurück, wenn er abgelehnt wurde — dann bleibt die Anzeige,
   * wie sie war. Dieselbe Bauart wie `onEquip` in der Ausrüstungs-Ansicht.
   */
  readonly onChange: (talentId: string, ranks: number) => boolean;
  /**
   * Ob überhaupt verteilt werden darf.
   *
   * **Während eines Kampfes nicht** (#425): `Combat.setTalents` sperrt sich ab
   * Zeit 0, ein Klick verfiele also stillschweigend. Eine Ansicht, die etwas
   * anbietet, das nicht ankommt, ist schlimmer als eine, die es sichtbar
   * verweigert.
   */
  readonly gesperrt?: boolean;
}

function element(tag: string, className: string, text?: string): HTMLElement {
  const knoten = document.createElement(tag);
  knoten.className = className;
  if (text !== undefined) knoten.textContent = text;
  return knoten;
}

/** Die Bäume in der Reihenfolge, in der die Talente stehen. */
function baeume(): { id: string; talente: TalentDef[] }[] {
  const gefunden: { id: string; talente: TalentDef[] }[] = [];
  for (const talent of TALENTS) {
    // Die Kennung eines Talents beginnt mit dem Baum: `calcination-trunk`.
    const baum = talent.id.slice(0, talent.id.indexOf('-'));
    const treffer = gefunden.find((b) => b.id === baum);
    if (treffer === undefined) gefunden.push({ id: baum, talente: [talent] });
    else treffer.talente.push(talent);
  }
  return gefunden;
}

/** Ob die Vorbedingung dieses Talents erfüllt ist. */
function freigeschaltet(talent: TalentDef, allocation: TalentAllocation): boolean {
  if (talent.requires === null) return true;
  const vorher = TALENTS.find((t) => t.id === talent.requires);
  if (vorher === undefined) return true;
  return effectiveRanks(allocation, vorher) > 0;
}

export function buildTalentView(options: TalentViewOptions): HTMLElement {
  const root = element('section', 'talents');
  const verbraucht = spentPoints(options.allocation);
  const frei = options.punkte - verbraucht;

  root.appendChild(
    element('p', 'talents-points', `${String(frei)} of ${String(options.punkte)} points left`),
  );
  if (options.gesperrt === true) {
    // Kein stummer Knopf, sondern ein Satz: Wer im Kampf klickt und nichts
    // geschieht, hält die Ansicht für kaputt statt die Regel für gewollt.
    root.appendChild(element('p', 'talents-locked', 'Talents are locked during a run.'));
  }

  for (const baum of baeume()) {
    const block = element('div', 'talents-tree');
    block.dataset.tree = baum.id;
    // **Mit Klartext daneben** (AC3 von #85, GAME-DESIGN 21.2): „CALCINATION
    // (Damage over time)". Hier stand der rohe Name, während die
    // Zusammenstellungs-Ansicht ihn längst mit Klartext zeigte — gemessen am
    // 2026-08-14: fünf der zehn Vorkommen im Dokument trugen ihn, fünf nicht,
    // und die fünf ohne waren alle hier.
    //
    // `withPlainText` gibt unbekannte Begriffe unverändert zurück. Das trifft
    // den Baum `global`, der kein alchemistischer Prozess ist und deshalb auch
    // keinen Klartext hat — er bleibt „GLOBAL", und das ist richtig so.
    block.appendChild(element('h4', 'talents-tree-name', withPlainText(baum.id.toUpperCase())));

    for (const talent of baum.talente) {
      const zeile = element('div', 'talents-row');
      zeile.dataset.talent = talent.id;
      const raenge = effectiveRanks(options.allocation, talent);
      const offen = freigeschaltet(talent, options.allocation);

      zeile.appendChild(
        element(
          'span',
          'talents-name',
          `${talent.name} ${String(raenge)}/${String(talent.maxRanks)}`,
        ),
      );

      for (const [aktion, zeichen, ziel] of [
        ['remove', '-', raenge - 1],
        ['add', '+', raenge + 1],
      ] as const) {
        const knopf = document.createElement('button');
        knopf.className = `talents-button talents-${aktion}`;
        knopf.type = 'button';
        knopf.textContent = zeichen;
        knopf.dataset.talent = talent.id;
        knopf.dataset.talentAction = aktion;
        // **Der Knopf bleibt stehen, auch wenn er nichts kann.** Sonst
        // verschwänden Wege aus der Ansicht, sobald die Punkte alle sind — und
        // der Spieler sähe nicht mehr, was es überhaupt gibt.
        knopf.disabled =
          options.gesperrt === true ||
          ziel < 0 ||
          ziel > talent.maxRanks ||
          (aktion === 'add' && (frei <= 0 || !offen));
        knopf.addEventListener('click', () => {
          options.onChange(talent.id, ziel);
        });
        zeile.appendChild(knopf);
      }

      if (!offen) {
        zeile.appendChild(element('span', 'talents-locked-hint', `needs ${talent.requires ?? ''}`));
      }
      block.appendChild(zeile);
    }
    root.appendChild(block);
  }

  return root;
}
