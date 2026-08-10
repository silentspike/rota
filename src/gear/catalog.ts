/**
 * Der Gegenstands-Katalog und der Weg zu den getragenen Werten (Issue #421a).
 *
 * WAS BIS HIERHER FEHLTE
 * ----------------------
 * `worn → essenceFactor → factors` ist seit #282 verdrahtet und wurde zu Recht
 * abgenommen — dieser Teil rechnet. Aber er beginnt bei `worn`, und **niemand
 * füllte `worn`**: Im ganzen Projekt gab es genau zwei `WornValues`, beide von
 * Hand geschrieben (`NOTHING_WORN` und einer für die Paritätsprüfung).
 *
 * Ein leeres `NOTHING_WORN` durchläuft die verdrahtete Kette fehlerfrei und
 * ergibt exakt die Werte eines nackten Spielers. Deshalb war nichts kaputt und
 * trotzdem nichts angeschlossen.
 *
 * KEIN GEGENSTAND WIRD VON HAND BALANCIERT
 * ----------------------------------------
 * GAME-DESIGN 13.3 (Zeile 1437): „Aus Gegenstandsstufe und Qualität errechnet
 * sich ein Wertebudget, das auf die Werte-Plätze verteilt wird. **Kein
 * Gegenstand wird von Hand balanciert** — ein neuer Gegenstand ist ein Eintrag
 * mit Stufe, Qualität und Werte-Arten."
 *
 * Deshalb schlägt `wornFrom` nach und **rechnet**, statt Zahlen aus dem Katalog
 * zu lesen. Die Rechenstelle steht seit #114 in `budget.ts` und `sources.ts` —
 * sie war es, die kein Produktionscode benutzte.
 *
 * DIE GRUNDWERTE KOMMEN ZUSÄTZLICH
 * --------------------------------
 * 13.5 und Zeile 1422: „Ein Gegenstand trägt **immer** die drei Grundwerte
 * ESSENCE, VESSEL und WARD." Immer heisst: nicht je Eintrag genannt. Sie
 * folgen aus der Gegenstandsstufe, wie die Rüstung eines Diablo-Gegenstands —
 * die `affixKinds` nennen nur die Affixe.
 */
import rawItems from '../../data/items.json';
import { distributeBudget, MAX_ITEM_LEVEL, QUALITY_AFFIX_SLOTS, QUALITY_COUNT } from './budget.js';
import { POOL_NAMES } from './pools.js';
import { SLOT_COUNT } from './slots.js';
import { basePermille, ESSENCE_AT_LEVEL_1, VESSEL_AT_LEVEL_1, WARD_AT_LEVEL_1 } from './sources.js';
import { feldFuerAffixArt, NOTHING_WORN, type WornValues } from './worn.js';

/**
 * Die Arten, die wirklich einen Wert am Spieler füllen.
 *
 * **Abgeleitet, nicht abgeschrieben** — sie steht nur in der Fehlermeldung.
 * Eine zweite Liste von Hand wäre genau die zweite Wahrheit, deren Fehlen den
 * stillen Verlust erst möglich gemacht hat.
 */
const WERTBARE_AFFIX_ARTEN: readonly string[] = POOL_NAMES.filter(
  (kind) => feldFuerAffixArt(kind) !== null,
);

/**
 * Prüft die Werte-Arten eines Eintrags und wirft beim ersten Fehler.
 *
 * # Warum das eine eigene Funktion ist
 *
 * Der Katalog wird beim Laden des Moduls geprüft (das `ITEMS`-IIFE unten).
 * Eine Prüfung, die nur dort steht, lässt sich nicht messen, ohne
 * `data/items.json` zu ändern — und eine Prüfung, die nie rot war, ist eine
 * Behauptung. Derselbe Aufruf steht im Ladeweg; hier ist er auch erreichbar.
 *
 * # Zwei verschiedene Fehler, und der zweite ist der stille
 *
 * 1. **Die Art gibt es nicht** — ein Tippfehler. Fällt beim Laden auf.
 * 2. **Die Art ist ein Topf ohne Wert.** `POOL_NAMES` führt die Töpfe aus
 *    13.1b, `WornValues` die Werte, die der Kampf liest. `active` ist ein
 *    Topf ohne Wert, weil aktive Verstärkungen im Kampf entstehen und nicht
 *    auf einem Gegenstand stehen.
 *
 * Der zweite blieb bis #421b unbemerkt: Ein Eintrag mit `active` lud
 * fehlerfrei und legte seinen Anteil in ein Feld, das niemand liest —
 * gemessen **406 von 1219 Promille**, ohne Ausnahme, ohne roten Fall, ohne
 * Log. Der Gegenstand sah vollwertig aus und war ein Drittel schwächer.
 *
 * **Die Prüfung gehört ins Laden und nicht in `wornFrom`:** Ein Katalog, der
 * nicht lädt, ist sichtbar. Ein Kampf, der still weniger rechnet, nicht.
 */
