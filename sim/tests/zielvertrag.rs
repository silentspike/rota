//! Der Zielvertrag (Issue #939) — die Struktur, ohne Kampf.
//!
//! Die Abnahme selbst (AC8) braucht echte Laeufe und steht in
//! `zielvertrag_ac8.rs`. Hier steht, was sich ohne einen einzigen gerechneten
//! Kampf pruefen laesst — und das ist das meiste, weil der Vertrag Messungen
//! ordnet, statt sie zu erzeugen.

use std::cmp::Ordering;

use rota_sim::objective::{
    BalanceObjective, BoundsVerdict, CampaignCoverage, CandidateFeasibility, CandidateMeasurement,
    CandidateVerdict, ChangeCostRule, Coverage, CoverageObligations, Decision, Determinism,
    FeasibilityThresholds, LexicographicScore, PortfolioConstraints, Probability, RankingStage,
    RobustnessRequirements, RunContract, StageValue, TargetBand, Violation, RANKING_ORDER,
    STAGE_COUNT,
};

/// Die Rangfolge, wie Issue #939 sie vorgibt — **unabhaengig hingeschrieben**.
///
/// # Warum nicht `RANKING_ORDER`
///
/// Ein Prueffall, der seine Erwartung aus der Konstante bezieht, die er
/// pruefen soll, prueft nur, dass der Code mit sich selbst uebereinstimmt.
/// Gemessen: Beim Vertauschen der Stufen 1 und 2 in `RANKING_ORDER` blieb ein
/// solcher Fall gruen — er sortierte seine Paare gleich mit.
///
/// Das ist hier zweimal hintereinander passiert, auf zwei Ebenen: erst im
/// Fall, der DASS statt WO prueft, dann im Paar-Fall, der seine Paare aus der
/// mutierten Liste bildete. Die Vorgabe gehoert deshalb hierher, in den Test,
/// wo sie nur von Hand geaendert werden kann.
const VORGABE: [RankingStage; STAGE_COUNT] = [
    RankingStage::SuccessMargin,
    RankingStage::SurvivalReserve,
    RankingStage::MechanicQuality,
    RankingStage::TargetBandDistance,
    RankingStage::TimeToWin,
    RankingStage::Robustness,
    RankingStage::DesignerChangeCost,
    RankingStage::ResourceEfficiency,
];

/// Eine Messung, bei der **jede** Stufe einen Wert traegt.
///
/// Der Ausgangspunkt fuer jeden Stufen-Prueffall: Nur wenn alle Stufen
/// gemessen sind, sagt ein Vergleich etwas ueber die Reihenfolge aus. Eine
/// ungemessene Stufe schweigt, und ein Test auf schweigenden Stufen prueft
/// nichts.
fn alle_stufen(wert: f64) -> LexicographicScore {
    let mut score = LexicographicScore::unmeasured();
    for stage in RANKING_ORDER {
        score.set(stage, StageValue::Measured(wert));
    }
    score
}

/// Ein zulaessiger Kandidat mit diesen Stufenwerten.
fn zulaessig(score: LexicographicScore) -> CandidateVerdict {
    CandidateVerdict {
        feasible: true,
        violations: Vec::new(),
        score,
    }
}

// ------------------------------------------------------------------- AC1

