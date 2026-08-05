//! Gleichlauf ueber einen Lauf **mit getragener Ausruestung** (Issue #177, AC8).
//!
//! # Warum der bestehende Lauf nicht reicht
//!
//! `parity_recording.rs` spielt dieselbe Aufzeichnung ohne Ausruestung ab. Er
//! belegt damit AC7 — dass sich nichts bewegt — aber ueber die neuen Rechenwege
//! sagt er nichts: Keine Zeit wird umgerechnet, kein Wurf faellt.
//!
//! Eine Seite koennte den Tick-Abstand abrunden statt aufzurunden, oder den
//! Zufallsstrom einmal mehr drehen als die andere, und jener Lauf bliebe
//! **bitgleich**. Auffallen wuerde es erst im Spiel, bei einem Spieler mit
//! Ausruestung, und dann ohne jeden Hinweis darauf, welche Regel es war.
//!
//! Hier ist beides im Spiel:
//!
//! ```text
//! CATALYSIS 170 Promille   krumme Zwischenwerte, erzwingt die Rasterung
//! PURITY    375 Promille   zwischen beiden Kurzschluessen — es wird gewuerfelt
//! YIELD     250 Promille   Kritfaktor 1,75 statt 1,5
//! ```
//!
//! **Die Pruefsumme je Takt ist das Schaerfere am Vergleich.** Ein Endergebnis
//! kann zufaellig uebereinstimmen, waehrend der Weg dahin auseinanderlief.
//!
//! # Gemessen, nicht behauptet
//!
//! Am 2026-08-04 gegengeprueft, indem die Abklingzeit das Tempo wieder ignoriert
//! (`snap_up(now + spell.cooldown_ms)` statt `... + self.hasted(...)`):
//!
//! ```text
//! der_lauf_mit_ausruestung_ergibt_denselben_schaden        ok
//! beide_seiten_ziehen_gleich_oft_aus_dem_zufallsstrom      ok
//! die_pruefsumme_stimmt_in_jedem_einzelnen_takt            FAILED
//!   erste Abweichung in Takt 0 (0 ms)
//! ```
//!
//! **Nur die Pruefsumme faengt es.** Schaden, Sieg-Zeitpunkt und Ziehungen
//! bleiben unveraendert — in dieser Aufzeichnung ist die Wartezeit der Engpass,
//! nicht die Abklingzeit, und ein verschluckter Tempo-Wert dort bewegt keine
//! einzige Ergebniszahl. Ohne den taktweisen Vergleich waere der Fehler
//! unsichtbar geblieben, bis jemand mit anderem Zauberkasten spielt.

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::gear::{WornValues, CRIT_BASE_PERMILLE};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

mod common;
use common::float;

