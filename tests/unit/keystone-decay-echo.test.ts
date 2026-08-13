import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';

/**
 * PUTREFACTIONs Schlüsseltalent (Issue #419, Bauart aus #418).
 *
 * # Was es ändert
 *
 * Ohne das Talent hört die Zersetzung mit dem Brand auf: Der letzte Stapel
 * liegt noch seine Frist ab, dann ist der Boss sauber. Mit ihm legt der
 * **erloschene** Brand weiter Stapel — bis die Lage vergangen ist, die bei
 * seinem Ende lag.
 *
 * # Der Vorgänger war wirkungslos, und dieser Fall hält fest, warum
 *
 * Mein erster Entwurf ließ beim Verfall eines Stapels jeden **laufenden**
 * Brand einmal ticken. Gemessen überlappen die beiden Zeiten aber gar nicht —
 * wenn der erste Stapel verfällt, ist der Brand längst aus. Die erste
 * Zusicherung unten misst genau diese Lücke („es GIBT die Lücke"). Wäre sie
 * eines Tages nicht mehr wahr, hinge das Talent in der Luft.
 *
 * # Die Begrenzung ist der Kern
 *
 * Was der Nachlauf legt, verlängert ihn **nicht**. Sonst trüge jeder neue
 * Stapel den nächsten, und ein einziger Tastendruck hielte die Zersetzung
 * endlos — deshalb steht unten ein eigener Fall auf das Ende.
 */

const MIT_TALENT = { 'putrefaction-keystone': 1 };
const ROT = 'rot';

/** Der ganze Verlauf der Stapelzahl, alle 100 ms — für die Grenze unten. */
function verlauf(bisMs: number, talents?: Record<string, number>): number[] {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  combat.cast(alsSpellIndex(combat.indexOf(ROT)), 0);
  const werte: number[] = [];
  for (let t = 0; t <= bisMs; t += 100) {
    combat.stepPassive(t);
    werte.push(combat.decayStacks);
  }
  return werte;
}

/** Wirkt ROT einmal und lässt den Kampf bis `bisMs` passiv weiterlaufen. */
function nachRot(bisMs: number, talents?: Record<string, number>) {
  const combat = new Combat({
    school: CALCINATION.value,
    boss: FIRST_CRUCIBLE.value,
    ...(talents === undefined ? {} : { talents }),
  });
  const rot = combat.indexOf(ROT);
  combat.cast(alsSpellIndex(rot), 0);
  for (let t = 0; t <= bisMs; t += 100) combat.stepPassive(t);
  return {
    stapel: combat.decayStacks,
    brandAktiv: combat.isDotActive(rot, bisMs),
    ticks: combat.dotTickCount(rot),
    schaden: combat.damageDealt,
  };
}

describe('Die Zusicherung vor der Messung', () => {
  it('es GIBT die Lücke: der Brand ist aus, die Stapel liegen noch', () => {
    // **Der Träger des ganzen Talents.** Genau hier fand mein erster Entwurf
    // nichts vor — er wartete auf den Stapel-Verfall und traf einen toten
    // Brand. Der zweite Entwurf setzt an derselben Stelle an, nur andersherum.
    const lauf = nachRot(6000);
    expect(lauf.brandAktiv, 'ROT läuft bei 6000 ms noch — der Fall misst nichts').toBe(false);
    expect(lauf.stapel, 'ohne liegende Stapel gäbe es nichts fortzuführen').toBeGreaterThan(0);
  });

  it('ohne Talent ist die Zersetzung bei 16000 ms vorbei', () => {
    expect(nachRot(16000).stapel, 'ohne Talent darf nach dem Brand nichts nachwachsen').toBe(0);
  });
});

describe('AC2 — der Nachlauf', () => {
  it('mit Talent liegen dort noch Stapel', () => {
    const ohne = nachRot(16000);
    const mit = nachRot(16000, MIT_TALENT);
    expect(ohne.stapel).toBe(0);
    expect(mit.stapel, 'der erloschene Brand muss weiter gelegt haben').toBeGreaterThan(0);
  });

  it('und er legt NUR Stapel, keinen Schaden', () => {
    // **Der Unterschied zu „Lasting Rot".** Ein toter Brand, der weiter
    // zuschlägt, wäre eine verlängerte Laufzeit — dieses Talent lässt allein
    // die Zersetzung wachsen. Tick-Zahl UND Schaden müssen gleich bleiben.
    const ohne = nachRot(20000);
    const mit = nachRot(20000, MIT_TALENT);
    expect(ohne.ticks, 'ROT tickt gar nicht — der Vergleich misst nichts').toBeGreaterThan(0);
    expect(mit.ticks, 'der Nachlauf darf keinen Brand-Tick hinzufügen').toBe(ohne.ticks);
    expect(mit.schaden, 'der Nachlauf darf keinen Schaden austeilen').toBe(ohne.schaden);
  });
});

