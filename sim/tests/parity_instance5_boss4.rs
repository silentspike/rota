//! Der 4. Boss der fuenften Instanz im Paritaets-Weg — der Abschluss.
//!
//! # Was er verlangt
//!
//! VIER Bausteine und zwei Phasen: `doom`, `energy`, `escalation` und der
//! unterbrechbare Zauber. Der letzte hat kein Boss-Feld — er steht als
//! `mechanic: interruptibleCast` in der Zeitleiste, und wer nur Felder zaehlt,
//! kommt hier auf drei. 11.2 nennt ihn als neunten Baustein.
//!
//! Damit steht dieser Boss an der Obergrenze von 11.1 ("2-4 aus dem
//! Baukasten"). Zwei Uhren
//! zaehlen aufwaerts und wollen dieselbe Antwort zu verschiedenen Zeitpunkten,
//! und die Verkuerzung schiebt beide Zeitpunkte gegeneinander.
//!
//! # Die Phasen sind hier an Ereignisse GEBUNDEN
//!
//! Das ist der Unterschied zu allem, was bisher im Bestand steht: Im gesamten
//! Datenbestand trug genau EIN Zeitleisten-Ereignis ein `phase`-Feld, und das
//! stand in `test-phasenbindung.json`. Die beiden spielbaren Phasen-Bosse
//! (boss-1-2, boss-3-4) binden kein einziges — die Moeglichkeit aus #557 war im
//! Spiel ungenutzt.
//!
//! Hier haengt `Culmination` an Phase 1 und `Last Reckoning` an Phase 2. Ein
//! Ereignis mit `phase` VERFAELLT, solange die Schwelle nicht erreicht ist; es
//! wird nicht nachgeholt.
//!
//! # Beide Phasen sind gemessen, nicht erschlossen
//!
//! Die Aufzeichnung fuehrt seit dieser Instanz ein Feld `phasesSeen`: jede
//! Phase, die im Lauf ANLAG, aufsteigend. Fuer diesen Boss steht dort
//! `[0, 1, 2]`.
//!
//! **Eine Liste und keine Zahl, und das ist der Kern.** `phase_at()` bildet den
//! HP-Anteil direkt auf eine Phasenzahl ab. Faellt der Boss in einem Schlag
//! unter beide Schwellen, springt der Wert von 0 auf 2 — Phase 1 hat nie
//! angelegen, und die hoechste erreichte Phase waere trotzdem 2 gewesen.
//! `[0, 2]` und `[0, 1, 2]` haben dasselbe Maximum und bedeuten Verschiedenes.
//!
//! Die drei anderen Bosse der Instanz tragen keine Phasen und stehen auf `[0]`.
//! Das ist die Gegenprobe zur Messung: Zeigten alle vier dasselbe, waere nicht
//! die Phase gemessen, sondern eine Konstante.
//!
//! Was hier weiterhin NICHT steht, ist die Balance — schaffbar mit Restzeit und
//! die zehn Prozent Unterschied. Das misst der Balance-Laeufer.
//!
//! # Eine offene Frage haengt an diesem Boss
//!
//! Kapitel 21.3 fuehrt "begrenzte Ressource schonen" unter den bewusst NICHT
//! uebernommenen Mustern, und `doom.clearCharges: 3` laesst sich so lesen. Ob
//! damit eine SPIELER-Ressource gemeint ist (dann bleibt alles, und boss-1-2
//! belegt es seit Instanz 1) oder das Muster als solches (dann faellt es auch
//! bei zwei ausgelieferten Bossen), entscheidet der Auftraggeber. Beide Lesarten
//! stehen im Kommentar von `data/bosses/boss-5-4.json`.
//!
//! Praktisch fuer diese Datei: Faellt die Entscheidung gegen das Muster, aendert
//! sich die Rotation, und damit ALLE Zahlen unten — `purge == 2` zuerst. Die
//! Zusicherungen sind dann nicht falsch, sondern veraltet.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/instance5-boss-4.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(JSON_TEXT).expect("instance5-boss-4.json ist kein JSON");
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
    // **Der Traeger der Signatur.** PURGE ist der einzige Zauber mit
    // `clearsDoom`. Faellt die Zahl auf null, laeuft die Verhaengnis-Uhr
    // unbehelligt, und von vier Bausteinen waere einer nie beruehrt worden.
    assert_eq!(
        json["expected"]["casts"]["purge"].as_u64(),
        Some(2),
        "purge wird nicht 2-mal gewirkt — die Verhaengnis-Uhr wird nicht bedient"
    );
    assert_eq!(
        json["expected"]["casts"]["quench"].as_u64(),
        Some(1),
        "quench wird nicht 1-mal gewirkt — das Last Reckoning wird nicht unterbrochen"
    );
    // **Der Lauf reicht bis zum phasengebundenen Ereignis.** `Last Reckoning`
    // steht bei 45000 ms und feuert nur ab Phase 2. Endete der Kampf frueher,
    // pruefte die Aufzeichnung die Bindung nie — und saehe genauso aus.
    assert!(
        json["expected"]["killedAtMs"].as_i64().unwrap_or(-1) > 45000,
        "der Kampf endet vor dem Last Reckoning — die Phasenbindung wird nicht gefahren"
    );
    // **Beide Schwellen liegen an, keine wird uebersprungen.** Als LISTE, weil
    // `[0, 2]` dasselbe Maximum hat und etwas anderes bedeutet.
    let phasen: Vec<u64> = json["expected"]["phasesSeen"]
        .as_array()
        .expect("phasesSeen fehlt — die Aufzeichnung ist aelter als die Phasenmessung")
        .iter()
        .map(|v| v.as_u64().expect("Phase ist keine Zahl"))
        .collect();
    assert_eq!(
        phasen,
        vec![0, 1, 2],
        "nicht beide Phasen liegen an — eine Schwelle faellt nicht oder wird uebersprungen"
    );
    assert_eq!(
        json["expected"]["damage"].as_u64(),
        Some(4792),
        "anderer Schaden — der Boss oder die Rotation hat sich geaendert"
    );
    assert_eq!(
        json["expected"]["diedAtMs"].as_i64(),
        Some(-1),
        "der Spieler stirbt — eine der beiden Uhren laeuft dem Lauf davon"
    );
    assert_eq!(
        json["bossId"].as_str(),
        Some("boss-5-4"),
        "diese Aufzeichnung faehrt einen anderen Boss als gedacht"
    );
}

#[test]
fn beide_seiten_rechnen_denselben_schaden() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-4").expect("boss-5-4 fehlt im Simulator");
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

/// Takt fuer Takt — hier haengen beide Uhren und die Phasenbindung drin.
#[test]
fn die_pruefsumme_stimmt_in_jedem_takt() {
    let json = data();
    let aufzeichnung = recording(&json);
    let boss = boss_by_id("boss-5-4").expect("boss-5-4 fehlt im Simulator");
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
