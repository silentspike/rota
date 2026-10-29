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
//! # Ausgabe zum Weiterverarbeiten
//!
//! Je Boss und Durchlauf eine Zeile `ZEILE <durchlauf> <boss> …` mit Listen,
//! Kämpfen, Wegen, Wanduhr, dem Wert der gefundenen Liste in voller Genauigkeit
//! und der Liste selbst (`rotation=`, Zeilen durch ` | ` getrennt), dazu je
//! eine Zeile `MASSSTAB …`. Mit `--bewerte <datei>` liest das Programm die
//! `ZEILE`-Einträge einer anderen Messung — etwa desselben Laufs auf
//! `origin/main` — und misst deren Listen auf dem Maßstab dieses Stands nach.
//!
//! ```text
//! cargo remote -c -- build --release --example analytisch-suche-920
//! target/release/examples/analytisch-suche-920
//! target/release/examples/analytisch-suche-920 --bewerte main.txt
//! ```

use std::time::Instant;

use rota_sim::analytisch::{ohne_ausintegration, verweigerungen};
use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run_erwartet, BalanceConfig, Durchgangswahl};
use rota_sim::data::{boss_by_id, Boss};
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

/// Was eine Liste auf dem gemeinsamen Maßstab erzielt.
fn auf_dem_massstab(config: &BalanceConfig, apl: &Apl) -> (f64, f64, &'static str) {
    let e = run_erwartet(
        &BalanceConfig {
            iterations: MASSSTAB,
            ..*config
        },
        apl,
    );
    let weg = match e.weg {
        Durchgangswahl::Analytisch => "analytisch",
        Durchgangswahl::EinDurchgang => "ein-kampf",
        Durchgangswahl::Gestaffelt => "stichprobe",
    };
    (e.schaden, e.stderr, weg)
}

/// Die Liste als eine Zeile: ihre Regeln, durch ` | ` getrennt.
fn als_zeile(apl: &Apl) -> String {
    apl.rules
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn berichte(durchlauf: &str, boss: &Boss, lauf: &Lauf, config: &BalanceConfig) {
    let best = &lauf.ergebnis.best;
    println!(
        "ZEILE {durchlauf} {} listen={} kaempfe={} analytisch={} andere={} verweigert={} \
         sekunden={:.3} wert={:?} fehler={:?} rotation={}",
        boss.id,
        lauf.ergebnis.measured,
        lauf.kaempfe,
        lauf.analytisch,
        lauf.andere_wege,
        lauf.verweigert,
        lauf.sekunden,
        best.mean,
        best.stderr,
        als_zeile(&best.apl)
    );
    let (wert, fehler, weg) = auf_dem_massstab(config, &best.apl);
    println!(
        "MASSSTAB {durchlauf} {} wert={wert:.2} fehler={fehler:.2} weg={weg}",
        boss.id
    );
}

/// `--bewerte <datei>`: die Listen einer anderen Messung auf diesem Maßstab.
///
/// Die Liste wird aus ihrer Textform zurückgelesen und gegen sie gehalten:
/// Stimmt die Textform des Gelesenen nicht mit der Zeile überein, bricht der
/// Lauf ab, statt eine andere Liste zu bewerten.
fn bewerte(datei: &str) {
    let text =
        std::fs::read_to_string(datei).unwrap_or_else(|f| panic!("{datei} nicht lesbar: {f}"));
    let mut gezaehlt = 0;
    for zeile in text.lines().filter(|z| z.starts_with("ZEILE ")) {
        let mut teile = zeile.splitn(4, ' ');
        let (_, durchlauf, boss_id) = (teile.next(), teile.next(), teile.next());
        let durchlauf = durchlauf.expect("Durchlauf");
        let boss_id = boss_id.expect("Boss");
        let rotation = zeile
            .split_once(" rotation=")
            .map(|(_, r)| r)
            .expect("rotation=");
        let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
        let apl = parse(AplName("bewertet"), &rotation.replace(" | ", "\n"))
            .unwrap_or_else(|f| panic!("{boss_id}: Liste nicht lesbar: {f}"));
        assert_eq!(
            als_zeile(&apl),
            rotation,
            "{boss_id}: die zurückgelesene Liste ist eine andere"
        );
        let (wert, fehler, weg) = auf_dem_massstab(&config(boss), &apl);
        println!(
            "MASSSTAB {durchlauf}-auf-diesem-stand {boss_id} wert={wert:.2} fehler={fehler:.2} weg={weg}"
        );
        gezaehlt += 1;
    }
    assert!(gezaehlt > 0, "{datei} enthält keine ZEILE");
    println!("bewertet: {gezaehlt} Listen aus {datei}");
}

fn main() {
    let argumente: Vec<String> = std::env::args().collect();
    if let Some(stelle) = argumente.iter().position(|a| a == "--bewerte") {
        let datei = argumente
            .get(stelle + 1)
            .expect("nach --bewerte fehlt die Datei");
        bewerte(datei);
        return;
    }

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

    let wanduhr = Instant::now();
    let (mut summe_mit, mut summe_ohne) = ((0u64, 0u64, 0f64), (0u64, 0u64, 0f64));
    let mut verweigert = 0;
    let mut bosse_analytisch = 0;
    let mut gleiche_listen = 0;
    for boss in bosse.bosses() {
        let cfg = config(boss);
        println!(
            "BOSS {} purity={} stufe={}",
            boss.id,
            cfg.worn.purity,
            (instanz(boss) - 1) * 6 + 1
        );
        let mit = suche(&cfg, &pool);
        let ohne = ohne_ausintegration(|| suche(&cfg, &pool));
        assert_eq!(
            ohne.analytisch, 0,
            "{}: abgeschaltet hat die Suche trotzdem analytisch gerechnet",
            boss.id
        );
        berichte("mit", boss, &mit, &cfg);
        berichte("ohne", boss, &ohne, &cfg);
        let gleich = als_zeile(&mit.ergebnis.best.apl) == als_zeile(&ohne.ergebnis.best.apl);
        println!(
            "LISTE {} {}",
            boss.id,
            if gleich { "gleich" } else { "VERSCHIEDEN" }
        );
        gleiche_listen += usize::from(gleich);
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
    println!(
        "Gleiche Liste mit und ohne   {gleiche_listen} von {}",
        bosse.len()
    );
    println!("Verweigerte Randabgleiche    {verweigert}");
    println!(
        "Wanduhr des ganzen Laufs     {:.2} s",
        wanduhr.elapsed().as_secs_f64()
    );
}
