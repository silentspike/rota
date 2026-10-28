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
//! Sie läuft **unmittelbar vor und unmittelbar nach** der Kalibrierung des
//! Vollkampfs. Verglichen wird gegen das Mittel beider; liegen die zwei selbst
//! weiter als die Toleranz auseinander, hat sich die Last im Fenster geändert,
//! und das Programm sagt das, statt zu urteilen
//! (`kalibrierung::ac1_vergleich`).
//!
//! # `--kontrolle`
//!
//! Fährt zusätzlich einen Lauf mit **fester** Dauer von zwei Sekunden und
//! stellt seine Streuung neben die des adaptiven. #912 verlangt diesen
//! Vergleich ausdrücklich: Ohne ihn ist nicht belegt, dass die Adaptivität
//! etwas bringt.
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
    ac1_vergleich, bestaetigen, gespeicherte_rate, kalibriere, kontrolllauf_feste_dauer,
    profildatei, profilverzeichnis, schluessel_erheben, Ac1Urteil, Arbeitsart, Bestaetigung,
    HostPerfVector, Profil, RateModel, AC1_TOLERANZ, STREUUNG_MAX, TAKT_DRIFT_MAX,
};
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::verteilung::{verteilung_ueber, Treffer};

/// Der Boss, gegen den gerechnet wird. Fest, damit der Vergleich zwischen
/// Rechnern trägt — eine andere Zeitleiste ist eine andere Arbeit.
const BOSS: &str = "boss-1-1";

/// Wie viele Kämpfe ein Messabschnitt des Vollkampfs umfasst.
///
/// **Ein Paket, keine Gesamtmenge.** Die Zahl der Pakete entscheidet der
/// adaptive Lauf; hier steht nur, wie groß eines ist. 20 000 Kämpfe brauchen
/// auf dem Laptop rund 0,1 Sekunden und auf gonet rund 0,17 — klein genug, dass
/// die Abbruchbedingung feinkörnig greift, groß genug, dass der Aufruf-Aufwand
/// nicht mitgemessen wird.
const KAEMPFE_JE_PAKET: u32 = 20_000;

/// Wie viele Kämpfe der Referenzlauf aus AC1 am Stück rechnet.
///
/// Dieselbe Menge wie das Programm, mit dem die Werte in #912 erhoben sind
/// (`orch-verteilung`: 200 000 Kämpfe, feste Rotation, Aufwärmlauf davor) —
/// zehnmal ein Paket der Kalibrierung, in einem einzigen Aufruf: die Arbeit
/// am Stück und nicht die Kalibrierung selbst.
const REFERENZ_KAEMPFE: u32 = 200_000;

