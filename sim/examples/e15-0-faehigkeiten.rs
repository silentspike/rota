//! Was dieser Rechner leistet — je Arbeitsart gemessen (Issue #912, E15.0).
//!
//! ```text
//!   cargo run --release --example e15-0-faehigkeiten
//!   cargo run --release --example e15-0-faehigkeiten -- --kontrolle
//! ```
//!
//! # Was dieses Programm beantwortet
//!
//! Nicht „wie viele Kerne hat die Maschine" — das sagt jedes Werkzeug. Sondern
//! **was sie bei DIESER Arbeit schafft**, mit der Streuung daneben und mit dem
//! Grund, aus dem die Messung aufgehört hat.
//!
//! Jede Zahl steht neben dem Druck, unter dem sie entstand. Eine unruhige
//! Maschine liefert eine Zahl, die nichts bedeutet — und die sieht genauso aus
//! wie eine, die etwas bedeutet.
//!
//! # `--fremdlast N`
//!
//! Belegt N Stränge mit Rechenarbeit, WÄHREND gemessen wird, und zeigt, dass
//! die Neuerhebung dann auslöst (#912, AC5). Ohne Fremdlast muss sie
//! schweigen — die Kontrollprobe dazu steht als Prüffall in
//! `kalibrierung::bestaetigen_loest_nicht_dauernd_aus`.
//!
//! **Die Reihenfolge ist Pflicht:** Erst ein ruhiger Lauf, der ablegt; dann
//! der Lauf unter Last, der die Abweichung meldet. Wer nur den zweiten fährt,
//! hat kein gespeichertes Profil und bekommt `Fehlt` statt `Abweichung`.
//!
//! # AC1: die Referenz läuft im selben Programm
//!
//! AC1 verlangt eine Referenz, die **gleichzeitig auf derselben Maschine**
//! erhoben ist — gleiche Konfiguration, gleicher Zeitraum, gleiche Last. Sie
//! ist die echte Arbeit am Stück, so wie die Werte in #912 entstanden sind:
//! [`REFERENZ_KAEMPFE`] Kämpfe in EINEM Aufruf von `balance::run`, mit
//! derselben Rotation, denselben Strängen und denselben getragenen Werten
//! wie die Kalibrierung, nach einem Aufwärmlauf.
//!
//! Sie läuft **verschränkt** mit der Kalibrierung des Vollkampfs: ein
//! Fenster des durchgehenden Laufs, dann ein Referenzlauf am Stück, wieder
//! ein Fenster, wieder ein Referenzlauf — jedes Fenster so lang wie der
//! letzte Referenzabschnitt, auf denselben Kernen
//! (`kalibrierung::kalibriere_verschraenkt`). Verglichen wird Mittel gegen
//! Mittel (`Verschraenkt::ac1`).
//!
//! # Der Vollkampf wird als durchgehender Lauf kalibriert (Weg b)
//!
//! Entscheidung des Orchestrators vom 28.10.: Die echte Arbeit läuft in
//! langen Abschnitten, nicht in 20k-Happen. Bis dahin kalibrierte dieses
//! Programm mit Paketen zu 20 000 Kämpfen und maß so neben der Rechenrate die
//! Sperre am Ende jedes Pakets mit — auf node3 unter 8 Fremdsträngen
//! 10–13 ms je Paket, und die Kalibrierung lag 12,7 bis 16,6 % unter der
//! Arbeit am Stück. Jetzt misst `balance::strom`: Helfer rechnen ohne Sperre,
//! und die Kampfzahl eines Fensters kommt aus dem Zählwerk. Diese Fenster
//! sind die Abschnitte der Abbruchregel.
//!
//! Bis zum 28.10. lief die Referenz einmal davor und einmal danach. Auf node3
//! lagen die zwei 10,7 % auseinander: Unter dieser Drift trägt ein
//! Vorher/Nachher-Vergleich kein 10-Prozent-Kriterium. Und der Lauf davor
//! schob auf dem belasteten Laptop den Druck so hoch, dass die Kalibrierung
//! danach mit `Druck` abbrach.
//!
//! # `--kontrolle`
//!
//! Fährt zusätzlich den verschränkten Lauf mit **fester** Dauer von zwei
//! Sekunden und fragt, ob die adaptive Messung näher an ihrer Referenz liegt
//! als die verkürzte (`kalibrierung::genauigkeitsprobe`). #912 verlangt diese
//! Kontrolle: Ohne sie ist nicht belegt, dass die Adaptivität etwas bringt.
//! Braucht die volle Erhebung des Vollkampfs, also einen Lauf ohne Profil.
//!
//! # Warum das Programm nicht schreibt, was es misst
//!
//! Es druckt. Das Profil legt der Aufrufer ab — ein Messprogramm, das
//! nebenbei Dateien anlegt, tut zwei Dinge, und beim nächsten Fehler weiß
//! niemand, welches davon schiefging.

use std::time::{Duration, Instant};

