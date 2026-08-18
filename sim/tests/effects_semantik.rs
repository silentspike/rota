//! Zufallsquellen, Monotonie-Grenzen und Wertebereiche (Issue #966, AC3–AC5).
//!
//! Die drei Zusicherungen, die ueber den blossen Knotentyp hinausgehen:
//! dass die Zufalls-Semantik eine echte Quellenmaske speist, dass der
//! Monotonie-Hinweis seine Beschraenkung traegt, und dass die Wertebereiche
//! dem numerischen Vertrag als Eingabe zur Verfuegung stehen.

use rota_sim::data::EFFECT_NODES;
use rota_sim::effects::passes::beispielknoten;
use rota_sim::effects::{
    zertifikat, Bounds, EffectNode, EffectOp, LocalMonotonicityHint, MonotonicDirection,
    MonotonicityDomain, NumerischerVertrag, RandomSourceMask, RandomnessSemantics,
    ZertifikatVerweigert, ZertifikatsBedarf,
};
use rota_sim::rng::Stream;

// ------------------------------------------------------------------- AC3

#[test]
fn die_zufalls_semantik_speist_die_quellenmaske() {
    assert!(RandomSourceMask::KEINE.ist_leer());
    assert!(RandomSourceMask::KEINE.ein_durchgang_genuegt());

    let krit = RandomnessSemantics::CritRoll.quellen();
    assert!(krit.enthaelt(Stream::Crit));
    assert!(!krit.enthaelt(Stream::DotTick));
    assert!(
        !krit.ein_durchgang_genuegt(),
        "Mit aktivem Krit-Strom darf ein Lauf NICHT auf einen Durchgang reduziert werden."
    );

    let tick = RandomnessSemantics::Stream(Stream::DotTick).quellen();
    assert!(tick.enthaelt(Stream::DotTick));
    assert!(!tick.enthaelt(Stream::Crit));

    assert!(
        RandomnessSemantics::Deterministic.quellen().ist_leer(),
        "Ein deterministischer Effekt darf keine Quelle beisteuern."
    );
}

#[test]
fn unbekannte_zufalls_semantik_nennt_alle_quellen() {
    // Unbekannt muss langsam sein, nie falsch: Wer nichts angibt, darf nicht
    // weniger Quellen nennen als wer etwas angibt.
    let unbekannt = RandomnessSemantics::Unknown.quellen();
    for strom in [Stream::Crit, Stream::DotTick, Stream::Heal] {
        assert!(
            unbekannt.enthaelt(strom),
            "Der Strom {strom:?} fehlt in der Maske fuer eine unbekannte Zufalls-Semantik."
        );
    }
    assert!(!unbekannt.ein_durchgang_genuegt());
    assert_eq!(
        unbekannt.anzahl(),
        rota_sim::rng::STREAM_COUNT as u32,
        "ALLE muss genau so viele Stroeme nennen, wie es gibt — die Zahl ist aus \
         STREAM_COUNT abgeleitet und darf nicht davon abweichen."
    );
}

#[test]
fn die_maske_der_echten_beschreibung_nennt_die_stroeme_die_vorkommen() {
    let maske = RandomSourceMask::aus_knoten(EFFECT_NODES);
    assert!(
        !EFFECT_NODES.is_empty(),
        "Ohne Knoten prueft dieser Fall nichts."
    );
    // Gemessen an den Daten: Schaden und Braende ziehen, Heilung nicht —
    // `Stream::Heal` ist laut `rng.rs` bewusst reserviert, weil alle
    // Koagulations-Wege deterministisch sind (Design 21.1).
    assert!(
        maske.enthaelt(Stream::Crit),
        "Kein Knoten nennt den Krit-Strom, dabei tragen Schadenszauber CritRoll."
    );
    assert!(
        maske.enthaelt(Stream::DotTick),
        "Kein Knoten nennt den Brand-Strom, dabei ziehen Brand-Ticks daraus."
    );
    assert!(
        !maske.enthaelt(Stream::Heal),
        "Ein Knoten nennt den Heil-Strom — im Kampf zieht daraus nichts, und die \
         Beschreibung soll den gemessenen Zustand tragen, nicht den erwarteten."
    );
    assert!(
        !maske.ein_durchgang_genuegt(),
        "Die echte Beschreibung nennt aktive Quellen; ein Durchgang genuegt hier nicht."
    );
}

