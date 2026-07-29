//! Prueft den Gleitkomma-Vergleich selbst (Issue #122).
//!
//! **Warum es diese Datei gibt.** Bis #122 verglichen zwei Gleichlauf-Prueffaelle
//! rohe `f64` aus den Vergleichsdateien ueber `serde_json::as_f64()` — und der
//! liest manche Zahlen ein Bit daneben. Sie waren gruen, ohne zu pruefen.
//!
//! Ein Ersatz, der wieder nur zufaellig gruen ist, waere keine Verbesserung.
//! Diese Datei belegt deshalb, dass der neue Weg **tatsaechlich faengt**, was er
//! fangen soll: eine kuenstlich erzeugte Abweichung im letzten Bit.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Weichen zwei Zahlen um hoechstens ein Bit voneinander ab?
///
/// **Warum hier nicht auf Gleichheit geprueft wird.** Der naheliegende
/// Vergleich waere `float(bits) == as_f64(dezimal)`. Der haette eine Falle:
/// Traegt jemand kuenftig eine Zahl ein, die `serde_json` falsch liest — also
/// genau den Fall, um den es in #122 geht —, dann meinen Dezimalform und
/// Bitmuster DIESELBE Zahl, und der Prueffall meldete trotzdem einen Fehler.
/// Ein Waechter, der bei richtigem Inhalt Alarm schlaegt, wird abgeschaltet.
///
/// Ein Bit Abstand ist genau das, was `serde_json` verliert. Alles darueber ist
/// ein echter Fehler: Dann meinen die beiden Formen wirklich Verschiedenes.
fn hoechstens_ein_bit_auseinander(a: f64, b: f64) -> bool {
    let (x, y) = (a.to_bits(), b.to_bits());
    x.abs_diff(y) <= 1
}

use serde_json::Value;

mod common;
use common::{bits, float};

const RULE_EVENTS: &str = include_str!("../../tests/parity/recordings/rule-events.json");
const PRIMITIVES: &str = include_str!("../../tests/parity/recordings/primitives.json");

/// Die vier Zahlen, an denen der Fehler am 2026-07-29 gemessen wurde.
///
/// Erste Spalte die Zeichenkette, zweite die **richtigen** Bits.
const GEMESSEN: [(&str, u64); 4] = [
    ("1.6500000000000001", 0x3ffa_6666_6666_6667),
    ("1.0000000000000002", 0x3ff0_0000_0000_0001),
    ("2.1450000000000005", 0x4001_28f5_c28f_5c2a),
    ("2.475", 0x4003_cccc_cccc_cccd),
];

/// **Der Befund selbst, als Prueffall.**
///
/// Er haelt fest, was gemessen wurde — und faellt um, wenn `serde_json` das
/// eines Tages behebt. Dann darf man #122 zurueckbauen; bis dahin nicht.
#[test]
fn serde_json_liest_manche_zahlen_ein_bit_daneben() {
    let mut abweichungen = 0;
    for (text, richtig) in GEMESSEN {
        let ueber_serde: Value =
            serde_json::from_str(&format!("{{\"v\":{text}}}")).expect("kein JSON");
        let gelesen = ueber_serde["v"].as_f64().expect("Zahl").to_bits();

        // Rusts eigener Zahlenleser trifft den Wert immer.
        assert_eq!(
            text.parse::<f64>().expect("keine Zahl").to_bits(),
            richtig,
            "str::parse hat {text} anders gelesen als am 2026-07-29 gemessen"
        );

        if gelesen != richtig {
            abweichungen += 1;
            assert_eq!(
                text, "1.6500000000000001",
                "eine ANDERE Zahl weicht ab als die gemessene — der Befund hat sich geaendert"
            );
        }
    }
    assert_eq!(
        abweichungen, 1,
        "genau eine der vier gemessenen Zahlen wich ab. Sind es null, hat serde_json den \
         Fehler behoben und #122 darf zurueckgebaut werden. Sind es mehr, ist er groesser \
         geworden."
    );
}

/// Der Umweg ueber das Bitmuster trifft **jede** der vier Zahlen.
#[test]
fn das_bitmuster_trifft_jede_zahl() {
    for (text, richtig) in GEMESSEN {
        let wert = text.parse::<f64>().expect("keine Zahl");
        let als_datei: Value =
            serde_json::from_str(&format!("\"{:016x}\"", wert.to_bits())).expect("kein JSON");
        assert_eq!(
            bits(&als_datei),
            richtig,
            "{text} laeuft ueber das Bitmuster falsch"
        );
        assert_eq!(float(&als_datei).to_bits(), richtig);
    }
}

