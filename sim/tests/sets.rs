//! Gleichlauf der Satz-Rechnung (Issue E6-3, AC5).
//!
//! **Aussagekraeftig ohne einen einzigen ausgelieferten Satz.** Welche 18 Boni
//! es gibt, ist Katalog-Inhalt (GAME-DESIGN 22, vor E10). Geprueft wird die
//! Rechenregel — dieselbe Lage wie bei den Toepfen (#114).
//!
//! Die Folge enthaelt ausdruecklich den Fall, in dem sich **nichts** aendern
//! darf: eine Leiste ohne Satz-Teil. Ohne ihn belegte der Rest nur, dass
//! ueberhaupt etwas passiert.

use serde_json::Value;

mod common;
use common::bits;

use rota_sim::checksum::{mix_float, mix_int, FNV_OFFSET};
use rota_sim::sets::{
    active_rotation_effects, count_pieces, tiers_reached, value_share_of, SetBonus, SetDef,
    BONUS_ROTATION, BONUS_VALUE, SET_COUNT, SET_TIERS,
};

/// Der unveraenderte Inhalt der Vergleichsdatei aus dem Spiel.
const SETS: &str = include_str!("../../tests/parity/recordings/sets.json");

fn datei() -> Value {
    let value: Value = serde_json::from_str(SETS).expect("sets.json ist kein JSON");
    assert_eq!(
        value["schema"].as_u64(),
        Some(1),
        "unbekannte Fassung von sets.json"
    );
    value
}

/// Die zwei Pruef-Saetze, wie sie im Spielcode stehen.
fn pruef_saetze() -> [SetDef; 2] {
    let wert = |share: f64| SetBonus {
        kind: BONUS_VALUE,
        share,
        effect: "",
    };
    let rotation = |effect: &'static str| SetBonus {
        kind: BONUS_ROTATION,
        share: 0.0,
        effect,
    };
    [
        SetDef {
            id: 0,
            school: "CALCINATION",
            tiers: [wert(0.1), wert(0.2), rotation("test-a-sechser")],
        },
        SetDef {
            id: 1,
            school: "SUBLIMATION",
            tiers: [wert(0.05), wert(0.15), rotation("test-b-sechser")],
        },
    ]
}

/// **Die Struktur ist auf beiden Seiten dieselbe.**
#[test]
fn beide_seiten_fuehren_dieselben_stufen() {
    let value = datei();
    let stufen: Vec<u64> = value["tiers"]
        .as_array()
        .expect("tiers fehlen")
        .iter()
        .map(|v| v.as_u64().expect("Zahl erwartet"))
        .collect();
    assert_eq!(
        stufen,
        SET_TIERS.iter().map(|s| u64::from(*s)).collect::<Vec<_>>(),
        "die Bonus-Stufen laufen auseinander"
    );
    assert_eq!(
        value["setCount"].as_u64(),
        Some(SET_COUNT as u64),
        "andere Zahl Saetze"
    );

    // Und die Pruef-Saetze selbst — sonst rechnete der Simulator mit anderen
    // Boni als das Spiel und der Vergleich waere wertlos.
    let saetze = value["sets"].as_array().expect("sets fehlen");
    assert_eq!(saetze.len(), pruef_saetze().len());
    for (index, (aus_datei, meiner)) in saetze.iter().zip(pruef_saetze()).enumerate() {
        assert_eq!(
            aus_datei["school"].as_str(),
            Some(meiner.school),
            "Satz {index}: andere Schule"
        );
        let stufen = aus_datei["tiers"].as_array().expect("tiers fehlen");
        for (tier, bonus) in stufen.iter().zip(meiner.tiers) {
            assert_eq!(
                tier["kind"].as_u64(),
                Some(u64::from(bonus.kind)),
                "Satz {index}: andere Bonus-Art"
            );
            assert_eq!(
                tier["effect"].as_str(),
                Some(bonus.effect),
                "Satz {index}: andere Effekt-Kennung"
            );
        }
    }
}

