//! Unbekanntes fällt konservativ zurück — je Bedingung, nicht im Ganzen (Epic #892).
//!
//! # Was hier festgenagelt wird
//!
//! **DW3** („Eine Sonderregel ohne vollständige Semantik fällt nachweislich auf
//! den konservativen Pfad“) und **MAP2** („ein Prüffall mit unvollständiger
//! Deklaration“) aus Epic #892. Bis hierher belegte das ein einziges Feld
//! (`timing`, in `effects_analysen.rs`) und der völlig blinde Knoten. Ein
//! Knoten, dem genau EINE andere Angabe fehlt, war ungeprüft — und genau der ist
//! der Alltagsfall: der fast fertige Knoten, nicht der leere.
//!
//! Je Bedingung von [`EffectNode::hat_luecke`] stehen deshalb zwei Fälle da:
//! `an_der_signatur::*` an einer Bossregel (DW3) und `am_baustein::*` an einem
//! Schadensknoten (MAP2). Beide gehen von einem VOLLSTÄNDIGEN Knoten aus,
//! setzen genau ein Feld auf seinen Unbekannt-Wert und verlangen, dass jede
//! registrierte Analyse zurückfällt. Geprüft wird die Folge (`alle_konservativ`),
//! nicht `hat_luecke` selbst — sonst wäre eine gestrichene Bedingung an der
//! falschen Zusicherung rot.
//!
//! # Abweichung vom Wortlaut von #911 AC4 — Festlegung des Orchestrators
//!
//! #911 AC4 zählt zu den Angaben, die eine Signatur deklarieren muss, auch
//! „Wertebereiche, Monotonie“. `hat_luecke` prüft beide nicht, und das bleibt so
//! (Weg b, vom Orchestrator für #892 festgelegt): [`Bounds::UNKNOWN`] und ein
//! Hinweis ohne Richtung sind GÜLTIGE Deklarationen. „Der Bereich ist
//! unbekannt“ ist eine Aussage, keine fehlende — so ist es mit #966 abgenommen,
//! `effects_felder.rs` führt `Bounds::UNKNOWN` als gültigen Wert eines Feldes.
//!
//! Der andere Weg hätte jede deklarierte Bossregel im Ganzen zurückfallen lassen:
//! Jeder fehlt heute der Bereich oder die Richtung — gemessen am 2026-10-24 an
//! 115 deklarierten Signaturknoten, alle elf Signaturarten, in
//! [`die_echten_signaturen_sind_deklariert_und_fallen_nicht_im_ganzen_zurueck`].
//! Laut `sim/build.rs` (`knoten_fuer_boss`) fehlt sogar beides.
//!
//! Weg b verlangt dafür den Nachweis an jeder Stelle, die den Wert LIEST: Dort
//! muss „unbekannt“ den konservativen Ausgang erzwingen. Welche Analysen ihn
//! lesen, wird gemessen und nicht aufgezählt
//! ([`jeder_gemessene_leser_von_wertebereich_und_monotonie_ist_hier_geprueft`]).
//!
//! # Gemessen am 2026-10-24, als Mutation (`/work/tmp`, nicht eingecheckt)
//!
//! Je Bedingung aus `hat_luecke` gestrichen: rot genau `an_der_signatur::<feld>`
//! und `am_baustein::<feld>` (25 bestanden, 2 durchgefallen), an der Folge, nicht
//! an `hat_luecke`. Je Leser ein Eingriff, der „unbekannt“ wie bekannt behandelt
//! oder eine Schranke erfindet: rot an seinem Fall. `Bounds::UNKNOWN` als Lücke
//! (Weg a): fünf Fälle rot, darunter der Weg-b-Kern. Nur die eigene
//! Richtungsprüfung in `zertifikat` gestrichen: rot an
//! [`ohne_richtung_oder_bezug_gibt_es_kein_zertifikat_auch_mit_allen_zutaten`]
//! (26 bestanden, 1 durchgefallen) — solange der Fall nur `is_err()` verlangte,
//! blieb das grün (27 bestanden). Rückbau jedes Mal bytegleich.
//!
//! # Was hier NICHT belegt ist
//!
//! Dass ein neuer Baustein den Bau bricht (DW1/DW2): Das ist ein
//! Übersetzungsfehler und steht in `tools/wirkung-baubruch-gegenprobe.py`, das
//! der sim-Job bei jeder Änderung fährt.

