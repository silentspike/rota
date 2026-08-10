//! PIN macht einen Schlag unterbrechbar, QUENCH unterbricht ihn (Issue #478).
//!
//! # Warum es diese Datei braucht
//!
//! `coverage.test.ts` verlangt, dass jeder ausgelieferte Zauber in einer
//! Aufzeichnung **gewirkt** wird — und `gegenstueck.test.ts`, dass die
//! Aufzeichnung von der Rust-Seite auch gelesen wird. Ohne diese Datei liegt
//! `pin-befreiung.json` unter `recordings/`, wird exportiert, von `freshness`
//! geprueft und von niemandem verglichen. Genau diese Luecke hat bei
//! `fixation-spells.json` einen echten Paritaetsfehler verdeckt (#456).
//!
//! # Warum `test-caster` und kein anderer Boss
//!
//! Seine Zeitleiste besteht aus **drei `strike`-Eintraegen und keinem einzigen
//! `interruptibleCast`** (8500, 9000, 18000, je 40 Schaden). Wird hier etwas
//! unterbrochen, kann es nur PIN freigelegt haben. Gegen einen Boss, der
//! ohnehin wirkt, liesse sich das nicht trennen.
//!
//! **Der Boss steht NICHT in der Aufzeichnung**, sondern wird hier gewaehlt —
//! wie in `parity_cast_time.rs`. Auf der Spiel-Seite geht er ueber
//! `ReplayOptions.boss`; fehlt er dort, faellt der Abspieler still auf
//! `FIRST_CRUCIBLE` zurueck, der gar keine Zeitleiste hat. Die erste Fassung
//! dieser Aufzeichnung meldete deshalb `damageTaken: 0`, waehrend derselbe
//! Ablauf direkt gespielt 64 ergab.
//!
//! # Die tragende Zahl ist `damage_taken`, nicht `damage`
//!
//! PIN macht keinen Schaden, er verhindert welchen. Gemessen im Spiel:
//! **64 mit PIN gegen 96 ohne** — eine Differenz von genau einem Schlag nach
//! der Ward-Minderung.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/pin-befreiung.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("pin-befreiung.json ist kein JSON");
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
    let boss = boss_by_id("test-caster").expect("test-caster fehlt");
    Fight::new(FightConfig {
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        boss,
        ..FightConfig::default()
    })
}

/// **Die Zusicherung vor jeder Messung.**
///
/// Eine Aufzeichnung, in der PIN nicht vorkommt, belegt fuer ihn nichts — und
/// genau dafuer entsteht sie. Geprueft wird BEIDES: dass er auf der Leiste
/// liegt und dass er gedrueckt wird. Nur das erste hatte bei anderen
/// Aufzeichnungen schon zu einem Zauber gefuehrt, den niemand je wirkt.
#[test]
fn pin_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let pin = index_of("pin").expect("pin fehlt im Simulator");
    let pin_slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == pin)
        .map(|b| b.slot)
        .expect("pin liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == pin_slot)
        .count();
    assert!(
        druecke > 0,
        "pin liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["pin"]
            .as_u64()
            .expect("casts.pin fehlt"),
        "Simulator und Spiel zaehlen verschieden viele PIN-Wirkungen"
    );
}

/// **Der Zauber muss WIRKEN, nicht nur geworfen werden.**
///
/// `damage_taken` unter 96 heisst, dass ein Schlag nicht getroffen hat — und
/// bei einem Boss aus lauter `strike`-Eintraegen kann das nur PIN ermoeglicht
/// haben. Stuende hier 96, waere PIN gedrueckt und wirkungslos, und alle
/// anderen Faelle blieben trotzdem gruen.
#[test]
fn der_befreite_schlag_trifft_nicht() {
    let json = data();
    let erwartet = &json["expected"];
    let taken = u32_at(erwartet, "damageTaken");
    assert!(
        taken < 96,
        "damageTaken ist {taken}; ohne PIN faellt jeder der drei Schlaege \
         (96 nach der Ward-Minderung). Ist die Zahl gleich, hat PIN nichts \
         befreit — dann belegt diese Aufzeichnung nur seinen WURF."
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
    // Pruefsumme sagt „irgendwo anders", diese Zeile sagt „der Schlag ist
    // gefallen, obwohl er nicht sollte".
    assert_eq!(
        ergebnis.damage_taken,
        u32_at(erwartet, "damageTaken"),
        "Simulator und Spiel lassen verschieden viel Boss-Schaden durch"
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
