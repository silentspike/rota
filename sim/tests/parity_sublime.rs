//! **SUBLIMEs Merker, auf beiden Seiten gleich** (Issue #480)
//!
//! SUBLIME befreit den nächsten Wurf von der globalen Wartezeit. Der Merker
//! dafür ist der erste seiner Art im Datenmodell — ein Zustand, der zwischen
//! zwei Würfen lebt.
//!
//! # Warum dieser Fall der Wächter für die Prüfsumme ist
//!
//! Ein Merker, der nur auf einer Seite in der Prüfsumme steht, fällt sonst
//! nirgends auf: Beide Seiten könnten ihn verschieden führen — einer
//! verbraucht ihn beim nächsten Wurf, der andere beim übernächsten —, und die
//! Zahlen stimmten weiter, bis irgendwann eine Rotation auseinanderliefe.
//!
//! Dieser Fall vergleicht die Prüfsumme Takt für Takt. Nimmt jemand den
//! Merker aus einer der beiden Prüfsummen heraus, weichen sie ab.
//!
//! **Das gilt für die vier Merker, die folgen** (CONDENSE, RETORT, PUTRID
//! BLOOM, DECAY): Wer einen baut, braucht denselben Nachweis.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/sublime-merker.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("sublime-merker.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn aufbau(json: &Value) -> (Recording, &'static rota_sim::data::Boss, u32) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| Binding {
            slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
            spell: index_of(b["spellId"].as_str().expect("spellId")).expect("Zauber unbekannt"),
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();
    let recording = Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    (recording, boss, ticks)
}

#[test]
fn ac0_der_lauf_wirft_sublime_und_setzt_den_merker() {
    // **Die Zusicherung vor allem anderen.** Ohne Würfe misst der Fall eine
    // Runde ohne den Zauber, um den es geht; ohne Merker-Takte wäre der
    // Zustand nie gesetzt worden und die Prüfsumme trüge überall dieselbe
    // Null — beides wäre grün und belegte nichts.
    let json = data();
    let wuerfe = json["expected"]["sublimeCasts"]
        .as_u64()
        .expect("sublimeCasts fehlt");
    assert!(
        wuerfe > 0,
        "die Aufzeichnung wirft SUBLIME kein einziges Mal"
    );

    let takte = json["expected"]["merkerTakte"]
        .as_u64()
        .expect("merkerTakte fehlt");
    assert!(
        takte > 0,
        "der Merker stand in keinem einzigen Takt — dann ist er nie gesetzt \
         worden, und die Prüfsumme trägt überall dieselbe Null"
    );
}

#[test]
fn ac1_beide_seiten_rechnen_denselben_schaden() {
    // Die Wirkungsgröße: Mit dem Merker fällt ein Wurf mehr in dieselbe Zeit.
    let json = data();
    let (recording, boss, ticks) = aufbau(&json);
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, false);
    assert_eq!(
        u64::from(fight.damage()),
        json["expected"]["damageDealt"]
            .as_u64()
            .expect("damageDealt fehlt"),
        "der Schaden geht auseinander"
    );
}

#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_takt() {
    // **Der Nachweis, dass der Merker auf BEIDEN Seiten in der Prüfsumme
    // steht.** Fehlt er auf einer, weichen die Summen ab, sobald er gesetzt
    // ist — und `merkerTakte > 0` oben sichert zu, dass das vorkommt.
    let json = data();
    let (recording, boss, ticks) = aufbau(&json);
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|c| u32::try_from(c.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect();
    assert!(!erwartet.is_empty(), "keine Prüfsummen in der Aufzeichnung");

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, true);

    assert_eq!(
        ergebnis.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    for (i, (a, b)) in ergebnis.checksums.iter().zip(erwartet.iter()).enumerate() {
        assert_eq!(
            a, b,
            "Prüfsumme weicht in Takt {i} ab — steht der Merker auf beiden \
             Seiten in der Prüfsumme?"
        );
    }
}
