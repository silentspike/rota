//! Der Boss-WIDERSTAND, Takt fuer Takt (Issue #328, AC7).
//!
//! WARUM ES DIESE DATEI GIBT: DER SIMULATOR KANNTE DAS FELD NICHT
//! --------------------------------------------------------------
//! Gemessen am 2026-08-05, vor diesem Issue:
//!
//! ```text
//! $ grep -c "resist" sim/build.rs
//! 0
//! ```
//!
//! `Boss` hatte kein solches Feld, `fight.rs` setzte `factors.resistance` an
//! keiner Stelle — es blieb auf 1,0. Das SPIEL wendet den Boss-Widerstand
//! dagegen seit langem an (`combat.ts`, `resistanceFactor`).
//!
//! **Beide Seiten rechneten also verschieden, und niemand hat es gemerkt**,
//! weil alle elf Boss-Dateien `resistances: {}` trugen: Die Suche im Spiel
//! lieferte dann ebenfalls 1,0. Eine Abweichung, die auf ihre erste Datenzeile
//! gewartet hat — und die erste Datenzeile ist dieses Issue.
//!
//! DER VERGLEICH IST DIE AUSSAGE, nicht die Bitgleichheit allein
//! -------------------------------------------------------------
//! Dieselbe Druckfolge laeuft in der Vergleichsdatei zweimal: gegen den Boss
//! MIT Widerstand und gegen einen OHNE. Waeren die Zahlen gleich, waere die
//! Bitgleichheit wertlos — sie waere auch dann gruen, wenn der Widerstand auf
//! BEIDEN Seiten nicht ankommt. Genau das war der Zustand vorher.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const RESISTANCE_JSON: &str = include_str!("../../tests/parity/recordings/resistances.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(RESISTANCE_JSON).expect("resistances.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn recording(json: &Value) -> Recording {
    // **Die Belegung kommt aus der DATEI**, nicht aus einer Abschrift hier.
    // Der Pruef-Zauber steht in keiner Leiste des Spiels; wer seinen Platz hier
    // hinschriebe, haette die Zuordnung an zwei Orten — und einer laeuft weg.
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("ticks zu gross"),
        bindings,
        presses,
    }
}

fn replay(json: &Value) -> rota_sim::ReplayResult {
    let recording = recording(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn zahlen(json: &Value, feld: &str) -> Vec<i64> {
    json[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| v.as_i64().expect("Zahl"))
        .collect()
}

#[test]
fn der_fall_kann_den_unterschied_ueberhaupt_zeigen() {
    // **Die Zusicherung, die diesem Pruefstand seinen Wert gibt.** Ohne sie
    // waere alles Weitere gruen, auch wenn der Widerstand auf beiden Seiten
    // spurlos verpufft.
    let json = data();
    let erwartet = &json["expected"];
    let mit = erwartet["damage"].as_u64().expect("damage fehlt");
    let ohne = erwartet["damageWithoutResistance"]
        .as_u64()
        .expect("damageWithoutResistance fehlt");

    assert!(mit > 0, "ohne Schaden misst der Fall nichts");
    assert!(
        mit < ohne,
        "der Widerstand mindert nichts: {mit} gegen {ohne} — dann prueft alles \
         Weitere einen Kampf ohne Wirkung"
    );

    // Und die Staerke im Korridor des Designs (20-30 %), aus den zwei Zahlen
    // gerechnet und nicht aus dem Datenfeld abgeschrieben.
    #[allow(clippy::cast_precision_loss)]
    let anteil = mit as f64 / ohne as f64;
    assert!(
        (0.70..=0.80).contains(&anteil),
        "Minderung ausserhalb des Korridors: Anteil {anteil}"
    );
}

#[test]
fn der_boss_traegt_den_widerstand_auch_im_simulator() {
    // Die Bruecke zwischen Datei und einkompilierten Daten. Ohne sie liefe
    // alles Weitere gegen einen Boss, der im Simulator vielleicht gar keinen
    // Widerstand hat — und die Pruefsummen stimmten dann aus dem falschen Grund
    // nicht ueberein.
    let boss = boss_by_id("test-resistant").expect("test-resistant fehlt");
    assert_eq!(
        boss.resistance_for("CALCINATION"),
        0.75,
        "der Simulator liest den Widerstand nicht aus den Daten"
    );
    // Und der Gegenfall: eine Schule ohne Eintrag bekommt die Eins.
    assert_eq!(boss.resistance_for("COAGULATION"), 1.0);
    // Der Anker ohne jeden Eintrag.
    let ohne = boss_by_id("test-metronome").expect("test-metronome fehlt");
    assert_eq!(ohne.resistance_for("CALCINATION"), 1.0);
}

#[test]
fn die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    let result = replay(&json);
    let erwartet: Vec<u32> = zahlen(&json, "checksums")
        .into_iter()
        .map(|v| u32::try_from(v).expect("32 Bit"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte: Spiel {}, Simulator {}",
        erwartet.len(),
        result.checksums.len()
    );

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  Simulator: {}",
            d.tick, d.time_ms, d.expected, d.actual,
        );
    }
}

