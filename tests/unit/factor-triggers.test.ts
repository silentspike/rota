import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { NEUTRAL_FACTORS, type DamageFactors } from '../../src/core/damage.js';
import { NOTHING_WORN } from '../../src/gear/worn.js';
// **Der ROHBESTAND, nicht die geparste Fassung** (Issue #425).
//
// `parseSpell` wirft unbekannte Felder weg: Solange `SpellDef` kein `buff`
// kennt, kann `'buff' in spell` auf `CALCINATION.value.spells` NIE wahr
// werden — auch dann nicht, wenn es in der Datei steht. Eine erste Fassung
// dieses Wächters fragte genau dort und blieb deshalb grün, obwohl der
// Auslöser da war.
//
// Alle sieben Dateien, nicht nur die Kalzination: Ein `buff` könnte in einer
// anderen Schule auftauchen, und ein Wächter, der nur eine ansieht, meldet
// die Lücke weiter, wenn sie längst geschlossen ist.
import rawCalcination from '../../data/spells/calcination.json';
import rawCommon from '../../data/spells/common.json';
import rawCoagulation from '../../data/spells/coagulation.json';
import rawFixation from '../../data/spells/fixation.json';
import rawSublimation from '../../data/spells/sublimation.json';
import rawPutrefaction from '../../data/spells/putrefaction.json';
import rawTestCast from '../../data/spells/test-cast.json';

const ROHE_SCHULEN: readonly { spells: readonly Record<string, unknown>[] }[] = [
  rawCalcination,
  rawCommon,
  rawCoagulation,
  rawFixation,
  rawSublimation,
  rawPutrefaction,
  rawTestCast,
] as never;

/** Trägt IRGENDEIN Zauber in IRGENDEINER Datei ein `buff`-Feld? */
function esGibtEinenBuffZauber(): boolean {
  return ROHE_SCHULEN.some((datei) => datei.spells.some((spell) => 'buff' in spell));
}

/**
 * Je Schadensfaktor ein Aufbau, der ihn auslöst — und die Meldung, welcher in
 * **keinem** davon abweicht (Issue #425).
 *
 * # Der Unterschied zu #424
 *
 * `factor-write-sites.test.ts` fragt: Gibt es überhaupt eine Zeile, die diesen
 * Topf beschreibt? Das ist ein Quelltext-Scan und trennt scharf, aber flach.
 *
 * Hier steht die schwerere Frage: Wird der Topf **im Kampf** wirklich gefüllt?
 * Eine Schreibstelle, die nie ausgeführt wird, ist so wirkungslos wie keine —
 * und der Unterschied fällt in keinem Test auf, weil ein Faktor auf 1 sich wie
 * ein korrekt gefüllter verhält.
 *
 * # Warum ein Aufbau je Topf und nicht ein Kampf für alle
 *
 * **Daran ist der erste Entwurf von #424 gescheitert.** Ein Kampf mit
 * FIRST_CRUCIBLE und kindle/smoulder/pyre lässt fünf von acht Töpfen neutral,
 * ohne dass irgendetwas kaputt wäre: Der Boss hat keine Widerstände, die drei
 * Zauber öffnen kein Synergie-Fenster, und die Zersetzung hängt an CANKER.
 *
 * „Nicht angeschlossen" und „in diesem Lauf nicht ausgelöst" sahen dort gleich
 * aus. Jeder Aufbau hier weiß deshalb, **womit** er seinen Topf treffen will —
 * und meldet, wenn er ihn verfehlt.
 */

const BOSS = FIRST_CRUCIBLE.value;

function neu(bossId?: string): Combat {
  const boss = bossId === undefined ? BOSS : bossById(bossId);
  if (boss === undefined) throw new Error(`Boss ${bossId} fehlt`);
  return new Combat({ school: CALCINATION.value, boss });
}

/** Ein Wirken samt Zeitlauf, damit auch verzögerte Wirkungen greifen. */
function wirke(combat: Combat, id: string, timeMs: number): void {
  combat.cast(combat.indexOf(id), timeMs);
  for (let t = timeMs; t <= timeMs + 3000; t += 10) combat.stepPassive(t);
}

