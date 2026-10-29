//! Der analytische Krit an Durchgangswahl und Rotationssuche (#920, E11.4d).
//!
//! # Was hier zugesichert wird
//!
//! | Kriterium | Prüffälle |
//! | --- | --- |
//! | AC2 — die Maske ist abgeleitet, je Aufbau × Boss, mit der Bossseite | `ac2_*` |
//! | AC3 — die Prüfung ist zweistellig | `ac3_*` |
//! | AC4 — die Antwort ist eine Aufteilung je Quelle | `ac4_*` |
//! | Boss mit Phasen bleibt bei der Stichprobe, first-crucible geht analytisch | `ac2_lebenspunkt_phasen_*`, `an_first_crucible_*` |
//! | die Suche nimmt den Weg | `die_suche_*` |
//! | die Ableitung wird nachgeprüft | `der_randabgleich_*`, `an_jedem_unberuehrten_*` |
//!
//! Gegen den Vorgabe-Boss `first-crucible` würfelt nichts außer dem Krit, und er
//! hat kein einziges Zusatzsystem — der Boss, an dem der Weg greifen muss.
//! `boss-1-2` trägt Lebenspunkt-Phasen — der Boss, an dem er nicht greifen darf.

use rota_sim::analytisch::{
    beruehrt_den_kontrollfluss, erwartung, kann_ausintegrieren, ohne_ausintegration, raender,
    randabgleich, verweigerungen, Abhaengigkeitsmaske, Begruendung, Ergebnisgroesse, Fundstelle,
    StochastischeAufteilung, Verweigerung,
};
use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run_erwartet, run_gestaffelt, BalanceConfig, Durchgangswahl, Staffelung};
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::effects::passes::beispielknoten;
use rota_sim::effects::{
    ControlFlowEffect, EffectOp, RandomSourceMask, RandomnessSemantics, SignatureId, StateMask,
};
use rota_sim::fortschritt::{Weg, Zaehlwerk};
use rota_sim::gear::WornValues;
use rota_sim::rng::Stream;
use rota_sim::rotation_search::{candidates_for, greedy};
use rota_sim::scoring::ScoringSet;

const GOOD: &str = include_str!("../apl/step1-good.apl");

fn gute_liste() -> Apl {
    parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
}

fn liste(text: &str) -> Apl {
    parse(AplName("pruefliste"), text).expect("die Prüfliste ist gültig")
}

/// Ein Lauf wie in der Suche: volle Kampfdauer, fünfzig Durchgänge.
fn gegen(boss: &str, purity: u32) -> BalanceConfig {
    let mut worn = WornValues::none();
    worn.purity = purity;
    BalanceConfig {
        iterations: 50,
        base_seed: 17,
        threads: 1,
        stop_on_kill: false,
        worn,
        boss: boss_by_id(boss).expect("den Boss gibt es"),
        ..BalanceConfig::default()
    }
}

fn maske(boss: &str, purity: u32, apl: &Apl) -> Abhaengigkeitsmaske {
    Abhaengigkeitsmaske::ableiten(&gegen(boss, purity), apl)
}

fn quellen(liste: &[Stream]) -> RandomSourceMask {
    liste.iter().fold(RandomSourceMask::KEINE, |m, s| {
        m.union(RandomSourceMask::einer(*s))
    })
}

// ---------------------------------------------------------------- AC2

