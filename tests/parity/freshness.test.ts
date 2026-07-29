/**
 * Sind die gemeinsamen Vergleichsdateien noch aktuell? (Issue #54)
 *
 * **Das ist der Test, der den ganzen Abgleich zusammenhält.** Der Rust-Simulator
 * prüft gegen die Dateien unter `recordings/`. Ändert jemand eine Regel im Spiel
 * und schreibt sie nicht neu, prüft der Simulator weiter gegen die alten Werte —
 * und bleibt grün, obwohl beide Seiten auseinanderlaufen. Der Prüffall sagte
 * dann genau in dem Moment nichts mehr aus, in dem er gebraucht wird.
 *
 * Schlägt dieser Test fehl, ist das **keine kaputte Prüfung**, sondern der
 * Hinweis, dass die Dateien neu geschrieben gehören:
 *
 * ```text
 * npx vite-node tests/parity/export-parity.mjs
 * ```
 *
 * Danach zeigt der Diff, was sich geändert hat — und ob das gewollt war.
 */
import { describe, expect, it } from 'vitest';

// Als Modul eingebunden statt über das Dateisystem gelesen: Das Projekt führt
// bewusst keine Node-Typen (`tsconfig.json`), und für den Vergleich ist der
// Inhalt maßgeblich, nicht die Formatierung der Datei.
import eingecheckteAufzeichnung from './recordings/cooldown-precision.json';
import eingecheckteBausteine from './recordings/primitives.json';
import eingecheckteFehlgriffe from './recordings/misplays.json';
import eingecheckteEreignisse from './recordings/rule-events.json';
import eingecheckteVerstaerkungen from './recordings/buffs.json';
import eingecheckteProtokollArten from './recordings/log-kinds.json';
import eingecheckterBossZustand from './recordings/vessel-phases.json';
import {
  bossStatePayload,
  misplaysPayload,
  primitivesPayload,
  recordingPayload,
  ruleEventsPayload,
  buffsPayload,
  logKindsPayload,
} from './payloads.js';

const HINWEIS =
  'Die Datei ist nicht mehr aktuell. Neu schreiben mit:\n' +
  '  npx vite-node tests/parity/export-parity.mjs\n' +
  'Der Diff zeigt dann, welche Regel sich geändert hat.';

describe('Gemeinsame Vergleichsdateien', () => {
  it('die aufgezeichnete Runde entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteAufzeichnung, HINWEIS).toEqual(recordingPayload());
  });

  it('die Bausteine entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteBausteine, HINWEIS).toEqual(primitivesPayload());
  });

  it('die Runde mit Fehlgriffen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteFehlgriffe, HINWEIS).toEqual(misplaysPayload());
  });

  it('die Ereignisfolge entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteEreignisse, HINWEIS).toEqual(ruleEventsPayload());
  });

  it('die Verstärkungen entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteVerstaerkungen, HINWEIS).toEqual(buffsPayload());
  });

  it('die Protokoll-Arten entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteProtokollArten, HINWEIS).toEqual(logKindsPayload());
  it('der Kampf gegen den Boss mit Phasen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterBossZustand, HINWEIS).toEqual(bossStatePayload());
  });
});

describe('Die Ereignisfolge taugt als Prüffall (E3-1b)', () => {
  const payload = ruleEventsPayload();
  const steps = payload['steps'] as { event: number; value: number }[];
  const after = payload['checksumAfterEach'] as number[];

  it('berührt jedes der sechs Ereignisse', () => {
    // Eine Folge, die ein Ereignis auslässt, ließe eine Abweichung genau dort
    // durch.
    expect(new Set(steps.map((s) => s.event)).size).toBe(6);
  });

  it('führt für jeden Schritt eine Prüfsumme mit', () => {
    // Nur die Endsumme zu vergleichen sagt DASS, nicht WO es auseinanderläuft.
    expect(after.length).toBe(steps.length);
    expect(new Set(after).size).toBe(after.length);
  });

  it('enthält einen Unterschied im letzten Bit', () => {
    // Wer über den gerundeten Wert prüfsummiert statt über die Bits, besteht
    // alles andere und fällt hier durch.
    const werte = steps.map((s) => s.value);
    expect(werte).toContain(1);
    expect(werte.some((v) => v !== 1 && Math.abs(v - 1) < 1e-15)).toBe(true);
  });
});

