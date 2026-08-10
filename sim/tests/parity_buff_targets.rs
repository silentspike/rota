//! Die fuenf Verstaerkungen auf fuenf Zielgroessen (Issue #540).
//!
//! # Was diese Datei zeigt
//!
//! FLASHPOINT, QUICKSILVER, DISTIL, GRIPLOCK und VIRULENCE verstaerken je eine
//! ANDERE Groesse — Ressource, Umwandlung, Verkuerzung, Punktegewinn,
//! Stapelgewicht. Alle tragen `kind: 0`.
//!
//! Die Aufzeichnung belegt damit mehr als "die Zielgroesse kommt an": Sie
//! zeigt, dass mehrere gleichzeitig laufen, OHNE einander zu verdraengen.
//! GAME-DESIGN 328 ("gleiche Art ersetzt einander") gilt innerhalb einer
//! Zielgroesse; ueber Zielgroessen hinweg muessen sie koexistieren.
//!
//! # DREI gleichzeitig, nicht fuenf — gerechnet, nicht versaeumt
//!
//! Fuenf Druecke brauchen 7500 ms GCD-Zeit, das laengste Fenster ist 6000 ms.
//! Und der erste Nutzer kann fruehestens bei 7500 ms wirken, wenn bereits zwei
//! Fenster geschlossen sind. Die Begruendung steht ausfuehrlich im Kopf von
//! `recordBuffTargets` in `tests/parity/payloads.ts`.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/buff-targets.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON_TEXT).expect("buff-targets.json ist kein JSON");
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

/// **Die Zusicherung vor jeder Messung** — fuer FLASHPOINT einzeln.
///
/// Fuenf Zauber in einer Datei: Ein gemeinsamer Fall waere schon gruen, wenn
/// nur EINER von ihnen gedrueckt wird. Die Zielgroesse hier ist `resource`.
#[test]
fn flashpoint_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("flashpoint").expect("flashpoint fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("flashpoint liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "flashpoint liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["flashpoint"]
            .as_u64()
            .expect("casts.flashpoint fehlt"),
        "Simulator und Spiel zaehlen verschieden viele FLASHPOINT-Wirkungen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer QUICKSILVER einzeln.
///
/// Fuenf Zauber in einer Datei: Ein gemeinsamer Fall waere schon gruen, wenn
/// nur EINER von ihnen gedrueckt wird. Die Zielgroesse hier ist `convert`.
#[test]
fn quicksilver_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("quicksilver").expect("quicksilver fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("quicksilver liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "quicksilver liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["quicksilver"]
            .as_u64()
            .expect("casts.quicksilver fehlt"),
        "Simulator und Spiel zaehlen verschieden viele QUICKSILVER-Wirkungen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer DISTIL einzeln.
///
/// Fuenf Zauber in einer Datei: Ein gemeinsamer Fall waere schon gruen, wenn
/// nur EINER von ihnen gedrueckt wird. Die Zielgroesse hier ist `cdreduce`.
#[test]
fn distil_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("distil").expect("distil fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("distil liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "distil liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["distil"]
            .as_u64()
            .expect("casts.distil fehlt"),
        "Simulator und Spiel zaehlen verschieden viele DISTIL-Wirkungen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer GRIPLOCK einzeln.
///
/// Fuenf Zauber in einer Datei: Ein gemeinsamer Fall waere schon gruen, wenn
/// nur EINER von ihnen gedrueckt wird. Die Zielgroesse hier ist `control`.
#[test]
fn griplock_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("griplock").expect("griplock fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("griplock liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "griplock liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["griplock"]
            .as_u64()
            .expect("casts.griplock fehlt"),
        "Simulator und Spiel zaehlen verschieden viele GRIPLOCK-Wirkungen"
    );
}

/// **Die Zusicherung vor jeder Messung** — fuer VIRULENCE einzeln.
///
/// Fuenf Zauber in einer Datei: Ein gemeinsamer Fall waere schon gruen, wenn
/// nur EINER von ihnen gedrueckt wird. Die Zielgroesse hier ist `stack`.
#[test]
fn virulence_wird_in_dieser_aufzeichnung_wirklich_gewirkt() {
    let json = data();
    let aufzeichnung = recording(&json);

    let zauber = index_of("virulence").expect("virulence fehlt im Simulator");
    let slot = aufzeichnung
        .bindings
        .iter()
        .find(|b| b.spell == zauber)
        .map(|b| b.slot)
        .expect("virulence liegt nicht auf der Leiste dieser Aufzeichnung");

    let druecke = aufzeichnung
        .presses
        .iter()
        .filter(|p| p.slot == slot)
        .count();
    assert!(
        druecke > 0,
        "virulence liegt zwar auf der Leiste, wird aber nie gedrueckt — dann ist \
         seine Bitgleichheit weiterhin nur angenommen"
    );

    assert_eq!(
        u64::try_from(druecke).expect("passt"),
        json["expected"]["casts"]["virulence"]
            .as_u64()
            .expect("casts.virulence fehlt"),
        "Simulator und Spiel zaehlen verschieden viele VIRULENCE-Wirkungen"
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

    // **Der umgewandelte Schaden** — gemessen ungeprüft bis hierher.
    //
    // `converted` steht in 34 Aufzeichnungen und wurde in keinem einzigen
    // Paritätsfall verglichen. Belegt mit einer Mutation: Die Zeile, die ihn
    // aufbaut, wird von 8 Testfällen durchlaufen, und ihn dort um 5 zu
    // verfälschen ließ die ganze Suite grün.
    //
    // Diese Aufzeichnung eignet sich, weil sie einen Wert UNGLEICH NULL trägt
    // (112). Die meisten anderen führen eine 0 — dort wäre der Vergleich
    // erfüllt, ohne etwas zu belegen; deshalb steht die Zusicherung darunter.
    let umgewandelt = u32_at(erwartet, "converted");
    assert!(
        umgewandelt > 0,
        "diese Aufzeichnung wandelt keinen Schaden um — dann vergleicht die \
         Zeile darunter zwei Nullen und belegt nichts"
    );
    assert_eq!(
        ergebnis.converted, umgewandelt,
        "Simulator und Spiel wandeln verschieden viel Schaden um"
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

    // `first_divergence` nennt den Takt statt nur „ungleich" — bei dreitausend
    // Eintraegen ist das der Unterschied zwischen einem Hinweis und einer
    // Suche.
    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "Simulator und Spiel laufen auseinander"
    );
}