/// AC1 — der Vertrag hat fuenf Bestandteile.
///
/// **Die Destrukturierung ohne `..` ist die eigentliche Pruefung**: Kommt ein
/// Feld dazu oder faellt eines weg, bricht der Bau. Ein `assert` auf die Zahl
/// fuenf koennte man nachziehen, ohne hinzusehen; das hier nicht. Derselbe
/// Griff sichert in `speicher.rs` den rohen Ergebnisvektor.
#[test]
fn ac1_der_vertrag_hat_fuenf_bestandteile() {
    let BalanceObjective {
        hard_constraints,
        lexicographic_dimensions,
        coverage_obligations,
        robustness_requirements,
        designer_change_cost,
    } = BalanceObjective::decided();

    assert_eq!(
        hard_constraints.min_win_probability,
        Some(0.99),
        "die entschiedene Siegschwelle steht im Vertrag"
    );
    assert_eq!(
        lexicographic_dimensions.len(),
        STAGE_COUNT,
        "acht Stufen, keine mehr und keine weniger"
    );

    let CoverageObligations {
        solution_families_per_mechanic,
        every_spell_needs_a_niche,
        every_key_talent_represented,
        every_gear_archetype_represented,
    } = coverage_obligations;
    assert!(
        solution_families_per_mechanic >= 2,
        "eine einzige Loesungsfamilie je Mechanik waere keine Wahl, sondern ein Zwang"
    );
    assert!(every_spell_needs_a_niche);
    assert!(every_key_talent_represented);
    assert!(every_gear_archetype_represented);

    let RobustnessRequirements {
        min_encounters,
        min_probability_per_encounter,
    } = robustness_requirements;
    assert!(
        min_encounters >= 2,
        "Robustheit aus einer einzigen Begegnung ist keine"
    );
    assert!(min_probability_per_encounter > 0.0);

    let ChangeCostRule {
        cost_per_changed_value,
    } = designer_change_cost;
    assert!(
        cost_per_changed_value > 0.0,
        "kostet eine Aenderung nichts, entscheidet Stufe 7 nie etwas"
    );
}

// ------------------------------------------------------------------- AC2

/// AC2 — die Bedingungen liegen auf drei Ebenen.
///
/// Ein einzelner Kandidat kann gar nicht erfuellen, dass *jeder Zauber* eine
/// Nische hat. Der Test zeigt, dass die Ebenen getrennt urteilen: Ein
/// Kandidat, der auf seiner Ebene zulaessig ist, sagt nichts ueber die Menge —
/// und eine erfuellte Menge nichts ueber den Lauf.
#[test]
fn ac2_die_drei_ebenen_urteilen_getrennt() {
    let vertrag = BalanceObjective::decided();
    let kandidat = CandidateMeasurement {
        feasibility: CandidateFeasibility {
            encounter_win_condition: Probability::Exact(1.0),
            mandatory_mechanics_solved: Probability::Unmeasured,
            parameter_bounds: BoundsVerdict::Within,
        },
        stages: alle_stufen(1.0),
    };
    assert!(
        vertrag.judge(&kandidat).feasible,
        "Ebene 1 haengt nur an den Groessen des Kandidaten"
    );

    // Ebene 2: die Loesungsmenge. Zwei von drei Nachweisen erbracht — das
    // reicht nicht, und es beruehrt die Zulaessigkeit des Kandidaten nicht.
    let menge = PortfolioConstraints {
        every_mechanic_has_multiple_solution_families: Coverage::Measured {
            covered: 5,
            required: 5,
        },
        no_school_is_globally_mandatory: Coverage::Measured {
            covered: 4,
            required: 4,
        },
        robustness_across_encounters: Coverage::Measured {
            covered: 1,
            required: 3,
        },
    };
    assert!(
        !menge.all_met(),
        "eine Mechanik mit nur einer Loesung laesst die Menge durchfallen"
    );

    // Ebene 3: der Lauf als Ganzes.
    let lauf = CampaignCoverage {
        every_spell_has_viable_niche: Coverage::Measured {
            covered: 60,
            required: 60,
        },
        every_key_talent_is_represented: Coverage::Unmeasured,
        every_intended_gear_archetype_is_represented: Coverage::Measured {
            covered: 4,
            required: 4,
        },
    };
    assert!(
        !lauf.all_met(),
        "ein nicht nachgewiesener Nachweis ist kein erbrachter"
    );
}

