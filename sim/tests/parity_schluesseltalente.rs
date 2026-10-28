//! Die zwei Schluesseltalente mit Zustand rechnen auf beiden Seiten gleich
//! (#419, #418 AC4).
//!
//! # Was diese Datei absichert
//!
//! `decay-echo` (PUTREFACTION) und `shield-refund` (COAGULATION) legen
//! Kampfzustand an — den Endpunkt des Nachlaufs, den Merker der Nachlage und
//! ihre Grundlage. Seit #419 steht er in beiden Pruefsummen, aber NUR bei
//! verteiltem Talent; jede andere Aufzeichnung bleibt bitgleich. Diese drei
//! sind deshalb die einzigen, die ihn ueberhaupt vergleichen.
//!
//! Je Aufzeichnung: Takt fuer Takt dieselbe Pruefsumme, dieselbe Feldfolge
//! mit Talent, dieselben Ergebnisse — und die positive Kontrolle, dass das
//! Talent in dieser Runde etwas aendert. Die dritte (`schluessel-nachlage-
//! ablauf`) belegt das Gegenteil: Dort darf es NICHTS aendern, weil der
//! gelegte Schild ungebrochen ablaeuft (Abnahme von #1146).
//!
//! # Die Verteilung kommt als PROMILLE und SCHALTER herein
//!
//! Wie bei `kontrolle-preis-talent`: Der Simulator kennt die Talent-Daten nicht
//! und beginnt bei den Promille. Die Schluesseltalente stehen als Kennung in
//! `keystones` und werden hier auf die Schalter in `FightConfig` gelegt. Was
//! diese Datei NICHT prueft, ist die Umrechnung Raenge -> Promille.

use std::fs;

use serde_json::Value;

use rota_sim::checksum::record_fields;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::replay::{first_divergence, Binding, Press, Recording, ReplayResult};
use rota_sim::{Fight, FightConfig, TICK_MS};

// `wurzel()` wird gebraucht, `programm()` und `rufe()` nicht — sie gehoeren zu
// den Suiten, die das gebaute Programm starten.
#[allow(dead_code)]
mod gemeinsam;

const NACHLAUF: &str = "tests/parity/recordings/schluessel-nachlauf.json";
const NACHLAGE: &str = "tests/parity/recordings/schluessel-nachlage.json";
const NACHLAGE_ABLAUF: &str = "tests/parity/recordings/schluessel-nachlage-ablauf.json";

/// Die Aufzeichnung, **zur Laufzeit** gefunden statt zur Bauzeit eingesetzt —
/// derselbe Weg wie in `parity_kontrolle_preis_talent.rs`.
fn daten(pfad: &str) -> Value {
    let pfad = gemeinsam::wurzel().join(pfad);
    let text = fs::read_to_string(&pfad).unwrap_or_else(|_| panic!("{} fehlt", pfad.display()));
    let json: Value =
        serde_json::from_str(&text).unwrap_or_else(|_| panic!("{} ist kein JSON", pfad.display()));
    assert_eq!(json["schema"].as_u64(), Some(1), "unbekannte Fassung");
    json
}

fn zahl(wert: &Value, was: &str) -> u32 {
    u32::try_from(
        wert.as_u64()
            .unwrap_or_else(|| panic!("{was} ist keine Zahl")),
    )
    .expect("zu gross")
}