/// Der unveraenderte Inhalt des Laufs mit Ausruestung.
const WORN_JSON: &str = include_str!("../../tests/parity/recordings/worn-values.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(WORN_JSON).expect("worn-values.json ist kein JSON");
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

/// Die getragenen Werte **aus der Datei**, nicht aus dem Simulator.
///
/// Waeren sie hier abgetippt, pruefte der Vergleich nur, ob der Simulator mit
/// sich selbst uebereinstimmt — und eine Aenderung auf der Spielseite ginge
/// unbemerkt durch. Dieselbe Ueberlegung wie bei der Tastenbelegung in
/// `parity_recording.rs`.
fn worn(json: &Value) -> WornValues {
    let node = &json["worn"];
    WornValues {
        catalysis: u32_at(node, "catalysis"),
        purity: u32_at(node, "purity"),
        yield_bonus: u32_at(node, "yieldBonus"),
        essence: u32_at(node, "essence"),
        vessel: u32_at(node, "vessel"),
        ward: u32_at(node, "ward"),
        gear_effects: u32_at(node, "gearEffects"),
    }
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

fn replay_it(json: &Value, with_checksums: bool) -> (Recording, rota_sim::ReplayResult) {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        worn: worn(json),
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, with_checksums);
    (recording, result)
}

#[test]
fn die_beiden_abgeleiteten_werte_stimmen_bitgenau_ueberein() {
    // Kritchance und Kritfaktor sind Fliesskommazahlen und gehen in den Schaden
    // ein. Sie stehen deshalb als Bitmuster in der Datei — die Regel dieses
    // Verzeichnisses seit #122, weil `serde_json` manche `f64` um ein einziges
    // Bit falsch liest.
    let json = data();
    let getragen = worn(&json);

    assert_eq!(
        CRIT_BASE_PERMILLE,
        u32_at(&json, "critBasePermille"),
        "Spiel und Simulator gehen von verschiedenen Kritschaden-Basen aus"
    );
    // **Ueber das Bitmuster, nicht ueber die Dezimalschreibweise** (#122):
    // `serde_json` liest manche `f64` um ein einziges Bit falsch, und der Krit
    // geht in den Schaden ein.
    assert_eq!(getragen.crit_chance(), float(&json["critChanceBits"]));
    assert_eq!(getragen.crit_factor(), float(&json["critFactorBits"]));
    // **Topf 4** — seit der Nacharbeit an #177 Teil des Laufs. Ohne diese Zeile
    // liefe der neue Faktor durch die Pruefsumme, ohne je einzeln verglichen
    // worden zu sein.
    assert_eq!(
        getragen.gear_effect_factor(),
        float(&json["gearEffectFactorBits"])
    );
    assert!(
        getragen.gear_effect_factor() > 1.0,
        "Topf 4 ist neutral — dieser Lauf prueft ihn nicht"
    );

    // Und die Werte sind nicht zufaellig die neutralen: Ein Lauf mit
    // Kritchance 0 pruefte die halbe Datei nicht.
    assert!(getragen.crit_chance() > 0.0 && getragen.crit_chance() < 1.0);
    assert!(getragen.crit_factor() > 1.5);
}

#[test]
fn der_lauf_mit_ausruestung_ergibt_denselben_schaden() {
    let json = data();
    let (recording, result) = replay_it(&json, false);
    let expected = &json["expected"];

    assert_eq!(
        result.damage,
        u32_at(expected, "damage"),
        "der Simulator rechnet mit Ausruestung einen anderen Schaden als das Spiel"
    );
    assert_eq!(
        result.killed_at_ms.map(i64::from).unwrap_or(-1),
        expected["killedAtMs"].as_i64().expect("killedAtMs fehlt"),
        "der Boss faellt zu einer anderen Zeit"
    );
    assert_eq!(
        result.accepted,
        u32_at(expected, "accepted"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert!(
        !recording.presses.is_empty(),
        "eine Aufzeichnung ohne Tastendruecke prueft nichts"
    );
}

#[test]
fn beide_seiten_ziehen_gleich_oft_aus_dem_zufallsstrom() {
    // **Die Zahl, an der ein Krit-Fehler zuerst auffaellt.** Wer einmal mehr
    // oder weniger wuerfelt als die andere Seite, verschiebt jeden folgenden
    // Wurf — der Schaden liefe dann irgendwann auseinander, aber die Ursache
    // laege hundert Takte frueher.
    let json = data();
    let (_, result) = replay_it(&json, false);

    assert_eq!(
        result.random_draws,
        u32_at(&json["expected"], "randomDraws"),
        "verschieden viele Ziehungen — der Krit-Wurf faellt nicht an denselben Stellen"
    );
    // Und es wird ueberhaupt gezogen: Ein Lauf mit null Ziehungen pruefte am
    // Krit vorbei und saehe trotzdem gruen aus.
    assert!(
        result.random_draws > 0,
        "kein einziger Wurf — dieser Lauf prueft den Krit nicht"
    );
}

#[test]
fn die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    // **Der harte Pruefstein.** Ein Endergebnis sagt nur, DASS etwas nicht
    // stimmt; die Folge sagt, in welchem Takt — und damit fast immer auch,
    // welche Regel es war.
    let json = data();
    let (_, result) = replay_it(&json, true);
    let erwartet: Vec<u32> = json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl erwartet")).expect("passt nicht in 32 Bit"))
        .collect();

    assert_eq!(
        result.checksums.len(),
        erwartet.len(),
        "verschieden viele Takte"
    );
    assert!(erwartet.len() > 1, "eine einzige Pruefsumme prueft nichts");

    if let Some(d) = first_divergence(&erwartet, &result.checksums, TICK_MS) {
        panic!(
            "erste Abweichung in Takt {} ({} ms): Spiel {}, Simulator {} — {} Takte stimmten vorher",
            d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
        );
    }

    assert_eq!(
        result.final_checksum,
        u32_at(&json["expected"], "finalChecksum"),
        "Schluss-Pruefsumme"
    );
    assert_eq!(
        result.sequence_checksum,
        u32_at(&json["expected"], "sequenceChecksum"),
        "Folge-Pruefsumme"
    );
}

#[test]
fn dieser_lauf_unterscheidet_sich_wirklich_von_dem_ohne_ausruestung() {
    // **Der Gegenfall zur ganzen Datei.** Waeren die getragenen Werte
    // wirkungslos — etwa weil `worn` gar nicht bis in den Kampf durchgereicht
    // wird — kaeme derselbe Lauf heraus wie ohne Ausruestung, und alle
    // Pruefungen oben blieben gruen. Sie vergleichen nur Spiel und Simulator
    // miteinander, nicht gegen den Zustand davor.
    let json = data();
    let recording = recording(&json);

    let mit = Fight::new(FightConfig {
        seed: recording.seed,
        worn: worn(&json),
        ..FightConfig::default()
    })
    .replay(&recording, false);

    let ohne = Fight::new(FightConfig {
        seed: recording.seed,
        worn: WornValues::none(),
        ..FightConfig::default()
    })
    .replay(&recording, false);

    assert_ne!(
        mit.damage, ohne.damage,
        "die getragenen Werte bewegen den Schaden nicht"
    );
    assert_eq!(
        ohne.random_draws, 0,
        "ohne Ausruestung darf kein Wurf fallen"
    );
    assert!(mit.random_draws > 0, "mit PURITY muss gewuerfelt werden");
}

#[test]
fn essence_aus_der_ausruestung_wirkt_im_kampf() {
    // Issue #282, AC2. **Der Nachweis, dass die Leitung etwas transportiert.**
    // Ohne ihn belegte die Suite nur, dass ohne Ausrüstung nichts passiert —
    // und das war vorher auch schon so.
    use rota_sim::gear::WornValues;
    use rota_sim::{rotation, Fight, FightConfig};

    let ohne = Fight::new(FightConfig::default()).run(rotation::good);
    let mit = Fight::new(FightConfig {
        worn: WornValues {
            essence: 500, // +50 %
            ..WornValues::none()
        },
        ..FightConfig::default()
    })
    .run(rotation::good);

    // **Nicht der Schaden — die ZEIT.** Mein erster Entwurf verglich
    // `damage` und schlug fehl: 4230 gegen 4240, also WENIGER mit ESSENCE.
    // Kein Fehler in der Leitung, sondern in der Messung — der Lauf endet mit
    // dem Tod des Bosses, und wer härter schlägt, überschießt beim letzten
    // Treffer weniger. Der Gesamtschaden ist dann fast die Boss-HP, egal wie
    // stark der Spieler ist.
    //
    // `BalanceConfig` warnt vor genau dieser Verwechslung: `stop_on_kill`
    // misst, OB der Boss fällt, nicht WIE VIEL Schaden entsteht.
    let ohne_zeit = ohne.killed_at_ms.expect("ohne Ausruestung faellt der Boss");
    let mit_zeit = mit.killed_at_ms.expect("mit Ausruestung faellt der Boss");

    assert!(
        mit_zeit < ohne_zeit,
        "ESSENCE 500 aendert die Zeit bis zum Tod nicht: {mit_zeit} gegen \
         {ohne_zeit} ms — die Leitung transportiert nichts"
    );

    // **Und zwar in der richtigen Groessenordnung.** Ein Faktor, der wirkt,
    // aber falsch skaliert, waere oben ebenfalls gruen. +50 % Schaden heisst
    // etwa zwei Drittel der Zeit; grosszuegig geprueft, weil die Rotation in
    // Bloecken von 1500 ms arbeitet und der letzte Treffer ueberschiesst.
    let verhaeltnis = f64::from(mit_zeit) / f64::from(ohne_zeit);
    assert!(
        (0.6..=0.75).contains(&verhaeltnis),
        "ESSENCE 500 sollte die Zeit auf etwa zwei Drittel druecken, gemessen \
         {verhaeltnis:.3} ({mit_zeit} gegen {ohne_zeit} ms)"
    );
}

#[test]
fn vessel_kommt_zur_stufen_basis_dazu_und_ersetzt_sie_nicht() {
    // Issue #282, AC3. Zwei Aussagen, die einzeln beide zu schwach waeren.
    use rota_sim::data::{stat_at_level, PLAYER};
    use rota_sim::fight::vessel_with_gear;

    let basis_stufe_1 = stat_at_level(PLAYER.vessel, 1);
    let basis_stufe_100 = stat_at_level(PLAYER.vessel, 100);

    // Der Gegenfall zuerst: Ohne Ausruestung exakt die Basis, kein
    // Rundungsrest, kein Versatz (AC5).
    assert_eq!(vessel_with_gear(basis_stufe_1, 0), basis_stufe_1);
    assert_eq!(vessel_with_gear(basis_stufe_100, 0), basis_stufe_100);

    // Erstens: Ausruestung WIRKT.
    let mit = vessel_with_gear(basis_stufe_1, 500);
    assert!(
        mit > basis_stufe_1,
        "VESSEL 500 aendert nichts: {mit} gegen {basis_stufe_1}"
    );

    // Zweitens — und das ist die eigentliche Falle: Sie ersetzt die
    // Stufen-Basis NICHT. Ein Spieler auf Stufe 100 ohne Ausruestung muss
    // mehr Leben haben als einer auf Stufe 1 mit einem sehr guten Fund.
    //
    // Wer statt zu addieren ersetzt, faellt genau hier durch: Dann haengt das
    // Leben nur noch an der Ausruestung, und hundert Stufen sind umsonst.
    assert!(
        basis_stufe_100 > vessel_with_gear(basis_stufe_1, 1000),
        "Stufe 100 ohne Ausruestung ({basis_stufe_100}) ist nicht mehr als \
         Stufe 1 mit doppeltem VESSEL ({}) — die Stufen-Basis wurde ersetzt \
         statt erhoeht",
        vessel_with_gear(basis_stufe_1, 1000)
    );

    // Und die Basis waechst ueberhaupt mit der Stufe — sonst waere der
    // Vergleich oben trivial erfuellt.
    assert!(basis_stufe_100 > basis_stufe_1 * 10);

    // **Der Vergleich oben genuegt NICHT, und das ist gemessen.** Ich habe die
    // Funktion versuchsweise so gebrochen, dass sie die Basis ersetzt
    // (`worn_permille * 100` statt der Rechnung) — und alle Faelle oben
    // blieben gruen. Die Zahlen passten zufaellig: Stufe 100 hat so viel
    // Leben, dass sie auch gegen einen erfundenen Wert gewinnt.
    //
    // Was die Ersetzung wirklich ausschliesst, ist die Abhaengigkeit von der
    // BASIS: Dieselbe Ausruestung an zwei verschiedenen Basen muss zwei
    // verschiedene Ergebnisse geben, und zwar im Verhaeltnis der Basen.
    let klein = vessel_with_gear(basis_stufe_1, 500);
    let gross = vessel_with_gear(basis_stufe_100, 500);
    assert!(
        gross > klein,
        "dieselbe Ausruestung gibt an beiden Basen dasselbe ({gross} und \
         {klein}) — dann haengt das Ergebnis nicht an der Basis, die \
         Stufen-Basis ist ersetzt"
    );

    // Und zwar IM VERHAELTNIS der Basen, nicht bloss groesser. Ein Ergebnis,
    // das mit der Basis waechst, aber nach einer anderen Regel, kaeme sonst
    // durch.
    let verhaeltnis_basen = f64::from(basis_stufe_100) / f64::from(basis_stufe_1);
    let verhaeltnis_werte = f64::from(gross) / f64::from(klein);
    assert!(
        (verhaeltnis_basen - verhaeltnis_werte).abs() < 0.01,
        "Basen stehen {verhaeltnis_basen:.3} zueinander, Ergebnisse \
         {verhaeltnis_werte:.3} — die Ausruestung wirkt nicht als Faktor auf \
         die Basis"
    );
}

#[test]
fn ward_aus_der_ausruestung_mindert_zusaetzlich() {
    // Issue #282, AC4.
    use rota_sim::data::PLAYER;

    let roh = 1000;
    let ohne = PLAYER.incoming_with_gear(roh, 0);

    // Der Gegenfall zuerst (AC5): ohne Ausruestung exakt wie bisher.
    assert_eq!(ohne, PLAYER.incoming(roh));

    let mit = PLAYER.incoming_with_gear(roh, 300);
    assert!(mit < ohne, "WARD 300 mindert nichts: {mit} gegen {ohne}");

    // **Additiv, nicht multiplikativ** — und das ist der Unterschied, den ein
    // blosser Groessenvergleich nicht zeigt. Multiplikativ ergaeben Basis und
    // Zusatz ein anderes Ergebnis, und beide waeren "kleiner als ohne".
    let erwartet_additiv = (f64::from(roh) * (1.0 - (PLAYER.ward_reduction + 0.3)))
        .round()
        .max(0.0) as u32;
    assert_eq!(
        mit, erwartet_additiv,
        "WARD wirkt nicht additiv zur Basis — bei zwei Minderungen im selben \
         Topf wird addiert (13.1b)"
    );

    // Der Deckel: Bei voller Minderung null Schaden, nicht negativ und keine
    // Heilung. Das ist eine Rechenregel, keine Balance-Zahl.
    assert_eq!(PLAYER.incoming_with_gear(roh, 5000), 0);

    // Und der Gegenfall dazu: Ohne den Deckel waere das Ergebnis oben
    // ebenfalls 0 (als u32 abgeschnitten) — deshalb zusaetzlich knapp
    // darunter pruefen, wo die Rechnung noch etwas liefern muss.
    assert!(
        PLAYER.incoming_with_gear(roh, 700) > 0,
        "knapp unter dem Deckel darf nicht schon null herauskommen"
    );
}
