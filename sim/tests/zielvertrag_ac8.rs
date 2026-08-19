//! AC8 — die Abnahme zu Issue #939, an echten Kaempfen.
//!
//! > Ein Kandidat, der eine harte Bedingung verletzt, kann NICHT ueber eine
//! > gute Zahl in einer anderen Dimension gewinnen. Nachgewiesen an einem Boss
//! > mit umgekehrter Siegbedingung — dort ist der Fehler am leichtesten zu
//! > erzeugen.
//!
//! # Warum gerade dort
//!
//! Bei `boss-2-3` fuellt sich der Ring von selbst, und ein aufgebrauchter Ring
//! ist die Niederlage. Der Spieler treibt ihn mit, wenn er Schaden macht —
//! eine reine Schadensrotation verliert deshalb **frueher** als eine sparsame.
//! Jede Zielfunktion der Form „maximiere Schaden" findet dort genau die
//! falsche Antwort.
//!
//! # Die Kontrollprobe ist nicht optional
//!
//! Ohne sie pruefte dieser Fall nur, dass der schadensstarke Kandidat schlecht
//! ist. Er muss am gewoehnlichen Boss **gewinnen** — sonst ist er kein
//! Gegenbeispiel, sondern bloss untauglich.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig, BalanceResult};
use rota_sim::data::{boss_by_id, Boss};
use rota_sim::objective::{
    BalanceObjective, BoundsVerdict, CandidateMeasurement, Decision, Determinism,
    FeasibilityThresholds, Outcome, Probability, RankingStage, RunContract, StageValue, TargetBand,
    WinCondition,
};

/// Der schadensstarke Kandidat.
const VIEL_SCHADEN: &str = include_str!("../apl/e607-flashpoint.apl");
/// Der sparsame Kandidat: kein einziger Zauber mit Schaden.
const SPARSAM: &str = include_str!("../apl/e939-schutz.apl");

/// Wie viele Durchgaenge je Lauf.
///
/// Zwanzig genuegen, weil der Kampf ohne Zufallsquelle rechnet: Bei einer
/// Reinheit von null ist jeder Durchgang derselbe. Der Vertrag sagt das mit
/// [`Determinism::Exact`], und [`CandidateMeasurement::from_balance_result`]
/// prueft die Behauptung nach.
const DURCHGAENGE: u32 = 20;

fn liste(name: &'static str, quelle: &str) -> Apl {
    parse(AplName(name), quelle).expect("die eingebaute Liste ist gueltig")
}

fn boss(id: &str) -> &'static Boss {
    boss_by_id(id).unwrap_or_else(|| panic!("Boss {id} fehlt im Datensatz"))
}

fn lauf(apl: &Apl, boss: &'static Boss) -> BalanceResult {
    let config = BalanceConfig {
        iterations: DURCHGAENGE,
        base_seed: 4242,
        threads: 1,
        boss,
        ..BalanceConfig::default()
    };
    run(&config, apl)
}

/// Der Laufvertrag dieses Prueffalls.
///
/// Das Zielband ist so gelegt, dass der **schadensstarke** Kandidat es trifft
/// und der sparsame es weit verfehlt. Das ist Absicht: Nur so gibt es
/// ueberhaupt eine Dimension, in der der unzulaessige Kandidat besser
/// dasteht — und genau die darf ihn nicht gewinnen lassen.
fn vertrag() -> RunContract {
    RunContract {
        version: 939,
        thresholds: FeasibilityThresholds {
            min_win_probability: Some(0.99),
            min_mechanic_probability: None,
        },
        determinism: Determinism::Exact,
        target_band: Some(TargetBand {
            low: 5000.0,
            high: 6000.0,
        }),
        vessel_max: None,
    }
}

fn zaehle_ausgaenge(result: &BalanceResult, condition: WinCondition) -> (usize, usize, usize) {
    let mut siege = 0;
    let mut ueberdruck = 0;
    let mut tode = 0;
    for sample in &result.samples {
        match Outcome::of(sample, condition) {
            Outcome::Won => siege += 1,
            Outcome::Overpressure => ueberdruck += 1,
            Outcome::Died => tode += 1,
        }
    }
    (siege, ueberdruck, tode)
}

