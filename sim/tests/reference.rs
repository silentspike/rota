//! Gleichlauf mit der Referenz — die Sollwerte aus `STEP1-SPEC.md` § 4.
//!
//! **Der harte Pruefstein gehoert zu Issue #54**, nicht zu #53. Er steht hier
//! trotzdem schon, aus einem einfachen Grund: Ein Simulator, der die Regeln
//! anders rechnet als Spiel und Referenz, ist kein halbfertiger Simulator,
//! sondern ein falscher. Die Zahlen jetzt zu pruefen kostet nichts und
//! verhindert, dass E2-2-bis-E2-5 auf einer schiefen Grundlage entstehen.
//!
//! Die Sollwerte sind bewusst als Zahlen ausgeschrieben und nicht zur Laufzeit
//! aus der Referenz gezogen — dieselbe Begruendung wie in
//! `tests/parity/step1.test.ts` auf der Spiel-Seite: *"Ein Pruefall, der seinen
//! Sollwert aus dem Prüfling zieht, prueft nichts."*
//!
//! Massgeblich bleibt der frische Lauf von `python3 tools/step1-check.py`.

use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::rotation;
use rota_sim::{Fight, FightConfig, RunResult};

/// Ein Durchgang bis zum Sieg, so wie ihn das Spiel rechnet.
fn lauf(apl: rota_sim::fight::Rotation) -> RunResult {
    Fight::new(FightConfig::default()).run(apl)
}

/// Ein Durchgang ueber die vollen 60 s, ohne Abbruch beim Sieg — nur so lassen
/// sich zwei Rotationen fair vergleichen.
fn voller_lauf(apl: rota_sim::fight::Rotation) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run(apl)
}

#[test]
fn gute_rotation_trifft_die_referenz() {
    let r = lauf(rotation::good);

    assert_eq!(r.damage, 4240, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, Some(40_510), "Sieg-Zeitpunkt");
    assert_eq!(r.remaining_ms, 19_490, "Reserve");
    assert_eq!(r.casts[KINDLE], 20, "KINDLE-Wirkungen");
    assert_eq!(r.casts[SMOULDER], 4, "SMOULDER-Wirkungen");
    assert_eq!(r.casts[PYRE], 4, "PYRE-Wirkungen");
    assert_eq!(r.damage_by_spell[PYRE], 1440, "PYRE-Schaden");
    assert_eq!(r.damage_by_spell[SMOULDER], 800, "SMOULDER-Ticks");
    // Dieselbe Zeile prueft `tests/parity/step1.test.ts` auf der Spiel-Seite:
    // Der Rest ist KINDLE, 20 Wirkungen à 100.
    assert_eq!(r.damage_by_spell[KINDLE], 2000, "KINDLE-Schaden");
    assert_eq!(r.lost_dot_ticks[SMOULDER], 0, "verlorene DoT-Ticks");
    assert_eq!(r.resource_overflow, 0, "verpuffte Hitze");
    assert_eq!(r.idle_ms, 0, "Leerlauf");
    assert_eq!(r.random_draws, 0, "Ziehungen — in Schritt 1 gibt es keine");
}

#[test]
fn schlechte_rotation_trifft_die_referenz() {
    let r = lauf(rotation::bad);

    assert_eq!(r.damage, 4044, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, None, "der Boss muss ueberleben");
    assert_eq!(r.casts[KINDLE], 24);
    assert_eq!(r.casts[SMOULDER], 8);
    assert_eq!(r.casts[PYRE], 8);
    assert_eq!(r.damage_by_spell[PYRE], 684, "PYRE-Schaden");
    assert_eq!(r.damage_by_spell[SMOULDER], 960, "SMOULDER-Ticks");
    assert_eq!(r.damage_by_spell[KINDLE], 2400, "KINDLE-Schaden");
    assert_eq!(r.lost_dot_ticks[SMOULDER], 14, "verlorene DoT-Ticks");
    assert_eq!(r.resource_overflow, 0);
    assert_eq!(r.idle_ms, 0);
}