use rota_sim::data::EFFECT_NODES;
use rota_sim::effects::passes::{anzahl_analysen, beispielknoten, REGISTRY};
use rota_sim::effects::registry::{
    alle_analysen, alle_konservativ, AnalysisResult, Evidence, Verdict,
};
use rota_sim::effects::{
    zertifikat, Bounds, ControlFlowEffect, EffectNode, EffectOp, LocalMonotonicityHint,
    MonotonicDirection, MonotonicityDomain, NumerischerVertrag, RandomnessSemantics, SignatureId,
    StackingSemantics, StateMask, Target, TimingSemantics, Trigger, ZertifikatVerweigert,
    ZertifikatsBedarf,
};

/// Alle Bausteine, für die Messung der Leser unten.
///
/// Der `match` hat keinen Auffangzweig: Ein neuer Baustein lässt diese Datei
/// nicht übersetzen, bis er hier eingeordnet ist — und wer ihn einordnet, sieht
/// die Liste direkt darüber. Dass er dann auch IN der Liste steht, prüft
/// nichts. Der Anker zwingt nur zum Hinsehen.
fn bausteine() -> Vec<EffectOp> {
    let liste = vec![
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
        EffectOp::CustomSignature(SignatureId("prueffall/rueckfall")),
    ];
    for op in &liste {
        match op {
            EffectOp::DealDamage
            | EffectOp::ApplyDot
            | EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::Heal
            | EffectOp::Shield
            | EffectOp::ConvertDamage
            | EffectOp::ChangeBossState
            | EffectOp::ChangePhase
            | EffectOp::CustomSignature(_) => {}
        }
    }
    liste
}

/// Eine Bossregel mit vollständiger Semantik.
fn signatur() -> EffectNode {
    let mut knoten = beispielknoten(EffectOp::CustomSignature(SignatureId("prueffall/dw3")));
    knoten.origin = "prueffall/dw3";
    knoten
}

fn konservativ(knoten: &EffectNode) -> usize {
    alle_analysen(knoten)
        .iter()
        .filter(|r| r.verdict == Verdict::Conservative)
        .count()
}

/// Das Ergebnis einer Analyse über einen Knoten, gesucht über ihren Namen.
fn ergebnis(pass: &str, knoten: &EffectNode) -> AnalysisResult {
    alle_analysen(knoten)
        .into_iter()
        .find(|r| r.pass.0 == pass)
        .unwrap_or_else(|| panic!("Analyse \"{pass}\" ist nicht registriert"))
}

/// Vom vollständigen Knoten aus genau EIN Feld auf unbekannt — dann fällt jede
/// registrierte Analyse zurück, mit allen sechs konservativen Zuständen.
fn genau_eine_angabe_fehlt(mut knoten: EffectNode, feld: &str, setzen: impl Fn(&mut EffectNode)) {
    assert!(
        !knoten.hat_luecke(),
        "{feld}: Der Ausgangsknoten ist schon unvollständig — dann fehlt nicht genau eine \
         Angabe, und der Fall prüft nichts."
    );
    let vorher = konservativ(&knoten);
    assert!(
        vorher < anzahl_analysen(),
        "{feld}: Schon der vollständige Knoten fällt überall zurück ({vorher} von {}). Dann \
         unterscheidet der Rückfall nichts, und dieser Fall wäre auch ohne Lücke grün.",
        anzahl_analysen()
    );

    setzen(&mut knoten);

    // **Die Folge zuerst, die Benennung danach.** Wer eine Bedingung aus
    // `hat_luecke` streicht, soll HIER rot werden — an der Zusicherung, um
    // die es geht.
    let ergebnisse = alle_analysen(&knoten);
    assert_eq!(
        ergebnisse.len(),
        anzahl_analysen(),
        "{feld}: Es haben {} Analysen geantwortet, registriert sind {}.",
        ergebnisse.len(),
        anzahl_analysen()
    );
    let nicht: Vec<String> = ergebnisse
        .iter()
        .filter(|r| r.verdict != Verdict::Conservative)
        .map(|r| format!("{} ({:?}: {})", r.pass, r.verdict, r.reason))
        .collect();
    assert!(
        nicht.is_empty(),
        "Nur `{feld}` fehlt — und diese Analysen fallen trotzdem nicht zurück: {nicht:?}. \
         Vorher waren {vorher} von {} konservativ. Unbekannt muss langsam sein, nie falsch.",
        anzahl_analysen()
    );
    assert!(
        alle_konservativ(&knoten),
        "{feld}: alle_konservativ widerspricht den Einzelurteilen"
    );
    for r in &ergebnisse {
        assert!(
            r.fallback.vollstaendig(),
            "{feld}: \"{}\" setzt nur {} von 6 konservativen Zuständen — ein halber Rückfall \
             sieht aus wie Vorsicht und ist keine.",
            r.pass,
            r.fallback.anzahl()
        );
    }
    assert!(
        knoten.hat_luecke(),
        "{feld}: fällt zurück, gilt aber nicht als Lücke"
    );
    if matches!(knoten.op, EffectOp::CustomSignature(_)) {
        assert!(
            knoten.signature_ohne_semantik(),
            "{feld}: Die Signatur fällt zurück, gilt aber nicht als Signatur ohne Semantik."
        );
    }
}

