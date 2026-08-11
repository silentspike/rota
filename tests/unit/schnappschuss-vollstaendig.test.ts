/**
 * Der Dev-Schnappschuss führt JEDEN öffentlichen Getter von `Combat`
 * (Issue #625).
 *
 * # Warum dieser Fall der eigentliche Auftrag ist
 *
 * Die 56 fehlenden Größen nachzureichen ist eine einmalige Arbeit. Dass sie
 * **fehlen konnten**, ist das Problem — und das wiederholt sich beim nächsten
 * Getter, wenn niemand hinsieht. Gemessen an einem einzigen Tag:
 *
 * ```text
 *   eine Sitzung suchte  bossEnergy, bossStacks     nicht durchgereicht
 *   eine andere          talentAllocation, level    nicht durchgereicht
 * ```
 *
 * Beide Größen lagen in `Combat` bereit. Beide Male hatte jemand früher eine
 * sinnvolle Auswahl getroffen, die später nicht mehr passte — und beide Male
 * fiel es erst auf, als eine Browser-Abnahme daran scheiterte.
 *
 * **Dieser Fall macht die Entscheidung dauerhaft.** Kommt morgen ein Getter
 * dazu und wird nicht durchgereicht, wird er rot und nennt den Namen.
 *
 * Das ist keine Theorie: Während dieser Arbeit kam `activeSetEffects` durch
 * einen fremden Merge dazu. Meine Zählung von einer Stunde vorher sagte 55
 * fehlende Getter, die Datei enthielt 56.
 *
 * # Warum die Liste aus dem QUELLTEXT kommt und nicht aus einer Konstanten
 *
 * Eine gepflegte Liste altert genauso wie die Auswahl, gegen die dieser Fall
 * schützt — sie wäre dieselbe Falle eine Ebene höher. Der Fall liest deshalb
 * `src/core/combat.ts` und zählt, was dort als `get name()` steht.
 *
 * **Die Zusicherung dazu steht als eigener Fall**: Findet das Muster gar
 * nichts — umbenannte Datei, geänderte Formatierung —, wäre alles darunter
 * trivial erfüllt.
 */
import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, bossById } from '../../src/core/data.js';
import { TICK_MS } from '../../src/core/loop.js';

/** Jeder `get name()` in `Combat` — aus dem Quelltext, nicht aus einer Liste. */
function getterImQuelltext(): string[] {
  const quelle = readFileSync(new URL('../../src/core/combat.ts', import.meta.url), 'utf8');
  const namen = [...quelle.matchAll(/^\s+get ([a-zA-Z]\w*)\(\)/gm)].map((t) => t[1] as string);
  return [...new Set(namen)].sort();
}

/**
 * Ein Kampf, in dem etwas passiert ist.
 *
 * **Nicht der Anfangszustand**: Dort stehen fast alle Größen auf 0, und ein
 * Schnappschuss, der nur Nullen führt, sähe vollständig aus, ohne es zu sein.
 */
function nachEinigenSekunden(): Combat {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: bossById('test-doomsayer') ?? FIRST_CRUCIBLE.value,
  });
  const kindle = alsSpellIndex(combat.indexOf('kindle'));
  for (let timeMs = 0; timeMs < 6000; timeMs += TICK_MS) {
    combat.stepPassive(timeMs);
    if (combat.isReady(kindle, timeMs)) combat.cast(kindle, timeMs);
  }
  return combat;
}

const GETTER = getterImQuelltext();

describe('die Messung selbst trägt', () => {
  it('der Quelltext liefert überhaupt Getter', () => {
    // **Ohne diesen Fall wäre alles darunter trivial erfüllt.** Wird
    // `combat.ts` umbenannt oder die Formatierung geändert, findet das Muster
    // nichts — und eine leere Liste ist in jeder Schnittmenge enthalten.
    expect(GETTER.length, 'im Quelltext wurden keine Getter gefunden').toBeGreaterThan(60);
  });

  it('und darunter die, an denen zwei Sitzungen gescheitert sind', () => {
    // Namentlich, nicht als Zahl: Diese sechs sind der Anlass des Auftrags.
    for (const name of [
      'bossEnergy',
      'bossStacks',
      'talentAllocation',
      'level',
      'playerHp',
      'shield',
    ]) {
      expect(GETTER, `${name} fehlt im Quelltext — wurde er umbenannt?`).toContain(name);
    }
  });
});