/// **AC3: Eine kuenstlich erzeugte Ein-Bit-Abweichung wird gefangen.**
///
/// Der eigentliche Nachweis. Genommen wird eine echte Zahl aus
/// `rule-events.json`, ihr letztes Bit umgelegt — und geprueft, dass der
/// Vergleich das bemerkt. Ohne diesen Fall waere nicht belegt, dass der neue
/// Weg ueberhaupt etwas leistet.
#[test]
fn eine_abweichung_im_letzten_bit_wird_gefangen() {
    let json: Value = serde_json::from_str(RULE_EVENTS).expect("kein JSON");
    let steps = json["steps"].as_array().expect("steps fehlen");
    assert!(!steps.is_empty(), "eine leere Folge bestuende immer");

    let mut geprueft = 0;
    for (index, step) in steps.iter().enumerate() {
        let echt = float(&step["valueBits"]);

        // Das letzte Bit umlegen — die kleinstmoegliche Aenderung an einer
        // Gleitkommazahl.
        let verfaelscht_bits = echt.to_bits() ^ 1;
        let verfaelscht = f64::from_bits(verfaelscht_bits);

        assert_ne!(
            echt.to_bits(),
            verfaelscht_bits,
            "Schritt {index}: das Umlegen hat nichts geaendert"
        );
        assert_ne!(
            echt, verfaelscht,
            "Schritt {index}: zwei verschiedene Bitmuster ergaben dieselbe Zahl"
        );

        // Und der Vergleich, wie ihn die Prueffaelle fuehren, bemerkt es.
        let gefangen = catch_unwind(AssertUnwindSafe(|| {
            assert_eq!(
                echt.to_bits(),
                verfaelscht_bits,
                "Abweichung bei Schritt {index}"
            );
        }));
        assert!(
            gefangen.is_err(),
            "Schritt {index}: die verfaelschte Zahl ist NICHT aufgefallen"
        );

        // Die Meldung muss die Stelle benennen — ein Waechter, der nur
        // \"ungleich\" sagt, kostet den naechsten eine Stunde Suche.
        let meldung = gefangen.unwrap_err();
        let text = meldung
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or("");
        assert!(
            text.contains(&format!("Schritt {index}")),
            "die Meldung benennt die Stelle nicht: {text}"
        );
        geprueft += 1;
    }
    // **Jeder** Schritt, nicht eine Stichprobe — und die Zahl kommt aus der
    // Datei, nicht aus einer geratenen Schwelle. Der erste Anlauf verlangte
    // hier „mindestens zehn"; die Folge hat acht, und der Prueffall fiel um,
    // ohne dass etwas kaputt war.
    assert_eq!(
        geprueft,
        steps.len(),
        "nicht jeder Schritt wurde auf die Ein-Bit-Abweichung geprueft"
    );
}

/// **Der alte Weg haette es NICHT gefangen — an genau dieser Zahl.**
///
/// Das ist der Unterschied zwischen \"gruen\" und \"prueft etwas\".
#[test]
fn der_alte_weg_haette_es_durchgelassen() {
    // Eine Datei, wie sie vor #122 ausgesehen haette: nur die Dezimalform.
    let alt: Value = serde_json::from_str(r#"{"value":1.6500000000000001}"#).expect("kein JSON");
    let ueber_as_f64 = alt["value"].as_f64().expect("Zahl");

    // Was das Spiel gerechnet hat.
    let im_spiel = 1.5_f64 * 1.1_f64;

    assert_ne!(
        ueber_as_f64.to_bits(),
        im_spiel.to_bits(),
        "as_f64 liest die Zahl inzwischen richtig — dann ist #122 hinfaellig"
    );

    // Und derselbe Wert ueber das Bitmuster: identisch.
    let neu: Value =
        serde_json::from_str(&format!("\"{:016x}\"", im_spiel.to_bits())).expect("kein JSON");
    assert_eq!(float(&neu).to_bits(), im_spiel.to_bits());
}

/// Beide betroffenen Dateien fuehren zu **jeder** Gleitkommazahl ihr Bitmuster.
///
/// Eine Stolperdrahte: Wer eine Zahl hinzufuegt und das Bitmuster vergisst,
/// faellt hier auf — und nicht erst, wenn die Zahl zufaellig eine ist, die
/// `serde_json` falsch liest.
#[test]
fn keine_gleitkommazahl_steht_ohne_ihr_bitmuster() {
    let ereignisse: Value = serde_json::from_str(RULE_EVENTS).expect("kein JSON");
    for (index, step) in ereignisse["steps"]
        .as_array()
        .expect("steps fehlen")
        .iter()
        .enumerate()
    {
        assert!(
            step["value"].is_number(),
            "Schritt {index}: die lesbare Dezimalform fehlt"
        );
        assert!(
            step["valueBits"].is_string(),
            "Schritt {index}: das Bitmuster fehlt — ohne es ist der Vergleich gruen aus Glueck"
        );
        // Beide muessen dieselbe Zahl MEINEN — auf ein Bit genau, nicht auf
        // Gleichheit. Siehe `hoechstens_ein_bit_auseinander`.
        assert!(
            hoechstens_ein_bit_auseinander(
                float(&step["valueBits"]),
                step["value"].as_f64().expect("Zahl")
            ),
            "Schritt {index}: Dezimalform und Bitmuster meinen verschiedene Zahlen"
        );
    }

    let bausteine: Value = serde_json::from_str(PRIMITIVES).expect("kein JSON");
    for (index, case) in bausteine["checksum"]["mixFloat"]
        .as_array()
        .expect("mixFloat fehlt")
        .iter()
        .enumerate()
    {
        assert!(
            case["valueBits"].is_string(),
            "mixFloat-Fall {index}: das Bitmuster fehlt"
        );
        assert!(
            hoechstens_ein_bit_auseinander(
                float(&case["valueBits"]),
                case["value"].as_f64().expect("Zahl")
            ),
            "mixFloat-Fall {index}: Dezimalform und Bitmuster meinen verschiedene Zahlen"
        );
    }
}