/// **Die Bossseite.** Ein Phasenknoten würfelt nicht — er liest den
/// Fortschritt am Boss, und der hängt am Krit. Die knotenlokale Prüfung aus
/// #908 sah das nicht.
#[test]
fn ac2_lebenspunkt_phasen_beruehren_den_krit() {
    let apl = gute_liste();
    let config = gegen("boss-1-2", 250);
    assert!(
        !config.boss.phases.is_empty(),
        "Trägerzusicherung: boss-1-2 trägt Lebenspunkt-Phasen"
    );
    let maske = Abhaengigkeitsmaske::ableiten(&config, &apl);
    let krit = maske.quelle(Stream::Crit);
    assert!(
        krit.zieht,
        "Trägerzusicherung: bei PURITY 250 zieht der Krit"
    );
    assert!(
        krit.verzweigt_an
            .contains(&Fundstelle::Knoten("boss-1-2/phases")),
        "die Phasen von boss-1-2 lesen den Fortschritt und verzweigen daran — die \
         Maske muss sie nennen, gefunden: {:?}",
        krit.verzweigt_an
    );
    assert_eq!(
        Durchgangswahl::fuer(&config, &apl, Ergebnisgroesse::ErwarteterSchaden),
        Durchgangswahl::Gestaffelt,
        "an einem Boss mit Lebenspunkt-Phasen bleibt es bei der Stichprobe"
    );
    let erwartet = run_erwartet(&config, &apl);
    assert_eq!(erwartet.weg, Durchgangswahl::Gestaffelt);
    assert!(
        erwartet.stderr > 0.0,
        "die Stichprobe streut ({}) — sonst wäre nicht gewürfelt worden",
        erwartet.stderr
    );
}

#[test]
fn ac2_ohne_bosssystem_bleibt_der_krit_unberuehrt() {
    let maske = maske("first-crucible", 250, &gute_liste());
    let krit = maske.quelle(Stream::Crit);
    assert!(
        krit.zieht,
        "Trägerzusicherung: bei PURITY 250 zieht der Krit"
    );
    assert!(
        krit.veraendert.intersects(StateMask::BOSS_HP),
        "Trägerzusicherung: die Maske sieht, dass der Krit den Fortschritt verändert \
         ({:?}) — sonst hieße „unberührt“ nur, dass sie nichts gesehen hat",
        krit.veraendert
    );
    assert!(
        krit.verzweigt_an.is_empty(),
        "first-crucible hat kein Zusatzsystem und keine Phase; gefunden: {:?}",
        krit.verzweigt_an
    );
}

#[test]
fn ac2_abbruch_beim_sieg_beruehrt() {
    let config = BalanceConfig {
        stop_on_kill: true,
        ..gegen("first-crucible", 250)
    };
    let maske = Abhaengigkeitsmaske::ableiten(&config, &gute_liste());
    assert!(
        maske
            .quelle(Stream::Crit)
            .verzweigt_an
            .contains(&Fundstelle::AbbruchBeimSieg),
        "mit Abbruch beim Sieg endet der Kampf früher, wenn mehr kritet: {:?}",
        maske.quelle(Stream::Crit).verzweigt_an
    );
}

/// Eine Zeile, die den Fortschritt liest, verzweigt am Krit — eine, die die
/// Restlaufzeit eines Brandes liest, nicht.
#[test]
fn ac2_eine_zeile_die_den_fortschritt_liest_beruehrt() {
    let mit_schwelle = liste(
        "smoulder if dot.smoulder.remains < gcd\n\
         pyre if boss.hp_pct < 30\n\
         kindle\n",
    );
    let krit = maske("first-crucible", 250, &mit_schwelle);
    assert!(
        krit.quelle(Stream::Crit)
            .verzweigt_an
            .contains(&Fundstelle::Zeile {
                zeile: 2,
                zauber: "pyre"
            }),
        "`pyre if boss.hp_pct < 30` liest den Fortschritt: {:?}",
        krit.quelle(Stream::Crit).verzweigt_an
    );
    // Die Gegenprobe ohne die Schwelle: dieselben Zauber, Bedingungen auf
    // Brand-Laufzeit und Hitze. Beide liest der Krit nicht.
    let ohne = maske("first-crucible", 250, &gute_liste());
    assert!(
        ohne.quelle(Stream::Crit).verzweigt_an.is_empty(),
        "step1-good liest Brand und Hitze, nicht den Fortschritt: {:?}",
        ohne.quelle(Stream::Crit).verzweigt_an
    );
}

