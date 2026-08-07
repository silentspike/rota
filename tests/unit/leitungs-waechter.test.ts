/**
 * Welche `Combat`-Getter haben keinen Weg ins `ReplayResult`? (Issue #612)
 *
 * # Warum es diesen Wächter gibt
 *
 * Dreimal dieselbe Bauart, dreimal einzeln gefunden:
 *
 * ```text
 *   #564  bossHealed                                geführt, nicht abrufbar
 *   #576  damageTaken, absorbed, healed, converted  dasselbe, vier auf einmal
 *   #605  controlSpent, controlFailed               dasselbe — und controlFailed
 *                                                   wird dem Spieler angezeigt
 * ```
 *
 * `recording.ts` **beschreibt die Klasse seit #576 schriftlich**: „Dieselbe
 * Bauart wie `boss_healed` vor #564: geführt, paritätsgesichert, und im
 * Werkzeug nicht abrufbar." Der Kommentar stand da, als #605 gefunden wurde.
 *
 * Ein Kommentar wird einmal gelesen. Ein Prüffall meldet sich wieder.
 *
 * # Was eine fehlende Leitung kostet — gemessen, nicht argumentiert
 *
 * Nach #605, als `controlSpent` seinen Weg ins `ReplayResult` bekam, wurden
 * dieselben Grössen neu vermessen:
 *
 * ```text
 *   control.cost  stall   4 -> 2    0,00 %  ->  50,00 %
 *   control.cost  stay    3 -> 1    0,00 %  ->  66,67 %
 *   control.cost  anchor  6 -> 3    0,00 %  ->   8,70 %
 * ```
 *
 * **Die Nullen waren keine Messung, sondern die fehlende Leitung.** Elf
 * Kontroll-Grössen standen in #592 auf 0,00 % — nicht weil sie belanglos
 * wären, sondern weil ihre einzige beobachtbare Folge nirgends ankam.
 *
 * Eine fehlende Leitung erzeugt eine 0,00 %, die aussieht wie ein Befund. Das
 * ist der Grund, warum es diesen Wächter gibt.
 *
 * # Er prüft, ob eine Grösse ANKOMMT — nicht, ob sie richtig ist
 *
 * Das sind zwei Fragen, und bisher hat nur die erste jemand gestellt. Genau
 * deshalb mussten #564, #576 und #605 einzeln gefunden werden: **Die Mechanik
 * war jedes Mal geprüft.** Ein Fall, der `combat.controlSpent` liest, bleibt
 * grün, wenn die Leitung ins `ReplayResult` fehlt — er fragt die Quelle, nicht
 * das Ziel.
 *
 * # AC5 — „keine Leitung" gegen „Leitung da, in diesem Lauf nicht gefüllt"
 *
 * Der Wächter arbeitet auf der **Struktur**, nicht auf Werten. Er vergleicht
 * die Getter des Prototyps mit den Schlüsseln eines echten `ReplayResult` und
 * mit den Zuweisungen in `recording.ts`. Ein Feld, das im Ergebnis steht und
 * `0` trägt, gilt als vorhanden — die Null ist eine Aussage über den Lauf, kein
 * Mangel der Leitung.
 *
 * Daran ist der erste Entwurf von #424 gescheitert: Er zählte gefüllte Werte
 * und hielt jede Null für ein Loch.
 *
 * # Warum die Zuweisungen aus dem Quelltext kommen
 *
 * `damageDealt` heisst im Ergebnis `damage`, `shieldAbsorbed` heisst
 * `absorbed`. Ein Vergleich über Namensgleichheit meldete beide als ohne
 * Leitung — 59 statt 57 Namen, zwei davon falsch. Die Zuweisungen
 * `feld: combat.getter` sind die einzige Stelle, an der die Zuordnung wirklich
 * steht.
 */
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { replayRecording } from '../../src/devtools/recording.js';

const QUELLE = new URL('../../src/devtools/recording.ts', import.meta.url).pathname;
const CHECKSUM = new URL('../../src/core/checksum.ts', import.meta.url).pathname;

/**
 * Die Getter, die keinen Weg ins `ReplayResult` haben — mit ihrer Einordnung.
 *
 * **Jeder Name trägt eine Zeile.** Ein Wächter, der Namen meldet, ohne zu
 * sagen was sie bedeuten, wird beim ersten roten Lauf entfernt statt gelesen.
 */
