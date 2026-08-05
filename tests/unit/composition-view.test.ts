/**
 * Die Gruppierung und die Abwesenheit jeder Empfehlung (Issue #270, AC1 und AC5).
 *
 * AC5 IST DIE SCHWERE
 * -------------------
 * „Keine Empfehlung" laesst sich nicht dadurch belegen, dass man sie nicht
 * gebaut hat — „ich habe es nicht getan" ist kein Nachweis. Die Faelle unten
 * zeigen deshalb eine WIRKUNG: Ein Zauber wird zehnmal staerker, und es
 * aendert sich nichts. Eine Sortierung nach Staerke, eine Markierung „bester
 * Zauber" oder eine Warnung muessten darauf reagieren.
 *
 * Die Gegenprobe steht daneben (`sortiertNachSchaden`): Sie zeigt, dass der
 * Vergleich ueberhaupt etwas sieht. Ohne sie waere die Zusicherung gruen, auch
 * wenn sie nichts messen koennte.
 *
 * GAME-DESIGN sagt zweimal, dass das Zusammenstellen eine der beiden
 * Kernaufgaben ist. Eine Sortierung nach „bester Zauber" nimmt sie weg — und
 * sie entsteht leicht, weil sie hilfreich wirkt.
 */
import { describe, expect, it } from 'vitest';
import { CALCINATION } from '../../src/core/data.js';
import { groupBySchool } from '../../src/progression/composition-view.js';
import { NO_SCHOOL_LABEL } from '../../src/progression/schools.js';

const SPELLS = CALCINATION.value.spells;

/** Wie die Ansicht die Zauber ordnet, als flache Folge von Kennungen. */
function folge(spells: readonly (typeof SPELLS)[number][]): string[] {
  return groupBySchool(spells).flatMap((g) => g.spells.map((s) => s.id));
}