/// Nicht gemessen ist nicht erfuellt — auf allen Ebenen dieselbe Regel.
#[test]
fn nicht_gemessen_ist_nicht_erfuellt() {
    assert!(!Coverage::Unmeasured.is_met());
    assert!(!Probability::Unmeasured.meets(0.0));
    assert!(
        !Probability::Unmeasured.meets(0.0),
        "auch eine Schwelle von null wird von einer fehlenden Messung nicht erfuellt"
    );

    let vertrag = BalanceObjective {
        hard_constraints: FeasibilityThresholds {
            min_win_probability: Some(0.99),
            min_mechanic_probability: Some(0.999),
        },
        ..BalanceObjective::decided()
    };
    let ungemessen = CandidateMeasurement {
        feasibility: CandidateFeasibility {
            encounter_win_condition: Probability::Exact(1.0),
            mandatory_mechanics_solved: Probability::Unmeasured,
            parameter_bounds: BoundsVerdict::Within,
        },
        stages: alle_stufen(1.0),
    };
    let urteil = vertrag.judge(&ungemessen);
    assert!(
        !urteil.feasible,
        "eine geforderte Bedingung ohne Messwert macht unzulaessig, nicht zulaessig"
    );
    assert!(urteil
        .violations
        .contains(&Violation::MechanicProbabilityTooLow {
            decisive: None,
            threshold: 0.999,
        }));
}

/// Die harte Bedingung „Mechanikerfolg" wirkt, **sobald** sie gespeist wird.
///
/// # Warum dieser Fall gebraucht wird
///
/// Die Schwelle steht in [`FeasibilityThresholds::DECIDED`] auf `None`, weil
/// heute kein Mechanikwert den Kampf verlaesst. Damit ist der ganze Zweig im
/// Alltag unbenutzt — und ein unbenutzter Zweig ist eine Leitung, von der
/// niemand weiss, ob sie angeschlossen ist. Dieser Fall speist sie von Hand
/// und prueft beide Richtungen.
#[test]
fn mechanikerfolg_wirkt_sobald_er_gemessen_ist() {
    let vertrag = BalanceObjective {
        hard_constraints: FeasibilityThresholds {
            min_win_probability: None,
            min_mechanic_probability: Some(0.999),
        },
        ..BalanceObjective::decided()
    };
    let grundmessung = CandidateMeasurement {
        feasibility: CandidateFeasibility {
            encounter_win_condition: Probability::Exact(1.0),
            mandatory_mechanics_solved: Probability::Unmeasured,
            parameter_bounds: BoundsVerdict::Within,
        },
        stages: alle_stufen(1.0),
    };

    assert!(
        !vertrag.judge(&grundmessung).feasible,
        "ohne Messwert bleibt die geforderte Bedingung unerfuellt"
    );
    assert!(
        vertrag
            .judge(&grundmessung.with_mechanics(Probability::Exact(0.9995)))
            .feasible,
        "ueber der Schwelle wird der Kandidat zulaessig"
    );
    assert!(
        !vertrag
            .judge(&grundmessung.with_mechanics(Probability::Exact(0.998)))
            .feasible,
        "knapp darunter nicht — sonst pruefte die Schwelle nichts"
    );
}

// ------------------------------------------------------------------- AC3

/// AC3 — „Sieg" taucht genau einmal auf, als Zulaessigkeitsbedingung.
///
/// Der Test prueft beide Haelften: dass keine Stufe der Rangfolge den Sieg
/// fuehrt, und dass die Zulaessigkeit **vor** jeder Stufe entscheidet.
#[test]
fn ac3_sieg_steht_nicht_in_der_rangfolge() {
    for stage in RANKING_ORDER {
        let name = stage.name().to_lowercase();
        assert!(
            !name.contains("sieg") || stage == RankingStage::TimeToWin,
            "die Stufe {name} fuehrt den Sieg ein zweites Mal"
        );
    }
    // Stufe 5 nennt die Zeit BIS zum Sieg — das ist eine Dauer, keine zweite
    // Siegbedingung. Der Unterschied ist pruefbar: Sie entscheidet nur unter
    // Kandidaten, die bereits gewonnen haben.
    assert_eq!(RankingStage::TimeToWin.rank(), 5);
}