/// **Gegen die Daten, nicht gegen eine Liste.** Welcher wertungsfähige Boss
/// den Krit berührt, folgt aus seinen Feldern: Phasen lesen den Fortschritt
/// (`Fight::check_phase`), ein Rennen den Betrag im Fenster (`Fight::deal`),
/// ein davonlaufender Ring den Sieg (`Fight::runaway_tick`), die Boss-Heilung
/// den Fortschritt, den sie zurückdreht (`Fight::undo_progress`). Die übrigen
/// Systeme lesen ihn nicht.
#[test]
fn ac2_bosssysteme_beruehren_nur_wo_sie_den_schaden_lesen() {
    let apl = gute_liste();
    let mut falsch = Vec::new();
    let (mut beruehrt, mut frei) = (0, 0);
    for boss in ScoringSet::all_scored().bosses() {
        let erwartet = !boss.phases.is_empty()
            || boss.race.is_some()
            || boss.runaway.is_some()
            || boss.boss_heal.is_some();
        let maske = maske(boss.id, 250, &apl);
        let krit = maske.quelle(Stream::Crit);
        if krit.beruehrt_den_kontrollfluss() {
            beruehrt += 1;
        } else {
            frei += 1;
        }
        if krit.beruehrt_den_kontrollfluss() != erwartet {
            falsch.push(format!(
                "{}: erwartet {}, abgeleitet {:?}",
                boss.id,
                if erwartet { "berührt" } else { "frei" },
                krit.verzweigt_an
            ));
        }
    }
    println!("wertungsfähig: {beruehrt} berührt, {frei} frei");
    assert!(
        beruehrt > 0 && frei > 0,
        "Trägerzusicherung: beide Fälle kommen vor ({beruehrt} berührt, {frei} frei)"
    );
    assert!(
        falsch.is_empty(),
        "{} Bosse falsch eingeordnet: {falsch:#?}",
        falsch.len()
    );
}

/// **Abgeleitet, nicht benannt:** Ein erfundener Knoten, den kein Boss und
/// keine Liste kennt, kippt die Maske allein über seine Eigenschaften — und
/// über einen Umweg: Ein Wandler liest den Fortschritt und schreibt Schutz,
/// und erst der Leser des Schutzes verzweigt.
#[test]
fn ac2_ein_erfundener_leser_kippt_die_maske_ueber_seine_eigenschaften() {
    let mut worn = WornValues::none();
    worn.purity = 250;

    let mut schaden = beispielknoten(EffectOp::DealDamage);
    schaden.randomness = RandomnessSemantics::CritRoll;
    schaden.writes = StateMask::BOSS_HP;

    let verzweigt = ControlFlowEffect {
        branches_on_hp: true,
        ..ControlFlowEffect::NONE
    };
    let mut schwelle = beispielknoten(EffectOp::CustomSignature(SignatureId("erfunden")));
    schwelle.reads = StateMask::BOSS_HP;
    schwelle.writes = StateMask::PHASE;
    schwelle.control_flow_effect = verzweigt;
    schwelle.origin = "erfunden/schwelle";

    let mut wandler = beispielknoten(EffectOp::ConvertDamage);
    wandler.reads = StateMask::BOSS_HP;
    wandler.writes = StateMask::SHIELD;
    wandler.origin = "erfunden/wandler";

    let mut schildleser = beispielknoten(EffectOp::DealDamage);
    schildleser.reads = StateMask::SHIELD;
    schildleser.writes = StateMask::PLAYER_HP;
    schildleser.control_flow_effect = verzweigt;
    schildleser.origin = "erfunden/schlag";

    let befund = |knoten: &[&rota_sim::effects::EffectNode]| {
        Abhaengigkeitsmaske::aus_teilen(knoten, &[], &worn)
            .quelle(Stream::Crit)
            .verzweigt_an
            .clone()
    };
    assert!(
        befund(&[&schaden]).is_empty(),
        "Trägerzusicherung: ein Treffer allein verzweigt nicht"
    );
    assert_eq!(
        befund(&[&schaden, &schwelle]),
        vec![Fundstelle::Knoten("erfunden/schwelle")],
        "ein Knoten, der den Fortschritt liest und verzweigt, kippt die Maske"
    );
    assert!(
        befund(&[&schaden, &schildleser]).is_empty(),
        "Trägerzusicherung: ohne Wandler erreicht der Krit den Schutz nicht"
    );
    assert_eq!(
        befund(&[&schaden, &wandler, &schildleser]),
        vec![Fundstelle::Knoten("erfunden/schlag")],
        "über den Wandler erreicht der Krit den Schutz — und der Schlag verzweigt daran"
    );
}

