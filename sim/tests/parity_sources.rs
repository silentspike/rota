//! Gleichlauf der **fuenf Quellen** (Issue #178).
//!
//! Geprueft wird gegen `tests/parity/recordings/sources.json`, erzeugt aus dem
//! Spielcode selbst.
//!
//! # Warum das ein eigener Prueffall ist
//!
//! Die Zahlen hier entscheiden, was ein Spieler ueberhaupt erreichen kann.
//! Liefen sie auseinander, rechnete der Balance-Laeufer mit anderen Grenzen als
//! das Spiel — und beide saehen fuer sich genommen richtig aus. Ein
//! Kampf-Prueffall faenge es nicht: Er bekommt die Werte uebergeben und fragt
//! nicht, wo sie herkommen.

use serde_json::Value;

use rota_sim::gear::MAX_ITEM_LEVEL;
use rota_sim::sources::{
    affix_permille, is_permanent, max_of, mixed_build, permanent_max, pure_build, total_max,
    BUDGET_PER_PERMILLE, SOURCE_COUNT, SOURCE_NAMES, STAT_COUNT, STAT_NAMES,
};

/// Der unveraenderte Inhalt der Vergleichsdatei.
const SOURCES: &str = include_str!("../../tests/parity/recordings/sources.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(SOURCES).expect("sources.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von sources.json"
    );
    value
}

fn u32_at(node: &Value, key: &str) -> u32 {
    let raw = node[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} fehlt oder ist keine Zahl"));
    u32::try_from(raw).expect("passt nicht in 32 Bit")
}

#[test]
fn beide_seiten_kennen_dieselben_quellen_und_werte() {
    // Die Reihenfolge ist Teil der Schnittstelle: Sie bestimmt, welcher Eintrag
    // welche Quelle meint. Eine Vertauschung liefe sonst still durch.
    let json = data();
    let quellen: Vec<&str> = json["sourceNames"]
        .as_array()
        .expect("sourceNames fehlt")
        .iter()
        .map(|v| v.as_str().expect("Name erwartet"))
        .collect();
    assert_eq!(quellen, SOURCE_NAMES.to_vec());

    let werte: Vec<&str> = json["statNames"]
        .as_array()
        .expect("statNames fehlt")
        .iter()
        .map(|v| v.as_str().expect("Name erwartet"))
        .collect();
    assert_eq!(werte, STAT_NAMES.to_vec());
}

#[test]
fn der_umrechner_ist_auf_beiden_seiten_derselbe() {
    // An ihm haengt jeder Affix-Wert. Waere er verschieden, stimmte keine
    // einzige Zahl ueberein — und alle folgenden Pruefungen zeigten auf die
    // Quellen statt auf die Ursache.
    let json = data();
    assert_eq!(
        u64::from(u32_at(&json, "budgetPerPermille")),
        BUDGET_PER_PERMILLE
    );
}

#[test]
fn jede_quelle_liefert_dieselbe_zahl_wie_im_spiel() {
    let json = data();
    let stats = json["stats"].as_array().expect("stats fehlt");
    assert_eq!(stats.len(), STAT_COUNT, "andere Zahl von Sonderwerten");

    for eintrag in stats {
        let stat = u32_at(eintrag, "stat") as usize;
        let quellen = eintrag["sources"].as_array().expect("sources fehlt");
        assert_eq!(quellen.len(), SOURCE_COUNT, "andere Zahl von Quellen");

        for quelle in quellen {
            let source = u32_at(quelle, "source") as usize;
            assert_eq!(
                max_of(source, stat, MAX_ITEM_LEVEL),
                u32_at(quelle, "max"),
                "{} auf {}",
                SOURCE_NAMES[source],
                STAT_NAMES[stat]
            );
        }

        assert_eq!(
            permanent_max(stat, MAX_ITEM_LEVEL),
            u32_at(eintrag, "permanentMax"),
            "dauerhafte Summe bei {}",
            STAT_NAMES[stat]
        );
        assert_eq!(
            total_max(stat, MAX_ITEM_LEVEL),
            u32_at(eintrag, "totalMax"),
            "Gesamtsumme bei {}",
            STAT_NAMES[stat]
        );
    }
}

