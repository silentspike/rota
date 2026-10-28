//! Wie gut findet die Rotationssuche das Optimum? (Auftrag 7886)
//!
//! # Die Frage
//!
//! `greedy` baut eine Rotation Zeile für Zeile und bewertet jede **einzeln**.
//! Ob es damit das Optimum trifft, war an genau EINEM Datenpunkt bekannt:
//! 5960,0 gegen 5960,0 bei 325 Listen, mit rund einem Fünfunddreißigstel der
//! Messungen. Ein Fall ist keine Aussage.
//!
//! Dieser Läufer stellt drei Verfahren über viele **kleine** Suchprobleme
//! gegeneinander, die erschöpfend lösbar sind — dort ist das Optimum bekannt
//! und nicht geschätzt.
//!
//! | Verfahren | was es tut |
//! | --- | --- |
//! | `greedy` ohne Paare | `greedy_opts(…, mit_paaren = false)` — die Suche vor #558 |
//! | `greedy` mit Paaren | `greedy_opts(…, mit_paaren = true)` — die heutige |
//! | erschöpfend | `exhaustive` — der Bezugspunkt |
//!
//! # Warum die Fälle KLEIN sein müssen
//!
//! Die erschöpfende Suche prüft jede Anordnung ohne Wiederholung. Bei neun
//! Kandidaten und Länge 4 sind das 3609 Listen, bei sechs und Länge 3 nur 156.
//! Der volle Raum (93 Kandidaten, Länge 6) läge bei über 600 Milliarden — dort
//! gibt es kein bekanntes Optimum, gegen das sich messen liesse. Genau deshalb
//! wird hier verkleinert, statt den echten Raum abzusuchen.
//!
//! # Warum die Fälle DETERMINISTISCH erzeugt werden
//!
//! Kein Zufallszahlengeber, auch keiner mit festem Startwert: Die Teilmenge
//! folgt aus dem Fall-Index über feste Schrittweiten. Ein zweiter Lauf bringt
//! dieselben Fälle, und wer einen einzelnen nachstellen will, braucht nur
//! seine Nummer.
//!
//! # Was `measure` hier bedeutet
//!
//! `iterations = 1` und fester Startwert — die Vorgabe von `exhaustive`
//! selbst. Jede Rotation ist damit exakt reproduzierbar, und ein Unterschied
//! im Ergebnis ist einer im Verfahren und nicht in der Streuung.
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{
    all_candidates, boss_kandidaten, exhaustive, greedy_opts, Candidate,
};
use rota_sim::SPELLS;

/// Die Bosse, über die die Fälle streuen.
///
/// Vier echte Instanz-Bosse mit verschiedenen Zeitleisten. Ein einziger Boss
/// würde die Frage auf eine Kampfform verengen — und ob `greedy` daneben
/// liegt, hängt daran, welche Zeilen überhaupt zusammenwirken.
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];

/// Wie viele Fälle erzeugt werden.
const FAELLE: usize = 60;

/// Fester Startwert für jede Messung.
const SEED: u32 = 0;

/// Durchgänge je Rotation — einer, siehe Dateikopf.
const RUNS: u32 = 1;

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

/// Ein Suchproblem: Boss, Kandidaten-Teilmenge, Länge.
struct Fall {
    nummer: usize,
    boss: &'static str,
    pool: Vec<Candidate>,
    max_len: usize,
}