/// **Die Beschreibung gegen den Kampf:** Jeder Zauber, der allein gewirkt
/// würfelt, trägt einen würfelnden Knoten. Sonst hielte die Maske einen Krit
/// für ruhend, der zieht — bis #920 so bei PYRE, PYROCLASM, WEDGE und EFFLUX,
/// die ohne Grundschaden Schaden austeilen.
///
/// Nur diese Richtung: Ein Knoten, der würfeln KANN, im Einzellauf aber nicht
/// zieht (WEDGE ohne Kontroll-Punkte), ist eine erlaubte Obermenge.
#[test]
fn jeder_wuerfelnde_zauber_traegt_einen_wuerfelnden_knoten() {
    let mut worn = WornValues::none();
    worn.purity = 500;
    let mut ohne_knoten = Vec::new();
    let mut wuerfelnd = 0;
    for (index, zauber) in rota_sim::SPELLS.iter().enumerate() {
        let apl = Apl {
            name: zauber.id.to_owned(),
            rules: vec![rota_sim::apl::Rule {
                spell: index,
                condition: rota_sim::apl::Condition::default(),
                line: 1,
            }],
        };
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            seed: 3,
            worn,
            ..rota_sim::FightConfig::default()
        });
        if fight.run_apl(&apl).random_draws == 0 {
            continue;
        }
        wuerfelnd += 1;
        let beschrieben = rota_sim::data::EFFECT_NODES
            .iter()
            .any(|n| n.origin == zauber.id && !n.randomness.ist_deterministisch());
        if !beschrieben {
            ohne_knoten.push(zauber.id);
        }
    }
    println!("{wuerfelnd} Zauber würfeln allein gewirkt");
    assert!(wuerfelnd > 0, "Trägerzusicherung: es wird gewürfelt");
    assert!(
        ohne_knoten.is_empty(),
        "diese Zauber würfeln, ohne dass ein Knoten es sagt: {ohne_knoten:?}"
    );
}

// ---------------------------------------------------------------- AC3

/// **Die Kontrollprobe aus dem Issue:** Für die Siegwahrscheinlichkeit sagt
/// die zweistellige Prüfung nein, während die einstellige ja sagen würde.
#[test]
fn ac3_die_siegwahrscheinlichkeit_wird_nicht_ausintegriert() {
    let maske = maske("first-crucible", 250, &gute_liste());
    assert!(
        !beruehrt_den_kontrollfluss(Stream::Crit, &maske),
        "Trägerzusicherung: einstellig würde hier ausintegriert"
    );
    assert!(
        kann_ausintegrieren(Stream::Crit, Ergebnisgroesse::ErwarteterSchaden, &maske),
        "Trägerzusicherung: für den erwarteten Schaden trägt es"
    );
    let falsch: Vec<Ergebnisgroesse> = Ergebnisgroesse::ALLE
        .into_iter()
        .filter(|g| !g.ist_linear())
        .filter(|g| kann_ausintegrieren(Stream::Crit, *g, &maske))
        .collect();
    assert!(
        Ergebnisgroesse::ALLE.iter().any(|g| !g.ist_linear()),
        "Trägerzusicherung: es gibt nichtlineare Größen"
    );
    assert!(
        falsch.is_empty(),
        "diese Größen sind im Krit nicht linear und wurden trotzdem ausintegriert: {falsch:?}"
    );
}

