/**
 * **Die Absicht gegen die Wirklichkeit** — wer schreibt welche Ereignisart?
 * (Issue #676)
 *
 * # Was hier geprüft wird und warum es das braucht
 *
 * `EVENT_KIND_INTENT` in `src/core/log.ts` sagt je Art, welche Seite sie
 * schreiben soll. Diese Datei fährt echte Kämpfe und vergleicht, was
 * tatsächlich im Verlauf landet. Beide Richtungen färben:
 *
 * ```text
 *   Absicht sagt "das Spiel schreibt"  und kein Lauf schreibt sie   -> rot
 *   Absicht sagt "das Spiel schreibt NICHT" und ein Lauf schreibt sie -> rot
 * ```
 *
 * # Warum nicht per grep über die Schreibstellen
 *
 * Beim Erarbeiten dieses Issues habe ich es **zweimal** mit grep versucht und
 * beide Male falsch gemessen — in entgegengesetzte Richtungen:
 *
 * ```text
 *   Muster "push(.*EVENT_X"      -> Art 8 als NICHT geschrieben gemeldet
 *   Muster "nur der Name"        -> Art 8 als NICHT geschrieben gemeldet
 * ```
 *
 * Der Grund ist beide Male derselbe: Der Aufruf in `combat.ts` ist über fünf
 * Zeilen umbrochen, und `EVENT_BOSS_ENERGY_TRIGGERED` steht allein auf einer
 * davon. **Ein Muster über Textzeilen misst die Formatierung, nicht die
 * Struktur.** Aufgefallen ist es nur, weil ich mit Art 8 einen Fall hatte,
 * dessen Antwort ich kannte.
 *
 * Ein Lauf hat dieses Problem nicht: Er zählt, was ankommt.
 *
 * # Die Grenze dieses Verfahrens — und wie sie behandelt wird
 *
 * Ein Lauf kann nur zeigen, was er auslöst. Eine Art, die keiner der Kämpfe
 * unten hervorruft, ist damit nicht widerlegt, sondern **unerreicht** — und
 * das ist etwas anderes. Genau daran scheiterte der bestehende Vergleich:
 * `log-parity.json` fährt `test-warden`, der keine Energieleiste hat, also
 * kam Art 8 dort nie vor und beide Seiten verglichen zwei Nullen.
 *
 * Deshalb führt `UNERREICHT` unten die Arten, für die noch kein Lauf
 * existiert, **namentlich und mit Grund**. Sie sind vom Soll-Ist-Vergleich
 * ausgenommen, aber sichtbar — eine stille Ausnahme wäre genau die Bauart,
 * die in #666 und #667 der Fehler war.
 */
import { describe, expect, it } from 'vitest';
import { Combat, alsSpellIndex } from '../../src/core/combat.js';
import { bossById, CALCINATION, FIRST_CRUCIBLE } from '../../src/core/data.js';
import { CombatLog, EVENT_KIND_INTENT, EVENT_NAMES } from '../../src/core/log.js';
import { TICK_MS } from '../../src/core/loop.js';

/**
 * Arten, die **kein** Lauf dieser Datei auslöst — mit dem Grund, warum.
 *
 * Sie werden nicht als „schreibt nicht" gewertet. Wer einen Lauf ergänzt,
 * der eine davon auslöst, streicht sie hier.
 */
const UNERREICHT: Readonly<Record<string, string>> = {
  // **Gemessen, nicht vermutet.** Die Krit-Chance ohne Ausrüstung liegt so
  // tief, dass keiner der Läufe unten einen auslöst. Ein Lauf mit YIELD
  // würde sie erreichen — das ist eine eigene Lage und gehört zu den
  // Ausrüstungs-Fällen, nicht hierher.
  crit: 'die Krit-Chance ohne getragene Werte löst in diesen Läufen keinen aus',
  // SALVE heilt nur, wenn der Spieler Schaden genommen hat UND der Zauber
  // bereit ist. In den Läufen unten fällt beides nicht zusammen.
  healed: 'verlangt verletzten Spieler und bereiten SALVE im selben Fenster',
};

