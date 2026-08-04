//! Der Waechter ueber den gemeinsamen Nummernraum der Protokoll-Arten
//! (Issues E3-6 und #134).
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
//! # Der blinde Fleck, den er selbst hatte (#134)
//!
//! Bis Issue #134 stand hier eine **von Hand gefuehrte Abschrift** namens
//! `MINE`. Sie verglich, was dort EINGETRAGEN war — und konnte nicht
//! bemerken, dass etwas FEHLT. Drei Arten (`playerDamage`, `playerDeath`,
//! `interruptLanded`) liefen seit #110 im Simulator mit, ohne je im
//! gemeinsamen Raum zu stehen. Aufgefallen waere es erst, wenn jemand ein
//! Simulator-Protokoll im Spiel liest: Nummer 10 haette dort nichts bedeutet.
//! **Genau der Zustand, den E3-6 abgeschafft hat.**
//!
//! Die Abschrift ist ersatzlos entfallen. An ihre Stelle tritt
//! [`LogKind::ALL`], das ein Makro in `sim/src/log.rs` aus derselben Liste
//! erzeugt wie die Varianten selbst — vergessen kann man es damit nicht mehr.
//!
//! **Warum das die Abschrift ersetzen darf, ohne ihren Zweck zu verlieren:**
//! Ihr Wert war die unabhaengige zweite Meinung. Die liefert weiterhin die
//! Gegenseite — `log-kinds.json` entsteht aus dem Spielcode und weiss vom
//! Simulator nichts. Eine falsche Nummer oder ein vertippter Name in `log.rs`
//! faellt gegen diese Datei auf, nicht gegen eine Abschrift, die derselbe
//! Mensch im selben Augenblick mitgeschrieben hat.
//!
//! # Was er leistet
//!
//! Er prueft nicht nur, DASS etwas abweicht, sondern **welche Kennnummer** —
//! ein Waechter, der nur "ungleich" meldet, kostet den naechsten eine Stunde
//! Suche. Und er prueft **beide Richtungen**: ob eine Art des Simulators im
//! Raum fehlt, und ob eine Art des Raums dem Simulator fehlt.

use serde_json::Value;

use rota_sim::log::LogKind;

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const LOG_KINDS: &str = include_str!("../../tests/parity/recordings/log-kinds.json");

/// Ein Eintrag des gemeinsamen Raums.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Art {
    nummer: u64,
    name: String,
    vom_spiel: bool,
}

fn raum_aus(text: &str) -> Vec<Art> {
    let value: Value = serde_json::from_str(text).expect("log-kinds.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von log-kinds.json"
    );
    value["kinds"]
        .as_array()
        .expect("kinds fehlen")
        .iter()
        .map(|entry| Art {
            nummer: entry["kind"].as_u64().expect("kind fehlt"),
            name: entry["name"].as_str().expect("name fehlt").to_owned(),
            vom_spiel: entry["writtenByGame"]
                .as_bool()
                .expect("writtenByGame fehlt"),
        })
        .collect()
}

fn raum() -> Vec<Art> {
    raum_aus(LOG_KINDS)
}

