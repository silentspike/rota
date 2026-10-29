//! Dieselbe Rotationssuche mit und ohne analytischen Krit (#920, E11.4d).
//!
//! # Was gemessen wird
//!
//! Über die wertungsfähigen Bosse ([`ScoringSet::all_scored`]) läuft je Boss
//! zweimal dieselbe gierige Suche: einmal so, wie die Suche heute rechnet —
//! mit dem analytischen Weg, wo die Abhängigkeitsmaske ihn zulässt —, einmal in
//! [`ohne_ausintegration`], also über die Stichprobe von [`RUNS`] Kämpfen je
//! Liste. Gezählt werden Listen, gerechnete Kämpfe (vom Zählwerk, also vom Kern
//! und nicht angefordert), die Wege je Liste und die Wanduhr.
//!
//! **Beide Durchläufe in einem Prozess, am Stück**, damit beide dieselbe
//! Maschine unter derselben Last sehen.
//!
//! # Warum PURITY über null
//!
//! Ohne Krit würfelt nichts, die Durchgangswahl nimmt [`Durchgangswahl::EinDurchgang`],
//! und beide Durchläufe wären derselbe. Die Ausrüstung kommt aus der Kurve:
//! die Eintrittsstufe der Instanz (`itemLevelOf(i, 0, 0) = (i − 1) · 6 + 1`,
//! `src/loot/rates.ts`; der Prolog wie Instanz 1) und darauf der gemischte
//! Aufbau aus [`mixed_build`] — dasselbe Budget auf CATALYSIS, PURITY und
//! YIELD. Ein Boss ohne PURITY über null bricht den Lauf ab.
//!
//! # Was die Zahlen nicht sagen
//!
//! Die gefundenen Rotationen müssen nicht dieselben sein. Mit dem analytischen
//! Weg ist der Standardfehler null, und `Measured::clearly_better_than` nimmt
//! jede echte Verbesserung an; die Stichprobe verwirft, was in ihren
//! Fehlerbalken liegt. Deshalb steht je Boss auch, was beide gefundenen Listen
//! auf **demselben** Maßstab erzielen ([`run_erwartet`] mit [`MASSSTAB`]
//! Durchgängen — analytisch, wo der Weg trägt, sonst die Stichprobe).
//!
//! ```text
//! cargo remote -c -- build --release --example analytisch-suche-920
//! target/release/examples/analytisch-suche-920
//! ```

use std::time::Instant;

use rota_sim::analytisch::{ohne_ausintegration, verweigerungen};
use rota_sim::balance::{run_erwartet, BalanceConfig, Durchgangswahl};
use rota_sim::data::Boss;
use rota_sim::factors::NEUTRAL;
use rota_sim::fortschritt::{Weg, Zaehlwerk};
use rota_sim::rotation_search::{all_candidates, greedy, Candidate, SearchResult};
use rota_sim::scoring::ScoringSet;
use rota_sim::sources::{mixed_build, STAT_PURITY};

/// Durchgänge je Liste — dieselben wie in `rotation-search`.
const RUNS: u32 = 50;
const SEED: u32 = 0;
/// Höchstlänge der gesuchten Listen — dieselbe wie in `rotation-search`.
const MAX_LEN: usize = 6;
/// Durchgänge für den gemeinsamen Maßstab der beiden gefundenen Listen.
const MASSSTAB: u32 = 2000;

/// Die Instanznummer eines Bosses; der Prolog zählt als erste.
fn instanz(boss: &Boss) -> u32 {
    boss.instance
        .strip_prefix("instance-")
        .and_then(|n| n.parse().ok())
        .unwrap_or(1)
}

fn config(boss: &'static Boss) -> BalanceConfig {
    let stufe = (instanz(boss) - 1) * 6 + 1;
    let worn = mixed_build(STAT_PURITY, stufe);
    assert!(
        worn.purity > 0 && worn.purity < 1000,
        "{}: PURITY {} auf Stufe {stufe} — dann würfelt nichts, und die Messung \
         vergliche denselben Weg mit sich selbst",
        boss.id,
        worn.purity
    );
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        worn,
        boss,
        ..BalanceConfig::default()
    }
}

/// Eine Suche mit ihrem eigenen Zählwerk.
struct Lauf {
    ergebnis: SearchResult,
    kaempfe: u64,
    analytisch: u64,
    andere_wege: u64,
    verweigert: u64,
    sekunden: f64,
}

fn suche(config: &BalanceConfig, pool: &[Candidate]) -> Lauf {
    let werk = Zaehlwerk::neu(1);
    let verweigert_vorher = verweigerungen();
    let start = Instant::now();
    let ergebnis = {
        let _anmeldung = werk.anmelden();
        greedy(config, pool, MAX_LEN)
    };
    let sekunden = start.elapsed().as_secs_f64();
    let stand = werk.momentaufnahme();
    let analytisch = stand.kandidaten[Weg::Analytisch.index()];
    Lauf {
        ergebnis,
        kaempfe: stand.kaempfe,
        analytisch,
        andere_wege: stand.kandidaten_gesamt() - analytisch,
        verweigert: verweigerungen() - verweigert_vorher,
        sekunden,
    }
}