#[test]
fn der_boss_trennt_die_beiden_rotationen() {
    // Das ist der eigentliche Zweck des Testbosses (STEP1-SPEC § 3): Die gute
    // Rotation legt ihn, die schlechte scheitert knapp bei 96,3 %.
    let gut = lauf(rotation::good);
    let schlecht = lauf(rotation::bad);

    assert!(gut.killed_at_ms.is_some());
    assert!(schlecht.killed_at_ms.is_none());

    let anteil = f64::from(schlecht.damage) / f64::from(rota_sim::BOSS.hp) * 100.0;
    assert!(
        (96.0..96.5).contains(&anteil),
        "die schlechte Rotation kommt auf {anteil:.1} %, erwartet sind 96,3 %"
    );
}

#[test]
fn der_vergleich_ueber_die_vollen_60_sekunden_stimmt() {
    let gut = voller_lauf(rotation::good).damage;
    let schlecht = voller_lauf(rotation::bad).damage;
    let nur_dot = voller_lauf(rotation::dot_error_only).damage;
    let nur_pyre = voller_lauf(rotation::pyre_error_only).damage;

    assert_eq!(gut, 5960, "gute Rotation ueber 60 s");
    assert_eq!(nur_dot, 5368, "nur der DoT-Fehler");
    assert_eq!(nur_pyre, 4524, "nur der PYRE-Fehler");
    assert_eq!(schlecht, 4044, "beide Fehler");

    // Das Abnahme-Kriterium aus GAME-DESIGN 20.3: mindestens 10 % Unterschied
    // zwischen guter und schlechter Rotation. Erreicht werden 47,4 %.
    let abstand = f64::from(gut - schlecht) / f64::from(schlecht) * 100.0;
    assert!(
        abstand >= 10.0,
        "Koennen lohnt sich nur um {abstand:.1} %, verlangt sind mindestens 10 %"
    );
    assert!(
        (47.3..47.5).contains(&abstand),
        "erwartet sind 47,4 %, gemessen wurden {abstand:.1} %"
    );
}

#[test]
fn der_pyre_bonus_greift_ab_15_und_nicht_ab_14() {
    // Aus STEP1-SPEC § 6, Zeile 3: "PYRE verdoppelt ab genau 15, nicht ab 14."
    // Der Sollwert steht dort als Beispiel: 14 Hitze -> 168, 15 Hitze -> 360.
    let spender = rota_sim::SPELLS[PYRE].spender.expect("pyre entlaedt");
    let bei_14 = spender.damage_per_point * 14;
    let bei_15 = spender.damage_per_point * 15 * spender.bonus_multiplier;

    assert_eq!(bei_14, 168);
    assert_eq!(bei_15, 360);
    assert_eq!(spender.bonus_threshold, 15);
}

#[test]
fn der_brand_effekt_tickt_sechsmal() {
    // Die am 2026-07-28 getroffene Entscheidung aus STEP1-SPEC 1.2: 12 s Laufzeit
    // bei 2 s Abstand ergeben SECHS Ticks, der letzte genau beim Ablauf.
    let dot = rota_sim::SPELLS[SMOULDER].dot.expect("smoulder tickt");
    assert_eq!(dot.duration_ms / dot.tick_interval_ms, 6);

    let mut fight = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let r = fight.run(rotation::good);

    // Vier Anwendungen ohne einen einzigen verlorenen Tick — die letzte wird vom
    // Kampfende abgeschnitten, deshalb sind es nicht 4 x 6 = 24 Ticks.
    let ticks = r.damage_by_spell[SMOULDER] / dot.tick_damage;
    assert_eq!(r.lost_dot_ticks[SMOULDER], 0);
    assert_eq!(
        ticks, 29,
        "erwartet werden 29 Ticks ueber die volle Kampfzeit"
    );
}