/// Je Bedingung von `hat_luecke` ein Fall an der Signatur (DW3) und einer am
/// Baustein (MAP2). Eine neue Bedingung gehört hier dazu; ein neues Feld zwingt
/// zum Hinsehen ([`jedes_feld_des_knotens_ist_hier_eingeordnet`]), eine neue
/// Bedingung an einem alten Feld nicht.
macro_rules! je_bedingung {
    ($($name:ident: $feld:literal, |$k:ident| $setzen:expr;)+) => {
        /// DW3 aus #892: eine Bossregel, der genau eine Angabe fehlt.
        mod an_der_signatur {
            use super::*;
            $(
                #[test]
                fn $name() {
                    genau_eine_angabe_fehlt(signatur(), $feld, |$k: &mut EffectNode| $setzen);
                }
            )+
        }

        /// MAP2 aus #892: ein gewöhnlicher Baustein, dem genau eine Angabe fehlt.
        mod am_baustein {
            use super::*;
            $(
                #[test]
                fn $name() {
                    genau_eine_angabe_fehlt(
                        beispielknoten(EffectOp::DealDamage),
                        $feld,
                        |$k: &mut EffectNode| $setzen,
                    );
                }
            )+
        }
    };
}

je_bedingung! {
    ausloeser: "trigger", |k| k.trigger = Trigger::Unknown;
    ziel: "target", |k| k.target = Target::Unknown;
    lesemaske: "reads", |k| k.reads = StateMask::UNKNOWN;
    schreibmaske: "writes", |k| k.writes = StateMask::UNKNOWN;
    zeitverhalten: "timing", |k| k.timing = TimingSemantics::Unknown;
    stapelverhalten: "stacking", |k| k.stacking = StackingSemantics::Unknown;
    zufall: "randomness", |k| k.randomness = RandomnessSemantics::Unknown;
    kontrollfluss: "control_flow_effect", |k| k.control_flow_effect = ControlFlowEffect::UNKNOWN;
}

/// Ein Übersetzungs-Anker, kein Laufzeitfall.
///
/// Das Muster nennt jedes Feld und hat kein `..`: Bekommt [`EffectNode`] ein
/// Feld dazu, übersetzt diese Datei nicht, bis es hier eingeordnet ist — als
/// Lückenbedingung (dann gehört es in `je_bedingung!`), als gültige
/// Unbekannt-Deklaration (dann gehört sein Leser in die Messung unten) oder als
/// Feld ohne Semantik.
#[test]
fn jedes_feld_des_knotens_ist_hier_eingeordnet() {
    let EffectNode {
        // Die Fallunterscheidung selbst und die Herkunft: keine Semantik.
        op: _,
        origin: _,
        // Die acht Bedingungen von `hat_luecke`.
        trigger: _,
        target: _,
        reads: _,
        writes: _,
        timing: _,
        stacking: _,
        randomness: _,
        control_flow_effect: _,
        // Gültig unbekannt (Weg b): geprüft an jedem Leser.
        monotonicity: _,
        numeric_bounds: _,
    } = beispielknoten(EffectOp::DealDamage);
}

// ------------------------------------------- Weg b: Wertebereich und Monotonie

