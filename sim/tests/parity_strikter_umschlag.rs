//! Ein strikter Vergleich trifft seinen Umschlag im selben Takt wie das Spiel
//! (Issue #1027, Fassung B, entschieden am 2026-10-28).
//!
//! # Was hier verglichen wird
//!
//! `strikter-umschlag.json` stammt aus der Schleife des Spiels
//! (`driveRotation` in `src/devtools/reference-run.ts`), die an **jedem** Takt
//! entscheidet. Eine Zeile `smoulder if dot.smoulder.remains < 1500` legt dort
//! einen Takt nach dem Erreichen der Schwelle nach: Bei 10500 steht die
//! Restzeit auf 1500, `<` ist falsch, bei 10510 ist sie 1490.
//!
//! Der Simulator springt von Ereignis zu Ereignis und muss diesen Takt selbst
//! wecken (`switch_point` in `sim/src/apl/mod.rs`). Bis #1027 lag sein Weckruf
//! auf dem Erreichen der Schwelle, wo `<` noch falsch ist; die Zeile griff dann
//! erst am nächsten fremden Ereignis — hier am Ablauf des Brandes bei 12000
//! und am Ende der Abklingzeit von PURGE bei 7500.
//!
//! # Warum ohne Füller
//!
//! Mit Füller entscheidet die Liste an jedem GCD-Ende ohnehin, und ein Weckruf
//! dazwischen fällt in einen GCD. Ohne Füller wartet sie, und genau dann hängt
//! der Wurf am Weckruf. Das ist die Lage, in der sich der Weg der #797-AC5-Suche
//! trennte (`docs/APL-STRICT-COMPARISON-2026-10-24.md`, „Am Weg der Suche").
//!
//! # Zwei Werte
//!
//! `dot.remains` (auch mit vertauschten Seiten, `1500 > dot.smoulder.remains`)
//! und `cooldown.remains` — #1027 verlangt mindestens zwei der fünf
//! zeitabhängigen Werte. `brand-nicht-strikt` ist die Gegenprobe mit `<=`: Sie
//! kippt am Erreichen und trifft auch ohne Fassung B.

mod listen_aufzeichnung;

use serde_json::Value;

use listen_aufzeichnung::{druecke, laeufe, lauf};
use rota_sim::data::{index_of, SPELLS};
use rota_sim::TICK_MS;

const JSON_TEXT: &str = include_str!("../../tests/parity/recordings/strikter-umschlag.json");

/// Die Läufe, die hier je einen eigenen Prüffall haben; `jeder_lauf_hat_einen_prueffall`
/// hält die Liste gegen die Aufzeichnung.
const LAEUFE: [&str; 4] = [
    "brand-strikt-ohne-fueller",
    "brand-strikt-gespiegelt",
    "abklingzeit-strikt",
    "brand-nicht-strikt",
];

fn data() -> Value {
    listen_aufzeichnung::data(JSON_TEXT)
}

fn zauber(id: &str) -> usize {
    index_of(id).unwrap_or_else(|| panic!("{id} fehlt"))
}

/// Die Würfe eines Zaubers in einem Lauf der Aufzeichnung.
fn wuerfe(json: &Value, name: &str, id: &str) -> Vec<u32> {
    let spell = zauber(id);
    druecke(json, lauf(json, name))
        .into_iter()
        .filter(|&(_, s)| s == spell)
        .map(|(t, _)| t)
        .collect()
}

/// **Die Zusicherung vor der Messung**: Das Spiel wirft in jedem strikten Lauf
/// genau einen Takt nach dem Erreichen der Schwelle, in der Gegenprobe genau
/// darauf.
///
/// **Aus der Aufzeichnung und den Zauberdaten gerechnet, nicht abgeschrieben.**
/// Stimmte das nicht, prüfte der Vergleich darunter eine Lage, in der der
/// Weckruf gar nicht entscheidet — und bliebe unter jeder Fassung grün.
#[test]
fn das_spiel_wirft_einen_takt_nach_dem_erreichen_der_schwelle() {
    let json = data();
    let laufzeit = SPELLS[zauber("smoulder")]
        .dot
        .expect("smoulder ist ein Brand")
        .duration_ms;
    for (name, versatz) in [
        ("brand-strikt-ohne-fueller", TICK_MS),
        ("brand-strikt-gespiegelt", TICK_MS),
        ("brand-nicht-strikt", 0),
    ] {
        let reihe = wuerfe(&json, name, "smoulder");
        assert!(
            reihe.len() >= 2,
            "{name}: SMOULDER wird kein zweites Mal gelegt — {reihe:?}"
        );
        for paar in reihe.windows(2) {
            assert_eq!(
                paar[1],
                paar[0] + laufzeit - 1500 + versatz,
                "{name}: Nachlegen nicht {versatz} ms nach dem Erreichen von 1500 ({reihe:?})"
            );
        }
    }

    let abklingzeit = SPELLS[zauber("purge")].cooldown_ms;
    assert_eq!(
        wuerfe(&json, "abklingzeit-strikt", "purge"),
        vec![0],
        "abklingzeit-strikt: PURGE nicht genau einmal bei 0"
    );
    assert_eq!(
        wuerfe(&json, "abklingzeit-strikt", "kindle")
            .first()
            .copied(),
        Some(abklingzeit - 1000 + TICK_MS),
        "abklingzeit-strikt: KINDLE nicht einen Takt nach dem Erreichen von 1000"
    );
}

/// Die Regeln: Spielen beide Seiten dieselben Drücke gleich ab? Grün unter
/// jeder Weckruf-Fassung — das Abspielen entscheidet nichts.
#[test]
fn das_abspielen_trifft_das_spiel_in_jedem_takt() {
    listen_aufzeichnung::das_abspielen_trifft_das_spiel(&data(), &LAEUFE);
}

fn entscheidet_wie_im_spiel(name: &str) {
    listen_aufzeichnung::entscheidet_wie_im_spiel(&data(), name);
}

#[test]
fn brand_strikt_ohne_fueller_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("brand-strikt-ohne-fueller");
}

#[test]
fn brand_strikt_gespiegelt_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("brand-strikt-gespiegelt");
}

#[test]
fn abklingzeit_strikt_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("abklingzeit-strikt");
}

#[test]
fn brand_nicht_strikt_entscheidet_wie_im_spiel() {
    entscheidet_wie_im_spiel("brand-nicht-strikt");
}

/// Die Liste oben gegen die Aufzeichnung, in beide Richtungen.
#[test]
fn jeder_lauf_hat_einen_prueffall() {
    let json = data();
    assert_eq!(
        laeufe(&json),
        LAEUFE,
        "Läufe der Aufzeichnung und Prüffälle dieser Datei passen nicht zusammen"
    );
}