/// AC8 — der unzulaessige Kandidat gewinnt nicht, obwohl er in einer Dimension
/// besser ist.
#[test]
fn ac8_verletzte_siegbedingung_schlaegt_jede_gute_zahl() {
    let umgekehrt = boss("boss-2-3");
    let bedingung = WinCondition::of(umgekehrt);
    assert!(
        bedingung.is_inverted(),
        "boss-2-3 muss das umgekehrte Ziel tragen — sonst prueft dieser Fall etwas anderes"
    );

    let schaden_lauf = lauf(&liste("e607-flashpoint", VIEL_SCHADEN), umgekehrt);
    let sparsam_lauf = lauf(&liste("e939-schutz", SPARSAM), umgekehrt);

    // ---- Die Ausgaenge, gemessen statt angenommen ----
    let (siege_schaden, ueberdruck_schaden, _) = zaehle_ausgaenge(&schaden_lauf, bedingung);
    let (siege_sparsam, _, tode_sparsam) = zaehle_ausgaenge(&sparsam_lauf, bedingung);

    assert_eq!(
        siege_schaden, 0,
        "der schadensstarke Kandidat muss am umgekehrten Boss verlieren"
    );
    assert_eq!(
        ueberdruck_schaden, DURCHGAENGE as usize,
        "und zwar am Ueberdruck, nicht am eigenen Tod"
    );
    assert_eq!(
        siege_sparsam, DURCHGAENGE as usize,
        "der sparsame Kandidat muss dort gewinnen — sonst gibt es keinen zulaessigen Gegenpart"
    );
    assert_eq!(tode_sparsam, 0);

    // ---- Die Zusicherung, dass dieser Fall die Sache wirklich beruehrt ----
    //
    // **NACHGEZOGEN am 2026-08-19, wie die Vorfassung es vorgesehen hatte.**
    // Sie stand hier woertlich: „Steht hier eines Tages 0, dann hat jemand die
    // Siegzahl im Simulator repariert — dann ist dieser Test nicht falsch, aber
    // seine Begruendung ist es, und beides gehoert nachgezogen."
    //
    // Genau das ist geschehen. `balance::siege` fuehrt die Umkehrung jetzt
    // (`fight.rs` bleibt unberuehrt, weil `killed_at` auch `stop_on_kill`
    // ausloest und in die Trace-Hashes eingeht). Die rohe Zahl meldet damit
    // dasselbe wie der Vertrag.
    //
    // Der Fall verliert dadurch NICHT seinen Zweck: Er belegt weiterhin, dass
    // der schadensstarke Kandidat hier keinen einzigen Sieg holt — nur eben
    // ohne den Umweg ueber eine Zahl, die das Gegenteil sagte.
    assert_eq!(
        schaden_lauf.kills, 0,
        "der schadensstarke Kandidat gewinnt am umgekehrten Boss nicht ein \
         einziges Mal — seit dem Fix sagt das auch die rohe Siegzahl"
    );
    assert!(
        schaden_lauf.kill_rate().abs() < 1e-12,
        "und die rohe Quote meldet null Prozent, nicht mehr hundert"
    );

    // ---- Und der schadensstarke Kandidat ist wirklich der staerkere ----
    assert!(
        schaden_lauf.damage.mean > sparsam_lauf.damage.mean,
        "AC8 verlangt einen Kandidaten, der VIEL SCHADEN macht: {} gegen {}",
        schaden_lauf.damage.mean,
        sparsam_lauf.damage.mean
    );

    // ---- Das Urteil ----
    let vertrag = vertrag();
    let ziel = BalanceObjective {
        hard_constraints: vertrag.thresholds,
        ..BalanceObjective::decided()
    };

    let schaden_messung =
        CandidateMeasurement::from_balance_result(&schaden_lauf, umgekehrt, &vertrag, Some(0.0))
            .with_bounds(BoundsVerdict::Within);
    let sparsam_messung =
        CandidateMeasurement::from_balance_result(&sparsam_lauf, umgekehrt, &vertrag, Some(0.0))
            .with_bounds(BoundsVerdict::Within);

    let schaden_urteil = ziel.judge(&schaden_messung);
    let sparsam_urteil = ziel.judge(&sparsam_messung);
    assert!(
        !schaden_urteil.feasible,
        "wer die Siegbedingung verletzt, ist unzulaessig: {:?}",
        schaden_urteil.violations
    );
    assert!(
        sparsam_urteil.feasible,
        "der sparsame Kandidat muss zulaessig sein: {:?}",
        sparsam_urteil.violations
    );

    // ---- Die Dimension, in der der Unzulaessige BESSER ist ----
    let schaden_band = schaden_urteil
        .score
        .get(RankingStage::TargetBandDistance)
        .value()
        .expect("das Zielband ist gemessen");
    let sparsam_band = sparsam_urteil
        .score
        .get(RankingStage::TargetBandDistance)
        .value()
        .expect("das Zielband ist gemessen");
    assert!(
        schaden_band > sparsam_band,
        "ohne eine Dimension, in der er besser ist, pruefte AC8 nichts: {schaden_band} gegen {sparsam_band}"
    );

    // ---- AC8 ----
    let vergleich = ziel.compare(&schaden_urteil, &sparsam_urteil);
    assert_eq!(
        vergleich.ordering,
        std::cmp::Ordering::Less,
        "der unzulaessige Kandidat darf nicht gewinnen"
    );
    assert_eq!(
        vergleich.decided_by,
        Decision::Feasibility,
        "und die Zulaessigkeit muss es entschieden haben, keine Stufe"
    );
}

