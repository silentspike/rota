/**
 * Zustands-Prüfsumme je Takt (GAME-DESIGN 19.1).
 *
 * **Wozu:** Weichen Spiel und Simulator voneinander ab, sagt die Folge der
 * Prüfsummen, **in welchem Takt** es passiert ist — statt nur, dass der
 * Endschaden nicht stimmt. Bei 6000 Takten je Kampf ist das der Unterschied
 * zwischen einer Minute und einem Nachmittag.
 *
 * **Abschaltbar**, weil sie Rechenzeit kostet: Sie liest bei jedem Takt den
 * ganzen Zustand. Im Auslieferungsstand läuft sie nie.
 *
 * Verfahren ist FNV-1a über 32 Bit — klein, ohne Speicheranforderung je Aufruf
 * und in jeder Sprache in zehn Zeilen nachzubauen. Das ist die Bedingung dafür,
 * dass der Rust-Simulator dieselbe Folge erzeugt (GAME-DESIGN 18.4).
 */
import { MAX_DECAY_STACKS, type Combat } from './combat.js';

const FNV_OFFSET = 0x811c9dc5;
const FNV_PRIME = 0x01000193;

/** Eine ganze Zahl in die Prüfsumme einrühren. */
export function mixInt(hash: number, value: number): number {
  let h = hash >>> 0;
  let v = value | 0;
  for (let byte = 0; byte < 4; byte++) {
    h = (h ^ (v & 0xff)) >>> 0;
    h = Math.imul(h, FNV_PRIME) >>> 0;
    v >>>= 8;
  }
  return h >>> 0;
}

/**
 * Die Namensfolge des letzten Laufs — **nur für den Gleichlauf-Wächter**
 * (Issue #236).
 *
 * # Warum zur Laufzeit und nicht aus dem Quelltext
 *
 * Eine Quelltext-Stelle ist nicht ein Wert. `combatChecksum` mischt in einer
 * Schleife über die Zauber fünf Werte **je Zauber** — bei drei Zaubern sind
 * das fünfzehn Mischungen aus fünf Zeilen. Wer die Namen aus dem Quelltext
 * zieht, zählt Zeilen und nicht Werte, und bekommt 37 gegen 34, obwohl beide
 * Seiten dasselbe tun.
 *
 * Zur Laufzeit ist außerdem gleichgültig, ob ein Wert direkt oder über eine
 * lokale Variable hereinkommt: gemischt ist gemischt.
 *
 * # Im laufenden Spiel kostet das nichts
 *
 * Aufgezeichnet wird nur, wenn `recording` gesetzt ist — sonst ist es ein
 * Vergleich gegen `null` und kein Eintrag. Der Kampf darf im Takt nichts
 * erzeugen (18.2), und diese Liste entsteht ausschließlich im Prüffall.
 */
let recording: string[] | null = null;

/** Zeichnet die Namensfolge des nächsten Aufrufs auf und gibt sie zurück. */
export function recordFieldOrder(run: () => void): readonly string[] {
  const names: string[] = [];
  recording = names;
  try {
    run();
  } finally {
    recording = null;
  }
  return names;
}

/**
 * Eine ganze Zahl einrühren, unter einem Namen.
 *
 * **Der Name geht NICHT in die Prüfsumme ein.** Täte er es, änderte sich jede
 * Prüfsumme im Projekt, und jede eingecheckte Vergleichsdatei wäre falsch.
 * Er dient allein dazu, die Reihenfolge benennbar zu machen: Meldet der
 * Wächter „bei `damage` läuft es auseinander", ist die Stelle gefunden —
 * statt „Prüfsumme stimmt nicht", was eine Stunde Suche bedeutet hat (#97).
 */
export function mixNamedInt(hash: number, name: string, value: number): number {
  recording?.push(name);
  return mixInt(hash, value);
}

/** Dasselbe für eine Fließkommazahl. Der Name geht auch hier nicht ein. */
export function mixNamedFloat(hash: number, name: string, value: number): number {
  recording?.push(name);
  return mixFloat(hash, value);
}

