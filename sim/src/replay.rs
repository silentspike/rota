//! Aufgezeichnete Runden abspielen (GAME-DESIGN 18.4, 19.1).
//!
//! Die gemeinsamen Pruefaelle sind ausdruecklich **aufgezeichnete echte
//! Durchlaeufe**, nicht von Hand geschriebene Listen:
//!
//! > Eine Sammlung fester Testfaelle der Form „diese Tastendruecke zu diesen
//! > Zeitpunkten mit dieser Ausruestung ergeben exakt diesen Schaden".
//! > **Beide Seiten — Simulator (Rust) und Spiel (Browser) — muessen jede Datei
//! > bestehen.**
//!
//! Der Unterschied zu den Rotationen aus [`crate::rotation`] ist wesentlich:
//! Dort entscheidet der Simulator selbst, was gewirkt wird — er prueft dann
//! seine eigene Vorstellung vom Spiel. Hier bekommt er die Tastendruecke
//! vorgegeben und darf nur noch ausrechnen, was daraus folgt.
//!
//! # Was eine Aufzeichnung enthaelt
//!
//! Takt und Belegung je Tastendruck, dazu den Startwert der Zufallsstroeme —
//! mehr braucht ein Kampf nicht, um sich exakt zu wiederholen. Ausdruecklich
//! **nicht** enthalten sind Zeitstempel des Browsers oder Bildzahlen: Die
//! haengen am Geraet, und genau davon soll eine Aufzeichnung frei sein.
//!
//! Die **Belegung** gehoert mit hinein, weil ein Tastendruck einen Platz nennt
//! und keinen Zauber (GAME-DESIGN 8.1: die Belegung ist frei waehlbar). Ohne sie
//! muesste der Simulator `src/core/loadout.ts` nachbilden — und eine spaetere
//! Umbelegung wuerde jede alte Aufzeichnung still umdeuten.

use crate::time::Millis;

/// Ein Tastendruck: in welchem Takt, auf welchen Platz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// Nummer des Takts, nicht die Zeit in Millisekunden.
    pub tick: u32,
    /// Belegter Platz auf der Zauberleiste.
    pub slot: u8,
}

/// Welcher Zauber auf welchem Platz liegt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    /// Platz auf der Zauberleiste.
    pub slot: u8,
    /// Index des Zaubers in [`crate::data::SPELLS`].
    pub spell: usize,
}

/// Eine aufgezeichnete Runde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    /// Name der Aufzeichnung, wie ihn das Spiel fuehrt.
    pub name: String,
    /// Startwert der Zufallsstroeme.
    pub seed: u32,
    /// Dauer in Takten.
    pub ticks: u32,
    /// Die Belegung, unter der aufgezeichnet wurde.
    pub bindings: Vec<Binding>,
    /// Die Tastendruecke, nach Takt geordnet.
    pub presses: Vec<Press>,
}

impl Recording {
    /// Welcher Zauber auf einem Platz liegt.
    ///
    /// `None` bedeutet: unbelegter Platz. Das Spiel wertet das als Fehleingabe
    /// und merkt sich nichts (GAME-DESIGN 21.1, verworfene Eingabe-Vormerkung) —
    /// der Simulator zaehlt den Druck dann ebenfalls als abgelehnt.
    #[must_use]
    pub fn spell_for_slot(&self, slot: u8) -> Option<usize> {
        self.bindings
            .iter()
            .find(|binding| binding.slot == slot)
            .map(|binding| binding.spell)
    }
}

