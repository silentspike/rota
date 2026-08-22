//! Der 2. Boss der fuenften Instanz im Paritaets-Weg.
//!
//! # Was er verlangt
//!
//! DAS TEURE AUFSPAREN: `stacks` und `race` zugleich. Der Stapel waechst von
//! Anfang an, das Rennen liegt spaet (32000 ms, Fenster 6000 ms). Wer fuer das
//! Fenster aufspart, verlaengert die Zeit, in der der Stapel waechst.
//!
//! # Warum hier KEIN Reinigungs-Zauber vorkommt — und warum das kein Mangel ist
//!
//! Dieser Boss traegt weder Gift noch Verhaengnis noch einen unterbrechbaren
//! Zauber. QUENCH, PURGE und CRUST stehen auf der Leiste, weil die Leiste fuer
//! alle vier Bosse der Instanz dieselbe ist, und bleiben hier bei null. Das ist
//! die richtige Antwort auf diesen Boss, nicht eine luecke in der Aufzeichnung.
//!
//! **Deshalb kann die Signatur nicht an einem Zauber haengen.** Sie haengt an
//! der Dauer: Der Kampf muss ueber das Rennen hinauslaufen, sonst hat die
//! Aufzeichnung die Haelfte des Bosses nie beruehrt. Genau darauf zielt die
//! Zusicherung unten — auf eine Zahl, die zeigt, dass die Sache STATTGEFUNDEN
//! hat, nicht dass sie vorhanden ist.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance5-boss-2.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance5-boss-2.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung** — mit Zahlen, nicht mit Anwesenheit.
#[test]
fn die_aufzeichnung_ist_noch_dieselbe() {
    let json = data();
    // **Der Traeger der Signatur ist hier die DAUER, nicht ein Zauber.** Das
    // Rennen beginnt bei 32000 ms und laeuft 6000 ms. Ein Kampf, der vorher
    // endete, haette den halben Boss nie beruehrt — und saehe in jeder anderen
    // Zusicherung genauso aus wie einer, der ihn bedient hat.
    assert!(
        json["expected"]["killedAtMs"].as_i64().unwrap_or(-1) > 38000,
        "der Kampf endet vor dem Ende des Rennens — die halbe Signatur kommt nicht vor"
    );
    assert_eq!(
        json["expected"]["casts"]["kindle"].as_u64(),
        Some(26),
        "kindle wird nicht 26-mal gewirkt — die Aufzeichnung ist eine andere geworden"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4544),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    // **Der Spieler ueberlebt trotz wachsendem Stapel.** Stuerbe er, endete die
    // Aufzeichnung vorzeitig und pruefte den Rest des Kampfes nicht mehr.
    assert_eq!(
        json["expected"]["diedAtMs"].as_i64(),
        Some(-1),
        "der Spieler stirbt — der Stapel waechst dem Lauf ueber den Kopf"
    );
    // **Die Gegenprobe zur Phasenmessung von boss-5-4.** Dieser Boss traegt
    // keine Phasen, also darf nur die Null anliegen. Ohne diese Zeile bewiese
    // das `[0, 1, 2]` drueben wenig: Ein Feld, das ueberall dasselbe meldet,
    // sieht bei einem Phasen-Boss genauso richtig aus wie ein gemessenes.
    let phasen: Vec<u64> = json["expected"]["phasesSeen"]
        .as_array()
        .expect("phasesSeen fehlt — die Aufzeichnung ist aelter als die Phasenmessung")
        .iter()
        .map(|v| v.as_u64().expect("Phase ist keine Zahl"))
        .collect();
    assert_eq!(
        phasen,
        vec![0],
        "ein Boss ohne Phasen meldet einen Phasenwechsel — die Messung greift daneben"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-5-2"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-2").expect("boss-5-2 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
        seed: aufzeichnung.seed,
        duration_ms: Some(aufzeichnung.ticks * TICK_MS),
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&aufzeichnung, true);
    let erwartet = &json["expected"];

    assert_eq!(
        ergebnis.damage,
        u32_at(erwartet, "damage"),
        "Simulator und Spiel rechnen verschiedenen Schaden"
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

/// Takt fuer Takt — hier haengen der wachsende Stapel und das Rennfenster drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-2").expect("boss-5-2 fehlt im Simulator");
    let mut fight = Fight::new(FightConfig {
        boss,
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
