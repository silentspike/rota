//! Leistung der inkrementellen Auswertung je Operator (Issue #917, AC6).
//!
//! Je Boss und Basislänge die Greedy-Basis wie in Anhang B.2.1 (60 s,
//! Startwert 5), dann je Operator alle Abwandlungen einer Runde:
//!
//! | Operator | Abwandlungen |
//! | --- | --- |
//! | Einfügen | jede offene Zeile an jeder Stelle |
//! | Entfernen | jede Zeile |
//! | Tauschen | jedes Paar |
//!
//! Gerechnet wird jede Abwandlung **zweimal**: als Vollkampf im Suchkampf und
//! inkrementell gegen die Aufzeichnung der Basis — die Zeit der Aufzeichnung
//! zählt zum inkrementellen Weg. Aufgezeichnet wird mit dem kleinsten
//! Universum, das der Operator braucht: beim Einfügen Basis und Vorrat, beim
//! Entfernen und Tauschen nur die Basis. Die Ergebnisse werden je Abwandlung verglichen;
//! weicht eines ab, bricht das Programm ab, bevor es eine Zahl meldet.
//!
//! **Eine Gesamtquote über alle Operatoren gibt es hier mit Absicht nicht**
//! (AC6): Die gemessenen 40,9 bis 51,6 Prozent stammen vom Greedy-Einfügen; für
//! die anderen Operatoren gibt es keine solche Zahl.
//!
//! ```text
//! cargo remote -c -- build --release --example inkrementell-operatoren
//! inkrementell-operatoren [--wiederholungen N]
//! ```
//!
//! Zeiten nur im Messfenster auf dem Messrechner; die Zählungen hängen an keiner
//! Maschine.

use std::time::Instant;

use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::fight::{FightConfig, SearchFight, Zielwerte};
use rota_sim::inkrementell::{Basislauf, RuleId};
use rota_sim::rotation_search::{all_candidates, boss_kandidaten, greedy_zuege, Candidate, Zuege};

const DAUER_MS: u32 = 60_000;
const STARTWERT: u32 = 5;
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-2-2", "boss-3-4"];

fn main() {
    let mut wiederholungen = 3;
    let mut argumente = std::env::args().skip(1);
    while let Some(a) = argumente.next() {
        match a.as_str() {
            "--wiederholungen" => {
                wiederholungen = argumente
                    .next()
                    .and_then(|w| w.parse().ok())
                    .expect("--wiederholungen braucht eine Zahl");
            }
            anderes => panic!("unbekanntes Argument {anderes}"),
        }
    }
    println!(
        "boss\tlaenge\toperator\tabwandlungen\tunveraendert\tereignisse_gespart_pct\t\
         voll_ms\tinkrementell_ms\tfaktor"
    );
    for boss in BOSSE {
        for laenge in [2, 5] {
            let Some((config, basis_apl, offen)) = basis(boss, laenge) else {
                println!("{boss}\t{laenge}\t-\tGreedy hielt vor Länge {laenge} an");
                continue;
            };
            let probe = Basislauf::aufzeichnen(config, &basis_apl, &offen);
            let liste = probe.basis();
            let operatoren: [(&str, Vec<Vec<RuleId>>); 3] = [
                (
                    "einfuegen",
                    (0..offen.len())
                        .flat_map(|k| {
                            let liste = &liste;
                            let neu = probe.vorrat(k);
                            (0..=liste.len()).map(move |stelle| {
                                let mut v = liste.clone();
                                v.insert(stelle, neu);
                                v
                            })
                        })
                        .collect(),
                ),
                (
                    "entfernen",
                    (0..liste.len())
                        .map(|stelle| {
                            let mut v = liste.clone();
                            v.remove(stelle);
                            v
                        })
                        .collect(),
                ),
                (
                    "tauschen",
                    (0..liste.len())
                        .flat_map(|a| (a + 1..liste.len()).map(move |b| (a, b)))
                        .map(|(a, b)| {
                            let mut v = liste.clone();
                            v.swap(a, b);
                            v
                        })
                        .collect(),
                ),
            ];
            for (name, abwandlungen) in &operatoren {
                // Das Universum so groß wie nötig: Nur das Einfügen braucht die
                // Bits des Vorrats; Entfernen und Tauschen setzen sich aus den
                // Regeln der Basis zusammen.
                let vorrat: &[Rule] = if *name == "einfuegen" { &offen } else { &[] };
                for _ in 0..wiederholungen {
                    messen(boss, laenge, name, config, &basis_apl, vorrat, abwandlungen);
                }
            }
        }
    }
}