#[test]
fn eine_beschreibung_ganz_ohne_zufall_erlaubt_einen_durchgang() {
    // Der Gegenpol: Ohne ihn koennte `ein_durchgang_genuegt` konstant `false`
    // liefern und alle Faelle oben waeren trotzdem gruen.
    let mut ohne = beispielknoten(EffectOp::DealDamage);
    ohne.randomness = RandomnessSemantics::Deterministic;
    let maske = RandomSourceMask::aus_knoten(&[ohne]);
    assert!(maske.ist_leer());
    assert!(
        maske.ein_durchgang_genuegt(),
        "Eine Beschreibung ohne jede Zufallsquelle MUSS auf einen Durchgang reduzierbar sein — \
         sonst ist die Maske ohne Nutzen."
    );
}

// ------------------------------------------------------------------- AC4

/// Ein Hinweis, der „monoton" sagt und fuer den gefragten Bezug gilt.
fn monotoner_hinweis() -> LocalMonotonicityHint {
    LocalMonotonicityHint {
        direction: MonotonicDirection::Increasing,
        domain: MonotonicityDomain::ExpectedDamageStableTrace,
    }
}

#[test]
fn der_hinweis_sagt_monoton_und_das_zertifikat_wird_trotzdem_verweigert() {
    // # Die Kontrollprobe aus der Verification strategy
    //
    // „Ein Fall, in dem der Hinweis 'monoton' sagt und das Zertifikat trotzdem
    // verweigert wird, gehoert dazu — sonst wird der Hinweis fuer ein
    // Zertifikat gehalten."
    //
    // Hier stimmt ALLES, was die Wirkungsbeschreibung liefern kann: Richtung
    // gesetzt, Bezug passend, kein Kontrollfluss-Zweig. Und trotzdem gibt es
    // kein Zertifikat, weil drei der vier Zutaten fehlen.
    let knoten = beispielknoten(EffectOp::DealDamage);
    let hinweis = monotoner_hinweis();
    assert!(
        hinweis.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
        "Der Hinweis muss fuer den gefragten Bezug tragen — sonst prueft dieser Fall den \
         falschen Grund."
    );
    assert!(
        !knoten.control_flow_effect.verzweigt(),
        "Der Knoten darf nicht verzweigen — sonst prueft dieser Fall den falschen Grund."
    );

    let ergebnis = zertifikat(
        hinweis,
        MonotonicityDomain::ExpectedDamageStableTrace,
        ZertifikatsBedarf::AUS_HINWEIS_ALLEIN,
        &knoten,
    );
    assert_eq!(
        ergebnis,
        Err(ZertifikatVerweigert::ZutatenFehlen),
        "Aus dem Hinweis ALLEIN darf nie ein Zertifikat folgen. Genau diese Verwechslung ist \
         das Non-Goal von Issue #966."
    );
    assert_eq!(
        ZertifikatsBedarf::AUS_HINWEIS_ALLEIN.fehlend(),
        vec!["Bosssemantik", "Zielfunktion", "Trace-Region"],
        "Die fehlenden Zutaten muessen benennbar sein, sonst ist die Verweigerung nicht \
         nachvollziehbar."
    );
}

#[test]
fn mit_allen_vier_zutaten_traegt_das_zertifikat() {
    // Der Gegenpol. Ohne ihn koennte `zertifikat` konstant verweigern, und der
    // Fall darueber waere aus dem falschen Grund gruen.
    let knoten = beispielknoten(EffectOp::DealDamage);
    let ergebnis = zertifikat(
        monotoner_hinweis(),
        MonotonicityDomain::ExpectedDamageStableTrace,
        ZertifikatsBedarf::VOLLSTAENDIG,
        &knoten,
    );
    assert_eq!(
        ergebnis,
        Ok(()),
        "Mit allen vier Zutaten und passendem Hinweis muss ein Zertifikat moeglich sein — \
         sonst verweigert die Funktion immer und sagt nichts aus."
    );
}

#[test]
fn der_hinweis_traegt_fuer_keine_der_drei_gegenbeispiel_zielgroessen() {
    // Die drei Zielgroessen, die AC4 ausdruecklich nennt.
    let knoten = beispielknoten(EffectOp::DealDamage);
    for gefragt in [
        MonotonicityDomain::VictoryOutcome,
        MonotonicityDomain::TimeToVictoryAcrossPhase,
        MonotonicityDomain::HpPredicate,
    ] {
        let ergebnis = zertifikat(
            monotoner_hinweis(),
            gefragt,
            // **Sogar mit allen vier Zutaten** — der Bezug allein reicht schon
            // als Grund. Waere hier AUS_HINWEIS_ALLEIN gesetzt, koennte der
            // Fall aus dem falschen Grund gruen sein.
            ZertifikatsBedarf::VOLLSTAENDIG,
            &knoten,
        );
        assert_eq!(
            ergebnis,
            Err(ZertifikatVerweigert::FalscherBezug {
                hinweis: MonotonicityDomain::ExpectedDamageStableTrace,
                gefragt,
            }),
            "Ein Hinweis fuer den stabilen Trace hat fuer {gefragt:?} getragen. Grundschaden \
             ist dort NICHT monoton: Beim Sieg mit umgekehrter Bedingung kehrt sich der \
             Zusammenhang um, ueber einen Phasenwechsel ist er stueckweise, und an einem \
             Lebenspunkt-Praedikat entscheidet die Seite der Schwelle statt der Betrag."
        );
    }
}

