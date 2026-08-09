//! Welcher Zauber welche Kontroll-Wirkung traegt (Issue #666).
//!
//! # Was diese Datei leistet — und was NICHT
//!
//! **Sie behebt keine Luecke.** Sie macht eine bestehende Deckung
//! ausdruecklich und nennt den Zauber beim Namen, statt drei fremde Faelle
//! anschlagen zu lassen.
//!
//! Das ist eine Korrektur an meiner eigenen Begruendung. Die urspruengliche
//! Fassung dieses Issues behauptete, `stay`, `anchor` und `latch` seien
//! ungeschuetzt. Das war eine FEHLMESSUNG, und sie stammte von mir:
//!
//! > Ich zaehlte rote Faelle mit `grep -c FAILED`. Bricht `build.rs` ab, gibt
//! > es keine solche Zeile — ich zaehlte 0 und las das als "kein Test faerbt".
//!
//! Tatsaechlich verweigert `sim/build.rs` die Vertauschungen laengst
//! (Zeilen 1700-1702):
//!
//! ```text
//!   "slowCast"    -> ("ControlEffect::SlowCast",    "amountMs")
//!   "delayStrike" -> ("ControlEffect::DelayStrike", "amountMs")
//!   "holdPhase"   -> ("ControlEffect::HoldPhase",   "amountPercent")
//! ```
//!
//! Wer `stay` von `delayStrike` auf `holdPhase` dreht, bekommt keinen roten
//! Test, sondern gar keinen Bau: *spells[stay]/control: Feld "amountPercent"
//! fehlt*. Und die baubaren Vertauschungen — die mit gleichem Amount-Feld —
//! faerben je 3 Rust- und 3 TS-Faelle, auch ohne diese Datei.
//!
//! Ein abgebrochener Bau und ein gruener Lauf sehen in einer Zaehlung, die nur
//! nach roten Zeilen sucht, gleich aus. Die SUITEN-Zahl haette es gezeigt:
//! 0 statt 97.
//!
//! # Was hier wirklich neu ist
//!
//! Der Vollstaendigkeits-Waechter darunter. `build.rs` prueft, ob das FELD zur
//! Wirkung passt, und die baubaren Vertauschungen sind gedeckt — aber
//! **niemand fordert einen NEUEN Traeger ein**. Der naechste Zauber mit einem
//! `control`-Feld kaeme ohne Prueffall herein, und kein Fall meldete es.
//!
//! Genau das ist bei `buff.target` ZWEIMAL passiert, bevor #661 den Filter
//! ueber den Datensatz gelegt hat: `test-cdreduce` und `test-convert` kamen
//! mit E10 dazu, die sechs spielbaren Verstaerker fehlten von Anfang an. Beide
//! Male fiel es nur auf, weil jemand aus einem anderen Grund in der Datei war.
use rota_sim::data::{ControlEffect, SPELLS};

/// Findet einen Zauber ueber seine Kennung.
fn zauber(id: &str) -> &'static rota_sim::data::Spell {
    SPELLS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("Zauber {id} fehlt im erzeugten Datensatz"))
}

/// Jeder Traeger einer Kontroll-Wirkung und die Boss-Handlung, auf die er
/// zielt.
///
/// **Diese Liste ist vollstaendig, und `die_liste_fuehrt_jeden_traeger` haelt
/// sie so.** Genau daran ist die Schwester-Liste in `buff_targets.rs` zweimal
/// gescheitert, bevor sie einen Waechter bekam (#562, #660): Eine Abschrift
/// des Datensatzes altert, und sie sagt nirgends, dass sie nur einen Teil
/// fuehrt.
const ERWARTET: &[(&str, ControlEffect)] = &[
    ("stall", ControlEffect::SlowCast),
    ("stay", ControlEffect::DelayStrike),
    ("anchor", ControlEffect::HoldPhase),
    ("wedge", ControlEffect::ConvertToDamage),
    ("capstone", ControlEffect::AllThree),
    ("latch", ControlEffect::DelayStrike),
];