/// Die Referenz aus AC1: [`REFERENZ_KAEMPFE`] Kämpfe am Stück, in Kämpfen je
/// Sekunde.
///
/// Der Aufwärmlauf mit einem Zehntel davor ist der aus `orch-verteilung`;
/// er zählt nicht. Konfiguration und Rotation sind dieselben wie bei der
/// Kalibrierung — eine andere Zeitleiste oder ein anderer Krit wäre eine
/// andere Arbeit.
fn referenzlauf(threads: usize, apl: &Apl) -> f64 {
    let _ = balance::run(&config(threads, REFERENZ_KAEMPFE / 10), apl);
    let cfg = config(threads, REFERENZ_KAEMPFE);
    let t0 = Instant::now();
    let r = balance::run(&cfg, apl);
    let dauer = t0.elapsed().as_secs_f64();
    // `count` ist bei der Reduktion aus #908 immer `iterations` und belegt
    // deshalb nicht, dass gerechnet wurde — das belegt
    // `ein_durchgang_genuegt` weiter unten. Hier sichert er nur, dass die
    // Statistik die angeforderte Menge hat.
    assert_eq!(
        r.damage.count, REFERENZ_KAEMPFE as usize,
        "der Referenzlauf hat nicht die angeforderte Menge"
    );
    f64::from(REFERENZ_KAEMPFE) / dauer
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

/// Misst eine Arbeitsart und druckt die Zeile dazu.
fn miss(art: Arbeitsart, arbeit: impl FnMut() -> f64) -> RateModel {
    let r = kalibriere(arbeit);
    println!(
        "  {:<20} {:>14.1} {:<22} Streuung {:>6.2} %  {:>3} Abschnitte  {:>6} ms  PSI {:>5}  Drift {:>7}  {:?}",
        art.schluessel(),
        r.rate_per_s,
        art.einheit(),
        r.spread * 100.0,
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
    r
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
    println!("  Kaempfe je Paket       {KAEMPFE_JE_PAKET}");
    println!();

    let druck_vorher = psi_some_avg10("/proc/pressure/cpu").unwrap_or(-1.0);
    println!("== Gemessene Raten (AC1, AC3) ==");

    let threads = caps.threads_usable.max(1);
    let cfg_voll = config(threads, KAEMPFE_JE_PAKET);
    // **Die Referenz aus AC1 umschließt die Kalibrierung**: ein Lauf
    // unmittelbar davor, einer unmittelbar danach, dieselbe Arbeit am Stück.
    let referenz_vorher = referenzlauf(threads, &apl);
    let scalar_full_fight = miss(Arbeitsart::ScalarFullFight, || {
        let r = balance::run(&cfg_voll, &apl);
        #[allow(clippy::cast_precision_loss)]
        {
            r.damage.count as f64
        }
    });
    let referenz_nachher = referenzlauf(threads, &apl);
    println!(
        "  {:<20} {:>14.1} Kaempfe/s              vorher, {} Kaempfe am Stueck (AC1)",
        "referenz", referenz_vorher, REFERENZ_KAEMPFE
    );
    println!(
        "  {:<20} {:>14.1} Kaempfe/s              nachher",
        "referenz", referenz_nachher
    );

    // **Kein vektorisierter Pfad im Simulator.** `caps.isa.vektorisierbar()`
    // sagt nur, dass die MASCHINE ihn könnte. Eine Rate hier wäre eine über
    // denselben skalaren Code unter anderem Namen — und die verglichen später
    // zwei Rechner in einer Disziplin, die keiner von beiden fährt.
    let avx_full_fight: Option<RateModel> = None;
    println!(
        "  {:<20} {:>14} — der Simulator hat keinen vektorisierten Vollkampf (ISA koennte: {})",
        Arbeitsart::AvxFullFight.schluessel(),
        "n/v",
        caps.isa.vektorisierbar()
    );

    // Bereichsrechnung: ein Register mit vielen Lücken, dann Abfragen darauf.
    let ledger_eval = miss(Arbeitsart::LedgerEval, || {
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
    });

    // Krit-Dynamikprogrammierung: eine Treffer-Folge durch den DP.
    let ledger_treffer: Vec<Treffer> = (0..60)
        .map(|i| Treffer {
            ohne_krit: 100 + i,
            mit_krit: 200 + i * 2,
        })
        .collect();
    let crit_dp = miss(Arbeitsart::CritDp, || {
        let v = verteilung_ueber(&ledger_treffer, 0.375, &[500, 1000, 2000]);
        let _ = &v;
        60.0
    });

    // Die Suche über Kandidatenzeilen.
    let pool = all_candidates();
    let pool_len = pool.len();
    let cfg_suche = config(threads, 200);
    let branch_and_bound = miss(Arbeitsart::BranchAndBound, || {
        let r = greedy(&cfg_suche, &pool, 4);
        assert!(
            !r.chosen.is_empty(),
            "die Suche fand nichts — Messung ungueltig"
        );
        #[allow(clippy::cast_precision_loss)]
        {
            pool_len as f64
        }
    });

    // Der Ausrüstungsdurchlauf.
    let gear_sweep = miss(Arbeitsart::GearSweep, || {
        // **Eine Million statt 400.** Beim ersten Lauf auf gonet brauchten 400
        // Fälle weniger als eine Millisekunde, bei 20 000 waren es drei — die
        // Rate stand daraufhin bei 32 029 405 Varianten je Sekunde, einer Zahl
        // aus der Uhrauflösung. `ABSCHNITT_MIN` fängt das jetzt, aber ein Paket,
        // das dauernd in den Abbruch läuft, misst gar nichts.
        let v = gear_balance::compare_variants(1_000_000, 3, 3);
        let _ = &v;
        1_000_000.0
    });

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
    if kontrolle {
        println!("== Kontrollprobe AC2: feste zwei Sekunden gegen adaptiv ==");
        let cfg_k = config(threads, KAEMPFE_JE_PAKET);
        let fest = kontrolllauf_feste_dauer(
            || {
                #[allow(clippy::cast_precision_loss)]
                {
                    balance::run(&cfg_k, &apl).damage.count as f64
                }
            },
            Duration::from_secs(2),
        );
        println!(
            "  fest   {:>12.1} Kaempfe/s   Streuung {:>6.2} %  {:>2} Abschnitte  {:>5} ms",
            fest.rate_per_s,
            fest.spread * 100.0,
            fest.samples,
            fest.duration_ms
        );
        println!(
            "  adaptiv{:>12.1} Kaempfe/s   Streuung {:>6.2} %  {:>2} Abschnitte  {:>5} ms",
            scalar_full_fight.rate_per_s,
            scalar_full_fight.spread * 100.0,
            scalar_full_fight.samples,
            scalar_full_fight.duration_ms
        );
        println!();
        let unterschied = (fest.rate_per_s - scalar_full_fight.rate_per_s)
            / scalar_full_fight.rate_per_s.max(1e-9)
            * 100.0;
        println!("  Unterschied der RATEN: {unterschied:+.1} %");
        println!();
        if fest.spread > scalar_full_fight.spread {
            println!(
                "  DIE ADAPTIVITAET BRINGT ETWAS: der feste Lauf streut um Faktor {:.2} staerker.",
                fest.spread / scalar_full_fight.spread.max(1e-9)
            );
        } else {
            println!(
                "  DER FESTE LAUF STREUT WENIGER — UND DAS IST DER BEFUND, NICHT DAS GEGENTEIL.\n\
                 \x20\n\
                 \x20 Eine kurze Messung sieht ruhiger aus, weil sie WENIGER SIEHT. Zwei Sekunden\n\
                 \x20 erfassen einen Ausschnitt, in dem die Maschine zufaellig gleichmaessig lief;\n\
                 \x20 dreissig Sekunden erfassen die Unruhe, die es wirklich gibt.\n\
                 \x20\n\
                 \x20 Die kleine Streuung des festen Laufs ist also KEINE Guete, sondern ihr\n\
                 \x20 Schein — und die Raten unterscheiden sich um die Zahl oben. Wer nach zwei\n\
                 \x20 Sekunden aufhoert, bekommt eine Zahl mit einem Fehlerbalken, der zu klein\n\
                 \x20 ist, und verteilt die Arbeit dauerhaft danach.\n\
                 \x20\n\
                 \x20 Das Issue hat den umgekehrten Ausgang erwartet. Gemessen ist dieser, und er\n\
                 \x20 traegt dieselbe Folgerung staerker: nicht 'kurz streut mehr', sondern 'kurz\n\
                 \x20 kann gar nicht wissen, ob es streut'."
            );
        }
        println!();
    }

    // ----------------------------------------------------------- das Profil
    let vektor = HostPerfVector {
        scalar_full_fight,
        avx_full_fight,
        ledger_eval,
        crit_dp,
        branch_and_bound,
        gear_sweep,
        gpu_bulk: None,
        network_rtt: None,
        network_bandwidth: None,
        current_pressure: druck_nachher.max(0.0),
    };

    let mut caps_gemessen = caps;
    caps_gemessen.calibrated_fights_per_s = vektor.scalar_full_fight.rate_per_s;
    caps_gemessen.calibration_spread = vektor.scalar_full_fight.spread;
    caps_gemessen.measured_at = vektor.scalar_full_fight.measured_at;

    let profil = Profil {
        schluessel,
        caps: caps_gemessen,
        raten: Arbeitsart::alle()
            .into_iter()
            .filter_map(|a| vektor.rate(a).map(|r| (a, *r)))
            .collect(),
    };

    // ------------------------------------------------- Profil, AC4 und AC5
    //
    // **Erst bestaetigen, dann ablegen.** Umgekehrt verglichen wir die frische
    // Messung mit sich selbst, und die Bestaetigung waere immer positiv — der
    // teuerste denkbare Fehler an dieser Stelle, weil er wie ein Erfolg
    // aussieht.
    let verzeichnis = profilverzeichnis();
    let pfad = verzeichnis.join(profildatei(&profil.schluessel));
    println!("== Profil (AC4, AC5) ==");
    println!("  Ablage                 {}", pfad.display());

    let vorher = std::fs::read_to_string(&pfad).ok();
    let mut abgewichen: Vec<Arbeitsart> = Vec::new();
    for art in Arbeitsart::alle() {
        let Some(r) = vektor.rate(art) else { continue };
        if r.samples == 0 {
            println!(
                "  {:<20} NICHT GEMESSEN ({:?}) — kein Vergleich, keine Ablage",
                art.schluessel(),
                r.reason
            );
            continue;
        }
        let gespeichert = vorher.as_deref().and_then(|t| gespeicherte_rate(t, art));
        let urteil = bestaetigen(gespeichert, r);
        if urteil == Bestaetigung::Abweichung {
            abgewichen.push(art);
        }
        let abweichung = gespeichert.map_or(f64::NAN, |g| (r.rate_per_s - g) / g * 100.0);
        println!(
            "  {:<20} {:<12?} gespeichert {:>14}  gemessen {:>14.1}  {:>7}",
            art.schluessel(),
            urteil,
            gespeichert.map_or_else(|| "—".to_owned(), |g| format!("{g:.1}")),
            r.rate_per_s,
            if abweichung.is_nan() {
                "—".to_owned()
            } else {
                format!("{abweichung:+.1} %")
            }
        );
    }

    // **Abgelegt wird JE RATE, und nur wenn sie stabil ist.**
    //
    // Gemessen am 2026-08-22 auf gonet: Der Lauf unter künstlicher Fremdlast
    // überschrieb das gute Profil mit seinen gedrückten Raten — und beim
    // nächsten ruhigen Lauf stand die Bestätigung dann wieder auf
    // `Abweichung`, diesmal nach oben. Eine Ablage, die jede Zahl nimmt,
    // vernichtet genau den Bezug, den sie herstellen soll.
    //
    // **Je Rate und nicht je Lauf**, weil beides vorkommt: Auf gonet ist
    // `crit_dp` stabil und `scalar_full_fight` nicht — der Knoten trägt
    // fremde Maschinen. Wer nur vollständige Läufe ablegt, hätte dort nie ein
    // Profil, und AC4 wäre auf einem der drei Rechner unerreichbar.
    //
    // Die alten Raten bleiben stehen, wo die neuen nichts taugen: Ein alter
    // Wert ist eine Messung von gestern, ein gedrückter ist keine.
    // **Eine abgewichene Rate legt NICHT ab.**
    //
    // AC5 sagt, was bei Abweichung zu tun ist: neu erheben. Die abweichende
    // Zahl ist der BEFUND, nicht die neue Wahrheit — wer sie ablegt, hat den
    // Befund beim nächsten Lauf verloren und die Ursache dazu.
    //
    // Gemessen am 2026-08-22 auf gonet: Unter zwölf Strängen Fremdlast meldete
    // `crit_dp` eine Abweichung von −43,5 Prozent, war dabei aber `Stabil` —
    // gleichmäßiger Druck erzeugt gleichmäßig falsche Raten mit kleiner
    // Streuung. Ohne diese Zeile hätte der Lauf den guten Wert überschrieben.
    let stabile: Vec<(Arbeitsart, RateModel)> = profil
        .raten
        .iter()
        .filter(|(a, r)| r.reason.brauchbar() && !abgewichen.contains(a))
        .copied()
        .collect();
    let mut zum_ablegen = profil.clone();
    zum_ablegen.raten = stabile.clone();
    zum_ablegen.raten.sort_by_key(|(a, _)| a.schluessel());
    let profiltext = zum_ablegen.speichern_mit_vorherigem(vorher.as_deref());
    let uebernommen = Profil::raten_lesen(&profiltext)
        .len()
        .saturating_sub(stabile.len());

    if stabile.is_empty() {
        println!(
            "  NICHT ABGELEGT — keine einzige Rate ist eine Kalibrierung.\n\
             \x20 Ein vorhandenes Profil bleibt unberuehrt."
        );
    } else if std::fs::create_dir_all(&verzeichnis).is_ok()
        && std::fs::write(&pfad, &profiltext).is_ok()
    {
        println!(
            "  abgelegt: {} neue Rate(n), {} aus dem alten Profil uebernommen.",
            stabile.len(),
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

    println!("== AC1: gegen die Referenz, im selben Lauf auf dieser Maschine ==");
    let ac1 = ac1_vergleich(&vektor.scalar_full_fight, referenz_vorher, referenz_nachher);
    println!(
        "  Referenz vorher {referenz_vorher:>12.1}   nachher {referenz_nachher:>12.1}   \
         Mittel {:>12.1}   Spanne {:>6.2} %",
        ac1.referenz,
        ac1.spanne * 100.0
    );
    println!(
        "  Kalibrierung    {:>12.1}   {:?}   Abweichung {:>+7.2} %   Toleranz +-{:.0} %",
        vektor.scalar_full_fight.rate_per_s,
        vektor.scalar_full_fight.reason,
        ac1.abweichung * 100.0,
        AC1_TOLERANZ * 100.0
    );
    match ac1.urteil {
        Ac1Urteil::Erfuellt => println!("  AC1 ERFUELLT."),
        Ac1Urteil::NichtErfuellt => println!(
            "  AC1 NICHT ERFUELLT — die Kalibrierung verfehlt die Arbeit am Stueck\n\
             \x20 unter derselben Last. Das Kriterium wird nicht gesenkt (#912)."
        ),
        Ac1Urteil::LastGewechselt => println!(
            "  AC1 NICHT BEURTEILBAR — die zwei Referenzlaeufe liegen weiter als die\n\
             \x20 Toleranz auseinander: Die Last hat sich im Messfenster geaendert."
        ),
        Ac1Urteil::KeineKalibrierung => println!(
            "  AC1 NICHT BEURTEILBAR — die Rate ist keine Kalibrierung ({:?}) und wird\n\
             \x20 nicht abgelegt. Die Abweichung oben steht zur Auskunft da.",
            vektor.scalar_full_fight.reason
        ),
        Ac1Urteil::Ungueltig => {
            println!("  AC1 NICHT BEURTEILBAR — eine der Zahlen ist keine Rate.")
        }
    }
    println!();

    // ------------------------------------------------------------- das Urteil
    println!("== Urteil ==");
    let mut brauchbar = true;
    for art in Arbeitsart::alle() {
        let Some(r) = vektor.rate(art) else { continue };
        if !r.reason.brauchbar() {
            println!(
                "  {:<20} NICHT BRAUCHBAR: {:?} bei Streuung {:.2} %",
                art.schluessel(),
                r.reason,
                r.spread * 100.0
            );
            brauchbar = false;
        }
    }
    // **Die Fremdlast wird angehalten, bevor gedruckt wird.** Ein Strang, der
    // nach dem Programmende weiterläuft, ist genau die Sorte Prozess, die
    // später jemand anders in seiner Messung findet und nicht zuordnen kann.
    if let Some(flag) = &_last {
        flag.store(false, std::sync::atomic::Ordering::Relaxed);
        println!("  Die Fremdlast ist angehalten.");
    }
    if brauchbar {
        println!(
            "  Alle Raten stabil (Streuung unter {:.0} %).",
            STREUUNG_MAX * 100.0
        );
    } else {
        println!(
            "  MINDESTENS EINE RATE IST KEINE KALIBRIERUNG. Sie steht oben trotzdem —\n\
             \x20 eine verschwiegene Zahl waere schlimmer als eine mit ihrem Grund daneben."
        );
    }
}