/**
 * Eine Fließkommazahl einrühren.
 *
 * Über die Bit-Darstellung, nicht über den gerundeten Wert: Zwei Zahlen, die
 * sich im letzten Bit unterscheiden, müssen verschiedene Prüfsummen ergeben —
 * genau diese Unterschiede sucht der Abgleich mit dem Simulator.
 */
const scratch = new Float64Array(1);
const scratchInts = new Int32Array(scratch.buffer);

export function mixFloat(hash: number, value: number): number {
  scratch[0] = value;
  return mixInt(mixInt(hash, scratchInts[0] as number), scratchInts[1] as number);
}

/**
 * Prüfsumme des vollständigen Kampfzustands.
 *
 * **Die Reihenfolge der Felder ist Teil der Schnittstelle** — sie muss im
 * Simulator gleich sein, sonst stimmen die Summen nie überein.
 */
export function combatChecksum(combat: Combat, timeMs: number): number {
  let h = FNV_OFFSET;
  h = mixNamedInt(h, 'time', timeMs);
  h = mixNamedFloat(h, 'damage', combat.damageDealt);
  h = mixNamedInt(h, 'resource', combat.resource);
  h = mixNamedInt(h, 'resourceOverflow', combat.resourceOverflow);
  h = mixNamedInt(h, 'fightState', combat.fightState);
  h = mixNamedFloat(h, 'gcdReadyAt', combat.gcd.readyAt);
  for (let i = 0; i < combat.spells.length; i++) {
    h = mixNamedFloat(h, 'cooldownRemaining', combat.cooldownRemainingMs(i, timeMs));
    h = mixNamedFloat(h, 'dotRemaining', combat.dotRemainingMs(i, timeMs));
    h = mixNamedInt(h, 'casts', combat.castCount(i));
    h = mixNamedInt(h, 'dotTicks', combat.dotTickCount(i));
    h = mixNamedInt(h, 'lostDotTicks', combat.lostDotTicks(i));
  }
  const draws = combat.random.drawCounts();
  for (let i = 0; i < draws.length; i++) h = mixNamedInt(h, 'randomDraws', draws[i] as number);
  // **Angehängt, nicht eingeschoben** (Issue #96, Boss-Phasen und
  // Energieleiste). Die bestehenden Positionen bleiben unverändert; die
  // Reihenfolge ist ohnehin willkürlich und muss nur auf beiden Seiten gleich
  // sein — `sim/src/fight.rs`, `checksum_at`.
  //
  // Ohne diese beiden Felder wäre die Prüfsumme für einen Boss mit Phasen
  // blind: Zwei Läufe in verschiedenen Phasen sähen gleich aus, und der
  // Abgleich meldete Übereinstimmung, wo keine ist.
  h = mixNamedInt(h, 'bossPhase', combat.bossPhase);
  h = mixNamedInt(h, 'bossEnergy', combat.bossEnergy);
  h = mixNamedInt(h, 'timelineIndex', combat.timelineIndex);
  // **Angehängt, nicht eingeschoben** (Issue #110). Ohne die Lebenspunkte wäre
  // die Prüfsumme für einen Boss, der Schaden austeilt, blind: Zwei Läufe mit
  // verschiedenem erlittenem Schaden sähen gleich aus.
  h = mixNamedInt(h, 'playerHp', combat.playerHp);
  // Und der laufende Boss-Zauber, damit ein verpasstes Unterbrechen auffällt,
  // bevor der Treffer überhaupt fällt.
  h = mixNamedInt(h, 'casting', combat.casting);
  // **Angehängt, nicht eingeschoben** (Issue #79). Ohne den Schutz wäre die
  // Prüfsumme für jeden Kampf mit Schutzschicht blind: Zwei Läufe mit
  // verschieden viel aufgefangenem Schaden sähen gleich aus, solange die
  // Lebenspunkte zufällig übereinstimmen.
  h = mixNamedInt(h, 'shield', combat.shield);
  h = mixNamedInt(h, 'shieldStacks', combat.shieldStacks);
  // Die laufende Umwandlung gehört dazu, nicht nur ihr Ergebnis: Sie
  // entscheidet über jeden künftigen Schlag.
  h = mixNamedInt(h, 'conversionPercent', combat.conversionPercent);
  // **Angehängt, nicht eingeschoben** (Issue #100). Ohne den Kontroll-Vorrat
  // wäre die Prüfsumme für jeden Kampf mit Fixierung blind, und die gesenkte
  // Phasen-Schwelle entscheidet über jeden künftigen Wechsel.
  h = mixNamedInt(h, 'control', combat.control);
  h = mixNamedInt(h, 'phaseHoldPercent', combat.phaseHoldPercent);
  // **Angehängt, nicht eingeschoben** (Issue #216, Zähler und Stapel). Ohne
  // sie wäre die Prüfsumme für einen Boss mit diesen Bausteinen blind: Zwei
  // Läufe mit verschiedenem Zählerstand sähen gleich aus, solange die
  // Lebenspunkte zufällig übereinstimmen — und beim Zähler stimmen sie
  // überein, denn er nimmt keine.
  //
  // Die Ladungen gehören dazu: Sie sind der Unterschied zwischen „kann noch
  // senken" und „muss es aushalten", und sie stehen in keiner anderen Größe.
  h = mixNamedInt(h, 'doom', combat.doom);
  h = mixNamedInt(h, 'doomCharges', combat.doomCharges);
  h = mixNamedInt(h, 'bossStacks', combat.bossStacks);
  // **Angehängt, nicht eingeschoben** (Issue #97). Die Feldfolge ist die
  // Schnittstelle zum Simulator; ein neues Feld in der Mitte verschöbe jede
  // eingecheckte Vergleichsdatei auf einen Schlag.
  //
  // Die Zahl der Stapel allein genügt nicht: Zwei Läufe mit gleich vielen,
  // aber verschieden alten Stapeln laufen gleich weiter — bis der erste
  // verfällt. Deshalb geht jede Restzeit einzeln ein, in fester Reihenfolge
  // (der älteste zuerst), und die unbelegten Plätze als 0.
  h = mixNamedInt(h, 'decayStacks', combat.decayStacks);
  for (let i = 0; i < MAX_DECAY_STACKS; i++)
    h = mixNamedInt(h, 'decayExpiry', combat.decayExpiryAt(i));
  // **Angehängt, nicht eingeschoben** (Issue #218). Ohne sie wäre die
  // Prüfsumme für einen Boss mit Unverwundbarkeits-Fenster blind: Zwei Läufe
  // mit verschieden viel verlorenem Schaden sähen gleich aus, solange der
  // ausgeteilte zufällig übereinstimmt — und im Fenster stimmt er überein,
  // denn dort wächst er gar nicht.
  //
  // Der geladene Zähler gehört dazu: Er entscheidet, ob das Fenster früh
  // endet, und steht in keiner anderen Größe.
  h = mixNamedInt(h, 'raceOpen', combat.raceOpen ? 1 : 0);
  h = mixNamedInt(h, 'raceProgress', combat.raceProgress);
  h = mixNamedInt(h, 'damageLostToRace', combat.damageLostToRace);
  h = mixNamedInt(h, 'bossHealed', combat.bossHealed);
  // **Angehängt, nicht eingeschoben** (Issue #240, die letzten drei Bausteine).
  //
  // Baustein 7: Die Eskalation wirkt ausschließlich auf den Zeitpunkt, zu dem
  // der laufende Boss-Zauber trifft. Ohne ihn fiele eine falsch gerechnete
  // Verkürzung erst auf, wenn der Treffer fällt — also Takte später, an einer
  // Stelle, die nicht mehr auf die Ursache zeigt. Denselben Wert verschiebt die
  // Fixierung (#100), für die er bisher ebenfalls fehlte.
  //
  // **Nur solange einer läuft.** Ohne laufenden Zauber trägt das Feld im Spiel
  // -1 und im Simulator 0 — ein Unterschied, der nichts bedeutet, aber in jedem
  // Takt ohne Boss-Zauber eine Abweichung meldete. Der feste Wert steht deshalb
  // auf beiden Seiten.
  h = mixNamedInt(h, 'castLandsAt', combat.casting >= 0 ? combat.castLandsAtMs : -1);
  // Baustein 8: Ob die Schwächung liegt, entscheidet über jeden weiteren Takt.
  // Der Zähler gehört dazu, weil er den Unterschied zwischen „nie gelegen" und
  // „gelegen und entfernt" trägt — im Zustand sehen die beiden gleich aus.
  h = mixNamedInt(h, 'afflicted', combat.afflicted ? 1 : 0);
  h = mixNamedInt(h, 'afflictionsCleared', combat.afflictionsCleared);
  // Issue #256: Der Anteil des SPIELERS am Ring-Verlust. `damage` steht schon
  // an Position 2, aber es mischt seit #247 zwei Herkünfte — die Reaktion
  // senkt den Ring von selbst. Zwei Läufe mit gleichem `damage` und
  // verschiedenem Spieler-Anteil sähen ohne dieses Feld identisch aus, und ein
  // Simulator, der die Trennung anders zieht, fiele nicht auf.
  //
  // **Angehängt, nicht eingeschoben** — wie jedes Feld seit #96. Der Name
  // kommt aus #236 dazu: Der Gleichlauf-Wächter nennt bei einer Abweichung die
  // Stelle beim Namen, statt nur ihre Nummer zu melden.
  h = mixNamedFloat(h, 'playerDamage', combat.playerDamage);
  // Issue #304: die laufende Wirkzeit des SPIELERS. Bis hierher trug die
  // Prüfsumme nur den Zustand des Bosses (`casting`, `castLandsAt`) — für den
  // Spieler gab es keinen, weil eine Wirkzeit nie lief.
  //
  // **Ohne diese zwei Felder wäre die Prüfung an ihrer empfindlichsten Stelle
  // blind:** Ein Zauber, der auf einer Seite eine Wirkzeit hat und auf der
  // anderen sofort trifft, ergäbe in jedem Takt dazwischen dieselbe Summe —
  // gleicher Schaden (noch keiner), gleiche Abklingzeit (noch keine), gleiche
  // Wartezeit. Erst der Treffer liefe auseinander, und die Prüfsumme zeigte auf
  // den Takt des Treffers statt auf den des Fehlers.
  //
  // **Angehängt, nicht eingeschoben** — wie jedes Feld seit #96.
  h = mixNamedInt(h, 'playerCasting', combat.playerCasting);
  // Dieselbe Konvention wie `castLandsAt` eine Handvoll Zeilen höher: ohne
  // laufende Wirkzeit ein fester Wert auf beiden Seiten. Sonst meldete jeder
  // Takt ohne Wirkzeit eine Abweichung, die nichts bedeutet.
  h = mixNamedInt(
    h,
    'playerCastEndsAt',
    combat.playerCasting >= 0 ? combat.playerCastEndsAtMs : -1,
  );
  // Issue #318: Die Synergie-Fenster. Ohne sie liefe ein Simulator, der das
  // Fenster anders lang rechnet oder es nicht verbraucht, still auseinander —
  // der Schaden wiche erst dann ab, wenn der Partner wirklich im Fenster
  // wirkt, und das kann Takte später sein oder nie.
  //
  // **Der offene Zustand gehört dazu, nicht nur die Zähler.** Zwei Läufe mit
  // gleicher Ausbeute können verschieden weit im Fenster stehen; ohne
  // `synergyOpenUntil` sähen sie hier gleich aus, und der Unterschied schlüge
  // erst beim nächsten Druck durch.
  //
  // **Angehängt, nicht eingeschoben** — wie jedes Feld seit #96. Ein
  // eingefügter Wert verschiebt jede Prüfsumme danach, und die Abweichung
  // zeigt dann auf die falsche Stelle.
  h = mixNamedInt(h, 'synergyUsed', combat.synergyUsed);
  h = mixNamedInt(h, 'synergyAvailable', combat.synergyAvailable);
  h = mixNamedFloat(h, 'synergyOpenUntil', combat.synergyWindowSum);
  return h >>> 0;
}
