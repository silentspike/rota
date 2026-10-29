//! Der Prüflauf vor Auslieferung über alle wertungsfähigen Bosse (#891 N1).
//!
//! # Was die Fälle zusammen belegen
//!
//! | Fall | Richtung |
//! | --- | --- |
//! | `jeder_wertungsfaehige_boss_steht_im_pruefplan` | fehlt ein Boss der Spielinstanzen, fällt der Fall beim Namen |
//! | `ein_boss_ohne_loesung_laesst_den_prueflauf_scheitern` | ein Prüfboss ohne Lösung sperrt den Lauf; die Kontrolle daneben gibt frei |
//! | `jede_kombination_traegt_die_erfolge_ihres_gegners` | die Erfolgsachse kommt aus `report::erfolge`, nicht aus einem Literal |
//! | `an_beiden_runaway_bossen_findet_die_suche_einen_sieg` | bei umgekehrtem Ziel sucht die Suche den Sieg, nicht den Schaden |
//! | `bei_umgekehrtem_ziel_ist_der_wert_der_ring_am_fristende` | die Wertung: Reserve bei Sieg, höchstens 0 sonst |
//! | `an_den_uebrigen_bossen_bleibt_der_schaden_die_zielgroesse` | die 19 übrigen Bosse rechnen bitgleich wie vorher |
//!
//! # Wogegen gemessen wird
//!
//! Ein kleiner Vorrat aus sechs Zaubern, Länge bis 3 — derselbe wie im
//! Prüffall des Report-Beispiels. Damit läuft derselbe Weg wie im Nachtlauf im
//! Debug-Bau in Sekunden; die Zahlen des vollen Laufs stehen im PR.

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::auslieferung::{
    alle_auslieferbar, loesung_suchen, messkonfiguration, nicht_auslieferbar, pruefe, pruefplan,
    Kombination, Rahmen, Stufe, Teil, BUDGET,
};
use rota_sim::balance;
use rota_sim::data::{self, boss_by_id, index_of};
use rota_sim::objective::{Outcome, WinCondition};
use rota_sim::report::{erfolge, erfolge_pruefen, feasibility_gegen, is_shippable};
use rota_sim::rotation_search::{bewerte, candidates_for, Candidate};
use rota_sim::scoring::ScoringSet;
use rota_sim::Boss;

/// Sechs Zauber, Länge bis 3 — siehe Kopf.
fn klein() -> Rahmen {
    let ids = ["kindle", "smoulder", "pyre", "crust", "quench", "fester"];
    let grundmenge: Vec<usize> = ids.iter().map(|id| index_of(id).expect("Zauber")).collect();
    let pool: Vec<Candidate> = grundmenge.iter().flat_map(|&i| candidates_for(i)).collect();
    Rahmen {
        pool,
        grundmenge,
        max_len: 3,
        budget: 1_000_000,
    }
}

fn kombination(boss_id: &str) -> Kombination {
    Kombination {
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt im Bestand")),
        stufe: Stufe::Normal,
    }
}

