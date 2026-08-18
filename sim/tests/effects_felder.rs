//! Jedes Feld des Wirkungsknotens hat einen Leser (Issue #966, AC2).
//!
//! # Warum das ein eigener Prueflauf ist
//!
//! Ein Knotentyp mit zehn Feldern sieht vollstaendig aus. Ob die Felder
//! **gelesen** werden, sagt er nicht — und ein Feld ohne Leser ist genau die
//! „gepflegte Liste, nur eine Ebene tiefer versteckt", gegen die Epic E16
//! gebaut ist. Die Stop-Bedingung des Issues lautet deshalb: **kein Feld ohne
//! Leser.**
//!
//! # Die Frage muss richtig gestellt werden
//!
//! Der erste Entwurf mutierte je Feld **einen** Wert auf einen anderen und
//! fragte, ob sich etwas aendert. Damit meldete er `stacking` als ungelesen —
//! obwohl es einen Leser hat. Die gewaehlten zwei Werte (`None` und `Refresh`)
//! lagen zufaellig im selben Zweig. **Die Schwaeche lag in der Mutation, nicht
//! im Leser**, und haette ich sie „passend" gemacht, waere der Prueflauf
//! beliebig geworden.
//!
//! Deshalb probiert dieser Lauf **alle** sinnvollen Werte jedes Feldes durch
//! und fragt: Gibt es **zwei**, die zu verschiedenen Ableitungen fuehren? Das
//! ist die Frage, die „hat ein Leser" wirklich bedeutet, und sie laesst sich
//! nicht durch die Wahl der Mutation schoenen.
//!
//! # Die zweite Falle
//!
//! Jeder Knoten wird von jeder Analyse zuerst durch [`EffectNode::hat_luecke`]
//! geschickt. Ein Feld auf seinen `Unknown`-Wert zu setzen aendert deshalb
//! **immer** alle Ergebnisse — und misst nur, dass die Lueckenpruefung
//! funktioniert. Kein `Unknown` kommt in den Wertelisten vor, und
//! [`ohne_luecke`] sichert das ab. Ohne diese Zusicherung waere der ganze
//! Prueflauf gruen und bedeutungslos.

use rota_sim::effects::passes::beispielknoten;
use rota_sim::effects::registry::{alle_analysen, AnalysisResult};
use rota_sim::effects::{
    Bounds, ControlFlowEffect, EffectNode, EffectOp, LocalMonotonicityHint, MonotonicDirection,
    MonotonicityDomain, RandomnessSemantics, SignatureId, StackingSemantics, StateMask, Target,
    TimingSemantics, Trigger,
};

/// Die Bausteine, ueber denen jedes Feld geprueft wird.
///
/// Mehrere, weil ein Feld nur fuer bestimmte Bausteine einen Leser haben kann:
/// `stacking` entscheidet bei einem Brand etwas anderes als bei einem
/// Ruecksetzer. Ein Feld gilt als gelesen, sobald es bei **irgendeinem**
/// Baustein zwei unterscheidbare Ableitungen erzeugt.
const BASIS_BAUSTEINE: &[EffectOp] = &[
    EffectOp::DealDamage,
    EffectOp::ApplyDot,
    EffectOp::ApplyBuff,
    EffectOp::ModifyResource,
    EffectOp::ReduceCooldown,
    EffectOp::ResetCooldown,
    EffectOp::FreeNextGcd,
    EffectOp::Heal,
    EffectOp::Shield,
    EffectOp::ConvertDamage,
    EffectOp::ChangeBossState,
    EffectOp::ChangePhase,
    EffectOp::CustomSignature(SignatureId("prueffall")),
];

/// Sichert, dass ein Knoten keine Luecke hat — sonst misst der Vergleich nur
/// [`EffectNode::hat_luecke`] und nicht den Leser des Feldes.
fn ohne_luecke(knoten: &EffectNode, wo: &str) {
    assert!(
        !knoten.hat_luecke(),
        "{wo}: Der Knoten traegt eine Luecke. Dann wuerde der Vergleich nur die \
         Lueckenpruefung ausloesen und ueber den Leser des Feldes NICHTS aussagen."
    );
}

/// Alle Setzer eines Feldes, jeder mit einem **gueltigen** Wert.
type Setzer = Box<dyn Fn(&mut EffectNode)>;