/** Ein Lauf und die Arten, die er geschrieben hat. */
function artenAus(log: CombatLog): Set<string> {
  const gesehen = new Set<string>();
  for (const eintrag of log.toArray()) gesehen.add(eintrag.type);
  return gesehen;
}

/** Passiver Lauf gegen einen Boss — die Zeitleiste und die Leiste laufen allein. */
function passiv(bossId: string, bisMs: number): Set<string> {
  const boss = bossById(bossId);
  if (boss === undefined) throw new Error(`data/bosses/${bossId}.json fehlt.`);
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never });
  const log = new CombatLog(8192);
  combat.log = log;
  combat.beginFight(0);
  for (let t = 0; t <= bisMs; t += TICK_MS) combat.stepPassive(t);
  return artenAus(log);
}

/**
 * Lauf mit Tastendrücken. `waehle` liefert den Zauber-Index oder -1.
 *
 * **Warum die Auswahl je Lauf übergeben wird und nicht fest ist:** Eine
 * einzige Prioritätenliste deckt die Arten NICHT ab. Gemessen: Mit der
 * Reihenfolge CRUST > QUENCH > SALVE > SMOULDER > KINDLE kommt `kindle` nie
 * an die Reihe — und damit fehlt ausgerechnet `damage`, die häufigste Art
 * überhaupt. Jeder Lauf unten hat deshalb einen Zweck und eine eigene Wahl.
 */
function mitDruecken(
  bossId: string,
  waehle: (combat: Combat, timeMs: number, tick: number) => number,
  ticks: number,
): Set<string> {
  const boss = bossById(bossId) ?? FIRST_CRUCIBLE.value;
  const combat = new Combat({ school: CALCINATION.value, boss: boss as never, seed: 0 });
  const log = new CombatLog(16_384);
  combat.log = log;
  combat.beginFight(0);

  for (let tick = 0; tick < ticks; tick++) {
    const timeMs = tick * TICK_MS;
    combat.stepPassive(timeMs);
    const gewaehlt = waehle(combat, timeMs, tick);
    if (gewaehlt >= 0 && combat.isReady(alsSpellIndex(gewaehlt), timeMs)) {
      combat.cast(alsSpellIndex(gewaehlt), timeMs);
    }
  }
  return artenAus(log);
}

/**
 * Alle Arten, die irgendein Lauf dieser Datei geschrieben hat.
 *
 * Jeder Eintrag steht für eine Lage, die eine bestimmte Art hervorruft — die
 * Zuordnung steht daneben, damit beim Streichen eines Laufs auffällt, was
 * dabei verlorengeht.
 */
function allesGeschriebene(): Set<string> {
  const alle = new Set<string>();
  const dazu = (arten: Set<string>): void => {
    for (const art of arten) alle.add(art);
  };

  // **Ein Boss MIT Energieleiste** — sonst bleibt Art 8 unerreicht. Genau
  // daran scheiterte der bestehende Vergleich: `log-parity.json` fährt
  // `test-warden` mit `energy: null`, dort kam die Art nie vor, und beide
  // Seiten verglichen zwei Nullen (Issue #676).
  //                                          -> bossEnergyTriggered
  dazu(passiv('test-vessel', 60_000));

  // Nur KINDLE: der einfachste Schadensweg.  -> damage, cast, playerDamage
  dazu(mitDruecken('test-warden', (c) => c.indexOf('kindle'), 3000));

  // Ein Boss, dessen Phase wirklich wechselt. -> phase
  dazu(mitDruecken('test-phase', (c) => c.indexOf('kindle'), 3000));

  // Die volle Leiste an einem Boss mit Zeitleiste. Hier kommt KINDLE nicht
  // dran — das ist gewollt, der Lauf darüber deckt es ab.
  //     -> bossTimelineFired, interruptLanded, dotTick, shieldAbsorb
  //
  // **Die Reihenfolge ist gemessen, nicht gewählt.** Sie nimmt den ERSTEN
  // bereiten Zauber dieser festen Folge. Feiner gesteuerte Fassungen — etwa
  // „SMOULDER nur, wenn der Brand ausläuft" — lieferten `dotTick` NICHT:
  // gemessen 20 SMOULDER-Würfe und kein einziger Tick im Verlauf. Warum die
  // grobe Folge ihn hervorruft und die gezielte nicht, ist offen; hier zählt,
  // welche Lage die Art nachweislich erzeugt.
  dazu(
    mitDruecken(
      'test-warden',
      (c, timeMs) => {
        for (const name of ['crust', 'quench', 'salve', 'smoulder', 'kindle']) {
          const i = c.indexOf(name);
          if (i >= 0 && c.isReady(alsSpellIndex(i), timeMs)) return i;
        }
        return -1;
      },
      3000,
    ),
  );

  // **Eine Verstärkung, die ausläuft** (Issue #682).  -> buffExpire
  //
  // Vorher trug die Art eine `bekannteLuecke` und fiel damit aus beiden
  // Vergleichen; seit #682 schreiben beide Seiten sie, die Ausnahme ist weg
  // — und ohne diesen Lauf stünde sie als „kein Lauf tut es" da.
  //
  // SLAG setzt die Verstärkung und hält vier Wartezeiten. Der Lauf muss
  // deutlich länger sein als der Wurf, sonst endet er VOR dem Ablauf: Ein
  // Lauf, der zu früh aufhört, belegt nur seine eigene Uhr.
  dazu(mitDruecken('test-warden', (c) => c.indexOf('slag'), 9000));

  return alle;
}

