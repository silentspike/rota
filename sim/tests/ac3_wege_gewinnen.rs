//! AC3 fuer die Baeume, in denen er heute erfuellt ist (#419).
//!
//! # Was AC3 verlangt
//!
//! „Der Balance-Laeufer belegt, dass KEINER der Wege tot ist — jeder ist in
//! mindestens einer Zusammenstellung die beste Wahl." Verschaerft durch den
//! Orchestrator: Die Zusammenstellung muss SPIELBAR sein.
//!
//! # Die Schwelle: 10 Prozent
//!
//! Eine Rotation zaehlt, wenn ihr Schaden OHNE Talente hoechstens 10 % unter
//! der besten hier verglichenen liegt. Die Zahl ist nicht erfunden: Sie steht
//! als Kriterium in GAME-DESIGN 20.3. Eine zweite Schwelle daneben haette
//! begruendet werden muessen, eine bestehende traegt sich selbst.
//!
//! Ohne sie waere AC3 wertlos zu erfuellen — jeder Weg gewinnt in einer
//! Rotation, die eigens so verkrueppelt wurde, dass nur er wirkt. Gemessen:
//! `dotDuration` gewinnt in einer Rotation ohne Ausgeber, die 23 % zurueckliegt.
//!
//! # Was diese Datei NICHT abdeckt
//!
//! Nur CALCINATION und der globale Baum stehen hier, und im globalen nur zwei
//! von drei Wegen. Fuer die uebrigen ist AC3 heute NICHT erfuellt — gemessen,
//! gemeldet, und teils aus Gruenden, die keine Talent-Frage sind:
//!
//! ```text
//!   putrefaction build      wirkungslos: floor(1 x 1,5) = 1
//!   putrefaction duration   von `effect` in jeder Rotation geschlagen
//!   sublimation  reduction  gewinnt nirgends
//!   sublimation  charges    bewegt gar nichts
//!   coagulation  longer     bewegt gar nichts
//!   coagulation  stacks     gewinnt nur 44,6 % unter der besten Rotation
//!   fixation     alle drei  die Mechanik aendert Summen nicht, nur Verteilung
//!   global       build      strukturell von `release` geschlagen
//! ```
//!
//! Diese Datei behauptet also nicht, AC3 sei erfuellt. Sie haelt fest, WO er
//! es ist — damit der Teil nicht unbemerkt zurueckfaellt, waehrend am Rest
//! gearbeitet wird.

use rota_sim::apl::{Apl, AplName};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;

/// Entlader-schwer: BELLOWS speist, PYRE zuendet, ein Brand laeuft mit.
const ENTLADER: &str =
    "smoulder if dot.smoulder.remains < gcd\npyre if heat >= 15\nbellows\nkindle\n";
/// Brand-schwer: drei Braende am Laufen halten.
const BRAENDE: &str = "smoulder if dot.smoulder.remains < gcd\nsearing if dot.searing.remains < gcd\ncinder if dot.cinder.remains < gcd\nkindle\n";
/// Die Rotation aus `STEP1-SPEC` § 4.
const STANDARD: &str = "smoulder if dot.smoulder.remains < gcd\npyre if heat >= 15\nkindle\n";

fn liste(text: &str) -> Apl {
    rota_sim::apl::parse(AplName("ac3"), text).expect("APL fehlerhaft")
}

fn schaden(text: &str, setzen: impl FnOnce(&mut BalanceConfig)) -> f64 {
    let mut config = BalanceConfig {
        iterations: 60,
        base_seed: 5,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(60_000),
        boss: boss_by_id("boss-1-1").expect("boss-1-1 fehlt"),
        ..BalanceConfig::default()
    };
    setzen(&mut config);
    rota_sim::balance::run(&config, &liste(text)).damage.mean
}

/// **Die Zusicherung vor jedem Vergleich:** Alle drei Rotationen sind spielbar.
///
/// Ohne sie belegte „gewinnt in Rotation X" nichts — X koennte eine sein, die
/// niemand spielt.
#[test]
fn alle_drei_rotationen_liegen_innerhalb_der_schwelle() {
    let werte = [
        ("ENTLADER", schaden(ENTLADER, |_| {})),
        ("BRAENDE", schaden(BRAENDE, |_| {})),
        ("STANDARD", schaden(STANDARD, |_| {})),
    ];
    let beste = werte.iter().map(|(_, w)| *w).fold(f64::MIN, f64::max);
    assert!(beste > 0.0, "keine Rotation teilt Schaden aus");
    for (name, wert) in werte {
        let abstand = (beste - wert) / beste * 100.0;
        assert!(
            abstand <= 10.0,
            "{name} liegt {abstand:.1} % zurueck und ist damit kein Beleg (Schwelle 10 %)"
        );
    }
}

/// CALCINATION: drei Wege, drei Rotationen, drei verschiedene Sieger.
#[test]
fn jeder_calcination_weg_gewinnt_irgendwo() {
    // Fuenf Raenge je Weg, mit den Promille aus `talents.json`:
    // build 100, release 25, hold 100.
    let sieger = |text: &str| -> &'static str {
        let build = schaden(text, |c| c.talent_resource_permille = 500);
        let release = schaden(text, |c| c.talent_permille = 125);
        let hold = schaden(text, |c| c.talent_dot_duration_permille = 500);
        if build >= release && build >= hold {
            "build"
        } else if release >= hold {
            "release"
        } else {
            "hold"
        }
    };
    assert_eq!(
        sieger(STANDARD),
        "build",
        "der Aufbau-Weg gewinnt nirgends mehr"
    );
    assert_eq!(
        sieger(ENTLADER),
        "release",
        "der Schadens-Weg gewinnt nirgends mehr"
    );
    assert_eq!(
        sieger(BRAENDE),
        "hold",
        "der Laufzeit-Weg gewinnt nirgends mehr"
    );
}

/// GLOBAL: `release` und `hold` gewinnen je eine Rotation.
///
/// **`build` steht bewusst nicht hier.** Er gewinnt in keiner der drei — und
/// das ist strukturell: Mehr Hitze wirkt ueber den Entlader, der Zuschlag
/// wirkt direkt auf ihn. Der direkte Weg schlaegt den indirekten, solange
/// beide am selben Zauber haengen. Gemessen, nicht vermutet.
#[test]
fn zwei_globale_wege_gewinnen_je_eine_rotation() {
    let sieger = |text: &str| -> &'static str {
        let build = schaden(text, |c| {
            c.talent_resource_permille = 300;
            c.talent_decay_stack_permille = 300;
            c.talent_control_gain_permille = 300;
        });
        let release = schaden(text, |c| c.talent_spender_damage_permille = 500);
        let hold = schaden(text, |c| {
            c.talent_dot_duration_permille = 300;
            c.talent_decay_duration_permille = 300;
            c.talent_shield_duration_permille = 300;
        });
        if release >= build && release >= hold {
            "release"
        } else if hold >= build {
            "hold"
        } else {
            "build"
        }
    };
    assert_eq!(
        sieger(ENTLADER),
        "release",
        "der Entlader-Weg gewinnt nirgends mehr"
    );
    assert_eq!(
        sieger(BRAENDE),
        "hold",
        "der Haltbarkeits-Weg gewinnt nirgends mehr"
    );
}