export function pruefeAffixArten(id: string, affixKinds: readonly string[]): void {
  for (const kind of affixKinds) {
    if (!POOL_NAMES.includes(kind)) {
      throw new Error(
        `data/items.json: "${id}" nennt die Werte-Art "${kind}", die es nicht gibt. ` +
          `Erlaubt: ${POOL_NAMES.join(', ')}`,
      );
    }
    if (feldFuerAffixArt(kind) === null) {
      throw new Error(
        `data/items.json: "${id}" nennt die Werte-Art "${kind}", die zwar ein Topf ist, ` +
          `aber keinen Wert am Spieler füllt. Ein Gegenstand damit wäre stillschweigend ` +
          `schwächer. Wertbare Arten: ${WERTBARE_AFFIX_ARTEN.join(', ')}`,
      );
    }
  }
}

/** Ein Eintrag im Katalog — Stufe, Qualität und Werte-Arten, keine Zahlen. */
export interface ItemDef {
  readonly id: string;
  readonly name: string;
  /** Auf welchem der zehn Ausrüstungsplätze er getragen wird. */
  readonly slot: number;
  readonly itemLevel: number;
  /** 0 common, 1 rare, 2 epic, 3 legendary. */
  readonly quality: number;
  /** Welche Töpfe die Affixe füllen — Namen aus `POOL_NAMES`. */
  readonly affixKinds: readonly string[];
  /**
   * An welchen Boss dieses Stück gebunden ist, oder `''` für „überall"
   * (Issue #447).
   *
   * 13.3 nennt Boss-Bindungen als Teil des Katalogs. `mayDropAt` prüft sie
   * seit E6-2 und hatte keinen Aufrufer — heute trägt kein Eintrag eine
   * Bindung, und die Prüfung steht trotzdem: Wer sie erst mit dem ersten
   * gebundenen Stück einbaut, hat bis dahin einen Katalog, in dem sie überall
   * fallen.
   */
  readonly boundToBoss: string;
}

/**
 * Wie oft ein Sieg überhaupt etwas fallen lässt, in Promille (Issue #447).
 *
 * **VORLÄUFIG.** Keine der zehn Funktionen in `src/loot/` sagt, ob etwas
 * fällt — nur was. Die Häufigkeit bestimmt, wie sich ein Sieg anfühlt, und
 * gehört dem Auftraggeber. Sie steht deshalb in den Daten und nicht im Code:
 * Der Wechsel ist ein Zahlenwert, kein Umbau.
 */
export const DROP_CHANCE_PER_MILLE: number = (() => {
  const wert = Math.trunc(
    Number((rawItems as { dropChancePerMille?: unknown }).dropChancePerMille),
  );
  if (!Number.isFinite(wert) || wert < 0 || wert > 1000) {
    throw new Error(
      `data/items.json: dropChancePerMille ist ${String(wert)} — erlaubt sind 0 bis 1000. ` +
        'Bei 0 fällt nie etwas, bei 1000 nach jedem Sieg; beides ist eine Aussage, ' +
        'alles darüber hinaus ein Tippfehler.',
    );
  }
  return wert;
})();

/**
 * Alle Gegenstände.
 *
 * Die Prüfung bricht hart ab, wie bei `base.json` und `levels.json`: Das ist
 * keine Inhaltsdatei, sondern eine Voraussetzung. Ein Eintrag mit mehr
 * Werte-Arten als Plätzen ergäbe einen Gegenstand, dessen letzte Arten still
 * verfielen — und das fiele niemandem auf, weil er trotzdem etwas trägt.
 */
export const ITEMS: readonly ItemDef[] = (() => {
  const roh = rawItems as { items?: unknown[] };
  const alle: ItemDef[] = [];
  const gesehen = new Set<string>();

  for (const eintrag of roh.items ?? []) {
    const t = eintrag as Record<string, unknown>;
    const id = typeof t['id'] === 'string' ? t['id'] : '';
    if (id.length === 0) throw new Error('data/items.json: ein Gegenstand ohne Kennung.');
    // **Kennungen sind der Schlüssel im Spielstand.** Zwei gleiche wären dort
    // ein Eintrag, und der Fund des einen zählte für den anderen.
    if (gesehen.has(id)) {
      throw new Error(`data/items.json: die Kennung "${id}" kommt zweimal vor.`);
    }
    gesehen.add(id);

    const itemLevel = Math.trunc(Number(t['itemLevel'] ?? 0));
    if (!Number.isInteger(itemLevel) || itemLevel < 1 || itemLevel > MAX_ITEM_LEVEL) {
      throw new Error(
        `data/items.json: "${id}" hat Stufe ${String(itemLevel)} — erlaubt sind 1 bis ` +
          `${String(MAX_ITEM_LEVEL)}.`,
      );
    }
    const quality = Math.trunc(Number(t['quality'] ?? -1));
    if (!Number.isInteger(quality) || quality < 0 || quality >= QUALITY_COUNT) {
      throw new Error(
        `data/items.json: "${id}" hat Qualität ${String(quality)} — erlaubt sind 0 bis ` +
          `${String(QUALITY_COUNT - 1)}.`,
      );
    }
    const slot = Math.trunc(Number(t['slot'] ?? -1));
    if (!Number.isInteger(slot) || slot < 0 || slot >= SLOT_COUNT) {
      throw new Error(
        `data/items.json: "${id}" liegt auf Platz ${String(slot)} — es gibt ` +
          `${String(SLOT_COUNT)}.`,
      );
    }

    const affixKinds = Array.isArray(t['affixKinds'])
      ? (t['affixKinds'] as unknown[]).map((x) => String(x))
      : [];
    const plaetze = QUALITY_AFFIX_SLOTS[quality] ?? 0;
    // **Mehr Arten als Plätze wäre still**: Die überzähligen bekämen nie einen
    // Anteil am Budget, und der Gegenstand sähe im Katalog reicher aus, als er
    // im Kampf ist.
    if (affixKinds.length > plaetze) {
      throw new Error(
        `data/items.json: "${id}" nennt ${String(affixKinds.length)} Werte-Arten, hat bei ` +
          `Qualität ${String(quality)} aber ${String(plaetze)} Affix-Plätze.`,
      );
    }
    pruefeAffixArten(id, affixKinds);

    alle.push({
      id,
      name: typeof t['name'] === 'string' ? t['name'] : id,
      slot,
      itemLevel,
      quality,
      affixKinds,
      boundToBoss: typeof t['boundToBoss'] === 'string' ? t['boundToBoss'] : '',
    });
  }
  return alle;
})();

