//! Der Waechter ueber den gemeinsamen Nummernraum der Protokoll-Arten
//! (Issue E3-6).
//!
//! # Warum es ihn gibt
//!
//! Bis E3-6 fuehrten Spiel und Simulator eigene Nummern:
//!
//! ```text
//!   Nr.   Simulator          Spiel
//!    2    DotTick     gegen  crit
//!    3    Spend       gegen  dotTick
//!    4    Kill        gegen  phase
//! ```
//!
//! Verglichen hat das nie jemand — die Paritaet prueft Zahlen und
//! Pruefsummen, nicht Protokolle. Es war kein Fehler im Lauf, sondern eine
//! gestellte Falle: Die Auswertung in Epic E7 liest genau diese Daten, und
//! eine verschobene Bedeutung ergaebe eine Fehleranalyse, die plausibel
//! aussieht und falsch ist.
//!
//! # Was er leistet
//!
//! Er prueft nicht nur, DASS etwas abweicht, sondern **welche Kennnummer** —
//! ein Waechter, der nur "ungleich" meldet, kostet den naechsten eine Stunde
//! Suche.

use serde_json::Value;

use rota_sim::log::LogKind;

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const LOG_KINDS: &str = include_str!("../../tests/parity/recordings/log-kinds.json");

/// Die Arten, die **dieser** Seite gehoeren, mit ihrer Kennnummer und ihrem
/// Namen im gemeinsamen Raum.
const MINE: [(LogKind, u64, &str); 15] = [
    (LogKind::Cast, 0, "cast"),
    (LogKind::Damage, 1, "damage"),
    (LogKind::DotTick, 2, "dotTick"),
    (LogKind::Spend, 4, "spend"),
    (LogKind::Kill, 5, "kill"),
    // Ab hier Issue #96. Die 6 teilt sich der Simulator seither mit dem Spiel:
    // Beide schreiben den Phasenwechsel, dieselbe Zahl, dieselbe Bedeutung.
    // 8 und 9 sind neu und gehoeren vorerst allein dem Simulator.
    (LogKind::BossPhase, 6, "phase"),
    (LogKind::BossEnergy, 8, "bossEnergyTriggered"),
    (LogKind::BossTimeline, 9, "bossTimelineFired"),
    // Issue #110: Spieler-Leben und Unterbrechen.
    (LogKind::PlayerDamage, 10, "playerDamage"),
    (LogKind::PlayerDeath, 11, "playerDeath"),
    (LogKind::InterruptLanded, 12, "interruptLanded"),
    // Issue #79: die drei Wege der Koagulation.
    (LogKind::ShieldAbsorb, 13, "shieldAbsorb"),
    (LogKind::Healed, 14, "healed"),
    (LogKind::ConvertedToShield, 15, "convertedToShield"),
    // Issue #100: die Fixierung.
    (LogKind::ControlSpent, 16, "controlSpent"),
];

fn kinds() -> Vec<(u64, String, bool)> {
    let value: Value = serde_json::from_str(LOG_KINDS).expect("log-kinds.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von log-kinds.json"
    );
    value["kinds"]
        .as_array()
        .expect("kinds fehlen")
        .iter()
        .map(|entry| {
            (
                entry["kind"].as_u64().expect("kind fehlt"),
                entry["name"].as_str().expect("name fehlt").to_owned(),
                entry["writtenByGame"]
                    .as_bool()
                    .expect("writtenByGame fehlt"),
            )
        })
        .collect()
}

