/**
 * Die Beschriftung eines Stücks (`itemLabel`, Issue #480).
 *
 * **Diese Funktion hatte bis hierher keinen einzigen Prüffall.** Sie erzeugt
 * die Zeile, an der ein Spieler zwei Stücke vergleicht — und sie zeigte bis
 * #480 die Stufe der VORLAGE, während die Rechnung inzwischen die des Stücks
 * nimmt.
 *
 * Eine Anzeige, die der Rechnung widerspricht, ist schlimmer als eine, die
 * fehlt: Der Spieler wählt dann nach einer Zahl, die nicht die ist, nach der
 * die Stücke wirken. Und niemand meldet es als Fehler, weil beide Zahlen
 * plausibel aussehen.
 */

import { describe, expect, it } from 'vitest';
import { itemLabel } from '../../src/progression/gear-view.js';
import { itemById, type OwnedItemLike } from '../../src/gear/catalog.js';
import { MAX_ITEM_LEVEL } from '../../src/gear/budget.js';

const KENNUNG = 'test-relic';
const VORLAGE = itemById(KENNUNG)?.itemLevel ?? 0;

function stueck(felder: Partial<OwnedItemLike> = {}): OwnedItemLike {
  return { itemId: KENNUNG, upgradeSteps: 0, equippedSlot: null, ...felder };
}

/** Die Zahl hinter „ilvl" — an ihr vergleicht der Spieler. */
function ilvlAus(text: string): number {
  return Number(/ilvl (\d+)/.exec(text)?.[1] ?? NaN);
}

describe('itemLabel — die angezeigte Stufe ist die, nach der gerechnet wird', () => {
  it('ein unaufgewertetes Stück zeigt die Stufe der Vorlage', () => {
    expect(ilvlAus(itemLabel(stueck()))).toBe(VORLAGE);
    // ZUSICHERUNG: Die Vorlage ist nicht 0 — sonst prüfte der Fall gegen einen
    // Wert, den auch eine kaputte Auflösung liefert.
    expect(VORLAGE).toBeGreaterThan(0);
  });

  it('ein Stück mit 14 Aufwertungen zeigt die HÖHERE Zahl', () => {
    // Das Kriterium, wie es der Orchestrator gestellt hat.
    const roh = ilvlAus(itemLabel(stueck()));
    const gehoben = ilvlAus(itemLabel(stueck({ upgradeSteps: 14 })));
    expect(gehoben).toBe(VORLAGE + 14);
    expect(gehoben).toBeGreaterThan(roh);
  });

  it('und die gewürfelte Fundstufe schlägt ebenfalls durch', () => {
    // Die zweite Achse. Beide laufen in `effectiveLevelOf` zusammen, aber dass
    // die Beschriftung auch diese sieht, ist damit nicht selbstverständlich.
    expect(ilvlAus(itemLabel(stueck({ itemLevel: 85 })))).toBe(85);
    expect(ilvlAus(itemLabel(stueck({ itemLevel: 85, upgradeSteps: 3 })))).toBe(88);
  });

  it('die Anzeige klemmt wie die Rechnung — keine Zahl über dem Höchstwert', () => {
    // Stünde hier eine 140, während gerechnet wird, als wären es 100, wäre der
    // Widerspruch nur an anderer Stelle.
    const text = itemLabel(stueck({ itemLevel: 85, upgradeSteps: 55 }));
    expect(ilvlAus(text)).toBe(MAX_ITEM_LEVEL);
  });

  it('die Aufwertung steht weiterhin als „+14" dabei', () => {
    // Sie ist in der Stufe schon enthalten; sichtbar bleibt sie trotzdem, weil
    // zwei Stücke sonst gleich heissen und die Identität am Listenindex hängt.
    expect(itemLabel(stueck({ upgradeSteps: 14 }))).toContain('+14');
    expect(itemLabel(stueck())).not.toContain('+');
  });

  it('die Qualität ist die des Stücks, nicht die der Vorlage', () => {
    const vorlageWort = itemLabel(stueck());
    const eigen = itemLabel(stueck({ quality: 0 }));
    expect(eigen).toContain('common');
    // ZUSICHERUNG: Die Vorlage trägt eine ANDERE Qualität — sonst wäre der
    // Fall auch grün, wenn `quality` gar nicht durchkommt.
    expect(vorlageWort).not.toContain('common');
  });

  it('ein unbekanntes Stück bleibt unbekannt', () => {
    expect(itemLabel(stueck({ itemId: 'gibt-es-nicht' }))).toBe('gibt-es-nicht (unknown)');
  });
});