use rota_sim::akkumulator::TrialIdSet;
use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::{self, BalanceConfig};
use rota_sim::data::{boss_by_id, EFFECT_NODES};
use rota_sim::durchgaenge::ein_durchgang_genuegt;
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::WornValues;
use rota_sim::gear_balance;
use rota_sim::hostcaps::{drucksignale, erlaubte_kerne, kernvorraete, psi_some_avg10, HostCaps};
use rota_sim::kalibrierung::{
    bestaetigungslauf, erheben_mit, genauigkeitsprobe, gespeicherte_rate, kalibriere,
    kalibriere_verschraenkt, profildatei, profilverzeichnis, schluessel_erheben,
    verschraenkt_feste_dauer, Abschnittswert, Ac1Urteil, Arbeitsart, Bestaetigung, Erhebung,
    HostPerfVector, Lauf, Profil, RateModel, Verschraenkt, AC1_TOLERANZ, STREUUNG_MAX,
    TAKT_DRIFT_MAX,
};
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::verteilung::{verteilung_ueber, Treffer};

/// Der Boss, gegen den gerechnet wird. Fest, damit der Vergleich zwischen
/// Rechnern trägt — eine andere Zeitleiste ist eine andere Arbeit.
const BOSS: &str = "boss-1-1";

/// Wie viele Kämpfe der Referenzlauf aus AC1 am Stück rechnet.
///
/// Dieselbe Menge wie das Programm, mit dem die Werte in #912 erhoben sind
/// (`orch-verteilung`: 200 000 Kämpfe, feste Rotation, Aufwärmlauf davor), in
/// einem einzigen Aufruf: die Arbeit am Stück.
const REFERENZ_KAEMPFE: u32 = 200_000;

/// Wie lang ein Fenster des durchgehenden Laufs im Bestätigungslauf ist —
/// und im verschränkten Lauf, bis der erste Referenzabschnitt seine Dauer
/// vorgibt.
///
/// Eine halbe Sekunde ist auf node3 die Größenordnung eines Referenzlaufs
/// am Stück (200 000 Kämpfe bei rund 400 000 je Sekunde); zehn Fenster
/// füllen die Mindestdauer.
const STROM_FENSTER: Duration = Duration::from_millis(500);

/// Ein Fenster des durchgehenden Laufs als Abschnitt der Kalibrierung: die
/// Kämpfe, die das Zählwerk bis zum Fensterende zählte, und das Fenster
/// selbst.
fn stromfenster(cfg: &BalanceConfig, apl: &Apl, dauer: Duration) -> Abschnittswert {
    let m = balance::strom(cfg, apl, dauer);
    Abschnittswert {
        #[allow(clippy::cast_precision_loss)]
        einheiten: m.kaempfe as f64,
        fenster: Some(m.fenster),
    }
}

/// Ein Referenzabschnitt: [`REFERENZ_KAEMPFE`] Kämpfe in EINEM Aufruf.
///
/// Konfiguration und Rotation sind dieselben wie bei der Kalibrierung — eine
/// andere Zeitleiste oder ein anderer Krit wäre eine andere Arbeit.
fn referenzblock(cfg_stueck: &BalanceConfig, apl: &Apl) -> f64 {
    let r = balance::run(cfg_stueck, apl);
    // `count` ist bei der Reduktion aus #908 immer `iterations` und belegt
    // deshalb nicht, dass gerechnet wurde — das belegt
    // `ein_durchgang_genuegt` weiter unten. Hier sichert er nur, dass die
    // Statistik die angeforderte Menge hat.
    assert_eq!(
        r.damage.count, REFERENZ_KAEMPFE as usize,
        "der Referenzlauf hat nicht die angeforderte Menge"
    );
    f64::from(REFERENZ_KAEMPFE)
}

/// Getragene Sonderwerte mit **echtem** Krit-Zufall.
///
/// # Warum das hier nicht die Vorgabe sein darf
///
/// `balance::run` rechnet bei `WornValues::none()` **einen** Durchgang statt
/// aller: `durchgaenge::ein_durchgang_genuegt` erkennt, dass ohne Zufall jeder
/// Kampf denselben Ausgang hat (Issue #908). Das ist eine richtige Optimierung
/// des Simulators — und sie macht eine Durchsatzmessung wertlos.
///
/// **Gemessen am 2026-08-22 auf gonet**, erster Lauf dieses Programms: 12 183
/// 553 Kämpfe je Sekunde gegen die Referenz von 121 689. Faktor hundert, und
/// die Zahl sah plausibel formatiert aus. Aufgefallen ist es nur, weil die
/// Referenz danebenstand.
///
/// `purity = 375` ist der Wert, den drei der Paritäts-Aufzeichnungen tragen
/// (`worn-values.json`) — also einer, bei dem das Spiel wirklich würfelt.
fn worn_mit_zufall() -> WornValues {
    let mut w = WornValues::none();
    w.purity = 375;
    w
}

fn config(threads: usize, iterations: u32) -> BalanceConfig {
    BalanceConfig {
        iterations,
        base_seed: 5,
        threads,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id(BOSS).expect("Boss fehlt"),
        worn: worn_mit_zufall(),
        ..BalanceConfig::default()
    }
}

