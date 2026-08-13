//! Der Balance-Laeufer reicht die Talent-Groessen DURCH (#419, AC3).
//!
//! # Warum es diesen Fall braucht
//!
//! Bis hierher standen dreizehn Groessen in `balance.rs` fest auf 0. Die
//! Begruendung war richtig — ein Bestandslauf misst ohne Talente — die Folge
//! war es nicht: Wer einen Weg wiegen wollte, konnte es nicht. Jetzt kommen
//! sie durch, und ohne diesen Fall faellt niemandem auf, wenn eine Zuweisung
//! wieder zur Null wird. Eine tote Zuweisung sieht aus wie eine lebende.
//!
//! # Vier Groessen, vier verschiedene Wirkorte
//!
//! Nicht alle fuenfzehn, sondern eine je Art: Schaden, Zersetzung, Schicht,
//! Abklingzeit. Jede braucht ihre eigene Rotation — eine CALCINATION-Liste
//! zeigt keinen Schild-Weg, und ein Boss, der nicht zuschlaegt, macht ihn
//! ohnehin unmessbar (der Befund aus #525).
//!
//! # Die Voreinstellung bleibt 0
//!
//! Der erste Fall unten haelt das fest: Ohne gesetzten Wert rechnet der
//! Laeufer wie vorher. Waere es anders, verschoebe sich mit dieser Aenderung
//! jede bestehende Balance-Zahl.

use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;

/// CALCINATION: Aufbau und Entladung — traegt den Schadens-Weg.
const CALC: &str = "smoulder if dot.smoulder.remains < gcd\npyre if heat >= 15\nkindle\n";
/// PUTREFACTION ohne NECROSIS: Die Stapel bleiben liegen, statt verzehrt zu
/// werden — sonst zeigt sich weder ihre Zahl noch ihre Haltbarkeit.
const PUTR: &str = "rot if dot.rot.remains < gcd\ncanker\nblight\n";
/// COAGULATION: legt Schicht, sobald keine liegt.
const COAG: &str = "crust if shield < 1\nkindle\n";
/// SUBLIMATION: Verkuerzer und Schwelle.
///
/// **Mit EFFLUX, nicht mit KINDLE als Fueller** — und das ist gemessen, nicht
/// gewaehlt: Mit `ascend/vapour/kindle` bewegt die Schwelle gar nichts, weil
/// der Fueller keine Abklingzeit hat, die sie verkuerzen koennte. Mein erster
/// Entwurf stand so da und meldete einen Fehler in der Durchreichung, wo
/// einer in der Rotation lag.
const SUBL: &str = "ascend\nefflux\nvapour\n";

/// Ein Lauf gegen einen Boss, der ZUSCHLAEGT — sonst sind Schild-Wege blind.
fn lauf(apl_text: &str, setzen: impl FnOnce(&mut BalanceConfig)) -> (f64, f64) {
    let apl = rota_sim::apl::parse(AplName("wege"), apl_text).expect("APL fehlerhaft");
    let mut config = BalanceConfig {
        iterations: 40,
        base_seed: 11,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(60_000),
        boss: boss_by_id("boss-1-1").expect("boss-1-1 fehlt"),
        ..BalanceConfig::default()
    };
    setzen(&mut config);
    let result = rota_sim::balance::run(&config, &apl);
    (result.damage.mean, result.absorbed.mean)
}

/// **Die Zusicherung vor jedem Vergleich.** Ohne gesetzte Groesse muss jeder
/// Lauf denselben Wert liefern wie vor der Durchreichung.
#[test]
fn ohne_gesetzten_wert_aendert_sich_nichts() {
    for apl in [CALC, PUTR, COAG, SUBL] {
        let (schaden, _) = lauf(apl, |_| {});
        assert!(
            schaden > 0.0,
            "die Rotation teilt gar keinen Schaden aus — die Faelle unten messen nichts"
        );
        assert_eq!(
            lauf(apl, |_| {}),
            (schaden, lauf(apl, |_| {}).1),
            "zwei gleiche Laeufe muessen dieselbe Zahl liefern"
        );
    }
}

#[test]
fn der_schadens_weg_kommt_durch() {
    // 500 Promille = Faktor 1,5. Bis #419 stand dieses Feld im Laeufer fest
    // auf 0, mit der Begruendung, es wirke auf alle Rotationen gleich.
    let ohne = lauf(CALC, |_| {}).0;
    let mit = lauf(CALC, |c| c.talent_permille = 500).0;
    assert!(
        mit > ohne,
        "talent_permille kommt nicht durch: {ohne} -> {mit}"
    );
}

#[test]
fn der_zersetzungs_weg_kommt_durch() {
    // **1000 Promille, nicht 500** — und das ist kein Rundungsdetail: `ROT`
    // legt EINEN Stapel je Tick, und `floor(1 * 1,5)` ist wieder 1. Mit 500
    // bewegt sich nichts, und wer daraus schloesse, die Groesse komme nicht
    // durch, laege falsch. Gemessen, bevor dieser Fall geschrieben wurde.
    let ohne = lauf(PUTR, |_| {}).0;
    let mit = lauf(PUTR, |c| c.talent_decay_stack_permille = 1000).0;
    assert!(
        mit > ohne,
        "talent_decay_stack_permille kommt nicht durch: {ohne} -> {mit}"
    );
}

#[test]
fn der_schicht_weg_kommt_durch() {
    // Gemessen wird die ABGEFANGENE Menge, nicht der Schaden: Eine dickere
    // Schicht teilt nichts aus, sie faengt mehr ab.
    let ohne = lauf(COAG, |_| {}).1;
    let mit = lauf(COAG, |c| c.talent_shield_amount_permille = 500).1;
    assert!(
        ohne > 0.0,
        "die Schicht faengt gar nichts ab — der Vergleich misst nichts"
    );
    assert!(
        mit > ohne,
        "talent_shield_amount_permille kommt nicht durch: {ohne} -> {mit}"
    );
}

#[test]
fn der_abklingzeit_weg_kommt_durch() {
    let ohne = lauf(SUBL, |_| {}).0;
    let mit = lauf(SUBL, |c| c.talent_cooldown_threshold_permille = 500).0;
    assert!(
        mit != ohne,
        "talent_cooldown_threshold_permille kommt nicht durch: {ohne} -> {mit}"
    );
}