/// Die Zulaessigkeit entscheidet vor jeder Stufe — und ist selbst keine.
#[test]
fn ac3_die_zulaessigkeit_ist_keine_stufe() {
    let vertrag = BalanceObjective::decided();
    let schlecht_aber_zulaessig = zulaessig(alle_stufen(-1000.0));
    let gut_aber_unzulaessig = CandidateVerdict {
        feasible: false,
        violations: vec![Violation::WinProbabilityTooLow {
            decisive: Some(0.5),
            threshold: 0.99,
        }],
        score: alle_stufen(1000.0),
    };

    let vergleich = vertrag.compare(&schlecht_aber_zulaessig, &gut_aber_unzulaessig);
    assert_eq!(vergleich.ordering, Ordering::Greater);
    assert_eq!(
        vergleich.decided_by,
        Decision::Feasibility,
        "keine Stufe darf hier zum Zug gekommen sein"
    );
}

// ------------------------------------------------------------- AC4, AC4b

/// AC4 — eine harte Bedingung ist eine Wahrscheinlichkeit, kein Wahrheitswert.
#[test]
fn ac4_harte_bedingung_ist_eine_wahrscheinlichkeit() {
    let stichprobe = Probability::from_trials(97, 100);
    assert!(
        matches!(stichprobe, Probability::Sampled { .. }),
        "aus Erfolgen und Versuchen entsteht eine Stichprobe, kein ja/nein"
    );
    assert_eq!(
        Probability::from_trials(0, 0),
        Probability::Unmeasured,
        "null Versuche sind keine Messung mit dem Ergebnis null Prozent"
    );
}

/// AC4b — die untere Grenze entscheidet, nie der Schaetzwert.
///
/// **Der Fall, um den es geht:** hundert von hundert Siegen. Der Schaetzwert
/// ist 1,0 und bestuende jede Schwelle. Belegt ist eine Sicherheit von
/// 99 Prozent damit nicht.
#[test]
fn ac4b_die_untere_grenze_entscheidet() {
    let alle_gewonnen = Probability::from_trials(100, 100);
    assert_eq!(alle_gewonnen.estimate(), Some(1.0));

    let untere = alle_gewonnen.decisive().expect("gemessen");
    // **Kontrollzeile mit festem Wert.** Die Wilson-Grenze fuer 100 von 100
    // liegt bei 0,9630. Steht hier eine andere Zahl, rechnet die Formel etwas
    // anderes als sie soll — und das faellt sonst nicht auf, weil jede Zahl
    // unter 1,0 den Test darunter bestehen laesst.
    assert!(
        (untere - 0.963_010).abs() < 1e-4,
        "untere Wilson-Grenze fuer 100/100 erwartet 0,9630, gemessen {untere}"
    );
    assert!(
        !alle_gewonnen.meets(0.99),
        "ein Schaetzwert von 1,0 aus hundert Durchgaengen belegt keine 99 Prozent"
    );
}

/// Die Gegenprobe: Mit genug Durchgaengen ist die Schwelle erreichbar.
///
/// Ohne sie pruefte der Test darueber nur, dass die Bedingung **nie** erfuellt
/// wird — und eine Schwelle, die niemand erreichen kann, ist keine Schwelle,
/// sondern eine Sperre.
#[test]
fn ac4b_mit_genug_durchgaengen_ist_die_schwelle_erreichbar() {
    assert!(
        !Probability::from_trials(380, 380).meets(0.99),
        "380 Durchgaenge reichen noch nicht"
    );
    assert!(
        Probability::from_trials(400, 400).meets(0.99),
        "400 Durchgaenge reichen — die Schwelle ist erreichbar, nicht gesperrt"
    );
}

