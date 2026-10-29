//! Die inkrementelle Auswertung gegen die Vollsimulation (Issue #917, AC0–AC2,
//! AC6 Korrektheit je Operator).
//!
//! # Der Vertrag
//!
//! ```text
//! berechneter Startprüfpunkt  ≤  tatsächlicher erster Divergenzpunkt
//! optimierter Suchlauf        =  bitgleich dasselbe Ergebnis
//! ```
//!
//! Ein einziger zu später Punkt ist ein Fehlschlag, auch wenn alle anderen
//! stimmen — der Suffix-Lauf begänne in einem Zustand, den die Abwandlung nie
//! erreicht hätte. Jede Prüfung hier läuft deshalb über `pruefe` aus dem
//! Hilfsmodul: Vollsimulation als aufgezeichneter Suchkampf UND als Prüfkampf,
//! erste Abweichung aus der echten Entscheidungsfolge, Ergebnis bitgleich.
//!
//! # Die Basislisten
//!
//! Geschriebene Listen aus `sim/apl/` an den Bossen, für die sie gebaut wurden,
//! dazu eine Liste mit **Lücken oben** — bedingte Zeilen über dem Füller. An ihr
//! ist die Annahme „Position p → Prüfpunkt p“ gemessen widerlegt worden
//! (`docs/SIM-ARCHITEKTUR-PLAN.md`, E11.4): Eine weiter unten eingefügte Zeile
//! wirkt dort schon vor der Einfügeposition.

#[allow(dead_code)]
mod inkrementell_hilfe;

use inkrementell_hilfe::{
    eingefuegt, entfernt, getauscht, kampf, pruefe, vorrat, wuerfelnder_kampf, Zaehlung, Zufall,
};
use rota_sim::apl::{parse, Apl, AplName, Rule};
use rota_sim::fight::{AuditFight, FightConfig};
use rota_sim::inkrementell::{Basislauf, RuleId};

/// Eine Liste mit Lücken oben: Die bedingten Zeilen greifen oft nicht, und
/// darunter steht der Füller.
const LUECKEN_OBEN: &str = "\
pyre     if heat >= 15
smoulder if dot.smoulder.remains < gcd
quench   if boss.casting >= 1
kindle
";

/// Die Basislisten mit ihrem Boss.
fn basen() -> Vec<(&'static str, Apl, &'static str)> {
    let liste = |name: &'static str, text: &str| {
        parse(AplName(name), text).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    vec![
        (
            "step3-good",
            liste("step3-good", include_str!("../apl/step3-good.apl")),
            "boss-1-1",
        ),
        (
            "e797-mit-quench",
            liste(
                "e797-mit-quench",
                include_str!("../apl/e797-mit-quench.apl"),
            ),
            "test-lethal-strike",
        ),
        (
            "instance1-unterbrechen",
            liste(
                "instance1-unterbrechen",
                include_str!("../apl/instance1-unterbrechen.apl"),
            ),
            "boss-1-4",
        ),
        (
            "step553-alle-bedingt",
            liste(
                "step553-alle-bedingt",
                include_str!("../apl/step553-alle-bedingt.apl"),
            ),
            "boss-2-2",
        ),
        (
            "e607b-basis",
            liste("e607b-basis", include_str!("../apl/e607b-basis.apl")),
            "boss-3-2",
        ),
        (
            "luecken-oben",
            liste("luecken-oben", LUECKEN_OBEN),
            "boss-1-2",
        ),
    ]
}

/// Zeichnet die Basis mit dem ganzen Kandidatenvorrat des Bosses auf.
fn aufgezeichnet(config: FightConfig, apl: &Apl) -> (Basislauf, Vec<Rule>) {
    let vorrat = vorrat(&config);
    (Basislauf::aufzeichnen(config, apl, &vorrat), vorrat)
}

#[test]
fn die_aufzeichnung_aendert_den_lauf_nicht() {
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, _) = aufgezeichnet(config, &apl);
        let mut pruef = AuditFight::new(config);
        pruef.run_apl(&apl);
        assert_eq!(basis.ziel(), pruef.zielwerte(), "{name} an {boss}");
        // Der Träger: Ohne Entscheidungen prüfte hier nichts etwas.
        assert!(
            basis.punkte().len() > 10,
            "{name}: nur {} Prüfpunkte",
            basis.punkte().len()
        );
        assert!(
            basis.ziel().damage > 0,
            "{name}: kein Schaden — läuft die Liste?"
        );
    }
}

