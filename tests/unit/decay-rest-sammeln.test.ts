import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE, TALENTS } from '../../src/core/data.js';
import { talentDecayStackPermille } from '../../src/progression/talents.js';

/**
 * Der Stapel-Aufbau sammelt den Rest — und heute speist ihn niemand (#419).
 *
 * # Was die Rechnung tut
 *
 * `floor(1 × 1,5)` ist wieder 1. Ein Faktor auf die Zahl gelegter Stapel war
 * damit wirkungslos, solange jede Quelle genau einen legt (canker 1, blight 1,
 * rot 1). Statt aufzurunden — das schenkte jedem ersten Rang einen Stapel —
 * sammelt der Kampf den Rest: Was am ganzen fehlt, wartet auf den nächsten
 * Wurf derselben Quelle.
 *
 * # Warum hier keine Wirkung mehr geprüft wird
 *
 * Der Umbau der vier Wege hat `putrefaction-build` von „legt Stapel" auf
 * „hebt die Obergrenze" gestellt. Seither trägt **kein Talent** mehr
 * `decayStackPerRankPerMille` — der Faktor ist null, und die Rechnung ist ein
 * No-Op. Über ein Talent ist sie von dieser Seite nicht mehr erreichbar.
 *
 * Ihre Wirkung prüft deshalb `sim/tests/decay_rest.rs`: Dort steht der Faktor
 * als Zahl in `FightConfig`, ohne Umweg über eine Verteilung. Die Folge
 * `1, 3, 4` bei Faktor 1,5 steht dort mit derselben Begründung.
 *
 * # Was hier steht, ist die LÜCKE
 *
 * Ein No-Op ist kein toter Zweig — er läuft bei jedem Stapel-Ereignis mit und
 * ist korrekt. Aber ohne einen Fall, der ihn benennt, wäre nicht zu sehen,
 * dass er heute nichts tut. Genau daran ist in diesem Projekt mehrfach Arbeit
 * gestorben: gebaut, grün, und von niemandem erreicht.
 */

describe('Die Zusicherung: die Quellen legen genau EINEN Stapel', () => {
  it('CANKER legt einen — daran hing die ganze Rundungsfalle', () => {
    const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value });
    combat.cast(alsSpellIndex(combat.indexOf('canker')), 0);
    expect(combat.decayStacks, 'CANKER legt nicht genau einen Stapel').toBe(1);
  });
});

describe('Heute speist KEIN Talent diesen Faktor', () => {
  it('auch bei voll verteilten Talenten bleibt er null', () => {
    // **Der Fall, der die Lücke sichtbar hält.** Er wird rot an dem Tag, an
    // dem wieder ein Talent Stapel legt — und dann weiß der Nächste sofort:
    // Die Rest-Rechnung tut wieder etwas, und `sim/tests/decay_rest.rs` prüft
    // ab jetzt etwas, das ein Spieler auch erreichen kann.
    //
    // **Alle Talente auf vollem Rang**, nicht nur die verdächtigen: Ein neues
    // Talent in einem beliebigen Baum würde sonst übersehen.
    const alleVoll: Record<string, number> = {};
    for (const talent of TALENTS) alleVoll[talent.id] = talent.maxRanks;

    expect(
      talentDecayStackPermille(alleVoll),
      'ein Talent legt wieder Stapel — dann ist der Restwert gespeist, und die ' +
        'Wirkung gehört wieder auf diese Seite geprüft (siehe sim/tests/decay_rest.rs)',
    ).toBe(0);
  });

  it('und die Rechnung bleibt deshalb ohne Wirkung', () => {
    // Die Gegenprobe zur Zeile oben, eine Ebene tiefer: Was der Faktor null
    // bedeutet, sieht man an den Stapeln — sechs Würfe, sechs Stapel bis zum
    // Deckel, kein einziger zusätzlicher.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: { 'putrefaction-build': 5 },
    });
    const canker = alsSpellIndex(combat.indexOf('canker'));
    const werte: number[] = [];
    for (let i = 0; i < 4; i++) {
      combat.cast(canker, i * 3000);
      werte.push(combat.decayStacks);
    }
    expect(werte, 'ein Wurf hat mehr als einen Stapel gelegt').toEqual([1, 2, 3, 4]);
  });
});
