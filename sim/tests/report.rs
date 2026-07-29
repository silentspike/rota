//! Die drei Auswertungen (Issue #57).
//!
//! * **AC1** Zauber-Vergleichstabelle mit Beitrag je Zauber
//! * **AC2** Machbarkeit von THE FIRST CRUCIBLE mit der ausgewiesenen Reserve
//! * **AC3** Die Prüfung muss **nein sagen können**
//!
//! AC4 (Anbindung an `nightly.yml`) lässt sich nicht als Test formulieren — der
//! Nachweis dafür ist der Workflow-Lauf selbst.

use rota_sim::apl::{parse, Apl};
use rota_sim::report::{
    achievement_run, feasibility, is_shippable, meaningless, spell_table, time_tiers,
};
use rota_sim::{Fight, FightConfig};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");
/// Die schlechte Rotation, wie sie im Repository liegt.
const BAD: &str = include_str!("../apl/step1-bad.apl");

fn apl(text: &str, name: &str) -> Apl {
    parse(name, text).expect("die eingebauten Listen sind gültig")
}

#[test]
fn ac1_die_tabelle_nennt_alle_drei_zauber_mit_beitrag() {
    let result = Fight::new(FightConfig::default()).run_apl(&apl(GOOD, "gut"));
    let table = spell_table(&result);

    assert_eq!(table.len(), 3, "alle drei Zauber muessen aufgefuehrt sein");

    // Nach Schaden sortiert: KINDLE 2000, PYRE 1440, SMOULDER 800.
    assert_eq!(table[0].id, "kindle");
    assert_eq!(table[0].damage, 2000);
    assert_eq!(table[0].casts, 20);
    assert_eq!(table[1].id, "pyre");
    assert_eq!(table[1].damage, 1440);
    assert_eq!(table[1].casts, 4);
    assert_eq!(table[2].id, "smoulder");
    assert_eq!(table[2].damage, 800);
    assert_eq!(table[2].casts, 4);

    // Die Anteile ergeben zusammen das Ganze — sonst fehlt eine Schadensquelle
    // in der Tabelle, und genau dafuer ist sie da.
    let summe: f64 = table.iter().map(|row| row.share).sum();
    assert!(
        (summe - 1.0).abs() < 1e-9,
        "die Anteile ergeben {summe}, erwartet 1,0"
    );

    // Schaden je Wirkung: KINDLE 100, PYRE 360, SMOULDER 200 (fuenf Ticks
    // à 40 je Anwendung).
    assert!((table[0].damage_per_cast - 100.0).abs() < 1e-9);
    assert!((table[1].damage_per_cast - 360.0).abs() < 1e-9);
    assert!((table[2].damage_per_cast - 200.0).abs() < 1e-9);
}

#[test]
fn ac1_kein_zauber_aus_schritt_1_ist_tot() {
    // Der eigentliche Zweck der Tabelle (Leitplanke 2 des Auftraggebers): tote
    // Zauber aufdecken, die niemand nutzt. In Schritt 1 darf keiner tot sein —
    // waere einer es, waeren die drei Zauber falsch bemessen.
    let result = Fight::new(FightConfig::default()).run_apl(&apl(GOOD, "gut"));
    for row in spell_table(&result) {
        assert!(!row.is_dead(), "{} wird nie gewirkt", row.id);
    }
}

#[test]
fn ac1_eine_rotation_die_einen_zauber_auslaesst_zeigt_ihn_als_tot() {
    // Die Gegenprobe: Ohne sie wuesste niemand, ob `is_dead` ueberhaupt
    // ansprechen kann.
    let nur_kindle = apl("kindle\n", "nur-kindle");
    let result = Fight::new(FightConfig {
        stop_on_kill: false,
        ..FightConfig::default()
    })
    .run_apl(&nur_kindle);
    let table = spell_table(&result);

    let tote: Vec<&str> = table
        .iter()
        .filter(|row| row.is_dead())
        .map(|row| row.id)
        .collect();
    assert_eq!(tote.len(), 2, "SMOULDER und PYRE kommen nicht vor");
    assert!(tote.contains(&"smoulder"));
    assert!(tote.contains(&"pyre"));
    assert!(!table.iter().any(|row| row.id == "kindle" && row.is_dead()));
}

#[test]
fn ac2_the_first_crucible_faellt_mit_der_ausgewiesenen_reserve() {
    let f = feasibility(&apl(GOOD, "gut"), None);

    assert_eq!(f.boss, "The First Crucible");
    assert!(f.defeated, "der Boss muss mit der guten Rotation fallen");
    assert_eq!(
        f.kill_time_ms,
        Some(40_510),
        "Sieg-Zeitpunkt aus STEP1-SPEC § 4"
    );
    assert_eq!(f.limit_ms, 60_000, "Ueberdruck-Timer");
    assert_eq!(f.reserve_ms, 19_490, "Reserve aus STEP1-SPEC § 4");
    assert!(
        (f.reserve_share - 19_490.0 / 60_000.0).abs() < 1e-9,
        "Reserve als Anteil"
    );
    assert_eq!(f.damage, 4240);
}

#[test]
fn ac2_die_schlechte_rotation_schafft_ihn_nicht() {
    // Derselbe Boss, dieselbe Pruefung, anderes Urteil — das ist der Beleg
    // dafuer, dass die Machbarkeit an der Rotation haengt und nicht am Boss
    // allein.
    let f = feasibility(&apl(BAD, "schlecht"), None);
    assert!(!f.defeated);
    assert_eq!(f.kill_time_ms, None);
    assert_eq!(f.reserve_ms, 0);
    assert_eq!(f.damage, 4044);
    assert!(
        (f.hp_share - 4044.0 / 4200.0).abs() < 1e-9,
        "96,3 % der Lebenspunkte abgetragen"
    );
}