/// Ob zwei Werte des Feldes zu verschiedenen Ableitungen fuehren.
///
/// Vergleicht die **vollstaendigen** Ergebnisse, nicht nur die Verdikte: Drei
/// der Analysen sind beschreibend und tragen ihre Aussage in der `evidence`
/// (siehe Modul-Kopf von `effects::passes`). Wer nur Verdikte vergleicht,
/// haelt sie faelschlich fuer wirkungslos.
fn hat_leser(feld: &str, werte: &[Setzer]) -> bool {
    assert!(
        werte.len() >= 2,
        "{feld}: mit weniger als zwei Werten kann dieser Fall nichts feststellen."
    );
    for op in BASIS_BAUSTEINE {
        let mut gesehen: Vec<Vec<AnalysisResult>> = Vec::new();
        for setzen in werte {
            let mut knoten = beispielknoten(*op);
            setzen(&mut knoten);
            ohne_luecke(&knoten, &format!("{feld} bei {}", op.name()));
            let ergebnis = alle_analysen(&knoten);
            assert!(
                !ergebnis.is_empty(),
                "{feld}: keine Analysen registriert — dann prueft dieser Fall nichts."
            );
            if !gesehen.contains(&ergebnis) {
                gesehen.push(ergebnis);
            }
        }
        if gesehen.len() >= 2 {
            return true;
        }
    }
    false
}

/// Die zehn Felder mit je allen sinnvollen Werten.
///
/// `op` steht nicht dabei: Es ist kein Feld neben den anderen, sondern die
/// Fallunterscheidung, ueber die jede Analyse ohnehin ihr `match` fuehrt.
fn felder() -> Vec<(&'static str, Vec<Setzer>)> {
    vec![
        (
            "trigger",
            vec![
                Box::new(|n: &mut EffectNode| n.trigger = Trigger::OnCast) as Setzer,
                Box::new(|n: &mut EffectNode| n.trigger = Trigger::OnTick),
                Box::new(|n: &mut EffectNode| n.trigger = Trigger::OnExpire),
                Box::new(|n: &mut EffectNode| n.trigger = Trigger::OnTimeline),
                Box::new(|n: &mut EffectNode| n.trigger = Trigger::OnPhaseChange),
            ],
        ),
        (
            "target",
            vec![
                Box::new(|n: &mut EffectNode| n.target = Target::Boss) as Setzer,
                Box::new(|n: &mut EffectNode| n.target = Target::Player),
                Box::new(|n: &mut EffectNode| n.target = Target::AllCooldowns),
                Box::new(|n: &mut EffectNode| n.target = Target::OneCooldown),
            ],
        ),
        (
            "reads",
            vec![
                Box::new(|n: &mut EffectNode| n.reads = StateMask::RESOURCE) as Setzer,
                Box::new(|n: &mut EffectNode| n.reads = StateMask::BUFFS.union(StateMask::DOTS)),
                Box::new(|n: &mut EffectNode| n.reads = StateMask::EMPTY),
            ],
        ),
        (
            "writes",
            vec![
                Box::new(|n: &mut EffectNode| n.writes = StateMask::BOSS_HP) as Setzer,
                Box::new(|n: &mut EffectNode| {
                    n.writes = StateMask::PLAYER_HP.union(StateMask::SHIELD)
                }),
                Box::new(|n: &mut EffectNode| n.writes = StateMask::EMPTY),
            ],
        ),
        (
            "timing",
            vec![
                Box::new(|n: &mut EffectNode| n.timing = TimingSemantics::Immediate) as Setzer,
                Box::new(|n: &mut EffectNode| n.timing = TimingSemantics::Snapshot),
                Box::new(|n: &mut EffectNode| n.timing = TimingSemantics::Live),
            ],
        ),
        (
            "stacking",
            vec![
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::None) as Setzer,
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::Refresh),
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::Extend),
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::Additive),
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::Capped(3)),
                Box::new(|n: &mut EffectNode| n.stacking = StackingSemantics::Replace),
            ],
        ),
        (
            "randomness",
            vec![
                Box::new(|n: &mut EffectNode| n.randomness = RandomnessSemantics::Deterministic)
                    as Setzer,
                Box::new(|n: &mut EffectNode| n.randomness = RandomnessSemantics::CritRoll),
                Box::new(|n: &mut EffectNode| {
                    n.randomness = RandomnessSemantics::Stream(rota_sim::rng::Stream::DotTick)
                }),
            ],
        ),
        (
            "monotonicity",
            vec![
                Box::new(|n: &mut EffectNode| {
                    n.monotonicity = LocalMonotonicityHint {
                        direction: MonotonicDirection::Increasing,
                        domain: MonotonicityDomain::ExpectedDamageStableTrace,
                    }
                }) as Setzer,
                Box::new(|n: &mut EffectNode| {
                    n.monotonicity = LocalMonotonicityHint {
                        direction: MonotonicDirection::NonMonotonic,
                        domain: MonotonicityDomain::ExpectedDamageStableTrace,
                    }
                }),
                Box::new(|n: &mut EffectNode| {
                    n.monotonicity = LocalMonotonicityHint {
                        direction: MonotonicDirection::Increasing,
                        domain: MonotonicityDomain::VictoryOutcome,
                    }
                }),
            ],
        ),
        (
            "numeric_bounds",
            vec![
                Box::new(|n: &mut EffectNode| n.numeric_bounds = Bounds::exact(0, 42)) as Setzer,
                Box::new(|n: &mut EffectNode| n.numeric_bounds = Bounds::UNKNOWN),
                Box::new(|n: &mut EffectNode| {
                    n.numeric_bounds = Bounds {
                        min: Some(0),
                        max: None,
                    }
                }),
            ],
        ),
        (
            "control_flow_effect",
            vec![
                Box::new(|n: &mut EffectNode| n.control_flow_effect = ControlFlowEffect::NONE)
                    as Setzer,
                Box::new(|n: &mut EffectNode| {
                    n.control_flow_effect = ControlFlowEffect {
                        branches_on_damage: true,
                        branches_on_hp: false,
                        branches_on_victory: false,
                        unknown: false,
                    }
                }),
                Box::new(|n: &mut EffectNode| {
                    n.control_flow_effect = ControlFlowEffect {
                        branches_on_damage: false,
                        branches_on_hp: false,
                        branches_on_victory: true,
                        unknown: false,
                    }
                }),
            ],
        ),
    ]
}