/// AC4b — exakt gerechnet gibt es keine Konfidenzgrenze.
#[test]
fn ac4b_exakt_gerechnet_entscheidet_der_wert_unmittelbar() {
    let exakt = Probability::Exact(0.995);
    assert_eq!(exakt.decisive(), Some(0.995));
    assert!(exakt.meets(0.99));
    assert_eq!(
        exakt.estimate(),
        exakt.decisive(),
        "ohne Stichprobe fallen Schaetzwert und entscheidender Wert zusammen"
    );
}

// ------------------------------------------------------------------- AC5

/// AC5 — die Schwellen sind versionierte Felder des Laufvertrags.
///
/// Derselbe Kandidat, zwei Vertraege, zwei Urteile. Waeren die Schwellen
/// globale Konstanten, koennte dieser Test gar nicht geschrieben werden.
#[test]
fn ac5_schwellen_sind_felder_des_laufvertrags() {
    let kandidat = CandidateMeasurement {
        feasibility: CandidateFeasibility {
            encounter_win_condition: Probability::Exact(0.995),
            mandatory_mechanics_solved: Probability::Exact(1.0),
            parameter_bounds: BoundsVerdict::Within,
        },
        stages: alle_stufen(1.0),
    };

    let erste_instanz = BalanceObjective {
        hard_constraints: FeasibilityThresholds {
            min_win_probability: Some(0.99),
            min_mechanic_probability: None,
        },
        ..BalanceObjective::decided()
    };
    let endgame = BalanceObjective {
        hard_constraints: FeasibilityThresholds {
            min_win_probability: Some(0.999),
            min_mechanic_probability: None,
        },
        ..BalanceObjective::decided()
    };

    assert!(erste_instanz.judge(&kandidat).feasible);
    assert!(
        !endgame.judge(&kandidat).feasible,
        "derselbe Kandidat, strengerer Vertrag, anderes Urteil"
    );

    // Und die Version wandert mit: Zwei Laeufe mit verschiedenen Schwellen
    // duerfen nicht dieselbe Vertragsversion tragen.
    let a = RunContract {
        version: 1,
        thresholds: erste_instanz.hard_constraints,
        determinism: Determinism::Exact,
        target_band: None,
        vessel_max: None,
    };
    let b = RunContract {
        version: 2,
        thresholds: endgame.hard_constraints,
        ..a
    };
    assert_ne!(a.version, b.version);
    assert_ne!(a.thresholds, b.thresholds);
}

// ------------------------------------------------------------ AC6, AC6b

/// AC6 — die Rangfolge steht in genau der entschiedenen Reihenfolge.
#[test]
fn ac6_die_reihenfolge_ist_die_entschiedene() {
    let erwartet = [
        (1, RankingStage::SuccessMargin),
        (2, RankingStage::SurvivalReserve),
        (3, RankingStage::MechanicQuality),
        (4, RankingStage::TargetBandDistance),
        (5, RankingStage::TimeToWin),
        (6, RankingStage::Robustness),
        (7, RankingStage::DesignerChangeCost),
        (8, RankingStage::ResourceEfficiency),
    ];
    for (platz, stufe) in erwartet {
        assert_eq!(stufe.rank(), platz, "{} steht falsch", stufe.name());
        assert_eq!(RANKING_ORDER[platz - 1], stufe);
    }
    assert_eq!(
        BalanceObjective::decided().lexicographic_dimensions,
        RANKING_ORDER
    );
}