/// Was beim Abspielen herauskam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayResult {
    /// Gesamtschaden.
    pub damage: u32,
    /// Kampfzeit des Sieges, oder `None`.
    pub killed_at_ms: Option<Millis>,
    /// Angenommene Tastendruecke.
    pub accepted: u32,
    /// Abgelehnte Tastendruecke — zu frueh, unbelegter Platz.
    pub rejected: u32,
    /// Wirkungen je Zauber.
    pub casts: Vec<u32>,
    /// Pruefsumme des Endzustands.
    pub final_checksum: u32,
    /// Pruefsumme ueber die ganze Folge — ein Wert fuer den schnellen Vergleich.
    pub sequence_checksum: u32,
    /// Wie oft ein Synergie-Fenster genutzt wurde (#318).
    pub synergy_used: u32,
    /// Wie oft eines offen stand, das der Partner haette nutzen koennen.
    pub synergy_available: u32,
    /// Pruefsumme je Takt. Nur gefuellt, wenn ausdruecklich verlangt.
    pub checksums: Vec<u32>,
    /// Ziehungen aus den Zufallsstroemen.
    pub random_draws: u32,
    /// Boss-Phase je Takt. Wie `checksums` nur auf Verlangen gefuellt.
    ///
    /// Die drei folgenden Spuren gehen ohnehin in die Pruefsumme ein — aber
    /// eine abweichende Pruefsumme sagt nur *dass* etwas anders ist, nicht
    /// *was*. Mit den Spuren zeigt ein Fehlschlag direkt auf die Phase, die
    /// Energieleiste oder die Zeitleiste (Issue #96).
    pub phases: Vec<u32>,
    /// Stand der Energieleiste je Takt.
    pub energy: Vec<u32>,
    /// Schaden, der im Unverwundbarkeits-Fenster verpuffte (Issue #218).
    ///
    /// **Verloren, nicht ausgeteilt** — die Zahl gehoert in die Fehleranalyse
    /// (GAME-DESIGN 15.3) und ist deshalb ein eigenes Feld, kein Abzug.
    pub damage_lost_to_race: u32,
    /// Wie viel Fortschritt der Boss zurueckgedreht hat (Issue #218).
    pub boss_healed: u32,
    /// Wie viel Fortschritt der Spieler zurueckgedraengt hat (Issue #247).
    pub stabilised: u32,
    /// Ob der Spieler das Wettrennen gewonnen hat (Issue #639).
    ///
    /// **Bis #639 wurde das Feld in `fight.rs` gefuehrt und NIE gelesen** —
    /// vier Zeilen Anlage, Ruecksetzen und Setzen, kein einziger Leser. Die
    /// Spielseite liefert `raceWon` seit #638 ins `ReplayResult`, verglichen
    /// wurde also eine Haelfte gegen nichts.
    pub race_won: bool,
    /// Stand des toedlichen Zaehlers je Takt (Issue #216).
    pub doom: Vec<u32>,
    /// Verbleibende Ladungen der Ruecksetz-Handlung je Takt.
    pub doom_charges: Vec<u32>,
    /// Zahl der exponentiellen Stapel je Takt.
    pub stacks: Vec<u32>,
    /// Stand des Zeitleisten-Zeigers je Takt.
    pub timeline_index: Vec<u32>,
    /// Verbleibende Lebenspunkte des Spielers je Takt (Issue #110).
    pub player_hp: Vec<u32>,
    /// Stand der Schutzschicht je Takt (COAGULATION, Issue #79).
    pub shield: Vec<u32>,
    /// Ob die Schwaechung je Takt liegt, als 0 oder 1.
    pub afflicted: Vec<u32>,
    /// Wann der laufende Boss-Zauber trifft, oder -1 — die Spur der Eskalation.
    pub cast_lands_at: Vec<i64>,
    /// Wie oft die Schwaechung entfernt wurde.
    pub afflictions_cleared: u32,
    /// Wie viele Zyklen die Zeitleiste gelaufen ist.
    pub cycle: u32,
    /// Wie viel die Schutzschicht insgesamt aufgefangen hat.
    pub shield_absorbed: u32,
    /// Wie viel Leben insgesamt zurueckgeholt wurde.
    pub healed: u32,
    /// Wie viel Schutz die Umwandlung erzeugt hat.
    pub converted: u32,
    /// Gesamter erlittener Schaden, **nach** WARD.
    pub damage_taken: u32,
    /// Kampfzeit des Spielertods, oder `None`.
    pub died_at_ms: Option<Millis>,
    /// Ausgegebene Kontroll-Punkte (Issue #605).
    pub control_spent: u32,
    /// Wie oft eine Kontroll-Buchung mangels Punkten abgebrochen ist.
    ///
    /// **Es fehlte nicht die Groesse, sondern die Leitung.** Beide Seiten
    /// zaehlten sie im Kampf; im Spiel steht sie sogar im `RunSummary` und
    /// wird als „Control wasted" angezeigt. Nur kam sie in KEINEM der beiden
    /// Lauf-Ergebnisse an — und damit konnte keine Aufzeichnung je belegen,
    /// dass ein Kontroll-Zauber mangels Punkten gescheitert ist.
    pub control_failed: u32,
    /// Wie oft der Spieler einen Boss-Zauber unterbrochen hat.
    pub interrupts_landed: u32,
    /// Wie oft einer durchgekommen ist.
    pub interrupts_missed: u32,
    /// Wie oft ein Boss-Angriff eine Wirkzeit des SPIELERS abgebrochen hat
    /// (Issue #304).
    pub casts_interrupted: u32,
    /// Schaden je Zauber (Issue #328).
    ///
    /// **Die Pruefsumme traegt die SUMME, nicht die Zuordnung.** Seit der
    /// Widerstand am Zauber haengt, koennen zwei Umsetzungen, die ihn
    /// verschieden verteilen, auf dieselbe Summe kommen — diese Aufteilung
    /// sagt, welcher Zauber gemeint war.
    pub damage_by_spell: [u32; crate::data::SPELL_COUNT],
    /// Wie oft ein eigener Druck eine laufende Wirkzeit abgebrochen hat.
    ///
    /// **Getrennt vom Zaehler darueber**, obwohl beide dasselbe Feld raeumen:
    /// Fuer den Spieler sind "der Boss hat es genommen" und "ich habe es
    /// weggeworfen" zwei verschiedene Dinge.
    pub casts_cancelled: u32,
}

