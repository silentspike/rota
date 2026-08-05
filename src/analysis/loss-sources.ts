/**
 * Die fuenf Verlustquellen (GAME-DESIGN 15.2, Issue #81).
 *
 * WARUM DIESE DATEI SO GEBAUT IST
 * ------------------------------
 * Zwei der fuenf Quellen sind heute nicht rechenbar: Das Kampfprotokoll weist
 * im SPIEL keine Boss-Gelegenheiten aus (Issue #151), und der Zaehler fuer
 * "leeres Fenster ohne bereiten Zauber" fehlt ebenfalls.
 *
 * Sie werden deshalb **eingehaengt, nicht eingebaut**: Jede Quelle liefert
 * ihre Zahl ueber eine Funktion, die von aussen kommt. Fehlt die Datengrundlage,
 * bleibt die Quelle einfach weg — und wenn sie da ist, ist der Anschluss eine
 * Zeile statt eines Umbaus.
 *
 * Das Vorbild ist sublimation.rs aus Issue #78: Die Verkuerzungs-Betraege kamen
 * als Tabelle vom Aufrufer, damit das Modul fertig dastand, waehrend die
 * Datenfrage noch offen war.
 *
 * WAS HIER NIEMALS PASSIERT
 * -------------------------
 * Eine Reihenfolge vorrechnen. Die Analyse benennt, was liegen blieb — nie,
 * was man stattdessen haette tun sollen (GAME-DESIGN 21.2). Ein Prueffall
 * wacht ueber die Wortwahl.
 */

/** Die fuenf Arten, in der Reihenfolge des Designs. */
export type LossKind =
  'missedCasts' | 'idle' | 'missedSynergies' | 'lostDotTicks' | 'missedInterrupts';

/** Was eine Quelle beitraegt. `null` heisst: heute nicht rechenbar. */
export interface LossContribution {
  /** Liegengelassener Schaden. */
  readonly damage: number;
  /** Ein Satz fuer den Spieler, englisch (Design-Gesetz 5). */
  readonly detail: string;
  /**
   * Wie oft es vorkam, wo die Anzahl selbst eine Aussage ist.
   *
   * Heute nur bei den Unterbrechungen: Ihre Zeile nennt die Anzahl, weil sie
   * keinen Prozentsatz bekommt (GAME-DESIGN 15.2).
   */
  readonly count?: number;
}

/**
 * Eine Quelle, wie sie eingehaengt wird.
 *
 * `compute` gibt `null` zurueck, wenn die Datengrundlage fehlt. Das ist kein
 * Fehler, sondern der ehrliche Zustand: Eine Quelle, die nicht gemessen werden
 * kann, wird nicht gezeigt — statt mit einer Null zu erscheinen, die wie
 * "kein Verlust" aussieht.
 */
export interface LossSourceDef {
  readonly kind: LossKind;
  /** Kurzname in der Anzeige, englisch. */
  readonly label: string;
  compute(): LossContribution | null;
}

export interface RankedLoss extends LossContribution {
  readonly kind: LossKind;
  readonly label: string;
  /** Anteil am Gesamtpotenzial, 0 bis 1. */
  readonly share: number;
}

/**
 * Verpasste Unterbrechungen — **getrennt ausgewiesen**.
 *
 * Entschieden vom Auftraggeber am 2026-08-04 (GAME-DESIGN 15.2): Sie bekommen
 * eine eigene Zahl, aber **nicht** dieselbe Prozentrechnung wie die vier
 * Schadensquellen.
 *
 * Die vier oberen messen NICHT AUSGETEILTEN Schaden. Diese misst
 * EINGESTECKTEN. Beides in einen Prozentsatz zu werfen vergleicht
 * Unvergleichbares — und die Zahl gewinnt immer: Ein Boss-Zauber übersteigt
 * den Spielerschaden je Wirkung leicht um das Zwanzigfache. Die Quelle stünde
 * damit fast immer oben, unabhängig von der Rotation, und der wichtigste
 * Hinweis handelte wieder vom Unterbrecher — genau das, was die
 * Gelegenheits-Regel verhindern sollte.
 *
 * Die Rechnung selbst ist unverändert: Anzahl × Schaden des durchgekommenen
 * Boss-Zaubers.
 */
export interface InterruptLoss {
  readonly missed: number;
  /** Schaden, den der Spieler GENOMMEN hat — nicht entgangener Schaden. */
  readonly damageTaken: number;
  /** Ein Satz für den Spieler, englisch (Design-Gesetz 5). */
  readonly detail: string;
}

/**
 * Die eine Art, die eine andere Größe misst als alle anderen.
 *
 * Sie steht hier als Konstante und nicht als Sonderfall im Rumpf, damit
 * sichtbar ist, WARUM sie anders behandelt wird: nicht weil sie besonders
 * ist, sondern weil ihre Einheit eine andere ist.
 */
const MEASURES_DAMAGE_TAKEN: LossKind = 'missedInterrupts';

