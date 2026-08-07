//! Der Pruefpunkt fuer die neue Rotation (Issue #224).
//!
//! # Er haelt einen Befund fest, keine Erwartung
//!
//! Die uebliche Fassung waere "step3-good schlaegt step1-good". Sie waere
//! heute rot, und zwar zu Recht: **Die neue Rotation ist genau dann am besten,
//! wenn alle vier neuen Zauber unter KINDLE stehen und nie gewirkt werden.**
//!
//! Ein Prueffall, der eine unerfuellte Erwartung setzt, ist rot ohne Fehler —
//! und wird dann abgeschaltet oder gelockert. Dieser hier schreibt stattdessen
//! auf, was heute gilt. Aendert jemand die Zauber, die Abklingzeiten oder den
//! Zersetzungs-Bonus, faellt er auf: **in beide Richtungen.**
//!
//! # Warum ohne Ausruestung gemessen wird
//!
//! Hier ist es schaerfer. Ohne PURITY wuerfelt nichts, die Streuung ist null,
//! und jeder Unterschied ist echt statt Rauschen.
//!
//! Der Vergleich MIT Ausruestung gehoert in den Bericht (AC3) und misst etwas
//! anderes: dort geht es um die alte Reihe aus #118, und die lief mit
//! purity 800. Wer die beiden Messungen vermischt, vergleicht Ausruestung
//! statt Rotationen.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig};

const ALT: &str = include_str!("../apl/step1-good.apl");
const NEU: &str = include_str!("../apl/step3-good.apl");

fn schaden(name: &str, text: &str) -> f64 {
    let apl: Apl = parse(AplName(name), text).expect("die eingebaute Liste ist gueltig");
    run(
        &BalanceConfig {
            iterations: 50,
            stop_on_kill: false,
            ..BalanceConfig::default()
        },
        &apl,
    )
    .damage
    .mean
}

#[test]
fn beide_listen_sind_ueberhaupt_lesbar() {
    // Ohne diese Zeile koennte eine kaputte Liste als "gleichauf" durchgehen —
    // `parse` wuerde erst im Vergleich unten scheitern, und die Meldung zeigte
    // auf den Vergleich statt auf die Datei.
    assert!(parse(AplName("step1-good"), ALT).is_ok());
    assert!(parse(AplName("step3-good"), NEU).is_ok());
}

#[test]
fn die_neue_liste_kennt_die_neuen_zauber() {
    // **Der Gegenfall zum ganzen Prueffall.** Ohne ihn bliebe er auch dann
    // gruen, wenn step3-good eine Abschrift von step1-good waere — und
    // "gleichauf" haette dann gar nichts zu bedeuten.
    for zauber in ["vapour", "reflux", "canker", "fester"] {
        assert!(
            NEU.contains(zauber),
            "step3-good nennt {zauber} nicht — sie prueft die neuen Zauber nicht"
        );
        assert!(
            !ALT.contains(zauber),
            "step1-good nennt {zauber} — dann ist sie kein Vergleichspunkt mehr"
        );
    }
}

#[test]
fn die_neue_liste_holt_die_alte_ein_ohne_sie_zu_schlagen() {
    // **Der Befund vom 2026-08-05, festgeschrieben.**
    //
    // Gemessen: beide 5960,0 ohne Ausruestung. Die neue Liste erreicht die
    // alte genau dann, wenn ihre vier zusaetzlichen Zauber unter KINDLE stehen
    // und deshalb nie an die Reihe kommen.
    //
    // Die Ursache steht im Bericht (`examples/rotation-balance.rs`): KINDLEs
    // Abklingzeit IST die globale Wartezeit, er ist also immer bereit — und er
    // macht mit 100 den hoechsten Schaden im Kasten. Es gibt nie ein Fenster,
    // in dem ein Werkzeug-Zauber die bessere Wahl waere.
    let alt = schaden("step1-good", ALT);
    let neu = schaden("step3-good", NEU);

    assert!(alt > 0.0, "die alte Liste teilt gar keinen Schaden aus");
    assert!(
        (neu - alt).abs() < 1.0,
        "die Listen liegen nicht mehr gleichauf: alt {alt}, neu {neu}\n\
         Das ist kein Fehler, sondern eine Aenderung — pruefe, ob ein Zauber, \
         eine Abklingzeit oder der Zersetzungs-Bonus angefasst wurde, und \
         schreib den Bericht neu."
    );
}