/// AC6 — jede Stufe kommt ueberhaupt zum Zug.
///
/// Fuer jede Stufe k: Alle Stufen sind gleich, nur k weicht ab. Dann muss
/// **k** entschieden haben.
///
/// **Was dieser Fall NICHT prueft, obwohl der Name es einmal behauptete:** den
/// PLATZ der Stufe. Gemessen am 2026-08-19 mit vertauschten Stufen 1 und 2 —
/// er blieb gruen, weil eine allein abweichende Stufe an jedem Platz
/// entscheidet. Den Platz nagelt
/// [`ac6_jedes_paar_haelt_die_reihenfolge`] fest.
#[test]
fn ac6_jede_stufe_kommt_zum_zug() {
    let vertrag = BalanceObjective::decided();
    for stufe in RANKING_ORDER {
        let mut besser = alle_stufen(1.0);
        besser.set(stufe, StageValue::Measured(2.0));
        let vergleich = vertrag.compare(&zulaessig(besser), &zulaessig(alle_stufen(1.0)));

        assert_eq!(
            vergleich.decided_by,
            Decision::Stage(stufe),
            "{} sollte entscheiden, tat es aber nicht",
            stufe.name()
        );
        assert_eq!(vergleich.ordering, Ordering::Greater);
        assert!(
            vergleich.silent_stages.is_empty(),
            "in diesem Aufbau ist jede Stufe gemessen"
        );
    }
}

/// AC6 — **der Platz** jeder Stufe, paarweise festgenagelt.
///
/// Fuer jedes Paar `(i, j)` mit `i` vor `j`: Ein Kandidat, der auf `i` vorn
/// und auf `j` hinten liegt, muss gewinnen — und `i` muss es entschieden
/// haben. Achtundzwanzig Paare, und zusammen lassen sie genau eine
/// Reihenfolge zu.
///
/// # Warum es diesen Fall gibt
///
/// Am 2026-08-19 habe ich die Stufen 1 und 2 in `RANKING_ORDER` vertauscht,
/// um zu sehen, welche Prueffaelle das faengt. Es war genau **einer**: der,
/// der die Reihenfolge abliest. Der Fall darueber hiess damals „jede Stufe
/// entscheidet an ihrem Platz" und blieb gruen — er prueft, DASS jede Stufe
/// entscheidet, nicht WO sie steht.
///
/// Eine abgelesene Liste und eine geprüfte Ordnung sind zweierlei: Die Liste
/// laesst sich mit der Rangfolge zusammen umsortieren, ohne dass etwas rot
/// wird. Dieser Fall nicht.
#[test]
fn ac6_jedes_paar_haelt_die_reihenfolge() {
    let vertrag = BalanceObjective::decided();
    let mut geprueft = 0;

    for (platz_vorn, vorn) in VORGABE.iter().enumerate() {
        for hinten in VORGABE.iter().skip(platz_vorn + 1) {
            let mut a = alle_stufen(0.0);
            a.set(*vorn, StageValue::Measured(1.0));
            let mut b = alle_stufen(0.0);
            b.set(*hinten, StageValue::Measured(1.0));

            let vergleich = vertrag.compare(&zulaessig(a), &zulaessig(b));
            assert_eq!(
                vergleich.ordering,
                Ordering::Greater,
                "{} steht vor {} und muss deshalb gewinnen",
                vorn.name(),
                hinten.name()
            );
            assert_eq!(
                vergleich.decided_by,
                Decision::Stage(*vorn),
                "entschieden haette die vordere Stufe {}",
                vorn.name()
            );
            geprueft += 1;
        }
    }

    // Die Zahl gehoert in den Datensatz: Bei acht Stufen sind es 8*7/2 Paare.
    // Steht hier eine kleinere Zahl, hat die Schleife weniger geprueft als der
    // Name verspricht — und das faellt sonst nicht auf, weil alles gruen ist.
    assert_eq!(geprueft, 28, "acht Stufen ergeben 28 geordnete Paare");
}

/// AC6 — eine hoehere Stufe schlaegt jede tiefere, egal wie gross der
/// Unterschied dort ist.
#[test]
fn ac6_die_hoehere_stufe_schlaegt_die_tiefere() {
    let vertrag = BalanceObjective::decided();

    let mut knapp_vorn = alle_stufen(0.0);
    knapp_vorn.set(RankingStage::SuccessMargin, StageValue::Measured(0.000_001));
    let mut weit_vorn_aber_tiefer = alle_stufen(0.0);
    weit_vorn_aber_tiefer.set(RankingStage::ResourceEfficiency, StageValue::Measured(1e9));

    let vergleich = vertrag.compare(&zulaessig(knapp_vorn), &zulaessig(weit_vorn_aber_tiefer));
    assert_eq!(vergleich.ordering, Ordering::Greater);
    assert_eq!(
        vergleich.decided_by,
        Decision::Stage(RankingStage::SuccessMargin),
        "eine Milliarde auf Stufe 8 schlaegt kein Millionstel auf Stufe 1"
    );
}

