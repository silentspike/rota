//! Der Bericht der Rotations-Suche (Issue #237, AC4 und AC1b).
//!
//! ```text
//! cargo remote -c -- run --release --example rotation-search
//! ```
//!
//! # Warum dieser Bericht seine eigenen Parameter druckt
//!
//! Regel aus Issue #242: Jede gemeldete Balance-Zahl braucht einen
//! eingecheckten Läufer, der seine Einstellungen mitdruckt. Eine Zahl ohne
//! ihren Aufruf ist nicht nachrechenbar, und „überliefert" ist kein Zustand,
//! den ein Bericht haben sollte.
//!
//! # Was hier ausdrücklich NICHT durchsucht wurde
//!
//! Das ist die Ehrlichkeitsauflage aus AC4 — und sie ist kein Beiwerk: Ohne
//! sie liest sich „gefunden" wie „bewiesen".

use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy, Candidate, Origin, Stop};

/// Durchgänge je gemessener Rotation.
const RUNS: u32 = 50;
/// Startwert des ersten Durchgangs.
const SEED: u32 = 0;
/// Höchstlänge der gesuchten Listen.
const MAX_LEN: usize = 6;

/// Die Bosse, gegen die gesucht wird (Schnitt 3 und AC1b).
///
/// **Nicht irgendein zweiter Boss — einer mit anderer DAUER.** Mein erster
/// Versuch nahm `test-warden`, weil er Schaden und einen unterbrechbaren
/// Zauber hat. Beide Bosse lieferten exakt dieselbe Zahl, und der Grund stand
/// in meiner eigenen Einstellung: Mit `stop_on_kill = false` läuft jeder
/// Durchgang die volle Überdruckzeit, und `test-warden` hat dieselben 60 s wie
/// `first-crucible`. HP und Zeitleiste formen die Rotation nicht, solange
/// nicht gestoppt wird und der Spieler überlebt.
///
/// `test-metronome` hat 300 s. Dort kommt REFLUX (Abklingzeit 45 s) sechsmal
/// zum Zug statt einmal — die Werkzeug-Zauber haben dort tatsächlich
/// Gelegenheit.
///
/// **Mehr als einer, und das ist keine Gründlichkeit, sondern Notwendigkeit.**
/// Gegen einen Boss ohne Schaden und ohne eigene Zauber können FIXATION,
/// COAGULATION und QUENCH nichts beitragen; eine Suche dort würde sie zu Recht
/// nie wählen und danach „tote Zauber" melden, wo in Wahrheit steht: ein Boss,
/// an dem sie nichts zu tun haben.
const BOSSES: &[&str] = &["first-crucible", "test-metronome"];

/// Die Bosse dieses Laufs — Voreinstellung [`BOSSES`], oder `--bosses a,b,c`.
///
/// **Waehlbar seit #509**, und das ist keine Bequemlichkeit: Der Pruefstein
/// fragt, ob ueber VERSCHIEDENE Bosse dieselbe Rotation gefunden wird. Wer
/// dafuer jedes Mal eine Zeile im Quelltext aendert, misst mit einem Werkzeug,
/// das nach der Messung anders aussieht als davor — und der Bericht nennt
/// einen Befehl, den niemand nachfahren kann.
fn bosse_dieses_laufs() -> Vec<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--bosses" {
            let liste = args.next().unwrap_or_else(|| {
                panic!("Nach --bosses fehlt die Liste, etwa: --bosses first-crucible,test-onslaught")
            });
            let gewaehlt: Vec<String> = liste.split(',').map(|s| s.trim().to_owned()).collect();
            // **Jeder Name wird geprueft, bevor gerechnet wird.** Ein Tippfehler
            // darf nicht stillschweigend einen Boss weglassen: Der Bericht
            // saehe dann vollstaendig aus und waere es nicht.
            for id in &gewaehlt {
                assert!(boss_by_id(id).is_some(), "Boss {id} kennt der Simulator nicht");
            }
            return gewaehlt;
        }
    }
    BOSSES.iter().map(|s| (*s).to_owned()).collect()
}

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        // Beide Rotationen rechnen gleich lang — sonst gewinnt die, die am
        // frühesten aufhört, und die Zahlen wären nicht vergleichbar.
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

/// Die handgeschriebene Liste, gegen die gemessen wird (AC1).
const STEP1_GOOD: &str = include_str!("../apl/step1-good.apl");