/// Die gestaffelte Stichprobe fragt nach der Siegwahrscheinlichkeit — an
/// demselben Boss, an dem die Suche analytisch rechnet, würfelt sie.
#[test]
fn ac3_der_gestaffelte_lauf_bleibt_bei_der_stichprobe() {
    let apl = gute_liste();
    let config = gegen("first-crucible", 250);
    assert_eq!(
        Durchgangswahl::fuer(&config, &apl, Ergebnisgroesse::ErwarteterSchaden),
        Durchgangswahl::Analytisch,
        "Trägerzusicherung: für den erwarteten Schaden greift hier der analytische Weg"
    );
    assert_eq!(
        Durchgangswahl::fuer(&config, &apl, Ergebnisgroesse::Siegwahrscheinlichkeit),
        Durchgangswahl::Gestaffelt
    );
    let lauf = run_gestaffelt(
        &config,
        &apl,
        Staffelung {
            schwelle: 0.9,
            alpha: 0.05,
            budget: 512,
        },
    )
    .expect("gültige Vorgabe");
    assert_eq!(lauf.wahl, Durchgangswahl::Gestaffelt);
    assert!(lauf.wuerfe > 0, "Trägerzusicherung: gewürfelt wurde");
}

// ---------------------------------------------------------------- AC4

#[test]
fn ac4_jede_quelle_steht_in_genau_einer_gruppe() {
    let a = StochastischeAufteilung::fuer(
        &gegen("first-crucible", 250),
        &gute_liste(),
        Ergebnisgroesse::ErwarteterSchaden,
    );
    assert_eq!(a.ausintegriert, quellen(&[Stream::Crit, Stream::DotTick]));
    assert_eq!(a.ruhend, quellen(&[Stream::Heal]), "aus Heal zieht nichts");
    assert!(a.gezogen.ist_leer(), "{:?}", a.gezogen);
    assert_eq!(
        a.gezogen.union(a.ausintegriert).union(a.ruhend),
        RandomSourceMask::ALLE,
        "jede Quelle steht in einer Gruppe"
    );
    assert_eq!(
        a.gezogen.anzahl() + a.ausintegriert.anzahl() + a.ruhend.anzahl(),
        RandomSourceMask::ALLE.anzahl(),
        "und in keiner zweiten"
    );
    let reihenfolge: Vec<Stream> = a.gueltigkeitsnachweis.iter().map(|(s, _)| *s).collect();
    assert_eq!(reihenfolge, Stream::ALL.to_vec(), "ein Nachweis je Quelle");
    for (quelle, begruendung) in &a.gueltigkeitsnachweis {
        match (quelle, begruendung) {
            (Stream::Heal, Begruendung::Ruht) => {}
            (Stream::Crit | Stream::DotTick, Begruendung::Ausintegriert(m))
                if m.intersects(StateMask::BOSS_HP) => {}
            anders => panic!("unerwarteter Nachweis: {anders:?}"),
        }
    }
    assert!(a.analytisch());
}

#[test]
fn ac4_eine_beruehrte_quelle_nennt_ihre_fundstelle() {
    let a = StochastischeAufteilung::fuer(
        &gegen("boss-1-2", 250),
        &gute_liste(),
        Ergebnisgroesse::ErwarteterSchaden,
    );
    assert_eq!(a.gezogen, quellen(&[Stream::Crit, Stream::DotTick]));
    assert_eq!(a.ruhend, quellen(&[Stream::Heal]));
    let krit = a
        .gueltigkeitsnachweis
        .iter()
        .find(|(s, _)| *s == Stream::Crit)
        .map(|(_, b)| b.clone());
    match krit {
        Some(Begruendung::Kontrollfluss(stellen)) => assert!(
            stellen.contains(&Fundstelle::Knoten("boss-1-2/phases")),
            "{stellen:?}"
        ),
        anders => panic!("der Krit an boss-1-2 muss am Kontrollfluss hängen: {anders:?}"),
    }
    assert!(!a.analytisch());
}

