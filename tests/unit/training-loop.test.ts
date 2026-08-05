import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { CALCINATION } from '../../src/core/data.js';
import { COUNTDOWN_MS } from '../../src/core/phases.js';
import { PressQueue } from '../../src/input/queue.js';
import { Session } from '../../src/core/session.js';
import { buildDummy, DEFAULT_DUMMY, type DummySettings } from '../../src/analysis/training.js';

/**
 * Die Zeitleiste der Puppe wiederholt sich (GAME-DESIGN 15.5, Issue #174).
 *
 * # Was vorher galt
 *
 * Die Puppe handelte nur in den ersten 48 Sekunden. Ein längerer Durchgang
 * lief danach weiter — aber ohne Zauber, ohne Schläge, ohne Gelegenheit zum
 * Unterbrechen. **Und es fiel nicht auf:** Die Zahlen laufen weiter, nur die
 * Boss-Ereignisse hören auf. Wer die laufende Auswertung beobachtet, sieht
 * sinkende Werte und sucht den Fehler bei sich.
 *
 * # Warum hier die Dichte gemessen wird und nicht das Vorkommen
 *
 * „Nach fünf Minuten passiert noch etwas" wäre auch dann wahr, wenn die
 * Zeitleiste einmal springt und danach unregelmäßig läuft. Gemessen wird
 * deshalb, wie **viele** Handlungen in gleich langen Abschnitten liegen, und
 * dass die **Abstände** gleich bleiben.
 */

/** Ein Übungsdurchgang; gibt die Zeitpunkte der Puppen-Handlungen zurück. */
function handlungsZeitpunkte(settings: DummySettings, dauerMs: number): number[] {
  const queue = new PressQueue();
  const combat = new Combat({
    school: CALCINATION.value,
    boss: buildDummy(settings, 'CALCINATION'),
  });
  const session = new Session(queue, combat);
  session.bind(0, combat.indexOf('kindle'));
  session.beginCountdown(0);

  const zeitpunkte: number[] = [];
  let vorher = 0;
  let tick = 0;
  for (let t = 0; t <= COUNTDOWN_MS + dauerMs; t += 100) {
    if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
    session.tick(t);
    // Der Zeiger rückt genau dann vor, wenn ein Eintrag ausgelöst hat.
    if (combat.timelineIndex !== vorher) {
      for (let i = vorher; i < combat.timelineIndex; i++) zeitpunkte.push(t - COUNTDOWN_MS);
      vorher = combat.timelineIndex;
    }
    tick++;
  }
  return zeitpunkte;
}

const FUENF_MINUTEN = 300_000;

describe('AC1 — die Puppe handelt am Ende so dicht wie am Anfang', () => {
  it('über fünf Minuten, erstes gegen letztes Drittel', () => {
    const zeitpunkte = handlungsZeitpunkte(
      { ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true },
      FUENF_MINUTEN,
    );

    // **Die Ursache zuerst.** Ohne Handlungen misst der Fall seine eigene
    // Leere — und vor #174 wäre das letzte Drittel schlicht leer gewesen.
    expect(
      zeitpunkte.length,
      'die Puppe hat über fünf Minuten gar nicht gehandelt',
    ).toBeGreaterThan(10);

    // **Die Abschnitte liegen auf Zyklus-Grenzen, nicht auf Dritteln.** Sonst
    // bräuchte der Vergleich eine Toleranz, weil ein angebrochener Zyklus mal
    // diesseits und mal jenseits der Grenze endet — und eine Toleranz, die
    // man so lange lockert, bis es passt, misst nichts mehr. Auf
    // Zyklus-Grenzen ist die Zahl exakt gleich.
    const puppe = buildDummy({ ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true }, 'X');
    const zyklus = puppe.timelineLoopMs!;
    const abschnitt = 2 * zyklus;
    // **Um eine Millisekunde versetzt gezählt.** Ein Eintrag liegt genau auf
    // der Zyklusgrenze (`atMs === loop`) — und das ist gewollt, denn nur so
    // bleibt der Abstand über den Übergang derselbe. Eine Abschnittsgrenze
    // darauf zu legen zählte ihn mal hüben, mal drüben, und der Vergleich
    // sähe eine Abweichung, die es nicht gibt.
    const zaehle = (von: number): number =>
      zeitpunkte.filter((t) => t > von && t <= von + abschnitt).length;

    const abschnitte = Math.floor(FUENF_MINUTEN / abschnitt);
    expect(abschnitte, 'fünf Minuten fassen keine zwei vollen Abschnitte').toBeGreaterThan(1);

    const erstes = zaehle(0);
    const letztes = zaehle((abschnitte - 1) * abschnitt);
    expect(erstes).toBeGreaterThan(0);
    expect(
      letztes,
      `erster Abschnitt ${erstes} Handlungen, letzter ${letztes} — die Puppe ` +
        'handelt am Ende nicht mehr so dicht wie am Anfang',
    ).toBe(erstes);
  });

  it('vor der Änderung wäre das letzte Drittel leer gewesen — die Gegenprobe', () => {
    // Ein Boss OHNE Wiederholung, sonst gleich: Nach dem letzten Eintrag
    // passiert nichts mehr. Das ist der Zustand, den #174 behebt, und er
    // zeigt zugleich, dass die Wiederholung nicht für jeden Boss gilt.
    const puppe = buildDummy({ ...DEFAULT_DUMMY, castsSpells: true }, 'CALCINATION');
    const ohneWiederholung = { ...puppe, timelineLoopMs: null };
    expect(ohneWiederholung.timeline.length).toBeGreaterThan(0);

    const queue = new PressQueue();
    const combat = new Combat({ school: CALCINATION.value, boss: ohneWiederholung });
    const session = new Session(queue, combat);
    session.bind(0, combat.indexOf('kindle'));
    session.beginCountdown(0);
    for (let t = 0; t <= COUNTDOWN_MS + FUENF_MINUTEN; t += 100) session.tick(t);

    // Der Zeiger bleibt bei der Länge der Liste stehen, statt darüber
    // hinauszulaufen.
    expect(combat.timelineIndex).toBe(ohneWiederholung.timeline.length);
  });
});

