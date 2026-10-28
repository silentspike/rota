//! Die Abnahme von Epic #903, an echten Kaempfen.
//!
//! > Ein warmer Lauf mit gewechselter Zielfunktion rechnet null Kaempfe neu,
//! > beide Gegenproben stimmen, und das Pruefprogramm fuer die Unabhaengigkeit
//! > des Ergebnisschemas ist einmal nachweislich rot geworden.
//!
//! # Warum es diese Datei neben `speicher_trennung.rs` gibt
//!
//! Dort steht der Zielwechsel ueber von Hand gefuellten Entwuerfen, und die
//! beiden Zielfunktionen sind Konstanten. Das belegt die Speicher, nicht den
//! Weg: Die Fingerabdruecke muessen aus den Vertraegen **entstehen**, die
//! Ergebnisse aus `balance::run`, und gefahren wird der Weg, den eine Suche
//! geht — [`Speicherverbund::bewerte`].
//!
//! # Wie der Hauptfall rot wird
//!
//! Baut man die Zielfunktion in den Ergebnisschluessel ein — die eine Zeile in
//! `Speicherverbund::bewerte_mit`, die den Schluessel bildet —, faellt
//! `e19_ein_zielwechsel_ueber_echten_laeufen_rechnet_null_kaempfe_neu`. Die
//! beiden Kontrollzeilen bleiben dabei gruen, weil keine von ihnen die
//! Zielfunktion wechselt. Die Mutation selbst steht im Pull Request: Sie laesst
//! sich nicht einchecken, ohne den Code zu aendern, den sie prueft.
//!
//! # Zwei Zaehler, nicht einer
//!
//! Jeder Fall zaehlt die Kaempfe doppelt: am Zaehler des Ergebnisspeichers und
//! an einem Laeufer, der `balance::run` umhuellt. Der erste sagt, was der
//! Speicher zu tun glaubt; der zweite, was tatsaechlich gerechnet wurde. Eine
//! Null am ersten allein waere auch die Null eines Zaehlers, der neben dem
//! Aufruf haengt.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig, BalanceResult};
use rota_sim::data::{boss_by_id, stat_at_level, Boss, PLAYER};
use rota_sim::factors::{DamageFactors, NEUTRAL};
use rota_sim::fight::vessel_with_gear;
use rota_sim::objective::{
    BalanceObjective, BoundsVerdict, CandidateMeasurement, CandidateVerdict, ChangeCostRule,
    Decision, Determinism, FeasibilityThresholds, RankingStage, RunContract, TargetBand,
};
use rota_sim::speicher::{
    BewertungsSchluessel, ErgebnisSchluessel, Kandidat, Optimalitaet, SemantikHash,
    Speicherverbund, Zielfunktion, ZielfunktionsHash,
};

/// Die schnellste der vier Listen gegen boss-1-1.
const BASIS: &str = include_str!("../apl/e607-basis.apl");
/// Die Liste, die am wenigsten Schaden durchlaesst.
const K_CRUST: &str = include_str!("../apl/e607k-crust.apl");
/// Dazwischen.
const B_CRUST: &str = include_str!("../apl/e607b-crust.apl");
/// Die schadensstaerkste.
const FLASHPOINT: &str = include_str!("../apl/e607-flashpoint.apl");
/// Gewinnt am umgekehrten Boss, ohne einen Zauber mit Schaden.
const SCHUTZ: &str = include_str!("../apl/e939-schutz.apl");
/// Verliert gegen boss-1-1 — der unzulaessige Kandidat.
const VERLIERT: &str = include_str!("../apl/step1-good.apl");

/// Durchgaenge je Lauf — wie in `zielvertrag_ac8.rs`.
///
/// Der Kampf hat ohne Reinheit keine Zufallsquelle; der Vertrag sagt das mit
/// [`Determinism::Exact`], und die Speisung prueft es nach.
const DURCHGAENGE: u32 = 20;

fn liste(name: &'static str, quelle: &str) -> Apl {
    parse(AplName(name), quelle).expect("die eingebaute Liste ist gueltig")
}

fn boss(id: &str) -> &'static Boss {
    boss_by_id(id).unwrap_or_else(|| panic!("Boss {id} fehlt im Datensatz"))
}

/// Die vier Listen der Abnahme.
fn listen() -> Vec<Apl> {
    vec![
        liste("e607-basis", BASIS),
        liste("e607k-crust", K_CRUST),
        liste("e607b-crust", B_CRUST),
        liste("e607-flashpoint", FLASHPOINT),
    ]
}

fn lauf_config(gegner: &'static Boss) -> BalanceConfig {
    BalanceConfig {
        iterations: DURCHGAENGE,
        base_seed: 4242,
        threads: 1,
        boss: gegner,
        ..BalanceConfig::default()
    }
}

