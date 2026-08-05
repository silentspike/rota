/**
 * Was der Aufstieg im Player Home zeigt (Issue #248, AC6).
 *
 * SIE RECHNET, SIE ZEIGT NICHT
 * ----------------------------
 * Kein DOM, kein Text mit Auszeichnung — dieselbe Trennung wie bei der
 * Werte-Übersicht (#188). Was hier entsteht, sind Zeilen mit fertigen Zahlen;
 * die Ansicht setzt sie nur. Der Grund ist prüfbar: Die Prüffälle laufen ohne
 * Browser, und eine Rechnung, die im DOM steckt, ließe sich dort gar nicht
 * messen.
 *
 * ALLES GERECHNET, NICHTS GESPEICHERT
 * -----------------------------------
 * Der Spielstand trägt zwei Zahlen — Stufe und Erfahrung (AC5). Was hier
 * daraus wird, entsteht bei jedem Aufruf neu: der Anteil bis zur nächsten
 * Stufe, die Zahl der Plätze, der Grundwert auf dieser Stufe. Ein gespeicherter
 * Anzeigewert wäre kein Fehler, den man merkt — er wäre nur alt.
 *
 * Spieltexte englisch (Design-Gesetz 5).
 */
import { LEVELS } from '../core/data.js';
import { levelProgress, xpToNext, type LevelState } from './level.js';
import { barSlotsAtLevel, nextBarSlotLevel } from './unlocks.js';

/** Eine Zeile der Übersicht. */
export interface LevelRow {
  /** Kennung, kleingeschrieben — als Merkmal am Knoten, für Prüfungen. */
  readonly id: string;
  /** Beschriftung im Spiel, englisch. */
  readonly label: string;
  /** Der fertige Wert als Text. */
  readonly value: string;
  /**
   * Ein Zusatz, oder `null`.
   *
   * Für das, was die Zahl allein nicht sagt — „next slot at 15" neben der Zahl
   * der Plätze. `null` heißt: Es gibt nichts hinzuzufügen, nicht „leerer Text".
   */
  readonly note: string | null;
}

/**
 * Der Anteil bis zur nächsten Stufe, als Zahl zwischen 0 und 1.
 *
 * Getrennt von den Zeilen, weil die Ansicht daraus einen Balken macht und
 * keinen Text. Eine Prozentzahl im Text wäre dieselbe Größe zweimal.
 */
export function levelBarFill(state: LevelState): number {
  return levelProgress(state);
}

/**
 * Die Zeilen der Aufstiegs-Übersicht.
 *
 * **Auf der Höchststufe steht `MAX` statt einer Zahl.** Ein Balken, der bei
 * einer unerreichbaren Zahl hängt, sähe aus wie ein Fehler; „noch 197008 bis
 * Stufe 101" wäre ein Versprechen, das niemand einlöst.
 */
export function levelOverview(state: LevelState): LevelRow[] {
  const stufe = Math.min(Math.max(Math.trunc(state.level), 1), LEVELS.maxLevel);
  const bedarf = xpToNext(stufe);
  const hoechste = stufe >= LEVELS.maxLevel;
  const plaetze = barSlotsAtLevel(stufe);
  const naechsterPlatz = nextBarSlotLevel(stufe);

  return [
    {
      id: 'level',
      label: 'Level',
      value: String(stufe),
      note: hoechste ? 'maximum reached' : null,
    },
    {
      id: 'experience',
      label: 'Experience',
      value: hoechste ? 'MAX' : `${state.xp} / ${bedarf}`,
      note: hoechste ? null : `${bedarf - state.xp} to next level`,
    },
    {
      id: 'bar-slots',
      label: 'Bar slots',
      value: String(plaetze),
      // **Die nächste Freischalt-Stufe, nicht die nächste Stufe.** Der Spieler
      // soll wissen, worauf er zusteuert; „next level" stünde schon oben.
      note: naechsterPlatz === null ? 'all unlocked' : `next at level ${naechsterPlatz}`,
    },
    {
      id: 'stat-growth',
      label: 'Stat growth',
      // **Der FAKTOR, nicht der Wert.** Hier stand einen Entwurf lang „Vessel
      // 5000" — und im Bild fiel auf, dass die Werte-Übersicht direkt darunter
      // dieselbe Zahl unter demselben Namen zeigt. Zwei Zahlen mit einem Namen
      // auf einem Bildschirm ist genau der Fall, der in #247 den ganzen
      // Baustein umgeworfen hat, eine Ebene kleiner.
      //
      // Der Faktor dagegen steht nirgends sonst und beantwortet die Frage, die
      // hierher gehört: Was hat der Aufstieg gebracht?
      value: `x${(LEVELS.factorPerMille[stufe - 1] ?? 1000) / 1000}`,
      note: `+${LEVELS.growthPerLevelPercent}% per level`,
    },
  ];
}
