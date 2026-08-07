//! Die Kontrolle senkt den erlittenen Schaden (Issue #576, AC3).
//!
//! **Drei Laeufe gegen boss-1-4, die sich in genau einer Sache
//! unterscheiden:** welchen Kontroll-Zauber der Spieler beim laufenden
//! Boss-Zauber wirft — keinen, STAY oder STALL.
//!
//! | Lauf | erlitten | unterbrochen | verpasst | BIND | QUENCH |
//! | --- | --- | --- | --- | --- | --- |
//! | none | 560 | 0 | 1 | 27 | 1 |
//! | stay | 560 | 0 | 1 | 26 | 1 |
//! | stall | **0** | **1** | 0 | 26 | 1 |
//!
//! # Warum der stay-Lauf da ist, und warum ohne ihn nichts bewiesen waere
//!
//! STALL verdraengt einen BIND-Wurf. Ein Vergleich gegen „gar kein
//! Kontroll-Zauber" unterschiede sich in ZWEI Dingen — der Wirkung und einem
//! GCD. STAY kostet Punkte, loest denselben GCD aus und verdraengt denselben
//! BIND, wirkt aber auf die Zeitleiste statt auf den laufenden Cast.
//!
//! Er aendert nichts: 560 wie ohne Kontrolle. Damit gehoert die Differenz
//! STALL und nicht der Verdraengung — und die gleiche BIND-Zahl (26 bei
//! beiden) belegt, dass die Last wirklich dieselbe war.
//!
//! # Warum die Null hier eine Aussage ist
//!
//! boss-1-4 traegt EINEN Zeitleisten-Eintrag: `Sealing Chant`, 700 Schaden,
//! 2500 ms Wirkzeit. Es gibt kein „ein Teil kommt durch" — der Zauber trifft
//! ganz (560 nach 20 % Ward) oder gar nicht. Die Null ist die Struktur des
//! Bosses, kein gewaehlter Randwert.
//!
//! # Warum boss-1-4 und kein Pruefboss
//!
//! Er ist der einzige Boss mit unterbrechbarem Zauber, den der Spieler
//! ueberlebt. An boss-1-1 steht `damageTaken` bei 5040, ob unterbrochen wird
//! oder nicht: Dort stirbt der Spieler, und die Groesse misst dann seine
//! Lebenspunkte statt der Wirkung der Kontrolle. Wer den Nachweis dort
//! fuehrte, bekaeme in jeder Spalte dieselbe Zahl und hielte die Mechanik
//! fuer wirkungslos.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/kontrolle-wirkt.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("kontrolle-wirkt.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn lauf<'a>(json: &'a Value, weg: &str) -> &'a Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["way"].as_str() == Some(weg))
        .unwrap_or_else(|| panic!("kein Lauf fuer Weg {weg}"))
}