#[test]
fn ac4_bei_einer_nichtlinearen_groesse_wird_gezogen() {
    let a = StochastischeAufteilung::fuer(
        &gegen("first-crucible", 250),
        &gute_liste(),
        Ergebnisgroesse::Siegwahrscheinlichkeit,
    );
    assert_eq!(a.gezogen, quellen(&[Stream::Crit, Stream::DotTick]));
    assert!(a.ausintegriert.ist_leer());
    assert!(a
        .gueltigkeitsnachweis
        .iter()
        .any(|(s, b)| *s == Stream::Crit
            && *b == Begruendung::NichtLinear(Ergebnisgroesse::Siegwahrscheinlichkeit)));
    assert!(!a.analytisch());
}

#[test]
fn ac4_ohne_purity_ruht_jede_quelle() {
    let a = StochastischeAufteilung::fuer(
        &gegen("first-crucible", 0),
        &gute_liste(),
        Ergebnisgroesse::ErwarteterSchaden,
    );
    assert_eq!(a.ruhend, RandomSourceMask::ALLE);
    assert!(!a.analytisch(), "ohne Wurf gibt es nichts auszuintegrieren");
}

// ------------------------------------------- Abnahme: Boss und Stichprobe

/// Der analytische Weg greift an first-crucible, und sein Erwartungswert liegt
/// in der Stichproben-Unsicherheit der gestaffelten Stichprobe auf denselben
/// Startwerten.
#[test]
fn an_first_crucible_trifft_der_analytische_weg_die_gestaffelte_stichprobe() {
    let apl = gute_liste();
    let config = gegen("first-crucible", 250);
    assert_eq!(
        Durchgangswahl::fuer(&config, &apl, Ergebnisgroesse::ErwarteterSchaden),
        Durchgangswahl::Analytisch
    );
    let erwartet = run_erwartet(&config, &apl);
    assert_eq!(erwartet.weg, Durchgangswahl::Analytisch);
    assert_eq!(erwartet.stderr, 0.0, "ausintegriert streut nichts mehr");

    // Schwelle 0,999 bei alpha 0,01: gegen first-crucible gewinnt step1-good
    // jeden Kampf, die Folge hält erst nach einigen tausend.
    let lauf = run_gestaffelt(
        &config,
        &apl,
        Staffelung {
            schwelle: 0.999,
            alpha: 0.01,
            budget: 20_000,
        },
    )
    .expect("gültige Vorgabe");
    assert_eq!(lauf.wahl, Durchgangswahl::Gestaffelt);
    let (mittel, fehler) = lauf
        .schaden_mit_fehler()
        .expect("mehr als ein Kampf gerechnet");
    assert!(
        fehler > 0.0,
        "Trägerzusicherung: die Stichprobe streut ({fehler})"
    );
    let abstand = (erwartet.schaden - mittel).abs();
    println!(
        "analytisch {:.2} Schaden je Kampf; gestaffelt {mittel:.2} ± {fehler:.2} \
         (Standardfehler) aus {} Kämpfen; Abstand {abstand:.2} Schaden je Kampf = \
         {:.2} Standardfehler",
        erwartet.schaden,
        lauf.kaempfe,
        abstand / fehler
    );
    assert!(
        abstand < 4.0 * fehler,
        "der analytische Wert liegt {abstand:.2} Schaden je Kampf neben dem Mittel der \
         gestaffelten Stichprobe, erlaubt sind vier Standardfehler ({:.2})",
        4.0 * fehler
    );
}

// ------------------------------------------- die Nachprüfung