#[test]
fn jeder_wertungsfaehige_boss_steht_im_pruefplan() {
    // **Die Erwartung kommt NICHT aus `ScoringSet`.** Der Prüfplan baut auf
    // ihr auf; ein Fall, der dieselbe Funktion fragt, erbt jeden ihrer Fehler.
    // Hier wird die Regel aus #981 direkt an den Daten gelesen: Wertungsfähig
    // ist, wessen `instance` eine Spielinstanz benennt.
    let erwartet: Vec<&str> = data::BOSSES
        .iter()
        .filter(|b| data::SCORED_INSTANCES.contains(&b.instance))
        .map(|b| b.id)
        .collect();
    // Die Erwartung selbst trägt: Prolog und letzte Instanz stehen darin, ein
    // Prüfprofil nicht. Ohne diese Zeilen bestünde der Fall auch mit einer
    // leeren Erwartung gegen einen leeren Plan.
    for id in ["first-crucible", "boss-1-1", "boss-5-4"] {
        assert!(
            erwartet.contains(&id),
            "{id} fehlt in der Erwartung: {erwartet:?}"
        );
    }
    assert!(!erwartet.contains(&"test-onslaught"));

    let plan = pruefplan();
    for stufe in Stufe::ALLE {
        let im_plan: Vec<&str> = plan
            .iter()
            .filter(|k| k.stufe == stufe)
            .map(|k| k.boss.id)
            .collect();
        let fehlen: Vec<&str> = erwartet
            .iter()
            .filter(|id| !im_plan.contains(id))
            .copied()
            .collect();
        let zu_viel: Vec<&str> = im_plan
            .iter()
            .filter(|id| !erwartet.contains(id))
            .copied()
            .collect();
        let doppelt: Vec<&str> = im_plan
            .iter()
            .enumerate()
            .filter(|(i, id)| im_plan[..*i].contains(id))
            .map(|(_, id)| *id)
            .collect();
        assert!(
            fehlen.is_empty() && zu_viel.is_empty() && doppelt.is_empty(),
            "Stufe {}: Pruefplan mit {} von {} wertungsfaehigen Bossen.\n  es fehlen: {fehlen:?}\n  \
             zu viel: {zu_viel:?}\n  doppelt: {doppelt:?}",
            stufe.name(),
            im_plan.len() - zu_viel.len() - doppelt.len(),
            erwartet.len()
        );
    }
    assert_eq!(
        plan.len(),
        erwartet.len() * Stufe::ALLE.len(),
        "je Boss und Stufe genau eine Kombination"
    );
}

#[test]
fn ein_boss_ohne_loesung_laesst_den_prueflauf_scheitern() {
    let rahmen = klein();

    // **Die positive Kontrolle zuerst.** Ein Prüflauf, der jeden Boss sperrt,
    // bestünde den Fall darunter ebenso — und wäre wertlos.
    let gut = pruefe(kombination("first-crucible"), &rahmen);
    assert!(
        gut.auslieferbar(),
        "first-crucible muss mit dem kleinen Vorrat zu schaffen sein: {:#?}",
        gut.erfolge
    );
    assert!(alle_auslieferbar(std::slice::from_ref(&gut)));

    // **Der Prüfboss ohne Lösung.** 999 999 Lebenspunkte in 60 s — keine
    // Liste aus diesem Vorrat bringt das auf, und auch keine andere.
    let onslaught = boss_by_id("test-onslaught").expect("Pruefboss");
    let ohne = pruefe(kombination("test-onslaught"), &rahmen);
    assert!(
        !ohne.auslieferbar(),
        "test-onslaught ({} Lebenspunkte, {} ms) gilt als auslieferbar: {:#?}",
        onslaught.hp,
        onslaught.overpressure_ms,
        ohne.erfolge
    );
    assert_eq!(
        ohne.machbarkeit.boss, onslaught.name,
        "geprueft wurde ein anderer Boss als der der Kombination"
    );
    let sieg = ohne
        .erfolge
        .iter()
        .find(|c| c.name == "Sieg")
        .expect("Sieg-Eintrag");
    assert!(!sieg.solved);
    assert!(
        sieg.evidence.contains("nicht besiegt"),
        "der Beleg nennt den Grund nicht: {}",
        sieg.evidence
    );

    // **Das Urteil über den Lauf.** Eine einzige Kombination ohne Lösung sperrt
    // ihn, auch neben einer, die frei ist — und wer sie sucht, findet sie beim
    // Namen.
    let lauf = [gut, ohne];
    assert!(
        !alle_auslieferbar(&lauf),
        "ein Lauf mit einem Boss ohne Loesung wurde freigegeben"
    );
    let gesperrt: Vec<&str> = nicht_auslieferbar(&lauf)
        .iter()
        .map(|p| p.kombination.boss.id)
        .collect();
    assert_eq!(gesperrt, ["test-onslaught"]);

    // Ein leerer Lauf hat nichts geprüft und gibt deshalb nichts frei.
    assert!(!alle_auslieferbar(&[]));
}

