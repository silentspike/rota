//! Gleichlauf der **Bausteine**: Zufallsstroeme und Pruefsumme.
//!
//! Geprueft wird gegen `tests/parity/recordings/primitives.json` — eine Datei,
//! die aus dem Spielcode selbst erzeugt wird
//! (`npx vite-node tests/parity/export-primitives.ts`).
//!
//! # Warum das ein eigener Pruefall ist
//!
//! Ein Kampf-Prueffall wuerde diese Fehler in Schritt 1 **nicht** finden. Auf
//! Stufe 1 ist PURITY = 0, es gibt keinen Krit und keine Heilung — also zieht
//! niemand aus den Zufallsstroemen (`STEP1-SPEC` § 2). Ein Simulator mit einem
//! voellig anderen Erzeuger besteht heute jeden Kampf-Prueffall und laeuft ab
//! Schritt 3 mit dem ersten Krit auseinander. Dann steht die Suche am falschen
//! Ende: im Kampfablauf, wo der Fehler nicht ist.
//!
//! Genau das ist beim Bau von E2-2 aufgefallen: Der Simulator hatte zunaechst
//! SplitMix64, das Spiel hat mulberry32. Diese Datei haette es sofort gezeigt.

use serde_json::Value;

mod common;
use common::float;

use rota_sim::checksum::{mix_float, mix_int, FNV_OFFSET};
use rota_sim::rng::{RandomStream, Streams, STREAM_COUNT};
use rota_sim::Stream;

/// Der unveraenderte Inhalt der Vergleichsdatei.
const PRIMITIVES: &str = include_str!("../../tests/parity/recordings/primitives.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(PRIMITIVES).expect("primitives.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von primitives.json"
    );
    value
}

fn u32_at(node: &Value) -> u32 {
    u32::try_from(node.as_u64().expect("Zahl erwartet")).expect("passt nicht in 32 Bit")
}

#[test]
fn die_stroeme_heissen_wie_im_spiel() {
    // Die Reihenfolge ist Teil der Schnittstelle: Sie bestimmt, an welcher
    // Stelle die Ziehungszaehler in die Zustands-Pruefsumme eingehen.
    let json = data();
    let names = json["streamNames"].as_array().expect("streamNames fehlt");
    assert_eq!(names.len(), STREAM_COUNT);
    for (index, stream) in Stream::ALL.iter().enumerate() {
        assert_eq!(
            stream.name(),
            names[index].as_str().expect("Name erwartet"),
            "Strom {index} heisst anders als im Spiel"
        );
    }
}

#[test]
fn die_startwerte_der_stroeme_stimmen() {
    // Stimmt schon die Ableitung nicht, ist jede weitere Ziehung sinnlos — und
    // der Fehler liegt dann nicht im Erzeuger, sondern eine Ebene darueber.
    let json = data();
    for case in json["streams"].as_array().expect("streams fehlt") {
        let seed = u32_at(&case["seed"]);
        let expected = case["streamSeeds"].as_array().expect("streamSeeds fehlt");
        let streams = Streams::new(seed);
        for (index, stream) in Stream::ALL.iter().enumerate() {
            assert_eq!(
                streams.stream_seed(*stream),
                u32_at(&expected[index]),
                "Startwert von Strom {} bei seed {seed}",
                stream.name()
            );
        }
    }
}

#[test]
fn die_ziehungen_stimmen_mit_dem_spiel_ueberein() {
    let json = data();
    for case in json["streams"].as_array().expect("streams fehlt") {
        let seed = u32_at(&case["seed"]);
        let draws = &case["draws"];
        for stream in Stream::ALL {
            let expected = draws[stream.name()]
                .as_array()
                .unwrap_or_else(|| panic!("Ziehungen fuer {} fehlen", stream.name()));

            // Je Strom ein frischer Satz: Im Spiel zieht das Werkzeug ebenfalls
            // erst alle Werte des einen Stroms, dann die des naechsten.
            let mut streams = Streams::new(seed);
            let actual: Vec<u32> = (0..expected.len())
                .map(|_| streams.next_u32(stream))
                .collect();
            let expected: Vec<u32> = expected.iter().map(u32_at).collect();

            assert_eq!(
                actual,
                expected,
                "Strom {} bei seed {seed} liefert eine andere Folge als das Spiel",
                stream.name()
            );
        }
    }
}

#[test]
fn ziehungen_liegen_zwischen_null_und_eins() {
    // Das Spiel gibt `wert / 4294967296` zurueck. Der Simulator rechnet mit der
    // rohen Zahl; diese Umrechnung muss trotzdem dieselbe Spanne treffen.
    let mut stream = RandomStream::new(12345);
    for _ in 0..1000 {
        let value = stream.next_f64();
        assert!((0.0..1.0).contains(&value));
    }
}

#[test]
fn mix_int_stimmt_mit_dem_spiel_ueberein() {
    let json = data();
    for case in json["checksum"]["mixInt"].as_array().expect("mixInt fehlt") {
        let value = i32::try_from(case["value"].as_i64().expect("Zahl erwartet"))
            .expect("passt nicht in 32 Bit");
        let expected = u32_at(&case["result"]);
        assert_eq!(
            mix_int(FNV_OFFSET, value),
            expected,
            "mix_int({value}) weicht ab — meist ein Vorzeichen- oder Schiebe-Fehler"
        );
    }
}

#[test]
fn mix_float_stimmt_mit_dem_spiel_ueberein() {
    let json = data();
    for case in json["checksum"]["mixFloat"]
        .as_array()
        .expect("mixFloat fehlt")
    {
        // Ueber das Bitmuster, nicht ueber `as_f64()` (Issue #122).
        let value = float(&case["valueBits"]);
        let expected = u32_at(&case["result"]);
        assert_eq!(
            mix_float(FNV_OFFSET, value),
            expected,
            "mix_float({value}) weicht ab — meist die Reihenfolge der beiden 32-Bit-Woerter"
        );
    }
}

#[test]
fn der_anfangswert_stimmt() {
    let json = data();
    assert_eq!(u32_at(&json["checksum"]["offset"]), FNV_OFFSET);
}

#[test]
fn eine_kette_aus_beidem_stimmt() {
    // Der Einzelschritt kann stimmen und die Kette trotzdem nicht — etwa, wenn
    // eine Umsetzung zwischendurch auf 32 Bit kuerzt und die andere nicht.
    let json = data();
    let expected = u32_at(&json["checksum"]["chain"]);
    let mut h = FNV_OFFSET;
    for value in [1, 2, 3, 4, 5] {
        h = mix_int(h, value);
    }
    for value in [1.5, 2.5] {
        h = mix_float(h, value);
    }
    assert_eq!(h, expected);
}