/// **Richtung 1: Jede Art des Simulators steht im gemeinsamen Raum.**
///
/// Das ist die Richtung, die bis #134 blind war. Als `Result` statt als
/// `assert!`, damit die Gegenprobe sie aufrufen kann, ohne den Prueflauf
/// abzubrechen — ein Waechter, dessen Wirkung man nicht zeigen kann, ist eine
/// Behauptung.
fn jede_art_des_simulators_im_raum(raum: &[Art]) -> Result<(), String> {
    for kind in LogKind::ALL {
        let nummer = u64::from(kind.number());
        match raum.iter().find(|art| art.nummer == nummer) {
            None => {
                return Err(format!(
                    "Kennnummer {nummer} ({}) fehlt im gemeinsamen Raum. Der Simulator \
                     schreibt sie, das Spiel kennt sie nicht — wer ein Simulator-Protokoll \
                     im Spiel liest, saehe dort eine Zahl ohne Bedeutung. Entweder \
                     log-kinds.json neu schreiben (npx vite-node \
                     tests/parity/export-parity.mjs) oder die Art in sim/src/log.rs \
                     entfernen.",
                    kind.name()
                ));
            }
            Some(art) if art.name != kind.name() => {
                return Err(format!(
                    "Kennnummer {nummer}: Der Simulator nennt sie \"{}\", das Spiel \"{}\". \
                     Dieselbe Zahl bedeutet auf beiden Seiten etwas anderes — genau der \
                     Fehler, den Issue E3-6 behoben hat.",
                    kind.name(),
                    art.name
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// **Richtung 2: Jede Art, die dem Simulator gehoert, kennt er auch.**
///
/// Eine Art, die das Spiel NICHT schreibt, muss der Simulator schreiben —
/// sonst schriebe sie niemand, und sie waere eine tote Nummer. Arten, die das
/// Spiel schreibt, darf der Simulator dagegen fehlen: `crit` und `buffExpire`
/// gehoeren ihm nicht.
fn jede_art_des_raums_dem_simulator_bekannt(raum: &[Art]) -> Result<(), String> {
    for art in raum.iter().filter(|art| !art.vom_spiel) {
        if !LogKind::ALL
            .iter()
            .any(|kind| u64::from(kind.number()) == art.nummer)
        {
            return Err(format!(
                "Kennnummer {} ({}) ist im gemeinsamen Raum als Sache des Simulators \
                 eingetragen — das Spiel schreibt sie nicht —, aber sim/src/log.rs kennt \
                 sie nicht. Damit schriebe sie niemand.",
                art.nummer, art.name
            ));
        }
    }
    Ok(())
}

/// AC1 am echten Stand.
#[test]
fn jede_art_des_simulators_steht_im_gemeinsamen_raum() {
    if let Err(meldung) = jede_art_des_simulators_im_raum(&raum()) {
        panic!("{meldung}");
    }
}

/// AC2 am echten Stand.
#[test]
fn jede_art_des_raums_ist_dem_simulator_bekannt() {
    if let Err(meldung) = jede_art_des_raums_dem_simulator_bekannt(&raum()) {
        panic!("{meldung}");
    }
}

/// **Gegenprobe zu AC1** — der Nachweis, dass die Pruefung ueberhaupt greift.
///
/// Ohne sie waere der Prueffall darueber nur gruen, und gruen war die alte
/// Fassung auch: Sie hat drei fehlende Arten wochenlang nicht gemeldet.
#[test]
fn eine_fehlende_art_des_simulators_wird_gemeldet() {
    let letzte = LogKind::ALL.last().expect("keine Arten");
    let ohne: Vec<Art> = raum()
        .into_iter()
        .filter(|art| art.nummer != u64::from(letzte.number()))
        .collect();

    let meldung = jede_art_des_simulators_im_raum(&ohne)
        .expect_err("die fehlende Art ist NICHT aufgefallen — der Waechter ist blind");

    // Und sie muss sagen, WELCHE — nicht nur, dass etwas fehlt.
    assert!(
        meldung.contains(letzte.name()),
        "die Meldung nennt den Namen nicht: {meldung}"
    );
    assert!(
        meldung.contains(&letzte.number().to_string()),
        "die Meldung nennt die Nummer nicht: {meldung}"
    );
}

/// **Gegenprobe zu AC1, zweiter Fall:** dieselbe Nummer, anderer Name.
#[test]
fn ein_abweichender_name_wird_gemeldet() {
    let erste = LogKind::ALL.first().expect("keine Arten");
    let verdreht: Vec<Art> = raum()
        .into_iter()
        .map(|mut art| {
            if art.nummer == u64::from(erste.number()) {
                art.name = "etwasAnderes".to_owned();
            }
            art
        })
        .collect();

    let meldung = jede_art_des_simulators_im_raum(&verdreht)
        .expect_err("der abweichende Name ist NICHT aufgefallen");
    assert!(meldung.contains("etwasAnderes"), "{meldung}");
    assert!(meldung.contains(erste.name()), "{meldung}");
}

/// **Gegenprobe zu AC2.**
#[test]
fn eine_dem_simulator_unbekannte_art_wird_gemeldet() {
    // Eine Art, die das Spiel nicht schreibt und der Simulator nicht kennt —
    // genau der Zustand aus #110, nur andersherum.
    let mut erfunden = raum();
    erfunden.push(Art {
        nummer: 99,
        name: "erfundeneArt".to_owned(),
        vom_spiel: false,
    });

    let meldung = jede_art_des_raums_dem_simulator_bekannt(&erfunden)
        .expect_err("die unbekannte Art ist NICHT aufgefallen");
    assert!(meldung.contains("erfundeneArt"), "{meldung}");
    assert!(meldung.contains("99"), "{meldung}");
}

/// Keine Nummer im gemeinsamen Raum gehoert niemandem (AC3, unveraendert).
///
/// **Der erste Anlauf dieses Tests war falsch** und hat es sofort gemeldet:
/// Ich hatte die beiden Teilmengen als disjunkt angenommen. Sie sind es nicht
/// — 0, 1 und 2 schreiben BEIDE Seiten, und genau das ist der Sinn eines
/// gemeinsamen Raums.
#[test]
fn jede_nummer_gehoert_mindestens_einer_seite() {
    for art in &raum() {
        let vom_simulator = LogKind::ALL
            .iter()
            .any(|kind| u64::from(kind.number()) == art.nummer);
        assert!(
            art.vom_spiel || vom_simulator,
            "Kennnummer {} ({}) schreibt niemand — eine tote Nummer im gemeinsamen Raum.",
            art.nummer,
            art.name
        );
    }
}

/// Der Raum ist luecken- und dublettenfrei (AC3, unveraendert).
#[test]
fn der_gemeinsame_raum_ist_vollstaendig() {
    let gemeinsam = raum();
    for (index, art) in gemeinsam.iter().enumerate() {
        assert_eq!(
            art.nummer, index as u64,
            "der gemeinsame Raum hat bei {index} eine Luecke oder eine Dublette"
        );
    }
    assert!(
        !gemeinsam.is_empty(),
        "ein leerer Raum bestuende jede Pruefung"
    );
}

/// Die Aufzaehlung des Simulators ist in sich stimmig.
///
/// **Keine Zaehlung.** `LogKind::ALL` entsteht im Makro aus derselben Liste
/// wie die Varianten; eine feste Zahl waere genau die Pruefung, die beim
/// naechsten Zuwachs wieder blind ist — jemand setzt sie hoch, ohne
/// nachzusehen. Geprueft wird stattdessen, dass keine Nummer und kein Name
/// doppelt vergeben ist.
#[test]
fn der_simulator_vergibt_keine_nummer_und_keinen_namen_zweimal() {
    let mut nummern: Vec<u8> = LogKind::ALL.iter().map(|k| k.number()).collect();
    let vorher = nummern.len();
    nummern.sort_unstable();
    nummern.dedup();
    assert_eq!(
        nummern.len(),
        vorher,
        "eine Kennnummer ist doppelt vergeben"
    );

    let mut namen: Vec<&str> = LogKind::ALL.iter().map(|k| k.name()).collect();
    namen.sort_unstable();
    namen.dedup();
    assert_eq!(namen.len(), vorher, "ein Name ist doppelt vergeben");
}