/// Wo zwei Prueffsummen-Folgen zum ersten Mal auseinanderlaufen.
///
/// Das ist der eigentliche Gewinn der Pruefsummen: Nicht *dass* zwei
/// Umsetzungen abweichen, sondern **wo**. Bei 6.000 Takten ist das der
/// Unterschied zwischen einer Minute und einem Nachmittag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Divergence {
    /// Nummer des ersten abweichenden Takts.
    pub tick: u32,
    /// Kampfzeit dieses Takts.
    pub time_ms: Millis,
    /// Was das Spiel gerechnet hat.
    pub expected: u32,
    /// Was der Simulator gerechnet hat.
    pub actual: u32,
    /// Wie viele Takte vorher uebereinstimmten.
    pub matched_ticks: u32,
}

/// Vergleicht zwei Pruefsummen-Folgen und nennt den ersten Unterschied.
///
/// Unterschiedlich lange Folgen sind selbst schon ein Befund: Dann hat eine
/// Seite frueher abgebrochen — meist, weil der Boss dort zu einem anderen Takt
/// gefallen ist.
#[must_use]
pub fn first_divergence(expected: &[u32], actual: &[u32], tick_ms: Millis) -> Option<Divergence> {
    let shared = expected.len().min(actual.len());
    for tick in 0..shared {
        if expected[tick] != actual[tick] {
            return Some(Divergence {
                tick: tick as u32,
                time_ms: tick as Millis * tick_ms,
                expected: expected[tick],
                actual: actual[tick],
                matched_ticks: tick as u32,
            });
        }
    }
    if expected.len() == actual.len() {
        return None;
    }
    // Gleicher Anfang, verschiedene Laenge: Die Abweichung liegt am ersten Takt,
    // den nur eine Seite hat.
    let tick = shared as u32;
    Some(Divergence {
        tick,
        time_ms: tick * tick_ms,
        expected: expected.get(shared).copied().unwrap_or(0),
        actual: actual.get(shared).copied().unwrap_or(0),
        matched_ticks: tick,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_folgen_haben_keine_abweichung() {
        assert_eq!(first_divergence(&[1, 2, 3], &[1, 2, 3], 10), None);
        assert_eq!(first_divergence(&[], &[], 10), None);
    }

    #[test]
    fn nennt_den_ersten_abweichenden_takt() {
        let d = first_divergence(&[1, 2, 3, 4], &[1, 2, 9, 4], 10).expect("weicht ab");
        assert_eq!(d.tick, 2);
        assert_eq!(d.time_ms, 20);
        assert_eq!(d.expected, 3);
        assert_eq!(d.actual, 9);
        assert_eq!(d.matched_ticks, 2);
    }

    #[test]
    fn verschiedene_laenge_ist_selbst_ein_befund() {
        let d = first_divergence(&[1, 2, 3], &[1, 2], 10).expect("weicht ab");
        assert_eq!(d.tick, 2, "der erste Takt, den nur eine Seite hat");
        assert_eq!(d.expected, 3);
        assert_eq!(d.actual, 0);
    }

    #[test]
    fn unbelegter_platz_hat_keinen_zauber() {
        let recording = Recording {
            name: "test".to_owned(),
            seed: 0,
            ticks: 10,
            bindings: vec![Binding { slot: 0, spell: 0 }, Binding { slot: 2, spell: 2 }],
            presses: Vec::new(),
        };
        assert_eq!(recording.spell_for_slot(0), Some(0));
        assert_eq!(recording.spell_for_slot(2), Some(2));
        assert_eq!(recording.spell_for_slot(1), None);
        assert_eq!(recording.spell_for_slot(9), None);
    }
}
