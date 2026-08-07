/**
 * Jede Signatur der zweiten Instanz ändert den Ausgang (Issue #574, AC2/AC6).
 *
 * # Der Unterschied zu Instanz 1
 *
 * Dort genügte „ohne die Signatur verläuft der Kampf messbar anders". Hier
 * verlangt AC2 mehr: **ohne die Signatur verliert man.** Ein Baustein, der
 * nur kostet, erfüllt das nicht.
 *
 * Genau diese Unterscheidung hat `boss-2-1` in seinem ersten Entwurf
 * verfehlt: Das Gift kostete 2088 Lebenspunkte und ließ den Siegzeitpunkt
 * unverändert — es wirkte messbar und entschied nichts. Erst ab 60 Schaden je
 * Tick kippt der Ausgang; die Datei trägt die Reihe im Kopfkommentar.
 */
import { describe, expect, it } from 'vitest';
import { Combat, FIGHT_WON, FIGHT_DIED } from '../../src/core/combat.js';
import { BOSSES, CALCINATION } from '../../src/core/data.js';
import type { BossDef } from '../../src/core/data.js';

function boss(id: string): BossDef {
  const eintrag = BOSSES.get(id);
  expect(eintrag, `${id} steht nicht in BOSSES`).toBeDefined();
  return eintrag!.value;
}

/**
 * Fährt einen Kampf bis zum Ende und gibt zurück, WIE er endete.
 *
 * **`checkDefeat` statt `damageDealt >= hp`.** Letzteres beantwortet nicht,
 * wer gewonnen hat: Bei einem Boss mit umgekehrtem Ziel ist der volle Ring
 * die Niederlage, und ein toter Spieler wirkt ohne diesen Aufruf weiter
 * (#581).
 */
function ausgang(b: BossDef, reinigen: boolean): { zustand: number; ende: number; hp: number } {
  const c = new Combat({ school: CALCINATION.value, boss: b });
  const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
  for (let t = 0; t <= 60_000; t += 10) {
    c.stepPassive(t);
    if (c.checkDefeat(t)) return { zustand: c.fightState, ende: t, hp: c.playerHp };
    if (!c.gcd.isReady(t)) continue;
    if (reinigen && c.afflicted && c.isOffCooldown(i('quench'), t)) c.cast(i('quench'), t);
    else if (!c.isDotActive(i('smoulder'), t) && c.isOffCooldown(i('smoulder'), t))
      c.cast(i('smoulder'), t);
    else if (c.resource >= 15 && c.isOffCooldown(i('pyre'), t)) c.cast(i('pyre'), t);
    else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
  }
  return { zustand: c.fightState, ende: -1, hp: c.playerHp };
}

describe('boss-2-1 — ohne Reinigung verliert man', () => {
  it('WER NICHT REINIGT, STIRBT — der wichtigere der beiden Fälle', () => {
    const r = ausgang(boss('boss-2-1'), false);
    expect(r.zustand, 'ohne Reinigung wird der Kampf gewonnen — die Signatur fordert nichts').toBe(
      FIGHT_DIED,
    );
  });

  it('wer reinigt, gewinnt — und lebt dabei', () => {
    const r = ausgang(boss('boss-2-1'), true);
    expect(r.zustand).toBe(FIGHT_WON);
    expect(r.hp, 'der Sieger ist tot').toBeGreaterThan(0);
    // AC2 verlangt Restzeit unter 40 %.
    expect((60_000 - r.ende) / 60_000).toBeLessThan(0.4);
  });

  it('und das Gift ist scharf genug, dass die Reinigung ENTSCHEIDET', () => {
    // **Die Gegenprobe zur Schwelle.** Bei 30 Schaden je Tick gewinnt man
    // auch ohne Reinigung — dann kostet der Baustein nur. Dieser Fall hält
    // fest, dass die eingestellte Schärfe über der Schwelle liegt.
    const b = boss('boss-2-1');
    const schwach = { ...b, affliction: { ...b.affliction!, damage: 30 } };
    expect(ausgang(schwach, false).zustand, 'bei 30 sollte man überleben').toBe(FIGHT_WON);
    expect(
      b.affliction!.damage,
      'das Gift ist wieder unter die Schwelle gerutscht',
    ).toBeGreaterThan(50);
  });
});

