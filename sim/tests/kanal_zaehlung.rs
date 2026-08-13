//! Der Simulator zaehlt eine Kanalisierung als Wurf (Issue #323).
//!
//! # Warum diese Datei entstanden ist — und warum sie bleibt
//!
//! Am 2026-08-14 sah es eine Stunde lang so aus, als gaebe es hier ein
//! Phaenomen. In `parity_channel.rs` steht
//! `die_zahl_der_wirkungen_stimmt_auf_beiden_seiten`; er vergleicht
//! `result.casts` gegen die Zahl aus der Aufzeichnung. Ein Lauf mit
//! ENTFERNTER Zaehlzeile meldete:
//!
//! ```text
//! der_schluss_stand_stimmt                        FAILED
//! die_pruefsummen_stimmen_in_jedem_einzelnen_takt FAILED
//! die_zahl_der_wirkungen_stimmt_auf_beiden_seiten ok      <- musste rot sein
//! ```
//!
//! Neun Erklaerungen wurden einzeln ausgeschlossen — eine zweite Quelle fuer
//! `casts`, eine dritte Zaehlstelle, ein Praefix-Treffer bei `index_of`, eine
//! eigene Zaehlung in `replay.rs`, geteilter Zustand zwischen den Faellen und
//! anderes. Im Code war nichts.
//!
//! **Die Gegenmessung hat es aufgeloest:** Dieselbe Mutation, frisch gesetzt,
//! ergab DREI rote Faelle — mit `left: 0, right: 3` genau dort, wo der Fall
//! vorher gruen war. Der erste Lauf hatte etwas gemeldet, das nicht
//! stattgefunden hat; woran es lag, ist nicht gemessen und wird deshalb hier
//! nicht behauptet (denkbar: ein inkrementeller Build, der gegen eine aeltere
//! Bibliothek gebunden hat).
//!
//! # Warum die Faelle trotzdem bleiben
//!
//! Nicht weil die Frage offen waere — sie ist beantwortet. Sondern weil sie
//! die Zaehlung OHNE die Replay-Strecke sichern. Der Fall in
//! `parity_channel.rs` misst beides zugleich: den Zaehler und den Weg ueber
//! Aufzeichnung, `unwrap_or` und `replay()`. Wird er einmal rot, sagt er nicht,
//! welches von beidem es war.
//!
//! Diese Datei trennt das. Und sie faengt kuenftig genau die Verwechslung, die
//! hier eine Stunde gekostet hat: Ein Ergebnis, das aussieht wie eine Aussage
//! ueber den Code, waehrend es eine ueber den Lauf war.

use rota_sim::data::SPELLS;
use rota_sim::fight::{Fight, FightConfig};

/// Der Pruef-Kanalzauber — ueber seinen Bezeichner, nicht ueber einen Index.
///
/// Ein `const ZIEL: usize = 43` ist am 2026-08-13 an genau dieser Klasse
/// gekippt: Ein neuer Zauber schob den Katalog, und der Fall mass einen
/// anderen Zauber, ohne es zu sagen.
fn kanal() -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == "test-channel")
        .expect("test-channel fehlt im Katalog")
}

/// Der Zauber mit WIRKZEIT — die Gegenprobe zur Kanalisierung.
fn wirkzeit_zauber() -> usize {
    SPELLS
        .iter()
        .position(|s| s.id == "test-cast")
        .expect("test-cast fehlt im Katalog")
}

/// Wirft den Kanal genau einmal, im ersten Takt.
///
/// **Eine freie Funktion und keine Closure:** `Fight::run` nimmt einen
/// Funktionszeiger. Ein `move |f|` mit gefangenem Index laesst sich nicht
/// uebergeben — der Index wird deshalb hier drin bestimmt.
fn takt_kanal(f: &Fight) -> Option<usize> {
    if f.time() == 0 && f.cast_count(kanal()) == 0 {
        return Some(kanal());
    }
    None
}

/// Dasselbe fuer den Zauber mit Wirkzeit.
fn takt_wirkzeit(f: &Fight) -> Option<usize> {
    if f.time() == 0 && f.cast_count(wirkzeit_zauber()) == 0 {
        return Some(wirkzeit_zauber());
    }
    None
}

/// Wirft den Kanal einmal und laesst ihn vollstaendig durchlaufen.
fn nach_vollem_kanal() -> Fight {
    let index = kanal();
    let spell = &SPELLS[index];
    let mut fight = Fight::new(FightConfig {
        // Reichlich ueber die Kanaldauer hinaus, damit auch der letzte Schritt
        // faellt und das Ende geraeumt ist.
        duration_ms: Some(spell.channel_ms + 3_000),
        ..FightConfig::default()
    });
    fight.run(takt_kanal);
    fight
}

#[test]
fn der_pruefzauber_kanalisiert_ueberhaupt() {
    // **Die Zusicherung vor der Messung.** Ohne sie pruefte der Fall unten
    // einen Zauber, der gar nicht kanalisiert — und `cast_count` waere dann
    // aus einem ganz anderen Grund richtig.
    let spell = &SPELLS[kanal()];
    assert!(
        spell.channel_ms > 0 && spell.channel_tick_ms > 0,
        "test-channel traegt keine Kanalisierung mehr"
    );
    assert!(
        spell.base_damage > 0,
        "test-channel teilt nichts aus — dann sagt ein Zaehler von 1 nichts \
         darueber, ob er gewirkt hat"
    );
}

#[test]
fn ein_durchgelaufener_kanal_zaehlt_als_ein_wurf() {
    let fight = nach_vollem_kanal();
    let index = kanal();

    // **Der Traeger zuerst:** Hat der Kanal ueberhaupt stattgefunden? Ohne
    // diese Zeile waere `cast_count == 1` auch dann falsch-gruen, wenn der
    // Wurf abgelehnt wurde und der Zaehler aus einem anderen Grund stuende.
    assert!(
        fight.damage() > 0,
        "der Kanal hat nichts ausgeteilt — er wurde vermutlich abgelehnt"
    );

    assert_eq!(
        fight.cast_count(index),
        1,
        "ein vollstaendig durchgelaufener Kanal steht mit {} Wuerfen da",
        fight.cast_count(index)
    );
}

#[test]
fn ein_zauber_mit_wirkzeit_zaehlt_genauso() {
    // **Die Gegenprobe, die die Frage ueberhaupt entschieden hat.** Beide Typen
    // beginnen, laufen und loesen auf; nur einer wurde bis #834 gezaehlt. Faellt
    // dieser Fall zusammen mit dem daruber, liegt es an der Zaehlung im
    // Allgemeinen und nicht an der Kanalisierung.
    let index = wirkzeit_zauber();
    let wirkzeit = SPELLS[index].cast_time_ms;
    assert!(wirkzeit > 0, "test-cast traegt keine Wirkzeit mehr");

    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(wirkzeit + 3_000),
        ..FightConfig::default()
    });
    fight.run(takt_wirkzeit);

    assert_eq!(
        fight.cast_count(index),
        1,
        "der Zauber mit Wirkzeit zaehlt nicht"
    );
}