/// Die Gegenprobe zu AC8: Ohne die Zulaessigkeitssperre GEWINNT der
/// schadensstarke Kandidat.
///
/// Ohne diesen Fall bliebe offen, ob die Sperre ueberhaupt gewirkt hat oder ob
/// der Kandidat einfach schlechter war. Er wird hier kuenstlich fuer zulaessig
/// erklaert — sonst unveraendert — und gewinnt dann ueber das Zielband.
#[test]
fn ac8_gegenprobe_ohne_die_sperre_gewinnt_er() {
    let umgekehrt = boss("boss-2-3");
    let vertrag = vertrag();
    let ziel = BalanceObjective {
        hard_constraints: vertrag.thresholds,
        ..BalanceObjective::decided()
    };

    let schaden_lauf = lauf(&liste("e607-flashpoint", VIEL_SCHADEN), umgekehrt);
    let sparsam_lauf = lauf(&liste("e939-schutz", SPARSAM), umgekehrt);

    let mut schaden_urteil = ziel.judge(
        &CandidateMeasurement::from_balance_result(&schaden_lauf, umgekehrt, &vertrag, Some(0.0))
            .with_bounds(BoundsVerdict::Within),
    );
    let mut sparsam_urteil = ziel.judge(
        &CandidateMeasurement::from_balance_result(&sparsam_lauf, umgekehrt, &vertrag, Some(0.0))
            .with_bounds(BoundsVerdict::Within),
    );

    // Die Sperre aufheben — und die beiden Stufen ueber dem Zielband
    // gleichstellen, damit wirklich das Zielband entscheidet und nicht die
    // Erfolgswahrscheinlichkeit, die den Unterschied ja gerade ausmacht.
    schaden_urteil.feasible = true;
    schaden_urteil.violations.clear();
    for stufe in [RankingStage::SuccessMargin, RankingStage::SurvivalReserve] {
        schaden_urteil.score.set(stufe, StageValue::Measured(0.0));
        sparsam_urteil.score.set(stufe, StageValue::Measured(0.0));
    }

    let vergleich = ziel.compare(&schaden_urteil, &sparsam_urteil);
    assert_eq!(
        vergleich.ordering,
        std::cmp::Ordering::Greater,
        "ohne die Sperre muss der schadensstarke Kandidat gewinnen — sonst sperrt sie nichts"
    );
    assert_eq!(
        vergleich.decided_by,
        Decision::Stage(RankingStage::TargetBandDistance)
    );
}