#[test]
fn ohne_richtung_gibt_es_keinen_grund_ueber_zutaten_zu_reden() {
    let knoten = beispielknoten(EffectOp::DealDamage);
    let ergebnis = zertifikat(
        LocalMonotonicityHint::UNKNOWN,
        MonotonicityDomain::ExpectedDamageStableTrace,
        ZertifikatsBedarf::VOLLSTAENDIG,
        &knoten,
    );
    assert_eq!(ergebnis, Err(ZertifikatVerweigert::KeinHinweis));
}

#[test]
fn ein_verzweigender_knoten_bekommt_kein_zertifikat() {
    let mut knoten = beispielknoten(EffectOp::DealDamage);
    knoten.control_flow_effect.branches_on_victory = true;
    let ergebnis = zertifikat(
        monotoner_hinweis(),
        MonotonicityDomain::ExpectedDamageStableTrace,
        ZertifikatsBedarf::VOLLSTAENDIG,
        &knoten,
    );
    assert_eq!(ergebnis, Err(ZertifikatVerweigert::KontrollflussVerzweigt));
}

// ------------------------------------------------------------------- AC5

#[test]
fn die_wertebereiche_stehen_dem_numerischen_vertrag_zur_verfuegung() {
    let vertrag = NumerischerVertrag::aus_knoten(EFFECT_NODES);
    assert_eq!(
        vertrag.knoten,
        EFFECT_NODES.len(),
        "Der Vertrag muss ueber ALLE Knoten gehen, nicht ueber eine Auswahl."
    );
    assert!(
        vertrag.mit_enger_schranke > 0,
        "Kein einziger der {} Knoten traegt eine enge Schranke — dann ist der Wertebereich \
         nirgends erfasst und AC5 nicht erfuellt.",
        vertrag.knoten
    );
    // Die offene Huelle ist der erwartete Zustand und kein Mangel:
    // Verstaerkungen heben den Grundwert nach oben, eine Obergrenze waere hier
    // eine Behauptung. Der Fall haelt fest, dass das SICHTBAR ist.
    assert!(
        vertrag.ohne_schranke > 0 || vertrag.traegt(),
        "Entweder traegt die Huelle, oder es gibt Knoten ohne Schranke — beides zugleich \
         nicht zu haben waere ein Widerspruch in der Zaehlung."
    );
}

#[test]
fn eine_unbekannte_grenze_steckt_die_huelle_an() {
    // Sonst waere die Huelle enger als einer ihrer Teile, und eine erfundene
    // Schranke ist schlimmer als keine.
    let eng = Bounds::exact(0, 100);
    let offen = Bounds {
        min: Some(0),
        max: None,
    };
    assert_eq!(eng.huelle(offen).max, None);
    assert_eq!(eng.huelle(offen).min, Some(0));
    assert_eq!(eng.huelle(Bounds::UNKNOWN), Bounds::UNKNOWN);
    assert!(!eng.huelle(offen).ist_eng());
    assert!(eng.huelle(Bounds::exact(-5, 50)).ist_eng());
    assert_eq!(eng.huelle(Bounds::exact(-5, 50)), Bounds::exact(-5, 100));
}

#[test]
fn ein_vertrag_aus_lauter_engen_schranken_traegt() {
    // Der Gegenpol zu oben: Ohne ihn koennte `traegt` konstant `false` sein.
    let mut a = beispielknoten(EffectOp::DealDamage);
    a.numeric_bounds = Bounds::exact(0, 10);
    let mut b = beispielknoten(EffectOp::Heal);
    b.numeric_bounds = Bounds::exact(5, 30);
    let vertrag = NumerischerVertrag::aus_knoten(&[a, b]);
    assert!(vertrag.traegt());
    assert_eq!(vertrag.huelle, Bounds::exact(0, 30));
    assert_eq!(vertrag.mit_enger_schranke, 2);
    assert_eq!(vertrag.ohne_schranke, 0);
}

#[test]
fn ein_leerer_vertrag_traegt_nicht() {
    let leer: [EffectNode; 0] = [];
    let vertrag = NumerischerVertrag::aus_knoten(&leer);
    assert_eq!(vertrag.knoten, 0);
    assert!(
        !vertrag.traegt(),
        "Ein Vertrag ohne Knoten darf nicht tragen — sonst waere 'nichts gemessen' dasselbe \
         wie 'alles bekannt'."
    );
}