#[test]
fn jeder_pruefpunkt_setzt_den_lauf_bitgleich_fort() {
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, _) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        for (index, punkt) in basis.punkte().iter().enumerate() {
            let ziel = suche.fortsetzen(&punkt.state, &apl);
            assert_eq!(
                ziel,
                basis.ziel(),
                "{name} an {boss}: ab Prüfpunkt {index} ({} ms) fortgesetzt",
                punkt.accumulated_result.time
            );
        }
    }
}

#[test]
fn jede_einfuegung_startet_nie_nach_der_ersten_abweichung() {
    let mut zaehlung = Zaehlung::default();
    // Wie oft eine Einfügung VOR ihrer Position wirkte — der gemessene
    // Widerspruch zu „Position p → Prüfpunkt p“.
    let mut vor_der_position = 0;
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, vorrat) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        let liste = basis.basis();
        // Ein fester Ausschnitt des Vorrats, jede Zeile an jeder Stelle.
        let mut zufall = Zufall::new(917);
        for _ in 0..24 {
            let neu = basis.vorrat(zufall.bis(vorrat.len()));
            for stelle in 0..=liste.len() {
                let abgewandelt = eingefuegt(&liste, stelle, neu);
                let was = format!(
                    "{name} an {boss}: {:?} an Stelle {stelle}",
                    basis.regel(neu).condition
                );
                let befund = pruefe(&basis, config, &abgewandelt, &mut suche, &was);
                if befund.tatsaechlich.is_some_and(|t| t < stelle) {
                    vor_der_position += 1;
                }
                // AC2, Zeile 1 — wo die Abwandlung anders WÄHLT (Startpunkt =
                // Abweichung): Die neue Bedingung gilt, ihr Zauber ist bereit,
                // sie steht vor der damals gewählten Regel, und keine höher
                // stehende griff.
                if let (Some(b), Some(t)) = (befund.berechnet, befund.tatsaechlich) {
                    if b == t {
                        let punkt = &basis.punkte()[b];
                        let greift = |id: RuleId| {
                            punkt.valid_rules.enthaelt(usize::from(id))
                                && punkt.ready_spells.enthaelt(basis.regel(id).spell)
                        };
                        assert!(greift(neu), "{was}: die neue Zeile greift an {b} nicht");
                        assert!(
                            liste[..stelle].iter().all(|&id| !greift(id)),
                            "{was}: an {b} griff eine höher stehende Regel"
                        );
                        let damals = punkt
                            .selected_rule
                            .and_then(|id| liste.iter().position(|&x| x == id));
                        assert!(
                            damals.is_none_or(|g| g >= stelle),
                            "{was}: die damals gewählte Regel {damals:?} steht über der Einfügung"
                        );
                    }
                }
                zaehlung.nimm(&basis, &befund);
            }
        }
    }
    zaehlung.melden("Einfügen");
    assert!(
        zaehlung.unveraendert > 0,
        "kein einziges Mal „ändert nichts“"
    );
    assert!(
        zaehlung.unveraendert < zaehlung.geprueft,
        "jede Einfügung „ändert nichts“ — der Erkenner sagt immer dasselbe"
    );
    assert!(
        vor_der_position > 0,
        "keine Einfügung wirkte vor ihrer Position — die Prüfung sähe den widerlegten \
         Fall „Position p → Prüfpunkt p“ nicht"
    );
}

#[test]
fn jede_entfernung_startet_nie_nach_der_ersten_abweichung() {
    let mut zaehlung = Zaehlung::default();
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, _) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        let liste = basis.basis();
        for stelle in 0..liste.len() {
            let was = format!("{name} an {boss}: Zeile {stelle} entfernt");
            let befund = pruefe(&basis, config, &entfernt(&liste, stelle), &mut suche, &was);
            // AC2, Zeile 2 — wo die Abwandlung anders WÄHLT: Dort wurde die
            // entfernte Regel gewählt. (Schafft sie nur einen anderen
            // Entscheidungszeitpunkt, liegt der Startpunkt einen davor.)
            if let (Some(b), Some(t)) = (befund.berechnet, befund.tatsaechlich) {
                if b == t {
                    assert_eq!(
                        basis.punkte()[b].selected_rule,
                        Some(liste[stelle]),
                        "{was}: an {b} war nicht die entfernte Regel gewählt"
                    );
                }
            }
            zaehlung.nimm(&basis, &befund);
        }
    }
    zaehlung.melden("Entfernen");
    assert!(zaehlung.geprueft > 20);
}

