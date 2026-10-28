//! Anker für die Testlisten der Wirkungsbeschreibung (Epic #892, MAP3).
//!
//! # Warum es diese Datei gibt
//!
//! Eine Liste aller Werte einer Aufzählung, die in einem Prüffall neben dem Code
//! steht, altert still: Kommt ein Baustein oder ein Auslöser dazu, prüft die
//! Liste weiter die alten, und der Fall bleibt grün. Bis #892 standen solche
//! Listen in `effects_felder.rs` (`BASIS_BAUSTEINE`, die Wertelisten je Feld,
//! `assert_eq!(geprueft, 10)`) und `effects_rueckfall.rs` (`bausteine()`, die
//! Bezüge der Monotonie). Der Anker in `bausteine()` erzwang nur das Hinsehen:
//! Der `match` daneben war erschöpfend, die Liste darüber durfte trotzdem einen
//! Wert auslassen.
//!
//! [`jeder_wert!`] baut Liste und Anker aus **denselben** Namen. Fehlt der Liste
//! ein Wert, ist der `match` nicht erschöpfend, und die Prüfdatei übersetzt
//! nicht (E0004). Steht ein Wert doppelt, ist ein Arm unerreichbar, und
//! `clippy -D warnings` bricht. [`felder!`] tut dasselbe für die Felder von
//! [`EffectNode`]: ein Muster ohne `..` aus denselben Namen wie die Liste.
//!
//! Nicht jede Prüfdatei braucht jeden Anker; deshalb `dead_code` erlaubt.

#![allow(dead_code)]

use rota_sim::effects::{
    EffectOp, LocalMonotonicityHint, MonotonicDirection, MonotonicityDomain, RandomnessSemantics,
    SignatureId, StackingSemantics, Target, TimingSemantics, Trigger,
};
use rota_sim::rng::Stream;

/// Alle Werte einer Aufzählung als `Vec` — und ihr erschöpfender Anker aus
/// denselben Namen.
///
/// ```ignore
/// jeder_wert!(StackingSemantics { None, Refresh, Capped(3), Unknown })
/// ```
///
/// Eine Variante mit Nutzlast trägt einen Beispielwert in Klammern; der Anker
/// nimmt sie als `Variante { .. }`, gleich welche Nutzlast. Der Typ muss am
/// Aufrufort unter seinem Namen sichtbar sein.
macro_rules! jeder_wert {
    ($typ:ident { $($wert:ident $(($nutzlast:expr))?),+ $(,)? }) => {{
        // Nie aufgerufen: Er existiert, damit der Übersetzer die Liste prüft.
        #[allow(dead_code)]
        fn anker(wert: $typ) {
            match wert {
                $($typ::$wert { .. } => {})+
            }
        }
        vec![$($typ::$wert $(($nutzlast))?),+]
    }};
}

/// Die Felder des Knotens, je mit einem Setzer pro Wert — und ein Anker, der
/// jedes Feld verlangt.
///
/// Ergibt `Vec<(&'static str, Vec<Box<dyn Fn(&mut EffectNode)>>)>`. Der Anker ist
/// ein Muster über `EffectNode` ohne `..`, gebaut aus denselben Namen: Bekommt
/// der Knoten ein Feld dazu, übersetzt die Prüfdatei nicht, bis es hier steht
/// (E0027). `op` (die Fallunterscheidung) und `origin` (die Herkunft) tragen
/// keine Werte und stehen fest im Muster. `EffectNode` muss am Aufrufort
/// sichtbar sein; die Werte sind `Copy`.
#[allow(unused_macros)]
macro_rules! felder {
    ($($feld:ident: $werte:expr),+ $(,)?) => {{
        #[allow(dead_code)]
        fn anker(knoten: EffectNode) {
            let EffectNode {
                op: _,
                origin: _,
                $($feld: _),+
            } = knoten;
        }
        let felder: Vec<(&'static str, Vec<Box<dyn Fn(&mut EffectNode)>>)> = vec![$((
            stringify!($feld),
            $werte
                .into_iter()
                .map(|wert| {
                    Box::new(move |n: &mut EffectNode| n.$feld = wert) as Box<dyn Fn(&mut EffectNode)>
                })
                .collect(),
        )),+];
        felder
    }};
}

/// Jeder Baustein, eine Bossregel eingeschlossen.
pub fn jeder_baustein() -> Vec<EffectOp> {
    jeder_wert!(EffectOp {
        DealDamage,
        ApplyDot,
        ApplyBuff,
        ModifyResource,
        ReduceCooldown,
        ResetCooldown,
        FreeNextGcd,
        Heal,
        Shield,
        ConvertDamage,
        ChangeBossState,
        ChangePhase,
        CustomSignature(SignatureId("prueffall/anker")),
    })
}

/// Jeder Auslöser, `Unknown` eingeschlossen.
pub fn jeder_ausloeser() -> Vec<Trigger> {
    jeder_wert!(Trigger {
        OnCast,
        OnTick,
        OnExpire,
        OnTimeline,
        OnPhaseChange,
        Unknown,
    })
}

/// Jedes Ziel, `Unknown` eingeschlossen.
pub fn jedes_ziel() -> Vec<Target> {
    jeder_wert!(Target {
        Boss,
        Player,
        AllCooldowns,
        OneCooldown,
        Unknown,
    })
}

/// Jedes Zeitverhalten, `Unknown` eingeschlossen.
pub fn jedes_zeitverhalten() -> Vec<TimingSemantics> {
    jeder_wert!(TimingSemantics {
        Immediate,
        Snapshot,
        Live,
        Unknown,
    })
}

/// Jedes Stapelverhalten, `Unknown` eingeschlossen; `Capped` mit Grenze 3.
pub fn jedes_stapelverhalten() -> Vec<StackingSemantics> {
    jeder_wert!(StackingSemantics {
        None,
        Refresh,
        Extend,
        Additive,
        Capped(3),
        Replace,
        Unknown,
    })
}

/// Jede Zufallsquelle, `Unknown` eingeschlossen; `Stream` mit dem Brand-Strom.
pub fn jede_zufallsquelle() -> Vec<RandomnessSemantics> {
    jeder_wert!(RandomnessSemantics {
        Deterministic,
        CritRoll,
        Stream(Stream::DotTick),
        Unknown,
    })
}

/// Jede Monotonie-Richtung, `Unknown` eingeschlossen.
pub fn jede_richtung() -> Vec<MonotonicDirection> {
    jeder_wert!(MonotonicDirection {
        Increasing,
        Decreasing,
        NonMonotonic,
        Unknown,
    })
}

/// Jeder Bezug eines Monotonie-Hinweises, `Unspecified` eingeschlossen.
pub fn jeder_bezug() -> Vec<MonotonicityDomain> {
    jeder_wert!(MonotonicityDomain {
        ExpectedDamageStableTrace,
        VictoryOutcome,
        TimeToVictoryAcrossPhase,
        HpPredicate,
        SpellDiversity,
        Unspecified,
    })
}

/// Jeder Monotonie-Hinweis: jede Richtung mit jedem Bezug.
///
/// Auch die ohne Richtung oder Bezug: Beide sind gültige Deklarationen (Weg b,
/// siehe `effects_rueckfall.rs`), keine Lücke.
pub fn jeder_hinweis() -> Vec<LocalMonotonicityHint> {
    let bezuege = jeder_bezug();
    jede_richtung()
        .into_iter()
        .flat_map(|direction| {
            bezuege
                .iter()
                .map(move |&domain| LocalMonotonicityHint { direction, domain })
        })
        .collect()
}