#[test]
fn der_schluss_stand_stimmt() {
    let json = data();
    let result = replay(&json);
    let erwartet = &json["expected"];

    assert_eq!(
        u64::from(result.damage),
        erwartet["damage"].as_u64().expect("damage fehlt"),
        "verschiedener Schaden — der Widerstand kommt auf einer Seite nicht an"
    );
    assert_eq!(
        u64::from(result.sequence_checksum),
        erwartet["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "verschiedene Folge-Pruefsumme"
    );
    assert_eq!(
        u64::from(result.final_checksum),
        erwartet["finalChecksum"]
            .as_u64()
            .expect("finalChecksum fehlt"),
        "verschiedene Schluss-Pruefsumme"
    );
}

#[test]
fn ohne_widerstand_rechnet_der_simulator_dieselbe_folge_anders() {
    // **Der Gegenfall auf der SIMULATOR-Seite.** Er beantwortet die Frage, die
    // der erste Fall fuer die Spielseite stellt: Kommt der Widerstand HIER an?
    //
    // Dieselbe Aufzeichnung, derselbe Zufallskeim, ein anderer Boss. Kaeme
    // dieselbe Zahl heraus, liesse der Simulator das Feld fallen — und die
    // Pruefsummen oben stimmten trotzdem, solange das Spiel es ebenfalls taete.
    let json = data();
    let recording = recording(&json);
    let ohne = boss_by_id("test-metronome").expect("test-metronome fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        boss: ohne,
        duration_ms: Some(recording.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let neutral = fight.replay(&recording, false);

    assert_eq!(
        u64::from(neutral.damage),
        json["expected"]["damageWithoutResistance"]
            .as_u64()
            .expect("damageWithoutResistance fehlt"),
        "der Simulator kommt am Boss OHNE Widerstand nicht auf die Zahl des Spiels"
    );
    assert!(
        neutral.damage > replay(&json).damage,
        "derselbe Lauf ergibt mit und ohne Widerstand dasselbe — das Feld kommt \
         im Simulator nicht an"
    );
}

// ---------------------------------------------------------------------------
// Die Bedingung, auf der die Bitgleichheit ruht — und der Waechter dafuer
// ---------------------------------------------------------------------------

/// Ein Kampf mit gesetztem Balance-Regler gegen einen Boss OHNE Widerstand.
///
/// Das ist der Weg, den der Balance-Laeufer geht, und er muss offen bleiben:
/// Der Regler ist sein Werkzeug, um die Schadensformel zu vermessen.
#[test]
fn der_regler_allein_ist_erlaubt() {
    let ohne = boss_by_id("test-metronome").expect("test-metronome fehlt");
    let mut fight = Fight::new(FightConfig {
        boss: ohne,
        factors: rota_sim::factors::DamageFactors {
            resistance: 1.1,
            ..rota_sim::factors::NEUTRAL
        },
        ..FightConfig::default()
    });
    // Kein Abbruch, und der Regler kommt an: ein Lauf ohne Druecke reicht dafuer
    // nicht — gemessen wird nur, dass der Aufbau durchgeht.
    let _ = fight.replay(
        &Recording {
            name: "leer".to_owned(),
            seed: 0,
            ticks: 10,
            bindings: vec![],
            presses: vec![],
        },
        false,
    );
}

/// Und der Boss-Widerstand allein — der Weg jeder Aufzeichnung.
#[test]
fn der_boss_widerstand_allein_ist_erlaubt() {
    let hart = boss_by_id("test-resistant").expect("test-resistant fehlt");
    let _ = Fight::new(FightConfig {
        boss: hart,
        ..FightConfig::default()
    });
}

/// **Beide zusammen brechen ab.**
///
/// Der Simulator rechnete dann `1,1 x 0,75`, das Spiel nur `0,75` — und kein
/// Paritaets-Weg faehrt diese Kombination, also meldete sie niemand. Bis zum
/// 2026-08-05 stand die Bedingung nur als Kommentar da.
///
/// **Der Fall ist der Waechter**, nicht der Kommentar: Wer die Multiplikation
/// spaeter anders baut, bekommt hier eine rote Meldung mit den zwei Zahlen.
#[test]
#[should_panic(expected = "beide gesetzt")]
fn regler_und_boss_widerstand_zusammen_brechen_ab() {
    let hart = boss_by_id("test-resistant").expect("test-resistant fehlt");
    let _ = Fight::new(FightConfig {
        boss: hart,
        factors: rota_sim::factors::DamageFactors {
            resistance: 1.1,
            ..rota_sim::factors::NEUTRAL
        },
        ..FightConfig::default()
    });
}