#[test]
fn jeder_tausch_startet_nie_nach_der_ersten_abweichung() {
    let mut zaehlung = Zaehlung::default();
    let mut benachbart = 0;
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, _) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        let liste = basis.basis();
        for a in 0..liste.len() {
            for b in a + 1..liste.len() {
                let was = format!("{name} an {boss}: Zeilen {a} und {b} getauscht");
                let befund = pruefe(&basis, config, &getauscht(&liste, a, b), &mut suche, &was);
                // AC2, Zeile 3: Zwei benachbarte Zeilen wirken getauscht dort,
                // wo beide gültig und bereit waren — nie davor.
                if b == a + 1 {
                    benachbart += 1;
                    let greifen_beide = |p: &rota_sim::inkrementell::DecisionCheckpoint| {
                        [liste[a], liste[b]].iter().all(|&id| {
                            p.valid_rules.enthaelt(usize::from(id))
                                && p.ready_spells.enthaelt(basis.regel(id).spell)
                        })
                    };
                    let beide = basis.punkte().iter().position(greifen_beide);
                    match (befund.berechnet, beide) {
                        (None, _) => {}
                        (Some(s), Some(fruehest)) => {
                            assert!(s >= fruehest, "{was}: Startpunkt {s} vor {fruehest}");
                            assert!(
                                greifen_beide(&basis.punkte()[s]),
                                "{was}: am Startpunkt {s} greifen nicht beide"
                            );
                        }
                        (Some(s), None) => {
                            panic!("{was}: Startpunkt {s}, aber nie greifen beide")
                        }
                    }
                }
                zaehlung.nimm(&basis, &befund);
            }
        }
    }
    zaehlung.melden("Tauschen");
    assert!(benachbart > 10);
}

#[test]
fn gekoppelte_und_zerstoerende_abwandlungen_starten_nie_zu_spaet() {
    // Paar-Zug (#558), Dreiergruppe mit Verdrängen (#797, #1027): mehrere
    // Zeilen auf einmal, Zeilen raus und andere hinein.
    let mut zaehlung = Zaehlung::default();
    let mut zufall = Zufall::new(1027);
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, vorrat) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        let liste = basis.basis();
        for versuch in 0..24 {
            let mut abgewandelt = liste.clone();
            // Zerstören: null bis zwei Zeilen raus.
            for _ in 0..zufall.bis(3).min(abgewandelt.len()) {
                let raus = zufall.bis(abgewandelt.len());
                abgewandelt.remove(raus);
            }
            // Reparieren: zwei oder drei Zeilen zusammen hinein.
            let stelle = zufall.bis(abgewandelt.len() + 1);
            for k in 0..2 + zufall.bis(2) {
                abgewandelt.insert(stelle + k, basis.vorrat(zufall.bis(vorrat.len())));
            }
            let was = format!("{name} an {boss}: Versuch {versuch}, {abgewandelt:?}");
            let befund = pruefe(&basis, config, &abgewandelt, &mut suche, &was);
            zaehlung.nimm(&basis, &befund);
        }
    }
    zaehlung.melden("Zerstören und reparieren");
}

#[test]
fn zufaellige_abwandlungen_starten_nie_zu_spaet() {
    let mut zaehlung = Zaehlung::default();
    let mut zufall = Zufall::new(4917);
    for (name, apl, boss) in basen() {
        let config = kampf(boss, 5);
        let (basis, vorrat) = aufgezeichnet(config, &apl);
        let mut suche = basis.suchkampf();
        let universum = basis.basis().len() + vorrat.len();
        for versuch in 0..30 {
            let laenge = 1 + zufall.bis(8);
            let liste: Vec<RuleId> = (0..laenge)
                .map(|_| RuleId::try_from(zufall.bis(universum)).expect("passt"))
                .collect();
            let was = format!("{name} an {boss}: Zufallsliste {versuch}, {liste:?}");
            let befund = pruefe(&basis, config, &liste, &mut suche, &was);
            zaehlung.nimm(&basis, &befund);
        }
    }
    zaehlung.melden("Zufallslisten");
}

