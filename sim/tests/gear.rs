//! Gleichlauf der Ausruestungs-Rechnung (Issue E6-1, AC6).
//!
//! **Aussagekraeftig ohne einen einzigen ausgelieferten Gegenstand.** Der
//! Gegenstands-Katalog steht noch nicht (GAME-DESIGN 22, vor E6). Geprueft wird
//! deshalb die Rechenregel, nicht ihr Inhalt — dieselbe Lage wie beim
//! Auswahlmodus (#102), wo der Nachweis auch ohne ausgelieferten Zauber
//! getragen hat.
//!
//! Die Folge stammt aus dem Spielcode (`tests/parity/payloads.ts`) und liegt als
//! `gear.json` im Repository. Hier wird sie nachgerechnet.

use serde_json::Value;

mod common;
use common::bits;

use rota_sim::checksum::{mix_float, mix_int, FNV_OFFSET};
use rota_sim::gear::{
    budget_of, distribute_budget, undecided_slots, ValuePools, BASE_STAT_IDS, POOL_COUNT,
    POOL_NAMES, QUALITY_HAS_EFFECT, QUALITY_NAMES, QUALITY_STAT_SLOTS, SLOT_NAMES, SLOT_SPECIAL,
    SPECIAL_NAMES,
};

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const GEAR: &str = include_str!("../../tests/parity/recordings/gear.json");

fn datei() -> Value {
    let value: Value = serde_json::from_str(GEAR).expect("gear.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von gear.json"
    );
    value
}

fn namen(value: &Value, feld: &str) -> Vec<String> {
    value[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|entry| entry.as_str().expect("Zeichenkette erwartet").to_owned())
        .collect()
}

/// **Beide Seiten benennen dieselben Toepfe, Qualitaeten und Plaetze.**
///
/// Ohne diesen Vergleich koennten die Kennnummern gleich bleiben und ihre
/// Bedeutung auseinanderlaufen — genau der Fehler, den Issue E3-6 bei den
/// Protokoll-Arten gefunden hat.
#[test]
fn beide_seiten_benennen_dasselbe() {
    let value = datei();

    assert_eq!(namen(&value, "poolNames"), POOL_NAMES.to_vec());
    assert_eq!(namen(&value, "qualityNames"), QUALITY_NAMES.to_vec());
    assert_eq!(namen(&value, "slotNames"), SLOT_NAMES.to_vec());
    assert_eq!(namen(&value, "specialNames"), SPECIAL_NAMES.to_vec());
    assert_eq!(namen(&value, "baseStatIds"), BASE_STAT_IDS.to_vec());

    let slots: Vec<u64> = value["qualityStatSlots"]
        .as_array()
        .expect("qualityStatSlots fehlen")
        .iter()
        .map(|entry| entry.as_u64().expect("Zahl erwartet"))
        .collect();
    assert_eq!(
        slots,
        QUALITY_STAT_SLOTS
            .iter()
            .map(|s| *s as u64)
            .collect::<Vec<_>>(),
        "die Qualitaeten tragen verschieden viele Werte-Plaetze"
    );

    let effect: Vec<bool> = value["qualityHasEffect"]
        .as_array()
        .expect("qualityHasEffect fehlt")
        .iter()
        .map(|entry| entry.as_bool().expect("Wahrheitswert erwartet"))
        .collect();
    assert_eq!(effect, QUALITY_HAS_EFFECT.to_vec());
}

/// Die zwei unbestimmten Plaetze stehen auf beiden Seiten offen.
///
/// **Eine Stolperdrahte, keine Formalie:** Wer `relic` oder `shard` auf einer
/// Seite zuordnet und auf der anderen vergisst, faellt hier auf — und zwar
/// bevor ein Gegenstand mit einem geratenen Sonderwert im Spiel landet.
#[test]
fn dieselben_plaetze_sind_offen() {
    let value = datei();

    let special: Vec<i64> = value["slotSpecial"]
        .as_array()
        .expect("slotSpecial fehlt")
        .iter()
        .map(|entry| entry.as_i64().expect("Zahl erwartet"))
        .collect();
    assert_eq!(
        special,
        SLOT_SPECIAL
            .iter()
            .map(|s| i64::from(*s))
            .collect::<Vec<_>>(),
        "die Platz-Zuordnung laeuft auseinander"
    );

    let offen: Vec<u64> = value["undecidedSlots"]
        .as_array()
        .expect("undecidedSlots fehlen")
        .iter()
        .map(|entry| entry.as_u64().expect("Zahl erwartet"))
        .collect();
    assert_eq!(
        offen,
        undecided_slots()
            .iter()
            .map(|s| *s as u64)
            .collect::<Vec<_>>(),
        "eine Seite hat einen Platz zugeordnet, die andere nicht"
    );
    assert_eq!(offen.len(), 2, "13.2 laesst genau zwei Plaetze offen");
}