/// Ein vollständiger Knoten, dessen Wertebereich ausdrücklich unbekannt ist.
fn mit_unbekanntem_bereich(op: EffectOp) -> EffectNode {
    let mut knoten = beispielknoten(op);
    knoten.numeric_bounds = Bounds::UNKNOWN;
    knoten
}

#[test]
fn wertebereich_und_monotonie_unbekannt_sind_gueltige_deklarationen() {
    // Weg b im Kern: kein Rückfall im Ganzen. Wer hier eine Lücke daraus macht,
    // wählt Weg a — und dieser Fall verlangt, dass das bewusst geschieht.
    for op in [
        EffectOp::DealDamage,
        EffectOp::CustomSignature(SignatureId("prueffall/weg-b")),
    ] {
        let mut knoten = beispielknoten(op);
        knoten.numeric_bounds = Bounds::UNKNOWN;
        knoten.monotonicity = LocalMonotonicityHint::UNKNOWN;
        assert!(
            !knoten.hat_luecke(),
            "{op:?}: Ein unbekannter Wertebereich oder Hinweis zählt als Lücke — das ist Weg a, \
             nicht die Festlegung für #892."
        );
        assert!(!knoten.signature_ohne_semantik());
        assert!(
            !alle_konservativ(&knoten),
            "{op:?}: Mit unbekanntem Bereich und Hinweis fällt jede Analyse zurück — dann wäre \
             die Unbekannt-Deklaration eine Lücke unter anderem Namen."
        );
    }
}

#[test]
fn die_echten_signaturen_sind_deklariert_und_fallen_nicht_im_ganzen_zurueck() {
    let deklariert: Vec<&EffectNode> = EFFECT_NODES
        .iter()
        .filter(|n| matches!(n.op, EffectOp::CustomSignature(_)) && !n.hat_luecke())
        .collect();
    assert!(
        !deklariert.is_empty(),
        "Keine einzige deklarierte Signatur in den Daten — dann prüft dieser Fall nichts."
    );
    let mut ohne_angabe: Vec<&str> = deklariert
        .iter()
        .filter(|n| {
            n.numeric_bounds == Bounds::UNKNOWN
                || n.monotonicity.direction == MonotonicDirection::Unknown
        })
        .filter_map(|n| match n.op {
            EffectOp::CustomSignature(id) => Some(id.0),
            _ => None,
        })
        .collect();
    ohne_angabe.sort_unstable();
    ohne_angabe.dedup();
    // Die Zahl steht in der MELDUNG, nicht in der Bedingung: Sie darf wachsen.
    eprintln!(
        "Deklarierte Signaturknoten: {}. Signaturarten ohne Bereich oder Monotonie-Richtung — \
         unter Weg a fielen sie im Ganzen zurück: {} {ohne_angabe:?}",
        deklariert.len(),
        ohne_angabe.len()
    );
    for knoten in &deklariert {
        assert!(
            !alle_konservativ(knoten),
            "{}: eine deklarierte Bossregel fällt in jeder Analyse zurück. Unter Weg b soll \
             ihre Deklaration dort wirken, wo sie gelesen wird.",
            knoten.origin
        );
    }
}

#[test]
fn ohne_wertebereich_leitet_action_bound_fuer_zusatzaktionen_keine_obergrenze_ab() {
    // Der Zweig für Bausteine, die Zusatz-Aktionen erzeugen: Die Schranke kommt
    // aus `stacking` oder aus dem Wertebereich. Mit `Refresh` bleibt nur der
    // Bereich — genau dort muss „unbekannt“ zur fehlenden Obergrenze werden.
    for op in [
        EffectOp::ReduceCooldown,
        EffectOp::ResetCooldown,
        EffectOp::FreeNextGcd,
    ] {
        let mut eng = beispielknoten(op);
        eng.stacking = StackingSemantics::Refresh;
        eng.numeric_bounds = Bounds::exact(0, 3);
        let bekannt = ergebnis("action-bound", &eng);
        assert_eq!(
            (bekannt.verdict, bekannt.evidence),
            (Verdict::Permits, Evidence::Schranke(Bounds::exact(0, 3))),
            "{op:?}: Der Gegenpol trägt nicht — mit engem Bereich muss die Schranke ankommen."
        );
        for offen in [
            Bounds::UNKNOWN,
            Bounds {
                min: Some(0),
                max: None,
            },
            Bounds {
                min: None,
                max: Some(3),
            },
        ] {
            let mut knoten = eng;
            knoten.numeric_bounds = offen;
            let r = ergebnis("action-bound", &knoten);
            assert_eq!(
                (r.verdict, r.evidence),
                (Verdict::Conservative, Evidence::Aktionsgrenze(None)),
                "{op:?} mit {offen:?}: Ohne bekannten Bereich hat \"action-bound\" eine \
                 Obergrenze abgeleitet ({}).",
                r.reason
            );
            assert!(r.fallback.vollstaendig());
        }
    }
}