#[test]
fn ein_wuerfelnder_kampf_wird_ebenso_exakt_fortgesetzt() {
    // Der Zufallsstrom gehört zum Zustand. Ohne ihn zöge ein Suffix-Lauf die
    // Würfe von vorn.
    let (_, apl, boss) = basen().swap_remove(0);
    let config = wuerfelnder_kampf(boss, 5);
    let (basis, vorrat) = aufgezeichnet(config, &apl);
    assert!(
        basis.ziel().random_draws > 0,
        "der Kampf würfelt nicht — die Prüfung hätte keinen Träger"
    );
    let mut suche = basis.suchkampf();
    let liste = basis.basis();
    let mut zaehlung = Zaehlung::default();
    let mut zufall = Zufall::new(5);
    for _ in 0..20 {
        let neu = basis.vorrat(zufall.bis(vorrat.len()));
        let stelle = zufall.bis(liste.len() + 1);
        let was = format!("würfelnd: Zeile {neu} an Stelle {stelle}");
        let befund = pruefe(
            &basis,
            config,
            &eingefuegt(&liste, stelle, neu),
            &mut suche,
            &was,
        );
        zaehlung.nimm(&basis, &befund);
    }
    for stelle in 0..liste.len() {
        let was = format!("würfelnd: Zeile {stelle} entfernt");
        let befund = pruefe(&basis, config, &entfernt(&liste, stelle), &mut suche, &was);
        zaehlung.nimm(&basis, &befund);
    }
    zaehlung.melden("würfelnd");
}

#[test]
fn ein_universum_ueber_128_regeln_verliert_keine_regel() {
    // AC3: Die Regelmenge hat keine feste Breite. Der Vorrat zweimal
    // hintereinander sprengt die 128 Plätze ohne Speicheranforderung.
    let (_, apl, boss) = basen().swap_remove(0);
    let config = kampf(boss, 5);
    let einfach = vorrat(&config);
    let doppelt: Vec<Rule> = einfach.iter().chain(&einfach).cloned().collect();
    let basis = Basislauf::aufzeichnen(config, &apl, &doppelt);
    let breite = basis.punkte()[0].valid_rules.breite();
    assert!(
        breite > 128,
        "nur {breite} Regeln — der Fall prüft den Blockpfad nicht"
    );
    assert!(basis.punkte()[0].valid_rules.im_blockpfad());
    // Die Regeln jenseits von 128 tragen dieselben Bits wie ihr Zwilling
    // davor: Verlöre die Menge eine, wiche sie hier ab.
    let laenge = apl.rules.len();
    for punkt in basis.punkte() {
        for k in 0..einfach.len() {
            assert_eq!(
                punkt.valid_rules.enthaelt(laenge + k),
                punkt.valid_rules.enthaelt(laenge + einfach.len() + k),
                "Zeile {k} und ihr Zwilling bei {} ms",
                punkt.accumulated_result.time
            );
        }
    }
    let mut suche = basis.suchkampf();
    let liste = basis.basis();
    let mut zufall = Zufall::new(128);
    // Nur Zeilen jenseits von 128 — dort, wo eine feste Breite sie verlöre.
    let ab = 128 - laenge;
    for _ in 0..20 {
        let neu = basis.vorrat(ab + zufall.bis(doppelt.len() - ab));
        assert!(usize::from(neu) >= 128);
        let stelle = zufall.bis(liste.len() + 1);
        let was = format!("Blockpfad: Zeile {neu} an Stelle {stelle}");
        pruefe(
            &basis,
            config,
            &eingefuegt(&liste, stelle, neu),
            &mut suche,
            &was,
        );
    }
}

/// Ein Zeitpunkt auf dem Takt-Raster, an dem die Basis NICHT entscheidet,
/// mit Entscheidungen davor und danach — dort kann nur ein Weckruf der
/// Abwandlung einen Entscheidungspunkt schaffen.
fn zeit_zwischen_entscheidungen(basis: &Basislauf, ab: u32) -> u32 {
    let zeiten: Vec<u32> = basis
        .punkte()
        .iter()
        .map(|p| p.accumulated_result.time)
        .collect();
    (ab / 10..)
        .map(|takt| takt * 10)
        .find(|t| {
            !zeiten.contains(t) && zeiten.iter().any(|z| z < t) && zeiten.iter().any(|z| z > t)
        })
        .expect("die Basis entscheidet an jedem Takt")
}