/// Was eine gefundene Liste auf dem gemeinsamen Maßstab erzielt.
fn auf_dem_massstab(config: &BalanceConfig, lauf: &Lauf) -> (f64, f64, Durchgangswahl) {
    let e = run_erwartet(
        &BalanceConfig {
            iterations: MASSSTAB,
            ..*config
        },
        &lauf.ergebnis.best.apl,
    );
    (e.schaden, e.stderr, e.weg)
}

fn main() {
    let bosse = ScoringSet::all_scored();
    let pool = all_candidates();
    println!("== Rotationssuche mit und ohne analytischen Krit (#920) ==");
    println!(
        "Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={MAX_LEN}, Pool={} Kandidaten, \
         {} wertungsfaehige Bosse, threads=1, stop_on_kill=false",
        pool.len(),
        bosse.len()
    );
    println!(
        "Ausruestung: mixed_build(PURITY, (Instanz - 1) * 6 + 1), Massstab {MASSSTAB} Durchgaenge"
    );
    println!();
    println!(
        "{:<15} {:>7} {:>6}   {:>6} {:>9} {:>4} {:>4} {:>8}   {:>6} {:>9} {:>8}   {:>9} {:>9}",
        "Boss",
        "PURITY",
        "Stufe",
        "Listen",
        "Kaempfe",
        "anl.",
        "verw",
        "Sek.",
        "Listen",
        "Kaempfe",
        "Sek.",
        "Wert mit",
        "Wert ohne"
    );

    let wanduhr = Instant::now();
    let (mut summe_mit, mut summe_ohne) = ((0u64, 0u64, 0f64), (0u64, 0u64, 0f64));
    let mut verweigert = 0;
    let mut bosse_analytisch = 0;
    for boss in bosse.bosses() {
        let cfg = config(boss);
        let mit = suche(&cfg, &pool);
        let ohne = ohne_ausintegration(|| suche(&cfg, &pool));
        assert_eq!(
            ohne.analytisch, 0,
            "{}: abgeschaltet hat die Suche trotzdem analytisch gerechnet",
            boss.id
        );
        let (wert_mit, fehler_mit, weg_mit) = auf_dem_massstab(&cfg, &mit);
        let (wert_ohne, fehler_ohne, _) = auf_dem_massstab(&cfg, &ohne);
        if mit.analytisch > 0 {
            bosse_analytisch += 1;
        }
        verweigert += mit.verweigert;
        summe_mit.0 += mit.ergebnis.measured as u64;
        summe_mit.1 += mit.kaempfe;
        summe_mit.2 += mit.sekunden;
        summe_ohne.0 += ohne.ergebnis.measured as u64;
        summe_ohne.1 += ohne.kaempfe;
        summe_ohne.2 += ohne.sekunden;
        let massstab = match weg_mit {
            Durchgangswahl::Analytisch => "analytisch",
            Durchgangswahl::EinDurchgang => "ein Kampf",
            Durchgangswahl::Gestaffelt => "Stichprobe",
        };
        println!(
            "{:<15} {:>7} {:>6}   {:>6} {:>9} {:>4} {:>4} {:>8.2}   {:>6} {:>9} {:>8.2}   \
             {:>9.1} {:>9.1}  (+/- {:.1} / {:.1}, {massstab}; andere Wege mit: {})",
            boss.id,
            cfg.worn.purity,
            (instanz(boss) - 1) * 6 + 1,
            mit.ergebnis.measured,
            mit.kaempfe,
            mit.analytisch,
            mit.verweigert,
            mit.sekunden,
            ohne.ergebnis.measured,
            ohne.kaempfe,
            ohne.sekunden,
            wert_mit,
            wert_ohne,
            fehler_mit,
            fehler_ohne,
            mit.andere_wege
        );
    }
    println!();
    println!(
        "Summe mit analytischem Weg   {:>7} Listen {:>10} Kaempfe {:>9.2} s",
        summe_mit.0, summe_mit.1, summe_mit.2
    );
    println!(
        "Summe ohne (Stichprobe)      {:>7} Listen {:>10} Kaempfe {:>9.2} s",
        summe_ohne.0, summe_ohne.1, summe_ohne.2
    );
    println!(
        "Verhaeltnis ohne/mit         Kaempfe {:.2}, Wanduhr {:.2}",
        summe_ohne.1 as f64 / summe_mit.1 as f64,
        summe_ohne.2 / summe_mit.2
    );
    println!(
        "Bosse mit analytischem Weg   {bosse_analytisch} von {}",
        bosse.len()
    );
    println!("Verweigerte Randabgleiche    {verweigert}");
    println!(
        "Wanduhr des ganzen Laufs     {:.2} s",
        wanduhr.elapsed().as_secs_f64()
    );
}