#[test]
fn beim_wirken_reicht_action_bound_einen_unbekannten_bereich_unveraendert_weiter() {
    // Hier kommt die Obergrenze aus dem Auslöser (beim Wirken: so oft wie
    // gewirkt), nicht aus dem Bereich — das Urteil hängt deshalb nicht an ihm.
    // Die Angabe, die mitgeht, darf aber keinen Bereich erfinden.
    for op in [
        EffectOp::DealDamage,
        EffectOp::ApplyDot,
        EffectOp::ApplyBuff,
        EffectOp::ModifyResource,
        EffectOp::Heal,
        EffectOp::Shield,
        EffectOp::ConvertDamage,
    ] {
        let bekannt = ergebnis("action-bound", &beispielknoten(op));
        let r = ergebnis("action-bound", &mit_unbekanntem_bereich(op));
        assert_eq!(
            bekannt.evidence,
            Evidence::Schranke(beispielknoten(op).numeric_bounds),
            "{op:?}: Gegenpol — der bekannte Bereich kommt nicht an."
        );
        assert_eq!(
            r.evidence,
            Evidence::Schranke(Bounds::UNKNOWN),
            "{op:?}: Aus einem unbekannten Bereich ist eine Schranke geworden."
        );
        assert_eq!(
            r.verdict, bekannt.verdict,
            "{op:?}: Das Urteil liest hier den Bereich."
        );
    }
}

#[test]
fn die_kernelwahl_reicht_einen_unbekannten_bereich_unveraendert_weiter() {
    // Ist-Zustand: Die Kernelwahl urteilt über Zeitverhalten und Baustein, nicht
    // über den Bereich; sie gibt ihn als Angabe weiter. Wer einen Kernel mit
    // fester Darstellung daraus baut, muss `ist_eng()` selbst fragen — die
    // Angabe sagt ihm dann „nicht eng“.
    for op in [
        EffectOp::DealDamage,
        EffectOp::ModifyResource,
        EffectOp::Heal,
    ] {
        let bekannt = ergebnis("kernel-choice", &beispielknoten(op));
        let r = ergebnis("kernel-choice", &mit_unbekanntem_bereich(op));
        assert_eq!(
            bekannt.evidence,
            Evidence::Schranke(beispielknoten(op).numeric_bounds),
            "{op:?}: Gegenpol — der bekannte Bereich kommt nicht an."
        );
        assert_eq!(
            r.evidence,
            Evidence::Schranke(Bounds::UNKNOWN),
            "{op:?}: Aus einem unbekannten Bereich ist eine Schranke geworden."
        );
        assert!(!matches!(r.evidence, Evidence::Schranke(b) if b.ist_eng()));
        assert_eq!(r.verdict, bekannt.verdict);
    }
}

#[test]
fn ohne_monotonie_hinweis_faellt_die_monotonie_analyse_zurueck() {
    // Je eine der beiden Hälften fehlt, und beide zusammen: Jede reicht.
    let ohne = [
        LocalMonotonicityHint::UNKNOWN,
        LocalMonotonicityHint {
            direction: MonotonicDirection::Unknown,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
        },
        LocalMonotonicityHint {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::Unspecified,
        },
    ];
    for op in [
        EffectOp::DealDamage,
        EffectOp::ApplyDot,
        EffectOp::ModifyResource,
        EffectOp::ReduceCooldown,
        EffectOp::ResetCooldown,
    ] {
        let mit = beispielknoten(op);
        assert_eq!(
            ergebnis("monotonicity", &mit).verdict,
            Verdict::Permits,
            "{op:?}: Gegenpol — mit passendem Hinweis muss die Analyse erlauben, sonst wäre der \
             Rückfall unten aus dem falschen Grund grün."
        );
        for hinweis in ohne {
            let mut knoten = mit;
            knoten.monotonicity = hinweis;
            assert!(!knoten.hat_luecke());
            let r = ergebnis("monotonicity", &knoten);
            assert_eq!(
                r.verdict,
                Verdict::Conservative,
                "{op:?} mit {hinweis:?}: Ohne vollständigen Hinweis hat die Monotonie-Analyse \
                 erlaubt ({}).",
                r.reason
            );
            assert!(r.fallback.vollstaendig());
        }
    }
}

