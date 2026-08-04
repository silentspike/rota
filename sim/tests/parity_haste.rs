//! Gleichlauf der **Tempo-Umrechnung** (GAME-DESIGN 6.7, Issue #177).
//!
//! Geprueft wird gegen `tests/parity/recordings/haste.json`, erzeugt aus dem
//! Spielcode selbst (`npx vite-node tests/parity/export-parity.mjs`).
//!
//! # Warum das ein eigener Prueffall ist
//!
//! Dieselbe Ueberlegung wie bei `parity_primitives.rs`. Ein Kampf ohne
//! getragene Ausruestung hat CATALYSIS = 0 und laeuft **nie** durch diese
//! Rechnung. Eine Seite koennte abrunden statt aufzurunden, eine Seite koennte
//! das Takt-Raster vergessen — und jeder heutige Kampf-Prueffall bliebe gruen.
//!
//! Auffallen wuerde es erst mit dem ersten getragenen Gegenstand. Dann stuende
//! die Suche am falschen Ende: im Kampfablauf, wo der Fehler nicht ist.
//!
//! Das ist kein hypothetischer Fall. Bis zu diesem Issue hatte das **Spiel gar
//! kein `snapUp`** — nur der Simulator hatte es. Solange jede Abklingzeit ganze
//! Sekunden trug (6.4), fiel das nicht auf; mit dem ersten Tempo-Wert waere der
//! Simulator um bis zu einen Takt je Zauber schneller gelaufen als das Spiel.

use serde_json::Value;

use rota_sim::checksum::{mix_int, FNV_OFFSET};
use rota_sim::haste::{hasted_ms, tick_count};
use rota_sim::time::{Millis, TICK_MS};

/// Der unveraenderte Inhalt der Vergleichsdatei.
const HASTE: &str = include_str!("../../tests/parity/recordings/haste.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(HASTE).expect("haste.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von haste.json"
    );
    value
}