/// Eine feste Rotation, damit die Arbeit auf jedem Rechner dieselbe ist.
///
/// `greedy` würde je nach Timing verschieden weit kommen — dann verglichen zwei
/// Rechner zwei verschiedene Arbeiten und die Rate hinge daran, wer schneller
/// suchte. Genau dieser Fehler steht in `examples/orch-verteilung.rs`
/// beschrieben; die Rotation entsteht hier auf demselben Weg.
fn feste_rotation() -> Apl {
    let pool = all_candidates();
    let gefunden = greedy(&config(1, 20), &pool, 5);
    Apl {
        name: "e15-0".to_owned(),
        rules: gefunden
            .chosen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                spell: k.spell,
                condition: k.condition.clone(),
            })
            .collect(),
    }
}

/// Erhebt eine Arbeitsart (AC4, AC5) und druckt, was gelaufen ist: mit
/// Profil den Bestätigungslauf und, wenn er abwich, die volle Erhebung.
fn erhebe(
    art: Arbeitsart,
    gespeichert: Option<f64>,
    messen: impl FnMut(Lauf) -> RateModel,
) -> Erhebung {
    let e = erheben_mit(gespeichert, messen);
    let (name, einheit) = (art.schluessel(), art.einheit());
    match &e {
        Erhebung::Fehlt { voll } => zeile(name, "voll", einheit, voll),
        Erhebung::Bestaetigt { kurz } | Erhebung::NichtGemessen { kurz } => {
            zeile(name, "kurz", einheit, kurz);
        }
        Erhebung::NeuErhoben { kurz, voll, .. } => {
            zeile(name, "kurz", einheit, kurz);
            zeile(name, "voll", einheit, voll);
        }
    }
    e
}

/// [`erhebe`] für eine Arbeit, die kurz wie voll dieselbe ist.
fn erhebe_einfach(
    art: Arbeitsart,
    gespeichert: Option<f64>,
    mut arbeit: impl FnMut() -> f64,
) -> Erhebung {
    erhebe(art, gespeichert, |lauf| match lauf {
        Lauf::Kurz => bestaetigungslauf(&mut arbeit),
        Lauf::Voll => kalibriere(&mut arbeit),
    })
}

/// Eine Zeile je gemessener Rate.
///
/// **Streuung und Unsicherheit stehen beide da**: Die Streuung beschreibt die
/// Abschnitte, die Unsicherheit das Mittel — an ihr hängt `Stabil`
/// (`STREUUNG_MAX`).
fn zeile(name: &str, lauf: &str, einheit: &str, r: &RateModel) {
    println!(
        "  {:<20} {:<4} {:>14.1} {:<22} Streuung {:>6.2} %  Unsicherheit {:>5.2} %  {:>4} Abschnitte  {:>6} ms  PSI {:>5}  Drift {:>7}  {:?}",
        name,
        lauf,
        r.rate_per_s,
        einheit,
        r.spread * 100.0,
        r.unsicherheit() * 100.0,
        r.samples,
        r.duration_ms,
        r.psi_cpu
            .map_or_else(|| "—".to_owned(), |p| format!("{p:.1}%")),
        // **Die Drift wird markiert, nicht bewertet.** Sie bricht seit dem
        // 2026-08-22 nichts mehr ab (die Begründung steht bei
        // `TAKT_DRIFT_MAX`), aber wer eine auffällige Rate erklärt, will sie
        // sehen — und `TAKT_DRIFT_MAX` sagt, ab wann sie auffällig ist.
        r.takt_drift.map_or_else(
            || "—".to_owned(),
            |d| {
                format!(
                    "{:.1}%{}",
                    d * 100.0,
                    if d > TAKT_DRIFT_MAX { "!" } else { " " }
                )
            },
        ),
        r.reason
    );
}

