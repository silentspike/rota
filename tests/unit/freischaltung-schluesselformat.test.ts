/**
 * Die Freischaltung liest die Liste, die das Spiel wirklich führt
 * (Fehlerbehebung 2026-08-11, Issue #421c).
 *
 * # Der Fehler, den dieser Fall verhindert
 *
 * `defeatedBosses` führt Schlüssel aus Kennung **und** Stufe, gebildet über
 * `markKey` — bewusst so, weil ein Boss auf mythisch nicht derselbe Kampf ist
 * wie auf normal. `freigeschalteteInstanzen` fragte darin nach der **reinen**
 * Kennung:
 *
 * ```text
 *   geschrieben   markKey('boss-1-1', 1)
 *   gesucht       gefallen.has('boss-1-1')
 * ```
 *
 * Die beiden Mengen berührten sich nie. `abgeschlossen()` lieferte immer
 * `false`, und **keine Instanz mit `requires` wurde je freigeschaltet** —
 * instance-2 und instance-3 waren für jeden Spieler unerreichbar, neun von
 * dreizehn Bossen nicht spielbar.
 *
 * # Warum die bestehenden Prüffälle grün blieben
 *
 * `instanz-3.test.ts` übergibt **reine Kennungen**. Die Funktion arbeitet
 * damit korrekt; sie bekommt im Spiel nur nie solche Werte.
 *
 * > Dieselbe Klasse wie #421c selbst: Damals war `freigeschalteteInstanzen`
 * > gebaut und von niemandem gerufen. Diesmal wird sie gerufen und bekommt das
 * > falsche Format. Beide Male prüfte der Fall die FUNKTION statt den WEG.
 *
 * **Dieser Fall benutzt deshalb `markKey` selbst**, statt die Zeichenkette
 * nachzubauen. Ein Nachbau antwortete aus einer zweiten Quelle: Ändert jemand
 * das Trennzeichen, zöge er mit und bliebe grün, während das Spiel bricht.
 */
import { describe, expect, it } from 'vitest';

import { BOSSES } from '../../src/core/data.js';
import { markKey } from '../../src/analysis/progress-mark.js';
import { freigeschalteteInstanzen, waehlbareBosse } from '../../src/progression/boss-select.js';

/** Die vier Bosse einer Instanz, so besiegt wie das Spiel es aufschreibt. */
function besiegtWieImSpiel(ids: readonly string[], schwierigkeit = 1): string[] {
  return ids.map((id) => markKey(id, schwierigkeit));
}

const INSTANCE_1 = ['boss-1-1', 'boss-1-2', 'boss-1-3', 'boss-1-4'];
const INSTANCE_2 = ['boss-2-1', 'boss-2-2', 'boss-2-3', 'boss-2-4'];

describe('die Zusicherung: das Format ist wirklich zusammengesetzt', () => {
  it('markKey liefert nicht die blanke Kennung', () => {
    // **Ohne diese Zeile prüfte alles darunter nichts.** Wäre `markKey` eines
    // Tages die Identität, wären beide Formate gleich, und der Fall bliebe
    // grün — während er nichts mehr unterscheidet.
    expect(markKey('boss-1-1', 1)).not.toBe('boss-1-1');
    expect(markKey('boss-1-1', 1).startsWith('boss-1-1')).toBe(true);
  });
});

describe('vier besiegte Bosse öffnen die nächste Instanz', () => {
  it('mit den Schlüsseln, die das Spiel schreibt', () => {
    // **Der Fall, der gefehlt hat.** Vorher stand hier sinngemäß
    // `freigeschalteteInstanzen(['boss-1-1', …])` — reine Kennungen, die im
    // Spiel nie vorkommen.
    const offen = freigeschalteteInstanzen(besiegtWieImSpiel(INSTANCE_1), BOSSES);
    expect(offen, 'instance-2 bleibt gesperrt, obwohl instance-1 durch ist').toContain(
      'instance-2',
    );
  });

  it('und instance-3 erst nach instance-2', () => {
    const nurEins = freigeschalteteInstanzen(besiegtWieImSpiel(INSTANCE_1), BOSSES);
    expect(nurEins, 'instance-3 öffnet zu früh').not.toContain('instance-3');

    const beide = freigeschalteteInstanzen(
      besiegtWieImSpiel([...INSTANCE_1, ...INSTANCE_2]),
      BOSSES,
    );
    expect(beide).toContain('instance-3');
  });

  it('auch auf einer höheren Schwierigkeit — der Sieg zählt, nicht die Stufe', () => {
    // Wer mythisch bezwingt, hat erst recht bezwungen. Eine Freischaltung, die
    // ausgerechnet den schwereren Sieg nicht zählt, wäre die Umkehrung dessen,
    // was sie belohnen soll.
    const offen = freigeschalteteInstanzen(besiegtWieImSpiel(INSTANCE_1, 3), BOSSES);
    expect(offen).toContain('instance-2');
  });

  it('GEGENPROBE: drei von vier reichen nicht', () => {
    // Ohne sie wäre auch eine Funktion grün, die jede Instanz sofort öffnet.
    const offen = freigeschalteteInstanzen(besiegtWieImSpiel(INSTANCE_1.slice(0, 3)), BOSSES);
    expect(offen, 'instance-2 öffnet schon bei drei Bossen').not.toContain('instance-2');
  });
});

describe('und die Bosse erscheinen wirklich in der Auswahl', () => {
  it('waehlbareBosse zeigt instance-2 nach vier Siegen', () => {
    // **Der Weg bis zur Oberfläche.** `freigeschalteteInstanzen` allein sagt
    // nur, welche Instanz offen ist; der Spieler sieht die Liste aus
    // `waehlbareBosse`. Genau zwischen diesen beiden lag der Fehler von #421c.
    const ids = waehlbareBosse(BOSSES, besiegtWieImSpiel(INSTANCE_1)).map((b) => b.id);
    for (const id of INSTANCE_2) {
      expect(ids, `${id} fehlt in der Auswahl`).toContain(id);
    }
  });

  it('und ohne Siege nur den Prolog und instance-1', () => {
    const ids = waehlbareBosse(BOSSES, []).map((b) => b.id);
    expect(ids).toContain('first-crucible');
    expect(ids).toContain('boss-1-1');
    expect(ids, 'instance-2 ist ohne Siege sichtbar').not.toContain('boss-2-1');
  });
});
