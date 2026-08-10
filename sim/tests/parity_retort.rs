//! **RETORTs Merker, auf beiden Seiten gleich** (Issue #480)
//!
//! RETORT gibt die Wartezeit zurück, die ein Boss-Abbruch gekostet hat. Sein
//! Merker ist der dritte seiner Art — und der erste, der eine ZAHL trägt
//! statt eines Ja/Nein.
//!
//! # Warum die Zahl den Unterschied macht
//!
//! Bei SUBLIME und CONDENSE steht fest, was der Merker bewirkt. Hier hängt es
//! am Zeitpunkt: Ein Abbruch kurz vor Ablauf der Wartezeit erstattet weniger
//! als einer kurz danach. Liefen beide Seiten im Betrag auseinander, wäre der
//! Fehler eine falsche Zahl und kein fehlender Zustand — schwerer zu sehen.
//!
//! Dieser Fall vergleicht die Prüfsumme Takt für Takt. Nimmt jemand den
//! Merker aus einer der beiden Prüfsummen heraus, weichen sie ab.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/retort-erstattung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("retort-erstattung.json ist kein JSON");
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
fn ac0_der_lauf_wirft_retort_und_setzt_den_merker() {
    // **Die Zusicherung vor allem anderen.** Ohne Würfe misst der Fall eine
    // Runde ohne den Zauber, um den es geht; ohne Merker-Takte wäre der
    // Zustand nie gesetzt worden und die Prüfsumme trüge überall dieselbe
    // Null — beides wäre grün und belegte nichts.
    let json = data();
    let wuerfe = json["expected"]["retortCasts"]
        .as_u64()
        .expect("retortCasts fehlt");
    assert!(
        wuerfe > 0,
        "die Aufzeichnung wirft RETORT kein einziges Mal"
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
        // **Die Stufe aus der Datei**, nicht die Voreinstellung: `test-cast`
        // liegt über der Grenze, und die Stufe bewegt die Lebenspunkte und
        // damit jede Prüfsumme.
        player_level: u32::try_from(json["playerLevel"].as_u64().expect("playerLevel fehlt"))
            .expect("passt"),
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
        // **Die Stufe aus der Datei**, nicht die Voreinstellung: `test-cast`
        // liegt über der Grenze, und die Stufe bewegt die Lebenspunkte und
        // damit jede Prüfsumme.
        player_level: u32::try_from(json["playerLevel"].as_u64().expect("playerLevel fehlt"))
            .expect("passt"),
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