/// **Die Positivkontrolle des Abgleichs:** Wo der Krit den Kampf verändert,
/// muss er das an den Rändern sehen. Ohne sie wäre ein „stimmt überein" unten
/// auch das Urteil eines Abgleichs, der nichts vergleicht.
#[test]
fn der_randabgleich_erkennt_einen_anderen_kampf() {
    let apl = gute_liste();
    let mit_abbruch = BalanceConfig {
        stop_on_kill: true,
        ..gegen("first-crucible", 250)
    };
    for (fall, config) in [
        ("boss-1-2, Phasen", gegen("boss-1-2", 250)),
        ("first-crucible, Abbruch beim Sieg", mit_abbruch),
    ] {
        let (ohne, mit) = raender(&config, &apl).expect("das Protokoll reicht");
        let urteil = randabgleich(&ohne, &mit, config.worn.crit_factor());
        assert!(
            matches!(urteil, Err(Verweigerung::Abweichung { .. })),
            "{fall}: die Ränder fochten verschiedene Kämpfe, der Abgleich sagt {urteil:?}"
        );
    }
}

/// **Schaden ohne Krit ist kein anderer Kampf.** PYROCLASM bucht neben seinem
/// Entladeschaden den Rest der laufenden Brände (`Fight::cast`,
/// `consume_active_dots`) — ohne Krit-Faktor, also in beiden Randläufen
/// derselbe Betrag. Gemessen am 29.10. auf node3: Alle 211 Verweigerungen der
/// Suche über 21 Bosse hingen an diesem Eintrag.
#[test]
fn der_randabgleich_nimmt_schaden_ohne_krit_hin() {
    let apl = liste("smoulder if dot.smoulder.remains < 1500\npyroclasm\n");
    let config = gegen("first-crucible", 250);
    let (ohne, mit) = raender(&config, &apl).expect("das Protokoll reicht");
    let pyroclasm =
        u8::try_from(index_of("pyroclasm").expect("den Zauber gibt es")).expect("passt");
    let gleich_ohne_krit = ohne
        .eintraege
        .iter()
        .zip(&mit.eintraege)
        .filter(|(a, b)| {
            a.kind == rota_sim::log::LogKind::Damage
                && a.spell == pyroclasm
                && a.value > 0
                && a.value == b.value
        })
        .count();
    assert!(
        gleich_ohne_krit > 0,
        "Trägerzusicherung: PYROCLASM bucht einen Betrag, der in beiden Rändern gleich ist"
    );
    let aufteilung =
        StochastischeAufteilung::fuer(&config, &apl, Ergebnisgroesse::ErwarteterSchaden);
    assert!(
        aufteilung.analytisch(),
        "Trägerzusicherung: die Maske sieht die Liste frei"
    );
    assert_eq!(
        randabgleich(&ohne, &mit, config.worn.crit_factor()),
        Ok(()),
        "{gleich_ohne_krit} Beträge ohne Krit-Faktor sind derselbe Kampf, kein anderer"
    );
}

/// An jedem wertungsfähigen Boss, den die Maske frei sieht, fochten die
/// Ränder denselben Kampf — die Nachprüfung der Bossbeschreibung im Kampf.
#[test]
fn an_jedem_unberuehrten_wertungsboss_stimmen_die_raender() {
    let apl = gute_liste();
    let mut geprueft = 0;
    let mut verweigert = Vec::new();
    for boss in ScoringSet::all_scored().bosses() {
        let config = gegen(boss.id, 250);
        let aufteilung =
            StochastischeAufteilung::fuer(&config, &apl, Ergebnisgroesse::ErwarteterSchaden);
        if !aufteilung.analytisch() {
            continue;
        }
        geprueft += 1;
        if let Err(grund) = erwartung(&config, &apl, &aufteilung) {
            verweigert.push(format!("{}: {grund}", boss.id));
        }
    }
    println!("geprüft: {geprueft} Bosse");
    assert!(
        geprueft > 1,
        "Trägerzusicherung: mehr als ein Boss geht analytisch"
    );
    assert!(verweigert.is_empty(), "{verweigert:#?}");
}

