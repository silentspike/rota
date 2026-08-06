//! Der WARD-Deckel, auf beiden Seiten gleich (Issue #466, AC3).
//!
//! # Warum es diese Datei braucht, obwohl sie heute nichts fangen kann
//!
//! Der Deckel ist eine **Regel, keine Zahl in den Daten**: Kein Gegenstand
//! traegt heute genug WARD, um ihn zu erreichen. Ohne diesen Vergleich waere er
//! auf beiden Seiten eingebaut und auf keiner geprueft — und genau daraus
//! entstand #456: Zwei Seiten klammerten das Synergie-Fenster verschieden,
//! monatelang, waehrend die Paritaetspruefung gruen blieb. Sie vergleicht, was
//! aufgezeichnet ist.
//!
//! Sobald der Katalog waechst (#421b), wird der Deckel erreicht. Dann soll der
//! Vergleich schon stehen.
//!
//! # Die vier Laeufe
//!
//! Sie unterscheiden sich in genau einer Sache — dem getragenen WARD. Keine
//! Tastendruecke: Der Spieler wirkt nichts, der Boss schlaegt zu. So misst der
//! Lauf ausschliesslich die Minderung.
//!
//!   400 Promille   Gesamtminderung  60 %   unter dem Deckel
//!   550                             75 %   genau darauf
//!   900                            110 %   darueber, gedeckelt
//!  5000                            520 %   weit darueber, gleiches Ergebnis
//!
//! **Die drei letzten muessen denselben Schaden ergeben, der erste einen
//! anderen.** Das erste ist AC1, das zweite AC2 — und ohne AC2 waere AC1 auch
//! erfuellt, wenn WARD ueberhaupt nichts taete.

use rota_sim::data::{boss_by_id, index_of, PLAYER};
use rota_sim::gear::WornValues;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::time::TICK_MS;
use rota_sim::{Fight, FightConfig};
use serde_json::Value;

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/ward-cap.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("ward-cap.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

/// Einen Lauf nachfahren — **ohne** Tastendruecke.
fn fahre(json: &Value, lauf: &Value) -> rota_sim::ReplayResult {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt"));
    let ticks = u32_at(json, "ticks");

    let recording = Recording {
        name: json["name"].as_str().expect("name fehlt").to_owned(),
        seed: 0,
        ticks,
        // Eine Bindung muss stehen, sonst nimmt `replay` sie als leer nicht an;
        // gedrueckt wird nichts.
        bindings: vec![Binding {
            slot: 0,
            spell: index_of("kindle").expect("kindle fehlt"),
        }],
        presses: Vec::<Press>::new(),
    };

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        worn: WornValues {
            ward: u32_at(lauf, "wardPermille"),
            ..WornValues::default()
        },
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn laeufe(json: &Value) -> Vec<Value> {
    json["runs"].as_array().expect("runs fehlen").to_vec()
}

/// **Die Zusicherung vor jeder Messung.**
#[test]
fn die_aufzeichnung_misst_ueberhaupt_etwas() {
    let json = data();
    let alle = laeufe(&json);
    assert_eq!(alle.len(), 4, "vier Laeufe erwartet");

    for lauf in &alle {
        assert!(
            u32_at(&lauf["expected"], "damageTaken") > 0,
            "ein Lauf ohne eingehenden Schaden prueft die Minderung nicht — \
             traegt der Boss eine Zeitleiste?"
        );
    }

    // Und der Deckel in der Datei ist derselbe, den der Simulator kennt.
    // Stuende dort etwas anderes, waere die Aufzeichnung mit einer anderen
    // Regel entstanden als der, gegen die hier geprueft wird.
    assert_eq!(
        PLAYER.ward_cap_permille,
        u32_at(&json, "wardCapPermille"),
        "Spiel und Simulator gehen von verschiedenen WARD-Deckeln aus"
    );
}

#[test]
fn jeder_lauf_ergibt_denselben_schaden_wie_im_spiel() {
    let json = data();
    for lauf in laeufe(&json) {
        let ergebnis = fahre(&json, &lauf);
        let erwartet = &lauf["expected"];
        let ward = u32_at(&lauf, "wardPermille");

        assert_eq!(
            ergebnis.damage_taken,
            u32_at(erwartet, "damageTaken"),
            "bei {ward} Promille WARD rechnen Spiel und Simulator verschieden"
        );
        assert_eq!(
            *ergebnis.player_hp.last().expect("Spur leer"),
            u32_at(erwartet, "playerHp"),
            "bei {ward} Promille WARD bleiben verschiedene Lebenspunkte"
        );
    }
}

/// AC1 — ueber dem Deckel aendert sich nichts mehr.
#[test]
fn ueber_dem_deckel_mindert_ward_nicht_mehr() {
    let json = data();
    let alle = laeufe(&json);
    let am_deckel = fahre(&json, &alle[1]).damage_taken;

    for lauf in &alle[1..] {
        let ward = u32_at(lauf, "wardPermille");
        assert_eq!(
            fahre(&json, lauf).damage_taken,
            am_deckel,
            "{ward} Promille ergibt einen anderen Schaden als der Deckel — \
             er ist eine Grenze, keine Daempfung"
        );
    }
}

/// AC2 — und darunter wirkt jeder Punkt.
///
/// **Ohne diesen Fall waere der obige auch erfuellt, wenn WARD ueberhaupt
/// nichts taete.**
#[test]
fn unter_dem_deckel_wirkt_jeder_punkt() {
    let json = data();
    let alle = laeufe(&json);
    let drunter = fahre(&json, &alle[0]).damage_taken;
    let am_deckel = fahre(&json, &alle[1]).damage_taken;

    assert!(
        drunter > am_deckel,
        "unter dem Deckel muss weniger WARD mehr Schaden durchlassen: \
         {drunter} gegen {am_deckel}"
    );
}

/// Der Zweck des Deckels, an der Zahl.
#[test]
fn auch_mit_absurd_viel_ward_kommt_schaden_durch() {
    let json = data();
    let alle = laeufe(&json);
    let extrem = fahre(&json, &alle[3]);

    assert!(
        extrem.damage_taken > 0,
        "mit 5000 Promille WARD nimmt der Spieler keinen Schaden — dann ist er \
         unverwundbar, und genau das verhindert der Deckel (#466)"
    );
    // Ungedeckelt waere die Minderung 5,2 gewesen.
    assert!(
        u32_at(&alle[3], "totalPermilleUncapped") > 1000,
        "dieser Lauf liegt gar nicht ueber der Rechenregel — dann zeigt er den \
         Balance-Deckel nicht"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    for lauf in laeufe(&json) {
        let ergebnis = fahre(&json, &lauf);
        let ward = u32_at(&lauf, "wardPermille");
        let erwartet: Vec<u32> = lauf["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| {
                u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross")
            })
            .collect();

        assert_eq!(
            ergebnis.checksums.len(),
            erwartet.len(),
            "bei {ward} Promille verschieden viele Takte"
        );
        for (tick, (links, rechts)) in ergebnis.checksums.iter().zip(erwartet.iter()).enumerate() {
            assert_eq!(
                links, rechts,
                "bei {ward} Promille laufen Spiel und Simulator ab Takt {tick} auseinander"
            );
        }
    }
}