describe('Die Aufzeichnung taugt als Prüffall', () => {
  const payload = recordingPayload();
  const expected = payload['expected'] as Record<string, number>;
  const presses = payload['presses'] as { tick: number; slot: number }[];
  const checksums = payload['checksums'] as number[];

  it('enthält überhaupt Tastendrücke', () => {
    // Eine leere Aufzeichnung würde jeden Simulator bestehen lassen.
    expect(presses.length).toBeGreaterThan(0);
    expect(expected['accepted']).toBe(presses.length);
    expect(expected['rejected']).toBe(0);
  });

  it('ist nach Takt geordnet', () => {
    // Beide Seiten arbeiten die Drücke in einem Durchgang ab; eine unsortierte
    // Liste würde im Simulator stillschweigend Drücke verschlucken.
    for (let i = 1; i < presses.length; i++) {
      const aktuell = presses[i]?.tick ?? 0;
      const davor = presses[i - 1]?.tick ?? 0;
      expect(aktuell).toBeGreaterThanOrEqual(davor);
    }
  });

  it('trifft die Werte der Referenz aus STEP1-SPEC § 4', () => {
    // Die Aufzeichnung ist die gute Rotation, einmal mitgeschrieben — sie muss
    // dieselben Zahlen ergeben wie `tools/step1-check.py`. Weicht sie ab, hat
    // das Aufzeichnen selbst etwas verändert.
    expect(expected['damage']).toBe(4240);
    expect(expected['killedAtMs']).toBe(40_510);
    expect(expected['randomDraws']).toBe(0);
  });

  it('hat für jeden gerechneten Takt eine Prüfsumme', () => {
    // 40510 ms Kampf bei 10 ms Takt: Takt 0 bis 4051, also 4052 Werte.
    const killedAtMs = expected['killedAtMs'] ?? -1;
    expect(checksums.length).toBe(killedAtMs / 10 + 1);
  });
});

