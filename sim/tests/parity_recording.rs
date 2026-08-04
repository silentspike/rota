//! **Der harte Pruefstein von Epic E2** (Issue #54, GAME-DESIGN 18.4).
//!
//! Eine im Spiel aufgezeichnete Runde wird im Simulator abgespielt. Beide Seiten
//! muessen auf denselben Schaden kommen — und auf dieselbe Zustands-Pruefsumme,
//! **Takt fuer Takt**.
//!
//! > Ohne diesen Abgleich waere die ganze Balance-Arbeit fuer ein Spiel gemacht,
//! > das es so nicht gibt.
//!
//! Die Vergleichsdatei entsteht aus dem Spielcode selbst
//! (`npx vite-node tests/parity/export-recording.ts`), nicht aus einer
//! abgetippten Liste: Die Pruefaelle sind ausdruecklich *aufgezeichnete echte
//! Durchlaeufe*.
//!
//! # Warum die Pruefsumme je Takt und nicht nur am Ende
//!
//! Ein Endergebnis sagt nur, **dass** etwas nicht stimmt. Bei 4.052 Takten waere
//! die Suche danach ein Nachmittag. Die Folge der Pruefsummen sagt, **in welchem
//! Takt** es auseinanderlaeuft — und damit fast immer auch, welche Regel es war.

use std::collections::BTreeMap;

use serde_json::Value;

use rota_sim::data::index_of;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der unveraenderte Inhalt der aufgezeichneten Runde.
const RECORDING_JSON: &str = include_str!("../../tests/parity/recordings/cooldown-precision.json");

fn data() -> Value {
    let value: Value =
        serde_json::from_str(RECORDING_JSON).expect("cooldown-precision.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung der Aufzeichnung"
    );
    value
}

/// Baut die Aufzeichnung aus der Datei.
///
/// Die Belegung kommt aus der Datei und nicht aus dem Simulator: Ein Tastendruck
/// nennt einen Platz, keinen Zauber, und die Belegung ist frei waehlbar
/// (GAME-DESIGN 8.1). Die Datei bleibt dadurch selbsttragend.
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

fn expected_checksums(json: &Value) -> Vec<u32> {
    json["checksums"]
        .as_array()
        .expect("checksums fehlen")
        .iter()
        .map(|value| {
            u32::try_from(value.as_u64().expect("Zahl erwartet")).expect("passt nicht in 32 Bit")
        })
        .collect()
}

fn replay_it(json: &Value, with_checksums: bool) -> (Recording, rota_sim::ReplayResult) {
    let recording = recording(json);
    let mut fight = Fight::new(FightConfig {
        seed: recording.seed,
        ..FightConfig::default()
    });
    let result = fight.replay(&recording, with_checksums);
    (recording, result)
}

#[test]
fn ac3_die_aufgezeichnete_runde_ergibt_denselben_schaden() {
    let json = data();
    let (recording, result) = replay_it(&json, false);
    let expected = &json["expected"];

    assert_eq!(
        result.damage,
        u32::try_from(expected["damage"].as_u64().expect("damage fehlt")).expect("zu gross"),
        "der Simulator rechnet einen anderen Schaden als das Spiel"
    );

    let killed = expected["killedAtMs"].as_i64().expect("killedAtMs fehlt");
    assert_eq!(
        result.killed_at_ms.map(i64::from).unwrap_or(-1),
        killed,
        "der Boss faellt zu einer anderen Zeit"
    );

    // Ohne diese Zeile bestuende der Test auch dann, wenn jeder Tastendruck
    // abgelehnt und trotzdem zufaellig derselbe Schaden erreicht worden waere.
    assert_eq!(
        result.accepted,
        u32::try_from(expected["accepted"].as_u64().expect("accepted fehlt")).expect("zu gross"),
        "andere Zahl angenommener Tastendruecke"
    );
    assert_eq!(
        result.rejected,
        u32::try_from(expected["rejected"].as_u64().expect("rejected fehlt")).expect("zu gross"),
        "andere Zahl abgelehnter Tastendruecke"
    );
    assert!(
        !recording.presses.is_empty(),
        "eine Aufzeichnung ohne Tastendruecke prueft nichts"
    );
}