/// Erzeugt die Fälle **deterministisch** aus dem Index.
///
/// Drei Grössen streuen, und alle drei aus derselben Zahl:
///
/// | Grösse | Regel | Werte |
/// | --- | --- | --- |
/// | Boss | `i % 4` | die vier der ersten Instanz |
/// | Länge | `3 + (i / 4) % 2` | 3 oder 4 |
/// | Kandidaten | `6 + (i * 3) % 4` | 6 bis 9 |
///
/// Die Teilmenge selbst wird ab einem Versatz in fester Schrittweite aus dem
/// vollen Kandidatenraum gezogen — **ohne den Erzeuger anzufassen**. Das war
/// die offene Frage im Auftrag: `all_candidates()` liefert einen `Vec`, und
/// eine Teilmenge daraus braucht keinen Eingriff.
fn faelle() -> Vec<Fall> {
    let voll = all_candidates();
    assert!(
        voll.len() >= 20,
        "der Kandidatenraum hat nur {} Zeilen — die Teilmengen waeren dann \
         kaum verschieden, und die Streuung ueber die Faelle waere keine",
        voll.len()
    );

    (0..FAELLE)
        .map(|i| {
            let anzahl = 6 + (i * 3) % 4;
            let max_len = 3 + (i / BOSSE.len()) % 2;
            let versatz = (i * 17) % voll.len();
            // Schrittweite teilerfremd zur Poolgrösse zu halten, ist nicht
            // nötig: Ein Zyklus, der sich schliesst, liefert einfach weniger
            // verschiedene Zeilen — und das fängt die Zusicherung unten.
            let schritt = 1 + (i * 5) % 7;
            let mut pool: Vec<Candidate> = Vec::new();
            let mut k = versatz;
            while pool.len() < anzahl {
                let kandidat = voll[k % voll.len()].clone();
                if !pool.contains(&kandidat) {
                    pool.push(kandidat);
                }
                k += schritt;
                // Ist der Zyklus zu kurz, um `anzahl` verschiedene Zeilen zu
                // liefern, rückt der Startpunkt weiter.
                if k > versatz + voll.len() * schritt {
                    k = versatz + pool.len() + 1;
                }
            }
            Fall {
                nummer: i,
                boss: BOSSE[i % BOSSE.len()],
                pool,
                max_len,
            }
        })
        .collect()
}

/// Der Raum, den `greedy` an diesem Boss **tatsächlich** durchsucht (#1088).
///
/// Seit #1088 legt `greedy_zuege` an einem doom-Boss für jeden Zauber im
/// Vorrat, der doom senkt, eine bedingte Zeile dazu ([`boss_kandidaten`]).
/// Der Bezugspunkt muss denselben Raum absuchen. Sonst ist „Optimum" das
/// eines kleineren Raums, und ein `greedy`, das mit der ergänzten Zeile
/// darüber hinauskäme, würde hier als Verfehlen gebucht.
///
/// Gemessen am 2026-10-28 (#1041): Das betrifft 3 der 60 Fälle (`boss-1-2`,
/// `purge`), und in keinem ändert die Zeile das Optimum — geändert hat sich
/// nur die Messungszahl der erschöpfenden Suche. In `suchguete-erweitert`
/// brach derselbe Unterschied eine Zusicherung; dort steht der Beleg.
///
/// `greedy` selbst bekommt weiter den Vorrat: Es ergänzt selbst, und mit
/// diesem Raum stünde die Zeile doppelt zur Wahl.
fn suchraum(config: &BalanceConfig, pool: &[Candidate]) -> Vec<Candidate> {
    let mut raum = pool.to_vec();
    raum.extend(boss_kandidaten(config, pool));
    raum
}

/// Was ein Verfahren über alle Fälle geleistet hat.
#[derive(Default)]
struct Bilanz {
    treffer: usize,
    regret_summe: f64,
    regret_max: f64,
    messungen: usize,
    schlechtester_fall: Option<usize>,
}

impl Bilanz {
    fn buchen(&mut self, fall: usize, gefunden: f64, optimum: f64, messungen: usize) {
        self.messungen += messungen;
        // **Gleichheit auf Bit-Ebene, nicht mit Toleranz.** Beide Verfahren
        // messen dieselbe Rotation mit demselben Startwert; ein Unterschied
        // ist einer in der Auswahl, nicht im Rechnen. Eine Toleranz würde
        // genau die Fälle verstecken, um die es hier geht.
        if (gefunden - optimum).abs() < f64::EPSILON {
            self.treffer += 1;
            return;
        }
        let regret = if optimum > 0.0 {
            (optimum - gefunden) / optimum
        } else {
            0.0
        };
        self.regret_summe += regret;
        if regret > self.regret_max {
            self.regret_max = regret;
            self.schlechtester_fall = Some(fall);
        }
    }

