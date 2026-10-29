/**
 * Die Regeln ÜBER dem Kampf, aus dem laufenden Spielcode (Issue #922, E11.4e).
 *
 * # Wozu
 *
 * Die Fortschritts-Simulation im Simulator (`sim/src/laufbahn/regeln.rs`)
 * rechnet Beute, Material, Aufwertung, Punkte, Erfahrung, Freischaltung, die
 * getragenen Werte und den automatischen Vergleich nach. Bis #922 brauchte das
 * niemand — `data/player/reward.json` hielt ausdrücklich fest, der Simulator
 * rechne keine Progression und es gebe deshalb keine Paritätspflicht. Mit der
 * Fortschritts-Simulation gibt es sie. Diese Datei ist die Spielseite davon.
 *
 * # Jede Zahl stammt aus einem Aufruf, keine aus einer Abschrift
 *
 * Gerufen werden die Funktionen, die das Spiel selbst benutzt: `rollLoot` mit
 * `lootStreamFor`, `upgradeItem`, `pointsBreakdown`, `xpForVictory`,
 * `grantXp`, `freigeschalteteInstanzen`, `wornFrom`, `estimateDamagePerHit`,
 * `compareBagItem` und `erwarteterStand`. Ändert jemand eine dieser Regeln,
 * ändert sich die Datei — und `freshness.test.ts` verlangt den Neuschrieb.
 *
 * # Warum Zeilen statt Objekte
 *
 * Über 5000 Fälle, je ein Objekt, ergäben mit `serialize` sechsstellige
 * Zeilenzahlen, und ein Diff wäre unlesbar. Jeder Fall steht deshalb als EINE
 * Zeile: Eingaben, ein senkrechter Strich, Ergebnisse. `-` heißt `null`.
 * Gleitkommazahlen stehen als Bitmuster (`bitsOf`), aus dem Grund, den
 * `bitsOf` selbst nennt.
 */
import { BOSSES, INSTANCES, LEVELS, REWARD } from '../../src/core/data.js';
import { seedForStream } from '../../src/core/rng.js';
import { markKey } from '../../src/analysis/progress-mark.js';
import { erwarteterStand } from '../../src/devtools/expected-gear.js';
import { QUALITY_COUNT } from '../../src/gear/budget.js';
import {
  DROP_CHANCE_PER_MILLE,
  ITEMS,
  wornFrom,
  type ItemDef,
  type OwnedItemLike,
} from '../../src/gear/catalog.js';
import { effectiveLevelOf } from '../../src/gear/instance.js';
import { NO_SET } from '../../src/gear/sets.js';
import type { WornValues } from '../../src/gear/worn.js';
import { compareBagItem, estimateDamagePerHit } from '../../src/loot/estimate.js';
import {
  itemLevelOf,
  progressOf,
  qualityCapOf,
  weightsFor,
  WEIGHT_TOTAL,
} from '../../src/loot/rates.js';
import { candidatesFor, rollLoot } from '../../src/loot/roll.js';
import { LOOT_STREAM_INDEX, lootStreamFor } from '../../src/loot/stream.js';
import {
  MATERIAL_PER_LEVEL,
  MATERIAL_PER_QUALITY,
  materialFor,
  upgradeCapOf,
  upgradeItem,
} from '../../src/loot/upgrade.js';
import { freigeschalteteInstanzen } from '../../src/progression/boss-select.js';
import { grantXp, xpForVictory } from '../../src/progression/level.js';
import { pointsBreakdown } from '../../src/progression/points.js';
import { bitsOf, SCHEMA } from './payloads.js';

export const LAUFBAHN_REGELN_NAME = 'laufbahn-regeln';

/** Ein Wert in einer Zeile: `null` als `-`, Wahrheitswerte als 0/1. */
function feld(wert: number | boolean | string | null): string {
  if (wert === null) return '-';
  if (typeof wert === 'boolean') return wert ? '1' : '0';
  return String(wert);
}

