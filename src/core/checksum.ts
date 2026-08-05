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
  h = mixInt(h, timeMs);
  h = mixFloat(h, combat.damageDealt);
  h = mixInt(h, combat.resource);
  h = mixInt(h, combat.resourceOverflow);
  h = mixInt(h, combat.fightState);
  h = mixFloat(h, combat.gcd.readyAt);
  for (let i = 0; i < combat.spells.length; i++) {
    h = mixFloat(h, combat.cooldownRemainingMs(i, timeMs));
    h = mixFloat(h, combat.dotRemainingMs(i, timeMs));
    h = mixInt(h, combat.castCount(i));
    h = mixInt(h, combat.dotTickCount(i));
    h = mixInt(h, combat.lostDotTicks(i));
  }
  const draws = combat.random.drawCounts();
  for (let i = 0; i < draws.length; i++) h = mixInt(h, draws[i] as number);
  // **Angehängt, nicht eingeschoben** (Issue #96, Boss-Phasen und
  // Energieleiste). Die bestehenden Positionen bleiben unverändert; die
  // Reihenfolge ist ohnehin willkürlich und muss nur auf beiden Seiten gleich
  // sein — `sim/src/fight.rs`, `checksum_at`.
  //
  // Ohne diese beiden Felder wäre die Prüfsumme für einen Boss mit Phasen
  // blind: Zwei Läufe in verschiedenen Phasen sähen gleich aus, und der
  // Abgleich meldete Übereinstimmung, wo keine ist.
  h = mixInt(h, combat.bossPhase);
  h = mixInt(h, combat.bossEnergy);
  h = mixInt(h, combat.timelineIndex);
  // **Angehängt, nicht eingeschoben** (Issue #110). Ohne die Lebenspunkte wäre
  // die Prüfsumme für einen Boss, der Schaden austeilt, blind: Zwei Läufe mit
  // verschiedenem erlittenem Schaden sähen gleich aus.
  h = mixInt(h, combat.playerHp);
  // Und der laufende Boss-Zauber, damit ein verpasstes Unterbrechen auffällt,
  // bevor der Treffer überhaupt fällt.
  h = mixInt(h, combat.casting);
  // **Angehängt, nicht eingeschoben** (Issue #79). Ohne den Schutz wäre die
  // Prüfsumme für jeden Kampf mit Schutzschicht blind: Zwei Läufe mit
  // verschieden viel aufgefangenem Schaden sähen gleich aus, solange die
  // Lebenspunkte zufällig übereinstimmen.
  h = mixInt(h, combat.shield);
  h = mixInt(h, combat.shieldStacks);
  // Die laufende Umwandlung gehört dazu, nicht nur ihr Ergebnis: Sie
  // entscheidet über jeden künftigen Schlag.
  h = mixInt(h, combat.conversionPercent);
  // **Angehängt, nicht eingeschoben** (Issue #100). Ohne den Kontroll-Vorrat
  // wäre die Prüfsumme für jeden Kampf mit Fixierung blind, und die gesenkte
  // Phasen-Schwelle entscheidet über jeden künftigen Wechsel.
  h = mixInt(h, combat.control);
  h = mixInt(h, combat.phaseHoldPercent);
  // **Angehängt, nicht eingeschoben** (Issue #216, Zähler und Stapel). Ohne
  // sie wäre die Prüfsumme für einen Boss mit diesen Bausteinen blind: Zwei
  // Läufe mit verschiedenem Zählerstand sähen gleich aus, solange die
  // Lebenspunkte zufällig übereinstimmen — und beim Zähler stimmen sie
  // überein, denn er nimmt keine.
  //
  // Die Ladungen gehören dazu: Sie sind der Unterschied zwischen „kann noch
  // senken" und „muss es aushalten", und sie stehen in keiner anderen Größe.
  h = mixInt(h, combat.doom);
  h = mixInt(h, combat.doomCharges);
  h = mixInt(h, combat.bossStacks);
  // **Angehängt, nicht eingeschoben** (Issue #97). Die Feldfolge ist die
  // Schnittstelle zum Simulator; ein neues Feld in der Mitte verschöbe jede
  // eingecheckte Vergleichsdatei auf einen Schlag.
  //
  // Die Zahl der Stapel allein genügt nicht: Zwei Läufe mit gleich vielen,
  // aber verschieden alten Stapeln laufen gleich weiter — bis der erste
  // verfällt. Deshalb geht jede Restzeit einzeln ein, in fester Reihenfolge
  // (der älteste zuerst), und die unbelegten Plätze als 0.
  h = mixInt(h, combat.decayStacks);
  for (let i = 0; i < MAX_DECAY_STACKS; i++) h = mixInt(h, combat.decayExpiryAt(i));
  // **Angehängt, nicht eingeschoben** (Issue #218). Ohne sie wäre die
  // Prüfsumme für einen Boss mit Unverwundbarkeits-Fenster blind: Zwei Läufe
  // mit verschieden viel verlorenem Schaden sähen gleich aus, solange der
  // ausgeteilte zufällig übereinstimmt — und im Fenster stimmt er überein,
  // denn dort wächst er gar nicht.
  //
  // Der geladene Zähler gehört dazu: Er entscheidet, ob das Fenster früh
  // endet, und steht in keiner anderen Größe.
  h = mixInt(h, combat.raceOpen ? 1 : 0);
  h = mixInt(h, combat.raceProgress);
  h = mixInt(h, combat.damageLostToRace);
  h = mixInt(h, combat.bossHealed);
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
  h = mixInt(h, combat.casting >= 0 ? combat.castLandsAtMs : -1);
  // Baustein 8: Ob die Schwächung liegt, entscheidet über jeden weiteren Takt.
  // Der Zähler gehört dazu, weil er den Unterschied zwischen „nie gelegen" und
  // „gelegen und entfernt" trägt — im Zustand sehen die beiden gleich aus.
  h = mixInt(h, combat.afflicted ? 1 : 0);
  h = mixInt(h, combat.afflictionsCleared);
  return h >>> 0;
}