/**
 * Was ein Boss an Fortschritt zurückgedreht hat (11.2, Baustein 5; #218).
 *
 * **Die zweite Größe, die nicht in die Prozentrechnung gehört** — aus einem
 * anderen Grund als die verpassten Unterbrechungen, aber mit derselben Folge:
 *
 * | | misst |
 * | --- | --- |
 * | die vier Quellen | Schaden, der **nicht ausgeteilt** wurde |
 * | verpasste Unterbrechungen | Schaden, den der Spieler **genommen** hat |
 * | Boss-Heilung | Schaden, der **rückgängig gemacht** wurde |
 *
 * Drei Einheiten, eine Zeile je Einheit. Sie in eine Prozentrechnung zu werfen
 * verglich Unvergleichbares — und die Prozentzahl wäre falsch, ohne dass man
 * es ihr ansieht.
 */
export interface BossHealLoss {
  /** Wie viel Fortschritt zurückgedreht wurde. */
  readonly amount: number;
  /** Ein Satz für den Spieler, englisch (Design-Gesetz 5). */
  readonly detail: string;
}

export interface LossReport {
  /** Alle rechenbaren Quellen mit Verlust ueber 0, groesste zuerst. */
  readonly sources: readonly RankedLoss[];
  /** Die drei groessten — sie werden hervorgehoben (GAME-DESIGN 15.2). */
  readonly top: readonly RankedLoss[];
  readonly totalLost: number;
  /** Erzielt plus verloren. Bezugsgroesse aller Prozente. */
  readonly potential: number;
  readonly totalShare: number;
  /**
   * Quellen, die heute nicht gemessen werden koennen.
   *
   * Sie stehen hier, damit die Luecke SICHTBAR ist. Eine fehlende Quelle
   * stillschweigend wegzulassen hiesse, dem Spieler eine vollstaendige
   * Rechnung vorzutaeuschen.
   */
  readonly unavailable: readonly LossKind[];
  /**
   * Verpasste Unterbrechungen, getrennt von der Prozentrechnung oben.
   *
   * `null` heißt: keine gemessen oder nicht messbar. Wo sie nicht messbar
   * sind, stehen sie zusätzlich in `unavailable`.
   */
  readonly interrupts: InterruptLoss | null;
  /**
   * Zurückgedrehter Fortschritt, getrennt von der Prozentrechnung oben.
   *
   * `null` heißt: Der Boss hat nichts zurückgedreht — entweder trägt er den
   * Baustein nicht, oder es kam kein Zauber durch.
   */
  readonly bossHeal: BossHealLoss | null;
}

/**
 * Die Quellen zusammenrechnen und ordnen.
 *
 * Der Bezug aller Prozente ist das Gesamtpotenzial dieses Durchlaufs — erzielter
 * Schaden plus Summe der Verluste. Nicht ein theoretisches Optimum: Ein solches
 * gibt es hier bewusst nicht, und die alte Rechnung "Kampfzeit durch
 * Abklingzeit" ist genau daran gescheitert (gemessen: 193 theoretische Maxima
 * gegen 160 verfuegbare Aktionen).
 */
export function rankLosses(
  damageDealt: number,
  defs: readonly LossSourceDef[],
  bossHealed = 0,
): LossReport {
  const computed: { def: LossSourceDef; value: LossContribution }[] = [];
  const unavailable: LossKind[] = [];
  let interrupts: InterruptLoss | null = null;

  for (const def of defs) {
    const value = def.compute();
    if (value === null) {
      unavailable.push(def.kind);
      continue;
    }
    // Andere Einheit, andere Zeile. Sie geht NICHT in totalLost, potential
    // oder totalShare ein -- sonst stuende sie in einem Prozentsatz, der
    // ausgeteilten Schaden meint.
    if (def.kind === MEASURES_DAMAGE_TAKEN) {
      if (value.damage > 0) {
        interrupts = {
          missed: value.count ?? 0,
          damageTaken: value.damage,
          detail: value.detail,
        };
      }
      continue;
    }
    computed.push({ def, value });
  }

  const totalLost = computed.reduce((sum, entry) => sum + entry.value.damage, 0);
  const potential = damageDealt + totalLost;
  const shareOf = (value: number): number => (potential > 0 ? value / potential : 0);

  const sources = computed
    .filter((entry) => entry.value.damage > 0)
    .map((entry): RankedLoss => ({
      kind: entry.def.kind,
      label: entry.def.label,
      damage: entry.value.damage,
      detail: entry.value.detail,
      share: shareOf(entry.value.damage),
    }))
    .sort((a, b) => b.damage - a.damage || a.kind.localeCompare(b.kind));

  return {
    sources,
    top: sources.slice(0, 3),
    totalLost,
    potential,
    totalShare: shareOf(totalLost),
    unavailable,
    interrupts,
    // **Nicht in `sources`, nicht in `totalLost`, nicht in `potential`.**
    // Dieselbe Trennung wie bei den Unterbrechungen und aus demselben Grund:
    // eine andere Einheit.
    bossHeal:
      bossHealed > 0
        ? {
            amount: bossHealed,
            detail: `${bossHealed.toLocaleString('en-US')} progress undone`,
          }
        : null,
  };
}

// --------------------------------------------------------------------------
// Die heute rechenbaren Quellen
// --------------------------------------------------------------------------