#[test]
fn ac3_die_pruefung_kann_nein_sagen() {
    // **Eine Pruefung, die nur ja sagen kann, prueft nichts.**
    //
    // Hergestellt ueber ein zu kurzes Zeitlimit: Der Boss braucht 40.510 ms,
    // bekommt aber nur 10.000. Ob die Lage ueber zu wenig Zeit oder zu viele
    // Lebenspunkte unschaffbar wird, ist fuer die Aussage gleichgueltig — beides
    // ist eine Lage, die nachweislich nicht zu schaffen ist.
    let f = feasibility(&apl(GOOD, "gut"), Some(10_000));

    assert!(!f.defeated, "in 10 s ist der Boss nicht zu schaffen");
    assert_eq!(f.kill_time_ms, None);
    assert_eq!(f.limit_ms, 10_000);
    assert!(
        f.damage < 4200,
        "in der kurzen Zeit kann nicht der volle Schaden entstehen"
    );

    // Und der Pruefflauf zieht daraus das richtige Urteil.
    let checks = achievement_run(&apl(GOOD, "gut"), Some(10_000));
    assert!(
        !is_shippable(&checks),
        "eine unschaffbare Lage darf nicht freigegeben werden"
    );
    let sieg = checks
        .iter()
        .find(|c| c.name == "Sieg")
        .expect("Sieg-Eintrag");
    assert!(!sieg.solved);
    assert!(
        sieg.evidence.contains("nicht besiegt"),
        "die Begruendung muss den Grund nennen: {}",
        sieg.evidence
    );
}

#[test]
fn ac3_die_grenze_zwischen_schaffbar_und_nicht_liegt_an_der_bestzeit() {
    // Genau ein Takt entscheidet. Das ist der schaerfste Nachweis dafuer, dass
    // die Pruefung wirklich rechnet und nicht raet.
    let gut = apl(GOOD, "gut");
    assert!(
        feasibility(&gut, Some(40_520)).defeated,
        "einen Takt nach der Bestzeit muss es reichen"
    );
    assert!(
        !feasibility(&gut, Some(40_510)).defeated,
        "bei genau der Bestzeit nicht mehr: Der Kampf laeuft, solange die Zeit \
         KLEINER als das Limit ist — bei 40.510 endet er, und der Sieg wuerde \
         genau dort festgestellt. Dieselbe Regel wie beim Ueberdruck, der bei \
         >= greift und nicht bei >."
    );
}

#[test]
fn der_erfolgs_prueflauf_gibt_den_boss_frei() {
    let checks = achievement_run(&apl(GOOD, "gut"), None);

    assert_eq!(checks.len(), 4, "Sieg plus drei Zeitmarken");
    assert!(
        is_shippable(&checks),
        "mit der guten Rotation ist alles loesbar"
    );

    for check in &checks {
        assert!(
            !check.evidence.is_empty(),
            "{} hat keine Begruendung",
            check.name
        );
    }

    // Die Zeitmarken leiten sich aus der Bestzeit ab (GAME-DESIGN 21.6).
    let tiers = time_tiers(40_510);
    assert!(checks
        .iter()
        .any(|c| c.name == "Zeitmarke Gold" && c.evidence.contains(&tiers.gold_ms.to_string())));
}

#[test]
fn eine_zeitmarke_ueber_dem_zeitlimit_ist_erfuellt_aber_bedeutungslos() {
    // Der Sonderfall, der beim Schreiben auffiel: Bronze liegt bei 160 % der
    // Bestzeit. Bei 40.510 ms sind das 64.816 ms — mehr als der Ueberdruck-Timer
    // von 60.000 ms. Eine Marke, die der Kampf gar nicht erreichen kann, darf
    // nicht als geloest durchgehen, nur weil die Bestzeit darunter liegt.
    let checks = achievement_run(&apl(GOOD, "gut"), None);
    let bronze = checks
        .iter()
        .find(|c| c.name == "Zeitmarke Bronze")
        .expect("Bronze-Eintrag");

    let tiers = time_tiers(40_510);
    assert!(
        tiers.bronze_ms > 60_000,
        "Voraussetzung des Tests: Bronze liegt ueber dem Zeitlimit ({} ms)",
        tiers.bronze_ms
    );
    assert!(
        bronze.solved,
        "die Bestzeit liegt unter der Marke, also ist sie erfuellt"
    );
    assert!(
        !bronze.meaningful,
        "eine Marke ueber dem Zeitlimit erfuellt jeder Sieg — sie unterscheidet nichts"
    );
    assert!(
        bronze.evidence.contains("von jedem Sieg erfuellt"),
        "die Begruendung muss es benennen: {}",
        bronze.evidence
    );

    // Gold und Silber liegen im Limit und unterscheiden deshalb sehr wohl.
    for name in ["Zeitmarke Gold", "Zeitmarke Silber"] {
        let check = checks.iter().find(|c| c.name == name).expect(name);
        assert!(check.meaningful, "{name} liegt im Zeitlimit");
    }

    // Bedeutungslose Marken sperren die Auslieferung NICHT — 12.9 verlangt nur,
    // dass eine Loesung existiert. Sie gehoeren aber in den Bericht.
    assert!(is_shippable(&checks));
    let ohne_wirkung = meaningless(&checks);
    assert_eq!(ohne_wirkung.len(), 1);
    assert_eq!(ohne_wirkung[0].name, "Zeitmarke Bronze");
}