/// AC6b — Stufe 3 schweigt hoerbar, statt still zu entscheiden.
///
/// Sie ist **definiert, nicht gestrichen**: Die Messvorschrift steht an
/// [`RankingStage::MechanicQuality`]. Solange sie ungespeist ist, wird sie
/// uebersprungen — und der Vergleich sagt, dass er sie uebersprungen hat.
#[test]
fn ac6b_stufe_drei_schweigt_hoerbar() {
    let vertrag = BalanceObjective::decided();
    let mut ohne_mechanik = alle_stufen(1.0);
    ohne_mechanik.set(RankingStage::MechanicQuality, StageValue::Unmeasured);
    let mut gegner = alle_stufen(1.0);
    gegner.set(RankingStage::MechanicQuality, StageValue::Unmeasured);
    // Erst auf Stufe 4 unterscheiden sie sich.
    gegner.set(RankingStage::TargetBandDistance, StageValue::Measured(0.5));

    let vergleich = vertrag.compare(&zulaessig(ohne_mechanik), &zulaessig(gegner));
    assert_eq!(
        vergleich.decided_by,
        Decision::Stage(RankingStage::TargetBandDistance),
        "die naechste gemessene Stufe entscheidet"
    );
    assert!(
        vergleich
            .silent_stages
            .contains(RankingStage::MechanicQuality),
        "die uebersprungene Stufe muss im Ergebnis stehen — sonst ist es Schweigen"
    );
    assert_eq!(
        vergleich.silent_stages.len(),
        1,
        "genau eine Stufe war ungemessen"
    );
}

/// Eine ungemessene Stufe bekommt weder den besten noch den schlechtesten
/// Platz.
///
/// Beides waere eine Aussage ueber einen Wert, den niemand kennt.
#[test]
fn ungemessene_stufe_wird_nicht_zum_vorteil_und_nicht_zum_nachteil() {
    let vertrag = BalanceObjective::decided();
    let mut ungemessen = alle_stufen(1.0);
    ungemessen.set(RankingStage::SuccessMargin, StageValue::Unmeasured);

    let gegen_besseren = vertrag.compare(&zulaessig(ungemessen), &{
        let mut b = alle_stufen(1.0);
        b.set(RankingStage::SuccessMargin, StageValue::Measured(5.0));
        zulaessig(b)
    });
    assert_eq!(
        gegen_besseren.ordering,
        Ordering::Equal,
        "die ungemessene Stufe darf nicht verlieren"
    );
    assert_eq!(gegen_besseren.decided_by, Decision::Tie);

    let gegen_schlechteren = vertrag.compare(&zulaessig(ungemessen), &{
        let mut b = alle_stufen(1.0);
        b.set(RankingStage::SuccessMargin, StageValue::Measured(-5.0));
        zulaessig(b)
    });
    assert_eq!(
        gegen_schlechteren.ordering,
        Ordering::Equal,
        "und sie darf auch nicht gewinnen"
    );
}

// ------------------------------------------------------------------- AC7