#[test]
fn jede_kombination_traegt_die_erfolge_ihres_gegners() {
    let rahmen = klein();
    let p = pruefe(kombination("first-crucible"), &rahmen);
    let erwartet = erfolge(p.kombination.gegner()).len();
    assert_eq!(
        p.erfolge.len(),
        erwartet,
        "mit Sieg stehen alle {erwartet} Erfolge des Gegners im Ergebnis: {:#?}",
        p.erfolge
    );

    // Jeder Teil hat gemessen, und zwar etwas: Ein Posten mit null Kämpfen
    // hiesse, dass das Zählwerk den Teil nicht sieht — dann wäre die Kampfzahl
    // im Nachtlauf zu klein, und niemand merkte es.
    let teile: Vec<Teil> = p.posten.iter().map(|q| q.teil).collect();
    assert_eq!(teile, Teil::ALLE);
    for posten in &p.posten {
        if posten.teil == Teil::Mechanikproben {
            // First-crucible hat keine Mechanik — dort gibt es nichts zu
            // rechnen, und null ist die richtige Zahl.
            continue;
        }
        assert!(
            posten.kaempfe > 0,
            "{} hat keinen Kampf gezaehlt",
            posten.teil.name()
        );
    }
    assert_eq!(
        p.posten
            .iter()
            .find(|q| q.teil == Teil::Machbarkeit)
            .map(|q| q.kaempfe),
        Some(1),
        "die Machbarkeit rechnet genau einen Kampf"
    );
}

/// Die wertungsfähigen Bosse mit umgekehrtem Ziel, aus den Daten.
fn umgekehrte() -> Vec<&'static Boss> {
    ScoringSet::all_scored()
        .bosses()
        .iter()
        .copied()
        .filter(|b| WinCondition::of(b).is_inverted())
        .collect()
}

fn liste(text: &str, name: &str) -> Apl {
    parse(AplName(name), text).expect("eingebaute Liste")
}

#[test]
fn an_beiden_runaway_bossen_findet_die_suche_einen_sieg() {
    // **#891 N1, Entscheidung (A) des Orchestrators vom 29.10.** Bis hierher
    // maximierte die Suche an jedem Boss den Schaden. An `boss-2-3` und
    // `boss-5-3` ist der Ring auf null die Niederlage — die gefundene Liste
    // verlor dort (Ring bei 26000 bzw. 23000 ms auf null, Job 9acc8de4),
    // obwohl schon die leere Rotation beide gewinnt.
    //
    // Gesucht wird mit dem vollen Vorrat, derselben Suche wie im Nachtlauf.
    let bosse = umgekehrte();
    let ids: Vec<&str> = bosse.iter().map(|b| b.id).collect();
    assert!(
        ids.contains(&"boss-2-3") && ids.contains(&"boss-5-3"),
        "Voraussetzung: beide runaway-Bosse sind wertungsfaehig: {ids:?}"
    );

    let rahmen = Rahmen::voll(BUDGET);
    let mut gesperrt: Vec<String> = Vec::new();
    for boss in bosse {
        let elite = loesung_suchen(&messkonfiguration(boss), &rahmen);
        let f = feasibility_gegen(boss, &elite.best.apl, None);
        if !is_shippable(&erfolge_pruefen(&erfolge(boss), &f)) {
            gesperrt.push(format!(
                "{}: Ausgang {:?}, Ring {} von {}, Liste [{}]",
                boss.id,
                f.ausgang,
                f.ring_rest,
                f.hp,
                elite.best.apl.to_string().trim().replace('\n', " | ")
            ));
        }
    }
    assert!(
        gesperrt.is_empty(),
        "nicht auslieferbar, obwohl ein Sieg existiert (schon die leere Rotation gewinnt \
         beide): {gesperrt:#?}"
    );
}

