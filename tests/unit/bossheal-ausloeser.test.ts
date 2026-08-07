/**
 * Baustein 5 hat zwei Auslöser, nicht einen (Issue #609).
 *
 * # Was vorher schief war
 *
 * `healBoss()` war schon immer auslöser-agnostisch — es liest den Datensatz
 * und dreht zurück. Gerufen wurde es aber an **einer** Stelle: wenn ein
 * Boss-Zauber durchkam. Damit verlangte jeder Boss, der Fortschritt kosten
 * soll, dieselbe Taste, und `boss-1-4` und `boss-2-4` lagen näher beieinander,
 * als ihre Signaturen behaupten.
 *
 * GAME-DESIGN 11.2 sagt zu Baustein 5 wörtlich „**ein Fehler** macht
 * Fortschritt rückgängig" — nicht „eine verpasste Unterbrechung". Der Code war
 * enger als das Design.
 *
 * # Warum die Zusicherungen den BETRAG nennen, nicht nur den Ausgang
 *
 * Nach Guide 8.68: Ein Boss, der sich zurückheilt, ändert mehr als
 * „gewonnen/verloren". Ein Fall, der nur den Ausgang prüft, bliebe grün, wenn
 * die Heilung um den Faktor zehn danebenläge — und genau daran hängt, ob der
 * Kampf noch schaffbar ist.
 */
import { describe, expect, it } from 'vitest';
import { Combat } from '../../src/core/combat.js';
import { BOSSES, CALCINATION, type BossDef } from '../../src/core/data.js';

function boss(id: string): BossDef {
  const eintrag = BOSSES.get(id);
  expect(eintrag, `${id} steht nicht in BOSSES`).toBeDefined();
  return eintrag!.value;
}

/**
 * Fährt `test-lapse` bis `bisMs` und gibt zurück, wie viel zurückgedreht wurde.
 *
 * @param reinigen ob QUENCH gedrückt wird, sobald die Schwächung steht.
 */