/// Fügt eine geschriebene Zeile ein und prüft, dass sie über ihren WECKRUF
/// abweicht: Der Startpunkt liegt genau einen Prüfpunkt vor der Abweichung.
fn weckruf_fall(
    basis_text: &str,
    zeile: &str,
    stelle: usize,
    boss: &str,
) -> inkrementell_hilfe::Befund {
    let config = kampf(boss, 5);
    let basis_apl = parse(AplName("basis"), basis_text).expect("Basis");
    let probe = Basislauf::aufzeichnen(config, &basis_apl, &[]);
    let t = zeit_zwischen_entscheidungen(&probe, 4000);
    let zeile = zeile.replace("{T}", &t.to_string());
    let neu = parse(AplName("neu"), &zeile).expect("Zeile");
    let basis = Basislauf::aufzeichnen(config, &basis_apl, &neu.rules);
    let mut suche = basis.suchkampf();
    let liste = eingefuegt(&basis.basis(), stelle, basis.vorrat(0));
    let was = format!("{zeile} in „{}“", basis_text.trim().replace('\n', " / "));
    let befund = pruefe(&basis, config, &liste, &mut suche, &was);
    let (Some(b), Some(t_ab)) = (befund.berechnet, befund.tatsaechlich) else {
        panic!("{was}: keine Abweichung — der Fall prüft den Weckruf nicht ({befund:?})");
    };
    assert_eq!(
        t_ab,
        b + 1,
        "{was}: die Abweichung liegt nicht genau einen Prüfpunkt nach dem Startpunkt \
         — es war keine Abweichung über den Weckruf"
    );
    befund
}

#[test]
fn ein_weckruf_der_abwandlung_wird_als_abweichung_erkannt() {
    // 1. Die Basis wartet (kein Füller, keine Hitze für PYRE); die eingefügte
    //    Zeile weckt zwischen zwei ihrer Entscheidungen und WIRKT dort.
    weckruf_fall(
        "pyre if heat >= 15\n",
        "kindle if time >= {T}\n",
        0,
        "boss-1-1",
    );
    // 2. Dieselbe Basis; die Zeile weckt, greift aber nicht (die Hitze reicht
    //    nie). Die Cast-Folge bleibt — der Entscheidungszeitpunkt nicht. Ein
    //    Startpunkt, der nur die Wahl ansähe, meldete hier „ändert nichts“.
    let still = weckruf_fall(
        "pyre if heat >= 15\n",
        "kindle if time >= {T} and heat >= 1000\n",
        0,
        "boss-1-1",
    );
    assert!(!still.auswertung.unveraendert());
    // 3. Ein Zauber OHNE Wartezeit, mitten in einer laufenden: Der Füller hält
    //    die Wartezeit dauernd besetzt, QUENCH darf trotzdem — sein Weckruf
    //    darf nicht auf das Ende der Wartezeit geschoben werden.
    weckruf_fall("kindle\n", "quench if time >= {T}\n", 0, "boss-1-1");
}

#[test]
fn eine_entfernte_zeile_tilgt_ihren_weckruf() {
    // Die Basis trägt eine Zeile, die weckt und nie greift; ohne sie entfällt
    // der Entscheidungspunkt. Abweichung über den Weckruf, nicht über die Wahl.
    let config = kampf("boss-1-1", 5);
    let probe = Basislauf::aufzeichnen(
        config,
        &parse(AplName("probe"), "pyre if heat >= 15\n").expect("Probe"),
        &[],
    );
    let t = zeit_zwischen_entscheidungen(&probe, 4000);
    let text = format!("kindle if time >= {t} and heat >= 1000\npyre if heat >= 15\n");
    let apl = parse(AplName("basis"), &text).expect("Basis");
    let basis = Basislauf::aufzeichnen(config, &apl, &[]);
    let mut suche = basis.suchkampf();
    let liste = entfernt(&basis.basis(), 0);
    let befund = pruefe(&basis, config, &liste, &mut suche, "Weckzeile entfernt");
    let (Some(b), Some(t_ab)) = (befund.berechnet, befund.tatsaechlich) else {
        panic!("keine Abweichung — der Fall prüft den getilgten Weckruf nicht ({befund:?})");
    };
    assert_eq!(t_ab, b + 1, "keine Abweichung über den Weckruf");
}