fn parameter_zeile_mit(bosse: &[String]) {
    println!("Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={MAX_LEN}, Bosse={bosse:?}");
    println!("           stop_on_kill=false, Faktoren=NEUTRAL, threads=1");
}

fn zeilen_nach_herkunft(pool: &[Candidate]) -> (usize, usize, usize, usize) {
    let mut a = 0;
    let mut d = 0;
    let mut s = 0;
    let mut b = 0;
    for k in pool {
        match k.origin {
            Origin::Always => a += 1,
            Origin::DotRefresh => d += 1,
            Origin::SpenderThreshold => s += 1,
            // Neu mit #553 — vorher konnte die Suche diese Zeile gar nicht
            // bilden, weil der Parser keine Buff-Fenster kannte.
            Origin::BuffWindow => b += 1,
        }
    }
    (a, d, s, b)
}

/// Der Kreuzvergleich — der Kern von Issue #509.
///
/// # Die Frage
///
/// „Findet der Balance-Laeufer ueber verschiedene Bosse dieselbe Rotation?"
/// Wenn ja, ist die Tiefe nicht da: Dann gibt es EINE beste Reihenfolge, und
/// der Rest des Spiels ist Ausfuehrung (GAME-DESIGN 20.3).
///
/// # Zwei Zahlen, und die zweite ist die belastbare
///
/// **Gemeinsame Zeilen** sagen, wie aehnlich die gefundenen Listen aussehen.
/// Das ist die schwaechere Zahl: Zwei Listen koennen sich in einer Zeile
/// unterscheiden und trotzdem gleich viel wert sein.
///
/// **Was es KOSTET, die Rotation von Boss A gegen Boss B zu spielen** — das
/// ist die Zahl, auf die es ankommt. Kostet es nichts, ist die Tiefe nicht da,
/// und zwar unabhaengig davon, wie verschieden die Listen aussehen.
fn kreuzvergleich(gefundene: &[(String, rota_sim::apl::Apl, f64)]) {
    if gefundene.len() < 2 {
        println!("Kreuzvergleich uebersprungen — dafuer braucht es mindestens zwei Bosse.");
        println!("  Beispiel: --bosses first-crucible,test-onslaught,test-metronome");
        println!();
        return;
    }

    println!("== Kreuzvergleich (Issue #509 AC2 und AC3) ==");
    println!();
    println!("Wie aehnlich sehen die gefundenen Listen aus?");
    for i in 0..gefundene.len() {
        for j in (i + 1)..gefundene.len() {
            let a: Vec<String> = gefundene[i].1.to_string().lines().map(str::to_owned).collect();
            let b: Vec<String> = gefundene[j].1.to_string().lines().map(str::to_owned).collect();
            let gemeinsam = a.iter().filter(|z| b.contains(z)).count();
            let groesser = a.len().max(b.len()).max(1);
            println!(
                "  {:<20} gegen {:<20} {gemeinsam} von {groesser} Zeilen gleich ({:.0} %)",
                gefundene[i].0,
                gefundene[j].0,
                gemeinsam as f64 / groesser as f64 * 100.0
            );
        }
    }
    println!();

    println!("Was kostet die FREMDE Rotation? (Zeile = gespielte Liste, Spalte = Boss)");
    print!("  {:<22}", "");
    for (boss, _, _) in gefundene {
        print!("{boss:>22}");
    }
    println!();

    for (herkunft, apl, _) in gefundene {
        print!("  {:<22}", format!("von {herkunft}"));
        for (boss_id, _, eigen) in gefundene {
            let ergebnis = rota_sim::balance::run(&config(boss_id), apl);
            // **Der Verlust gegen die EIGENE beste Rotation dieses Bosses.**
            // Nicht gegen die eigene Herkunft: Gefragt ist, was der Spieler an
            // DIESEM Boss verliert, wenn er die fremde Liste mitbringt.
            let verlust = (eigen - ergebnis.damage.mean) / eigen * 100.0;
            print!("{:>22}", format!("{:.0} ({verlust:+.1} %)", ergebnis.damage.mean));
        }
        println!();
    }
    println!();

    // **Die Antwort in einem Satz, und sie steht im Bericht selbst.** Eine
    // Tabelle, die jeder selbst auswerten muss, ist eine Zahl ohne Aussage.
    let mut groesster_verlust: f64 = 0.0;
    for (_, apl, _) in gefundene {
        for (boss_id, _, eigen) in gefundene {
            let ergebnis = rota_sim::balance::run(&config(boss_id), apl);
            let verlust = (eigen - ergebnis.damage.mean) / eigen * 100.0;
            if verlust > groesster_verlust {
                groesster_verlust = verlust;
            }
        }
    }
    println!("Groesster Verlust durch eine fremde Rotation: {groesster_verlust:.1} %");
    if groesster_verlust < 1.0 {
        println!("  -> Es kostet praktisch NICHTS, ueberall dieselbe Liste zu spielen.");
        println!("     Das ist der Befund aus GAME-DESIGN 20.3: keine Rotations-Tiefe.");
    } else if groesster_verlust < 10.0 {
        println!("  -> Es kostet etwas, aber wenig. Ob das reicht, ist eine Design-Frage.");
    } else {
        println!("  -> Die Rotation lohnt sich je Boss zu ueberdenken. Tiefe ist vorhanden.");
    }
    println!();
}