function lauf(reinigen: boolean, bisMs: number): { geheilt: number; schritte: number } {
  const c = new Combat({ school: CALCINATION.value, boss: boss('test-lapse') });
  const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
  for (let t = 0; t <= bisMs; t += 10) {
    c.stepPassive(t);
    if (reinigen && c.afflicted && c.isOffCooldown(i('quench'), t)) c.cast(i('quench'), t);
    else if (c.gcd.isReady(t) && c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
  }
  return { geheilt: c.bossHealed, schritte: c.bossHealed / 50 };
}

describe('der zweite Auslöser: eine verpasste Reinigung dreht Fortschritt zurück', () => {
  it('der Prüf-Boss trägt den neuen Auslöser überhaupt', () => {
    // Die Zusicherung vor der Messung: Stünde hier `interrupt`, wären alle
    // Fälle darunter Aussagen über den ALTEN Auslöser — und grün.
    const b = boss('test-lapse');
    expect(b.bossHeal?.on).toBe('affliction');
    expect(b.affliction, 'ohne Schwächung löst der Auslöser nie aus').not.toBeNull();
  });

  it('WER NICHT REINIGT, VERLIERT FORTSCHRITT — je Schritt, nicht einmal', () => {
    // Schwächung ab 5000 ms, Takt 1000 ms. Bis 12000 ms sind das sieben
    // Schritte; der erste setzt sie nur, die folgenden zehren.
    const r = lauf(false, 12_000);
    expect(r.geheilt, 'gar nichts zurückgedreht — der Auslöser greift nicht').toBeGreaterThan(0);

    // **Der Betrag, nicht nur „mehr als null"** (Guide 8.68). Sieben zehrende
    // Schritte à 50. Läge die Heilung um einen Faktor daneben, bliebe ein
    // `toBeGreaterThan(0)` grün.
    //
    // **Und der Betrag musste gemessen werden, nicht gerechnet:** Der erste
    // Entwurf des Prüf-Bosses drehte 400 je Schritt zurück, mehr als die
    // Rotation nachlegt. `undoProgress` deckelt bei null, also zählten von
    // sechs Schritten nur zwei — `bossHealed` stand bei 800 statt bei 2400.
    // Der Fall hätte den Deckel gemessen und nicht den Auslöser.
    expect(r.schritte).toBe(7);
    expect(r.geheilt).toBe(350);
  });

  it('wer reinigt, verliert nichts — die Gegenprobe', () => {
    // Ohne diesen Fall bliebe offen, ob die Heilung am Nicht-Reinigen hängt
    // oder einfach an der Zeit.
    expect(lauf(true, 12_000).geheilt).toBe(0);
  });

  it('und länger warten kostet mehr — der Preis wächst mit der Zeit', () => {
    // **Der Grund für „je Schritt".** Ein einmaliges Zurückdrehen wäre eine
    // Strafe für das Bekommen, nicht für das Nicht-Klären: Wer sofort reinigt,
    // zahlte dasselbe wie wer es laufen lässt. Dieser Fall hält fest, dass es
    // sich anders verhält.
    expect(lauf(false, 20_000).geheilt).toBeGreaterThan(lauf(false, 12_000).geheilt);
  });
});

describe('der alte Auslöser bleibt, wo er war', () => {
  it('eine Schwächung ALLEIN dreht nichts zurück', () => {
    // **Die Gegenprobe zum neuen Auslöser.** `boss-2-1` trägt dieselbe
    // Schwächung wie `test-lapse`, aber keine Boss-Heilung. Bliebe hier etwas
    // hängen, käme das Zurückdrehen von der Schwächung selbst statt vom Feld —
    // und der neue Auslöser wäre unbedingt statt geschaltet.
    const b = boss('boss-2-1');
    expect(b.affliction, 'ohne Schwächung prüft der Fall nichts').not.toBeNull();
    expect(b.bossHeal, 'dieser Boss hat inzwischen eine Heilung — Vorlage wechseln').toBeNull();

    const c = new Combat({ school: CALCINATION.value, boss: b });
    for (let t = 0; t <= 20_000; t += 10) {
      c.stepPassive(t);
      if (c.gcd.isReady(t) && c.isOffCooldown(c.indexOf('kindle'), t))
        c.cast(c.indexOf('kindle'), t);
    }
    expect(c.afflicted, 'die Schwächung stand nie — der Fall misst nichts').toBe(true);
    expect(c.bossHealed).toBe(0);
  });

  it('und `boss-2-4` trägt weiterhin den voreingestellten Auslöser', () => {
    // Der Boss, der Baustein 5 heute im Spiel trägt. Wäre er beim Umbau
    // versehentlich auf `affliction` gefallen, änderte sich sein Kampf, ohne
    // dass es jemand geschrieben hat.
    //
    // **Warum hier keine Rotation läuft:** QUENCH klärt die Schwächung UND
    // unterbricht. Einen Lauf zu bauen, der unterbricht ohne zu reinigen, geht
    // mit diesem Zauberkasten nicht — der erste Entwurf dieses Falls scheiterte
    // genau daran und meldete „die Schwächung stand nie".
    expect(boss('boss-2-4').bossHeal?.on).toBe('interrupt');
  });
});

describe('das FELD schaltet — nicht der Aufruf allein', () => {
  it('zwei Bosse, die sich NUR im Feld unterscheiden, verhalten sich verschieden', () => {
    // **Der Kernsatz dieses Issues, als eigener Fall.** Die erste Fassung
    // beschrieb ihn im Kommentar („beide Läufe") und fuhr nur einen — sie
    // belegte damit, dass der Auslöser WIRKT, nicht dass er SCHALTET.
    //
    // **Und der naheliegende zweite Boss taugte nicht.** `boss-2-4` trägt
    // ebenfalls `interrupt` und eine Schwächung, unterscheidet sich aber in
    // allem übrigen. Gemessen:
    //
    //     bis 12000 ms   geheilt 0, afflicted FALSE   (seine Schwächung
    //                    beginnt erst bei 20000 ms — eine Null ohne Lage)
    //     bis 32000 ms   geheilt 1600, missed 2       (sein ALTER Auslöser
    //                    greift, weil ohne QUENCH Zauber durchkommen)
    //
    // Ein Vergleich mit ihm misst den Boss, nicht das Feld. Deshalb der
    // Zwilling: identisch bis auf `on`, mit leerer Zeitleiste, damit der alte
    // Auslöser gar nicht greifen kann.
    const mit = boss('test-lapse');
    const ohne = boss('test-lapse-interrupt');

    expect(mit.bossHeal?.on).toBe('affliction');
    expect(ohne.bossHeal?.on).toBe('interrupt');

    // Die Zusicherung, dass die beiden wirklich Zwillinge sind: Wäre etwas
    // anderes verschieden, verglichen die Läufe unten zwei Bosse statt eines
    // Feldes.
    expect(ohne.affliction).toEqual(mit.affliction);
    expect(ohne.bossHeal?.amount).toBe(mit.bossHeal?.amount);
    expect(ohne.hp).toBe(mit.hp);
    expect(ohne.timeline, 'mit Zeitleiste könnte der ALTE Auslöser greifen').toEqual([]);

    const fahren = (b: BossDef): { geheilt: number; stand: boolean; verpasst: number } => {
      const c = new Combat({ school: CALCINATION.value, boss: b });
      for (let t = 0; t <= 12_000; t += 10) {
        c.stepPassive(t);
        if (c.gcd.isReady(t) && c.isOffCooldown(c.indexOf('kindle'), t))
          c.cast(c.indexOf('kindle'), t);
      }
      return { geheilt: c.bossHealed, stand: c.afflicted, verpasst: c.interruptsMissed };
    };

    const a = fahren(mit);
    const b = fahren(ohne);

    // Beide müssen dieselbe Lage haben — sonst vergleicht der Fall Äpfel mit
    // einem leeren Korb.
    expect(a.stand, 'die Schwächung stand nie — der Fall misst nichts').toBe(true);
    expect(b.stand, 'beim Zwilling stand sie nicht — die Lage ist verschieden').toBe(true);
    expect(b.verpasst, 'ein Zauber kam durch — dann greift der ALTE Auslöser').toBe(0);

    expect(a.geheilt, 'mit `on: affliction` wird nicht zurückgedreht').toBeGreaterThan(0);
    expect(b.geheilt, 'ohne `on: affliction` wird trotzdem zurückgedreht').toBe(0);
  });
});