/// Belegt Stränge mit Rechenarbeit, bis das Signal fällt.
///
/// **Eine reine Rechenschleife ohne Speicherzugriff.** Sie soll CPU-Druck
/// erzeugen und nicht Speicherbandbreite — sonst misst die Probe unten zwei
/// Störungen zugleich, und welche gewirkt hat, sagt niemand.
fn fremdlast_starten(straenge: usize) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    let laeuft = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    for _ in 0..straenge {
        let flag = std::sync::Arc::clone(&laeuft);
        std::thread::spawn(move || {
            let mut x = 1u64;
            while flag.load(std::sync::atomic::Ordering::Relaxed) {
                for _ in 0..100_000 {
                    x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                }
                std::hint::black_box(x);
            }
        });
    }
    laeuft
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let kontrolle = argv.iter().any(|a| a == "--kontrolle");
    let fremdlast: usize = argv
        .iter()
        .position(|a| a == "--fremdlast")
        .and_then(|i| argv.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let _last = (fremdlast > 0).then(|| {
        println!("== FREMDLAST: {fremdlast} Straenge rechnen mit (AC5) ==");
        println!("  Die Raten unten sind damit ABSICHTLICH gedrueckt.");
        println!();
        fremdlast_starten(fremdlast)
    });

    // ------------------------------------------------------------- gemeldet
    let caps = HostCaps::erheben();
    // Eine Paketversion identifiziert keinen Bau: zwei verschiedene Binaries
    // können beide 0.1.0 heißen. Das Profil gehört zu den tatsächlichen Bytes.
    let artefakt = std::env::current_exe()
        .and_then(|pfad| {
            let ausgabe = std::process::Command::new("sha256sum").arg(pfad).output()?;
            let text = String::from_utf8_lossy(&ausgabe.stdout);
            let hash = text.split_whitespace().next().unwrap_or("");
            if !ausgabe.status.success()
                || hash.len() != 64
                || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(std::io::Error::other(
                    "Artefakt-Hash konnte nicht erhoben werden",
                ));
            }
            Ok(hash.to_owned())
        })
        .unwrap_or_else(|fehler| {
            eprintln!("Kein gültiger Profilschlüssel: {fehler}");
            std::process::exit(1);
        });
    let schluessel = schluessel_erheben(&artefakt);
    println!("== Was die Maschine MELDET ==");
    println!("  Rechner                {}", schluessel.host);
    println!("  Kernel                 {}", schluessel.kernel);
    println!("  Taktregime             {}", schluessel.taktregime);
    println!("  Straenge gesamt        {}", caps.threads_total);
    println!(
        "  Straenge benutzbar     {}{}",
        caps.threads_usable,
        if caps.gebunden() {
            "   <- GEBUNDEN, der Prozess darf nicht alle"
        } else {
            ""
        }
    );
    println!("  L2 je Kern             {} Bytes", caps.l2_per_core);
    println!("  L3                     {} Bytes", caps.l3);
    println!(
        "  RAM gesamt / frei      {} / {}",
        caps.ram_total, caps.ram_free
    );
    println!(
        "  ISA                    avx {} · avx2 {} · fma {} · avx512 {}",
        caps.isa.avx, caps.isa.avx2, caps.isa.fma, caps.isa.avx512
    );
    println!();

    // ------------------------------------------------------- Kernvorräte AC6
    println!("== Kernvorraete (AC6) ==");
    let erlaubt = erlaubte_kerne().unwrap_or_default();
    match kernvorraete(&erlaubt) {
        None => println!("  keine cpufreq-Angaben — die Klassen sind hier nicht messbar"),
        Some(v) if v.len() == 1 => println!(
            "  EIN Vorrat: {} Kerne, {} kHz, {} B L2 je Kern — homogene Maschine",
            v[0].kerne.len(),
            v[0].max_khz,
            v[0].l2_per_core
        ),
        Some(vorraete) => {
            println!(
                "  {} Vorraete, absteigend nach Hoechsttakt:",
                vorraete.len()
            );
            for (i, v) in vorraete.iter().enumerate() {
                println!(
                    "    {i}  {:>2} Kerne  {:>8} kHz  {:>9} B L2 je Kern  {:?}",
                    v.kerne.len(),
                    v.max_khz,
                    v.l2_per_core,
                    v.kerne
                );
            }
        }
    }
    println!();

    // -------------------------------------------------------------- gemessen
    let apl = feste_rotation();
    println!("== Arbeitspaket ==");
    println!("  Boss                   {BOSS}");
    println!(
        "  Rotation               {} Zeilen aus greedy",
        apl.rules.len()
    );
    println!(
        "  Vollkampf              durchgehender Lauf (balance::strom), Fenster aus dem Zaehlwerk"
    );
    println!(
        "  Referenz (AC1)         {REFERENZ_KAEMPFE} Kaempfe am Stueck, verschraenkt, \
         jedes Fenster so lang wie der letzte Referenzabschnitt"
    );
    println!();

    // **Das Profil wird VOR der Messung gelesen.** Mit Profil läuft je
    // Arbeitsart nur ein kurzer Bestätigungslauf (AC4); erst wenn er um mehr
    // als 25 % abweicht, die volle Erhebung (AC5). Bis zum 28.10. lief immer
    // die volle Erhebung, und das Profil wurde danach nur verglichen — mit
    // Profil auf node3 2:13 gegen 2:39 ohne.
    let verzeichnis = profilverzeichnis();
    let pfad = verzeichnis.join(profildatei(&schluessel));
    let vorher = std::fs::read_to_string(&pfad).ok();
    let gespeichert = |art: Arbeitsart| vorher.as_deref().and_then(|t| gespeicherte_rate(t, art));

    let druck_vorher = psi_some_avg10("/proc/pressure/cpu").unwrap_or(-1.0);
    println!("== Gemessene Raten (AC1, AC3, AC4) ==");
    println!(
        "  Profil                 {}",
        if vorher.is_some() {
            "liegt vor — je Art ein Bestaetigungslauf, voll nur bei Abweichung"
        } else {
            "fehlt — volle Erhebung"
        }
    );
    let beginn = Instant::now();

    let threads = caps.threads_usable.max(1);
    let cfg_strom = config(threads, REFERENZ_KAEMPFE);
    let cfg_stueck = config(threads, REFERENZ_KAEMPFE);
    // **Die volle Erhebung des Vollkampfs IST der verschränkte Lauf** mit der
    // Referenz aus AC1: Fenster des durchgehenden Laufs, Lauf am Stück,
    // Fenster, Lauf … Jedes Fenster ist so lang wie der Referenzabschnitt vor
    // ihm, damit eine Drift beide Reihen gleich trifft. Der Bestätigungslauf
    // misst dieselben Fenster, eine halbe Sekunde lang.
    let fenster = std::cell::Cell::new(STROM_FENSTER);
    let mut kalibrierabschnitt = || stromfenster(&cfg_strom, &apl, fenster.get());
    let mut referenzabschnitt = || {
        let t = Instant::now();
        let n = referenzblock(&cfg_stueck, &apl);
        fenster.set(t.elapsed());
        n
    };
    let mut ac1_lauf: Option<Verschraenkt> = None;
    let scalar = erhebe(
        Arbeitsart::ScalarFullFight,
        gespeichert(Arbeitsart::ScalarFullFight),
        |lauf| match lauf {
            Lauf::Kurz => bestaetigungslauf(|| stromfenster(&cfg_strom, &apl, STROM_FENSTER)),
            Lauf::Voll => {
                let v = kalibriere_verschraenkt(&mut kalibrierabschnitt, &mut referenzabschnitt);
                ac1_lauf = Some(v);
                v.kalibrierung
            }
        },
    );
    if let Some(v) = &ac1_lauf {
        zeile("referenz (AC1)", "voll", "Kaempfe/s am Stueck", &v.referenz);
    }

    // **Kein vektorisierter Pfad im Simulator.** `caps.isa.vektorisierbar()`
    // sagt nur, dass die MASCHINE ihn könnte. Eine Rate hier wäre eine über
    // denselben skalaren Code unter anderem Namen — und die verglichen später
    // zwei Rechner in einer Disziplin, die keiner von beiden fährt.
    let avx_full_fight: Option<RateModel> = None;
    println!(
        "  {:<20}       {:>14} — der Simulator hat keinen vektorisierten Vollkampf (ISA koennte: {})",
        Arbeitsart::AvxFullFight.schluessel(),
        "n/v",
        caps.isa.vektorisierbar()
    );

    // Bereichsrechnung: ein Register mit vielen Lücken, dann Abfragen darauf.
    let ledger = erhebe_einfach(
        Arbeitsart::LedgerEval,
        gespeichert(Arbeitsart::LedgerEval),
        || {
            let mut menge = TrialIdSet::leer();
            for i in 0..2000u32 {
                menge.vereinige(&TrialIdSet::aus_bereich(i * 4, i * 4 + 2));
            }
            let mut treffer = 0u64;
            for i in 0..20_000u32 {
                if menge.enthaelt(i) {
                    treffer += 1;
                }
            }
            // Der Rückgabewert darf nicht wegoptimiert werden dürfen: Ohne die
            // Verwendung von `treffer` stünde hier eine Rate für eine Schleife,
            // die nichts tut.
            assert!(
                treffer > 0,
                "die Bereichsabfrage traf nie — Messung ungueltig"
            );
            20_000.0
        },
    );

    // Krit-Dynamikprogrammierung: eine Treffer-Folge durch den DP.
    let ledger_treffer: Vec<Treffer> = (0..60)
        .map(|i| Treffer {
            ohne_krit: 100 + i,
            mit_krit: 200 + i * 2,
        })
        .collect();
    let crit = erhebe_einfach(Arbeitsart::CritDp, gespeichert(Arbeitsart::CritDp), || {
        let v = verteilung_ueber(&ledger_treffer, 0.375, &[500, 1000, 2000]);
        let _ = &v;
        60.0
    });

    // Die Suche über Kandidatenzeilen.
    let pool = all_candidates();
    let pool_len = pool.len();
    let cfg_suche = config(threads, 200);
    let suche = erhebe_einfach(
        Arbeitsart::BranchAndBound,
        gespeichert(Arbeitsart::BranchAndBound),
        || {
            let r = greedy(&cfg_suche, &pool, 4);
            assert!(
                !r.chosen.is_empty(),
                "die Suche fand nichts — Messung ungueltig"
            );
            #[allow(clippy::cast_precision_loss)]
            {
                pool_len as f64
            }
        },
    );

    // Der Ausrüstungsdurchlauf.
    let gear = erhebe_einfach(
        Arbeitsart::GearSweep,
        gespeichert(Arbeitsart::GearSweep),
        || {
            // **Eine Million statt 400.** Beim ersten Lauf auf gonet brauchten 400
            // Fälle weniger als eine Millisekunde, bei 20 000 waren es drei — die
            // Rate stand daraufhin bei 32 029 405 Varianten je Sekunde, einer Zahl
            // aus der Uhrauflösung. `ABSCHNITT_MIN` fängt das jetzt, aber ein Paket,
            // das dauernd in den Abbruch läuft, misst gar nichts.
            let v = gear_balance::compare_variants(1_000_000, 3, 3);
            let _ = &v;
            1_000_000.0
        },
    );
    let dauer_erhebung = beginn.elapsed();

    let druck_nachher = psi_some_avg10("/proc/pressure/cpu").unwrap_or(-1.0);
    println!();
    println!("== Druck waehrend der Messung ==");
    println!("  PSI cpu some avg10     vorher {druck_vorher:.2} %  nachher {druck_nachher:.2} %");
    for s in drucksignale() {
        println!(
            "  {:<22} {:>6.2} %  Grenze {:>5.2} %  {}{}",
            s.name,
            s.wert,
            s.grenze,
            s.quelle,
            if s.ueberschritten() {
                "   UEBERSCHRITTEN"
            } else {
                ""
            }
        );
    }
    println!();

    // -------------------------------------------------- Kontrollprobe zu AC2
    //
    // **Genauigkeit, nicht Streuung** (Entscheidung des Orchestrators vom
    // 28.10.): Eine auf zwei Sekunden verkürzte Messung sieht unter Drift
    // ruhiger aus, weil sie weniger sieht. Gefragt wird deshalb, welche der
    // zwei näher an ihrer verschränkten Referenz liegt — beide verschränkt,
    // mit denselben Fenstern und Läufen am Stück.
    if kontrolle {
        println!("== Kontrollprobe AC2: verschraenkt fest 2 s gegen verschraenkt adaptiv ==");
        match ac1_lauf {
            None => println!(
                "  Nicht moeglich: keine volle Erhebung des Vollkampfs in diesem Lauf\n\
                 \x20 (Profil bestaetigt). Ohne Profil fahren."
            ),
            Some(adaptiv) => {
                let fest = verschraenkt_feste_dauer(
                    &mut kalibrierabschnitt,
                    &mut referenzabschnitt,
                    Duration::from_secs(2),
                );
                for (name, v) in [("adaptiv", &adaptiv), ("fest 2 s", &fest)] {
                    println!(
                        "  {name:<9} Kalibrierung {:>12.1}  Referenz {:>12.1}  Streuung {:>6.2} / {:>6.2} %  \
                         {:>3} Paare  {:>6} ms  {:?}",
                        v.kalibrierung.rate_per_s,
                        v.referenz.rate_per_s,
                        v.kalibrierung.spread * 100.0,
                        v.referenz.spread * 100.0,
                        v.kalibrierung.samples,
                        v.kalibrierung.duration_ms,
                        v.kalibrierung.reason
                    );
                }
                match genauigkeitsprobe(&adaptiv, &fest) {
                    None => println!("  NICHT BEURTEILBAR — eine der vier Zahlen ist keine Rate."),
                    Some(p) => {
                        println!(
                            "  Abstand zur eigenen Referenz: adaptiv {:.2} %, fest 2 s {:.2} %",
                            p.fehler_adaptiv * 100.0,
                            p.fehler_fest * 100.0
                        );
                        if p.adaptiv_genauer {
                            println!("  DIE ADAPTIVITAET BRINGT ETWAS: die adaptive Messung liegt naeher.");
                        } else {
                            println!(
                                "  BEFUND GEGEN DIE ADAPTIVITAET: die verkuerzte Messung liegt nicht\n\
                                 \x20 weiter weg. Die Zahlen oben sind der Beleg."
                            );
                        }
                    }
                }
            }
        }
        println!();
    }

    // ----------------------------------------------------------- das Profil
    let erhebungen = [
        (Arbeitsart::ScalarFullFight, scalar),
        (Arbeitsart::LedgerEval, ledger),
        (Arbeitsart::CritDp, crit),
        (Arbeitsart::BranchAndBound, suche),
        (Arbeitsart::GearSweep, gear),
    ];
    let vektor = HostPerfVector {
        scalar_full_fight: *scalar.rate(),
        avx_full_fight,
        ledger_eval: *ledger.rate(),
        crit_dp: *crit.rate(),
        branch_and_bound: *suche.rate(),
        gear_sweep: *gear.rate(),
        gpu_bulk: None,
        network_rtt: None,
        network_bandwidth: None,
        current_pressure: druck_nachher.max(0.0),
    };

    let mut caps_gemessen = caps;
    caps_gemessen.calibrated_fights_per_s = vektor.scalar_full_fight.rate_per_s;
    caps_gemessen.calibration_spread = vektor.scalar_full_fight.spread;
    caps_gemessen.measured_at = vektor.scalar_full_fight.measured_at;

    println!("== Profil (AC4, AC5) ==");
    println!("  Ablage                 {}", pfad.display());
    for (art, e) in &erhebungen {
        let g = gespeichert(*art);
        let gegen = |r: &RateModel| {
            g.filter(|_| r.samples > 0).map_or_else(
                || "—".to_owned(),
                |g| format!("{:+.1} %", (r.rate_per_s - g) / g * 100.0),
            )
        };
        let (was, kurz, voll) = match e {
            Erhebung::Fehlt { voll } => ("Fehlt".to_owned(), None, Some(voll)),
            Erhebung::Bestaetigt { kurz } => ("Bestaetigt".to_owned(), Some(kurz), None),
            Erhebung::NeuErhoben { kurz, voll, urteil } => {
                (format!("NeuErhoben/{urteil:?}"), Some(kurz), Some(voll))
            }
            Erhebung::NichtGemessen { kurz } => ("NichtGemessen".to_owned(), Some(kurz), None),
        };
        println!(
            "  {:<20} {:<22} gespeichert {:>14}  kurz {:>14} {:>8}  voll {:>14} {:>8}",
            art.schluessel(),
            was,
            g.map_or_else(|| "—".to_owned(), |g| format!("{g:.1}")),
            kurz.map_or_else(|| "—".to_owned(), |r| format!("{:.1}", r.rate_per_s)),
            kurz.map_or_else(|| "—".to_owned(), gegen),
            voll.map_or_else(|| "—".to_owned(), |r| format!("{:.1}", r.rate_per_s)),
            voll.map_or_else(|| "—".to_owned(), gegen),
        );
    }

    // **Abgelegt wird JE RATE, und nur eine stabile volle Erhebung ohne
    // Abweichung** (`Erhebung::abzulegen`). Gemessen am 2026-08-22 auf gonet:
    // Ein Lauf unter Fremdlast überschrieb das gute Profil mit gedrückten
    // Raten, und beim nächsten ruhigen Lauf stand die Bestätigung wieder auf
    // `Abweichung`. Die alten Raten bleiben stehen, wo die neuen nichts
    // taugen — ein alter Wert ist eine Messung von gestern, ein gedrückter
    // ist keine.
    let mut neue: Vec<(Arbeitsart, RateModel)> = erhebungen
        .iter()
        .filter_map(|(a, e)| e.abzulegen().map(|r| (*a, *r)))
        .collect();
    neue.sort_by_key(|(a, _)| a.schluessel());
    let zum_ablegen = Profil {
        schluessel,
        caps: caps_gemessen,
        raten: neue.clone(),
    };
    let profiltext = zum_ablegen.speichern_mit_vorherigem(vorher.as_deref());
    let uebernommen = Profil::raten_lesen(&profiltext)
        .len()
        .saturating_sub(neue.len());

    if neue.is_empty() {
        println!(
            "  NICHTS NEU ABGELEGT — keine Rate dieses Laufs ist eine neue Kalibrierung.\n\
             \x20 Ein vorhandenes Profil bleibt unberuehrt."
        );
    } else if std::fs::create_dir_all(&verzeichnis).is_ok()
        && std::fs::write(&pfad, &profiltext).is_ok()
    {
        println!(
            "  abgelegt: {} neue Rate(n), {} aus dem alten Profil uebernommen.",
            neue.len(),
            uebernommen
        );
    } else {
        println!("  NICHT ABGELEGT — das Verzeichnis liess sich nicht anlegen oder beschreiben.");
    }
    println!();
    println!("  Der Inhalt:");
    for zeile in profiltext.lines() {
        println!("    {zeile}");
    }
    println!();

    // -------------------------------------------------------- AC1: Referenz
    //
    // **Der Beleg dafür, dass Kalibrierung und Referenz echte Kämpfe
    // rechnen**, steht hier und nicht in einer Nachricht: Beide fahren
    // `worn_mit_zufall()`, und die Reduktion aus #908 greift dort nicht. Die
    // Abfrage kostet Millisekunden und beantwortet die Frage, die den ganzen
    // Vergleich trägt — mit `WornValues::none()` daneben als Gegenprobe.
    // **`damage.count` beantwortet die Frage NICHT**, und das ist der zweite
    // Anlauf dieser Stelle: Die Reduktion aus #908 rechnet einen Kampf und
    // VERVIELFACHT ihn — `(0..iterations).map(|i| Sample { seed: .., ..muster })`
    // in `balance.rs`. Die Statistik bekommt dieselbe Zahl von Werten wie ohne
    // Reduktion, also ist `count` immer `iterations`.
    //
    // Eine Zusicherung darauf konnte nie scheitern. Gefragt wird deshalb die
    // Funktion, an der die Entscheidung wirklich hängt.
    let ein_durchgang = ein_durchgang_genuegt(EFFECT_NODES, &WornValues::none());
    let ein_durchgang_hier = ein_durchgang_genuegt(EFFECT_NODES, &worn_mit_zufall());
    println!("== Ob Kalibrierung und Referenz echte Kaempfe rechnen ==");
    println!(
        "  mit WornValues::none()   ein_durchgang_genuegt = {ein_durchgang}{}",
        if ein_durchgang {
            "   <- EIN Kampf statt aller, dann vervielfacht"
        } else {
            "   <- alle Kaempfe werden gerechnet"
        }
    );
    println!(
        "  mit purity 375           ein_durchgang_genuegt = {ein_durchgang_hier}{}",
        if ein_durchgang_hier {
            "   <- Kalibrierung und Referenz oben waeren wertlos"
        } else {
            "   <- Kalibrierung und Referenz oben rechnen echte Kaempfe"
        }
    );
    println!(
        "  orch-verteilung faehrt `none()` und ist deshalb seit #908 keine\n\
         \x20 Durchsatzmessung mehr; es sagt das selbst. Die Referenz unten faehrt\n\
         \x20 dieselben getragenen Werte wie die Kalibrierung."
    );
    println!();

    println!("== AC1: gegen die Referenz, verschraenkt im selben Lauf ==");
    if let Some(ac1_lauf) = ac1_lauf {
        let ac1 = ac1_lauf.ac1();
        println!(
            "  Referenz am Stueck {:>12.1}   Unsicherheit {:>5.2} %   {} Abschnitte",
            ac1_lauf.referenz.rate_per_s,
            ac1_lauf.referenz.unsicherheit() * 100.0,
            ac1_lauf.referenz.samples
        );
        println!(
            "  Kalibrierung       {:>12.1}   Unsicherheit {:>5.2} %   {} Abschnitte   {:?}",
            ac1_lauf.kalibrierung.rate_per_s,
            ac1_lauf.kalibrierung.unsicherheit() * 100.0,
            ac1_lauf.kalibrierung.samples,
            ac1_lauf.kalibrierung.reason
        );
        println!(
            "  Abweichung         {:>+12.2} %   Toleranz +-{:.0} %",
            ac1.abweichung * 100.0,
            AC1_TOLERANZ * 100.0
        );
        match ac1.urteil {
            Ac1Urteil::Erfuellt => println!("  AC1 ERFUELLT."),
            Ac1Urteil::NichtErfuellt => println!(
                "  AC1 NICHT ERFUELLT — die Kalibrierung verfehlt die Arbeit am Stueck\n\
             \x20 unter derselben Last. Das Kriterium wird nicht gesenkt (#912)."
            ),
            Ac1Urteil::KeineKalibrierung => println!(
                "  AC1 NICHT BEURTEILBAR — mindestens eine Reihe ist nicht stabil ({:?}),\n\
             \x20 ihr Mittel nicht genau genug. Die Abweichung oben steht zur Auskunft da.",
                ac1_lauf.kalibrierung.reason
            ),
            Ac1Urteil::Ungueltig => {
                println!("  AC1 NICHT BEURTEILBAR — eine der Zahlen ist keine Rate.")
            }
        }
    } else {
        println!(
            "  In diesem Lauf nicht gemessen: Das Profil wurde bestaetigt, ohne volle\n\
             \x20 Erhebung. Fuer AC1 ohne Profil fahren (ROTA_PROFILVERZEICHNIS leer)."
        );
    }
    println!();

    // ------------------------------------------------------------- das Urteil
    println!("== Urteil ==");
    let mut alles_gut = true;
    for (art, e) in &erhebungen {
        match e {
            Erhebung::Bestaetigt { .. } => {
                println!("  {:<20} bestaetigt — das Profil gilt", art.schluessel());
            }
            Erhebung::NichtGemessen { kurz } => {
                println!(
                    "  {:<20} NICHT GEMESSEN ({:?}) — weder bestaetigt noch neu erhoben",
                    art.schluessel(),
                    kurz.reason
                );
                alles_gut = false;
            }
            Erhebung::Fehlt { voll } | Erhebung::NeuErhoben { voll, .. }
                if !voll.reason.brauchbar() =>
            {
                println!(
                    "  {:<20} NICHT STABIL: {:?} bei Unsicherheit {:.2} %",
                    art.schluessel(),
                    voll.reason,
                    voll.unsicherheit() * 100.0
                );
                alles_gut = false;
            }
            Erhebung::NeuErhoben {
                urteil: Bestaetigung::Abweichung,
                ..
            } => {
                println!(
                    "  {:<20} ABWEICHUNG — neu erhoben, die Rate weicht weiter ab und wird \
                     nicht abgelegt",
                    art.schluessel()
                );
                alles_gut = false;
            }
            Erhebung::Fehlt { .. } | Erhebung::NeuErhoben { .. } => {
                println!("  {:<20} kalibriert", art.schluessel());
            }
        }
    }
    println!(
        "  Dauer der Erhebung     {} ms  ({})",
        dauer_erhebung.as_millis(),
        if vorher.is_some() {
            "mit Profil"
        } else {
            "ohne Profil"
        }
    );
    // **Die Fremdlast wird angehalten, bevor gedruckt wird.** Ein Strang, der
    // nach dem Programmende weiterläuft, ist genau die Sorte Prozess, die
    // später jemand anders in seiner Messung findet und nicht zuordnen kann.
    if let Some(flag) = &_last {
        flag.store(false, std::sync::atomic::Ordering::Relaxed);
        println!("  Die Fremdlast ist angehalten.");
    }
    if alles_gut {
        println!(
            "  Jede Rate ist kalibriert oder bestaetigt (Unsicherheit unter {:.0} %).",
            STREUUNG_MAX * 100.0
        );
    } else {
        println!(
            "  MINDESTENS EINE RATE IST WEDER KALIBRIERT NOCH BESTAETIGT. Sie steht oben\n\
             \x20 trotzdem — eine verschwiegene Zahl waere schlimmer als eine mit ihrem Grund."
        );
    }
}