fn kandidaten(listen: &[Apl], config: BalanceConfig) -> Vec<Kandidat<'_>> {
    listen
        .iter()
        .map(|apl| Kandidat {
            apl,
            config,
            aenderungskosten: Some(0.0),
            grenzen: BoundsVerdict::Within,
        })
        .collect()
}

/// Die Lebensgrenze des Spielers auf Stufe 1 ohne Ausruestung — **aus dem
/// Simulator**, nicht abgeschrieben: dieselbe Rechnung wie in `Fight::new`.
fn lebensgrenze() -> u32 {
    vessel_with_gear(stat_at_level(PLAYER.vessel, 1), 0)
}

/// Laufvertrag 1: ohne Lebensgrenze, ohne Zielband.
///
/// Gemessen werden damit Stufe 1 (fuer jeden Sieger gleich), Stufe 5 und
/// Stufe 7 (fuer alle null) — es entscheidet die **Siegzeit**.
fn vertrag_a() -> RunContract {
    RunContract {
        version: 1,
        thresholds: FeasibilityThresholds::DECIDED,
        determinism: Determinism::Exact,
        target_band: None,
        vessel_max: None,
    }
}

fn ziel_a() -> Zielfunktion {
    Zielfunktion {
        ziel: BalanceObjective::decided(),
        vertrag: vertrag_a(),
    }
}

/// Laufvertrag 2: dieselbe Rangfolge, aber **mit Lebensgrenze**.
///
/// Damit bekommt Stufe 2 (Ueberlebensreserve) einen Wert, und sie steht vor
/// der Siegzeit. Das ist ein echter Zielwechsel im Sinn des Epics — die
/// Vertragsversion zaehlt mit, die Rangfolge selbst bleibt.
fn ziel_b() -> Zielfunktion {
    Zielfunktion {
        ziel: BalanceObjective::decided(),
        vertrag: RunContract {
            version: 2,
            vessel_max: Some(lebensgrenze()),
            ..vertrag_a()
        },
    }
}

/// Ein Laeufer, der mitzaehlt — die zweite, unabhaengige Zaehlung.
fn gezaehlt(laeufe: &mut u64) -> impl FnMut(&BalanceConfig, &Apl) -> BalanceResult + '_ {
    move |config: &BalanceConfig, apl: &Apl| {
        *laeufe += 1;
        run(config, apl)
    }
}

fn urteil_von(
    verbund: &Speicherverbund,
    kandidat: &Kandidat<'_>,
    ziel: &Zielfunktion,
) -> CandidateVerdict {
    verbund
        .urteile
        .hole(&BewertungsSchluessel {
            ergebnis: ErgebnisSchluessel::fuer_lauf(&kandidat.config, kandidat.apl),
            zielfunktion: ziel.fingerabdruck(),
        })
        .unwrap_or_else(|| {
            panic!(
                "kein Urteil fuer {} unter {}",
                kandidat.apl.name,
                ziel.fingerabdruck()
            )
        })
}