const EINORDNUNG: Readonly<Record<string, string>> = {
  // ---- Absicht: über die Zustands-Prüfsumme gedeckt -----------------------
  // Diese Grössen vergleicht die Parität je Takt. Ein eigenes Feld im Ergebnis
  // machte die Datei länger, nicht schärfer.
  //
  // (Die Namen stehen nicht einzeln hier — sie werden gegen `checksum.ts`
  // geprüft, damit die Liste nicht altert, wenn ein Feld dazukommt.)

  // ---- Absicht: abgeleitet, die Quelle ist gedeckt ------------------------
  bossHpRemaining: 'abgeleitet aus damageDealt (gedeckt)',
  bossHpFraction: 'abgeleitet aus damageDealt (gedeckt)',
  bossHpPercent: 'abgeleitet aus damageDealt (gedeckt)',
  damageFactors: 'Zwischenwert der Schadensrechnung, kein Ergebnis',
  critFactorsMirrorBase: 'Selbstprüfung der Faktoren, kein Lauf-Ergebnis',

  // ---- Absicht: Konfiguration, nicht Ergebnis -----------------------------
  level: 'Eingabe des Laufs, steht in der Aufzeichnung',
  wornValues: 'Eingabe des Laufs, steht in der Aufzeichnung',
  talentAllocation: 'Eingabe des Laufs',
  wardReduction: 'Eingabe des Laufs',
  resourceId: 'Eigenschaft der Schule',
  resourceCap: 'Eigenschaft der Schule',

  // ---- Absicht: Zeitpunkte und Anzeige ------------------------------------
  fightStartedAt: 'Uhrzeit, kein Ergebnis',
  endedAt: 'Uhrzeit — das Ende steht in killedAtMs und diedAtMs',
  castingStartedAtMs: 'Zwischenstand eines laufenden Zaubers',
  castingName: 'Anzeige-Text',
  playerDead: 'gedeckt: diedAtMs sagt dasselbe und mehr',

  // ---- Absicht: Zwischenstand, am Laufende bedeutungslos -------------------
  resetArmed: 'Zwischenstand, am Laufende ohne Aussage',
  resetArmedFor: 'Zwischenstand',
  resetChargesLeft: 'Zwischenstand',
  vulnerability: 'Zwischenstand',
  raceWon: 'Zwischenstand des Rennens',

  // ---- OFFENE BEFUNDE: kumulative Zähler ohne Leitung ---------------------
  //
  // **Hier standen bis #619 fünf Namen.** Sie haben jetzt eine Leitung, und
  // ihre Einträge sind mit ihr verschwunden — `interruptsMissed`,
  // `interruptsLanded`, `castsInterrupted`, `castsCancelled`, `cycle` und
  // `stabilised`.
  //
  // Das war kein Aufräumen nebenbei: Der Fall „AC3 — die Gegenrichtung" wurde
  // ROT, als die Leitungen lagen, und blieb es, bis die Einträge weg waren.
  // Genau dafür gibt es ihn. Ohne ihn behaupteten sechs Zeilen weiter, es gebe
  // keine Leitung.
  // **„OFFEN" kann zwei Dinge heissen**, und das ist bei #619 aufgefallen:
  //
  //   KEINE LEITUNG        die Grösse kommt nicht ins ReplayResult
  //   NIRGENDS VERGLICHEN  sie kommt nirgends an, auch nicht über ein
  //                        von Hand eingetragenes `expected`
  //
  // Bei den sechs aus #619 galt das erste für alle, das zweite nur für
  // `stabilised`. Ich hatte beides in einer Zeile zusammengezogen und dabei
  // die schärfere Aussage verloren. Jeder Eintrag sagt deshalb, welches gilt.
  bossEnergyTriggers:
    'OFFEN (keine Leitung UND nirgends verglichen) — wie oft die Energieleiste ' +
    'ausgelöst hat. Anders als die sechs aus #619 führt Rust ihn nicht, also ist ' +
    'er auch dort keine Lücke: Er hat auf keiner Seite einen Weg.',
};

function getterVonCombat(): string[] {
  const combat = new Combat({ school: CALCINATION.value, boss: FIRST_CRUCIBLE.value, seed: 0 });
  const proto = Object.getPrototypeOf(combat) as object;
  return Object.getOwnPropertyNames(proto).filter(
    (name) => Object.getOwnPropertyDescriptor(proto, name)?.get !== undefined,
  );
}

/**
 * Die Getter, aus denen ein Feld des `ReplayResult` gespeist wird.
 *
 * Gelesen aus den Zuweisungen `feld: combat.getter`. Das ist die einzige
 * Stelle, an der die Zuordnung wirklich steht — über Namensgleichheit gälten
 * `damageDealt` und `shieldAbsorbed` als ohne Leitung.
 */
function mitLeitung(): Set<string> {
  const quelle = readFileSync(QUELLE, 'utf8');
  return new Set([...quelle.matchAll(/\w+:\s*combat\.(\w+)/g)].map((m) => m[1] as string));
}

/** Die Getter, die je Takt in die Zustands-Prüfsumme eingehen. */
function inPruefsumme(): Set<string> {
  const quelle = readFileSync(CHECKSUM, 'utf8');
  return new Set([...quelle.matchAll(/combat\.(\w+)/g)].map((m) => m[1] as string));
}