/// **Der Kern: dieselbe Zusammenstellung ergibt dieselbe Rechnung.**
#[test]
fn dieselbe_zusammenstellung_ergibt_dasselbe() {
    let value = datei();
    let steps = value["steps"].as_array().expect("steps fehlen");
    assert!(!steps.is_empty(), "eine leere Folge bestuende immer");
    let saetze = pruef_saetze();

    for (index, step) in steps.iter().enumerate() {
        let equipped: Vec<i32> = step["equipped"]
            .as_array()
            .expect("equipped fehlt")
            .iter()
            .map(|v| i32::try_from(v.as_i64().expect("Zahl erwartet")).expect("passt nicht"))
            .collect();

        let mut pieces = [0u32; SET_COUNT];
        let getragen = count_pieces(&equipped, &mut pieces);
        let was = step["was"].as_str().unwrap_or("?");

        // Erst die Einzelgroessen — sie sagen, WAS abweicht.
        assert_eq!(
            u64::from(getragen),
            step["wornPieces"].as_u64().expect("wornPieces fehlt"),
            "Schritt {index} ({was}): andere Zahl getragener Teile"
        );

        let erwartet_pieces: Vec<u64> = step["pieces"]
            .as_array()
            .expect("pieces fehlen")
            .iter()
            .map(|v| v.as_u64().expect("Zahl erwartet"))
            .collect();
        assert_eq!(
            pieces.iter().map(|n| u64::from(*n)).collect::<Vec<_>>(),
            erwartet_pieces,
            "Schritt {index} ({was}): andere Teile-Zaehlung"
        );

        let erwartet_stufen: Vec<u64> = step["tiersReached"]
            .as_array()
            .expect("tiersReached fehlen")
            .iter()
            .map(|v| v.as_u64().expect("Zahl erwartet"))
            .collect();
        assert_eq!(
            pieces
                .iter()
                .map(|n| tiers_reached(*n) as u64)
                .collect::<Vec<_>>(),
            erwartet_stufen,
            "Schritt {index} ({was}): andere Zahl ausgeloester Stufen"
        );

        // Der Anteil ueber das BITMUSTER, nicht ueber die Dezimalform —
        // `serde_json` liest manche Zahlen ein Bit daneben (#122). Hier steht
        // 0.30000000000000004 in der Datei; genau die Sorte Zahl.
        let anteil = value_share_of(&saetze, &pieces);
        assert_eq!(
            anteil.to_bits(),
            bits(&step["valueShareBits"]),
            "Schritt {index} ({was}): anderer Werte-Anteil"
        );

        let erwartet_effekte: Vec<&str> = step["rotationEffects"]
            .as_array()
            .expect("rotationEffects fehlen")
            .iter()
            .map(|v| v.as_str().expect("Zeichenkette erwartet"))
            .collect();
        assert_eq!(
            active_rotation_effects(&saetze, &pieces),
            erwartet_effekte,
            "Schritt {index} ({was}): andere Rotations-Effekte"
        );

        // Und die Pruefsumme ueber alles — sie faengt, was die
        // Einzelvergleiche nicht ansehen.
        let mut hash = mix_int(FNV_OFFSET, getragen as i32);
        for zahl in &pieces {
            hash = mix_int(hash, *zahl as i32);
        }
        hash = mix_float(hash, anteil);
        assert_eq!(
            hash,
            u32::try_from(step["checksum"].as_u64().expect("checksum fehlt"))
                .expect("passt nicht in 32 Bit"),
            "Schritt {index} ({was}): der Gesamtzustand laeuft auseinander"
        );
    }
}

/// **Die Folge enthaelt den Fall, in dem sich nichts aendern darf.**
///
/// Ohne ihn belegte der Gleichlauf oben nur, dass beide Seiten dasselbe
/// rechnen — nicht, dass die Rechnung ueberhaupt etwas bewirkt. Ein Lauf, in
/// dem jede Zusammenstellung einen Satz traegt, waere gruen, auch wenn beide
/// Seiten gleich falsch rechneten.
#[test]
fn die_folge_enthaelt_den_nullfall_und_eine_wirkung() {
    let value = datei();
    let steps = value["steps"].as_array().expect("steps fehlen");

    let ohne = steps
        .iter()
        .find(|s| s["wornPieces"].as_u64() == Some(0))
        .expect("die Folge enthaelt keine Leiste OHNE Satz-Teil");
    assert_eq!(
        f64::from_bits(bits(&ohne["valueShareBits"])),
        0.0,
        "ohne Satz-Teil darf kein Anteil entstehen"
    );

    let mit_wirkung = steps
        .iter()
        .filter(|s| f64::from_bits(bits(&s["valueShareBits"])) > 0.0)
        .count();
    assert!(
        mit_wirkung >= 3,
        "die Folge zeigt zu wenige Zusammenstellungen MIT Wirkung: {mit_wirkung}"
    );

    let mit_effekt = steps
        .iter()
        .filter(|s| !s["rotationEffects"].as_array().is_none_or(|a| a.is_empty()))
        .count();
    assert!(
        mit_effekt >= 1,
        "die Folge zeigt keinen ausgeloesten Rotations-Effekt"
    );
}