/**
 * Je Topf: ein Aufbau, der ihn treffen soll, und der Wert danach.
 *
 * Die Aufbauten sind **absichtlich verschieden** — jeder trifft genau einen
 * Topf, und keiner verlässt sich darauf, dass ein anderer nebenbei mitläuft.
 */
const AUFBAUTEN: Readonly<Record<string, () => number>> = {
  essence: () => {
    const c = new Combat({
      school: CALCINATION.value,
      boss: BOSS,
      worn: { ...NOTHING_WORN, essence: 300 },
    });
    return c.damageFactors.essence;
  },
  gearEffects: () => {
    const c = new Combat({
      school: CALCINATION.value,
      boss: BOSS,
      worn: { ...NOTHING_WORN, gearEffects: 250 },
    });
    return c.damageFactors.gearEffects;
  },
  talents: () => {
    const c = neu();
    c.setTalents({ 'calcination-build': 3 });
    return c.damageFactors.talents;
  },
  synergy: () => {
    // CANKER ist der einzige Zauber mit `windowGcds` — gemessen, nicht geraten.
    //
    // **ZWEI Zauber, nicht einer** — und das ist der Grund, warum dieser
    // Aufbau zweimal gescheitert ist, bevor er stand:
    //
    //   CANKER  öffnet das Fenster        (`synergy.partner: "fester"`)
    //   FESTER  bekommt den Faktor        — erst SEIN Wirken setzt ihn
    //
    // Wer nur CANKER wirkt, misst eine 1 und schließt daraus, das System sei
    // tot. Und wer danach 3000 ms verstreichen lässt, misst ebenfalls eine 1,
    // weil das Fenster zwei Wartezeiten kurz ist.
    //
    // Genau diese zwei Fallen sind der Grund für #425: „nicht angeschlossen"
    // und „falsch getroffen" sehen von außen gleich aus.
    const c = neu();
    c.cast(c.indexOf('canker'), 0);
    c.cast(c.indexOf('fester'), 1500);
    return c.damageFactors.synergy;
  },
  vulnerability: () => {
    // Dieselbe Quelle, andere Wirkung: CANKER legt Zersetzungs-Stapel. Die
    // brauchen den Zeitlauf, das Synergie-Fenster verträgt ihn nicht — zwei
    // Töpfe, zwei Messzeitpunkte.
    const c = neu();
    wirke(c, 'canker', 0);
    return c.damageFactors.vulnerability;
  },
  resistance: () => {
    // **Nur über einen test-Boss.** FIRST_CRUCIBLE führt `resistances: {}`;
    // ein Aufbau mit ihm bliebe neutral, ohne dass etwas fehlt.
    const c = neu('test-resistant');
    wirke(c, 'kindle', 0);
    return c.damageFactors.resistance;
  },
};

/**
 * Wofür es heute **keinen** Auslöser gibt — mit dem Issue, das ihn bringt.
 *
 * **Diese Einträge melden sich selbst ab.** Sobald der Auslöser existiert,
 * weicht der Topf ab, AC3 unten wird rot und verlangt das Entfernen. Ein
 * Kommentar an dieser Stelle täte das nicht: Er bliebe stehen, und der Topf
 * wäre ab da unbewacht.
 */
const OHNE_AUSLOESER: Readonly<Record<string, string>> = {
  buffs:
    '#420 — kein Zauber des Spiels trägt ein `buff`-Feld. Der Prüf-Zauber ' +
    '`test-buff` liegt auf fix/e3-420-buffs und ist noch nicht gemergt.',
  crit:
    'per Typ von `setDamageFactor` ausgeschlossen — er wird aus `worn` ' +
    'abgeleitet und lebt nur in `factorsOnCrit`. Kein fehlender Auslöser, ' +
    'sondern Absicht (siehe #424).',
};

const ALLE_TOEPFE = Object.keys(NEUTRAL_FACTORS) as (keyof DamageFactors)[];

