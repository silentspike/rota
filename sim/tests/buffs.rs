//! Gleichlauf der Stapel-Regel (Issue E3-5, AC5).
//!
//! **Dieser Abgleich ist heute schon aussagekraeftig** — anders als der bei der
//! Sublimation (#104), der offen bleiben musste. Der Grund ist der Gegenstand:
//! Die Stapel-Regel haengt an keinem Zauber und an keinen Daten, die es noch
//! nicht gibt. Sie laesst sich deshalb vollstaendig gegenrechnen.
//!
//! Verglichen wird ueber die **Pruefsumme** des Faktors, nicht ueber die Zahl:
//! Ein Faktor wie 1,2 mal 1,5 ist in JSON nicht verlustfrei zu schreiben, seine
//! Bit-Darstellung schon. Und die Pruefsumme faellt auch dann auseinander, wenn
//! sich beide Seiten nur im letzten Bit unterscheiden — genau darum geht es,
//! weil Fliesskomma-Multiplikation nicht assoziativ ist.

use serde_json::Value;

use rota_sim::buffs::{Buffs, BUFF_KIND_CAPACITY};
use rota_sim::checksum::{mix_float, FNV_OFFSET};
use rota_sim::time::Millis;

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const BUFFS: &str = include_str!("../../tests/parity/recordings/buffs.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(BUFFS).expect("buffs.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von buffs.json"
    );
    value
}

#[test]
fn der_vorrat_ist_auf_beiden_seiten_gleich_gross() {
    // Die Zahl steht in der Datei, weil sie bestimmt, wie viele Arten
    // nebeneinander stehen koennen — und damit, welche Faktoren ins Produkt
    // eingehen.
    let json = data();
    assert_eq!(
        json["kindCapacity"].as_u64(),
        Some(BUFF_KIND_CAPACITY as u64)
    );
}

#[test]
fn die_wartezeit_ist_auf_beiden_seiten_gleich_lang() {
    let json = data();
    assert_eq!(json["gcdMs"].as_u64(), Some(u64::from(rota_sim::GCD_MS)));
}

/// **Dieselbe Folge, derselbe Faktor — nach JEDEM Schritt.**
///
/// Schritt fuer Schritt und nicht nur am Ende, weil nur das sagt, WO es
/// auseinanderlaeuft. Dieselbe Ueberlegung wie bei `rules_event.rs`.
#[test]
fn dieselbe_folge_ergibt_denselben_faktor() {
    let json = data();
    let gcd_ms = Millis::try_from(json["gcdMs"].as_u64().expect("gcdMs fehlt"))
        .expect("passt nicht in Millis");
    let steps = json["steps"].as_array().expect("steps fehlen");
    assert_eq!(steps.len(), 8, "die Folge hat ihre Laenge geaendert");

    let mut buffs = Buffs::new();
    let mut schritt = 0;

    // Die Folge steht hier in derselben Reihenfolge wie in `payloads.ts`.
    // Sie ist bewusst kurz und von Hand nachvollziehbar: Wer sie aendert,
    // aendert sie an beiden Stellen, und die Datei faellt sofort auf.
    let mut pruefe = |buffs: &Buffs, now: Millis| {
        let step = &steps[schritt];
        let erwartet = u32::try_from(step["checksum"].as_u64().expect("checksum fehlt"))
            .expect("passt nicht in 32 Bit");
        let gerechnet = mix_float(FNV_OFFSET, buffs.factor(now));
        assert_eq!(
            gerechnet,
            erwartet,
            "Schritt {schritt} ({}) laeuft auseinander",
            step["was"].as_str().unwrap_or("?")
        );
        assert_eq!(
            u64::from(buffs.active_count(now)),
            step["aktiv"].as_u64().expect("aktiv fehlt"),
            "Schritt {schritt}: andere Zahl laufender Arten"
        );
        schritt += 1;
    };

    buffs.apply_for_gcds(0, 1.2, 3, 0, gcd_ms);
    pruefe(&buffs, 0);
    buffs.apply_for_gcds(1, 1.5, 2, 0, gcd_ms);
    pruefe(&buffs, 0);
    buffs.apply_for_gcds(0, 1.2, 3, 0, gcd_ms);
    pruefe(&buffs, 0);
    buffs.apply_for_gcds(1, 1.1, 2, 0, gcd_ms);
    pruefe(&buffs, 0);
    buffs.apply_for_gcds(2, 1.000_000_000_000_000_2, 1, 0, gcd_ms);
    pruefe(&buffs, 0);

    buffs.expire(1500);
    pruefe(&buffs, 1500);
    buffs.expire(3000);
    pruefe(&buffs, 3000);
    buffs.expire(4500);
    pruefe(&buffs, 4500);

    assert_eq!(
        u64::from(buffs.replacement_count()),
        json["replacements"].as_u64().expect("replacements fehlt"),
        "andere Zahl von Ersetzungen"
    );
    assert_eq!(
        u64::from(buffs.expiration_count()),
        json["expirations"].as_u64().expect("expirations fehlt"),
        "andere Zahl von Ablaeufen"
    );
}

/// Die Folge prueft sich selbst: Nach einem Ablauf muss der Faktor **genau**
/// auf einen frueheren Wert zurueckfallen.
///
/// Waere das Ersetzen in Wahrheit ein Stapeln, stimmte diese Rueckkehr nicht
/// mehr — und zwar auf beiden Seiten gleichzeitig, weshalb der Abgleich allein
/// es nicht faende.
#[test]
fn nach_dem_ablauf_kehrt_der_faktor_auf_einen_frueheren_wert_zurueck() {
    let json = data();
    let steps = json["steps"].as_array().expect("steps fehlen");
    let summe = |i: usize| steps[i]["checksum"].as_u64().expect("checksum fehlt");

    // Schritt 2 und 3: dieselbe Art erneut angelegt — der Faktor bleibt.
    assert_eq!(summe(1), summe(2), "das Erneuern hat den Faktor veraendert");
    // Schritt 5 (nach dem Ablauf der dritten Art) == Schritt 3.
    assert_eq!(summe(5), summe(3));
    // Schritt 6 (nach dem Ablauf der zweiten) == Schritt 0.
    assert_eq!(summe(6), summe(0));
    // Und am Ende der neutrale Faktor.
    let neutral = mix_float(FNV_OFFSET, 1.0);
    assert_eq!(
        u32::try_from(summe(7)).expect("passt nicht in 32 Bit"),
        neutral
    );
}