describe('Der Kampf gegen den Boss mit Phasen taugt als Prüffall (#96)', () => {
  const payload = bossStatePayload();
  const expected = payload['expected'] as Record<string, number>;
  const boss = payload['boss'] as Record<string, unknown>;
  const phasen = payload['phases'] as number[];
  const energie = payload['energy'] as number[];
  const zeitleiste = payload['timelineIndex'] as number[];
  const hpAnteil = payload['hpPercent'] as number[];
  const checksums = payload['checksums'] as number[];

  it('berührt beide Phasen, beide Uhren und die ganze Zeitleiste', () => {
    // **Ohne diese Prüfung wäre die Datei wertlos.** Ein Kampf, in dem der Boss
    // nie unter 70 % fällt, die Energie nie auslöst und kein Zeitleisten-Eintrag
    // greift, sähe für jede Umsetzung gleich aus — auch für eine, die von all
    // dem nichts weiß.
    expect(expected['finalPhase']).toBe(2);
    expect(expected['energyTriggers']).toBeGreaterThan(0);
    expect(expected['finalTimelineIndex']).toBe(
      (boss['timeline'] as unknown[]).length,
    );
  });

  it('AC2: die Phase wechselt genau an der Schwelle, auf einen Takt genau', () => {
    const schwellen = boss['phases'] as { atHpPercent: number }[];
    let vorige = 0;
    let wechsel = 0;
    for (let tick = 0; tick < phasen.length; tick++) {
      const jetzt = phasen[tick] as number;
      if (jetzt === vorige) continue;
      wechsel++;
      // Die Phase darf nur um eins steigen — sonst wurde eine übersprungen.
      expect(jetzt).toBe(vorige + 1);
      const schwelle = schwellen[jetzt - 1]?.atHpPercent ?? 0;
      // Der Takt, in dem gewechselt wird, liegt AUF oder UNTER der Schwelle …
      expect(hpAnteil[tick] as number).toBeLessThanOrEqual(schwelle);
      // … und der Takt davor lag noch DARÜBER. Zusammen ist das „auf einen Takt
      // genau": Eine Umsetzung, die einen Takt zu früh oder zu spät wechselt,
      // fällt durch eine der beiden Bedingungen.
      expect(hpAnteil[tick - 1] as number).toBeGreaterThan(schwelle);
      vorige = jetzt;
    }
    expect(wechsel).toBe(schwellen.length);
  });

  it('AC3: die Energieleiste steigt allein nach der Uhr', () => {
    // Der Kern des Bausteins: Sie hängt NICHT am Schaden. Nachgewiesen über den
    // Abstand zwischen zwei Auslösungen — er muss konstant sein, obwohl der
    // Schaden über den Kampf schwankt.
    const energy = boss['energy'] as { perStepMs: number; amount: number; threshold: number };
    const proAusloesung = Math.ceil(energy.threshold / energy.amount);
    const ausloesungen: number[] = [];
    for (let tick = 1; tick < energie.length; tick++) {
      // Ein Rückgang heißt: Die Leiste hat ausgelöst und ist zurückgefallen.
      if ((energie[tick] as number) < (energie[tick - 1] as number)) ausloesungen.push(tick);
    }
    expect(ausloesungen.length).toBe(expected['energyTriggers']);
    expect(ausloesungen.length).toBeGreaterThan(1);
    for (let i = 1; i < ausloesungen.length; i++) {
      const abstand = (ausloesungen[i] as number) - (ausloesungen[i - 1] as number);
      // In Takten: proAusloesung Anstiege à perStepMs.
      expect(abstand).toBe((proAusloesung * energy.perStepMs) / 10);
    }
  });

  it('AC3: die Zeitleiste rückt zu den Zeitpunkten der Daten vor', () => {
    const eintraege = boss['timeline'] as { atMs: number }[];
    let vorige = 0;
    let getroffen = 0;
    for (let tick = 0; tick < zeitleiste.length; tick++) {
      const jetzt = zeitleiste[tick] as number;
      if (jetzt === vorige) continue;
      // Der Takt, in dem vorgerückt wird, muss GENAU der Zeitpunkt aus den
      // Daten sein — nicht einer davor, nicht einer danach.
      expect(tick * 10).toBe(eintraege[jetzt - 1]?.atMs);
      vorige = jetzt;
      getroffen++;
    }
    expect(getroffen).toBe(eintraege.length);
  });

  it('hat für jeden gerechneten Takt eine Prüfsumme und eine Spur', () => {
    const killedAtMs = expected['killedAtMs'] ?? -1;
    expect(checksums.length).toBe(killedAtMs / 10 + 1);
    expect(phasen.length).toBe(checksums.length);
    expect(energie.length).toBe(checksums.length);
    expect(zeitleiste.length).toBe(checksums.length);
  });

});

describe('Die Runde mit Fehlgriffen taugt als Prüffall (#95)', () => {
  const payload = misplaysPayload();
  const expected = payload['expected'] as Record<string, unknown>;
  const reasons = expected['rejectedByReason'] as Record<string, number>;

  it('enthält jeden der vier Fehlertypen mindestens einmal', () => {
    // Ohne diese Aufschlüsselung sähe eine Runde mit hundert Drücken derselben
    // falschen Sorte genauso aus wie eine mit allen vier.
    expect(reasons['onGcd'] ?? 0).toBeGreaterThan(0); // während der Wartezeit
    expect(reasons['onCooldown'] ?? 0).toBeGreaterThan(0); // auf Abklingzeit
    expect(expected['lostDotTicks'] as number).toBeGreaterThan(0); // zu früh erneuert
    expect(expected['idleTicks'] as number).toBeGreaterThan(0); // Leerlauf
    expect(reasons['ok'] ?? 0).toBeGreaterThan(0); // und etwas geht durch
  });

  it('besteht überwiegend aus Fehlgriffen', () => {
    // Sonst wäre es eine normale Runde mit ein paar Ausrutschern und würde den
    // Ablehnungspfad kaum berühren.
    expect(expected['rejected'] as number).toBeGreaterThan(expected['accepted'] as number);
  });

  it('ergibt ein anderes Ergebnis als die fehlerfreie Runde', () => {
    // Sonst prüfte diese Datei dasselbe wie cooldown-precision.
    expect(expected['damage']).not.toBe(4240);
    expect(expected['killedAtMs']).toBe(-1);
  });

  it('hat für jeden gerechneten Takt eine Prüfsumme', () => {
    const checksums = payload['checksums'] as number[];
    expect(checksums.length).toBe(payload['ticks']);
  });
});