/// Die Kontrollprobe: Derselbe Kandidat gewinnt am gewoehnlichen Boss.
///
/// Sonst zeigte AC8 nur, dass er schlecht ist.
#[test]
fn ac8_kontrollprobe_am_gewoehnlichen_boss() {
    let gewoehnlich = boss("boss-1-1");
    assert!(
        !WinCondition::of(gewoehnlich).is_inverted(),
        "boss-1-1 muss die gewoehnliche Siegbedingung tragen"
    );

    let ergebnis = lauf(&liste("e607-flashpoint", VIEL_SCHADEN), gewoehnlich);
    let (siege, _, _) = zaehle_ausgaenge(&ergebnis, WinCondition::of(gewoehnlich));
    assert_eq!(
        siege, DURCHGAENGE as usize,
        "derselbe Kandidat muss am gewoehnlichen Boss gewinnen"
    );

    let vertrag = vertrag();
    let ziel = BalanceObjective {
        hard_constraints: vertrag.thresholds,
        ..BalanceObjective::decided()
    };
    let urteil = ziel.judge(
        &CandidateMeasurement::from_balance_result(&ergebnis, gewoehnlich, &vertrag, Some(0.0))
            .with_bounds(BoundsVerdict::Within),
    );
    assert!(
        urteil.feasible,
        "und dort zulaessig sein: {:?}",
        urteil.violations
    );

    // Dort traegt Stufe 5 einen Wert — am umgekehrten Boss bleibt sie
    // ungemessen, weil die Siegzeit dort fuer jeden Sieger die Frist ist.
    assert!(
        matches!(
            urteil.score.get(RankingStage::TimeToWin),
            StageValue::Measured(_)
        ),
        "bei gewoehnlicher Siegbedingung ist die Zeit bis zum Sieg gemessen"
    );
}

/// Die Speisungs-Tabelle im Modulkopf, **gemessen statt behauptet**.
///
/// Der Kopf von `objective.rs` sagt, welche Stufen aus einem Balance-Ergebnis
/// speisbar sind. Eine solche Tabelle altert lautlos: Wer spaeter eine Stufe
/// anschliesst oder eine Speisung verliert, aendert den Text nicht mit.
///
/// Hier stehen die Zahlen als Zusicherung. Aendert sich eine, ist entweder die
/// Speisung gewachsen — dann gehoert die Tabelle nachgezogen — oder etwas ist
/// abgerissen.
#[test]
fn die_speisung_traegt_genau_die_angekuendigten_stufen() {
    let vertrag = vertrag();

    // Gewoehnlicher Boss: Stufen 1, 4, 5 und 7.
    let gewoehnlich = boss("boss-1-1");
    let messung = CandidateMeasurement::from_balance_result(
        &lauf(&liste("e607-flashpoint", VIEL_SCHADEN), gewoehnlich),
        gewoehnlich,
        &vertrag,
        Some(0.0),
    );
    assert_eq!(
        messung.stages.measured_count(),
        4,
        "erwartet: Erfolgsabstand, Zielband, Siegzeit, Aenderungskosten"
    );
    for stufe in [
        RankingStage::MechanicQuality,
        RankingStage::Robustness,
        RankingStage::ResourceEfficiency,
    ] {
        assert_eq!(
            messung.stages.get(stufe),
            StageValue::Unmeasured,
            "{} ist heute nicht speisbar",
            stufe.name()
        );
    }
    // Stufe 2 bleibt ungemessen, weil dieser Vertrag keine Lebensgrenze traegt
    // — nicht, weil sie unspeisbar waere. Der Unterschied steht in der Tabelle.
    assert_eq!(
        messung.stages.get(RankingStage::SurvivalReserve),
        StageValue::Unmeasured
    );
    let mit_grenze = RunContract {
        vessel_max: Some(10_000),
        ..vertrag
    };
    let reicher = CandidateMeasurement::from_balance_result(
        &lauf(&liste("e607-flashpoint", VIEL_SCHADEN), gewoehnlich),
        gewoehnlich,
        &mit_grenze,
        Some(0.0),
    );
    assert_eq!(
        reicher.stages.measured_count(),
        5,
        "mit Lebensgrenze kommt die Ueberlebensreserve dazu"
    );

    // Umgekehrter Boss: eine Stufe weniger, weil die Siegzeit dort die Frist
    // ist und fuer jeden Sieger gleich.
    let umgekehrt = boss("boss-2-3");
    let umgekehrte_messung = CandidateMeasurement::from_balance_result(
        &lauf(&liste("e939-schutz", SPARSAM), umgekehrt),
        umgekehrt,
        &vertrag,
        Some(0.0),
    );
    assert_eq!(umgekehrte_messung.stages.measured_count(), 3);
    assert_eq!(
        umgekehrte_messung.stages.get(RankingStage::TimeToWin),
        StageValue::Unmeasured,
        "am umgekehrten Boss entscheidet die Siegzeit nichts"
    );
}