function zeile(
  ein: readonly (number | boolean | string | null)[],
  aus: readonly (number | boolean | string | null)[],
): string {
  return `${ein.map(feld).join(' ')} | ${aus.map(feld).join(' ')}`;
}

/**
 * Eine feste Folge von Startwerten — die Ränder und ein Mischgenerator.
 *
 * Der Generator ist der aus `RandomStream` selbst nicht, mit Absicht: Die
 * Eingaben eines Vergleichs sollen nicht aus dem Verfahren stammen, das
 * verglichen wird.
 */
function startwerte(anzahl: number): number[] {
  const raus = [0, 1, 2, 3, 7, 42, 1000, 123456789, 2147483647, 2147483648, 4294967295];
  let x = 0x2545f491;
  while (raus.length < anzahl) {
    x = (Math.imul(x, 1103515245) + 12345) >>> 0;
    raus.push(x);
  }
  return raus;
}

/** Ein Katalog mit genau einem ungebundenen Stück — der Einhängepunkt aus `rollLoot`. */
function einzelkatalog(): ItemDef[] {
  const erstes = ITEMS.find((item) => item.boundToBoss === '' && item.setId === NO_SET);
  if (erstes === undefined) throw new Error('kein ungebundenes Stück ohne Satz im Katalog');
  return [erstes];
}

function beuteTabelle(): string[] {
  const raus: string[] = [];
  const gewichte = new Int32Array(QUALITY_COUNT);
  for (let instanz = -1; instanz <= 15; instanz++) {
    for (let schwierigkeit = -1; schwierigkeit <= 3; schwierigkeit++) {
      for (const endlos of [-1, 0, 1, 3, 7, 20]) {
        weightsFor(instanz, schwierigkeit, endlos, gewichte);
        raus.push(
          zeile(
            [instanz, schwierigkeit, endlos],
            [
              ...Array.from(gewichte),
              qualityCapOf(instanz),
              progressOf(instanz, schwierigkeit, endlos),
              itemLevelOf(instanz, schwierigkeit, endlos),
            ],
          ),
        );
      }
    }
  }
  return raus;
}

/**
 * Die Kontexte der Würfe: echte Bosse, ein Prüfboss und Ränder der drei Achsen.
 *
 * **Instanz 2 auf mythisch steht mit Absicht dabei**: Dort trägt die
 * Gewichtszeile 300 für Epic, die Obergrenze lässt aber nur Rare zu — der
 * einzige Kontext hier, in dem `weightsFor` gesperrtes Gewicht umverteilt.
 * Auf normal kommt das in keiner Instanz vor, und ohne ihn sähen die Würfe
 * eine fehlende Umverteilung nicht (Gegenprobe am 2026-10-29).
 */
const WURF_KONTEXTE: readonly [number, number, number, string][] = [
  [1, 0, 0, 'first-crucible'],
  [2, 0, 0, 'boss-2-1'],
  [2, 2, 0, 'boss-2-1'],
  [4, 0, 0, 'boss-4-1'],
  [5, 0, 0, 'boss-5-3'],
  [9, 2, 3, 'boss-1-1'],
  [14, 2, 10, 'test-warden'],
];

function wuerfe(): string[] {
  const raus: string[] = [];
  const kataloge: readonly [string, readonly ItemDef[]][] = [
    ['alle', ITEMS],
    ['einer', einzelkatalog()],
  ];
  for (const seed of startwerte(40)) {
    for (const [instanz, schwierigkeit, endlos, boss] of WURF_KONTEXTE) {
      for (const [katalogName, katalog] of kataloge) {
        for (const rate of [DROP_CHANCE_PER_MILLE, 100, 0]) {
          const strom = lootStreamFor(seed);
          const fund = rollLoot(
            strom,
            { instance: instanz, difficulty: schwierigkeit, endlessLevel: endlos, bossId: boss },
            katalog,
            rate,
          );
          raus.push(
            zeile(
              [seed, instanz, schwierigkeit, endlos, boss, katalogName, rate],
              [
                fund === null ? null : fund.itemId,
                fund === null ? null : fund.itemLevel,
                fund === null ? null : fund.quality,
                strom.drawCount,
              ],
            ),
          );
        }
      }
    }
  }
  return raus;
}

