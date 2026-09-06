//! Wählt der Balance-Läufer einen FIXATION-Zauber, wenn ein Schlag
//! unabwendbar ist — und lässt er ihn draußen, wenn nicht? (#797, AC5)
//!
//! # Warum ein Rust-Prüffall und kein Aufruf von `rota-sim run`
//!
//! `rota-sim run --search greedy` baut seine Konfiguration mit
//! `..BalanceConfig::default()`, und dort steht `stop_on_kill: true`
//! (`balance.rs`). Mit dieser Stellung endet ein Durchgang beim Sieg — tötet
//! die gefundene Rotation den Boss vor dem Schlag, **fällt er nie**, FIXATION
//! ist nicht nötig, und AC5 wäre rot aus dem falschen Grund.
//!
//! Hier steht die Stellung deshalb ausdrücklich auf `false`. Das ist die
//! Frage, die AC5 stellt: nicht „fällt der Boss", sondern „lohnt sich Können"
//! (`BalanceConfig`, Feld `stop_on_kill`).
//!
//! # Die zwei Bosse sind Zwillinge
//!
//! `test-lethal-strike` und `test-survivable-strike` unterscheiden sich in
//! **genau einer Zahl**: dem Schaden des Schlags (18400 gegen 4600). Alles
//! andere ist identisch. Nur deshalb ist ein Unterschied im Ergebnis dem
//! Schlag zuzuordnen — mit einem beliebigen anderen Boss als Kontrolle wäre er
//! es nicht.

use rota_sim::apl::Apl;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy, SearchResult};
use rota_sim::{Fight, FightConfig};

/// Der Boss, an dem der Schlag unabwendbar ist (18400 gegen eine gemessene
/// Obergrenze von 9200).
const TOEDLICH: &str = "test-lethal-strike";

/// Sein Zwilling — derselbe Boss, Schlag auf 4600 gesenkt.
const UEBERLEBBAR: &str = "test-survivable-strike";

/// Wie lang die gesuchte Liste werden darf.
///
/// Groß genug, dass FIXATION nicht bloß an der Längengrenze scheitert:
/// `SearchResult::paare_geprueft` trägt die Warnung, dass bei `max_len = 6`
/// die Liste voll ist, bevor die Einzelzüge aufhören zu tragen.
const MAX_LEN: usize = 10;

/// Kleinster positiver Wert der am 2026-08-21 gemessenen Reihe aus #797:
/// 1, 2, 5, 10, 20, 50 und 100 trennten die Zwillinge (PIN + BIND nur am
/// tödlichen Boss), 0 nicht. Die ganze Reihe bleibt unten als Messwerkzeug.
const PREVENTED_WEIGHT: f64 = 1.0;

fn konfiguration(boss_id: &str, prevented_weight: f64) -> BalanceConfig {
    BalanceConfig {
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt im Datensatz")),
        stop_on_kill: false,
        prevented_weight,
        ..BalanceConfig::default()
    }
}

/// Führt die Suche gegen einen Boss und gibt das Ergebnis zurück.
///
/// `prevented_weight` ist **kein Beiwerk**: Acht von elf FIXATION-Zaubern
/// tragen `baseDamage: 0` und kosten eine globale Wartezeit. Steht das Gewicht
/// auf 0, sind sie in jeder Runde der schlechteste Zug, und die Suche *kann*
/// sie nicht wählen — unabhängig davon, was sie verhindern.
fn suche_gegen(boss_id: &str, prevented_weight: f64) -> SearchResult {
    let config = konfiguration(boss_id, prevented_weight);
    greedy(&config, &all_candidates(), MAX_LEN)
}