describe('AC1 — nach Schule gruppiert', () => {
  it('legt jede Schule in eine eigene Gruppe', () => {
    const gruppen = groupBySchool(SPELLS);
    const namen = gruppen.map((g) => g.label);
    // Die Reihenfolge ist die des ersten Auftretens in den Daten — keine
    // Wertung, keine Alphabetik.
    expect(namen).toEqual([
      'CALCINATION',
      NO_SCHOOL_LABEL,
      'COAGULATION',
      'FIXATION',
      'SUBLIMATION',
      'PUTREFACTION',
    ]);
  });

  it('verliert keinen Zauber und erfindet keinen', () => {
    // Ohne diesen Fall koennte die Gruppierung einen Zauber unterschlagen,
    // und die Gruppen saehen trotzdem vollstaendig aus.
    expect(folge(SPELLS).sort()).toEqual(SPELLS.map((s) => s.id).sort());
  });

  it('gibt jeder Schule ihre Farbe und der schulunabhaengigen keine', () => {
    const gruppen = groupBySchool(SPELLS);
    for (const gruppe of gruppen) {
      if (gruppe.school === null) {
        expect(gruppe.colour, 'keine Schule, keine Farbe').toBeNull();
        expect(gruppe.label).toBe(NO_SCHOOL_LABEL);
      } else {
        expect(gruppe.colour, gruppe.label).toMatch(/^#[0-9a-f]{6}$/);
      }
    }
  });

  it('haelt QUENCH aus den Schulen heraus', () => {
    const ohne = groupBySchool(SPELLS).find((g) => g.school === null);
    expect(ohne?.spells.map((s) => s.id)).toEqual(['quench']);
  });
});

describe('AC5 — keine Empfehlung', () => {
  it('AENDERT SICH NICHT, wenn ein Zauber zehnmal staerker wird', () => {
    // **Der Wirkungsnachweis.** SMOULDER steht in den Daten bei 0
    // Grundschaden und ganz hinten in seiner Gruppe. Wird er der staerkste
    // Zauber des Spiels, muss er stehen bleiben, wo er ist.
    const vorher = folge(SPELLS);
    const verstaerkt = SPELLS.map((s) => (s.id === 'smoulder' ? { ...s, baseDamage: 10_000 } : s));
    expect(folge(verstaerkt), 'die Reihenfolge folgt den Daten, nicht der Staerke').toEqual(vorher);

    // Und auch die Gruppen selbst bleiben, wo sie sind.
    expect(groupBySchool(verstaerkt).map((g) => g.label)).toEqual(
      groupBySchool(SPELLS).map((g) => g.label),
    );
  });

  it('bleibt gleich, wenn der SCHWAECHSTE staerker wird als alle', () => {
    // Zwei Richtungen, weil eine Sortierung auch aufsteigend sein koennte.
    const vorher = folge(SPELLS);
    const kindleSchwach = SPELLS.map((s) => (s.id === 'kindle' ? { ...s, baseDamage: 1 } : s));
    expect(folge(kindleSchwach)).toEqual(vorher);
  });

  it('DIE GEGENPROBE: eine Sortierung nach Schaden waere sichtbar', () => {
    // Ohne diesen Fall wuesste niemand, ob die beiden oben ueberhaupt etwas
    // sehen koennen. Er baut die verbotene Sortierung nach und zeigt, dass
    // sie eine ANDERE Folge ergibt — die Zusicherungen oben haetten also
    // angeschlagen.
    const sortiertNachSchaden = [...SPELLS].sort((a, b) => b.baseDamage - a.baseDamage);
    expect(sortiertNachSchaden.map((s) => s.id)).not.toEqual(SPELLS.map((s) => s.id));

    const verstaerkt = SPELLS.map((s) => (s.id === 'smoulder' ? { ...s, baseDamage: 10_000 } : s));
    const danachSortiert = [...verstaerkt].sort((a, b) => b.baseDamage - a.baseDamage);
    expect(
      danachSortiert.map((s) => s.id),
      'mit Sortierung waere SMOULDER nach vorn gesprungen',
    ).not.toEqual(folge(verstaerkt));
  });
});

/**
 * Der Wort-Waechter — ERGAENZUNG, nicht Ersatz.
 *
 * Er prueft Text statt Verhalten und kann deshalb weniger als die Faelle
 * oben: Eine Sortierung nach Staerke wuerde er nicht sehen. Was er faengt,
 * ist die Warnung oder Auszeichnung, die jemand SPAETER einbaut — sie kaeme
 * mit einem dieser Woerter.
 *
 * Zwei Waechter mit verschiedenen Angriffspunkten sind besser als einer,
 * solange klar ist, welcher was kann. Das steht hier, damit niemand ihn fuer
 * mehr haelt.
 */
const QUELLE = import.meta.glob('/src/progression/composition-view.ts', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

describe('AC5 — auch der Wortlaut empfiehlt nichts', () => {
  it('findet den Quelltext ueberhaupt', () => {
    // Ohne diese Zusicherung waere der Fall unten gruen, weil er nichts
    // gelesen hat.
    expect(Object.values(QUELLE)[0]?.length ?? 0).toBeGreaterThan(1000);
  });

  it('traegt keine wertenden Woerter in den Spieltexten', () => {
    const text = Object.values(QUELLE)[0] ?? '';
    // Nur die Zeichenketten, die im Spiel erscheinen — Kommentare bleiben
    // aussen vor, sonst schluege die Begruendung oben selbst an. Das ist die
    // Falle, in die ein Waechter leicht tappt, der seinen eigenen Text liest.
    const spieltexte = [...text.matchAll(/'([^']*)'|`([^`]*)`/g)]
      .map((m) => m[1] ?? m[2] ?? '')
      .join(' ')
      .toLowerCase();
    for (const wort of ['best', 'recommend', 'optimal', 'weak', 'should', 'better']) {
      expect(spieltexte, `"${wort}" waere eine Empfehlung`).not.toContain(wort);
    }
  });

  it('SIEHT eines, wenn es dasteht — die Gegenprobe', () => {
    const erfunden = "const hinweis = 'best spell for this slot';";
    const spieltexte = [...erfunden.matchAll(/'([^']*)'/g)].map((m) => m[1] ?? '').join(' ');
    expect(spieltexte.toLowerCase()).toContain('best');
  });
});
