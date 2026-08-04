//! Ein reiner Aufbau gegen einen gemischten (Issue #178, AC4 und AC5).
//!
//! # Dieser Prueffall MISST, er entscheidet nicht
//!
//! Die Auflage sagt es woertlich: *"Ob der reine Aufbau besser oder schlechter
//! ist, beantwortet #118 — dieses Issue liefert die Zahlen, an denen gemessen
//! wird. Ein Ergebnis in die eine oder andere Richtung ist hier **kein**
//! Fehlschlag."*
//!
//! Geprueft wird deshalb, **dass der Vergleich laeuft und aussagekraeftig ist**:
//! beide Aufbauten mit demselben Budget, beide ueber dieselbe Kampfzeit, und
//! ein Ergebnis, das sich vom Nichts-Tragen unterscheidet. Was dabei
//! herauskommt, steht im Bericht — nicht in einer Zusicherung.
//!
//! # Warum das nicht in `balance.rs` gehoert
//!
//! Der Balance-Laeufer mittelt ueber viele Durchgaenge, weil PURITY echten
//! Zufall traegt (6.5). Hier geht es um die Frage davor: Kommt bei zwei
//! verschiedenen Aufbauten ueberhaupt Verschiedenes heraus? Die beantwortet ein
//! einzelner Durchgang, und zwar schneller und eindeutiger.

use rota_sim::gear::{WornValues, MAX_ITEM_LEVEL};
use rota_sim::rotation;
use rota_sim::sources::{
    build_total, mixed_build, permanent_max, pure_build, STAT_CATALYSIS, STAT_COUNT, STAT_NAMES,
    STAT_PURITY, STAT_YIELD,
};
use rota_sim::{Fight, FightConfig, RunResult};

/// Ein Durchgang ueber die volle Kampfzeit.
///
/// **`stop_on_kill: false`** — sonst faellt der Boss beim staerkeren Aufbau
/// frueher, und der kuerzere Kampf sieht aus wie weniger Wirkung
/// (`STEP1-SPEC` § 4).
fn lauf(worn: WornValues) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        worn,
        ..FightConfig::default()
    })
    .run(rotation::good)
}

#[test]
fn ohne_getragene_ausruestung_bleibt_die_referenz_stehen() {
    // **AC5.** Die Aufbauten oben sind Zahlenwerk; solange niemand sie traegt,
    // darf sich nichts bewegen.
    let r = Fight::new(FightConfig {
        worn: WornValues::none(),
        ..FightConfig::default()
    })
    .run(rotation::good);

    assert_eq!(r.damage, 4240, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, Some(40_510), "Sieg-Zeitpunkt");
    assert_eq!(r.random_draws, 0, "Ziehungen");
}

#[test]
fn der_vergleich_laeuft_ueberhaupt() {
    // **AC4, der Kern.** Beide Aufbauten muessen im Kampf ankommen und sich
    // vom Nichts-Tragen unterscheiden. Ohne diese Zeile pruefte der Rest nur,
    // ob zwei Nullen gleich sind.
    let ohne = lauf(WornValues::none());
    let rein = lauf(pure_build(STAT_CATALYSIS, MAX_ITEM_LEVEL));
    let gemischt = lauf(mixed_build(STAT_CATALYSIS, MAX_ITEM_LEVEL));

    assert_ne!(rein.damage, ohne.damage, "der reine Aufbau bewegt nichts");
    assert_ne!(
        gemischt.damage, ohne.damage,
        "der gemischte Aufbau bewegt nichts"
    );
    // Und die beiden unterscheiden sich voneinander — sonst waere der Vergleich
    // eine Tautologie.
    assert_ne!(
        rein.damage, gemischt.damage,
        "beide Aufbauten ergeben denselben Schaden — der Vergleich misst nichts"
    );
}

#[test]
fn beide_aufbauten_bekommen_dasselbe_budget() {
    // **Sonst verglichen man nicht zwei Aufbauten, sondern zwei Budgets.**
    for (stat, name) in STAT_NAMES.iter().enumerate() {
        let a = build_total(&pure_build(stat, MAX_ITEM_LEVEL));
        let b = build_total(&mixed_build(stat, MAX_ITEM_LEVEL));
        assert!(
            b <= a && a - b < STAT_COUNT as u32,
            "Bezugswert {name}: {a} gegen {b}"
        );
    }
}

