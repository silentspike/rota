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
import eingecheckterBuffKampf from './recordings/buff-combat.json';
import eingecheckteNeueFixierung from './recordings/fixation-spells.json';
import eingecheckteProtokollArten from './recordings/log-kinds.json';
import eingecheckterAuswahlmodus from './recordings/cooldown-reset.json';
import eingecheckterBossZustand from './recordings/vessel-phases.json';
import eingecheckterZaehler from './recordings/doom-stacks.json';
import eingecheckterAuswahlLauf from './recordings/cooldown-reset-run.json';
import eingecheckterWettlauf from './recordings/race-heal.json';
import eingecheckteDreiBausteine from './recordings/trial-blocks.json';
import eingecheckteWiederholung from './recordings/timeline-loop.json';
import eingecheckteSublimation from './recordings/sublimation.json';
import eingecheckteGezielteVerkuerzung from './recordings/gezielte-verkuerzung.json';
import eingecheckterWisp from './recordings/wisp-verkuerzung.json';
import eingecheckterAlembic from './recordings/alembic-zwei-ladungen.json';
import eingecheckterDeckel from './recordings/deckel-gefallen.json';
import eingecheckterPhasen from './recordings/phasen-bindung.json';
import eingecheckteAusruestung from './recordings/gear.json';
import eingecheckteSaetze from './recordings/sets.json';
import eingecheckterSpielerSchaden from './recordings/player-damage.json';
import eingecheckteKoagulation from './recordings/coagulation.json';
import eingecheckteKontrolle from './recordings/kontrolle-wirkt.json';
import eingecheckteFixierung from './recordings/fixation.json';
import eingecheckteNeueZauber from './recordings/new-calcination.json';
import eingecheckteProtokollParitaet from './recordings/log-parity.json';
import eingecheckteUmrechnung from './recordings/haste.json';
import eingecheckteGetrageneWerte from './recordings/worn-values.json';
import eingecheckteQuellen from './recordings/sources.json';
import eingecheckteWirkzeit from './recordings/cast-time.json';
import eingecheckteWiderstaende from './recordings/resistances.json';
import eingecheckteGemischteLeiste from './recordings/resistances-mixed.json';
// **Diese zwei fehlten hier**, und das ist ein Befund und keine Ergänzung:
// `runaway-progress` (#256) und `level-30` (#289) stehen seit ihrer Entstehung
// unter `recordings/`, aber in keinem Frische-Fall. Der Simulator prüft gegen
// sie — und wäre die Regel im Spiel gewandert, hätte er weiter gegen die alten
// Werte geprüft und wäre grün geblieben. Genau der Zustand, den die
// Beschreibung oben als „sagt nichts mehr aus, wenn er gebraucht wird"
// beschreibt. Gefunden beim Eintragen der dritten Datei.
import eingecheckteReaktion from './recordings/runaway-progress.json';
import eingecheckteStufe from './recordings/level-30.json';
// **Und beim vierten Mal fehlten wieder zwei** (Issue #456). `synergy-window`
// stand seit #318 unter `recordings/` und in keinem Frische-Fall; `synergy-haste`
// kam mit diesem Issue dazu. Beide Male fiel es nur auf, weil jemand zufaellig
// eintrug — genau wie oben bei `runaway-progress` und `level-30`.
//
// Deshalb steht unten jetzt ein Waechter, der die Liste selbst vergleicht.
// Derselbe Fehler zweimal auf dieselbe Weise gefunden heisst: Der naechste wird
// nicht gefunden.
import eingecheckteSynergie from './recordings/synergy-window.json';
import eingecheckteSynergieMitTempo from './recordings/synergy-haste.json';
import eingecheckterWardDeckel from './recordings/ward-cap.json';
import eingecheckteFaeulnisFueller from './recordings/putrefaction-filler.json';
import eingecheckteGlut from './recordings/cinder-ticks.json';
import eingecheckterBoss1 from './recordings/instance1-boss-1.json';
import eingecheckterZwei1 from './recordings/instance2-boss-1.json';
import eingecheckterZwei2 from './recordings/instance2-boss-2.json';
import eingecheckterZwei3 from './recordings/instance2-boss-3.json';
import eingecheckterZwei4 from './recordings/instance2-boss-4.json';
import eingecheckterBoss2 from './recordings/instance1-boss-2.json';
import eingecheckterBoss3 from './recordings/instance1-boss-3.json';
import eingecheckterBoss4 from './recordings/instance1-boss-4.json';
import eingecheckteZielgroessen from './recordings/buff-targets.json';
import eingecheckteCapStapelung from './recordings/cap-stapelung.json';
import eingecheckteFaeulnis from './recordings/rot-decay.json';
import eingecheckteBrandzeit from './recordings/dot-lifetime.json';
import eingecheckteVerlaengerung from './recordings/extend-effects.json';
import eingecheckteKerne from './recordings/kern-zauber.json';
import eingecheckteRestzeit from './recordings/efflux-restzeit.json';
import eingecheckteKrit from './recordings/krit-umwandlung.json';
import {
  bossStatePayload,
  doomStacksPayload,
  newCalcinationPayload,
  resetRunPayload,
  raceHealPayload,
  trialBlocksPayload,
  timelineLoopPayload,
  sublimationPayload,
  gezielteVerkuerzungPayload,
  wispPayload,
  alembicPayload,
  deckelPayload,
  phasenPayload,
  misplaysPayload,
  playerDamagePayload,
  coagulationPayload,
  kontrolleWirktPayload,
  fixationPayload,
  logParityPayload,
  primitivesPayload,
  recordingPayload,
  ruleEventsPayload,
  buffsPayload,
  logKindsPayload,
  cooldownResetPayload,
  gearPayload,
  setsPayload,
  hastePayload,
  wornPayload,
  sourcesPayload,
  castTimePayload,
  resistancePayload,
  resistanceMixedPayload,
  runawayProgressPayload,
  level30Payload,
  synergyPayload,
  synergyHastePayload,
  buffCombatPayload,
  wardCapPayload,
  putrefactionFillerPayload,
  fixationSpellsPayload,
  buffTargetsPayload,
  capStapelungPayload,
  instance1Payload,
  instance2Payload,
  cinderPayload,
  rotPayload,
  dotLifetimePayload,
  extendEffectsPayload,
  kernePayload,
  effluxPayload,
  kritPayload,
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

  it('die neuen Fixierungs-Zauber entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteNeueFixierung, HINWEIS).toEqual(fixationSpellsPayload());
  });

  it('der Kampf mit aktiver Verstärkung entspricht dem heutigen Spielstand', () => {
    // **Der Name in MIT_FRISCHE_FALL allein reicht nicht** (Issue #463): Die
    // Liste sagt nur, dass ein Fall existieren SOLL. Ohne diesen Vergleich
    // stünde `buff-combat` dort und die Datei bliebe trotzdem unbewacht —
    // der Simulator prüfte gegen einen Stand, den das Spiel längst verlassen
    // hat, und beide wären grün.
    expect(eingecheckterBuffKampf, HINWEIS).toEqual(buffCombatPayload());
  });

  it('die Synergie-Runde entspricht dem heutigen Spielstand', () => {
    // Stand seit #318 unter `recordings/` und in keinem Frische-Fall.
    expect(eingecheckteSynergie, HINWEIS).toEqual(synergyPayload());
  });

  it('die Synergie-Runde MIT TEMPO entspricht dem heutigen Spielstand', () => {
    // Die Aufzeichnung aus #456: Sie ist der einzige Ort, an dem beide Seiten
    // die Klammerung des Synergie-Fensters unter Tempo vergleichen. Veraltet
    // sie unbemerkt, ist genau der Nachweis wieder weg, für den sie entstand.
    expect(eingecheckteSynergieMitTempo, HINWEIS).toEqual(synergyHastePayload());
  });

  it('die Glut-Runde entspricht dem heutigen Spielstand', () => {
    // CINDER wird nur hier gewirkt — veraltet die Datei, ist der einzige
    // Paritäts-Nachweis für ihn wieder weg (Issue #477).
    expect(eingecheckteGlut, HINWEIS).toEqual(cinderPayload());
  });

  it('die vier Bosse der zweiten Instanz entsprechen dem heutigen Spielstand', () => {
    const erwartet = [
      eingecheckterZwei1,
      eingecheckterZwei2,
      eingecheckterZwei3,
      eingecheckterZwei4,
    ];
    for (let i = 0; i < erwartet.length; i++) {
      expect(erwartet[i], HINWEIS).toEqual(
        instance2Payload(`boss-2-${i + 1}`, `instance2-boss-${i + 1}`),
      );
    }
  });

  it('die vier Bosse der ersten Instanz entsprechen dem heutigen Spielstand', () => {
    const erwartet = [
      eingecheckterBoss1,
      eingecheckterBoss2,
      eingecheckterBoss3,
      eingecheckterBoss4,
    ];
    for (let i = 0; i < erwartet.length; i++) {
      expect(erwartet[i], HINWEIS).toEqual(
        instance1Payload(`boss-1-${i + 1}`, `instance1-boss-${i + 1}`),
      );
    }
  });

  it('die Verstärkungs-Runde entspricht dem heutigen Spielstand', () => {
    // Die fünf Zauber aus #540 werden nur hier gewirkt.
    expect(eingecheckteZielgroessen, HINWEIS).toEqual(buffTargetsPayload());
  });

  it('die Stapel-Runde entspricht dem heutigen Spielstand', () => {
    // SLAG und `test-cap` treffen nur hier aufeinander (Issue #562) — die
    // einzige Lage im Bestand, in der zwei verschiedene Verstaerkungs-Arten
    // EINEN Speicher teilen.
    expect(eingecheckteCapStapelung, HINWEIS).toEqual(capStapelungPayload());
  });

  it('die Brandzeit-Runde entspricht dem heutigen Spielstand', () => {
    // EMBER und PYROCLASM werden nur hier gewirkt (Issue #517).
    expect(eingecheckteBrandzeit, HINWEIS).toEqual(dotLifetimePayload());
  });

  it('die Verlängerungs-Runde entspricht dem heutigen Spielstand', () => {
    // HARDEN und SEPSIS werden nur hier gewirkt (Issue #523).
    expect(eingecheckteVerlaengerung, HINWEIS).toEqual(extendEffectsPayload());
  });

  it('der Krit-Lauf entspricht dem heutigen Spielstand', () => {
    // Die einzige KAMPF-Aufzeichnung mit einer Krit-Chance über null
    // (Issue #532). Veraltet sie, ist der Zufallsstrom wieder ungeprüft.
    expect(eingecheckteKrit, HINWEIS).toEqual(kritPayload());
  });

  it('der Restzeit-Lauf entspricht dem heutigen Spielstand', () => {
    // EFFLUX wird nur hier gewirkt (Issue #520).
    expect(eingecheckteRestzeit, HINWEIS).toEqual(effluxPayload());
  });

  it('die Kern-Zauber entsprechen dem heutigen Spielstand', () => {
    // CLASP, LATCH und PLATE werden nur hier gewirkt — veraltet die Datei,
    // ist der einzige Paritäts-Nachweis für alle drei weg (Issue #519).
    expect(eingecheckteKerne, HINWEIS).toEqual(kernePayload());
  });

  it('die Fäulnis über Zeit entspricht dem heutigen Spielstand', () => {
    // ROT ist der einzige Zauber, der Zersetzungs-Stapel OHNE Tastendruck legt
    // (Issue #498) — veraltet die Datei, ist der Nachweis dafür weg, dass das
    // Legen je Tick auf beiden Seiten gleich rechnet.
    expect(eingecheckteFaeulnis, HINWEIS).toEqual(rotPayload());
  });
  it('die Fäulnis-Füller entsprechen dem heutigen Spielstand', () => {
    // BLIGHT wird nur hier gewirkt — veraltet die Datei, ist der einzige
    // Paritäts-Nachweis für ihn wieder weg (Issue #479).
    expect(eingecheckteFaeulnisFueller, HINWEIS).toEqual(putrefactionFillerPayload());
  });

  it('der WARD-Deckel entspricht dem heutigen Spielstand', () => {
    // Er ist eine REGEL, keine Zahl in den Daten — heute trägt kein Gegenstand
    // genug WARD, um ihn zu erreichen. Veraltet diese Datei unbemerkt, ist der
    // einzige beidseitige Vergleich des Deckels wieder weg (Issue #466).
    expect(eingecheckterWardDeckel, HINWEIS).toEqual(wardCapPayload());
  });

  it('die Ausrüstungs-Töpfe entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteAusruestung, HINWEIS).toEqual(gearPayload());
  });

  it('die Sätze entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteSaetze, HINWEIS).toEqual(setsPayload());
  });

  it('die Protokoll-Arten entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteProtokollArten, HINWEIS).toEqual(logKindsPayload());
  });

  it('der Auswahlmodus entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterAuswahlmodus, HINWEIS).toEqual(cooldownResetPayload());
  });

  it('der Kampf gegen den Boss mit Phasen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterBossZustand, HINWEIS).toEqual(bossStatePayload());
  });

  it('der Lauf mit Zaehler und Stapel entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterZaehler, HINWEIS).toEqual(doomStacksPayload());
    expect(eingecheckteNeueZauber, HINWEIS).toEqual(newCalcinationPayload());
  });

  it('der Lauf mit dem Auswahlmodus entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterAuswahlLauf, HINWEIS).toEqual(resetRunPayload());
  });

  it('der Lauf mit Wettrennen und Boss-Heilung entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterWettlauf, HINWEIS).toEqual(raceHealPayload());
  });

  it('der Lauf mit den letzten beiden Bausteinen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteDreiBausteine, HINWEIS).toEqual(trialBlocksPayload());
  });

  it('der Durchgang gegen die wiederholte Zeitleiste entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteWiederholung, HINWEIS).toEqual(timelineLoopPayload());
  });

  it('der Lauf mit Verkürzungen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteSublimation, HINWEIS).toEqual(sublimationPayload());
  });

  it('der Phasenwechsel mit gebundenem Ereignis entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterPhasen, HINWEIS).toEqual(phasenPayload());
  });

  it('der Vorrat über dem gefallenen Deckel entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterDeckel, HINWEIS).toEqual(deckelPayload());
  });

  it('ALEMBICs zwei Ladungen entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckterAlembic, HINWEIS).toEqual(alembicPayload());
  });

  it('der Lauf mit GEZIELTEN Verkürzungen entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteGezielteVerkuerzung, HINWEIS).toEqual(gezielteVerkuerzungPayload());
  });

  it('WISPs Verkürzung entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterWisp, HINWEIS).toEqual(wispPayload());
  });

  it('der Kampf mit eingehendem Schaden entspricht dem heutigen Spielstand', () => {
    expect(eingecheckterSpielerSchaden, HINWEIS).toEqual(playerDamagePayload());
  });

  it('die drei Wege der Koagulation entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteKoagulation, HINWEIS).toEqual(coagulationPayload());
  });

  it('die Kontroll-Lage entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteKontrolle, HINWEIS).toEqual(kontrolleWirktPayload());
  });

  it('die Fixierung entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteFixierung, HINWEIS).toEqual(fixationPayload());
  });

  it('die Protokoll-Arten des Spiels entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteProtokollParitaet, HINWEIS).toEqual(logParityPayload());
  });

  it('die Tempo-Umrechnung entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteUmrechnung, HINWEIS).toEqual(hastePayload());
  });

  it('der Lauf mit getragener Ausrüstung entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteGetrageneWerte, HINWEIS).toEqual(wornPayload());
  });

  it('der Lauf mit einer WIRKZEIT entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteWirkzeit, HINWEIS).toEqual(castTimePayload());
  });

  it('der Lauf gegen einen Boss mit WIDERSTAENDEN entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteWiderstaende, HINWEIS).toEqual(resistancePayload());
  });

  it('der Lauf mit GEMISCHTER Leiste entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteGemischteLeiste, HINWEIS).toEqual(resistanceMixedPayload());
  });

  it('der Lauf mit der davonlaufenden Reaktion entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteReaktion, HINWEIS).toEqual(runawayProgressPayload());
  });

  it('der Lauf auf Stufe 30 entspricht dem heutigen Spielstand', () => {
    expect(eingecheckteStufe, HINWEIS).toEqual(level30Payload());
  });

  it('die fünf Quellen entsprechen dem heutigen Spielstand', () => {
    expect(eingecheckteQuellen, HINWEIS).toEqual(sourcesPayload());
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
    expect(expected['finalTimelineIndex']).toBe((boss['timeline'] as unknown[]).length);
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

/**
 * Der Wächter über die Liste oben (Issue #456).
 *
 * # Warum es ihn braucht
 *
 * Diese Datei ist von Hand gepflegt, und **zweimal ist dabei etwas
 * durchgerutscht**:
 *
 *   #256/#289   `runaway-progress` und `level-30` fehlten
 *   #318/#456   `synergy-window` fehlte, `synergy-haste` kam dazu
 *
 * Beide Male fiel es nur auf, weil jemand zufällig eine neue Datei eintrug und
 * dabei hinsah. Der Kommentar oben hält den ersten Fall fest — und hat den
 * zweiten nicht verhindert. Ein Hinweis, der zum zweiten Mal überlesen wird,
 * ist keine Prüfung.
 *
 * # Was eine fehlende Datei bedeutet
 *
 * Der Simulator prüft gegen sie weiter. Wandert die Regel im Spiel, prüft er
 * gegen die alten Werte und **bleibt grün** — genau der Zustand, den der Kopf
 * dieser Datei als „sagt nichts mehr aus, wenn er gebraucht wird" beschreibt.
 * Eine unbewachte Aufzeichnung ist schlimmer als keine: Sie sieht aus wie
 * Absicherung.
 *
 * # Über `import.meta.glob` und nicht über das Dateisystem
 *
 * Der Kopf dieser Datei hält fest, dass das Projekt **bewusst keine Node-Typen**
 * führt. `readdirSync` schiede damit aus. Vite löst `import.meta.glob` beim
 * Übersetzen auf — dieselbe Auskunft, ohne die Entscheidung umzustossen.
 */
/**
 * Welche Aufzeichnungen oben einen Frische-Fall haben.
 *
 * **Als Tabelle und nicht als Kommentar**, damit der Wächter darunter sie
 * vergleichen kann. Dieselbe Technik wie bei `LIST_GUARDS` in `entry-shape.ts`
 * und `OHNE_AUSLOESER` in `factor-triggers.test.ts`: eine Liste, die eine
 * Prüfung einfordert, statt einer, auf die man sich verlässt.
 */
const MIT_FRISCHE_FALL: readonly string[] = [
  'buff-combat',
  'fixation-spells',
  'cooldown-precision',
  'primitives',
  'misplays',
  'rule-events',
  'buffs',
  'log-kinds',
  'cooldown-reset',
  'vessel-phases',
  'doom-stacks',
  'cooldown-reset-run',
  'race-heal',
  'trial-blocks',
  'timeline-loop',
  'sublimation',
  'gezielte-verkuerzung',
  'wisp-verkuerzung',
  'alembic-zwei-ladungen',
  'deckel-gefallen',
  'phasen-bindung',
  'gear',
  'sets',
  'player-damage',
  'coagulation',
  'kontrolle-wirkt',
  'fixation',
  'new-calcination',
  'log-parity',
  'haste',
  'worn-values',
  'sources',
  'cast-time',
  'resistances',
  'resistances-mixed',
  'runaway-progress',
  'level-30',
  'synergy-window',
  'synergy-haste',
  'ward-cap',
  'putrefaction-filler',
  'buff-targets',
  'cap-stapelung',
  'cinder-ticks',
  'instance2-boss-1',
  'instance2-boss-2',
  'instance2-boss-3',
  'instance2-boss-4',
  'instance1-boss-1',
  'instance1-boss-2',
  'instance1-boss-3',
  'instance1-boss-4',
  'dot-lifetime',
  'extend-effects',
  'kern-zauber',
  'efflux-restzeit',
  'krit-umwandlung',
  'rot-decay',
];

describe('Die Frische-Liste ist vollständig', () => {
  it('jede Aufzeichnung unter recordings/ hat einen Frische-Fall', () => {
    const imVerzeichnis = Object.keys(import.meta.glob('./recordings/*.json')).map((pfad) =>
      pfad.replace('./recordings/', '').replace('.json', ''),
    );

    // **Die Zusicherung vor der Messung.** Fände das Muster nichts, wäre der
    // Vergleich leer gegen leer und ginge durch, ohne etwas zu prüfen. Genau
    // diese Zeile hat den ersten Entwurf dieses Wächters gefangen: Der Versuch,
    // den eigenen Quelltext über `?raw` zu lesen, lieferte eine leere
    // Zeichenkette — und ohne sie wäre er grün gewesen.
    expect(imVerzeichnis.length, 'das Glob-Muster findet keine Aufzeichnungen').toBeGreaterThan(20);

    const fehlen = imVerzeichnis.filter((name) => !MIT_FRISCHE_FALL.includes(name));
    expect(
      fehlen,
      `Diese Aufzeichnungen stehen unter recordings/, aber in keinem Frische-Fall:\n` +
        fehlen.map((n) => `  ${n}`).join('\n') +
        `\n\nDer Simulator prüft gegen sie und bliebe grün, wenn die Regel im ` +
        `Spiel wanderte. Einen Import und einen Vergleichsfall ergänzen und den ` +
        `Namen in MIT_FRISCHE_FALL nachtragen.`,
    ).toEqual([]);
  });

  it('und die Tabelle nennt keine Datei, die es nicht gibt', () => {
    // Die andere Richtung: Ein Name, den niemand mehr exportiert, machte die
    // Tabelle unzuverlässig — und der Wächter oben verliesse sich auf sie.
    const imVerzeichnis = Object.keys(import.meta.glob('./recordings/*.json')).map((pfad) =>
      pfad.replace('./recordings/', '').replace('.json', ''),
    );
    const uebrig = MIT_FRISCHE_FALL.filter((name) => !imVerzeichnis.includes(name));
    expect(uebrig, `Diese Namen stehen in MIT_FRISCHE_FALL, aber unter recordings/ nicht`).toEqual(
      [],
    );
  });
});

/**
 * WAS DIESER WÄCHTER FÄNGT UND WAS NICHT
 *
 * Er fängt den Fall, der zweimal passiert ist: eine neue Aufzeichnung, die
 * niemand einträgt. Er fängt **nicht**, dass jemand nur den Namen nachträgt und
 * den Vergleichsfall vergisst — dafür müsste er den eigenen Quelltext lesen,
 * und der Versuch scheiterte an Vites Auflösung.
 *
 * Das ist eine echte Lücke und steht hier, statt verschwiegen zu werden. Sie
 * ist deutlich kleiner als die vorherige: Wer den Namen einträgt, hat die Datei
 * vor Augen.
 */