function aufwertung(): string[] {
  const raus: string[] = [];
  const inhalte: readonly [number, number, number][] = [
    [1, 0, 0],
    [2, 0, 0],
    [5, 0, 0],
    [14, 2, 0],
    [9, 1, 3],
    [0, 0, 0],
  ];
  for (const stufe of [-1, 0, 1, 2, 6, 7, 8, 13, 25, 26, 40, 100]) {
    for (const material of [-1, 0, 3, 4, 5, 8, 23, 24, 1000, 1_000_000_000]) {
      for (const [instanz, schwierigkeit, endlos] of inhalte) {
        const r = upgradeItem(stufe, material, instanz, schwierigkeit, endlos);
        raus.push(
          zeile(
            [stufe, material, instanz, schwierigkeit, endlos],
            [r.level, r.spent, r.cappedByContent, upgradeCapOf(instanz, schwierigkeit, endlos)],
          ),
        );
      }
    }
  }
  return raus;
}

function punkte(): string[] {
  const raus: string[] = [];
  // Restzeiten mit Bruchteil stehen mit Absicht dabei: Der Simulator reicht
  // Mittelwerte über Durchgänge herein, und die müssen nicht ganzzahlig sein.
  const reserven = [
    0, 0.25, 1, 499, 500, 999, 1000, 1001, 12345, 15000, 20000, 20500.5, 45000, 59999, 60000, -5,
  ];
  const ringe = [0, 12.5, 37.5, 99.9, 100, -3];
  const quellen: [boolean, number, number | null][] = [];
  for (const sieg of [true, false]) {
    for (const reserve of reserven) quellen.push([sieg, reserve, null]);
    for (const ring of ringe) quellen.push([sieg, 0, ring]);
  }
  const stufen: readonly [number, number][] = [
    [1, 1],
    [1, 3],
    [1, 4],
    [1, 12],
    [1, 43],
    [5, 8],
  ];
  const bestwerte = [null, 0, 10000, 20000.5, 60000, 37.5];
  for (const [sieg, reserve, ring] of quellen) {
    for (const [bossStufe, spielerStufe] of stufen) {
      for (const bestwert of bestwerte) {
        const r = pointsBreakdown(
          { victory: sieg, remainingMs: reserve, heldPercent: ring },
          { bossLevel: bossStufe, playerLevel: spielerStufe },
          bestwert,
        );
        raus.push(
          zeile(
            [sieg, reserve, ring, bossStufe, spielerStufe, bestwert],
            [bitsOf(r.units), r.base, r.previousBase, r.repeatOnly, r.falloffPerMille, r.awarded],
          ),
        );
      }
    }
  }
  return raus;
}

function erfahrung(): { sieg: string[]; aufstieg: string[] } {
  const sieg: string[] = [];
  for (const bossStufe of [1, 2, 5, 30]) {
    for (const spielerStufe of [1, 2, 3, 4, 5, 6, 7, 8, 12, 43, 99, 100]) {
      sieg.push(zeile([bossStufe, spielerStufe], [xpForVictory(bossStufe, spielerStufe)]));
    }
  }
  const aufstieg: string[] = [];
  for (const stufe of [1, 2, 5, 99, 100]) {
    for (const xp of [0, 150]) {
      for (const menge of [0, -5, 1, 50, 199, 200, 201, 766, 5000, 1_000_000, 100_000_000]) {
        const r = grantXp({ level: stufe, xp }, menge);
        aufstieg.push(zeile([stufe, xp, menge], [r.level, r.xp]));
      }
    }
  }
  return { sieg, aufstieg };
}