describe('jeder Getter ist im Schnappschuss erreichbar', () => {
  /**
   * `combat()` aus `devtools` ist an einen `App` gebunden, den ein Prüflauf
   * ohne DOM nicht bauen kann. Geprüft wird deshalb der Vertrag, den die
   * Oberfläche erfüllt: **die Feldnamen des Typs**, abgelesen an der Datei.
   *
   * Das ist enger als ein Aufruf und trotzdem scharf: Wer einen Getter
   * hinzufügt und die Durchreichung vergisst, fehlt hier.
   */
  function felderImSchnappschuss(): Set<string> {
    const quelle = readFileSync(new URL('../../src/devtools/index.ts', import.meta.url), 'utf8');
    const namen = new Set<string>();
    // **Beide Typen, und beide von ihrem EIGENEN Anfang bis zur nächsten
    // schließenden Klammer.** Ein erster Entwurf schnitt von `CombatSnapshot`
    // bis `StateSnapshot` — und `StateSnapshot` steht WEITER OBEN (Zeile 103
    // gegen 177). Der Ausschnitt war leer, und der Fall meldete alle 59
    // Getter als fehlend, obwohl sie alle dastanden.
    //
    // Ein Messweg, der ins Leere greift, meldet nicht „ich habe nichts
    // gefunden", sondern „nichts ist da". Der Unterschied kostet eine
    // Fehlersuche am falschen Ende — deshalb steht unten eine Zusicherung
    // auf die Größe dieser Menge.
    for (const typ of ['CombatSnapshot', 'StateSnapshot']) {
      const ab = quelle.indexOf(`export interface ${typ} {`);
      if (ab < 0) continue;
      const bis = quelle.indexOf('\n}', ab);
      const block = quelle.slice(ab, bis);
      for (const treffer of block.matchAll(/^\s+readonly ([a-zA-Z]\w*)[?]?:/gm)) {
        namen.add(treffer[1] as string);
      }
    }
    return namen;
  }

  /**
   * Getter, die der Schnappschuss unter einem ANDEREN Namen führt.
   *
   * Sie sind erreichbar, nur anders benannt — meist weil der Snapshot-Name
   * älter ist als der Getter. Jede Zeile hier ist eine Ausnahme mit Grund;
   * eine ohne Grund gehört nicht hinein.
   */
  const ANDERS_BENANNT: Record<string, string> = {
    damageDealt: 'damage', // seit dem ersten Schnappschuss so
    bossHpRemaining: 'bossHpRemaining',
    fightState: 'state', // als lesbarer Name, nicht als Zahl
    endedAt: 'endedAtMs',
    fightStartedAt: 'fightStartedAtMs', // steht in state(), nicht in combat()
  };

  /** Getter, die `state()` führt statt `combat()` — beide zählen. */
  const IN_STATE = new Set([
    'playerCasting',
    'playerCastEndsAtMs',
    'castsInterrupted',
    'castsCancelled',
    'castingName',
    'casting',
    'castLandsAtMs',
    'interruptsLanded',
    'fightStartedAt',
  ]);

  it('die Feld-Liste selbst ist gefüllt — sonst fehlt „alles"', () => {
    // **Die Zusicherung, die beim ersten Anlauf gefehlt hat.** Greift das
    // Auslesen ins Leere, meldet der Fall darunter jeden Getter als fehlend
    // und schickt den Leser in den Spielcode statt in diese Funktion.
    expect(
      felderImSchnappschuss().size,
      'aus den Typen wurden keine Felder gelesen',
    ).toBeGreaterThan(70);
  });

  it('kein Getter fehlt — und wenn doch, steht sein Name hier', () => {
    const felder = felderImSchnappschuss();
    const fehlend = GETTER.filter(
      (g) => !felder.has(g) && !(g in ANDERS_BENANNT) && !IN_STATE.has(g),
    );
    // **Die Meldung nennt die Namen, nicht nur die Zahl.** Wer morgen einen
    // Getter hinzufügt, soll lesen können, welchen — nicht „71 statt 72".
    expect(fehlend, `diese Getter sind nicht durchgereicht: ${fehlend.join(', ')}`).toEqual([]);
  });

  it('die Ausnahmen sind echte Ausnahmen, keine Sammelstelle', () => {
    // Wächst diese Liste, hört der Fall auf zu schützen — dann kann man jeden
    // vergessenen Getter dort eintragen und es wird wieder grün.
    expect(Object.keys(ANDERS_BENANNT).length + IN_STATE.size).toBeLessThanOrEqual(16);
  });
});

describe('die Größen tragen in einem laufenden Kampf einen Wert', () => {
  const combat = nachEinigenSekunden();

  it('keiner der Getter liefert undefined', () => {
    // Ein Getter, der nur unter Bedingungen lesbar wäre, hinterließe im
    // Schnappschuss eine Lücke, die wie ein Wert aussieht.
    const roh = combat as unknown as Record<string, unknown>;
    const leer = GETTER.filter((g) => roh[g] === undefined);
    expect(leer, `diese Getter liefern undefined: ${leer.join(', ')}`).toEqual([]);
  });

  it('und der Kampf hat wirklich stattgefunden', () => {
    // Ohne diese Zeile misst der Fall darüber einen Anfangszustand, in dem
    // fast alles 0 ist — und 0 ist nicht undefined, also wäre er grün.
    expect(combat.damageDealt, 'im Messkampf ist kein Schaden gefallen').toBeGreaterThan(0);
    expect(combat.doom, 'der Verhängnis-Boss hat kein Verhängnis aufgebaut').toBeGreaterThan(0);
  });
});