describe('AC2 — und er ENDET', () => {
  it('bei 40000 ms ist auch mit Talent nichts mehr übrig', () => {
    // **Ohne diesen Fall wäre das Talent ein Selbstläufer.** Legte der
    // Nachlauf einen Stapel, dessen Frist ihn selbst am Leben hielte, liefe
    // die Zersetzung nach einem einzigen Tastendruck für immer weiter.
    //
    //   bei 16000 ms  Nachlauf steht    (Fall oben)
    //   bei 40000 ms  alles vergangen   (dieser Fall)
    expect(
      nachRot(16000, MIT_TALENT).stapel,
      'der Nachlauf fehlt — der Fall misst nichts',
    ).toBeGreaterThan(0);
    expect(nachRot(40000, MIT_TALENT).stapel, 'der Nachlauf darf sich nicht selbst tragen').toBe(0);
  });

  it('und nach dem letzten Stapel kommt NICHTS mehr nach', () => {
    // **Der Endstand ist kein Verlauf.** Der Fall darüber misst bei 40000 ms
    // und sähe nicht, wenn die Zersetzung zwischendurch auf null fiele und
    // danach noch einmal ansprünge — die Stapel wären zum Messzeitpunkt
    // längst wieder vergangen. Die Grenze aus dem Auftrag ist aber genau die:
    // Ist der letzte Stapel verfallen, ist Schluss.
    const werte = verlauf(40000, MIT_TALENT);

    const hoechste = Math.max(...werte);
    expect(hoechste, 'ohne einen einzigen Stapel misst der Verlauf nichts').toBeGreaterThan(0);

    const erstesEnde = werte.findIndex(
      (wert, i) => wert === 0 && werte.slice(0, i).some((v) => v > 0),
    );
    expect(
      erstesEnde,
      'die Zersetzung endet im Lauf gar nicht — dann ist sie unbegrenzt',
    ).toBeGreaterThan(0);

    const danach = werte.slice(erstesEnde);
    expect(
      Math.max(...danach),
      `nach dem letzten Stapel (bei ${erstesEnde * 100} ms) kam noch etwas nach`,
    ).toBe(0);
  });

  it('ein VERZEHR beendet den Nachlauf sofort — nicht erst die Uhr', () => {
    // **Die zweite Grenze, und bis hierher war sie ungeprüft.** Der Nachlauf
    // hängt an den Stapeln, nicht nur an der Zeit: Wer sie anderweitig
    // verliert, verliert ihn mit. NECROSIS ist der Weg dazu
    // (`consumesDecayStacks`), und ohne diesen Fall stand die Behauptung nur
    // als Kommentar im Code.
    //
    // Ohne ihn hätte der Nachlauf aus dem Nichts weiter gelegt: Der Brand ist
    // aus, die Stapel sind verzehrt — und es käme trotzdem einer nach.
    const combat = new Combat({
      school: CALCINATION.value,
      boss: FIRST_CRUCIBLE.value,
      talents: MIT_TALENT,
    });
    combat.cast(alsSpellIndex(combat.indexOf(ROT)), 0);
    // Bis 6000 ms: Der Brand ist aus (Laufzeit 4000), der Nachlauf trägt.
    for (let t = 0; t <= 6000; t += 100) combat.stepPassive(t);
    expect(combat.decayStacks, 'ohne liegende Stapel verzehrt NECROSIS nichts').toBeGreaterThan(0);

    combat.cast(alsSpellIndex(combat.indexOf('necrosis')), 6000);
    expect(combat.decayStacks, 'NECROSIS hat gar nicht verzehrt — der Fall misst nichts').toBe(0);

    for (let t = 6100; t <= 12000; t += 100) combat.stepPassive(t);
    expect(combat.decayStacks, 'nach dem Verzehr darf kein Stapel mehr nachkommen').toBe(0);
  });
});