#[test]
fn kein_feld_ohne_leser() {
    let mut ohne: Vec<&str> = Vec::new();
    let mut geprueft = 0usize;
    for (name, werte) in felder() {
        geprueft += 1;
        if !hat_leser(name, &werte) {
            ohne.push(name);
        }
    }
    assert_eq!(
        geprueft, 10,
        "Es wurden {geprueft} Felder geprueft, der Knoten hat zehn. Kommt ein Feld dazu, \
         gehoert es in diese Liste — sonst waechst der Knoten an der Pruefung vorbei."
    );
    assert!(
        ohne.is_empty(),
        "Diese Felder des Wirkungsknotens haben KEINEN Leser — kein einziges Wertepaar fuehrt \
         zu verschiedenen Ableitungen: {ohne:?}. Ein Feld ohne Leser ist die gepflegte Liste \
         eine Ebene tiefer: Es sieht nach Semantik aus und traegt keine. (Stop-Bedingung von \
         Issue #966.)"
    );
}

#[test]
fn kein_wert_der_wertelisten_erzeugt_eine_luecke() {
    // Die Zusicherung ueber die Zusicherung: Enthielte eine Werteliste einen
    // `Unknown`-Wert, waere `kein_feld_ohne_leser` gruen, ohne etwas ueber
    // Leser zu sagen — die Lueckenpruefung allein aendert schon jedes
    // Ergebnis. Dieser Fall haelt fest, dass ueber ALLE Felder, Werte und
    // Bausteine geprueft wird.
    let mut paare = 0usize;
    for (name, werte) in felder() {
        for setzen in &werte {
            for op in BASIS_BAUSTEINE {
                let mut knoten = beispielknoten(*op);
                setzen(&mut knoten);
                ohne_luecke(&knoten, &format!("{name} bei {}", op.name()));
                paare += 1;
            }
        }
    }
    let erwartet: usize =
        felder().iter().map(|(_, w)| w.len()).sum::<usize>() * BASIS_BAUSTEINE.len();
    assert_eq!(
        paare, erwartet,
        "Es wurden {paare} Paare geprueft, erwartet {erwartet} — die Schleife hat nicht alles \
         angesehen."
    );
    assert!(
        paare > 0,
        "Kein einziges Paar geprueft — dann sichert dieser Fall nichts ab."
    );
}