describe('Jeder Combat-Getter hat einen Weg oder eine Begründung', () => {
  it('es gibt überhaupt etwas zu prüfen', () => {
    // **Die Zusicherung vor der Messung.** Fände die Prototyp-Suche keine
    // Getter oder das ReplayResult keine Felder, wäre alles darunter grün und
    // prüfte nichts.
    const getter = getterVonCombat();
    const ergebnis = replayRecording({ name: 'zusicherung', seed: 0, ticks: 10, presses: [] });
    expect(getter.length, 'keine Getter gefunden').toBeGreaterThan(40);
    expect(Object.keys(ergebnis).length, 'kein ReplayResult-Feld gefunden').toBeGreaterThan(15);
    expect(mitLeitung().size, 'keine Zuweisung feld: combat.x gefunden').toBeGreaterThan(5);
    expect(inPruefsumme().size, 'checksum.ts liest keinen Getter').toBeGreaterThan(10);
  });

  it('AC1/AC2 — kein Getter ohne Weg und ohne Einordnung', () => {
    const gedeckt = inPruefsumme();
    const leitung = mitLeitung();
    const ohneAlles = getterVonCombat().filter(
      (g) => !leitung.has(g) && !gedeckt.has(g) && EINORDNUNG[g] === undefined,
    );

    expect(
      ohneAlles,
      'Diese Getter haben weder eine Leitung ins ReplayResult noch stehen sie in der ' +
        'Prüfsumme noch tragen sie eine Einordnung. Entweder eine Leitung legen oder ' +
        'eine Zeile in EINORDNUNG schreiben — mit Grund, nicht mit „egal".',
    ).toEqual([]);
  });

  it('AC3 — die Gegenrichtung: ein Eintrag für einen Getter MIT Leitung ist überholt', () => {
    // **Ohne diesen Fall verrottet die Liste.** Bekommt ein Getter eine
    // Leitung, bleibt sein Ausnahme-Eintrag stehen und behauptet weiter, es
    // gebe keine — genau das ist bei #576 und #605 passiert, wo die Klasse im
    // Kommentar stand und die Fälle trotzdem einzeln gefunden wurden.
    const leitung = mitLeitung();
    const ueberholt = Object.keys(EINORDNUNG).filter((g) => leitung.has(g));
    expect(
      ueberholt,
      'Diese Namen stehen in EINORDNUNG und haben inzwischen eine Leitung — ' +
        'die Einträge gehören entfernt',
    ).toEqual([]);
  });

  it('AC5 — eine Null im Ergebnis ist KEINE fehlende Leitung', () => {
    // Der Fehler, an dem der erste Entwurf von #424 scheiterte: Werte zählen
    // statt Struktur. Ein Lauf ohne Kontroll-Zauber liefert `controlSpent: 0` —
    // die Leitung ist da, der Lauf hat nur nichts ausgegeben.
    const ergebnis = replayRecording({ name: 'leer', seed: 0, ticks: 10, presses: [] });
    expect(ergebnis.controlSpent, 'der Prüflauf gibt doch Punkte aus').toBe(0);
    expect(
      Object.keys(ergebnis),
      'controlSpent fehlt im Ergebnis — dann prüft der Fall die falsche Sache',
    ).toContain('controlSpent');
    // Und die Leitung ist über die Zuweisung belegt, nicht über den Wert.
    expect(mitLeitung().has('controlSpent')).toBe(true);
  });

  it('die offenen Befunde stehen als solche da, nicht als Absicht', () => {
    // **AC4, ehrlich beantwortet.** Der Wächter ist grün eingecheckt, weil
    // jeder heutige Name eine Zeile hat. Rot wird er bei einem NEUEN Getter
    // ohne Eintrag — das ist sein Zweck. Damit die grüne Liste nicht als
    // „alles in Ordnung" gelesen wird, muss mindestens ein OFFEN darin stehen.
    const offen = Object.entries(EINORDNUNG).filter(([, grund]) => grund.startsWith('OFFEN'));
    expect(
      offen.length,
      'kein einziger offener Befund — dann ist die Liste geschönt',
    ).toBeGreaterThan(0);
    // **Diese Zusicherung hing bis #619 an einem NAMEN** (`interruptsMissed`),
    // und der ist gewandert, als er eine Leitung bekam — der Fall wurde rot
    // und meldete einen Fehler, der keiner war.
    //
    // Eine Zusicherung, die einen konkreten offenen Befund nennt, altert mit
    // seiner Behebung. Jetzt hängt sie an der EIGENSCHAFT: Jeder OFFEN-Eintrag
    // muss sagen, was fehlt. Ein blosses „OFFEN" ohne Satz wäre eine beruhigte
    // Liste mit anderem Wort.
    for (const [name, grund] of offen) {
      expect(grund.length, `${name} ist als OFFEN markiert, sagt aber nicht warum`).toBeGreaterThan(
        30,
      );
    }
  });
});