fn millis_at(node: &Value, key: &str) -> Millis {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    Millis::try_from(raw).expect("passt nicht in 32 Bit")
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

#[test]
fn der_takt_ist_auf_beiden_seiten_derselbe() {
    // Die ganze Umrechnung rastert auf ihn. Waere er verschieden, stimmte
    // keine einzige Zeit ueberein — und alle folgenden Pruefungen zeigten auf
    // die Umrechnung statt auf die Ursache.
    let json = data();
    assert_eq!(
        millis_at(&json, "tickMs"),
        TICK_MS,
        "Spiel und Simulator rechnen mit verschiedenen Logik-Takten"
    );
}

#[test]
fn jede_umrechnung_ergibt_dieselbe_zahl_wie_im_spiel() {
    let json = data();
    let steps = json["steps"].as_array().expect("steps fehlt");
    assert!(!steps.is_empty(), "haste.json enthaelt keine Schritte");

    for step in steps {
        let base = millis_at(step, "baseMs");
        let permille = u32_at(step, "permille");
        let erwartet = millis_at(step, "hastedMs");
        let gerechnet = hasted_ms(base, permille);

        assert_eq!(
            gerechnet, erwartet,
            "{base} ms bei +{permille} Promille: Simulator {gerechnet}, Spiel {erwartet}"
        );

        // Die Pruefsumme faengt zusaetzlich den Fall ab, dass jemand die
        // Vergleichsdatei von Hand nachbessert, damit sie wieder passt.
        let mut hash = FNV_OFFSET;
        hash = mix_int(hash, base as i32);
        hash = mix_int(hash, permille as i32);
        hash = mix_int(hash, gerechnet as i32);
        assert_eq!(
            hash,
            u32_at(step, "checksum"),
            "Pruefsumme bei {base} ms und +{permille}"
        );
    }
}

#[test]
fn die_vergleichsdatei_enthaelt_den_fall_ohne_tempo() {
    // **Der Gegenfall zur Datei selbst.** Ohne ihn koennte sie ausschliesslich
    // Faelle mit Tempo enthalten — dann pruefte sie zwar die Rechnung, aber
    // nicht die Zusicherung aus AC7, dass ohne Tempo nichts geschieht.
    let json = data();
    let steps = json["steps"].as_array().expect("steps fehlt");

    let ohne: Vec<&Value> = steps
        .iter()
        .filter(|s| u32_at(s, "permille") == 0 && millis_at(s, "baseMs") > 0)
        .collect();
    assert!(
        !ohne.is_empty(),
        "haste.json enthaelt keinen Fall ohne Tempo — AC7 waere ungeprueft"
    );
    for step in ohne {
        let base = millis_at(step, "baseMs");
        assert_eq!(
            millis_at(step, "hastedMs"),
            base,
            "{base} ms ohne Tempo hat sich bewegt"
        );
    }

    // Und der Gegenfall zum Gegenfall: Es muss auch Faelle MIT Tempo geben, in
    // denen sich etwas bewegt. Sonst waere die Datei mit lauter Nullen erfuellt.
    let bewegt = steps.iter().any(|s| {
        let base = millis_at(s, "baseMs");
        base > 0 && millis_at(s, "hastedMs") < base
    });
    assert!(
        bewegt,
        "keine einzige Zeit wird kuerzer — die Datei prueft nichts"
    );
}

#[test]
fn die_dots_behalten_ihre_dauer_und_bekommen_mehr_ticks() {
    let json = data();
    let steps = json["dotSteps"].as_array().expect("dotSteps fehlt");
    assert!(!steps.is_empty(), "keine DoT-Schritte");

    for step in steps {
        let dauer = millis_at(step, "durationMs");
        let basis = millis_at(step, "baseIntervalMs");
        let permille = u32_at(step, "permille");
        let abstand = hasted_ms(basis, permille);
        let ticks = tick_count(dauer, abstand);

        assert_eq!(
            abstand,
            millis_at(step, "intervalMs"),
            "Tick-Abstand bei {basis} ms und +{permille}"
        );
        assert_eq!(
            ticks,
            u32_at(step, "ticks"),
            "Tick-Zahl bei {basis} ms und +{permille}"
        );
        // **Kein anteiliger letzter Tick** (6.7): Was nicht mehr in die Dauer
        // passt, verfaellt — und beide Seiten muessen dasselbe verfallen lassen.
        assert_eq!(
            dauer - ticks * abstand,
            millis_at(step, "restMs"),
            "verfallener Rest bei {basis} ms und +{permille}"
        );

        let mut hash = FNV_OFFSET;
        hash = mix_int(hash, dauer as i32);
        hash = mix_int(hash, abstand as i32);
        hash = mix_int(hash, ticks as i32);
        assert_eq!(
            hash,
            u32_at(step, "checksum"),
            "Pruefsumme des DoT-Schritts"
        );
    }
}

#[test]
fn mehr_tempo_ergibt_in_der_datei_mehr_ticks() {
    // Die eigentliche Aussage aus 6.7 — als Ungleichung ueber die Datei, nicht
    // gegen abgetippte Zahlen. Waere sie falsch herum umgesetzt, blieben alle
    // Pruefungen oben trotzdem gruen: Sie vergleichen nur, ob beide Seiten
    // dasselbe rechnen, nicht ob es das Richtige ist.
    let json = data();
    let steps = json["dotSteps"].as_array().expect("dotSteps fehlt");

    for basis in [1000, 2000, 3000] {
        let reihe: Vec<(u32, u32)> = steps
            .iter()
            .filter(|s| millis_at(s, "baseIntervalMs") == basis)
            .map(|s| (u32_at(s, "permille"), u32_at(s, "ticks")))
            .collect();
        assert!(reihe.len() > 1, "zu wenige Schritte fuer {basis} ms");

        for fenster in reihe.windows(2) {
            let (tempo_a, ticks_a) = fenster[0];
            let (tempo_b, ticks_b) = fenster[1];
            assert!(
                tempo_b > tempo_a,
                "die Datei ist nicht nach Tempo geordnet: {tempo_a} vor {tempo_b}"
            );
            assert!(
                ticks_b >= ticks_a,
                "{basis} ms: +{tempo_b} ergab {ticks_b} Ticks, +{tempo_a} schon {ticks_a}"
            );
        }
        // Und der Vergleichsfall: Ueber die ganze Reihe muss es wirklich mehr
        // werden. Ein "nie weniger" allein waere mit lauter gleichen Zahlen
        // erfuellt.
        assert!(
            reihe.last().expect("Reihe leer").1 > reihe[0].1,
            "{basis} ms: Tempo bringt ueberhaupt keinen Tick mehr"
        );
    }
}