/// Spielt einen der drei Laeufe im Simulator nach.
///
/// Die Druecke stehen in der Datei, statt hier aus der Regel nachgebaut zu
/// werden — ein Nachbau, der minimal abweicht, ergaebe eine Abweichung, die
/// wie ein Rechenfehler aussieht.
fn replay(json: &Value, weg: &str) -> rota_sim::ReplayResult {
    let eintrag = lauf(json, weg);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(eintrag["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = eintrag["bindings"]
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
    let presses = eintrag["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    let recording = Recording {
        name: format!("kontrolle-wirkt-{weg}"),
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
    fight.replay(&recording, true)
}

fn erwartet(json: &Value, weg: &str, feld: &str) -> u64 {
    lauf(json, weg)["expected"][feld]
        .as_u64()
        .unwrap_or_else(|| panic!("{weg}/{feld} fehlt"))
}

/// **Die Zusicherung vor jeder Messung** — mit Zahlen, nicht mit Anwesenheit.
#[test]
fn die_lage_ist_noch_dieselbe() {
    let json = data();

    // Der Boss traegt GENAU einen Eintrag. Kaeme ein zweiter dazu, waere die
    // Null unten keine Struktur mehr, sondern ein Zufall.
    assert_eq!(
        json["boss"]["timeline"].as_array().map(Vec::len),
        Some(1),
        "boss-1-4 traegt nicht mehr genau einen Zeitleisten-Eintrag"
    );
    assert_eq!(
        json["boss"]["timeline"][0]["castTimeMs"].as_u64(),
        Some(2500),
        "die Wirkzeit hat sich geaendert — dann liegt der QUENCH-Zeitpunkt falsch"
    );
    assert_eq!(
        json["quenchAtMs"].as_u64(),
        Some(32_500),
        "der QUENCH-Zeitpunkt ist verschoben"
    );

    // **Alle drei druecken QUENCH genau einmal.** Ohne diese Zeile bliebe der
    // Einwand offen, ob im stall-Lauf schlicht MEHR gedrueckt wurde. Der erste
    // Entwurf hatte genau diesen Fehler: QUENCH wurde relativ zum laufenden
    // Cast geworfen und kam in `none` und `stay` gar nicht vor.
    for weg in ["none", "stay", "stall"] {
        assert_eq!(
            erwartet(&json, weg, "castsQuench"),
            1,
            "Lauf {weg} drueckt QUENCH nicht genau einmal"
        );
    }
}

#[test]
fn ac3_stall_senkt_den_erlittenen_schaden() {
    let json = data();

    // Die Messlatte: Ohne Kontrolle kommt der Zauber an.
    assert_eq!(erwartet(&json, "none", "damageTaken"), 560);
    assert_eq!(erwartet(&json, "none", "interruptsLanded"), 0);
    assert_eq!(erwartet(&json, "none", "interruptsMissed"), 1);

    // Die Wirkung: Mit STALL trifft er gar nicht.
    assert_eq!(erwartet(&json, "stall", "damageTaken"), 0);
    assert_eq!(erwartet(&json, "stall", "interruptsLanded"), 1);
    assert_eq!(erwartet(&json, "stall", "interruptsMissed"), 0);
}

#[test]
fn ac3_die_kontrolle_zeigt_dass_es_nicht_die_verdraengung_ist() {
    let json = data();

    // STAY traegt dieselbe Last und aendert nichts.
    assert_eq!(
        erwartet(&json, "stay", "damageTaken"),
        erwartet(&json, "none", "damageTaken"),
        "STAY veraendert den erlittenen Schaden — dann ist die Kontrolle keine"
    );
    assert_eq!(erwartet(&json, "stay", "interruptsLanded"), 0);

    // **Gleiche Last, gemessen statt behauptet.** Weichen die BIND-Zahlen ab,
    // war die Verdraengung verschieden und der Vergleich schief.
    assert_eq!(
        erwartet(&json, "stay", "castsBind"),
        erwartet(&json, "stall", "castsBind"),
        "STAY und STALL verdraengen verschieden viele BIND-Wuerfe"
    );
    assert_eq!(
        erwartet(&json, "stay", "castsControl"),
        erwartet(&json, "stall", "castsControl"),
        "verschieden viele Kontroll-Wuerfe"
    );

    // Und der Lauf ohne Kontrolle wirft einen BIND MEHR — das ist die
    // Verdraengung, die STAY und STALL gemeinsam haben.
    assert!(
        erwartet(&json, "none", "castsBind") > erwartet(&json, "stay", "castsBind"),
        "ohne Kontroll-Zauber wird nicht mehr BIND geworfen — dann verdraengt er nichts"
    );
}

#[test]
fn beide_seiten_rechnen_dasselbe() {
    let json = data();
    for weg in ["none", "stay", "stall"] {
        let ergebnis = replay(&json, weg);
        assert_eq!(
            u64::from(ergebnis.damage_taken),
            erwartet(&json, weg, "damageTaken"),
            "Lauf {weg}: anderer erlittener Schaden"
        );
        assert_eq!(
            u64::from(ergebnis.damage),
            erwartet(&json, weg, "damageDealt"),
            "Lauf {weg}: anderer ausgeteilter Schaden"
        );
        assert_eq!(
            u64::from(ergebnis.sequence_checksum),
            erwartet(&json, weg, "sequenceChecksum"),
            "Lauf {weg}: andere Folge-Pruefsumme"
        );
    }
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    for weg in ["none", "stay", "stall"] {
        let ergebnis = replay(&json, weg);
        let soll: Vec<u32> = lauf(&json, weg)["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("keine Zahl")).expect("zu gross"))
            .collect();

        assert!(!soll.is_empty(), "Lauf {weg} hat keine Pruefsummen");
        if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
            panic!("Lauf {weg}: {d:?}");
        }
        assert_eq!(
            ergebnis.checksums.len(),
            soll.len(),
            "Lauf {weg}: verschieden viele Takte"
        );
    }
}
