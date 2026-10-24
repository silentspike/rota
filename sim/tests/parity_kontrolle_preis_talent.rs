//! Der Kontroll-Preis unter einer Verteilung aus ZWEI Baeumen (#419).
//!
//! # Was diese Datei absichert
//!
//! LATCH kostet 2 Punkte. Fuenf Raenge `fixation-cost` teilen den Preis durch
//! 1,4 — gerundet 1, und das ist zugleich die Untergrenze, auf die beide Seiten
//! klemmen. Daneben liegt EIN Rang `putrefaction-duration`, der Schutz-Weg der
//! PUTREFACTION. Er hebt den Schutz je Zersetzungs-Stapel und sonst nichts; in
//! dieser Runde liegt kein einziger Stapel, also darf er nichts aendern.
//!
//! # Warum es sie braucht
//!
//! Jede Talent-Aufzeichnung davor trug genau EINEN Weg. Commit 347395fd hatte
//! im Spiel die Untergrenze `1` des Kontroll-Preises durch den Faktor des
//! Schutz-Wegs ersetzt — ohne ihn 1,0 und unsichtbar, mit einem Rang 1,12. Der
//! Simulator klemmte weiter auf 1 (`apply_fixation`). Keine Aufzeichnung trug
//! beide Wege zugleich, also verglich niemand die Stelle, an der sie
//! auseinanderliefen.
//!
//! # Die Verteilung kommt als PROMILLE herein
//!
//! Wie bei `decay-talent` und `spender-talent`: Der Simulator kennt die
//! Talent-Daten nicht und beginnt bei den Promille. Was diese Datei deshalb
//! NICHT prueft, ist die Umrechnung Raenge -> Promille.

use std::fs;

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording, ReplayResult};
use rota_sim::{Fight, FightConfig, TICK_MS};

// `wurzel()` wird gebraucht, `programm()` und `rufe()` nicht — sie gehoeren zu
// den Suiten, die das gebaute Programm starten.
#[allow(dead_code)]
mod gemeinsam;

/// Die Aufzeichnung, **zur Laufzeit** gefunden statt zur Bauzeit eingesetzt.
///
/// Derselbe Weg wie in `parity_spender_talent.rs`, aus demselben Grund. Der
/// Name steht als Literal mit Schraegstrich davor: `gegenstueck.test.ts` sucht
/// `/<name>.json` im Rust-Quelltext.
fn daten() -> Value {
    let pfad = gemeinsam::wurzel().join("tests/parity/recordings/kontrolle-preis-talent.json");
    let text = fs::read_to_string(&pfad).unwrap_or_else(|_| panic!("{} fehlt", pfad.display()));
    let json: Value =
        serde_json::from_str(&text).unwrap_or_else(|_| panic!("{} ist kein JSON", pfad.display()));
    assert_eq!(json["schema"].as_u64(), Some(1), "unbekannte Fassung");
    json
}

fn zahl(json: &Value, feld: &str) -> u32 {
    u32::try_from(
        json["talentPermille"][feld]
            .as_u64()
            .unwrap_or_else(|| panic!("talentPermille.{feld} fehlt")),
    )
    .expect("zu gross")
}

/// Die Runde aus der Aufzeichnung: feste Belegung, feste Druecke.
fn runde(json: &Value) -> Recording {
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
    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: 0,
        ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("zu gross"),
        bindings,
        presses,
    }
}