/// Der gemeldete Bestwert — und die **Eigenschaft**, die ihn ausmacht.
///
/// Geprueft wird nicht, welcher Name herauskommt, sondern dass kein einziger
/// Kandidat nach dem Zielvertrag besser ist. Der Name allein waere eine Zahl,
/// die man festnagelt; die Eigenschaft ist das, was „bester" heisst.
fn bester_mit_beleg(
    verbund: &Speicherverbund,
    kandidaten: &[Kandidat<'_>],
    ziel: &Zielfunktion,
) -> (String, CandidateVerdict) {
    let (aufbau, bestes) = verbund
        .archiv
        .optimalitaet
        .bester(ziel.fingerabdruck())
        .expect("nach einem Durchgang steht ein Bestwert");
    for kandidat in kandidaten {
        let urteil = urteil_von(verbund, kandidat, ziel);
        let vergleich = ziel.ziel.compare(&urteil, &bestes);
        assert_ne!(
            vergleich.ordering,
            Ordering::Greater,
            "{} ist nach dem Zielvertrag besser als der gemeldete Bestwert {aufbau} \
             (entschieden durch {:?})",
            kandidat.apl.name,
            vergleich.decided_by
        );
    }
    (aufbau.to_string(), bestes)
}

// ------------------------------------------------------- Die Fingerabdruecke

/// Eine Aenderung an einem Vertragsfeld, mit Namen.
type VertragsAenderung = (&'static str, fn(&mut Zielfunktion));

#[test]
fn der_fingerabdruck_der_zielfunktion_folgt_jedem_feld() {
    let basis = ziel_a();
    assert_eq!(
        basis.fingerabdruck(),
        ziel_a().fingerabdruck(),
        "derselbe Vertrag muss denselben Fingerabdruck ergeben — sonst waere jeder \
         Lauf ein Zielwechsel"
    );
    assert_eq!(
        basis.fingerabdruck(),
        ZielfunktionsHash::aus(&basis.ziel, &basis.vertrag)
    );

    // Jedes Feld beider Vertraege einzeln, je eine Aenderung. Fehlte eines im
    // Fingerabdruck, fiele genau diese Zeile — beim Namen.
    let aenderungen: &[VertragsAenderung] = &[
        ("ziel.hard_constraints.min_win_probability", |z| {
            z.ziel.hard_constraints.min_win_probability = Some(0.95);
        }),
        ("ziel.hard_constraints.min_mechanic_probability", |z| {
            z.ziel.hard_constraints.min_mechanic_probability = Some(0.999);
        }),
        ("ziel.lexicographic_dimensions", |z| {
            z.ziel.lexicographic_dimensions.swap(1, 4);
        }),
        ("ziel.coverage.solution_families_per_mechanic", |z| {
            z.ziel.coverage_obligations.solution_families_per_mechanic = 3;
        }),
        ("ziel.coverage.every_spell_needs_a_niche", |z| {
            z.ziel.coverage_obligations.every_spell_needs_a_niche = false;
        }),
        ("ziel.coverage.every_key_talent_represented", |z| {
            z.ziel.coverage_obligations.every_key_talent_represented = false;
        }),
        ("ziel.coverage.every_gear_archetype_represented", |z| {
            z.ziel.coverage_obligations.every_gear_archetype_represented = false;
        }),
        ("ziel.robustness.min_encounters", |z| {
            z.ziel.robustness_requirements.min_encounters = 3;
        }),
        ("ziel.robustness.min_probability_per_encounter", |z| {
            z.ziel.robustness_requirements.min_probability_per_encounter = 0.95;
        }),
        ("ziel.designer_change_cost", |z| {
            z.ziel.designer_change_cost = ChangeCostRule {
                cost_per_changed_value: 2.0,
            };
        }),
        ("vertrag.version", |z| z.vertrag.version = 2),
        ("vertrag.thresholds.min_win_probability", |z| {
            z.vertrag.thresholds.min_win_probability = Some(0.95);
        }),
        ("vertrag.thresholds.min_mechanic_probability", |z| {
            z.vertrag.thresholds.min_mechanic_probability = Some(0.999);
        }),
        ("vertrag.determinism", |z| {
            z.vertrag.determinism = Determinism::Stochastic;
        }),
        ("vertrag.target_band", |z| {
            z.vertrag.target_band = Some(TargetBand {
                low: 5000.0,
                high: 6000.0,
            });
        }),
        ("vertrag.target_band.low", |z| {
            z.vertrag.target_band = Some(TargetBand {
                low: 5001.0,
                high: 6000.0,
            });
        }),
        ("vertrag.target_band.high", |z| {
            z.vertrag.target_band = Some(TargetBand {
                low: 5000.0,
                high: 6001.0,
            });
        }),
        ("vertrag.vessel_max", |z| {
            z.vertrag.vessel_max = Some(lebensgrenze());
        }),
        ("vertrag.vessel_max.wert", |z| {
            z.vertrag.vessel_max = Some(lebensgrenze() + 1);
        }),
    ];

    let mut gesehen: BTreeMap<ZielfunktionsHash, &str> = BTreeMap::new();
    gesehen.insert(basis.fingerabdruck(), "basis");
    for (name, aendere) in aenderungen {
        let mut geaendert = basis;
        aendere(&mut geaendert);
        assert_ne!(
            geaendert, basis,
            "die Aenderung '{name}' aendert nichts — dann prueft sie nichts"
        );
        let fp = geaendert.fingerabdruck();
        if let Some(frueher) = gesehen.insert(fp, name) {
            panic!(
                "'{name}' ergibt denselben Fingerabdruck ({fp}) wie '{frueher}' — dieses \
                 Feld geht nicht in den Fingerabdruck ein, und ein Wechsel an ihm \
                 lieferte still das alte Urteil"
            );
        }
    }
    assert_eq!(gesehen.len(), aenderungen.len() + 1);
}

/// Eine Aenderung an einem Feld der Laufkonfiguration, mit Namen.
type LaufAenderung = (&'static str, fn(&mut BalanceConfig));

#[test]
fn der_fingerabdruck_der_semantik_folgt_dem_kampf_und_nicht_der_bewertung() {
    let apl = liste("e607-basis", BASIS);
    let basis = lauf_config(boss("boss-1-1"));
    let schluessel = ErgebnisSchluessel::fuer_lauf(&basis, &apl);
    assert_eq!(schluessel.semantik, SemantikHash::aus(&basis));
    assert_eq!(
        schluessel,
        ErgebnisSchluessel::fuer_lauf(&basis, &apl),
        "derselbe Lauf muss denselben Schluessel ergeben — sonst waere der Speicher \
         nie warm"
    );
    assert_eq!(schluessel.boss, "boss-1-1");
    assert!(
        schluessel.aufbau.starts_with("e607-basis#"),
        "der Name der Liste bleibt im Schluessel lesbar: {}",
        schluessel.aufbau
    );

    // ---- 1. Semantik: jede Aenderung ein eigener Semantik-Fingerabdruck ----
    let semantik: &[LaufAenderung] = &[
        ("iterations", |c| c.iterations += 1),
        ("base_seed", |c| c.base_seed += 1),
        ("duration_ms", |c| c.duration_ms = Some(60_000)),
        ("duration_ms.wert", |c| c.duration_ms = Some(60_001)),
        ("stop_on_kill", |c| c.stop_on_kill = !c.stop_on_kill),
        ("factors.essence", |c| c.factors.essence = 1.5),
        ("factors.talents", |c| c.factors.talents = 1.5),
        ("factors.gear_effects", |c| c.factors.gear_effects = 1.5),
        ("factors.buffs", |c| c.factors.buffs = 1.5),
        ("factors.synergy", |c| c.factors.synergy = 1.5),
        ("factors.resistance", |c| c.factors.resistance = 1.5),
        ("factors.vulnerability", |c| c.factors.vulnerability = 1.5),
        ("factors.crit", |c| c.factors.crit = 1.5),
    ];
    let mut gesehen: BTreeMap<SemantikHash, &str> = BTreeMap::new();
    gesehen.insert(schluessel.semantik, "basis");
    for (name, aendere) in semantik {
        let mut c = basis;
        aendere(&mut c);
        let k = ErgebnisSchluessel::fuer_lauf(&c, &apl);
        assert_eq!(
            k.aufbau, schluessel.aufbau,
            "'{name}' ist Semantik, nicht Aufbau"
        );
        if let Some(frueher) = gesehen.insert(k.semantik, name) {
            panic!(
                "'{name}' ergibt dieselbe Semantik wie '{frueher}' — ein Lauf mit \
                 geaendertem '{name}' bekaeme das alte Ergebnis"
            );
        }
    }

    // ---- 2. Aufbau: Ausruestung und Talente aendern den Aufbau-Teil ----
    let aufbau: &[LaufAenderung] = &[
        ("worn.catalysis", |c| c.worn.catalysis = 1),
        ("worn.purity", |c| c.worn.purity = 1),
        ("worn.yield_bonus", |c| c.worn.yield_bonus = 1),
        ("worn.essence", |c| c.worn.essence = 1),
        ("worn.vessel", |c| c.worn.vessel = 1),
        ("worn.ward", |c| c.worn.ward = 1),
        ("worn.gear_effects", |c| c.worn.gear_effects = 1),
        ("talent_resource_permille", |c| {
            c.talent_resource_permille = 1
        }),
        ("talent_dot_duration_permille", |c| {
            c.talent_dot_duration_permille = 1;
        }),
        ("talent_permille", |c| c.talent_permille = 1),
        ("talent_control_gain_permille", |c| {
            c.talent_control_gain_permille = 1;
        }),
        ("talent_decay_effect_permille", |c| {
            c.talent_decay_effect_permille = 1;
        }),
        ("talent_control_cost_permille", |c| {
            c.talent_control_cost_permille = 1;
        }),
        ("talent_control_effect_permille", |c| {
            c.talent_control_effect_permille = 1;
        }),
        ("talent_decay_stack_permille", |c| {
            c.talent_decay_stack_permille = 1;
        }),
        ("talent_decay_duration_permille", |c| {
            c.talent_decay_duration_permille = 1;
        }),
        ("talent_shield_amount_permille", |c| {
            c.talent_shield_amount_permille = 1;
        }),
        ("talent_shield_duration_permille", |c| {
            c.talent_shield_duration_permille = 1;
        }),
        ("talent_shield_stacks_bonus", |c| {
            c.talent_shield_stacks_bonus = 1
        }),
        ("talent_cooldown_reduction_permille", |c| {
            c.talent_cooldown_reduction_permille = 1;
        }),
        ("talent_cooldown_charges_bonus", |c| {
            c.talent_cooldown_charges_bonus = 1;
        }),
        ("talent_cooldown_threshold_permille", |c| {
            c.talent_cooldown_threshold_permille = 1;
        }),
        ("talent_spender_damage_permille", |c| {
            c.talent_spender_damage_permille = 1;
        }),
        ("talent_build_damage_permille", |c| {
            c.talent_build_damage_permille = 1;
        }),
        ("talent_decay_cap_bonus", |c| c.talent_decay_cap_bonus = 1),
        ("talent_decay_ward_permille", |c| {
            c.talent_decay_ward_permille = 1
        }),
        ("talent_convert_permille", |c| c.talent_convert_permille = 1),
    ];
    let mut aufbauten: BTreeMap<String, &str> = BTreeMap::new();
    aufbauten.insert(schluessel.aufbau.clone(), "basis");
    for (name, aendere) in aufbau {
        let mut c = basis;
        aendere(&mut c);
        let k = ErgebnisSchluessel::fuer_lauf(&c, &apl);
        assert_eq!(
            k.semantik, schluessel.semantik,
            "'{name}' gehoert zum Aufbau, nicht zur Semantik"
        );
        if let Some(frueher) = aufbauten.insert(k.aufbau.clone(), name) {
            panic!(
                "'{name}' ergibt denselben Aufbau wie '{frueher}' ({}) — zwei \
                 verschiedene Kaempfe bekaemen denselben Schluessel",
                k.aufbau
            );
        }
    }

    // ---- 3. NICHT im Schluessel: Rechenaufteilung und Bewertung ----
    //
    // Der wichtigste Teil dieses Falls. Stuenden Gewicht oder Bezugspunkt der
    // Bewertung im Schluessel, kostete eine geaenderte Gewichtung jeden Kampf.
    let nicht: &[LaufAenderung] = &[
        ("threads", |c| c.threads = 8),
        ("prevented_weight", |c| c.prevented_weight = 12.0),
        ("damage_taken_baseline", |c| {
            c.damage_taken_baseline = 4000.0
        }),
    ];
    for (name, aendere) in nicht {
        let mut c = basis;
        aendere(&mut c);
        assert_eq!(
            ErgebnisSchluessel::fuer_lauf(&c, &apl),
            schluessel,
            "'{name}' beruehrt keinen Kampf und darf den Schluessel nicht aendern"
        );
    }

    // ---- 4. Boss: ein eigenes Feld ----
    let anderer = BalanceConfig {
        boss: boss("boss-2-3"),
        ..basis
    };
    let k = ErgebnisSchluessel::fuer_lauf(&anderer, &apl);
    assert_eq!(k.boss, "boss-2-3");
    assert_eq!(k.semantik, schluessel.semantik);
    assert_eq!(k.aufbau, schluessel.aufbau);

    // ---- 5. Die Liste: Regeln ja, Quellzeilen nein ----
    let andere_liste = liste("e607-basis", K_CRUST);
    assert_ne!(
        ErgebnisSchluessel::fuer_lauf(&basis, &andere_liste).aufbau,
        schluessel.aufbau,
        "andere Regeln unter demselben Namen sind ein anderer Aufbau"
    );
    let mit_leerzeilen = liste("e607-basis", &format!("\n\n# verschoben\n{BASIS}"));
    assert_ne!(
        mit_leerzeilen.rules[0].line, apl.rules[0].line,
        "die Gegenprobe braucht verschobene Zeilen, sonst prueft sie nichts"
    );
    assert_eq!(
        ErgebnisSchluessel::fuer_lauf(&basis, &mit_leerzeilen),
        schluessel,
        "dieselben Regeln in anderen Zeilen rechnen denselben Kampf"
    );
}

/// Das Urteil aus dem Speicher ist dasselbe wie das aus dem Lauf.
///
/// # Warum das die Voraussetzung des ganzen Epics ist
///
/// Der Speicher haelt keine Einzelergebnisse. Liesse sich aus ihm nicht
/// **dasselbe** Urteil gewinnen wie aus dem frischen Lauf, waere ein
/// Zielwechsel ohne Kampf zwar billig, aber falsch.
///
/// Geprueft an einem gewoehnlichen und am umgekehrten Boss, dort mit einem
/// Sieger und einem Verlierer — die Siegzahl ist die eine Groesse, die der
/// Speicher anders bezieht als der Lauf.
#[test]
fn das_urteil_aus_dem_speicher_gleicht_dem_urteil_aus_dem_lauf() {
    let faelle: [(&str, &'static str, &str); 4] = [
        ("boss-1-1", "e607-flashpoint", FLASHPOINT),
        ("boss-1-1", "e607k-crust", K_CRUST),
        ("boss-2-3", "e939-schutz", SCHUTZ),
        ("boss-2-3", "e607-flashpoint", FLASHPOINT),
    ];
    let vertraege = [
        vertrag_a(),
        ziel_b().vertrag,
        RunContract {
            version: 3,
            determinism: Determinism::Stochastic,
            target_band: Some(TargetBand {
                low: 5000.0,
                high: 6000.0,
            }),
            ..ziel_b().vertrag
        },
    ];

    let mut siege = Vec::new();
    for (boss_id, name, quelle) in faelle {
        let gegner = boss(boss_id);
        let lauf = run(&lauf_config(gegner), &liste(name, quelle));
        let vektor = lauf
            .ergebnisentwurf()
            .abschliessen()
            .expect("ein echter Lauf ist vollstaendig");
        for vertrag in &vertraege {
            let aus_lauf =
                CandidateMeasurement::from_balance_result(&lauf, gegner, vertrag, Some(0.0));
            let aus_speicher =
                CandidateMeasurement::from_raw_outcome(&vektor, gegner, vertrag, Some(0.0));
            assert_eq!(
                aus_speicher, aus_lauf,
                "{name} gegen {boss_id} unter Vertrag {}: das Urteil aus dem Speicher \
                 weicht vom Urteil aus dem Lauf ab",
                vertrag.version
            );
            assert!(
                aus_lauf.stages.measured_count() >= 2,
                "ohne gemessene Stufen vergliche dieser Fall zwei leere Messungen"
            );
        }
        siege.push((name, boss_id, vektor.siege.0));
    }

    // Die Zusicherung, dass beide Ausgaenge vorkamen: Sonst haette der
    // Vergleich die Umkehrung nur an Siegern gesehen.
    assert!(
        siege.iter().any(|(_, _, s)| *s == 0) && siege.iter().any(|(_, _, s)| *s == DURCHGAENGE),
        "es braucht einen Kandidaten ohne und einen mit lauter Siegen: {siege:?}"
    );
}

// ------------------------------------------------------- Die Abnahme (E19)

/// **Die Abnahme des Epics.** Ein warmer Speicher, eine andere Zielfunktion:
/// null Kaempfe — und trotzdem ein anderes, richtiges Ergebnis.
#[test]
fn e19_ein_zielwechsel_ueber_echten_laeufen_rechnet_null_kaempfe_neu() {
    let listen = listen();
    let gegner = boss("boss-1-1");
    let kandidaten = kandidaten(&listen, lauf_config(gegner));
    let (a, b) = (ziel_a(), ziel_b());
    assert_ne!(
        a.fingerabdruck(),
        b.fingerabdruck(),
        "ohne verschiedene Fingerabdruecke gaebe es keinen Zielwechsel"
    );

    let mut verbund = Speicherverbund::neu();
    let mut laeufe = 0_u64;

    // ---- Kalt, unter Ziel A ----
    let kalt = verbund
        .bewerte_mit(&kandidaten, &a, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(
        kalt.gerechnete_kaempfe, 4,
        "ein kalter Speicher MUSS rechnen — sonst misst der Zaehler nichts"
    );
    assert_eq!(laeufe, 4, "und balance::run wurde viermal gerufen");
    let (bester_a, urteil_a) = bester_mit_beleg(&verbund, &kandidaten, &a);
    assert!(
        urteil_a.feasible,
        "unter Ziel A gibt es einen zulaessigen Bestwert"
    );

    // ---- DER ZIELWECHSEL ----
    let warm = verbund
        .bewerte_mit(&kandidaten, &b, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(
        warm.gerechnete_kaempfe, 0,
        "EIN ZIELWECHSEL DARF KEINEN EINZIGEN KAMPF KOSTEN — gerechnet wurden {}",
        warm.gerechnete_kaempfe
    );
    assert_eq!(
        laeufe, 4,
        "der Zaehler des Speichers sagt 0, aber balance::run wurde {} statt 4 mal gerufen",
        laeufe
    );
    assert!(warm.zielwechsel, "der Verbund hat den Wechsel bemerkt");
    assert_eq!(warm.urteile, 4, "bewertet wurde trotzdem jeder Kandidat");

    // ---- Beide Bewertungssaetze, keiner ueberschrieben ----
    assert_eq!(verbund.urteile.anzahl_fuer(a.fingerabdruck()), 4);
    assert_eq!(verbund.urteile.anzahl_fuer(b.fingerabdruck()), 4);
    assert_eq!(
        verbund.ergebnisse.anzahl(),
        4,
        "und nichts doppelt abgelegt"
    );

    // ---- Die positive Kontrolle: Der Wechsel hat die Antwort GEAENDERT ----
    //
    // Ohne sie waere null Kaempfe auch dann gruen, wenn die neue Zielfunktion
    // gar nicht angekommen waere.
    let (bester_b, urteil_b) = bester_mit_beleg(&verbund, &kandidaten, &b);
    assert_ne!(
        bester_a, bester_b,
        "zwei Zielfunktionen mit demselben Sieger pruefen den Wechsel nicht"
    );
    // Und er hat sie aus dem Grund geaendert, den der neue Vertrag mitbringt:
    // Unter B schlaegt der Sieger von B den von A ueber die
    // Ueberlebensreserve; unter A schlaegt der Sieger von A den von B ueber die
    // Siegzeit.
    let sieger = |aufbau: &str| {
        kandidaten
            .iter()
            .find(|k| aufbau.starts_with(&format!("{}#", k.apl.name)))
            .unwrap_or_else(|| panic!("{aufbau} ist keiner der Kandidaten"))
    };
    let unter_b = b
        .ziel
        .compare(&urteil_b, &urteil_von(&verbund, sieger(&bester_a), &b));
    assert_eq!(unter_b.ordering, Ordering::Greater);
    assert_eq!(
        unter_b.decided_by,
        Decision::Stage(RankingStage::SurvivalReserve),
        "unter B entscheidet die Ueberlebensreserve — die Stufe, die erst der neue \
         Vertrag misst"
    );
    let unter_a = a
        .ziel
        .compare(&urteil_a, &urteil_von(&verbund, sieger(&bester_b), &a));
    assert_eq!(unter_a.ordering, Ordering::Greater);
    assert_eq!(
        unter_a.decided_by,
        Decision::Stage(RankingStage::TimeToWin),
        "unter A entscheidet die Siegzeit"
    );

    // ---- Der alte Bestwert ist verworfen, nicht weitergereicht ----
    assert_eq!(
        verbund.archiv.optimalitaet.bester(a.fingerabdruck()),
        None,
        "nach dem Wechsel gibt es keinen Bestwert unter A mehr"
    );

    // ---- Das warme Urteil ist RICHTIG, nicht nur billig ----
    //
    // Frisch gerechnet, am Verbund vorbei: Jedes Urteil unter B muss dem
    // gleichen, das ein neuer Lauf unter B ergaebe.
    for kandidat in &kandidaten {
        let frisch = run(&kandidat.config, kandidat.apl);
        let erwartet = b.ziel.judge(
            &CandidateMeasurement::from_balance_result(
                &frisch,
                gegner,
                &b.vertrag,
                kandidat.aenderungskosten,
            )
            .with_bounds(kandidat.grenzen),
        );
        assert_eq!(
            urteil_von(&verbund, kandidat, &b),
            erwartet,
            "{}: das Urteil aus dem warmen Speicher weicht vom frischen ab",
            kandidat.apl.name
        );
    }
    assert_eq!(laeufe, 4, "die frischen Laeufe gingen am Verbund vorbei");
}

/// **Kontrollzeile 1:** Derselbe Lauf ohne Zielwechsel rechnet ebenfalls null.
///
/// Ohne sie misst die Abnahme nur, dass ein Zaehler auf null steht — nicht,
/// dass der Speicher warm ist.
#[test]
fn e19_kontrolle_eins_derselbe_lauf_ohne_zielwechsel_rechnet_null() {
    let listen = listen();
    let kandidaten = kandidaten(&listen, lauf_config(boss("boss-1-1")));
    let a = ziel_a();
    let mut verbund = Speicherverbund::neu();
    let mut laeufe = 0_u64;

    let kalt = verbund
        .bewerte_mit(&kandidaten, &a, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(kalt.gerechnete_kaempfe, 4);
    assert_eq!(laeufe, 4);
    let (vorher, _) = bester_mit_beleg(&verbund, &kandidaten, &a);

    let nochmal = verbund
        .bewerte_mit(&kandidaten, &a, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(
        nochmal.gerechnete_kaempfe, 0,
        "derselbe Lauf ueber einem warmen Speicher darf nichts rechnen"
    );
    assert_eq!(laeufe, 4, "balance::run wurde erneut gerufen");
    assert!(!nochmal.zielwechsel, "es war kein Zielwechsel");
    assert_eq!(
        verbund.urteile.anzahl(),
        4,
        "dieselbe Zielfunktion ueberschreibt ihre Urteile, statt sie zu verdoppeln"
    );
    let (nachher, _) = bester_mit_beleg(&verbund, &kandidaten, &a);
    assert_eq!(nachher, vorher, "und der Bestwert bleibt");
}

/// **Kontrollzeile 2:** Eine geaenderte Kampfsemantik rechnet ALLE Kaempfe neu.
///
/// Die wichtigere der beiden: Sie belegt, dass der Zaehler ueberhaupt
/// ausschlaegt. Ohne sie waere „null Kaempfe" auch dann gruen, wenn der
/// Speicher jeden Schluessel faelschlich als Treffer laese.
#[test]
fn e19_kontrolle_zwei_geaenderte_kampfsemantik_rechnet_alle_neu() {
    let listen = listen();
    let gegner = boss("boss-1-1");
    let alt = lauf_config(gegner);
    // Ein anderer Faktor der Schadensformel (Topf 1) — wie ein Kampf gerechnet
    // wird, nicht welcher Aufbau gerechnet wird.
    let neu = BalanceConfig {
        factors: DamageFactors {
            essence: 0.8,
            ..NEUTRAL
        },
        ..alt
    };
    assert_ne!(SemantikHash::aus(&alt), SemantikHash::aus(&neu));
    let a = ziel_a();
    let mut verbund = Speicherverbund::neu();
    let mut laeufe = 0_u64;

    let alte_kandidaten = kandidaten(&listen, alt);
    let kalt = verbund
        .bewerte_mit(&alte_kandidaten, &a, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(kalt.gerechnete_kaempfe, 4);

    let neue_kandidaten = kandidaten(&listen, neu);
    let danach = verbund
        .bewerte_mit(&neue_kandidaten, &a, gezaehlt(&mut laeufe))
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(
        danach.gerechnete_kaempfe, 4,
        "eine geaenderte Kampfsemantik MUSS alle Kaempfe neu rechnen — die alten \
         Ergebnisse gelten dann nicht mehr"
    );
    assert_eq!(laeufe, 8, "und balance::run wurde dafuer wirklich gerufen");
    assert!(!danach.zielwechsel, "die Zielfunktion blieb dieselbe");
    assert_eq!(
        verbund.ergebnisse.anzahl(),
        8,
        "beide Semantiken liegen nebeneinander, keine ueberschreibt die andere"
    );

    // Die Zusicherung, dass die Aenderung den Kampf beruehrt hat: Sonst haette
    // der Fall nur zwei Namen fuer dieselben Zahlen verglichen.
    for (vorher, nachher) in alte_kandidaten.iter().zip(&neue_kandidaten) {
        let v = verbund
            .ergebnisse
            .hole(&ErgebnisSchluessel::fuer_lauf(&vorher.config, vorher.apl))
            .expect("das alte Ergebnis liegt weiter");
        let n = verbund
            .ergebnisse
            .hole(&ErgebnisSchluessel::fuer_lauf(&nachher.config, nachher.apl))
            .expect("das neue Ergebnis liegt");
        assert_ne!(
            v, n,
            "{}: der geaenderte Faktor hat den Kampf nicht veraendert",
            vorher.apl.name
        );
    }
}

/// Der Bestwert folgt dem Zielvertrag — nicht der Reihenfolge, in der die
/// Kandidaten gemeldet werden.
///
/// Mit einem **unzulaessigen** Kandidaten darunter, der einmal zuerst und
/// einmal zuletzt kommt: Er darf weder liegen bleiben noch verdraengen.
#[test]
fn der_bestwert_folgt_dem_zielvertrag_nicht_der_meldereihenfolge() {
    let mut listen = listen();
    listen.insert(0, liste("step1-good", VERLIERT));
    let kandidaten = kandidaten(&listen, lauf_config(boss("boss-1-1")));
    let a = ziel_a();

    let mut verbund = Speicherverbund::neu();
    let bilanz = verbund
        .bewerte(&kandidaten, &a)
        .expect("echte Laeufe sind vollstaendig");
    assert_eq!(bilanz.gerechnete_kaempfe, 5);

    let verlierer = urteil_von(&verbund, &kandidaten[0], &a);
    assert!(
        !verlierer.feasible,
        "step1-good muss gegen boss-1-1 unzulaessig sein, sonst prueft dieser Fall nichts"
    );
    let (bester, _) = bester_mit_beleg(&verbund, &kandidaten, &a);

    let gemeldet: Vec<(String, CandidateVerdict)> = kandidaten
        .iter()
        .map(|k| {
            (
                ErgebnisSchluessel::fuer_lauf(&k.config, k.apl).aufbau,
                urteil_von(&verbund, k, &a),
            )
        })
        .collect();
    for umgekehrt in [false, true] {
        let mut reihe = gemeldet.clone();
        if umgekehrt {
            reihe.reverse();
        }
        let mut opt = Optimalitaet::<CandidateVerdict>::default();
        for (aufbau, urteil) in reihe {
            opt.melde_urteil(aufbau, urteil, &a);
        }
        let (name, urteil) = opt.bester(a.fingerabdruck()).expect("es wurde gemeldet");
        assert_eq!(
            name, bester,
            "die Reihenfolge (umgekehrt: {umgekehrt}) hat den Bestwert veraendert"
        );
        assert!(urteil.feasible);
    }
}