/// Die Verteilung aus der Aufzeichnung — **jedes** Feld, sonst ein Abbruch.
///
/// Ein unbekannter Schluessel fiele sonst still heraus: Der Simulator rechnete
/// ohne ihn, das Spiel mit ihm, und die Pruefsummen liefen auseinander, ohne
/// dass die Meldung auf die Ursache zeigte.
fn verteilung(json: &Value, config: &mut FightConfig) {
    let promille = json["talentPermille"]
        .as_object()
        .expect("talentPermille fehlt");
    for (feld, wert) in promille {
        let wert = zahl(wert, feld);
        match feld.as_str() {
            "damage" => config.talent_permille = wert,
            "decayWard" => config.talent_decay_ward_permille = wert,
            "shieldAmount" => config.talent_shield_amount_permille = wert,
            anderes => panic!("talentPermille.{anderes} kennt dieser Fall nicht"),
        }
    }
    let schluessel = json["keystones"].as_array().expect("keystones fehlen");
    assert!(
        !schluessel.is_empty(),
        "keine Schluesseltalente — die Datei misst nichts, wofuer sie da ist"
    );
    for kennung in schluessel {
        match kennung.as_str().expect("Kennung ist kein Text") {
            "decay-echo" => config.decay_echo = true,
            "shield-refund" => config.shield_refund = true,
            anderes => panic!("Schluesseltalent {anderes} kennt dieser Fall nicht"),
        }
    }
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
            tick: zahl(&p["tick"], "tick"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();
    Recording {
        name: json["name"].as_str().expect("name").to_owned(),
        seed: 0,
        ticks: zahl(&json["ticks"], "ticks"),
        bindings,
        presses,
    }
}

/// Spielt die Runde mit der Verteilung der Aufzeichnung — `aendern` verstellt
/// sie danach gezielt.
fn spielt(json: &Value, aendern: impl FnOnce(&mut FightConfig)) -> (Fight, ReplayResult) {
    let runde = runde(json);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let mut config = FightConfig {
        seed: 0,
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt")),
        duration_ms: Some(runde.ticks * TICK_MS),
        ..FightConfig::default()
    };
    verteilung(json, &mut config);
    aendern(&mut config);
    let mut fight = Fight::new(config);
    let ergebnis = fight.replay(&runde, true);
    (fight, ergebnis)
}

fn pruefsummen(json: &Value) -> Vec<u32> {
    json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| zahl(v, "Pruefsumme"))
        .collect()
}

/// Takt fuer Takt, nicht nur am Ende — die Aussage aus #418 AC4.
fn pruefsumme_in_jedem_takt(pfad: &str) {
    let json = daten(pfad);
    let (_, ergebnis) = spielt(&json, |_| {});
    let erwartet = pruefsummen(&json);
    assert_eq!(
        ergebnis.checksums.len(),
        erwartet.len(),
        "{pfad}: verschieden viele Takte"
    );
    if let Some(d) = first_divergence(&erwartet, &ergebnis.checksums, TICK_MS) {
        panic!(
            "{pfad}: die Pruefsummen laufen auseinander\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel: {}\n  Simulator: {}",
            d.tick, d.time_ms, d.expected, d.actual
        );
    }
}

/// Dieselben Namen in derselben Reihenfolge, MIT Talent gemessen (#236).
///
/// `doom-stacks` traegt die Folge ohne Talent; die Felder der
/// Schluesseltalente stehen nur hier.
fn feldfolge_mit_talent(pfad: &str) {
    let json = daten(pfad);
    let (fight, _) = spielt(&json, |_| {});
    let ticks = zahl(&json["ticks"], "ticks");
    let rust: Vec<&str> = record_fields(|| {
        let _ = fight.checksum_at((ticks - 1) * TICK_MS);
    })
    .into_iter()
    .map(|(name, _)| name)
    .collect();
    let spiel: Vec<&str> = json["fieldOrder"]
        .as_array()
        .expect("fieldOrder fehlt")
        .iter()
        .map(|n| n.as_str().expect("Feldname ist kein Text"))
        .collect();
    assert!(
        spiel.len() > 100,
        "{pfad}: zu wenige Namen ({})",
        spiel.len()
    );
    for (i, (r, s)) in rust.iter().zip(spiel.iter()).enumerate() {
        assert_eq!(r, s, "{pfad}: die Feldfolge weicht bei {i} ab");
    }
    assert_eq!(
        rust.len(),
        spiel.len(),
        "{pfad}: verschieden lange Feldfolgen"
    );
}

// ---------------------------------------------------------------------------
// decay-echo
// ---------------------------------------------------------------------------

#[test]
fn nachlauf_die_runde_faehrt_beide_enden_des_nachlaufs() {
    // **Die Zusicherung in der Datei**, vor jedem Vergleich: Der Endpunkt
    // faellt zweimal — bei 9000 ms, weil NECROSIS die Stapel verzehrt hat, und
    // bei 32 000 ms nach seiner Frist. Fehlte einer, verglich die Runde nur
    // einen der beiden Wege, auf denen `im_decay_echo` ihn abraeumt.
    let json = daten(NACHLAUF);
    let enden: Vec<u32> = json["expected"]["echoEnds"]
        .as_array()
        .expect("echoEnds fehlt")
        .iter()
        .map(|v| zahl(v, "echoEnds"))
        .collect();
    assert_eq!(enden, vec![900, 3200], "die Runde faehrt nicht beide Enden");
}