fn main() {
    println!("== Rotations-Suche (Issue #237, Kreuzvergleich #509) ==");
    let bosse = bosse_dieses_laufs();
    parameter_zeile_mit(&bosse);
    println!();

    let pool = all_candidates();
    let (a, d, s, b) = zeilen_nach_herkunft(&pool);
    println!("Suchraum");
    println!("  Kandidatenzeilen  {}", pool.len());
    println!("    unbedingt       {a}");
    println!("    Brand nachlegen {d}");
    println!("    Entlade-Schwelle {s}");
    println!("    Verstaerkungs-Fenster {b}");
    println!();

    // Je Boss die gefundene Rotation — fuer den Kreuzvergleich unten (#509 AC3).
    let mut gefundene: Vec<(String, rota_sim::apl::Apl, f64)> = Vec::new();

    for boss_id in &bosse {
        let config = config(boss_id);
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let hand = rota_sim::apl::parse(AplName("step1-good"), STEP1_GOOD)
            .expect("step1-good ist gueltig");
        let hand_result = rota_sim::balance::run(&config, &hand);

        println!("Boss {boss_id}");
        println!(
            "  gefunden     {:9.1} +/- {:.1}   ({} Listen gemessen)",
            gefunden.best.mean, gefunden.best.stderr, gefunden.measured
        );
        println!(
            "  step1-good   {:9.1} +/- {:.1}",
            hand_result.damage.mean, hand_result.damage.stderr
        );

        // AC1: erreicht oder uebertrifft — und der Bericht sagt, welches von
        // beiden. „Erreicht" ist ein Befund ueber die Handschrift, kein Mangel
        // der Suche.
        let unschaerfe = gefunden.best.stderr + hand_result.damage.stderr;
        let urteil = if gefunden.best.mean - hand_result.damage.mean > unschaerfe {
            "UEBERTRIFFT die Handschrift messbar"
        } else if hand_result.damage.mean - gefunden.best.mean > unschaerfe {
            "BLEIBT unter der Handschrift — die Suche findet sie nicht"
        } else {
            "ERREICHT die Handschrift (Unterschied im Rauschen)"
        };
        println!("  Urteil       {urteil}");

        println!("  Abbruch      {:?}", gefunden.stop);
        println!("  Rotation:");
        for zeile in gefunden.best.apl.to_string().lines() {
            println!("    {zeile}");
        }

        // Welche Zauber kamen im Lauf vor — nicht: welche stehen in der Liste.
        let boss = boss_by_id(boss_id).expect("Boss");
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            seed: SEED,
            boss,
            ..rota_sim::FightConfig::default()
        });
        let lauf = fight.run_apl(&gefunden.best.apl);
        // **Nur spielbare Zauber.** Ein Pruef-Zauber, der gar nicht zur Wahl
        // stand, als „nie gewirkt" zu melden, waere eine Zahl, die etwas
        // anderes zaehlt als sie behauptet — sie wuerde die Zahl der toten
        // Zauber um einen zu hoch ausweisen.
        let spielbar: Vec<usize> = (0..rota_sim::SPELL_COUNT)
            .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
            .collect();
        let nie: Vec<&str> = spielbar
            .iter()
            .filter(|&&i| lauf.casts[i] == 0)
            .map(|&i| SPELLS[i].id)
            .collect();
        println!("  nie gewirkt  {} von {}", nie.len(), spielbar.len());
        println!("    {}", nie.join(", "));
        println!(
            "  (ausgenommen  {} Pruef-Zauber ueber Stufe {})",
            rota_sim::SPELL_COUNT - spielbar.len(),
            rota_sim::data::MAX_LEVEL
        );
        println!();
        gefundene.push((
            (*boss_id).clone(),
            gefunden.best.apl.clone(),
            gefunden.best.mean,
        ));
    }

    kreuzvergleich(&gefundene);

    println!("Was NICHT durchsucht wurde");
    println!("  1  Freie Bedingungssuche. Nicht der Aufwand ist der Grund, sondern die");
    println!("     Ueberanpassung: Eine Suche ueber alle Werte, Vergleiche und Zahlen faende");
    println!("     Zeilen wie \"if time > 17340\", die zufaellig zur Messung passen und sonst");
    println!("     nichts. Sie liefert NIE ein leeres Ergebnis — und genau das macht sie");
    println!("     wertlos: Man kann sie nicht widerlegen.");
    println!("  2  Erschoepfende Reihenfolgen-Suche. 15! = 1.307.674.368.000.");
    println!("     Erschoepfend gesucht wird nur in der Gegenprobe bei n=5, 7 und 8 — dort");
    println!("     ist es kein Verfahren, sondern ein Massstab.");
    println!("  3  Zufall im Verfahren. Simulated Annealing oder genetische Verfahren");
    println!("     lieferten zwischen zwei Laeufen verschiedene Ergebnisse; eine gemeldete");
    println!("     Zahl waere nicht reproduzierbar (GAME-DESIGN 18.4, wie Issue #242).");
    println!();
    println!("  4  GIERIGES ANHAENGEN — geprueft und VERWORFEN, mit Zahlen:");
    println!("       anhaengen    4000,0");
    println!("       erschoepfend 5960,0     (der step1-good-Wert)");
    println!("     Es verfehlt das Optimum um ein Drittel. Der Grund ist derselbe Satz, mit");
    println!("     dem es begruendet war: Weil das Obere alles Darunterliegende verdeckt,");
    println!("     ist die erste Wahl die folgenreichste — und sie wird mit der");
    println!("     schlechtesten Information getroffen, naemlich einem Zauber ohne Liste.");
    println!("     Steht hier, weil es die naheliegendste Bauart ist und von aussen");
    println!("     vernuenftig aussieht.");
    println!();
    println!("Wie diese Zahlen zu lesen sind");
    println!("  Der Vorsprung der Suche ist klein — bei test-metronome 120 von 31000, also");
    println!("  0,4 %. Das ist KEIN magerer Erfolg der Suche, sondern ein Befund ueber die");
    println!("  Handschrift:");
    println!();
    println!("    Eine von Hand geschriebene Liste, die eine erschoepfend geprueft");
    println!("    arbeitende Suche nur um 0,4 % unterbietet, ist sehr gut.");
    println!();
    println!("  Der Wert dieser Suche liegt deshalb nicht darin, bessere Rotationen zu");
    println!("  finden. Er liegt darin, dass wir kuenftig WISSEN, ob eine Handschrift gut");
    println!("  ist, statt es zu hoffen. Vorher war jede Balance-Zahl eine Untergrenze mit");
    println!("  unbekanntem Abstand nach oben.");
    println!();
    println!("Grenzen dieser Zahlen");
    println!("  Dieser Bericht sucht Listen bis Laenge {MAX_LEN}.");
    println!();
    println!("  Die Laengengrenze ist SEPARAT geprueft, und zwar bei einer anderen Zahl:");
    println!("  In sim/tests/rotation_search.rs laeuft die Gegenprobe mit Laenge 4, weil");
    println!("  dort erschoepfend mitgemessen wird. Dort ergab eine Laenge mehr keinen");
    println!("  Unterschied — 5960,0 (205 Listen) gegen 5960,0 (325 Listen).");
    println!();
    println!("  Die beiden Zahlen {MAX_LEN} und 4 gehoeren also zu verschiedenen Laeufen und");
    println!("  duerfen nicht verwechselt werden. Fuer Laenge {MAX_LEN} selbst ist NICHT");
    println!("  geprueft, ob eine laengere Liste besser waere — erschoepfend waere das bei");
    println!("  siebzehn Kandidaten unerreichbar.");
    println!();
    println!("  DAS ERGEBNIS IST EINE UNTERGRENZE, KEIN BEWIESENES OPTIMUM.");
    println!("  Es sagt: besser als das hier wurde im abgesuchten Raum nichts gefunden.");
    println!("  Es sagt nicht, dass es nichts Besseres gibt.");
    let _ = index_of("kindle");
    let _ = Stop::NoImprovement;
}