#[test]
fn bei_umgekehrtem_ziel_ist_der_wert_der_ring_am_fristende() {
    // Die Festlegung des Orchestrators vom 29.10.: Bei `runaway` zählt der
    // Ring am Fristende (Reserve = Lebenspunkte minus Fortschritt), ein
    // gestorbener oder verlorener Durchgang höchstens 0. So liegt jeder Sieg
    // über jeder Niederlage, und unter Siegen gewinnt mehr Reserve.
    let boss = boss_by_id("boss-2-3").expect("Boss");
    let config = messkonfiguration(boss);
    let ring = f64::from(boss.hp);

    // Die leere Liste baut der Parser nicht ("keine einzige Regel"); die
    // Suche beginnt aber genau bei ihr — `greedy_zuege` misst sie als Start.
    let leer = Apl {
        name: "leer".to_owned(),
        rules: Vec::new(),
    };
    let mut siege: Vec<(String, f64)> = Vec::new();
    for (name, apl) in [
        ("leer", leer),
        (
            "e939-schutz",
            liste(include_str!("../apl/e939-schutz.apl"), "e939-schutz"),
        ),
    ] {
        let lauf = balance::run(&config, &apl);
        let s = lauf.samples[0];
        assert_eq!(
            Outcome::of(&s, WinCondition::of(boss)),
            Outcome::Won,
            "Voraussetzung: {name} gewinnt an boss-2-3"
        );
        let wert = bewerte(&config, apl).mean;
        assert_eq!(
            wert.to_bits(),
            (ring - f64::from(s.damage)).to_bits(),
            "{name}: der Wert ist der Ring am Fristende ({} minus {}), nicht {wert}",
            boss.hp,
            s.damage
        );
        assert!(wert > 0.0);
        siege.push((name.to_owned(), wert));
    }
    assert!(
        siege[1].1 > siege[0].1,
        "mehr Reserve gewinnt: e939-schutz haelt den Ring, die leere Liste nicht: {siege:?}"
    );

    let verliert = liste(
        include_str!("../apl/e607-flashpoint.apl"),
        "e607-flashpoint",
    );
    let s = balance::run(&config, &verliert).samples[0];
    assert_eq!(
        Outcome::of(&s, WinCondition::of(boss)),
        Outcome::Overpressure,
        "Voraussetzung: e607-flashpoint verliert an boss-2-3"
    );
    let wert = bewerte(&config, verliert).mean;
    assert!(
        wert <= 0.0,
        "eine Niederlage ist hoechstens 0 wert, nicht {wert}"
    );
    assert!(siege.iter().all(|(_, sieg)| *sieg > wert));
}

#[test]
fn an_den_uebrigen_bossen_bleibt_der_schaden_die_zielgroesse() {
    // **Die Wertung ändert sich nur bei umgekehrtem Ziel.** An jedem anderen
    // wertungsfähigen Boss ist der Wert einer Liste bitgleich der mittlere
    // Schaden — mit `prevented_weight` 0 genau `damage.mean`, wie vor #891 N1.
    let liste = liste(include_str!("../apl/step1-good.apl"), "step1-good");
    let mut geprueft = Vec::new();
    for &boss in ScoringSet::all_scored().bosses() {
        if WinCondition::of(boss).is_inverted() {
            continue;
        }
        let config = messkonfiguration(boss);
        let lauf = balance::run(&config, &liste);
        let gemessen = bewerte(&config, liste.clone());
        assert_eq!(
            (gemessen.mean.to_bits(), gemessen.stderr.to_bits()),
            (lauf.damage.mean.to_bits(), lauf.damage.stderr.to_bits()),
            "{}: der Wert ist nicht mehr der Schaden ({} gegen {})",
            boss.id,
            gemessen.mean,
            lauf.damage.mean
        );
        geprueft.push(boss.id);
    }
    // Die Zahl steht in der Meldung, die Erwartung kommt aus den Daten.
    let erwartet = ScoringSet::all_scored().len() - umgekehrte().len();
    assert_eq!(geprueft.len(), erwartet, "{geprueft:?}");
    assert!(
        !geprueft.is_empty(),
        "kein Boss mit gewoehnlichem Ziel geprueft — der Fall misst dann nichts"
    );
}