/// AC7 — zwischen zwei gleichwertigen Loesungen gewinnt die mit weniger
/// geaenderten Zahlen.
#[test]
fn ac7_weniger_geaenderte_zahlen_gewinnen() {
    let vertrag = BalanceObjective::decided();
    let regel = vertrag.designer_change_cost;

    // Gleichwertig bis Stufe 6, dann trennt sie die Zahl der Aenderungen.
    let mut sparsam = alle_stufen(1.0);
    sparsam.set(
        RankingStage::DesignerChangeCost,
        StageValue::Measured(-(3.0 * regel.cost_per_changed_value)),
    );
    let mut grosszuegig = alle_stufen(1.0);
    grosszuegig.set(
        RankingStage::DesignerChangeCost,
        StageValue::Measured(-(40.0 * regel.cost_per_changed_value)),
    );

    let vergleich = vertrag.compare(&zulaessig(sparsam), &zulaessig(grosszuegig));
    assert_eq!(vergleich.ordering, Ordering::Greater);
    assert_eq!(
        vergleich.decided_by,
        Decision::Stage(RankingStage::DesignerChangeCost)
    );
}

// -------------------------------------------------------- Weitere Zusagen

/// Zwei unzulaessige Kandidaten werden nicht gerangt.
#[test]
fn zwei_unzulaessige_stehen_gleich() {
    let vertrag = BalanceObjective::decided();
    let unzulaessig = |wert: f64| CandidateVerdict {
        feasible: false,
        violations: vec![Violation::ParameterBoundsUnmeasured],
        score: alle_stufen(wert),
    };
    let vergleich = vertrag.compare(&unzulaessig(100.0), &unzulaessig(-100.0));
    assert_eq!(vergleich.ordering, Ordering::Equal);
    assert_eq!(vergleich.decided_by, Decision::Feasibility);
}

/// Der Abstand zum Zielband ist innerhalb des Bandes null.
///
/// Nicht der Abstand zur Mitte — sonst waere das Band heimlich ein Zielwert.
#[test]
fn zielband_hat_innen_keinen_besseren_platz() {
    let band = TargetBand {
        low: 100.0,
        high: 200.0,
    };
    assert!((band.distance(100.0) - 0.0).abs() < f64::EPSILON);
    assert!((band.distance(150.0) - 0.0).abs() < f64::EPSILON);
    assert!((band.distance(200.0) - 0.0).abs() < f64::EPSILON);
    assert!((band.distance(80.0) - 20.0).abs() < f64::EPSILON);
    assert!((band.distance(230.0) - 30.0).abs() < f64::EPSILON);
}

/// Robustheit nimmt das Minimum und laesst sich nicht wegmitteln.
#[test]
fn robustheit_nimmt_die_schwaechste_begegnung() {
    let anforderung = RobustnessRequirements {
        min_encounters: 3,
        min_probability_per_encounter: 0.99,
    };
    let messung = CandidateMeasurement {
        feasibility: CandidateFeasibility {
            encounter_win_condition: Probability::Exact(1.0),
            mandatory_mechanics_solved: Probability::Exact(1.0),
            parameter_bounds: BoundsVerdict::Within,
        },
        stages: LexicographicScore::unmeasured(),
    };

    let stark = [
        Probability::Exact(1.0),
        Probability::Exact(1.0),
        Probability::Exact(0.5),
    ];
    let gemessen = messung.with_robustness(&stark, anforderung);
    assert_eq!(
        gemessen.stages.get(RankingStage::Robustness).value(),
        Some(0.5),
        "der Mittelwert waere 0,833 — und genau die schwache Begegnung waere verschwunden"
    );

    // Zu wenige Begegnungen lassen die Stufe ungemessen.
    let zu_wenige = messung.with_robustness(&stark[..2], anforderung);
    assert_eq!(
        zu_wenige.stages.get(RankingStage::Robustness),
        StageValue::Unmeasured
    );

    // Eine ungemessene Begegnung ebenfalls — sonst waere die Luecke
    // wegmittelbar.
    let mit_luecke = [
        Probability::Exact(1.0),
        Probability::Unmeasured,
        Probability::Exact(1.0),
    ];
    assert_eq!(
        messung
            .with_robustness(&mit_luecke, anforderung)
            .stages
            .get(RankingStage::Robustness),
        StageValue::Unmeasured
    );
}