describe('AC2 — die Absicht stimmt mit der Wirklichkeit überein', () => {
  it('die Lage misst überhaupt etwas', () => {
    // **Die Zusicherung VOR jedem Vergleich.** Schriebe kein Lauf irgendetwas,
    // wären beide Schleifen unten leer und beide Fälle grün — ein Wächter, der
    // nichts sieht, meldet nichts.
    const geschrieben = allesGeschriebene();
    expect(geschrieben.size, 'kein einziger Verlaufs-Eintrag in allen Läufen').toBeGreaterThan(0);
    expect(
      Object.keys(EVENT_KIND_INTENT).length,
      'die Absichts-Tabelle ist leer',
    ).toBeGreaterThanOrEqual(17);
  });

  it('die Absichts-Tabelle führt jede Art des Nummernraums — und keine fremde', () => {
    // Beide Richtungen, wie bei den Verstärker-Zielen in #660: Eine Tabelle,
    // die einen Teil ihrer Gattung führt, sieht vollständig aus. `controlSpent`
    // fehlte bis #676 in der Markdown-Tabelle — genau so.
    const imRaum = EVENT_NAMES.filter((n) => n !== undefined);
    for (const name of imRaum) {
      expect(
        EVENT_KIND_INTENT[name],
        `Art "${name}" steht im Nummernraum, aber in keiner Absicht`,
      ).toBeDefined();
    }
    for (const name of Object.keys(EVENT_KIND_INTENT)) {
      expect(imRaum, `Absicht für "${name}", die es im Nummernraum nicht gibt`).toContain(name);
    }
  });

  it('keine Art wird geschrieben, die laut Absicht das Spiel nicht schreibt', () => {
    // Die eindeutige Richtung: Hier kann kein Lauf zu wenig zeigen. Was
    // geschrieben wurde, wurde geschrieben.
    const geschrieben = allesGeschriebene();
    const unerwartet = [...geschrieben].filter((name) => EVENT_KIND_INTENT[name]?.game === false);
    expect(
      unerwartet,
      `diese Arten schreibt das Spiel, obwohl die Absicht "nein" sagt: ${unerwartet.join(', ')}`,
    ).toEqual([]);
  });

  it('jede Art, die das Spiel laut Absicht schreibt, wird auch geschrieben', () => {
    const geschrieben = allesGeschriebene();
    const fehlend = Object.entries(EVENT_KIND_INTENT)
      .filter(([name, absicht]) => absicht.game && !geschrieben.has(name))
      // **Zwei verschiedene Gründe, hier zu fehlen** — und sie sind nicht
      // dasselbe:
      //   UNERREICHT      die Art funktioniert, dieser Lauf löst sie nicht aus
      //   bekannteLuecke  die Art hat gar keine Schreibstelle, gemeldet
      // Beide werden ausgenommen, beide stehen namentlich im Code, und der
      // Fall unten färbt, wenn eine ohne Begründung dazukommt.
      .filter(([name, absicht]) => UNERREICHT[name] === undefined && !absicht.bekannteLuecke)
      .map(([name]) => name);
    expect(
      fehlend,
      `laut Absicht schreibt das Spiel diese Arten — kein Lauf tut es: ${fehlend.join(', ')}`,
    ).toEqual([]);
  });

  it('die bekannten Lücken sind gemeldet und nennen ihr Issue', () => {
    // Eine Ausnahme ohne Verweis ist eine stille Ausnahme. Diese Zeile ist
    // der Preis dafür, dass `bekannteLuecke` den Wächter beruhigen darf.
    const luecken = Object.entries(EVENT_KIND_INTENT).filter(([, a]) => a.bekannteLuecke);
    for (const [name, absicht] of luecken) {
      expect(
        absicht.bekannteLuecke,
        `"${name}" ist als bekannte Lücke markiert, nennt aber kein Issue`,
      ).toMatch(/#\d+/);
      // **Und nicht auf dieses Issue hier.** Mein erster Entwurf verwies auf
      // #676 — also auf das Issue, das diesen Wächter baut. Ein solcher
      // Verweis sieht gültig aus und führt nirgendwohin: Wird #676
      // geschlossen, bleibt die Lücke stehen und zeigt auf etwas Erledigtes.
      // Eine bekannte Lücke braucht den Zeiger auf ihre AUFLÖSUNG.
      expect(
        absicht.bekannteLuecke,
        `"${name}" verweist auf #676 — das ist der Wächter selbst, nicht der Weg hinaus`,
      ).not.toMatch(/#676\b/);
    }
    // **Die Zusicherung gegen das stille Wachsen.** Wer eine Art hier
    // einträgt, statt sie zu bauen, muss diese Liste anfassen — und damit
    // erklären, warum sie länger wird.
    //
    // Sie ist seit #682 **leer**: `buffExpire` war der einzige Eintrag, und
    // die Lücke ist geschlossen statt verwaltet. Dass die Liste schrumpfen
    // kann, ist der Zweck — eine Ausnahme, die niemand je wieder entfernt,
    // wäre eine Dauereinrichtung mit Issue-Nummer.
    expect(
      luecken.map(([name]) => name),
      'die Liste der bekannten Lücken hat sich geändert',
    ).toEqual([]);
  });

  it('AC4-Probe — der Wächter erkennt Art 8 als geschrieben', () => {
    // **Die Probe gegen meinen eigenen Messfehler.** Zwei grep-Muster haben
    // Art 8 als nicht geschrieben gemeldet, obwohl die Schreibstelle in
    // `combat.ts` steht — sie ist über fünf Zeilen umbrochen. Ein Wächter,
    // der seine eigene Erkennung nicht prüft, meldet Lücken, die es nicht
    // gibt. Diese Zeile ist der Fall, dessen Antwort ich kenne.
    const geschrieben = passiv('test-vessel', 60_000);
    expect(
      geschrieben.has('bossEnergyTriggered'),
      'Art 8 wird von test-vessel nicht geschrieben — entweder ist die ' +
        'Erkennung kaputt oder #679 ist nicht im Stand',
    ).toBe(true);
  });

  it('die unerreichten Arten sind benannt, nicht stillschweigend übergangen', () => {
    // Eine Ausnahme, die niemand sieht, ist eine Lücke. Diese Zeile zwingt
    // dazu, sie zu begründen — und färbt, wenn jemand eine Art in UNERREICHT
    // schreibt, die es gar nicht gibt.
    for (const [name, grund] of Object.entries(UNERREICHT)) {
      expect(
        EVENT_KIND_INTENT[name],
        `UNERREICHT nennt "${name}" — die Art gibt es nicht`,
      ).toBeDefined();
      expect(grund.length, `"${name}" steht ohne Grund in UNERREICHT`).toBeGreaterThan(10);
    }
  });
});