#[test]
fn ein_hinweis_ohne_richtung_oder_bezug_traegt_fuer_keinen_bezug() {
    // Alle Bezüge, auch „ohne Bezug“ selbst: Ein Hinweis ohne Bezug darf nicht
    // dadurch tragen, dass jemand nach „ohne Bezug“ fragt.
    for gefragt in [
        MonotonicityDomain::ExpectedDamageStableTrace,
        MonotonicityDomain::VictoryOutcome,
        MonotonicityDomain::TimeToVictoryAcrossPhase,
        MonotonicityDomain::HpPredicate,
        MonotonicityDomain::SpellDiversity,
        MonotonicityDomain::Unspecified,
    ] {
        let ohne_bezug = LocalMonotonicityHint {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::Unspecified,
        };
        let ohne_richtung = LocalMonotonicityHint {
            direction: MonotonicDirection::Unknown,
            domain: gefragt,
        };
        assert!(
            !ohne_bezug.traegt_fuer(gefragt),
            "ohne Bezug trägt für {gefragt:?}"
        );
        assert!(
            !ohne_richtung.traegt_fuer(gefragt),
            "ohne Richtung trägt für {gefragt:?}"
        );
        assert!(!LocalMonotonicityHint::UNKNOWN.traegt_fuer(gefragt));
    }
}

#[test]
fn ohne_richtung_oder_bezug_gibt_es_kein_zertifikat_auch_mit_allen_zutaten() {
    let knoten = beispielknoten(EffectOp::DealDamage);
    let stabil = MonotonicityDomain::ExpectedDamageStableTrace;
    let ohne_bezug = LocalMonotonicityHint {
        direction: MonotonicDirection::Increasing,
        domain: MonotonicityDomain::Unspecified,
    };
    // **Der Grund der Weigerung zählt, nicht nur die Weigerung.** `zertifikat`
    // prüft die Richtung selbst und fragt danach `traegt_fuer`, das ohne Richtung
    // ebenfalls ablehnt. Mit nur `is_err()` blieb das Streichen der eigenen
    // Richtungsprüfung grün: Der Grund wechselte still von `KeinHinweis` zu
    // `FalscherBezug` (gemessen bei der Prüfung von PR #1136).
    for hinweis in [
        LocalMonotonicityHint::UNKNOWN,
        LocalMonotonicityHint {
            direction: MonotonicDirection::Unknown,
            domain: stabil,
        },
    ] {
        assert_eq!(
            zertifikat(hinweis, stabil, ZertifikatsBedarf::VOLLSTAENDIG, &knoten),
            Err(ZertifikatVerweigert::KeinHinweis),
            "{hinweis:?}: Ohne Richtung muss das Zertifikat am fehlenden Hinweis scheitern."
        );
    }
    for gefragt in [stabil, MonotonicityDomain::Unspecified] {
        assert_eq!(
            zertifikat(
                ohne_bezug,
                gefragt,
                ZertifikatsBedarf::VOLLSTAENDIG,
                &knoten
            ),
            Err(ZertifikatVerweigert::FalscherBezug {
                hinweis: MonotonicityDomain::Unspecified,
                gefragt,
            }),
            "Ohne Bezug, gefragt {gefragt:?}: Das Zertifikat muss am Bezug scheitern."
        );
    }
    // Der Gegenpol: Mit vollständigem Hinweis und allen Zutaten gibt es eins.
    let voll = LocalMonotonicityHint {
        direction: MonotonicDirection::Increasing,
        domain: stabil,
    };
    assert_eq!(
        zertifikat(voll, stabil, ZertifikatsBedarf::VOLLSTAENDIG, &knoten),
        Ok(())
    );
}