    fn zeile(&self, name: &str, faelle: usize) {
        let quote = 100.0 * self.treffer as f64 / faelle as f64;
        let verfehlt = faelle - self.treffer;
        let regret_mittel = if verfehlt > 0 {
            100.0 * self.regret_summe / verfehlt as f64
        } else {
            0.0
        };
        // Der schlechteste Fall steht MIT seiner Nummer da, nicht nur mit
        // seinem Wert: Wer ihn nachstellen will, braucht die Nummer — die
        // Fallmenge ist deterministisch, also genuegt sie.
        let schlimmster = match self.schlechtester_fall {
            Some(nr) => format!("Fall {nr}"),
            None => "—".to_owned(),
        };
        println!(
            "  {name:<22} {:>3} / {faelle}  {quote:>5.1} %   {regret_mittel:>6.2} %  \
             {:>6.2} %   {:>9}   {schlimmster}",
            self.treffer,
            100.0 * self.regret_max,
            self.messungen
        );
    }
}

fn main() {
    let liste = faelle();

    println!("== Suchguete gegen exakt loesbare Raeume (Auftrag 7886) ==");
    println!();
    println!("  Faelle            {FAELLE}, deterministisch aus dem Index erzeugt");
    println!("  Bosse             {}", BOSSE.join(", "));
    println!("  Kandidaten je Fall 6 bis 9, Laenge 3 oder 4");
    println!("  Startwert         {SEED}");
    println!("  Durchgaenge        {RUNS} je Rotation (Vorgabe von `exhaustive`)");
    println!();

    let mut ohne_paare = Bilanz::default();
    let mut mit_paaren = Bilanz::default();
    let mut erschoepfend = Bilanz::default();
    let mut verfehlt: Vec<(usize, &'static str, usize, usize, f64, f64)> = Vec::new();
    // Die Zahl, die zeigt, dass `suchraum` hier ueberhaupt etwas aendert.
    let mut faelle_ergaenzt = 0usize;
    let mut namen_ergaenzt: Vec<String> = Vec::new();

    for fall in &liste {
        let cfg = config(fall.boss);
        // Der Bezugspunkt sucht in greedys Raum, `greedy` bekommt den Vorrat
        // und ergaenzt selbst — siehe `suchraum`.
        let raum = suchraum(&cfg, &fall.pool);
        let exakt = exhaustive(&cfg, &raum, fall.max_len);
        let optimum = exakt.best.mean;

        let ohne = greedy_opts(&cfg, &fall.pool, fall.max_len, &[], false);
        let mit = greedy_opts(&cfg, &fall.pool, fall.max_len, &[], true);

        // Gleicher Raum, an greedys eigener Meldung geprueft statt angenommen.
        let ergaenzt_hier: Vec<String> = raum[fall.pool.len()..]
            .iter()
            .map(|k| SPELLS[k.spell].id.to_string())
            .collect();
        assert_eq!(
            ohne.ergaenzt, ergaenzt_hier,
            "Fall {}: greedy ergaenzte {:?}, `suchraum` {:?} — dann sucht der Bezugspunkt \
             nicht in greedys Raum",
            fall.nummer, ohne.ergaenzt, ergaenzt_hier
        );
        if !ergaenzt_hier.is_empty() {
            faelle_ergaenzt += 1;
            for name in ergaenzt_hier {
                if !namen_ergaenzt.contains(&name) {
                    namen_ergaenzt.push(name);
                }
            }
        }

        erschoepfend.buchen(fall.nummer, optimum, optimum, exakt.measured);
        ohne_paare.buchen(fall.nummer, ohne.best.mean, optimum, ohne.measured);
        mit_paaren.buchen(fall.nummer, mit.best.mean, optimum, mit.measured);

        if (mit.best.mean - optimum).abs() >= f64::EPSILON {
            verfehlt.push((
                fall.nummer,
                fall.boss,
                fall.pool.len(),
                fall.max_len,
                mit.best.mean,
                optimum,
            ));
        }
    }

    println!(
        "  Verfahren               Treffer   Quote   Regret-M  Regret-Max   Messungen   schlechtester"
    );
    println!(
        "  -----------------------------------------------------------------------------------------"
    );
    ohne_paare.zeile("greedy ohne Paare", FAELLE);
    mit_paaren.zeile("greedy mit Paaren", FAELLE);
    erschoepfend.zeile("erschoepfend", FAELLE);
    println!();
    println!(
        "  Suchraum ueber den Vorrat hinaus (#1088): {faelle_ergaenzt} von {FAELLE} Faellen, \
         Zauber {namen_ergaenzt:?}"
    );
    println!();
    println!("  Regret-M ist das Mittel ueber die VERFEHLTEN Faelle, nicht ueber alle —");
    println!("  sonst verduennen die Treffer die Zahl und sie sinkt, je besser das");
    println!("  Verfahren wird.");
    println!();

    if verfehlt.is_empty() {
        println!("  KEIN Fall, in dem `greedy` mit Paaren das Optimum verfehlt.");
        println!(
            "  Abgesucht: {FAELLE} Faelle, {} bis {} Kandidaten, Laenge 3 bis 4, vier Bosse.",
            6, 9
        );
        println!("  Das ist ein Ergebnis, keine Fehlanzeige — aber es gilt NUR fuer diesen");
        println!("  Bereich. Der echte Suchraum ist um Groessenordnungen weiter.");
    } else {
        println!("  Faelle, in denen `greedy` mit Paaren das Optimum VERFEHLT:");
        for (nr, boss, pool, len, gefunden, optimum) in &verfehlt {
            println!(
                "    Fall {nr:>2}  {boss:<10} {pool} Kandidaten, Laenge {len}   \
                 gefunden {gefunden:.1}  Optimum {optimum:.1}  \
                 Regret {:.2} %",
                100.0 * (optimum - gefunden) / optimum
            );
        }
    }

    println!();
    let differenz = mit_paaren.treffer as i64 - ohne_paare.treffer as i64;
    println!("  HEBT DIE PAAR-BILDUNG DIE TREFFERQUOTE?");
    match differenz {
        0 => println!(
            "    Nein — beide treffen {} von {FAELLE}. In diesem Bereich macht sie \
             keinen Unterschied.",
            mit_paaren.treffer
        ),
        d if d > 0 => println!(
            "    Ja — {} von {FAELLE} gegen {} ohne Paare, also {d} Faelle mehr.",
            mit_paaren.treffer, ohne_paare.treffer
        ),
        d => println!(
            "    Sie SENKT sie — {} von {FAELLE} gegen {} ohne Paare, also {} weniger. \
             Das gehoert angesehen.",
            mit_paaren.treffer, ohne_paare.treffer, -d
        ),
    }
    println!(
        "    Kosten: {} Messungen mit Paaren gegen {} ohne.",
        mit_paaren.messungen, ohne_paare.messungen
    );

    // **Die Zusicherung, ohne die die ganze Tabelle nichts sagt.** Läuft die
    // erschöpfende Suche nicht mehr Messungen als `greedy`, dann hat sie den
    // Raum nicht abgesucht — und „Optimum" wäre nur ein zweiter Greedy-Lauf
    // unter anderem Namen.
    assert!(
        erschoepfend.messungen > mit_paaren.messungen,
        "die erschoepfende Suche machte {} Messungen, `greedy` {} — dann hat sie \
         den Raum nicht abgesucht und das „Optimum“ ist keines",
        erschoepfend.messungen,
        mit_paaren.messungen
    );
}