/// **Der Kern: dieselbe Folge von Beitraegen ergibt dieselben Faktoren.**
///
/// Die Folge deckt beide Regeln ab — gleiche Toepfe addieren, verschiedene
/// multiplizieren — und enthaelt den Fall, um den es in 13.1b eigentlich geht:
/// ein Satz-Bonus neben einem legendaeren Effekt.
#[test]
fn dieselben_beitraege_ergeben_dieselben_faktoren() {
    let value = datei();
    let steps = value["poolSteps"].as_array().expect("poolSteps fehlen");
    assert!(!steps.is_empty(), "die Folge ist leer");

    let mut pools = ValuePools::new();
    for (index, step) in steps.iter().enumerate() {
        let pool = step["pool"].as_u64().expect("pool fehlt") as usize;
        // Auch der EINGABEwert ueber das Bitmuster — sonst pruefte der Test
        // eine andere Zahl, als das Spiel gerechnet hat.
        let share = f64::from_bits(bits(&step["shareBits"]));
        assert!(
            pools.add(pool, share),
            "Schritt {index}: Topf {pool} gibt es nicht"
        );

        // Erst die einzelnen Faktoren — sie sagen, WAS abweicht.
        let erwartet = step["factorBits"].as_array().expect("factorBits fehlen");
        assert_eq!(
            erwartet.len(),
            POOL_COUNT,
            "Schritt {index}: andere Zahl Toepfe"
        );
        for (pool_index, wert) in erwartet.iter().enumerate() {
            assert_eq!(
                pools.factor(pool_index).to_bits(),
                bits(wert),
                "Schritt {index} ({}): Topf {} hat einen anderen Faktor",
                step["was"].as_str().unwrap_or("?"),
                POOL_NAMES[pool_index]
            );
        }

        // Dann das Produkt — **bitgleich**, nicht ungefaehr. Die Reihenfolge der
        // Multiplikationen liegt fest, weil Fliesskomma nicht assoziativ ist.
        assert_eq!(
            pools.product().to_bits(),
            bits(&step["productBits"]),
            "Schritt {index} ({}): das Produkt laeuft auseinander",
            step["was"].as_str().unwrap_or("?")
        );

        // Und die Pruefsumme ueber alles — sie faengt, was die Einzelvergleiche
        // nicht ansehen.
        let mut hash = FNV_OFFSET;
        for pool_index in 0..POOL_COUNT {
            hash = mix_float(hash, pools.factor(pool_index));
        }
        hash = mix_float(hash, pools.product());
        assert_eq!(
            hash,
            u32::try_from(step["checksum"].as_u64().expect("checksum fehlt"))
                .expect("passt nicht in 32 Bit"),
            "Schritt {index}: der Gesamtzustand laeuft auseinander"
        );
    }
}

/// Die Folge prueft sich selbst: Sie enthaelt beide Regeln.
///
/// **Ohne diesen Waechter waere der Gleichlauf-Nachweis oben wertlos**, sobald
/// jemand die Folge auf einen einzigen Topf zusammenstreicht. Er pruefte dann
/// weiter erfolgreich — nur eben nichts Interessantes mehr.
#[test]
fn die_folge_deckt_beide_regeln_ab() {
    let value = datei();
    let steps = value["poolSteps"].as_array().expect("poolSteps fehlen");

    let mut je_topf = [0usize; POOL_COUNT];
    for step in steps {
        let pool = step["pool"].as_u64().expect("pool fehlt") as usize;
        je_topf[pool] += 1;
    }

    let mehrfach = je_topf.iter().filter(|zahl| **zahl >= 2).count();
    let benutzt = je_topf.iter().filter(|zahl| **zahl >= 1).count();
    assert!(
        mehrfach >= 2,
        "die Folge zeigt das Addieren nur an einem Topf — zu wenig, um die Regel zu belegen"
    );
    assert!(
        benutzt >= 3,
        "die Folge zeigt das Multiplizieren zwischen zu wenigen Toepfen"
    );
}

/// **Dasselbe Budget, dieselbe Verteilung — ueber die ganze Stufenleiter.**
#[test]
fn dieselbe_stufe_ergibt_dieselben_werte() {
    let value = datei();
    let budgets = value["budgets"].as_array().expect("budgets fehlen");
    assert!(!budgets.is_empty(), "keine Budgets in der Datei");

    for (index, entry) in budgets.iter().enumerate() {
        let quality = entry["quality"].as_u64().expect("quality fehlt") as usize;
        let level = u32::try_from(entry["level"].as_u64().expect("level fehlt"))
            .expect("Stufe passt nicht");
        let erwartet_budget = entry["budget"].as_u64().expect("budget fehlt");

        assert_eq!(
            budget_of(level, quality),
            erwartet_budget,
            "Eintrag {index}: Qualitaet {quality}, Stufe {level} hat ein anderes Budget"
        );

        let mut werte = [0u64; 4];
        let count = distribute_budget(level, quality, &mut werte);
        let erwartet: Vec<u64> = entry["shares"]
            .as_array()
            .expect("shares fehlen")
            .iter()
            .map(|wert| wert.as_u64().expect("Zahl erwartet"))
            .collect();
        assert_eq!(
            werte[..count].to_vec(),
            erwartet,
            "Eintrag {index}: Qualitaet {quality}, Stufe {level} verteilt anders"
        );

        let mut hash = FNV_OFFSET;
        hash = mix_int(hash, level as i32);
        hash = mix_int(hash, quality as i32);
        hash = mix_int(hash, erwartet_budget as i32);
        for wert in &werte[..count] {
            hash = mix_int(hash, *wert as i32);
        }
        assert_eq!(
            hash,
            u32::try_from(entry["checksum"].as_u64().expect("checksum fehlt"))
                .expect("passt nicht in 32 Bit"),
            "Eintrag {index}: der Gesamtzustand laeuft auseinander"
        );
    }
}