/** Die ausgelieferten Bosse in Instanz- und Dateireihenfolge. */
function spielbosse(): string[] {
  const reihenfolge = new Map(INSTANCES.map((i) => [i.id, i.order]));
  return [...BOSSES.values()]
    .map((eintrag) => eintrag.value)
    .filter((boss) => reihenfolge.has(boss.instance))
    .sort((a, b) => (reihenfolge.get(a.instance) ?? 0) - (reihenfolge.get(b.instance) ?? 0))
    .map((boss) => boss.id);
}

function freischaltung(): string[] {
  const bosse = spielbosse();
  const faelle: string[][] = [];
  for (let n = 0; n <= bosse.length; n++) faelle.push(bosse.slice(0, n));
  // Lücken: je Instanz alle bis auf einen, damit "vollständig" auffällt.
  for (let ausgelassen = 0; ausgelassen < bosse.length; ausgelassen++) {
    faelle.push(bosse.filter((_, i) => i !== ausgelassen));
  }
  const raus: string[] = [];
  for (const besiegt of faelle) {
    // Abwechselnd normal und heroisch: `freigeschalteteInstanzen` löst die
    // Kennung aus dem Schlüssel, die Stufe darf nicht zählen.
    const schluessel = besiegt.map((id, i) => markKey(id, i % 2));
    const offen = freigeschalteteInstanzen(schluessel, BOSSES);
    raus.push(`${besiegt.length === 0 ? '-' : besiegt.join(',')} | ${offen.join(',')}`);
  }
  return raus;
}

/** Ein Stück als Zeilenfeld: Katalogindex:Stufe:Güte:Aufwertung:Platz. */
function stueckFeld(besitz: OwnedItemLike): string {
  const index = ITEMS.findIndex((item) => item.id === besitz.itemId);
  return [
    index,
    besitz.itemLevel ?? '-',
    besitz.quality ?? '-',
    besitz.upgradeSteps,
    besitz.equippedSlot ?? '-',
  ].join(':');
}

function werteFelder(werte: WornValues): number[] {
  return [
    werte.catalysis,
    werte.purity,
    werte.yieldBonus,
    werte.essence,
    werte.vessel,
    werte.ward,
    werte.gearEffects,
  ];
}

function getragen(): string[] {
  const raus: string[] = [];
  const frei = ITEMS.filter((item) => item.setId === NO_SET);
  let x = 0x9e3779b9;
  const zufall = (grenze: number): number => {
    x = (Math.imul(x, 1664525) + 1013904223) >>> 0;
    return x % grenze;
  };
  for (let fall = 0; fall < 80; fall++) {
    const besitz: OwnedItemLike[] = [];
    const belegt = new Set<number>();
    const anzahl = fall === 0 ? 0 : 1 + zufall(14);
    for (let k = 0; k < anzahl; k++) {
      const def = frei[zufall(frei.length)];
      if (def === undefined) continue;
      // Auf seinen Platz, solange der frei ist — sonst in den Beutel.
      const platz = belegt.has(def.slot) || zufall(4) === 0 ? null : def.slot;
      if (platz !== null) belegt.add(def.slot);
      besitz.push({
        itemId: def.id,
        upgradeSteps: zufall(3) === 0 ? zufall(60) : 0,
        equippedSlot: platz,
        itemLevel: 1 + zufall(99),
        quality: zufall(QUALITY_COUNT),
      });
    }
    const werte = wornFrom(besitz);
    const vergleiche = besitz.map((stueck, i) => {
      const v = compareBagItem(besitz, i);
      return stueck.equippedSlot !== null || v === undefined
        ? '-'
        : `${v.verdict}:${bitsOf(v.damagePercent)}`;
    });
    raus.push(
      `${besitz.length === 0 ? '-' : besitz.map(stueckFeld).join(',')} | ` +
        `${werteFelder(werte).join(' ')} ${bitsOf(estimateDamagePerHit(werte))} | ` +
        `${vergleiche.length === 0 ? '-' : vergleiche.join(',')}`,
    );
  }
  return raus;
}