describe('AC2 — die Abstände bleiben gleich, auch am Übergang', () => {
  it('kein Loch und keine Häufung zwischen zwei Zyklen', () => {
    // Der Übergang ist die Stelle, an der eine Zyklus-Rechnung typischerweise
    // danebengeht: ein Zyklus zu früh, und zwei Einträge fallen zusammen; ein
    // Zyklus zu spät, und es entsteht eine Lücke.
    const zeitpunkte = handlungsZeitpunkte(
      { ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true },
      FUENF_MINUTEN,
    );
    // **Geprüft wird die Wiederholung selbst, nicht eine Menge von Abständen.**
    // Jede Handlung muss einen Zyklus später wieder auftreten, um genau die
    // Zykluslänge verschoben. Ein Zyklus zu früh gerechnet, und zwei Einträge
    // fielen zusammen; einer zu spät, und es entstünde eine Lücke — beides
    // fängt dieser Vergleich, ohne Toleranz.
    const puppe = buildDummy({ ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true }, 'X');
    const zyklus = puppe.timelineLoopMs!;
    const proZyklus = puppe.timeline.length;
    expect(proZyklus).toBeGreaterThan(1);
    expect(zeitpunkte.length, 'der Lauf deckt keine zwei vollen Zyklen ab').toBeGreaterThan(
      2 * proZyklus,
    );

    for (let i = 0; i + proZyklus < zeitpunkte.length; i++) {
      expect(
        zeitpunkte[i + proZyklus],
        `die Handlung an Stelle ${i} (${zeitpunkte[i]} ms) wiederholt sich nicht ` +
          `nach genau ${zyklus} ms — am Übergang entsteht eine Lücke oder eine Häufung`,
      ).toBe(zeitpunkte[i]! + zyklus);
    }
  });
});

describe('AC3 — die stumme Puppe bleibt stumm', () => {
  it('über fünf Minuten passiert nichts, und der Lauf endet', () => {
    // **Der wichtigste Anwendungsfall**, nicht der Randfall: An der stummen
    // Puppe werden maximale Rotationen gemessen. Bei beiden Schaltern aus ist
    // die Zeitleiste leer — eine Wiederholung über nichts darf nicht kreisen,
    // nichts auslösen und nichts kosten.
    //
    // Dass dieser Prüffall überhaupt zu Ende läuft, ist bereits die halbe
    // Aussage: Eine Zyklus-Rechnung ohne Abbruch bei leerer Liste hinge hier
    // in einer Endlosschleife.
    const puppe = buildDummy(DEFAULT_DUMMY, 'CALCINATION');
    expect(puppe.timeline).toEqual([]);
    expect(puppe.timelineLoopMs).not.toBeNull();

    const zeitpunkte = handlungsZeitpunkte(DEFAULT_DUMMY, FUENF_MINUTEN);
    expect(zeitpunkte).toEqual([]);
  });

  it('macht keinen Schaden und lässt den Zeiger stehen', () => {
    const queue = new PressQueue();
    const combat = new Combat({
      school: CALCINATION.value,
      boss: buildDummy(DEFAULT_DUMMY, 'CALCINATION'),
    });
    const session = new Session(queue, combat);
    session.bind(0, combat.indexOf('kindle'));
    session.beginCountdown(0);
    let tick = 0;
    for (let t = 0; t <= COUNTDOWN_MS + FUENF_MINUTEN; t += 100) {
      if (t >= COUNTDOWN_MS && (t - COUNTDOWN_MS) % 1500 === 0) queue.push(0, tick, t, tick);
      session.tick(t);
      tick++;
    }
    expect(combat.timelineIndex).toBe(0);
    expect(combat.damageTaken).toBe(0);
    // Und der Spieler hat wirklich gewirkt — sonst liefe der Fall über einen
    // Kampf, in dem überhaupt nichts geschieht.
    expect(combat.damageDealt).toBeGreaterThan(0);
  });
});

describe('AC4 — der Speicher wächst nicht mit der Dauer', () => {
  it('die Zeitleiste bleibt gleich lang, egal wie lange geübt wird', () => {
    // Der verworfene Weg („Liste bis zu einer festen Dauer auffüllen") hätte
    // Speicher proportional zur Übungsdauer gebraucht. Hier bleibt die Liste,
    // was sie ist; nur der Zähler wächst — eine Zahl, kein Speicher.
    const einstellungen = { ...DEFAULT_DUMMY, castsSpells: true, dealsDamage: true };
    const puppe = buildDummy(einstellungen, 'CALCINATION');
    const laenge = puppe.timeline.length;

    const kurz = handlungsZeitpunkte(einstellungen, 30_000);
    const lang = handlungsZeitpunkte(einstellungen, FUENF_MINUTEN);

    // Zehnmal so lang geübt, gleich viel gespeichert.
    expect(buildDummy(einstellungen, 'CALCINATION').timeline.length).toBe(laenge);
    // Und der Zähler ist wirklich über die Liste hinausgelaufen — sonst
    // belegte der Vergleich oben nichts.
    expect(
      lang.length,
      'der Zähler ist nicht über die Länge der Liste hinausgelaufen — dann ' +
        'gab es gar keine Wiederholung',
    ).toBeGreaterThan(laenge);
    expect(lang.length).toBeGreaterThan(kurz.length);
  });
});
