//! DECAY schwaecht den Boss-Schaden nach Stapelzahl (Issue #479).
//!
//! # Warum es diese Datei braucht
//!
//! `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einer
//! Aufzeichnung **gewirkt** wird — und `gegenstueck.test.ts`, dass die
//! Aufzeichnung von der Rust-Seite auch gelesen wird. Ohne diese Datei liegt
//! `decay-ward.json` unter `recordings/`, wird exportiert, von `freshness`
//! geprueft und von niemandem verglichen. Genau diese Luecke hat bei
//! `fixation-spells.json` einen echten Paritaetsfehler verdeckt (#456).
//!
//! # Warum `test-onslaught`
//!
//! Er teilt am haertesten aus: drei Wirkzeiten a 2000 Schaden und ein Schlag
//! mit 500. Eine Minderung um ein paar Promille waere bei kleinen Zahlen im
//! Rundungsrauschen; hier steht sie deutlich in `damage_taken`.
//!
//! **Der Boss steht NICHT in der Aufzeichnung**, sondern wird hier gewaehlt.
//! Auf der Spiel-Seite geht er ueber `ReplayOptions.boss`; fehlt er dort,
//! faellt der Abspieler still auf `FIRST_CRUCIBLE` zurueck, der gar nicht
//! austeilt — und `damage_taken` waere null.
//!
//! # Die tragende Zahl ist `damage_taken`
//!
//! DECAY macht keinen Schaden, er nimmt welchen weg. Gemessen im Spiel an der
//! QUELLE (dem Datensatz): **4240 bei 40 Promille je Stapel gegen 5176 bei
//! einem** — 936 weniger erlittener Schaden.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/decay-ward.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("decay-ward.json ist kein JSON");
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

fn recording(json: &Value) -> Recording {
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt"))
                    .expect("slot > 255"),
                spell: index_of(spell_id)
                    .unwrap_or_else(|| panic!("Zauber {spell_id} kennt der Simulator nicht")),
            }
        })
        .collect();

    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick fehlt"))
                .expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot fehlt")).expect("slot > 255"),
        })
        .collect();

    Recording {
        name: json["name"].as_str().expect("name fehlt").to_owned(),
        seed: u32::try_from(json["seed"].as_u64().expect("seed fehlt")).expect("seed zu gross"),
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen"))
            .expect("ticks zu gross"),
        bindings,
        presses,
    }
}

fn aufbau(aufzeichnung: &Recording) -> Fight {
    let boss = boss_by_id("test-onslaught").expect("test-onslaught fehlt");
    Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        boss,
        ..FightConfig::default()
    })
}

/// **Die Zusicherung vor jeder Messung.**
///
/// Eine Aufzeichnung, in der DECAY nicht vorkommt, belegt fuer ihn nichts —
/// und genau dafuer entsteht sie. Geprueft wird BEIDES: dass er auf der Leiste
/// liegt und dass er gedrueckt wird.
#[test]
fn decay_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let decay = index_of("decay").expect("decay fehlt im Simulator");
    let decay_slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == decay)
        .map(|b| b.slot)
        .expect("decay liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == decay_slot)
        .count();
    assert!(
        druecke > 0,
        "decay liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["decay"]
            .as_u64()
            .expect("casts.decay fehlt"),
        "Simulator und Spiel zaehlen verschieden viele DECAY-Wirkungen"
    );
}

/// **Der Boss muss ueberhaupt austeilen.**
///
/// Eine Null hier hiesse, dass die Aufzeichnung gegen einen Boss ohne
/// Zeitleiste laeuft — dann mindert DECAY nichts, und jeder Vergleich
/// darunter ist gruen, ohne etwas zu belegen. Genau so ist die erste Fassung
/// von `pin-befreiung` entstanden.
#[test]
fn die_aufzeichnung_nimmt_ueberhaupt_schaden() {
    let json = data();
    let taken = u32_at(&json["expected"], "damageTaken");
    assert!(
        taken > 0,
        "damageTaken ist null — dieser Lauf trifft den Spieler nie, und eine \
         Schadensminderung ist daran nicht zu messen"
    );
}

#[test]
fn beide_seiten_rechnen_dasselbe() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = aufbau(&aufzeichnung);
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
    // **Die eigentliche Groesse dieses Zaubers.** Sie steht vor der
    // Pruefsumme, weil ihr Bruch etwas anderes bedeutet: eine abweichende
    // Pruefsumme sagt „irgendwo anders", diese Zeile sagt „die Minderung
    // rechnet auf den beiden Seiten verschieden".
    //
    // Hier haengt die Fliesskomma-Frage dran: Das Spiel bildet EINE
    // Promille-Summe und teilt EINMAL, weil `incoming_with_gear` genau das
    // tut. Zwei getrennte Divisionen ergaeben eine Abweichung, die erst in
    // dieser Zeile auffiele.
    assert_eq!(
        ergebnis.damage_taken,
        u32_at(erwartet, "damageTaken"),
        "Simulator und Spiel mindern den eingehenden Schaden verschieden"
    );
    assert_eq!(
        ergebnis.accepted,
        u32_at(erwartet, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        ergebnis.final_checksum,
        u32_at(erwartet, "finalChecksum"),
        "die Schluss-Pruefsumme weicht ab"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
///
/// **Fuer DECAY ist das die entscheidende Probe.** Sein Fenster laeuft ueber
/// ZEIT: Liefen die Seiten es verschieden lange, saehen die Endzahlen
/// womoeglich gleich aus, und erst der Takt-Vergleich zeigte den Unterschied.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = aufbau(&aufzeichnung);
    let ergebnis = fight.replay(&aufzeichnung, true);

    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();

    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