#[test]
fn ac3_jeder_zauber_wurde_gleich_oft_gewirkt() {
    let json = data();
    let (_, result) = replay_it(&json, false);
    let expected: BTreeMap<String, u64> =
        serde_json::from_value(json["expected"]["casts"].clone()).expect("casts fehlen");

    for (spell_id, count) in expected {
        let index = index_of(&spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt"));
        assert_eq!(
            u64::from(result.casts[index]),
            count,
            "{spell_id} wurde anders oft gewirkt"
        );
    }
}

#[test]
fn ac3_keine_ziehung_aus_den_zufallsstroemen() {
    // `STEP1-SPEC` § 2: kein Krit in Schritt 1. Eine einzige Ziehung wuerde die
    // Rechnung stillschweigend von exakt auf gemittelt umstellen.
    let json = data();
    let (_, result) = replay_it(&json, false);
    assert_eq!(result.random_draws, 0);
    assert_eq!(
        json["expected"]["randomDraws"].as_u64(),
        Some(0),
        "das Spiel hat gezogen, der Simulator nicht — oder umgekehrt"
    );
}

#[test]
fn ac4_die_pruefsumme_stimmt_in_jedem_einzelnen_takt() {
    let json = data();
    let (_, result) = replay_it(&json, true);
    let expected = expected_checksums(&json);

    assert!(
        !expected.is_empty(),
        "die Vergleichsdatei enthaelt keine Pruefsummen"
    );

    // Das ist der eigentliche Nachweis von AC4: Weicht irgendetwas ab, nennt die
    // Meldung den Takt — nicht bloss die Tatsache.
    if let Some(d) = first_divergence(&expected, &result.checksums, TICK_MS) {
        panic!(
            "Spiel und Simulator laufen auseinander:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Spiel:     {}\n  \
             Simulator: {}\n  \
             uebereinstimmende Takte davor: {}\n  \
             Laenge: Spiel {} Takte, Simulator {} Takte",
            d.tick,
            d.time_ms,
            d.expected,
            d.actual,
            d.matched_ticks,
            expected.len(),
            result.checksums.len()
        );
    }

    assert_eq!(
        result.checksums.len(),
        expected.len(),
        "unterschiedlich viele Takte gerechnet"
    );
}

#[test]
fn ac4_die_zusammenfassenden_pruefsummen_stimmen() {
    // Die Folge-Summe ist der schnelle Vergleich: ein Wert statt viertausend.
    // Sie ersetzt die Takt-Folge nicht — sie sagt nur, ob man ueberhaupt suchen muss.
    let json = data();
    let (_, result) = replay_it(&json, true);
    let expected = &json["expected"];

    assert_eq!(
        u64::from(result.sequence_checksum),
        expected["sequenceChecksum"]
            .as_u64()
            .expect("sequenceChecksum fehlt"),
        "die Folge-Pruefsumme weicht ab"
    );
    assert_eq!(
        u64::from(result.final_checksum),
        expected["finalChecksum"]
            .as_u64()
            .expect("finalChecksum fehlt"),
        "die Pruefsumme des Endzustands weicht ab"
    );
}

#[test]
fn ac4_eine_kuenstliche_abweichung_wird_mit_takt_gemeldet() {
    // Gegenprobe: Ein Vergleich, der nie etwas findet, ist wertlos. Hier wird
    // eine Abweichung eingebaut und geprueft, dass die Meldung den richtigen
    // Takt nennt.
    let json = data();
    let (_, result) = replay_it(&json, true);
    let mut verfaelscht = result.checksums.clone();
    let stelle = verfaelscht.len() / 3;
    verfaelscht[stelle] ^= 1;

    let d = first_divergence(&result.checksums, &verfaelscht, TICK_MS)
        .expect("die eingebaute Abweichung wurde nicht gefunden");
    assert_eq!(d.tick as usize, stelle);
    assert_eq!(d.time_ms, stelle as u32 * TICK_MS);
    assert_eq!(d.matched_ticks as usize, stelle);
}

#[test]
fn die_aufgezeichnete_runde_entspricht_der_referenz() {
    // Die Aufzeichnung ist die gute Rotation, einmal mitgeschrieben. Sie muss
    // deshalb dieselben Werte ergeben wie `tools/step1-check.py`
    // (`STEP1-SPEC` § 4) — sonst hat das Aufzeichnen selbst etwas veraendert.
    let json = data();
    let (_, result) = replay_it(&json, false);
    assert_eq!(result.damage, 4240);
    assert_eq!(result.killed_at_ms, Some(40_510));
}