// ------------------------------------------- die Suche

/// Dieselbe Suche zweimal: Mit dem analytischen Weg rechnet jede Liste zwei
/// Randläufe, ohne ihn fünfzig gewürfelte Kämpfe.
///
/// `measured` zählt die Versuche der Suche; dazu misst sie die leere Startliste
/// und am Ende die gefundene (`rotation_search.rs`, `measure(config, "start",
/// …)` und `"gefunden"`). Die leere Liste nennt keinen Zauber, also keine
/// Quelle — für sie gibt es nichts auszuintegrieren, und sie bleibt bei der
/// Stichprobe.
#[test]
fn die_suche_rechnet_mit_zwei_randlaeufen_statt_der_stichprobe() {
    let config = gegen("first-crucible", 250);
    let pool: Vec<_> = ["smoulder", "pyre", "kindle"]
        .iter()
        .flat_map(|id| candidates_for(index_of(id).expect("den Zauber gibt es")))
        .collect();
    let verweigert_vorher = verweigerungen();

    let zaehle = |f: &dyn Fn() -> rota_sim::rotation_search::SearchResult| {
        let werk = Zaehlwerk::neu(1);
        let ergebnis = {
            let _anmeldung = werk.anmelden();
            f()
        };
        (ergebnis, werk.momentaufnahme())
    };

    let (mit, stand_mit) = zaehle(&|| greedy(&config, &pool, 3));
    let (ohne, stand_ohne) = zaehle(&|| ohne_ausintegration(|| greedy(&config, &pool, 3)));

    let weg = |stand: &rota_sim::fortschritt::Momentaufnahme, w: Weg| stand.kandidaten[w.index()];
    let runs = u64::from(config.iterations);
    println!(
        "analytisch: {} Versuche, {} Kämpfe ({} analytisch, {} Stichprobe); \
         abgeschaltet: {} Versuche, {} Kämpfe ({} Stichprobe)",
        mit.measured,
        stand_mit.kaempfe,
        weg(&stand_mit, Weg::Analytisch),
        weg(&stand_mit, Weg::Stichprobe),
        ohne.measured,
        stand_ohne.kaempfe,
        weg(&stand_ohne, Weg::Stichprobe)
    );
    let n_mit = u64::try_from(mit.measured).expect("passt");
    let n_ohne = u64::try_from(ohne.measured).expect("passt");
    assert!(n_mit > 1, "Trägerzusicherung: die Suche hat gemessen");

    assert_eq!(
        weg(&stand_mit, Weg::Analytisch),
        n_mit + 1,
        "jeder Versuch und die gefundene Liste gingen den analytischen Weg"
    );
    assert_eq!(
        weg(&stand_mit, Weg::Stichprobe),
        1,
        "nur die leere Startliste"
    );
    assert_eq!(
        stand_mit.kaempfe,
        2 * (n_mit + 1) + runs,
        "zwei Randläufe je Liste, die Stichprobe nur für die leere"
    );

    assert_eq!(
        weg(&stand_ohne, Weg::Analytisch),
        0,
        "abgeschaltet nimmt die Suche den Weg nicht"
    );
    assert_eq!(weg(&stand_ohne, Weg::Stichprobe), n_ohne + 2);
    assert_eq!(
        stand_ohne.kaempfe,
        runs * (n_ohne + 2),
        "ohne ihn fünfzig Kämpfe je Liste"
    );

    assert_eq!(mit.best.stderr, 0.0, "ausintegriert streut nichts");
    assert!(ohne.best.stderr > 0.0, "gewürfelt streut");
    assert_eq!(
        verweigerungen(),
        verweigert_vorher,
        "kein Randabgleich verweigert"
    );
}