/// Die Greedy-Basis der Länge `laenge` und die offenen Zeilen, wie in
/// `greedy_zuege`.
fn basis(boss: &str, laenge: usize) -> Option<(FightConfig, Apl, Vec<Rule>)> {
    let boss = boss_by_id(boss).expect("Boss fehlt");
    let suche = BalanceConfig {
        iterations: 1,
        base_seed: STARTWERT,
        threads: 1,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        boss,
        ..BalanceConfig::default()
    };
    let pool = all_candidates();
    let erweitert: Vec<Candidate> = pool
        .iter()
        .chain(&boss_kandidaten(&suche, &pool))
        .cloned()
        .collect();
    let gefunden = greedy_zuege(&suche, &pool, laenge, &[], Zuege::NUR_EINZELN).chosen;
    if gefunden.len() != laenge {
        return None;
    }
    let offen = erweitert
        .iter()
        .filter(|k| !gefunden.contains(k))
        .map(Candidate::rule)
        .collect();
    let apl = Apl {
        name: format!("basis-{laenge}"),
        rules: gefunden.iter().map(Candidate::rule).collect(),
    };
    let config = FightConfig {
        seed: STARTWERT,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        log_capacity: 0,
        boss,
        ..FightConfig::default()
    };
    Some((config, apl, offen))
}

fn messen(
    boss: &str,
    laenge: usize,
    operator: &str,
    config: FightConfig,
    basis_apl: &Apl,
    offen: &[Rule],
    abwandlungen: &[Vec<RuleId>],
) {
    // Die Listen werden vorab gebaut, für beide Wege gleich — gemessen wird das
    // Rechnen, nicht das Zusammensetzen.
    let probe = Basislauf::aufzeichnen(config, basis_apl, offen);
    let apls: Vec<Apl> = abwandlungen.iter().map(|l| probe.apl(l)).collect();

    let beginn = Instant::now();
    let mut kampf = SearchFight::new(config);
    let voll: Vec<Zielwerte> = apls
        .iter()
        .map(|apl| {
            kampf.reset(config.seed);
            kampf.run_apl(apl)
        })
        .collect();
    let voll_zeit = beginn.elapsed();

    let beginn = Instant::now();
    let basis = Basislauf::aufzeichnen(config, basis_apl, offen);
    let mut suche = basis.suchkampf();
    let inkrementell: Vec<_> = abwandlungen
        .iter()
        .map(|l| basis.auswerten(l, &mut suche))
        .collect();
    let inkrementell_zeit = beginn.elapsed();

    let mut gespart = 0u64;
    for (index, (v, i)) in voll.iter().zip(&inkrementell).enumerate() {
        assert_eq!(
            *v,
            i.ziel(),
            "{boss}, Länge {laenge}, {operator}: Abwandlung {index} weicht ab"
        );
        gespart += u64::from(match i {
            rota_sim::inkrementell::Auswertung::Unveraendert(_) => basis.ereignisse(),
            rota_sim::inkrementell::Auswertung::Suffix { ab, .. } => {
                basis.punkte()[*ab].event_cursor
            }
        });
    }
    let unveraendert = inkrementell.iter().filter(|i| i.unveraendert()).count();
    let gesamt = u64::from(basis.ereignisse()) * abwandlungen.len() as u64;
    println!(
        "{boss}\t{laenge}\t{operator}\t{}\t{unveraendert}\t{:.1}\t{:.3}\t{:.3}\t{:.2}",
        abwandlungen.len(),
        100.0 * gespart as f64 / gesamt.max(1) as f64,
        voll_zeit.as_secs_f64() * 1e3,
        inkrementell_zeit.as_secs_f64() * 1e3,
        voll_zeit.as_secs_f64() / inkrementell_zeit.as_secs_f64()
    );
}