/// AC1 — die Zuordnung selbst.
#[test]
fn die_traeger_zielen_auf_die_erwarteten_boss_handlungen() {
    // Die drei Wirkungen sind sehr verschieden: `SlowCast` verschiebt den
    // laufenden Boss-Zauber, `DelayStrike` den naechsten Zeitleisten-Eintrag,
    // `HoldPhase` senkt die Phasen-Schwelle. Eine Vertauschung ist keine
    // kosmetische Aenderung, sondern eine andere Mechanik.
    for &(id, wirkung) in ERWARTET {
        let control = zauber(id)
            .control
            .unwrap_or_else(|| panic!("{id} traegt gar keine Kontroll-Wirkung"));
        assert_eq!(
            control.effect, wirkung,
            "{id} zielt auf die falsche Boss-Handlung"
        );
    }
}

/// AC2 — der Waechter ueber der Liste.
///
/// Er geht vom DATENSATZ aus und verlangt, dass jeder Traeger oben steht. Ein
/// neuer Zauber mit `control`-Feld macht ihn rot, bis er eingetragen ist.
#[test]
fn die_liste_fuehrt_jeden_traeger_des_datensatzes() {
    let im_datensatz: Vec<&str> = SPELLS
        .iter()
        .filter(|s| s.control.is_some())
        .map(|s| s.id)
        .collect();

    // **Die Zusicherung vor der Messung.** Faende der Filter nichts — etwa
    // weil das Feld anders heisst —, waere der Vergleich unten zwischen zwei
    // leeren Mengen und gruen, ohne einen einzigen Zauber beruehrt zu haben.
    assert!(
        im_datensatz.len() >= 6,
        "nur {} Kontroll-Traeger im Datensatz gefunden — der Filter greift nicht",
        im_datensatz.len()
    );

    let fehlen: Vec<&str> = im_datensatz
        .iter()
        .copied()
        .filter(|id| !ERWARTET.iter().any(|(e, _)| e == id))
        .collect();
    assert!(
        fehlen.is_empty(),
        "diese Zauber tragen eine Kontroll-Wirkung, stehen aber in keiner Zeile \
         von ERWARTET: {fehlen:?} — bis sie eingetragen sind, ist ihre \
         Boss-Handlung nur ueber Aufzeichnungen gedeckt, und die faerben nach \
         einem parity:export nicht mehr"
    );

    // Die Gegenrichtung: kein Eintrag ohne Zauber. Sonst bliebe eine Zeile
    // stehen, wenn ein Zauber verschwindet, und behauptete eine Deckung, die
    // es nicht gibt.
    let verwaist: Vec<&str> = ERWARTET
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !im_datensatz.contains(e))
        .collect();
    assert!(
        verwaist.is_empty(),
        "diese Eintraege haben keinen Zauber mehr im Datensatz: {verwaist:?}"
    );
}

/// **Die drei Wirkungen sind wirklich verschieden voneinander besetzt.**
///
/// Ohne diesen Fall belegte die Liste nur, dass jeder Zauber IRGENDEINE
/// Wirkung traegt. Kippten alle sechs auf `SlowCast`, waere `ERWARTET`
/// entsprechend angepasst und niemand saehe, dass die Mechanik ihre Vielfalt
/// verloren hat.
#[test]
fn jede_boss_handlung_hat_mindestens_einen_traeger() {
    for wirkung in [
        ControlEffect::SlowCast,
        ControlEffect::DelayStrike,
        ControlEffect::HoldPhase,
        ControlEffect::ConvertToDamage,
        ControlEffect::AllThree,
    ] {
        let treffer = SPELLS
            .iter()
            .filter_map(|s| s.control)
            .filter(|c| c.effect == wirkung)
            .count();
        assert!(
            treffer > 0,
            "keine Kontroll-Wirkung zielt mehr auf {wirkung:?} — entweder ist \
             ein Zauber umgebogen worden, oder die Handlung ist tot"
        );
    }
}