#[test]
fn nachlauf_die_pruefsumme_stimmt_in_jedem_takt() {
    pruefsumme_in_jedem_takt(NACHLAUF);
}

#[test]
fn nachlauf_dieselbe_feldfolge_mit_talent() {
    feldfolge_mit_talent(NACHLAUF);
}

#[test]
fn nachlauf_beide_seiten_rechnen_dasselbe_ergebnis() {
    let json = daten(NACHLAUF);
    let erwartet = &json["expected"];
    let (fight, ergebnis) = spielt(&json, |_| {});
    let rot = index_of("rot").expect("rot fehlt");
    let necrosis = index_of("necrosis").expect("necrosis fehlt");
    for (feld, simulator) in [
        ("decayStacks", fight.decay_stacks() as u32),
        ("damageDealt", ergebnis.damage),
        ("rotCasts", ergebnis.casts[rot]),
        ("rotTicks", fight.dot_tick_count(rot)),
        ("necrosisCasts", ergebnis.casts[necrosis]),
    ] {
        assert_eq!(
            erwartet[feld].as_u64(),
            Some(u64::from(simulator)),
            "{feld}: Spiel {}, Simulator {simulator}",
            erwartet[feld]
        );
    }
}

#[test]
fn nachlauf_ohne_das_talent_laeuft_die_runde_anders() {
    // **Die positive Kontrolle.** Ohne sie hiesse ein gruener Vergleich nur,
    // dass beide Seiten dasselbe rechnen — nicht, dass sie den Nachlauf
    // rechnen. Gemessen an den Stapeln am Ende, nicht an der Pruefsumme: Die
    // waere ohne Talent schon deshalb anders, weil die Felder fehlen.
    let json = daten(NACHLAUF);
    let (mit, _) = spielt(&json, |_| {});
    let (ohne, _) = spielt(&json, |c| c.decay_echo = false);
    assert!(mit.decay_stacks() > 0, "mit Talent liegt am Ende nichts");
    assert_ne!(
        ohne.decay_stacks(),
        mit.decay_stacks(),
        "der Nachlauf aendert in dieser Runde nichts"
    );
}

// ---------------------------------------------------------------------------
// shield-refund
// ---------------------------------------------------------------------------

#[test]
fn nachlage_die_runde_faehrt_jeden_zustand_des_merkers() {
    // Zwei Schilde, zwei Nachlagen — bei 5000 und 19 000 ms. Und KEINE bei
    // 12 000 ms, wo die Nachlage selbst bricht: Dort zeigt sich, dass der
    // Merker verbraucht ist.
    let json = daten(NACHLAGE);
    let nachlagen: Vec<u32> = json["expected"]["refunds"]
        .as_array()
        .expect("refunds fehlt")
        .iter()
        .map(|v| zahl(v, "refunds"))
        .collect();
    assert_eq!(
        nachlagen,
        vec![500, 1900],
        "die Runde faehrt den Merker nicht ganz"
    );
    assert_eq!(json["expected"]["crustCasts"].as_u64(), Some(2));
}

#[test]
fn nachlage_die_pruefsumme_stimmt_in_jedem_takt() {
    pruefsumme_in_jedem_takt(NACHLAGE);
}

#[test]
fn nachlage_dieselbe_feldfolge_mit_talent() {
    feldfolge_mit_talent(NACHLAGE);
}

#[test]
fn nachlage_beide_seiten_rechnen_dasselbe_ergebnis() {
    let json = daten(NACHLAGE);
    let erwartet = &json["expected"];
    let (fight, ergebnis) = spielt(&json, |_| {});
    let crust = index_of("crust").expect("crust fehlt");
    for (feld, simulator) in [
        ("shield", fight.shield()),
        ("shieldAbsorbed", ergebnis.shield_absorbed),
        ("damageTaken", ergebnis.damage_taken),
        ("playerHp", fight.player_hp()),
        ("crustCasts", ergebnis.casts[crust]),
    ] {
        assert_eq!(
            erwartet[feld].as_u64(),
            Some(u64::from(simulator)),
            "{feld}: Spiel {}, Simulator {simulator}",
            erwartet[feld]
        );
    }
}