/// **Jede Art dieser Seite traegt dieselbe Nummer wie im Spiel.**
///
/// Bei Abweichung nennt die Meldung die Kennnummer, beide Namen und die
/// Richtung — nicht nur "ungleich".
#[test]
fn dieselbe_nummer_bedeutet_auf_beiden_seiten_dasselbe() {
    let gemeinsam = kinds();

    for (kind, nummer, name) in MINE {
        let eintrag = gemeinsam
            .iter()
            .find(|(nr, _, _)| *nr == nummer)
            .unwrap_or_else(|| {
                panic!(
                    "Kennnummer {nummer} ({name}) fehlt im gemeinsamen Raum. \
                     Der Simulator schreibt sie, das Spiel kennt sie nicht — \
                     entweder log-kinds.json neu schreiben oder die Art hier entfernen."
                )
            });
        assert_eq!(
            eintrag.1, name,
            "Kennnummer {nummer}: Der Simulator nennt sie \"{name}\", das Spiel \"{}\". \
             Dieselbe Zahl bedeutet auf beiden Seiten etwas anderes — genau der Fehler, \
             den Issue E3-6 behoben hat.",
            eintrag.1
        );
        assert_eq!(
            kind as u8,
            u8::try_from(nummer).expect("Kennnummer passt nicht in ein Byte"),
            "LogKind::{kind:?} steht auf {}, im gemeinsamen Raum aber auf {nummer}",
            kind as u8
        );
    }
}

/// Keine Nummer im gemeinsamen Raum gehoert niemandem.
///
/// **Der erste Anlauf dieses Tests war falsch** und hat es sofort gemeldet:
/// Ich hatte die beiden Teilmengen als disjunkt angenommen. Sie sind es nicht
/// — 0, 1 und 2 schreiben BEIDE Seiten, und genau das ist der Sinn eines
/// gemeinsamen Raums. Die Ueberschneidung ist die Regel, nicht der Fehler;
/// falsch waere nur eine Nummer, die dieselbe Zahl verschieden benennt (siehe
/// oben) oder die niemand schreibt.
#[test]
fn jede_nummer_gehoert_mindestens_einer_seite() {
    let gemeinsam = kinds();
    let meine: Vec<u64> = MINE.iter().map(|(_, nr, _)| *nr).collect();

    for (nummer, name, vom_spiel) in &gemeinsam {
        assert!(
            *vom_spiel || meine.contains(nummer),
            "Kennnummer {nummer} ({name}) schreibt niemand — eine tote Nummer im \
             gemeinsamen Raum. Entweder gehoert sie einer Seite oder sie gehoert entfernt."
        );
    }
}

/// Der Simulator fuehrt genau die fuenf Arten, die ihm zustehen.
///
/// Eine Stolperdrahte: Wer eine Variante hinzufuegt oder umnummeriert, ohne
/// den gemeinsamen Raum anzupassen, faellt hier auf.
#[test]
fn der_simulator_fuehrt_genau_seine_fuenfzehn_arten() {
    let gemeinsam = kinds();
    assert_eq!(MINE.len(), 15, "der Simulator hat eine Art dazubekommen");

    // Was nur das Spiel schreibt, darf hier keine Variante haben.
    let nur_spiel: Vec<u64> = gemeinsam
        .iter()
        .filter(|(nr, _, vom_spiel)| *vom_spiel && !MINE.iter().any(|(_, m, _)| m == nr))
        .map(|(nr, _, _)| *nr)
        .collect();
    assert_eq!(
        nur_spiel,
        vec![3, 7],
        "andere Aufteilung als vereinbart — Krit und Verstaerkung laeuft ab \
         gehoeren dem Spiel; den Phasenwechsel (6) schreiben seit #96 beide"
    );

    let meine: Vec<u64> = MINE.iter().map(|(_, nr, _)| *nr).collect();
    assert_eq!(
        meine,
        vec![0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        "der Simulator belegt andere Nummern als vereinbart"
    );
}

/// Der Raum ist luecken- und dublettenfrei.
#[test]
fn der_gemeinsame_raum_ist_vollstaendig() {
    let gemeinsam = kinds();
    for (index, (nummer, _, _)) in gemeinsam.iter().enumerate() {
        assert_eq!(
            *nummer, index as u64,
            "der gemeinsame Raum hat bei {index} eine Luecke oder eine Dublette"
        );
    }
    assert_eq!(gemeinsam.len(), 17, "der Raum hat seine Groesse geaendert");
}