describe('Je Faktoren-Topf ein Auslöser (Issue #425)', () => {
  it('AC1 — jeder Topf ist entweder gebaut oder benannt, keiner vergessen', () => {
    // **Die Zusicherung vor der Messung.** Ohne sie liefe der Fall unten über
    // eine Teilmenge und wäre grün, weil er den Rest gar nicht ansieht.
    const abgedeckt = new Set([...Object.keys(AUFBAUTEN), ...Object.keys(OHNE_AUSLOESER)]);
    const vergessen = ALLE_TOEPFE.filter((t) => !abgedeckt.has(t));

    expect(
      vergessen,
      `Für diese Töpfe gibt es weder einen Aufbau noch einen Eintrag in ` +
        `OHNE_AUSLOESER:\n${vergessen.join('\n')}`,
    ).toEqual([]);
    expect(ALLE_TOEPFE.length, 'die Liste der Töpfe ist leer').toBe(8);
  });

  it('AC2 — jeder gebaute Aufbau trifft seinen Topf wirklich', () => {
    // **Der Kern.** Ein Aufbau, der seinen Topf nicht auslöst, meldet ihn
    // fälschlich als tot — genau der Fehler, an dem der erste Entwurf von
    // #424 gescheitert ist.
    const verfehlt: string[] = [];
    for (const [topf, aufbau] of Object.entries(AUFBAUTEN)) {
      const wert = aufbau();
      const neutral = NEUTRAL_FACTORS[topf as keyof DamageFactors];
      if (wert === neutral) verfehlt.push(`${topf} bleibt auf ${neutral}`);
    }

    expect(
      verfehlt,
      `Diese Aufbauten lösen ihren Topf NICHT aus:\n${verfehlt.join('\n')}\n\n` +
        `Entweder ist das System nicht angeschlossen — dann gehört ein Issue ` +
        `dazu — oder der Aufbau trifft es nicht, dann gehört er berichtigt. ` +
        `Beides sieht von außen gleich aus, und deshalb steht hier je Topf ` +
        `ein eigener Aufbau statt eines Kampfes für alle.`,
    ).toEqual([]);
  });

  it('AC3 — und die Ausnahmen sind noch nötig, nicht längst überholt', () => {
    // **Die Selbstabmeldung.** Sobald `test-buff` auf main liegt, trägt ein
    // Zauber ein `buff`-Feld, dieser Fall wird rot und verlangt, dass der
    // Eintrag verschwindet und ein Aufbau dazukommt.
    const ueberholt = Object.keys(OHNE_AUSLOESER).filter((topf) => {
      if (topf === 'crit') return false; // per Typ ausgeschlossen, kein Auslöser möglich
      return esGibtEinenBuffZauber();
    });

    expect(
      ueberholt,
      `Für diese Töpfe gibt es inzwischen einen Auslöser, und sie stehen ` +
        `trotzdem in OHNE_AUSLOESER:\n` +
        ueberholt.map((t) => `  ${t} — ${OHNE_AUSLOESER[t]}`).join('\n') +
        `\n\nEintrag entfernen und einen Aufbau in AUFBAUTEN ergänzen.`,
    ).toEqual([]);
  });

  it('AC4 — die Gegenprobe: ohne Auslöser bleibt derselbe Topf neutral', () => {
    // **Ohne diese Zeile belegt AC2 nichts.** Er zeigte sonst nur, dass die
    // Werte verschieden von 1 sind — nicht, dass der Aufbau die Ursache ist.
    const ohneAusruestung = neu();
    expect(ohneAusruestung.damageFactors.essence).toBe(NEUTRAL_FACTORS.essence);

    const ohneWiderstand = neu();
    wirke(ohneWiderstand, 'kindle', 0);
    expect(
      ohneWiderstand.damageFactors.resistance,
      'FIRST_CRUCIBLE führt `resistances: {}` — hier MUSS es neutral bleiben, ' +
        'sonst trifft der test-resistant-Aufbau in AC2 etwas anderes als gedacht',
    ).toBe(NEUTRAL_FACTORS.resistance);
  });
});