/// Vertrag und rohe Siegzahl meinen am umgekehrten Boss dasselbe.
///
/// # Wofuer dieser Fall vorher stand
///
/// Er hiess `die_rohe_siegzahl_ist_am_umgekehrten_boss_invertiert` und hielt
/// einen Bestandsfehler fest: `BalanceResult::kills` zaehlte, wie oft die
/// Lebenspunkte fielen — am umgekehrten Boss ist das die Niederlage. Der Fall
/// war die Begruendung dafuer, dass
/// [`CandidateMeasurement::from_balance_result`] die Ausgaenge neu bestimmt,
/// statt `kills` zu uebernehmen. Seine Doku sagte: „Faellt der Fall eines Tages
/// um, ist der Simulator repariert worden."
///
/// # Was jetzt geprueft wird
///
/// Der Simulator ist repariert (`balance::siege`, 2026-08-19). Der Fall prueft
/// deshalb die **Uebereinstimmung** statt der Abweichung — und das ist die
/// schaerfere Zusicherung: Sie faellt sowohl, wenn die Umkehrung wieder
/// verschwindet, als auch, wenn der Vertrag anfinge, etwas anderes zu meinen
/// als der Simulator.
///
/// Die Umgehung in `from_balance_result` bleibt trotzdem. Sie bestimmt die
/// Ausgaenge aus der Siegbedingung, nicht aus einer fremden Zahl — dass die
/// fremde Zahl heute stimmt, macht die Abhaengigkeit von ihr nicht besser.
#[test]
fn vertrag_und_rohe_siegzahl_stimmen_am_umgekehrten_boss_ueberein() {
    let umgekehrt = boss("boss-2-3");
    let ergebnis = lauf(&liste("e939-schutz", SPARSAM), umgekehrt);
    let (siege, _, _) = zaehle_ausgaenge(&ergebnis, WinCondition::of(umgekehrt));

    assert_eq!(
        siege, DURCHGAENGE as usize,
        "der Vertrag sieht lauter Siege"
    );
    assert_eq!(
        ergebnis.kills, DURCHGAENGE as usize,
        "und die rohe Zahl seit dem Fix ebenso — bis zum 2026-08-19 stand hier 0"
    );

    // Und die Wahrscheinlichkeit aus dem Vertrag ist exakt, nicht gestichprobt:
    // Der Kampf hat keine Zufallsquelle, also entscheidet der Wert unmittelbar.
    let messung =
        CandidateMeasurement::from_balance_result(&ergebnis, umgekehrt, &vertrag(), Some(0.0));
    assert_eq!(
        messung.feasibility.encounter_win_condition,
        Probability::Exact(1.0),
        "ohne Zufallsquelle gibt es keine Konfidenzgrenze"
    );
}