#[test]
fn beide_seiten_trennen_dauerhaft_und_kurzzeitig_gleich() {
    let json = data();
    let dauerhaft = json["permanent"].as_array().expect("permanent fehlt");
    assert_eq!(dauerhaft.len(), SOURCE_COUNT);
    for (source, wert) in dauerhaft.iter().enumerate() {
        assert_eq!(
            is_permanent(source),
            wert.as_bool().expect("Wahrheitswert erwartet"),
            "{}",
            SOURCE_NAMES[source]
        );
    }
    // Und die Trennung ist wirklich eine: Genau eine Quelle ist befristet.
    let befristet = dauerhaft
        .iter()
        .filter(|v| v.as_bool() == Some(false))
        .count();
    assert_eq!(befristet, 1, "die Trennung dauerhaft/kurzzeitig ist leer");
}

#[test]
fn die_beiden_aufbauten_stimmen_ueberein() {
    let json = data();
    for eintrag in json["stats"].as_array().expect("stats fehlt") {
        let stat = u32_at(eintrag, "stat") as usize;

        let rein = pure_build(stat, MAX_ITEM_LEVEL);
        let json_rein = &eintrag["pure"];
        assert_eq!(rein.catalysis, u32_at(json_rein, "catalysis"));
        assert_eq!(rein.purity, u32_at(json_rein, "purity"));
        assert_eq!(rein.yield_bonus, u32_at(json_rein, "yieldBonus"));

        let gemischt = mixed_build(stat, MAX_ITEM_LEVEL);
        let json_gemischt = &eintrag["mixed"];
        assert_eq!(gemischt.catalysis, u32_at(json_gemischt, "catalysis"));
        assert_eq!(gemischt.purity, u32_at(json_gemischt, "purity"));
        assert_eq!(gemischt.yield_bonus, u32_at(json_gemischt, "yieldBonus"));
    }
}

#[test]
fn der_affix_wert_folgt_auf_beiden_seiten_aus_dem_budget() {
    // **Die Ableitung, an der beide Seiten haengen.** Wer sie auf einer Seite
    // durch eine feste Zahl ersetzt, faellt hier auf — und zwar bei jeder
    // Stuetzstelle, nicht nur bei der hoechsten.
    let json = data();
    let affixe = json["affixes"].as_array().expect("affixes fehlt");
    assert!(!affixe.is_empty(), "keine Affix-Stuetzstellen");

    for eintrag in affixe {
        let level = u32_at(eintrag, "level");
        let quality = u32_at(eintrag, "quality") as usize;
        assert_eq!(
            affix_permille(level, quality),
            u32_at(eintrag, "permille"),
            "Affix auf Stufe {level}, Qualitaet {quality}"
        );
    }

    // Und die Stuetzstellen decken wirklich einen Bereich ab: Der hoechste Wert
    // liegt ueber dem niedrigsten. Ohne diese Zeile koennten alle gleich sein.
    let werte: Vec<u32> = affixe.iter().map(|e| u32_at(e, "permille")).collect();
    let kleinster = werte.iter().min().copied().unwrap_or(0);
    let groesster = werte.iter().max().copied().unwrap_or(0);
    assert!(
        groesster > kleinster,
        "alle Stuetzstellen tragen denselben Wert — die Datei prueft die Ableitung nicht"
    );
}

#[test]
fn die_grundwerte_rechnen_auf_beiden_seiten_gleich() {
    // Issue #282. **Nachgerechnet, nicht nachgeschlagen:** Die eingecheckte
    // Datei liefert die Basis mit, und diese Seite rechnet damit selbst. Wer
    // nur das Ergebnis vergliche, bekäme beim nächsten Kalibrieren einen
    // gemeldeten Unterschied zwischen den Seiten, wo nur eine Zahl bewegt
    // wurde — der Wächter schlüge an, ohne dass etwas auseinanderläuft.
    let json = data();
    let eintraege = json["baseStats"]
        .as_array()
        .expect("baseStats fehlt — sources.json stammt aus einem Stand vor #282");

    // Der Gegenfall: Eine leere Liste wäre unten in null Durchläufen grün.
    assert!(
        eintraege.len() >= 15,
        "nur {} Stuetzstellen — zu wenige, um eine Kurve zu belegen",
        eintraege.len()
    );

    let mut gewachsen = false;
    for eintrag in eintraege {
        let level = u32_at(eintrag, "level");
        let base = u32_at(eintrag, "base");
        let erwartet = u32_at(eintrag, "permille");
        let hier = rota_sim::sources::base_permille(level, base);
        assert_eq!(
            hier, erwartet,
            "Stufe {level}, Basis {base}: Simulator {hier}, Spiel {erwartet}"
        );
        if level > 1 && hier > base {
            gewachsen = true;
        }
    }

    // Ohne diesen Fall wäre der Vergleich oben auch bei einer flachen Kurve
    // grün — und eine flache Kurve hiesse, dass die Gegenstandsstufe nichts
    // bewirkt, also genau der Zustand, den #282 beheben soll.
    assert!(
        gewachsen,
        "kein einziger Wert waechst mit der Stufe — die Kurve ist flach"
    );
}