/// Spielt die Runde im Simulator — mit den Promille der Aufzeichnung, die
/// `aendern` danach gezielt verstellen kann.
fn spielt(json: &Value, aendern: impl FnOnce(&mut FightConfig)) -> ReplayResult {
    let runde = runde(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let mut config = FightConfig {
        seed: 0,
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt")),
        duration_ms: Some(runde.ticks * TICK_MS),
        talent_permille: zahl(json, "damage"),
        talent_control_cost_permille: zahl(json, "controlCost"),
        talent_decay_ward_permille: zahl(json, "decayWard"),
        ..FightConfig::default()
    };
    aendern(&mut config);
    Fight::new(config).replay(&runde, true)
}

/// **Die Zusicherung vor jedem Vergleich:** Die Runde erreicht die Untergrenze.
///
/// Belegt ueber den Simulator selbst, nicht ueber eine nachgebaute Formel:
/// Mit dem Kosten-Weg gelingen BEIDE LATCH aus zwei Punkten — das geht nur bei
/// Preis 1. Ohne den Kosten-Weg kostet LATCH 2, und der zweite scheitert. Erst
/// beides zusammen zeigt, dass der Preis hier genau AUF der Untergrenze liegt;
/// laege er darueber, belegte ein gruener Vergleich nichts ueber sie.
#[test]
fn ac0_die_runde_trifft_den_preis_eins() {
    let json = daten();
    let latch = index_of("latch").expect("latch fehlt");
    let bind = index_of("bind").expect("bind fehlt");

    let mit = spielt(&json, |_| {});
    assert_eq!(mit.casts[latch], 2, "die Runde wirft LATCH nicht zweimal");
    assert_eq!(
        mit.casts[bind], 1,
        "die Runde wirft BIND nicht genau einmal"
    );
    assert_eq!(
        (mit.control_failed, mit.control_spent),
        (0, 2),
        "mit dem Kosten-Weg muessen beide LATCH aus zwei Punkten gelingen — \
         dann ist der Preis 1"
    );

    let ohne = spielt(&json, |c| c.talent_control_cost_permille = 0);
    assert_eq!(
        ohne.control_failed, 1,
        "ohne den Kosten-Weg muss der zweite LATCH scheitern — sonst zeigt die \
         Runde den geteilten Preis gar nicht"
    );

    assert!(
        zahl(&json, "decayWard") > 0,
        "ohne den Schutz-Weg vergleicht diese Datei nichts, was 347395fd beruehrt"
    );
    // Die Aufzeichnung fuehrt die Wurfzahlen selbst — gelesen, nicht Zierrat.
    assert_eq!(json["expected"]["latchCasts"].as_u64(), Some(2));
    assert_eq!(json["expected"]["bindCasts"].as_u64(), Some(1));
}

/// Spiel und Simulator scheitern gleich oft und geben gleich viel aus.
///
/// **Ganze Zahlen, und das ist Teil der Aussage:** Ein Kontroll-Punkt ist
/// unteilbar. Steht in der Aufzeichnung ein Bruch, hat das Spiel einen
/// Preis gerechnet, den der Simulator nicht kennt.
#[test]
fn ac1_beide_seiten_geben_dasselbe_aus() {
    let json = daten();
    let ergebnis = spielt(&json, |_| {});
    for (feld, simulator) in [
        ("controlFailed", ergebnis.control_failed),
        ("controlSpent", ergebnis.control_spent),
        ("damageTaken", ergebnis.damage_taken),
        ("damageDealt", ergebnis.damage),
    ] {
        let spiel = &json["expected"][feld];
        assert_eq!(
            spiel.as_u64(),
            Some(u64::from(simulator)),
            "{feld}: Spiel {spiel}, Simulator {simulator}"
        );
    }
}

/// Takt fuer Takt, nicht nur am Ende.
#[test]
fn ac2_die_pruefsumme_stimmt_in_jedem_takt() {
    let json = daten();
    let ergebnis = spielt(&json, |_| {});
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Pruefsumme ist keine Zahl")).expect("zu gross"))
        .collect();
    assert_eq!(
        ergebnis.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    assert_eq!(
        first_divergence(&erwartet, &ergebnis.checksums, TICK_MS),
        None,
        "die Pruefsummen laufen auseinander"
    );
}

/// **Die Gegenprobe im Simulator: Der Schutz-Weg aendert diese Runde nicht.**
///
/// In ihr liegt kein Zersetzungs-Stapel, also hat der Schutz je Stapel nichts,
/// worauf er wirken koennte. Gleiche Pruefsummen mit und ohne ihn — das ist die
/// Erwartung, an der die Aufzeichnung des Spiels in `ac2` gemessen wird.
#[test]
fn ac3_der_schutz_weg_aendert_diese_runde_nicht() {
    let json = daten();
    let mit = spielt(&json, |_| {});
    let ohne = spielt(&json, |c| c.talent_decay_ward_permille = 0);
    assert!(
        !mit.checksums.is_empty(),
        "keine Pruefsummen — der Vergleich waere leer"
    );
    assert_eq!(
        first_divergence(&ohne.checksums, &mit.checksums, TICK_MS),
        None,
        "der Schutz-Weg bewegt eine Runde ohne einen einzigen Stapel"
    );
    assert_eq!(mit.checksums.len(), ohne.checksums.len());
}