function erwartet(): string[] {
  const raus: string[] = [];
  for (let vorstufe = -1; vorstufe <= 15; vorstufe++) {
    for (const plaetze of [10, 3, 0]) {
      const stand = erwarteterStand(vorstufe, plaetze);
      raus.push(
        `${vorstufe} ${plaetze} | ${stand.length === 0 ? '-' : stand.map(stueckFeld).join(',')} | ` +
          `${werteFelder(wornFrom(stand)).join(' ')}`,
      );
    }
  }
  return raus;
}

function effektiveStufen(): string[] {
  const raus: string[] = [];
  for (const stufe of [undefined, -3, 0, 1, 40, 99, 100]) {
    for (const schritte of [undefined, -1, 0, 1, 7, 60, 200]) {
      for (const vorlage of [0, 1, 40]) {
        const besitz = {
          ...(stufe === undefined ? {} : { itemLevel: stufe }),
          ...(schritte === undefined ? {} : { upgradeSteps: schritte }),
        };
        raus.push(
          zeile([stufe ?? null, schritte ?? null, vorlage], [effectiveLevelOf(besitz, vorlage)]),
        );
      }
    }
  }
  return raus;
}

export function laufbahnRegelnPayload(): Record<string, unknown> {
  const xp = erfahrung();
  return {
    schema: SCHEMA,
    konstanten: {
      dropChancePerMille: DROP_CHANCE_PER_MILLE,
      weightTotal: WEIGHT_TOTAL,
      lootStreamIndex: LOOT_STREAM_INDEX,
      // Die Ableitung des Beute-Stroms für einige Startwerte — dieselbe
      // Funktion, die `lootStreamFor` ruft.
      lootStreamSeeds: startwerte(12).map((s) => `${s} ${seedForStream(s, LOOT_STREAM_INDEX)}`),
      materialPerQuality: MATERIAL_PER_QUALITY,
      materialPerLevel: MATERIAL_PER_LEVEL,
      materialFor: [-1, 0, 1, 2, 3, 4].map((q) => `${q} ${materialFor(q)}`),
      pointsPerSecondBits: bitsOf(REWARD.pointsPerSecond),
      pointsExponentBits: bitsOf(REWARD.pointsExponent),
      heldPointsPerPercentBits: bitsOf(REWARD.heldPointsPerPercent),
      repeatPerMille: REWARD.repeatPerMille,
      xpPerBossLevel: LEVELS.xpPerBossLevel,
      maxLevel: LEVELS.maxLevel,
    },
    katalog: ITEMS.map(
      (item) =>
        `${item.id} ${item.slot} ${item.itemLevel} ${item.quality} ${item.boundToBoss === '' ? '-' : item.boundToBoss} ${item.setId} ${item.affixKinds.join(',')}`,
    ),
    kandidaten: WURF_KONTEXTE.map(
      ([, , , boss]) =>
        `${boss} | ${candidatesFor(boss)
          .map((i) => i.id)
          .join(',')}`,
    ),
    instanzen: INSTANCES.map((i) => `${i.id} ${i.number} ${i.order} ${i.requires ?? '-'}`),
    bossStufen: spielbosse().map((id) => {
      const eintrag = BOSSES.get(id);
      return `${id} ${eintrag?.value.level ?? '-'} ${eintrag?.value.instance ?? '-'}`;
    }),
    beute: beuteTabelle(),
    wuerfe: wuerfe(),
    aufwertung: aufwertung(),
    punkte: punkte(),
    erfahrungSieg: xp.sieg,
    erfahrungAufstieg: xp.aufstieg,
    freischaltung: freischaltung(),
    effektiveStufe: effektiveStufen(),
    getragen: getragen(),
    erwartet: erwartet(),
  };
}