/// Topf 4 rechnet auf beiden Seiten dieselbe Zahl aus denselben Quellen.
///
/// **Bis zur Entscheidung vom 2026-08-11 hing hier nichts aneinander.**
/// `gearEffectsPermille` stand im Spiel und `gear_effects_permille` im
/// Simulator, jedes mit einem eigenen Prueffall — aber es gab keinen
/// Vergleich. Die zwei Fassungen konnten auseinanderlaufen, ohne dass eine
/// Zeile rot wurde. Genau das ist die Luecke, die diese Datei fuer
/// `base_permille` schon schliesst.
#[test]
fn topf_4_stimmt_mit_dem_spiel_ueberein() {
    let json = data();
    let eintraege = json["gearEffects"]
        .as_array()
        .expect("gearEffects fehlt — sources.json stammt aus einem Stand vor #480");
    assert!(
        eintraege.len() >= 9,
        "nur {} Stuetzstellen — zu wenige",
        eintraege.len()
    );

    let mut gesehen_ungleich_null = false;
    for eintrag in eintraege {
        let legendaries = u32_at(eintrag, "legendaries");
        let share = u32_at(eintrag, "setSharePermille");
        let erwartet = u32_at(eintrag, "permille");
        let hier = rota_sim::sources::gear_effects_permille(legendaries, share);
        assert_eq!(
            hier, erwartet,
            "{legendaries} legendaere, Satz-Anteil {share}: Simulator {hier}, Spiel {erwartet}"
        );
        if hier > 0 {
            gesehen_ungleich_null = true;
        }
    }

    // Ohne diesen Fall waere der Vergleich auch dann gruen, wenn beide Seiten
    // durchgehend 0 lieferten — zwei uebereinstimmende Nullen.
    assert!(
        gesehen_ungleich_null,
        "kein einziger Wert ist ungleich null — der Vergleich prueft nichts"
    );
}

/// Der Satz-Anteil je Teilezahl — und die Deckelung auf der sechsten Stufe.
///
/// Der Simulator laedt `data/sets.json` nicht; das Spiel exportiert den Anteil
/// deshalb als Stuetzstellen. Geprueft wird die EIGENSCHAFT, die 13.5
/// vorschreibt: Die sechste Stufe traegt einen Rotations-Effekt und keinen
/// Wert, also muessen sechs Teile denselben Anteil ergeben wie vier.
#[test]
fn die_sechste_satz_stufe_traegt_keinen_wert() {
    let json = data();
    let eintraege = json["setShares"]
        .as_array()
        .expect("setShares fehlt — sources.json stammt aus einem Stand vor #480");
    let anteil = |teile: u32| -> u32 {
        eintraege
            .iter()
            .find(|e| u32_at(e, "pieces") == teile)
            .map(|e| u32_at(e, "permille"))
            .unwrap_or_else(|| panic!("keine Stuetzstelle fuer {teile} Teile"))
    };

    assert_eq!(
        anteil(6),
        anteil(4),
        "die sechste Stufe traegt einen Werte-Bonus, obwohl sie es nicht darf"
    );
    // Und die unteren Schwellen wirken sehr wohl — sonst waere die Gleichheit
    // oben aus dem falschen Grund erfuellt.
    assert!(anteil(4) > anteil(2), "die vierte Stufe traegt nichts bei");
    assert!(anteil(2) > anteil(1), "die zweite Stufe traegt nichts bei");
    assert_eq!(anteil(1), 0, "ein einzelnes Teil traegt schon etwas bei");
}
