//! Das Schluesseltalent rechnet auf beiden Seiten gleich (Issue #418, AC4).
//!
//! # Warum hier keine Aufzeichnung steht
//!
//! Eine Paritaets-Aufzeichnung vergleicht Spiel gegen Simulator ueber eine
//! Zauberfolge. Das Schluesseltalent aendert aber nicht den VERLAUF, sondern
//! den VERBRAUCH innerhalb einer Wirkung — und der laesst sich hier direkt
//! messen, ohne den Umweg ueber eine Datei.
//!
//! Was beide Seiten teilen muessen, ist die REGEL: halber Vorrat, zwei
//! Schlaege, ganzzahlig geteilt, Rest bleibt liegen. Genau die steht hier als
//! Rechnung, und dieselbe Rechnung steht auf der Spiel-Seite in
//! `tests/unit/keystone-split-spender.test.ts`.
//!
//! # Der Kern: es aendert den VERBRAUCH, nicht den Schaden
//!
//! "Wirkt sich aus" ist bei einem Faktor trivial. Ein Schluesseltalent muss
//! mehr — sonst waere es ein Faktor mit einem grossen Namen. Deshalb misst
//! dieser Fall den Vorrat NACH dem Entladen.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Ein Lauf, der Hitze auflaedt und dann entlaedt.
///
/// Gibt den Vorrat VOR dem Entladen und den Rest DANACH.
fn laden_und_entladen(split_spender: bool) -> (u16, u16) {
    let boss = boss_by_id("first-crucible").expect("Boss fehlt");
    let kindle = index_of("kindle").expect("kindle fehlt");
    let pyre = index_of("pyre").expect("pyre fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        split_spender,
        ..FightConfig::default()
    });

    // Zwoelf KINDLE laden auf, dann ein PYRE.
    let mut presses: Vec<Press> = (0..12).map(|n| Press { tick: n * 150, slot: 0 }).collect();
    let vor_dem_entladen_tick = 12 * 150;
    presses.push(Press {
        tick: vor_dem_entladen_tick,
        slot: 1,
    });

    let recording = Recording {
        name: "keystone".to_owned(),
        seed: 0,
        ticks: vor_dem_entladen_tick + 10,
        bindings: vec![
            Binding {
                slot: 0,
                spell: kindle,
            },
            Binding {
                slot: 1,
                spell: pyre,
            },
        ],
        presses,
    };

    // **Der Messlauf muss GENAUSO LANG sein wie der echte.** Ein erster
    // Anlauf fuhr ihn nur bis zum Entlade-Tick und las dort ab — in den zehn
    // Takten dazwischen verfaellt aber Hitze (`decay_per_step_ms`), und der
    // abgelesene Vorrat war um eins hoeher als der, mit dem das Entladen
    // wirklich rechnete. Ergebnis: ein erwarteter Rest von 0, ein gemessener
    // von 1, und die Regel sah falsch aus, obwohl sie stimmte.
    //
    // Deshalb faehrt der Messlauf dieselbe Zahl Takte und laesst nur den
    // Entlade-Druck weg.
    let mut messung = Fight::new(FightConfig {
        boss,
        split_spender,
        ..FightConfig::default()
    });
    let nur_laden = Recording {
        presses: recording.presses[..12].to_vec(),
        ..recording.clone()
    };
    messung.replay(&nur_laden, false);
    let vorher = messung.resource();

    fight.replay(&recording, false);
    (vorher, fight.resource())
}

#[test]
fn ac4_ohne_talent_verbraucht_der_entlade_zauber_alles() {
    let (vorher, nachher) = laden_und_entladen(false);
    assert!(
        vorher > 1,
        "der Vorrat ist {vorher} — dann misst der Fall unten nichts"
    );
    assert_eq!(
        nachher, 0,
        "ohne Talent muss der Entlade-Zauber alles verbrauchen"
    );
}

#[test]
fn ac4_mit_talent_bleibt_bei_ungeradem_vorrat_ein_rest() {
    // **Der eigentliche Nachweis, und er misst den VERBRAUCH.** Aus 15 werden
    // zweimal 7, eine Hitze bleibt uebrig — ganzzahlig bis zur letzten
    // Teilung (GAME-DESIGN 18.4).
    let (vorher, nachher) = laden_und_entladen(true);
    let erwartet = vorher - 2 * (vorher / 2);
    assert_eq!(
        nachher, erwartet,
        "bei Vorrat {vorher} muessen {erwartet} liegen bleiben, es sind {nachher}"
    );
}

#[test]
fn ac4_beide_laeufe_starten_mit_demselben_vorrat() {
    // **Die Ursache mitgemessen.** Ohne das erklaerte ein anderer Vorrat den
    // Unterschied ebenso gut wie das Talent.
    let (ohne, _) = laden_und_entladen(false);
    let (mit, _) = laden_und_entladen(true);
    assert_eq!(ohne, mit, "die beiden Laeufe starten verschieden");
}
