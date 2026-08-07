//! Die Fixierungs-Zauber im Paritaets-Weg (Issue #533).
//!
//! # Warum es diese Datei braucht
//!
//! `fixation-spells.json` lag unter `recordings/` und wurde von KEINER
//! Rust-Datei gelesen. Das ist der Befund aus #456: Eine Aufzeichnung ohne
//! Gegenstueck sieht aus wie Absicherung und ist keine — sie wird exportiert,
//! sie wird frisch gehalten, und niemand vergleicht die beiden Seiten.
//!
//! # Was sie zeigt
//!
//! TETHER, BIND, WEDGE und CAPSTONE — die Zauber, die Kontroll-Punkte
//! erzeugen statt Schaden.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/fixation-spells.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("fixation-spells.json ist kein JSON");
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

    // **Diese Datei hat weder `name` noch `seed`.** Sie stammt aus einer
    // aelteren Fassung des Exports (`bossId` und `controlSpent` statt
    // `casts`), und genau deshalb war sie so lange ungedeckt: Ein Gegenstueck
    // nach dem ueblichen Muster faellt hier mit „name fehlt" um, bevor es
    // irgendetwas vergleicht.
    //
    // Beides wird hier gesetzt statt gelesen — der Name aus dem Dateinamen,
    // der Startwert aus dem Umstand, dass es im Spieler-Kit keinen Zufall
    // gibt (Design-Gesetz 6).
    Recording {
        name: "fixation-spells".to_owned(),
        seed: 0,
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks fehlen"))
            .expect("ticks zu gross"),
        bindings,
        presses,
    }
}

/// **Die Zusicherung vor jeder Messung** — fuer TETHER einzeln.
#[test]
fn tether_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("tether").expect("tether fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("tether liegt nicht auf der Leiste dieser Aufzeichnung");

    // **Ohne Gegenzaehlung gegen `expected.casts`** — anders als bei den
    // uebrigen Gegenstuecken. Diese Datei fuehrt kein `casts`-Feld; die
    // Zusicherung sagt hier also nur, dass der Zauber gedrueckt WIRD, nicht
    // dass beide Seiten gleich oft zaehlen. Das Zaehlen deckt stattdessen die
    // Takt-Pruefsumme ab, die jeden Druck sieht.
    assert!(
        aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count()
            > 0,
        "tether liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer BIND einzeln.
#[test]
fn bind_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("bind").expect("bind fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("bind liegt nicht auf der Leiste dieser Aufzeichnung");

    // **Ohne Gegenzaehlung gegen `expected.casts`** — anders als bei den
    // uebrigen Gegenstuecken. Diese Datei fuehrt kein `casts`-Feld; die
    // Zusicherung sagt hier also nur, dass der Zauber gedrueckt WIRD, nicht
    // dass beide Seiten gleich oft zaehlen. Das Zaehlen deckt stattdessen die
    // Takt-Pruefsumme ab, die jeden Druck sieht.
    assert!(
        aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count()
            > 0,
        "bind liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer WEDGE einzeln.
#[test]
fn wedge_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("wedge").expect("wedge fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("wedge liegt nicht auf der Leiste dieser Aufzeichnung");

    // **Ohne Gegenzaehlung gegen `expected.casts`** — anders als bei den
    // uebrigen Gegenstuecken. Diese Datei fuehrt kein `casts`-Feld; die
    // Zusicherung sagt hier also nur, dass der Zauber gedrueckt WIRD, nicht
    // dass beide Seiten gleich oft zaehlen. Das Zaehlen deckt stattdessen die
    // Takt-Pruefsumme ab, die jeden Druck sieht.
    assert!(
        aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count()
            > 0,
        "wedge liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer CAPSTONE einzeln.
#[test]
fn capstone_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("capstone").expect("capstone fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("capstone liegt nicht auf der Leiste dieser Aufzeichnung");

    // **Ohne Gegenzaehlung gegen `expected.casts`** — anders als bei den
    // uebrigen Gegenstuecken. Diese Datei fuehrt kein `casts`-Feld; die
    // Zusicherung sagt hier also nur, dass der Zauber gedrueckt WIRD, nicht
    // dass beide Seiten gleich oft zaehlen. Das Zaehlen deckt stattdessen die
    // Takt-Pruefsumme ab, die jeden Druck sieht.
    assert!(
        aufzeichnung
            .presses
            .iter()
            .filter(|p| p.slot == slot)
            .count()
            > 0,
        "capstone liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);

    assert_eq!(
        ergebnis.damage,
        u32_at(&json["expected"], "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
    );
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let mut fight = Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
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