describe('boss-2-3 — das umgekehrte Ziel: wer nicht stabilisiert, verliert', () => {
  /**
   * Der erste Kampf im Spiel, den **Koagulation entscheidet**.
   *
   * Beide Ausgänge sind umgekehrt (GAME-DESIGN 11.2, Baustein 6): Ein voller
   * Ring ist die Niederlage, der Sieg ist das Überstehen der Frist.
   *
   * **Der Spieler treibt den Ring mit.** `bookProgress` erhöht denselben
   * Zähler für Spieler und Boss — eine reine Schadensrotation verliert
   * deshalb früher als eine sparsame (30040 ms gegen 37000 ms). Zurückdrängen
   * geht nur über `stabilise()`, das Schild und Heilung aufrufen.
   *
   * Ein erster Entwurf nannte als Signatur „wer nicht schneller ist,
   * verliert". Das war genau verkehrt.
   */
  function ausgangRing(art: 'ohne' | 'crust'): { zustand: number; ring: number; stab: number } {
    const b = boss('boss-2-3');
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
    let hoechster = 0;
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      hoechster = Math.max(hoechster, c.damageDealt / b.hp);
      if (c.checkDefeat(t)) return { zustand: c.fightState, ring: hoechster, stab: c.stabilised };
      if (!c.gcd.isReady(t)) continue;
      if (art === 'crust' && c.shield <= 0 && c.isOffCooldown(i('crust'), t)) c.cast(i('crust'), t);
      else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
    }
    return { zustand: c.fightState, ring: hoechster, stab: c.stabilised };
  }

  it('WER NICHT STABILISIERT, VERLIERT — die Reaktion kippt', () => {
    const r = ausgangRing('ohne');
    expect(r.stab, 'ohne Koagulation darf nichts stabilisiert werden').toBe(0);
    expect(r.zustand, 'ohne Stabilisierung wird der Kampf gewonnen').not.toBe(FIGHT_WON);
    expect(r.ring, 'der Ring läuft nicht voll — dann fordert die Signatur nichts').toBeGreaterThan(
      1,
    );
  });

  it('wer die Schicht legt, hält den Ring und übersteht die Frist', () => {
    const r = ausgangRing('crust');
    expect(r.stab, 'CRUST stabilisiert nicht — dann prüft der Fall nichts').toBeGreaterThan(0);
    expect(r.zustand).toBe(FIGHT_WON);
    // **Die eigentliche Aussage:** Der Ring bleibt unter 100 %. Ein Sieg mit
    // vollem Ring wäre keiner — bei diesem Boss ist das die Niederlage.
    expect(r.ring, 'der Ring lief voll und der Kampf gilt trotzdem als gewonnen').toBeLessThan(1);
  });
});

describe('boss-2-4 — die Kollision: wer nicht unterbricht, kommt nie durch', () => {
  /**
   * Der Abschluss der Instanz. Rennen und Wirkzauber überlappen (12000–17000
   * gegen 13000) — wer im Fenster unterbricht, verfehlt die Schwelle; wer die
   * Schwelle schafft, lässt den Zauber durch.
   *
   * Der Unterschied zu `boss-1-4`: Dort lagen beide auseinander und waren
   * einzeln zu bedienen.
   */
  function ausgang24(unterbrechen: boolean): { zustand: number; ende: number; geheilt: number } {
    const b = boss('boss-2-4');
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      if (c.checkDefeat(t)) return { zustand: c.fightState, ende: t, geheilt: c.bossHealed };
      if (!c.gcd.isReady(t)) continue;
      if (unterbrechen && c.casting !== -1 && c.isOffCooldown(i('quench'), t))
        c.cast(i('quench'), t);
      else if (!c.isDotActive(i('smoulder'), t) && c.isOffCooldown(i('smoulder'), t))
        c.cast(i('smoulder'), t);
      else if (c.resource >= 15 && c.isOffCooldown(i('pyre'), t)) c.cast(i('pyre'), t);
      else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
    }
    return { zustand: c.fightState, ende: -1, geheilt: c.bossHealed };
  }

  it('WER NICHT UNTERBRICHT, KOMMT NIE DURCH — der Boss heilt sich zurück', () => {
    const r = ausgang24(false);
    expect(r.geheilt, 'der Boss heilt nicht — dann fordert die Signatur nichts').toBeGreaterThan(
      1000,
    );
    expect(r.zustand).not.toBe(FIGHT_WON);
  });

  it('wer unterbricht, verhindert die Heilung ganz und gewinnt', () => {
    const r = ausgang24(true);
    expect(r.geheilt, 'trotz Unterbrechung heilt der Boss').toBe(0);
    expect(r.zustand).toBe(FIGHT_WON);
    expect((60_000 - r.ende) / 60_000, 'Restzeit über 40 % — zu leicht').toBeLessThan(0.4);
  });
});

