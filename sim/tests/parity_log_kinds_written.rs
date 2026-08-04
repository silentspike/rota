//! Schreiben beide Seiten dieselben Protokoll-Arten gleich **oft**? (Issue #151)
//!
//! Fuenf Arten standen im gemeinsamen Nummernraum, aber nur der Simulator
//! schrieb sie. Die Fehleranalyse laeuft im Browser — dort war alles leer, und
//! ein Protokoll, in dem die Groesse ueberall fehlt, kann eine Gelegenheit
//! nicht belegen.
//!
//! # Warum die ZAHL der Nachweis ist und nicht das Vorkommen
//!
//! Dass eine Art auftaucht, sagt wenig. Die Zahl sagt, ob sie an denselben
//! Stellen geschrieben wird.
//!
//! Das ist hier keine theoretische Sorge: Im Simulator laeuft **jeder**
//! ausgeteilte Schaden durch [`Fight::deal`] — eine Stelle. Im Spiel gibt es
//! die nicht; dort sind es zwei Stellen fuer Schaden und je eine fuer Schild,
//! Heilung, Unterbrechen und Zeitleiste. Die Schreibpunkte liegen verschieden.
//! Nur ihre Zahl kann zeigen, dass sie trotzdem dasselbe erfassen.
//!
//! Ein Spiel, das den Schlag einmal und der Simulator dreimal protokolliert,
//! waere schlimmer als eines, das gar nichts schreibt: Die Auswertung saehe
//! plausibel aus und waere falsch.

use std::collections::BTreeMap;

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/log-parity.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("log-parity.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

/// Spielt die aufgezeichnete Runde nach und zaehlt je Protokoll-Art.
fn nachspielen(json: &Value) -> (Fight, BTreeMap<&'static str, u32>) {
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
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
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, false);

    let mut zaehlung: BTreeMap<&'static str, u32> = BTreeMap::new();
    for eintrag in fight.log().entries() {
        *zaehlung.entry(eintrag.kind.name()).or_insert(0) += 1;
    }
    (fight, zaehlung)
}

/// Was das Spiel gezaehlt hat, ohne die Arten, die es gar nicht fuehrt.
fn vom_spiel(json: &Value) -> BTreeMap<String, u32> {
    json["counts"]
        .as_object()
        .expect("counts fehlen")
        .iter()
        .filter(|(_, wert)| wert.as_u64() != Some(0))
        .map(|(name, wert)| {
            (
                name.clone(),
                u32::try_from(wert.as_u64().expect("Zahl")).expect("32 Bit"),
            )
        })
        .collect()
}

#[test]
fn ac1_beide_seiten_schreiben_dieselben_arten() {
    let json = data();
    let (_, simulator) = nachspielen(&json);
    let spiel = vom_spiel(&json);

    // **Die Ursache zuerst:** Der Lauf muss die fuenf Arten ueberhaupt
    // ausloesen. Eine Art ohne Eintraege belegt nichts — dieselbe Falle wie
    // bei STALL in #100 und der Schutzschicht in #79.
    for art in [
        "bossTimelineFired",
        "playerDamage",
        "interruptLanded",
        "shieldAbsorb",
        "healed",
    ] {
        assert!(
            spiel.contains_key(art),
            "das Spiel hat {art} nie geschrieben — der Prueffall zeigt dann \
             nichts ueber diese Art"
        );
        assert!(
            simulator.contains_key(art),
            "der Simulator hat {art} nie geschrieben"
        );
    }
}

#[test]
fn ac2_beide_seiten_schreiben_sie_gleich_oft() {
    // **Der eigentliche Nachweis.** Siehe den Kopf dieser Datei: Die
    // Schreibpunkte liegen auf beiden Seiten verschieden, nur die Zahl zeigt,
    // dass sie dasselbe erfassen.
    let json = data();
    let (_, simulator) = nachspielen(&json);
    let spiel = vom_spiel(&json);

    // Nur die Arten vergleichen, die BEIDE Seiten fuehren. `spend` und `kill`
    // gehoeren allein dem Simulator (GAME-DESIGN, Nummernraum aus #105) — sie
    // hier zu fordern hiesse, das Spiel muesse den Boss-Tod protokollieren.
    let nur_simulator = ["spend", "kill"];

    let mut verglichen = 0;
    for (art, anzahl) in &simulator {
        if nur_simulator.contains(art) {
            continue;
        }
        let vom_spiel_gezaehlt = spiel.get(*art).copied().unwrap_or(0);
        assert_eq!(
            *anzahl, vom_spiel_gezaehlt,
            "{art}: Simulator {anzahl}, Spiel {vom_spiel_gezaehlt} — die Art \
             wird auf beiden Seiten verschieden oft geschrieben"
        );
        verglichen += 1;
    }

    // **Ohne diese Zeile prueft der Test nichts.** Waere die Zaehlung leer,
    // liefe die Schleife gar nicht und der Test bliebe gruen.
    assert!(
        verglichen >= 5,
        "es wurden nur {verglichen} Arten verglichen — zu wenige, um etwas \
         zu belegen"
    );
}

#[test]
fn ac3_die_kennzahlen_stimmen_ebenfalls_ueberein() {
    // Die Gegenprobe zur Zaehlung: Nicht nur die ZAHL der Eintraege stimmt,
    // sondern auch das, was sie beschreiben. Eine Seite koennte sonst
    // fuenfmal null protokollieren und dieselbe Zahl erreichen.
    let json = data();
    let (fight, _) = nachspielen(&json);
    let erwartet = &json["expected"];

    for (feld, gemessen) in [
        ("damageDealt", u64::from(fight.damage())),
        ("damageTaken", u64::from(fight.damage_taken())),
        ("shieldAbsorbed", u64::from(fight.shield_absorbed())),
        ("interruptsLanded", u64::from(fight.interrupts_landed())),
        ("healed", u64::from(fight.healed())),
        ("timelineIndex", fight.timeline_index() as u64),
    ] {
        assert_eq!(
            gemessen,
            erwartet[feld]
                .as_u64()
                .unwrap_or_else(|| panic!("{feld} fehlt")),
            "{feld} weicht ab"
        );
    }
}

#[test]
fn ac4_die_referenz_rechnung_ist_unberuehrt() {
    // Protokoll-Eintraege gehen NICHT in die Zustands-Pruefsumme ein. Bewegt
    // sich die Referenz, ist etwas anderes passiert als beabsichtigt.
    let mut fight = Fight::new(FightConfig::default());
    fn gute_rotation(f: &Fight) -> Option<usize> {
        let kindle = index_of("kindle").expect("kindle");
        let smoulder = index_of("smoulder").expect("smoulder");
        let pyre = index_of("pyre").expect("pyre");
        if f.dot_remaining_ms(smoulder) < 1500 && f.is_ready(smoulder) {
            Some(smoulder)
        } else if f.is_ready(pyre) && f.resource() >= 15 {
            Some(pyre)
        } else if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    }
    let ergebnis = fight.run(gute_rotation as fn(&Fight) -> Option<usize>);

    assert_eq!(
        ergebnis.damage, 4240,
        "die Referenz-Rechnung hat sich bewegt"
    );
    assert_eq!(ergebnis.killed_at_ms, Some(40_510));
}
