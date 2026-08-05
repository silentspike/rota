/**
 * Jede gemessene Groesse ist gekennzeichnet (Issue #294, AC2).
 *
 * DIE LEHRE AUS DEM BEFUND SELBST
 * -------------------------------
 * Dreizehn Groessen hatten zusammen achtzig Prueffaelle und keinen einzigen
 * Leser im Spiel. Sie waren nicht ungeprueft — sie waren sorgfaeltig geprueft
 * und vollstaendig ungenutzt. Genau deshalb ist es niemandem aufgefallen: Eine
 * ungeprueffte Groesse faellt auf, eine gepruefte und ungenutzte nicht.
 *
 * Sechs erscheinen jetzt. Die anderen sieben sollen NICHT erscheinen, und das
 * muss dastehen — sonst stehen sie beim naechsten systematischen Durchgang
 * wieder auf derselben Liste, und jemand misst dieselben achtzig Prueffaelle
 * ein zweites Mal.
 *
 * WAS DIESER WAECHTER NICHT KANN: Er prueft, dass eine Kennzeichnung
 * DASTEHT, nicht ob sie stimmt. Ob „Damage taken" wirklich erscheint, messen
 * die Browser-Nachweise.
 */
import { describe, expect, it } from 'vitest';

const QUELLE = import.meta.glob('/src/core/combat.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function combatQuelle(): string {
  const text = Object.values(QUELLE)[0];
  if (text === undefined || text.length === 0) {
    throw new Error('combat.ts nicht geladen — die Pruefung wuerde nichts messen');
  }
  return text;
}

/** Die dreizehn, die am 2026-08-05 ohne Leser im Spiel dastanden. */
const GEMESSEN_OHNE_LESER: readonly string[] = [
  'damageTaken',
  'stabilised',
  'shieldAbsorbed',
  'converted',
  'interruptsLanded',
  'controlFailed',
  'critFactorsMirrorBase',
  'cycle',
  'bossEnergyTriggers',
  'raceWon',
  'bossHpPercent',
  'playerDead',
  'diedAtMs',
];

/** Der Kommentarblock ueber einem Zugang. */
function kommentarVor(quelle: string, name: string): string {
  const zeilen = quelle.split('\n');
  const index = zeilen.findIndex((z) => new RegExp(`^  get ${name}\\(`).test(z));
  if (index < 0) return '';
  const gesammelt: string[] = [];
  for (let i = index - 1; i >= 0 && i > index - 14; i--) {
    const zeile = (zeilen[i] ?? '').trim();
    if (zeile.startsWith('*') || zeile.startsWith('/**') || zeile.startsWith('*/')) {
      gesammelt.unshift(zeile);
      continue;
    }
    break;
  }
  return gesammelt.join(' ');
}

describe('AC2 — jede der dreizehn sagt, ob sie erscheint', () => {
  it('findet den Quelltext ueberhaupt', () => {
    expect(combatQuelle().length).toBeGreaterThan(1000);
  });

  it('kennzeichnet JEDE einzelne', () => {
    const quelle = combatQuelle();
    const ohne: string[] = [];
    for (const name of GEMESSEN_OHNE_LESER) {
      const kommentar = kommentarVor(quelle, name);
      if (!/ANGEZEIGT|PRUEF-ZUGANG/.test(kommentar)) ohne.push(name);
    }
    expect(ohne, 'ohne Kennzeichnung — beim naechsten Durchgang wieder ein Fund').toEqual([]);
  });

  it('nennt bei jeder GENAU EINEN der beiden Zustaende', () => {
    // „angezeigt UND Pruef-Zugang" waere keine Aussage, sondern zwei.
    const quelle = combatQuelle();
    for (const name of GEMESSEN_OHNE_LESER) {
      const kommentar = kommentarVor(quelle, name);
      const angezeigt = /ANGEZEIGT/.test(kommentar);
      const pruef = /PRUEF-ZUGANG/.test(kommentar);
      expect(angezeigt !== pruef, `${name}: angezeigt=${angezeigt} pruef=${pruef}`).toBe(true);
    }
  });

  it('SIEHT eine fehlende Kennzeichnung — die Gegenprobe', () => {
    // Ohne sie waere nicht gezeigt, dass die Suche etwas findet.
    const erfunden = ['  /** Irgendein Wert. */', '  get irgendwas(): number {'].join('\n');
    expect(/ANGEZEIGT|PRUEF-ZUGANG/.test(kommentarVor(erfunden, 'irgendwas'))).toBe(false);
  });
});