#[test]
fn nachlage_ohne_das_talent_laeuft_die_runde_anders() {
    // Die positive Kontrolle, wie beim Nachlauf: am erlittenen Schaden, nicht
    // an der Pruefsumme.
    let json = daten(NACHLAGE);
    let (_, mit) = spielt(&json, |_| {});
    let (_, ohne) = spielt(&json, |c| c.shield_refund = false);
    assert!(
        ohne.damage_taken > mit.damage_taken,
        "ohne Nachlage muesste mehr ankommen: ohne {}, mit {}",
        ohne.damage_taken,
        mit.damage_taken
    );
}

// ---------------------------------------------------------------------------
// shield-refund, wenn der gelegte Schild ungebrochen ablaeuft
// ---------------------------------------------------------------------------
//
// Die Abnahme von #1146: Die Nachlage kommt aus dem Schild, der GELEGT wurde
// und JETZT bricht. Vorher gab eine Umwandlungs-Schicht (TEMPER) beim Bruch
// die Haelfte einer CRUST, die laengst ungebrochen abgelaufen war — 495 bei
// 15 000 ms, auf beiden Seiten gleich.

fn zahlen(json: &Value, feld: &str) -> Vec<u32> {
    json["expected"][feld]
        .as_array()
        .unwrap_or_else(|| panic!("expected.{feld} fehlt"))
        .iter()
        .map(|v| zahl(v, feld))
        .collect()
}

#[test]
fn ablauf_die_runde_faehrt_ablauf_und_zwei_brueche_ohne_nachlage() {
    // **Die Zusicherung in der Datei**: Die CRUST laeuft ab (Takt 1201), und
    // zwei Umwandlungs-Schichten brechen (1500, 2300). Fehlte der Ablauf, waere
    // der Merker verbraucht und die leere `refunds`-Liste belegte nichts.
    let json = daten(NACHLAGE_ABLAUF);
    assert_eq!(
        zahlen(&json, "expiries"),
        vec![1201],
        "die CRUST laeuft nicht ab"
    );
    assert_eq!(
        zahlen(&json, "breaks"),
        vec![1500, 2300],
        "die Umwandlung bricht nicht"
    );
    assert!(
        zahlen(&json, "refunds").is_empty(),
        "eine Umwandlung legt nach"
    );
    assert!(
        zahl(&json["expected"]["converted"], "converted") > 0,
        "TEMPER wandelt nichts um"
    );
}

#[test]
fn ablauf_die_pruefsumme_stimmt_in_jedem_takt() {
    pruefsumme_in_jedem_takt(NACHLAGE_ABLAUF);
}

#[test]
fn ablauf_dieselbe_feldfolge_mit_talent() {
    feldfolge_mit_talent(NACHLAGE_ABLAUF);
}

#[test]
fn ablauf_beide_seiten_rechnen_dasselbe_ergebnis() {
    let json = daten(NACHLAGE_ABLAUF);
    let erwartet = &json["expected"];
    let (fight, ergebnis) = spielt(&json, |_| {});
    for (feld, simulator) in [
        ("shield", fight.shield()),
        ("shieldAbsorbed", ergebnis.shield_absorbed),
        ("converted", ergebnis.converted),
        ("damageTaken", ergebnis.damage_taken),
        ("playerHp", fight.player_hp()),
    ] {
        assert_eq!(
            erwartet[feld].as_u64(),
            Some(u64::from(simulator)),
            "{feld}: Spiel {}, Simulator {simulator}",
            erwartet[feld]
        );
    }
}

#[test]
fn ablauf_mit_und_ohne_talent_kommt_gleich_viel_an() {
    // **Die Aussage der Runde, am Schaden gemessen**: Keine Nachlage — also
    // nimmt das Talent hier nichts ab. Vor der Abnahme kamen mit Talent 500
    // an, ohne 995: Die abgelaufene CRUST hatte nachgelegt.
    let json = daten(NACHLAGE_ABLAUF);
    let (_, mit) = spielt(&json, |_| {});
    let (_, ohne) = spielt(&json, |c| c.shield_refund = false);
    assert!(
        mit.damage_taken > 0,
        "es kommt gar nichts an — der Vergleich misst nichts"
    );
    assert_eq!(
        mit.damage_taken, ohne.damage_taken,
        "mit Talent kommt weniger an — eine Umwandlungs-Schicht hat nachgelegt"
    );
}