describe('boss-2-2 — der Abschluss: die REIHENFOLGE entscheidet', () => {
  /**
   * Die drei anderen Bosse der Instanz verlangen je **eine** Handlung. Dieser
   * verlangt eine **Reihenfolge**: Ab 8 s zehrt ein Gift, ab 10 s läuft ein
   * Fenster, und ab 11 s beginnt ein Zauber mit 2200 Schaden.
   *
   * QUENCH bedient beides — er klärt das Gift **und** unterbricht. Bei fünf
   * Sekunden Wartezeit reicht er nicht für beides.
   *
   * **Der Preis ist die Wartezeit, nicht der GCD.** Ein früherer Entwurf
   * wollte die Wahl stellen, *wen* man unterbricht; das scheiterte, weil
   * QUENCH `triggersGcd: false` trägt und blindes Drücken damit gratis ist.
   * Ohne Preis für die falsche Antwort gibt es keine Wahl.
   */
  function reihenfolge(art: 'nichts' | 'gift' | 'zauber'): { zustand: number; hp: number } {
    const b = boss('boss-2-2');
    const c = new Combat({ school: CALCINATION.value, boss: b });
    const i = (id: string): ReturnType<typeof c.indexOf> => c.indexOf(id);
    for (let t = 0; t <= 60_000; t += 10) {
      c.stepPassive(t);
      if (c.checkDefeat(t)) return { zustand: c.fightState, hp: c.playerHp };
      if (art === 'gift' && c.afflicted && c.isOffCooldown(i('quench'), t)) {
        c.cast(i('quench'), t);
        continue;
      }
      if (art === 'zauber' && c.casting !== -1 && c.isOffCooldown(i('quench'), t)) {
        c.cast(i('quench'), t);
        continue;
      }
      if (!c.gcd.isReady(t)) continue;
      if (!c.isDotActive(i('smoulder'), t) && c.isOffCooldown(i('smoulder'), t))
        c.cast(i('smoulder'), t);
      else if (c.resource >= 15 && c.isOffCooldown(i('pyre'), t)) c.cast(i('pyre'), t);
      else if (c.isOffCooldown(i('kindle'), t)) c.cast(i('kindle'), t);
    }
    return { zustand: c.fightState, hp: c.playerHp };
  }

  it('wer QUENCH gar nicht nutzt, stirbt', () => {
    expect(reihenfolge('nichts').zustand).toBe(FIGHT_DIED);
  });

  it('DIE FALSCHE REIHENFOLGE TÖTET — zuerst reinigen heißt, den Zauber durchzulassen', () => {
    // Der eigentliche Nachweis. Ohne ihn wäre es ein Boss mit zwei
    // Anforderungen, nicht einer mit einer Entscheidung.
    expect(reihenfolge('gift').zustand).toBe(FIGHT_DIED);
  });

  it('die richtige gewinnt — erst unterbrechen, das Gift danach', () => {
    const r = reihenfolge('zauber');
    expect(r.zustand).toBe(FIGHT_WON);
    expect(r.hp, 'der Sieger ist tot').toBeGreaterThan(0);
  });
});
