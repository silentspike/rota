/**
 * Die zehn Ausrüstungsplätze (GAME-DESIGN 13.2, Issue E6-1).
 *
 * > Bewusst **keine Körperteile**, weil es keine Spielfigur gibt — arkane
 * > Artefakte. (13.2)
 *
 * Jeder Platz trägt die **Grundwerte** ESSENCE, VESSEL und WARD — „jeder Fund
 * verbessert also spürbar etwas, auch wenn die Sonderwerte nicht passen" —
 * und darüber hinaus einen **platz-spezifischen Sonderwert in fester
 * Zuordnung**. Der Sinn dieser Zuordnung steht ausdrücklich im Design:
 *
 * > **Gezieltes Grinden statt blinden Sammelns** — jeder der zehn Plätze
 * > bekommt einen erkennbaren Charakter. (13.2)
 *
 * # Zwei Plätze haben noch keinen Charakter
 *
 * 13.2 zählt zehn Plätze auf und ordnet **acht** davon zu. `Relic` und `Shard`
 * kommen im gesamten Design-Dokument genau einmal vor — in der Namensliste.
 * Sie stehen hier deshalb als [`SPECIAL_UNDECIDED`], und
 * [`isSlotDecided`] meldet das.
 *
 * **Das ist Absicht und kein Platzhalter, den man später vergisst.** Ein
 * geratener Sonderwert wäre eine Design-Entscheidung im Code — und zwar eine,
 * die niemandem auffiele, weil sie plausibel aussähe. Ein Prüffall schlägt
 * fehl, sobald ein Gegenstand auf einem unbestimmten Platz einen Sonderwert
 * bekommt. Eintragen ist später eine Datenzeile, kein Umbau.
 *
 * # Zwei Kategorien sind noch keine Werte
 *
 * 6.1 kennt drei Grundwerte (ESSENCE, VESSEL, WARD), 6.2 drei Sonderwerte
 * (CATALYSIS, PURITY, YIELD). Das sind alle Werte, die das Dokument kennt. Die
 * Platz-Tabelle vergibt aber vier Kategorien, und zwei davon zeigen auf keinen
 * dieser sechs:
 *
 * | Kategorie              | Steht im Dokument als Wert? |
 * | ---------------------- | --------------------------- |
 * | CATALYSIS              | ja (6.2)                    |
 * | PURITY/YIELD           | ja (6.2)                    |
 * | „Verteidigung"         | **nein** — WARD? VESSEL? beide? |
 * | „Zauber-Verstärkung"   | **nein** — vermutlich eher ein Effekt aus Topf 4 als ein Wert |
 *
 * Sie stehen deshalb als eigene Kategorien und nicht als Verweis auf einen der
 * sechs Werte. Was sie genau sind, entscheidet das Design, nicht diese Datei.
 */

/** Die zehn Plätze in der Reihenfolge aus 13.2. */
export const SLOT_FOCUS = 0;
export const SLOT_CATALYST = 1;
export const SLOT_SIGIL = 2;
export const SLOT_RELIC = 3;
export const SLOT_GRIMOIRE = 4;
export const SLOT_SHARD = 5;
export const SLOT_TALISMAN = 6;
export const SLOT_PHYLACTERY = 7;
export const SLOT_ANCHOR = 8;
export const SLOT_PRISM = 9;

export const SLOT_COUNT = 10;

export const SLOT_NAMES: readonly string[] = [
  'focus',
  'catalyst',
  'sigil',
  'relic',
  'grimoire',
  'shard',
  'talisman',
  'phylactery',
  'anchor',
  'prism',
];

/**
 * Die Grundwerte, die auf **jedem** Platz sitzen (13.2, 6.1).
 *
 * > „Grundwerte (ESSENCE, VESSEL, WARD) auf jedem Platz — jeder Fund
 * > verbessert also spürbar etwas, auch wenn die Sonderwerte nicht passen."
 *
 * # Eine Frage, die das Design nicht beantwortet
 *
 * **Ziehen die Grundwerte aus demselben Budget wie die Werte-Plätze?** 13.3
 * sagt, das Budget werde „auf die Werte-Plätze verteilt", und gibt Common
 * genau einen davon. Ein Platz kann aber keine drei Grundwerte tragen. Also
 * entweder sind die Grundwerte **zusätzlich** zum Budget, oder „Werte-Plätze"
 * meint etwas anderes als hier angenommen.
 *
 * Beide Lesarten sind balance-relevant und keine davon steht im Dokument.
 * Diese Liste sagt deshalb nur, **welche** Grundwerte es gibt — nicht, woher
 * sie ihre Zahlen nehmen. Das entscheidet das Design.
 */
export const BASE_STAT_IDS: readonly string[] = ['essence', 'vessel', 'ward'];

/** Noch nicht zugeordnet — siehe Kopf dieser Datei. */
export const SPECIAL_UNDECIDED = -1;
/** CATALYSIS (6.2). */
export const SPECIAL_CATALYSIS = 0;
/** PURITY und YIELD gemeinsam (6.2). */
export const SPECIAL_PURITY_YIELD = 1;
/** „Verteidigung" — im Dokument kein benannter Wert. */
export const SPECIAL_DEFENSE = 2;
/** „Zauber-Verstärkung" — im Dokument kein benannter Wert. */
export const SPECIAL_SPELL_AMP = 3;

export const SPECIAL_NAMES: readonly string[] = ['catalysis', 'purityYield', 'defense', 'spellAmp'];

/**
 * Die feste Zuordnung aus 13.2 — **Daten, nicht Code**.
 *
 * Dieselbe Linie wie bei den Schulen und den Ereignis-Empfängern: Wer eine
 * Zuordnung ändert, ändert eine Zeile und keine Verzweigung.
 */
export const SLOT_SPECIAL: readonly number[] = [
  SPECIAL_CATALYSIS, // focus
  SPECIAL_CATALYSIS, // catalyst
  SPECIAL_PURITY_YIELD, // sigil
  SPECIAL_UNDECIDED, // relic      <- 13.2 sagt dazu nichts
  SPECIAL_SPELL_AMP, // grimoire
  SPECIAL_UNDECIDED, // shard      <- 13.2 sagt dazu nichts
  SPECIAL_PURITY_YIELD, // talisman
  SPECIAL_DEFENSE, // phylactery
  SPECIAL_DEFENSE, // anchor
  SPECIAL_SPELL_AMP, // prism
];

/** Trägt dieser Platz eine festgelegte Sonderwert-Zuordnung? */
export function isSlotDecided(slot: number): boolean {
  return (SLOT_SPECIAL[slot] ?? SPECIAL_UNDECIDED) !== SPECIAL_UNDECIDED;
}

/** Welche Plätze noch auf ihre Zuordnung warten. */
export function undecidedSlots(): number[] {
  const open: number[] = [];
  for (let slot = 0; slot < SLOT_COUNT; slot++) {
    if (!isSlotDecided(slot)) open.push(slot);
  }
  return open;
}