/// Dieselbe Umgebung wie bei der Suche, insbesondere dieselbe Abbruchregel.
/// Der Nachlauf liest tatsächliche Treffer und Unterbrechungen; eine
/// `kill_time_ms`-Statistik sagt nicht, ob der Spieler den Schlag erlebt hat.
fn nachlauf(boss_id: &str, apl: &Apl) -> Fight {
    let config = konfiguration(boss_id, PREVENTED_WEIGHT);
    let mut fight = Fight::new(FightConfig {
        boss: config.boss,
        seed: config.base_seed,
        duration_ms: config.duration_ms,
        stop_on_kill: config.stop_on_kill,
        factors: config.factors,
        worn: config.worn,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    fight
}

/// Die FIXATION-Zauber in einer gefundenen Rotation, beim Namen.
fn fixation_zeilen(ergebnis: &SearchResult) -> Vec<&'static str> {
    ergebnis
        .chosen
        .iter()
        .filter(|k| SPELLS[k.spell].school == Some("FIXATION"))
        .map(|k| SPELLS[k.spell].id)
        .collect()
}

/// **Träger-Zusicherung, und sie steht bewusst vorn.**
///
/// Alles Weitere misst, ob die Suche auf den tödlichen Schlag reagiert. Wäre
/// der Kampf vorbei, bevor der Schlag fällt, wäre jede folgende Zahl richtig
/// gemessen und würde etwas anderes belegen: „FIXATION nicht gewählt" wäre
/// nicht von „der Schlag fiel nie" zu unterscheiden.
///
/// Sie ist zugleich der Wächter über `stop_on_kill`: Stellt jemand künftig auf
/// `true` zurück, wird dieser Fall **rot** statt stillschweigend bedeutungslos.
///
/// Ohne Spieleraktionen trifft der Schlag in beiden Kämpfen, aber nur der
/// größere tötet. Der Zeitpunkt wird aus der Zeitleiste gelesen, nicht in
/// eine Suchbedingung geschrieben.
#[test]
fn ac5_der_schlag_faellt_ueberhaupt() {
    let leer = Apl {
        name: "ac5-ohne-eingriff".to_owned(),
        rules: Vec::new(),
    };
    let toedlich = nachlauf(TOEDLICH, &leer);
    let zwilling = nachlauf(UEBERLEBBAR, &leer);
    for (boss_id, fight) in [(TOEDLICH, &toedlich), (UEBERLEBBAR, &zwilling)] {
        let boss = boss_by_id(boss_id).expect("Prüfboss fehlt");
        assert_eq!(boss.timeline.len(), 1, "{boss_id}: kein einzelner Schlag");
        assert!(
            fight.time() >= boss.timeline[0].at_ms && fight.damage_taken() > 0,
            "{boss_id}: Schlag nicht erlebt (Zeit {}, Schaden {})",
            fight.time(),
            fight.damage_taken()
        );
        assert_eq!(fight.interrupts_landed(), 0);
    }
    assert!(
        toedlich.player_dead(),
        "der ungeschützte Spieler überlebt den tödlichen Schlag"
    );
    assert!(
        !zwilling.player_dead(),
        "der Kontrollschlag tötet bereits ohne Suchentscheidung"
    );
}

/// AC5: Am Boss mit dem unabwendbaren Schlag wählt die Suche FIXATION.
///
/// Die gefundenen Kontrollzeilen müssen im Nachlauf tatsächlich den Schlag
/// abwenden. Werden ausschließlich diese Zeilen entfernt, tötet er wieder.
/// So genügt kein unbenutzter FIXATION-Eintrag in einer ansonsten siegreichen
/// Liste; die Suche muss die Fähigkeit gefunden haben.
#[test]
fn ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist() {
    let gefunden = suche_gegen(TOEDLICH, PREVENTED_WEIGHT);
    let fixation = fixation_zeilen(&gefunden);
    assert!(
        !fixation.is_empty(),
        "Gewicht {PREVENTED_WEIGHT}: keine FIXATION-Zeile am tödlichen Boss"
    );
    let mit = nachlauf(TOEDLICH, &gefunden.best.apl);
    assert!(
        !mit.player_dead() && mit.interrupts_landed() > 0,
        "FIXATION {fixation:?} wendet den Schlag nicht ab: tot={}, Unterbrechungen={}",
        mit.player_dead(),
        mit.interrupts_landed()
    );

    let mut ohne_fixation = gefunden.best.apl.clone();
    ohne_fixation
        .rules
        .retain(|regel| SPELLS[regel.spell].school != Some("FIXATION"));
    assert!(
        !ohne_fixation.rules.is_empty(),
        "die Gegenprobe muss die übrige gefundene Rotation behalten"
    );
    let ohne = nachlauf(TOEDLICH, &ohne_fixation);
    assert!(
        ohne.player_dead() && ohne.damage_taken() > 0,
        "ohne FIXATION überlebt dieselbe Rotation — die Wahl belegt keine nötige Kontrolle"
    );
    println!(
        "Gewicht {PREVENTED_WEIGHT}, {TOEDLICH}: FIXATION {fixation:?}, \
         Unterbrechungen {}, tot mit/ohne FIXATION {}/{}",
        mit.interrupts_landed(),
        mit.player_dead(),
        ohne.player_dead()
    );
}

/// **Die Träger-Zusicherung aus AC5**: ohne den Schlag bleibt FIXATION draußen.
///
/// Ohne diesen Fall wäre nicht belegt, dass die **Wahl am Schlag hängt**. Sie
/// könnte auch daran liegen, dass das Gewicht hoch genug ist, um FIXATION
/// überall zu ziehen — dann wäre die Suche geändert und nicht die Fähigkeit,
/// und genau davor warnt die Auflage im Issue.
#[test]
fn ac5_ohne_den_schlag_bleibt_fixation_draussen() {
    let gefunden = suche_gegen(UEBERLEBBAR, PREVENTED_WEIGHT);
    assert!(
        !gefunden.chosen.is_empty(),
        "die Suche findet am Zwilling keine Rotation"
    );
    assert!(
        fixation_zeilen(&gefunden).is_empty(),
        "Gewicht {PREVENTED_WEIGHT}: FIXATION wird auch ohne tödlichen Schlag gewählt"
    );
    let fight = nachlauf(UEBERLEBBAR, &gefunden.best.apl);
    assert!(
        !fight.player_dead(),
        "die gefundene Kontrollrotation stirbt"
    );
    assert!(
        fight.damage_taken() + fight.shield_absorbed() > 0,
        "der Zwilling hat nie getroffen — Überleben allein wäre keine Gegenprobe"
    );
    println!(
        "Gewicht {PREVENTED_WEIGHT}, {UEBERLEBBAR}: FIXATION [], \
         Schaden {}, absorbiert {}, tot {}",
        fight.damage_taken(),
        fight.shield_absorbed(),
        fight.player_dead()
    );
}

/// Sucht das Gewichtsfenster und **gibt die ganze Reihe aus**, nicht nur den
/// Treffer.
///
/// # Warum die Reihe und nicht eine Zahl
///
/// Eine Zahl, die als einzige dasteht, sieht aus wie gewählt. Eine Reihe zeigt,
/// dass sie gefunden wurde — und wo die Grenzen liegen.
///
/// # Warum das Fenster das eigentliche Ergebnis von AC5 ist
///
/// Ein Gewicht hat zwei falsche Richtungen:
///
/// * **zu niedrig** — FIXATION wird nie gewählt, AC5 rot ohne Aussage
/// * **zu hoch** — FIXATION wird überall gewählt, auch am Zwilling. Dann hängt
///   die Wahl am Gewicht statt am Schlag, und der Nachweis belegt nichts.
///
/// Dass es überhaupt einen Bereich dazwischen gibt, ist die Aussage. Gibt es
/// keinen, ist **das** der Befund — und dann wird nicht am Boss gedreht, bis es
/// passt. Das ist die Auflage aus #797, wörtlich: „Dann hätte ich die Messung
/// gebaut, nicht die Fähigkeit."
#[test]
#[ignore = "Messwerkzeug: sucht das Gewichtsfenster, sichert nichts zu"]
fn die_gewichtsreihe_zeigt_ein_fenster() {
    let gewichte = [0.0, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];
    let mut fenster = Vec::new();

    println!("\n  Gewicht |  toedlich (18400)                | Zwilling (4600)");
    println!("  --------+----------------------------------+------------------");
    for w in gewichte {
        let t = fixation_zeilen(&suche_gegen(TOEDLICH, w));
        let z = fixation_zeilen(&suche_gegen(UEBERLEBBAR, w));
        let trennt = !t.is_empty() && z.is_empty();
        if trennt {
            fenster.push(w);
        }
        println!(
            "  {w:>7.1} | {:<32} | {:<16} {}",
            format!("{t:?}"),
            format!("{z:?}"),
            if trennt { "<- trennt" } else { "" }
        );
    }

    println!("\n  Gewichte, die die beiden Bosse trennen: {fenster:?}");
    if fenster.is_empty() {
        println!(
            "  BEFUND: KEIN Gewicht trennt sie. Die Suche unterscheidet nicht \
             zwischen einem toedlichen und einem harmlosen Schlag — das ist \
             eine Aussage ueber die Faehigkeit, die AC5 prueft, und kein \
             Grund, am Boss zu drehen."
        );

        // **Diese Reihe ist grob, und das gehoert zum Befund dazu.** Zwischen
        // 2 und 5 liegt Platz fuer ein schmales Fenster, das hier durchfaellt.
        // „Kein Gewicht trennt sie" heisst deshalb genau: keines DIESER acht.
        //
        // Bevor daraus ein Befund ueber die Faehigkeit wird, gehoert dort
        // feiner gemessen, wo die Spalten KIPPEN — also zwischen dem letzten
        // Gewicht, bei dem beide Bosse FIXATION weglassen, und dem ersten, bei
        // dem beide es waehlen. Springt es ohne Zwischenschritt von „keiner"
        // auf „beide", gibt es dort nichts zu finden; liegt eine Stufe
        // dazwischen, ist sie das Fenster.
        println!(
            "  ACHTUNG: Diese Reihe ist grob. Vor einem Befund ueber die \
             Faehigkeit zwischen den Werten nachmessen, an denen die Spalten \
             kippen — sonst heisst 'kein Fenster' nur 'keines dieser acht'."
        );
    }
}