/** Ein Gegenstand nach seiner Kennung, oder `undefined`. */
export function itemById(id: string): ItemDef | undefined {
  return ITEMS.find((item) => item.id === id);
}

/** Was ein Spielstand über einen besessenen Gegenstand hält. */
export interface OwnedItemLike {
  readonly itemId: string;
  readonly upgradeSteps: number;
  readonly equippedSlot: number | null;
}

/**
 * Die getragenen Werte aus den ausgerüsteten Stücken.
 *
 * **Das fehlende mittlere Glied.** Es stand in keinem Kriterium von #421 — das
 * Issue hiess „die Kette ist an drei Stellen offen" und war an vier offen.
 *
 * Gerechnet, nicht nachgeschlagen: `distributeBudget` verteilt das Budget aus
 * Stufe und Qualität auf die Affix-Plätze, `basePermille` liefert die drei
 * Grundwerte. Beide standen seit #114 da und hatten keinen Aufrufer.
 *
 * **Nur was auf einem Platz liegt.** Ein Stück im Beutel (`equippedSlot` ist
 * `null`) trägt nichts bei — sonst wäre Sammeln stärker als Ausrüsten.
 */
export function wornFrom(items: readonly OwnedItemLike[]): WornValues {
  const summe = { ...NOTHING_WORN } as Record<string, number>;
  const anteile: number[] = [0, 0, 0, 0];
  let getragen = 0;

  for (const besitz of items) {
    if (besitz.equippedSlot === null) continue;
    const def = itemById(besitz.itemId);
    // **Ein unbekannter Gegenstand wird übergangen, nicht abgebrochen.** Ein
    // Spielstand kann ein Stück tragen, das es nach einem Update nicht mehr
    // gibt; ein Kampf, der deswegen nicht startet, wäre schlimmer als einer,
    // der ohne dieses Stück rechnet.
    if (def === undefined) continue;
    getragen++;

    // Die drei Grundwerte trägt JEDER Gegenstand (13.5) — sie stehen nicht im
    // Eintrag und werden deshalb hier addiert, nicht dort gelesen.
    summe['essence'] = (summe['essence'] ?? 0) + basePermille(def.itemLevel, ESSENCE_AT_LEVEL_1);
    summe['vessel'] = (summe['vessel'] ?? 0) + basePermille(def.itemLevel, VESSEL_AT_LEVEL_1);
    summe['ward'] = (summe['ward'] ?? 0) + basePermille(def.itemLevel, WARD_AT_LEVEL_1);

    const anzahl = distributeBudget(def.itemLevel, def.quality, anteile);
    for (let i = 0; i < anzahl && i < def.affixKinds.length; i++) {
      const kind = def.affixKinds[i] ?? '';
      // **Dieselbe Abbildung wie in der Katalog-Prüfung** — eine Wahrheit,
      // zwei Aufrufer. Stünde sie hier ein zweites Mal als eigenes Ternary,
      // könnte die Prüfung eine Art erlauben, die das Rechnen nicht kennt;
      // genau diese Lücke war der stille Verlust.
      //
      // **Laut, nicht still.** Hierher kommt nichts, was `pruefeAffixArten`
      // durchgelassen hat — aber wenn doch, dann als Ausnahme und nicht als
      // ein Drittel weniger Werte.
      const feld = feldFuerAffixArt(kind);
      if (feld === null) {
        throw new Error(
          `wornFrom: "${besitz.itemId}" trägt die Werte-Art "${kind}" ohne Feld. ` +
            `Der Ladeweg hätte das fangen müssen (pruefeAffixArten).`,
        );
      }
      summe[feld] = (summe[feld] ?? 0) + (anteile[i] ?? 0);
    }
  }

  return getragen === 0 ? NOTHING_WORN : (summe as unknown as WornValues);
}