/** Was ein DoT-Tick dieses Zaubers ausgeteilt haette. */
export interface DotTickLoss {
  readonly spellId: string;
  readonly lostTicks: number;
  readonly damagePerTick: number;
}

/**
 * Verlorene DoT-Ticks — zu frueh erneuert.
 *
 * Ein eigenstaendiger Verlust und keine Ueberschneidung mit dem Leerlauf: Der
 * Spieler hat GEHANDELT, nur falsch. Das Fenster war nicht leer.
 */
export function dotTickSource(losses: readonly DotTickLoss[]): LossSourceDef {
  return {
    kind: 'lostDotTicks',
    label: 'Lost damage-over-time ticks',
    compute(): LossContribution | null {
      const damage = losses.reduce((sum, l) => sum + l.lostTicks * l.damagePerTick, 0);
      const ticks = losses.reduce((sum, l) => sum + l.lostTicks, 0);
      if (ticks === 0) return { damage: 0, detail: 'No ticks lost to early refresh.' };
      return {
        damage,
        detail:
          `${ticks} ${ticks === 1 ? 'tick' : 'ticks'} lost to early refresh — ` +
          `${Math.round(damage).toLocaleString('en-US')} damage`,
      };
    },
  };
}

/** Verpasste Synergien — verfuegbar minus genutzt. */
export function synergySource(used: number, available: number, bonusDamage: number): LossSourceDef {
  return {
    kind: 'missedSynergies',
    label: 'Missed synergies',
    compute(): LossContribution | null {
      // Ohne moegliche Synergien gibt es nichts zu messen. Eine Null waere
      // hier eine Aussage ueber etwas, das es nicht gibt.
      if (available <= 0) return null;
      const missed = Math.max(0, available - used);
      return {
        damage: missed * bonusDamage,
        detail: `${used} of ${available} combinations used`,
      };
    },
  };
}

/**
 * Verpasste Wirkungen — Fenster, in dem ein Zauber bereit war und nichts
 * gewirkt wurde.
 *
 * Die Zahl der Fenster kommt von aussen: Sie laesst sich aus den heutigen
 * Summen nicht rekonstruieren (Issue #151). `null` bedeutet, dass die
 * Grundlage fehlt — nicht, dass nichts verpasst wurde.
 */
export function missedCastSource(
  windowsWithReadySpell: number | null,
  averageDamagePerCast: number,
): LossSourceDef {
  return {
    kind: 'missedCasts',
    label: 'Missed casts',
    compute(): LossContribution | null {
      if (windowsWithReadySpell === null) return null;
      const damage = windowsWithReadySpell * averageDamagePerCast;
      return {
        damage,
        detail:
          `${windowsWithReadySpell} ${windowsWithReadySpell === 1 ? 'window' : 'windows'} ` +
          `with a ready spell and nothing cast — ` +
          `${Math.round(damage).toLocaleString('en-US')} damage`,
      };
    },
  };
}

/**
 * Leerlauf — Fenster, in denen KEIN Zauber bereit war.
 *
 * Auch diese Zahl kommt von aussen, aus demselben Grund. Zusammen mit den
 * verpassten Wirkungen ergibt sie genau die leeren Fenster, ohne
 * Ueberschneidung (AC7).
 */
export function idleSource(
  windowsWithNothingReady: number | null,
  gcdMs: number,
  averageDamagePerCast: number,
): LossSourceDef {
  return {
    kind: 'idle',
    label: 'Idle time',
    compute(): LossContribution | null {
      if (windowsWithNothingReady === null) return null;
      return {
        damage: windowsWithNothingReady * averageDamagePerCast,
        detail: `${((windowsWithNothingReady * gcdMs) / 1000).toFixed(1)} s with nothing ready`,
      };
    },
  };
}

/**
 * Verpasste Unterbrechungen — zaehlt ueber die GELEGENHEIT.
 *
 * Nicht "der Unterbrecher lag bereit", sondern "ein unterbrechbarer Boss-Zauber
 * lief UND kam durch". Ohne diese Regel steht ein Unterbrecher, der bereitlag,
 * weil es nichts zu unterbrechen gab, als groesste Verlustquelle ganz oben.
 *
 * Braucht Boss-Ereignisse, die das Spiel heute nicht schreibt (Issue #151).
 */
export function interruptSource(
  missedOpportunities: number | null,
  damagePerLandedCast: number,
): LossSourceDef {
  return {
    kind: 'missedInterrupts',
    label: 'Missed interrupts',
    compute(): LossContribution | null {
      if (missedOpportunities === null) return null;
      const damage = missedOpportunities * damagePerLandedCast;
      return {
        damage,
        count: missedOpportunities,
        // "damage taken", nicht "damage": Der Satz sagt die Einheit selbst,
        // damit die Zeile auch fuer sich gelesen nicht missverstanden wird.
        detail:
          `${missedOpportunities} interrupt${missedOpportunities === 1 ? '' : 's'} missed — ` +
          `${Math.round(damage).toLocaleString('en-US')} damage taken`,
      };
    },
  };
}