#[test]
fn beide_rechnen_ueber_die_volle_zeit_weiter_auch_nach_dem_sieg() {
    // Der zweite Weg, auf dem ein Vergleich unfair wird: Ein Aufbau, der den
    // Boss frueher legt, rechnet kuerzer und sieht dadurch schlechter aus, als
    // er ist (`STEP1-SPEC` § 4). `stop_on_kill: false` verhindert das.
    //
    // **Die erste Fassung verglich `remaining_ms` der beiden Laeufe und fiel.**
    // Zu Recht: Dieses Feld sagt, WANN der Boss fiel, und das ist bei einem
    // staerkeren Aufbau natuerlich frueher — es misst also gerade den
    // Unterschied, den es belegen sollte.
    //
    // Der richtige Nachweis ist ein anderer: Der ausgeteilte Schaden liegt
    // ueber den Lebenspunkten des Bosses. Das kann nur sein, wenn nach dem Sieg
    // weitergerechnet wurde.
    let hp = rota_sim::BOSS.hp;
    for aufbau in [
        WornValues::none(),
        pure_build(STAT_CATALYSIS, MAX_ITEM_LEVEL),
        mixed_build(STAT_CATALYSIS, MAX_ITEM_LEVEL),
    ] {
        let r = lauf(aufbau);
        assert!(
            r.damage > hp,
            "der Durchgang hat beim Sieg aufgehoert: {} gegen {hp} Lebenspunkte",
            r.damage
        );
        assert!(
            r.killed_at_ms.is_some(),
            "der Boss faellt in diesem Aufbau gar nicht"
        );
    }
}

#[test]
fn der_reine_aufbau_traegt_wirklich_nur_einen_wert() {
    for (stat, name) in STAT_NAMES.iter().enumerate() {
        let b = pure_build(stat, MAX_ITEM_LEVEL);
        let werte = [b.catalysis, b.purity, b.yield_bonus];
        assert!(werte[stat] > 0, "{name} ist leer");
        for (index, wert) in werte.iter().enumerate() {
            if index != stat {
                assert_eq!(
                    *wert, 0,
                    "reiner {name}-Aufbau traegt auch {}",
                    STAT_NAMES[index]
                );
            }
        }
    }
}

#[test]
fn ein_reiner_yield_aufbau_ist_wirkungslos_ohne_purity() {
    // **Der Fall, der die Struktur belegt.** YIELD erhoeht den Kritschaden —
    // wer nie kritet, hat davon nichts. Ein reiner YIELD-Aufbau ist deshalb
    // exakt so stark wie gar keine Ausruestung.
    //
    // Das ist keine Schwaeche des Modells, sondern genau die Abwaegung, die 6.7
    // dem Spieler zumuten will. Und es ist der Beleg, dass der Vergleich die
    // Wechselwirkung zwischen den Werten wirklich abbildet.
    let ohne = lauf(WornValues::none());
    let nur_yield = lauf(pure_build(STAT_YIELD, MAX_ITEM_LEVEL));
    assert_eq!(nur_yield.damage, ohne.damage);

    // Der Gegenfall: MIT Kritchance wirkt derselbe YIELD-Wert sehr wohl.
    let mit_beidem = lauf(WornValues {
        purity: permanent_max(STAT_PURITY, MAX_ITEM_LEVEL),
        yield_bonus: permanent_max(STAT_YIELD, MAX_ITEM_LEVEL),
        ..WornValues::none()
    });
    let nur_purity = lauf(pure_build(STAT_PURITY, MAX_ITEM_LEVEL));
    assert!(
        mit_beidem.damage > nur_purity.damage,
        "YIELD bringt neben PURITY nichts: {} gegen {}",
        mit_beidem.damage,
        nur_purity.damage
    );
}

#[test]
fn das_ergebnis_wird_berichtet_nicht_zugesichert() {
    // **Die Auflage in Code:** Hier steht bewusst KEINE Zusicherung darueber,
    // welcher Aufbau gewinnt. Der Prueffall druckt das Ergebnis, damit es beim
    // Balance-Lauf (#118) einen Ausgangspunkt gibt — und faellt nicht, wenn es
    // sich dreht.
    let ohne = lauf(WornValues::none());
    println!("\n  Aufbau-Vergleich, volle Kampfzeit, rotation::good");
    println!("  {:<24} {:>10} {:>10}", "Aufbau", "Schaden", "Ziehungen");
    println!(
        "  {:<24} {:>10} {:>10}",
        "nichts getragen", ohne.damage, ohne.random_draws
    );
    for (stat, name) in STAT_NAMES.iter().enumerate() {
        let r = lauf(pure_build(stat, MAX_ITEM_LEVEL));
        println!(
            "  {:<24} {:>10} {:>10}",
            format!("rein {name}"),
            r.damage,
            r.random_draws
        );
    }
    let g = lauf(mixed_build(STAT_CATALYSIS, MAX_ITEM_LEVEL));
    println!(
        "  {:<24} {:>10} {:>10}",
        "gemischt (Tempo-Budget)", g.damage, g.random_draws
    );
    println!();

    // Die einzige Zusicherung: Es kommt ueberhaupt etwas heraus.
    assert!(ohne.damage > 0);
}