#[test]
fn ein_knoten_ohne_wertebereich_laesst_den_numerischen_vertrag_nicht_tragen() {
    let mut eng = beispielknoten(EffectOp::DealDamage);
    eng.numeric_bounds = Bounds::exact(0, 10);
    let offen = mit_unbekanntem_bereich(EffectOp::Heal);
    assert!(
        NumerischerVertrag::aus_knoten(&[eng]).traegt(),
        "Gegenpol — ein enger Bereich allein muss tragen."
    );
    // Die Reihenfolge darf am Ergebnis nichts ändern.
    for knoten in [[eng, offen], [offen, eng]] {
        let vertrag = NumerischerVertrag::aus_knoten(&knoten);
        assert!(
            !vertrag.traegt(),
            "Ein unbekannter Bereich hat die Hülle nicht geöffnet."
        );
        assert_eq!(vertrag.huelle, Bounds::UNKNOWN);
        assert_eq!(
            (
                vertrag.knoten,
                vertrag.mit_enger_schranke,
                vertrag.ohne_schranke
            ),
            (2, 1, 1)
        );
    }
}

/// Die Analysen, deren Unbekannt-Verhalten oben je eigens geprüft ist.
///
/// Eine Abschrift — aber eine, die bemerkt, dass sie veraltet: Der Fall darunter
/// misst die Leser und verlangt Gleichheit in BEIDE Richtungen.
const GEPRUEFTE_LESER_WERTEBEREICH: &[&str] = &["action-bound", "kernel-choice"];
const GEPRUEFTE_LESER_MONOTONIE: &[&str] = &["monotonicity"];

#[test]
fn jeder_gemessene_leser_von_wertebereich_und_monotonie_ist_hier_geprueft() {
    let mut leser_bereich: Vec<&str> = Vec::new();
    let mut leser_monotonie: Vec<&str> = Vec::new();
    let mut verstoesse: Vec<String> = Vec::new();
    let mut paare = 0usize;
    for op in bausteine() {
        // `Refresh` daneben, weil ein Zweig von "action-bound" den Bereich nur
        // dann liest, wenn `stacking` keine eigene Schranke liefert.
        for stapel in [StackingSemantics::None, StackingSemantics::Refresh] {
            let mut basis = beispielknoten(op);
            basis.stacking = stapel;
            assert!(!basis.hat_luecke());
            let mut offen = basis;
            offen.numeric_bounds = Bounds::UNKNOWN;
            let mut ohne = basis;
            ohne.monotonicity = LocalMonotonicityHint::UNKNOWN;
            let bekannt = alle_analysen(&basis);
            for (unbekannt, leser, bereich) in [
                (alle_analysen(&offen), &mut leser_bereich, true),
                (alle_analysen(&ohne), &mut leser_monotonie, false),
            ] {
                paare += 1;
                for (b, u) in bekannt.iter().zip(&unbekannt) {
                    if b == u {
                        continue;
                    }
                    if !leser.contains(&b.pass.0) {
                        leser.push(b.pass.0);
                    }
                    // Unbekannt darf nicht schneller sein als bekannt.
                    if b.verdict != Verdict::Permits && u.verdict == Verdict::Permits {
                        verstoesse.push(format!("{} bei {op:?}: erlaubt erst ohne Angabe", u.pass));
                    }
                    if bereich && matches!(u.evidence, Evidence::Schranke(s) if s.ist_eng()) {
                        verstoesse.push(format!("{} bei {op:?}: erfindet eine Schranke", u.pass));
                    }
                    if !bereich && u.verdict == Verdict::Permits {
                        verstoesse.push(format!("{} bei {op:?}: erlaubt ohne Hinweis", u.pass));
                    }
                }
            }
        }
    }
    assert!(paare > 0 && !REGISTRY.is_empty(), "nichts verglichen");
    leser_bereich.sort_unstable();
    leser_monotonie.sort_unstable();
    assert!(
        verstoesse.is_empty(),
        "Unbekannt führt an diesen Lesern nicht zum konservativen Ausgang: {verstoesse:?}"
    );
    assert_eq!(
        leser_bereich, GEPRUEFTE_LESER_WERTEBEREICH,
        "Die gemessenen Leser des Wertebereichs weichen von den hier geprüften ab. Ein neuer \
         Leser braucht seinen eigenen Fall oben; ein weggefallener gehört aus der Liste."
    );
    assert_eq!(
        leser_monotonie, GEPRUEFTE_LESER_MONOTONIE,
        "Die gemessenen Leser des Monotonie-Hinweises weichen von den hier geprüften ab."
    );
}
