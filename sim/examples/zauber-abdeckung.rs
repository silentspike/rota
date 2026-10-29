//! Wächter über die Zauber-Abdeckung der ersten Instanz (Issue #550, AC5).
//!
//! # Was er hält, und warum es eine eigene Datei ist
//!
//! `rotation-search` misst dieselbe Zahl und **gibt sie aus** — mehr nicht. Ein
//! Nightly-Schritt, der ihn aufruft, wäre grün, egal welche Zahl herauskommt:
//! Der Läufer endet mit 0, solange er nicht abstürzt. Dieser Wächter vergleicht
//! die Zahl gegen eine Schwelle und endet mit 1, wenn sie unterschritten wird.
//!
//! # Die Zahl
//!
//! Über die **vier Bosse der ersten Instanz** zusammen: Wie viele der
//! spielbaren Zauber werden in mindestens einer besten Rotation überhaupt
//! gewirkt? Gemessen am 2026-08-14: **11 von 58**.
//!
//! Das ist die Schnittmengen-Frage aus GAME-DESIGN Leitplanke 2 („jeder Zauber
//! muss irgendwo die beste Wahl sein"), nicht die Summe der Einzelzahlen: Ein
//! Zauber, der nur gegen `boss-1-1` gewirkt wird, zählt hier mit.
//!
//! # Warum eine UNTERGRENZE und keine feste Zahl
//!
//! Steigt die Abdeckung, ist das der Zweck des Issues — dann soll der Wächter
//! nicht rot werden. Fällt sie, ist etwas verlorengegangen: ein Zauber, der
//! seine Nische hatte, oder ein Boss, der sie erzwang. Genau das soll auffallen.
//!
//! **Wer die Schwelle anhebt, muss die neue Zahl gemessen haben** — sie steht
//! hier als Konstante und nirgends sonst.
//!
//! # Nischen, nicht Suchwege (Variante B, Entscheidung vom 2026-10-29)
//!
//! Je Boss zählt die bessere von zwei Listen: was die freie Suche findet, und
//! was sie von der **Bestliste vom 2026-08-14** aus erreicht (`greedy_from`,
//! dieselbe Besser-Prüfung wie der Nachlauf in `rotation-search`).
//!
//! **Warum.** Bis zum 2026-10-29 zählte nur die freie Suche, und der Wächter
//! wurde rot, ohne dass ein Zauber seine Nische verloren hatte. Gemessen auf
//! node3, je Stand ein Bau dieses Läufers:
//!
//! ```text
//!   8382b950                                        11 von 58
//!   6d3ff723  #1135, Takt-Weckruf nach QUENCH        9   (Weckruf gestrichen: 11)
//!   6274ffa3  #1154, Entscheidungsmodell (a)(b)(c)   8
//!   2f0c0d77                                         8
//! ```
//!
//! Beide Änderungen sind beabsichtigt. Getroffen haben sie den Suchweg, nicht
//! die Zauber: Die Bestliste von `boss-1-1` (crucible-skin, aether, efflux)
//! erzielt an allen fünf Ständen bitgleich 9272. Die freie Suche gabelt in
//! Schritt 3 — `quench` 5610 statt `crucible-skin` 5410, weil QUENCH seit
//! #1135 keine Pause mehr kostet — und endet bei 9068, UNTER der Liste, die
//! sie nicht mehr findet. Ein Wächter über Suchwege schlägt bei jeder solchen
//! Wegänderung an; einer über Nischen erst, wenn eine Bestliste wirklich
//! verliert.
//!
//! **Was er weiter fängt.** Verliert ein Zauber seine Nische, sinkt seine
//! Bestliste unter die freie Suche, die freie Liste gewinnt, und die Zahl
//! fällt. Verschwindet eine Zeile aus den Kandidaten (Zauber umbenannt,
//! entfernt), bricht der Lauf mit ihrem Namen ab, statt still weniger zu
//! zählen.
//!
//! # Kosten
//!
//! Gemessen am 2026-08-14: **21 s** auf rustbuild (`--release`, vier Bosse,
//! `--max-len 6`, je 2258 Listen). Das ist der Grund, warum AC5 einen Wächter
//! bekommt und nicht den Absatz „zu teuer" — die Alternative stand in der AC
//! ausdrücklich offen.
//!
//! # Datum und Stand (AC1)
//!
//! Jeder Lauf schreibt in seinen Kopf, **wann** er lief (UTC) und **auf
//! welchem Stand**, und je Boss die Zauber, die die beste Rotation wirkt. Bis
//! zum 2026-10-29 stand dort nur die Zahl je Boss; die Messung vom 14.08. war
//! deshalb nur über den Issue-Kommentar einem Stand zuzuordnen.
//!
//! ```text
//! cargo run --release --example zauber-abdeckung -- --stand "$(git rev-parse HEAD)"
//! ```
//!
//! Der Stand kommt aus `--stand`, sonst aus `GITHUB_SHA` (der Nightly setzt
//! ihn), sonst steht `unbekannt` da. Der Läufer kann ihn nicht selbst ablesen:
//! `cargo remote` überträgt keinen Git-Verlauf, und auf node3 läuft ein
//! kopiertes Programm ohne Arbeitsbaum.
use std::time::{SystemTime, UNIX_EPOCH};

use rota_sim::apl::{Apl, AplName};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, Boss, TimelineEntry, BOSSES, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::{
    distribute_budget, WornValues, QUALITY_HAS_EFFECT, QUALITY_LEGENDARY, QUALITY_NAMES,
    QUALITY_RARE, SLOT_COUNT,
};
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{
    all_candidates, boss_kandidaten, greedy, greedy_from, Candidate, SearchResult,
};
use rota_sim::scoring::class_of;
use rota_sim::sources::gear_effects_permille;
use rota_sim::viabilitaet::{befunde, schulen, Befund, Befundart, Laufabschluss};
use rota_sim::Millis;

/// Die vier Bosse der ERSTEN Instanz.
///
/// **Vier aus EINER Instanz, nicht je einer aus vier** — das ist der Schnitt,
/// den AC2 verlangt und den vor dem 2026-08-14 keine Messung hatte. `#823`
/// nahm `boss-1-1, boss-2-2, boss-3-4, boss-4-4`; damit lässt sich nicht
/// sagen, ob eine Instanz in sich Abwechslung trägt.
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];

/// Höchstlänge der gesuchten Listen — dieselbe wie in der Messung zu AC2.
const MAX_LEN: usize = 6;

/// Durchgänge je Liste. Dieselben wie in `rotation-search`.
const RUNS: u32 = 50;
const SEED: u32 = 0;

/// **Die Untergrenze, gemessen am 2026-08-14.**
///
/// Elf von 58 spielbaren Zaubern werden über die vier Bosse zusammen gewirkt:
/// `aether, bellows, crucible-skin, efflux, kindle, purge, pyre, quench, rot,
/// searing, smoulder`.
const MINDESTENS: usize = 11;

/// **Die Bestlisten vom 2026-08-14**, je Boss der ersten Instanz (#550,
/// Variante B).
///
/// Ausgelesen mit `rotation-search --bosses boss-1-1,boss-1-2,boss-1-3,boss-1-4
/// --max-len 6` am Stand `8e25eb44` (Merge von #878, dem Stand der Messung
/// „11 von 58"), gebaut am 2026-10-29. Drei davon fand die freie Suche bis
/// `8382b950` unverändert wieder; an `boss-1-2` fand sie später eine bessere
/// (7544 statt 6397 am 14.08.) — dort gewinnt deshalb die freie Suche, wie es
/// soll.
///
/// Jede Zeile muss eine Kandidatenzeile derselben Suche sein; sonst bricht der
/// Lauf mit ihrem Namen ab (`bestliste`).
const BESTLISTEN: [(&str, [&str; MAX_LEN]); 4] = [
    (
        "boss-1-1",
        [
            "crucible-skin",
            "searing if dot.searing.remains < 1500",
            "rot",
            "aether",
            "efflux",
            "kindle",
        ],
    ),
    (
        "boss-1-2",
        [
            "searing if dot.searing.remains < 1500",
            "purge",
            "quench if boss.casting >= 1",
            "smoulder if dot.smoulder.remains < 1500",
            "rot",
            "kindle",
        ],
    ),
    (
        "boss-1-3",
        [
            "pyre if heat >= 15",
            "searing if dot.searing.remains < 1500",
            "rot",
            "smoulder if dot.smoulder.remains < 1500",
            "bellows",
            "kindle",
        ],
    ),
    (
        "boss-1-4",
        [
            "quench if boss.casting >= 1",
            "smoulder if dot.smoulder.remains < 1500",
            "pyre if heat >= 15",
            "searing if dot.searing.remains < 1500",
            "rot",
            "kindle",
        ],
    ),
];

fn boss(boss_id: &str) -> &'static Boss {
    boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"))
}

fn config(boss_id: &str, worn: WornValues) -> BalanceConfig {
    config_fuer(boss(boss_id), worn)
}

/// Dieselbe Konfiguration für einen Boss, der nicht aus den Daten kommen muss —
/// die Zwillinge des Prüfbosses aus AC3 entstehen erst zur Laufzeit.
fn config_fuer(boss: &'static Boss, worn: WornValues) -> BalanceConfig {
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

/// Die zwei Ausrüstungsstände für AC4 (Issue #550).
///
/// **Zwei, weil einer die falsche Frage beantwortet.** Stufe 85 ist das
/// Äusserste, was `itemLevelOf` vergibt (Instanz 14, Schwierigkeit 2) — dort
/// zeigt sich, ob Ausrüstung überhaupt etwas verschieben KANN. Ein Spieler
/// trägt sie in Instanz 1 aber nie; wer nur so misst, beantwortet eine Frage,
/// die sich im Spiel nicht stellt.
///
/// Stufe 13 ist der Stand, mit dem ein Spieler die vierte Instanz betritt
/// (`itemLevelOf(3, 0, 0)`, GAME-DESIGN 20.5 Punkt 1) — die höchste heute
/// erreichbare Vorstufe. Dort steht die Frage, die das Spiel wirklich stellt.
const AC4_STUFEN: [(u32, usize, &str); 2] = [
    (13, QUALITY_RARE, "erreichbar heute (Vorstufe Instanz 4)"),
    (
        85,
        QUALITY_LEGENDARY,
        "Aeusserstes, was itemLevelOf vergibt",
    ),
];

/// Der volle Satz auf einer Stufe und Qualität.
///
/// **Alle vier Töpfe belegt, aber auf zwei Wegen.** `distribute_budget` füllt
/// drei — `QUALITY_AFFIX_SLOTS = [1, 2, 3, 3]` gibt auch LEGENDARY nicht mehr.
/// Der vierte, `gear_effects`, gehört dem Aufrufer: `distribute_budget` rührt
/// ihn nicht an, und `best_build.rs:246` hält das mit einer Zusicherung fest
/// („Topf 4 gehoert der anderen Seite"). Gäbe es zwei Wege zu dieser Zahl,
/// liefen sie auseinander, sobald einer nachgezogen wird — und beide lieferten
/// weiter plausible Werte.
///
/// Ohne diese Zeilen wäre LEGENDARY hier ein EPIC: Beide haben drei
/// Affix-Plätze, der einzige Unterschied ist `QUALITY_HAS_EFFECT`.
///
/// Dieselbe Bauart wie `ausruestung()` in `stufen-durchsatz.rs`: Was belegt
/// wird, kommt aus `distribute_budget` und damit aus den Daten, nicht aus einer
/// Wahl hier.
fn voller_satz(stufe: u32, quality: usize) -> WornValues {
    let mut summe = WornValues::none();
    for _platz in 0..SLOT_COUNT {
        let mut werte = [0u64; 4];
        let belegt = distribute_budget(stufe, quality, &mut werte);
        for (topf, wert) in werte.iter().take(belegt).enumerate() {
            let w = u32::try_from(*wert).unwrap_or(u32::MAX);
            match topf % 4 {
                0 => summe.catalysis += w,
                1 => summe.purity += w,
                2 => summe.yield_bonus += w,
                _ => summe.essence += w,
            }
        }
    }
    // Der vierte Topf, vom Aufrufer gerechnet — dieselbe Funktion, die auch
    // die Spielseite benutzt. Satzanteil 0: diese Messung trägt keine Sätze.
    // Nur LEGENDARY trägt einen Sondereffekt (`QUALITY_HAS_EFFECT`).
    let legendaere = if QUALITY_HAS_EFFECT[quality] {
        u32::try_from(SLOT_COUNT).unwrap_or(0)
    } else {
        0
    };
    summe.gear_effects = gear_effects_permille(legendaere, 0);
    summe
}

/// Die Bestliste eines Bosses als Kandidatenzeilen derselben Suche.
///
/// Gesucht wird im selben Vorrat, den `greedy_zuege` bildet — `pool` und
/// dahinter `boss_kandidaten`, denn Zeilen wie `quench if boss.casting >= 1`
/// entstehen erst dort. Verglichen wird über den Text der Zeile, wie die Suche
/// sie ausgibt.
fn bestliste(cfg: &BalanceConfig, pool: &[Candidate], boss_id: &str) -> Vec<Candidate> {
    let (_, zeilen) = BESTLISTEN
        .iter()
        .find(|(b, _)| *b == boss_id)
        .unwrap_or_else(|| panic!("keine Bestliste fuer {boss_id}"));
    let mut vorrat = pool.to_vec();
    vorrat.extend(boss_kandidaten(cfg, pool));
    zeilen
        .iter()
        .map(|zeile| {
            vorrat
                .iter()
                .find(|k| k.rule().to_string() == *zeile)
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "Zeile `{zeile}` der Bestliste von {boss_id} ist keine Kandidatenzeile \
                         mehr. Ein Zauber wurde umbenannt oder entfernt, oder die Suche bildet \
                         die Zeile anders: Die Bestliste gehoert neu gemessen, nicht still \
                         gekuerzt."
                    )
                })
        })
        .collect()
}

/// Die Liste, die für einen Boss zählt (#550, Variante B).
///
/// Die freie Suche und die Suche von der Bestliste aus; die zweite gewinnt nur,
/// wenn sie MEHR bringt — dieselbe Prüfung wie der Nachlauf in
/// `rotation-search` (`versuch.best.mean > bisher`). Bei Gleichstand bleibt
/// die freie Suche, damit eine gleich gute neue Liste nicht von der alten
/// verdeckt wird.
///
/// Zurück kommen das Ergebnis, ob die Bestliste gewonnen hat, und beide Werte
/// für die Ausgabe.
fn beste_liste(
    cfg: &BalanceConfig,
    pool: &[Candidate],
    boss_id: &str,
) -> (SearchResult, bool, f64, f64) {
    let frei = greedy(cfg, pool, MAX_LEN);
    let von_bestliste = greedy_from(cfg, pool, MAX_LEN, &bestliste(cfg, pool, boss_id));
    let (wert_frei, wert_best) = (frei.best.mean, von_bestliste.best.mean);
    if von_bestliste.best.mean > frei.best.mean {
        (von_bestliste, true, wert_frei, wert_best)
    } else {
        (frei, false, wert_frei, wert_best)
    }
}

/// Was die Erhebung je Boss festhält (AC1).
struct Bosszeile {
    boss: &'static str,
    /// Ob die Bestliste vom 2026-08-14 gewonnen hat.
    bestliste_gewinnt: bool,
    /// Wert der freien Suche.
    wert_frei: f64,
    /// Wert der Suche von der Bestliste aus — `None` bei Bossen ohne
    /// Bestliste (alle außer der ersten Instanz, nur in `--schulen`).
    wert_best: Option<f64>,
    /// Die spielbaren Zauber, die der Lauf der besten Liste wirkt, als
    /// Indizes in `SPELLS` in der Reihenfolge der Daten.
    gewirkt: Vec<usize>,
    /// Ob der Spieler in diesem Lauf stirbt — gelesen vom Prüfboss (AC3).
    tot: bool,
    /// Wie viele Boss-Zauber der Lauf unterbricht — gelesen vom Prüfboss.
    unterbrochen: u32,
}

/// Sucht für einen Boss die beste Rotation und trägt ein, welche Zauber im
/// LAUF vorkamen.
///
/// **Was im Lauf vorkam, nicht was in der Liste steht.** Bei `boss-1-3` stand
/// `kindle` am 2026-08-14 in der besten Liste und wurde trotzdem nie gewirkt:
/// Die Bedingungen davor greifen immer. Wer die Liste zählte statt den Lauf,
/// bekäme dort einen Zauber zu viel.
///
/// Bosse mit Bestliste zählen die bessere von beiden ([`beste_liste`]), alle
/// anderen die freie Suche.
fn erhebe_boss(
    pool: &[Candidate],
    spielbar: &[usize],
    worn: WornValues,
    boss: &'static Boss,
) -> Bosszeile {
    let cfg = config_fuer(boss, worn);
    let (gefunden, bestliste_gewinnt, wert_frei, wert_best) =
        if BESTLISTEN.iter().any(|(b, _)| *b == boss.id) {
            let (gefunden, gewinnt, frei, best) = beste_liste(&cfg, pool, boss.id);
            (gefunden, gewinnt, frei, Some(best))
        } else {
            let frei = greedy(&cfg, pool, MAX_LEN);
            let wert = frei.best.mean;
            (frei, false, wert, None)
        };

    let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
        seed: SEED,
        boss,
        worn,
        ..rota_sim::FightConfig::default()
    });
    let lauf = fight.run_apl(&gefunden.best.apl);
    let mut gewirkt = Vec::new();
    for &i in spielbar {
        if lauf.casts[i] > 0 {
            gewirkt.push(i);
        }
    }
    Bosszeile {
        boss: boss.id,
        bestliste_gewinnt,
        wert_frei,
        wert_best,
        gewirkt,
        tot: fight.player_dead(),
        unterbrochen: fight.interrupts_landed(),
    }
}

/// Die Erhebung über die vier Bosse der ersten Instanz.
fn erhebe(pool: &[Candidate], spielbar: &[usize], worn: WornValues) -> Vec<Bosszeile> {
    BOSSE
        .iter()
        .map(|&boss_id| erhebe_boss(pool, spielbar, worn, boss(boss_id)))
        .collect()
}

/// Die Zauber, die in mindestens einer Zeile gewirkt werden — in der
/// Reihenfolge der Grundmenge.
fn irgendwo_gewirkt(zeilen: &[Bosszeile], spielbar: &[usize]) -> Vec<&'static str> {
    spielbar
        .iter()
        .filter(|i| zeilen.iter().any(|z| z.gewirkt.contains(i)))
        .map(|&i| SPELLS[i].id)
        .collect()
}

/// Bezeichner einer Menge von Zauber-Indizes, mit Komma getrennt.
fn namen(zauber: &[usize]) -> String {
    let namen: Vec<&str> = zauber.iter().map(|&i| SPELLS[i].id).collect();
    if namen.is_empty() {
        "—".to_owned()
    } else {
        namen.join(", ")
    }
}

// ------------------------------------------------- Datum und Stand (AC1)

/// Woher eine Erhebung stammt: wann sie lief und auf welchem Stand.
struct Herkunft {
    /// UTC, `JJJJ-MM-TT hh:mm:ss UTC`.
    datum: String,
    /// Der Commit — `None`, wenn ihn weder `--stand` noch `GITHUB_SHA`
    /// nennt.
    stand: Option<String>,
}

impl Herkunft {
    /// Die Herkunft dieses Laufs: die Uhr jetzt, der Stand aus Schalter oder
    /// Umgebung.
    fn jetzt() -> Self {
        let sekunden = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let args: Vec<String> = std::env::args().collect();
        Self {
            datum: datum_utc(sekunden),
            stand: stand_aus(&args, std::env::var("GITHUB_SHA").ok()),
        }
    }
}

/// Der Stand: `--stand <commit>` vor `GITHUB_SHA`; leer zählt als fehlend.
///
/// **Der Schalter gewinnt**, weil er die ausdrückliche Angabe ist: Wer im
/// Nightly-Umfeld einen anderen Stand nachmisst, soll nicht still den des
/// Laufs eingetragen bekommen.
fn stand_aus(args: &[String], github_sha: Option<String>) -> Option<String> {
    // ROTER STAND (#550 AC1): noch ohne Stand.
    let _ = (args, github_sha);
    None
}

/// Sekunden seit 1970-01-01 als UTC-Datum mit Uhrzeit.
///
/// **Von Hand, weil die Kiste keine Abhängigkeit trägt** (`sim/Cargo.toml`:
/// eine Uhr-Bibliothek wäre eine in allen Zielen des Pakets). Die Umrechnung
/// der Tage ist `civil_from_days` von Howard Hinnant, eingeschränkt auf Tage
/// ab 1970 — davor liegt keine Uhr, die hier abgelesen wird.
fn datum_utc(sekunden: u64) -> String {
    // ROTER STAND (#550 AC1): noch ohne Datum.
    let _ = sekunden;
    String::new()
}

/// Der Bericht des Laufs ohne Schalter, Zeile für Zeile — dieselben Zeilen,
/// die `main` druckt und der Nightly in seine Zusammenfassung schreibt.
fn bericht(herkunft: &Herkunft, zeilen: &[Bosszeile], spielbar: &[usize]) -> Vec<String> {
    // ROTER STAND (#550 AC1): die Ausgabe von vor dem 2026-10-29 — ohne Datum,
    // ohne Stand, je Boss nur die Zahl.
    let _ = herkunft;
    let mut out = vec![
        "== Zauber-Abdeckung der ersten Instanz (Issue #550, AC5) ==".to_owned(),
        String::new(),
    ];
    for z in zeilen {
        let quelle = if z.bestliste_gewinnt {
            "Bestliste"
        } else {
            "freie Suche"
        };
        out.push(format!(
            "  {:<10} {:>2} von {} gewirkt   {quelle:<11}  frei {:.1}, von der Bestliste {:.1}",
            z.boss,
            z.gewirkt.len(),
            spielbar.len(),
            z.wert_frei,
            z.wert_best.unwrap_or(0.0)
        ));
    }
    let gewirkt = irgendwo_gewirkt(zeilen, spielbar);
    out.push(String::new());
    out.push(format!(
        "  ueber alle {} Bosse zusammen   {} von {}",
        zeilen.len(),
        gewirkt.len(),
        spielbar.len()
    ));
    out.push(format!("    {}", gewirkt.join(", ")));
    out
}

// ------------------------------------- AC3: Schulen über die gebauten Bosse

/// Die Schule, die am 2026-10-29 über die gebauten Bosse in keiner besten
/// Rotation gewirkt wird — so heißt sie in den Daten.
const FIXATION: &str = "FIXATION";

/// Wie oft eine Liste an einem Boss Kontrollpunkte ausgibt (FIXATION).
///
/// Gezählt im Kampfprotokoll (`LogKind::ControlSpent`), nicht an den Würfen:
/// Ein FIXATION-Zauber ohne genug Punkte wird gewirkt und richtet nichts aus
/// (`Fight::apply_fixation`, „der Zauber ist vertan").
fn kontrolle_ausgegeben(boss: &'static Boss, liste: &Apl) -> usize {
    let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
        seed: SEED,
        boss,
        stop_on_kill: false,
        log_capacity: 100_000,
        ..rota_sim::FightConfig::default()
    });
    let _ = fight.run_apl(liste);
    fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::ControlSpent)
        .count()
}

/// Wie viele Listen eine Projektion der Viabilität höchstens messen darf —
/// dieselbe Grenze wie der Zauber-Vergleich im Bericht (`report.rs`,
/// `BUDGET`), damit beide Befunde unter denselben Mitteln entstehen.
const BUDGET: u64 = 1_000_000;

/// Ein gebauter Boss in der Schul-Erhebung.
struct Schulboss {
    /// Was die beste Rotation wirkt — dieselbe Zeile wie in der Erhebung oben.
    zeile: Bosszeile,
    /// Die Zauber mit dem Befund „mindestens bezeugt" (#946).
    bezeugt: Vec<usize>,
    /// Ob jeder Zauber genau einen Befund hat und jede Projektion vollständig
    /// gerechnet ist ([`Laufabschluss`]).
    abgeschlossen: bool,
    /// Je bezeugtem FIXATION-Zauber: wie oft seine Zeugen-Liste Kontrollpunkte
    /// ausgibt.
    ///
    /// **Ein Zeuge belegt einen Beitrag, nicht welchen** (#946): Er zählt, wenn
    /// die Liste ohne die Zeilen des Zaubers messbar weniger bringt. Ein Zauber,
    /// der nur eine globale Wartezeit belegt und dadurch einen anderen auf den
    /// richtigen Zeitpunkt schiebt, ist danach ebenso bezeugt. Für FIXATION
    /// lässt sich das unterscheiden: Ihre Wirkung ist das Ausgeben von
    /// Kontrollpunkten. Gibt die Liste keine aus, hat die Schule dort nicht
    /// gewirkt, sondern gewartet.
    fixation_zeugen: Vec<(usize, usize)>,
}

/// Die gebauten Bosse: alle wertungsfähigen, in der Reihenfolge der Daten.
///
/// **Aus der Klassifikation, nicht aus einer Liste hier.** Kommt ein Boss
/// dazu, zählt er mit, ohne dass jemand diese Datei anfasst — die Prüfbosse
/// (`test`, `testing`) bleiben draußen.
fn gebaute_bosse() -> Vec<&'static str> {
    BOSSES
        .iter()
        .filter(|b| class_of(b).is_scored())
        .map(|b| b.id)
        .collect()
}

/// Je gebautem Boss: die beste Rotation und die Befunde der Viabilität.
fn schulerhebung(pool: &[Candidate], spielbar: &[usize]) -> Vec<Schulboss> {
    gebaute_bosse()
        .into_iter()
        .map(|boss_id| {
            let zeile = erhebe_boss(pool, spielbar, WornValues::none(), boss(boss_id));
            let tabelle = befunde(
                &config(boss_id, WornValues::none()),
                pool,
                spielbar,
                MAX_LEN,
                BUDGET,
            );
            let bezeugt = tabelle
                .zeilen
                .iter()
                .filter(|z| z.befund.art() == Befundart::MindestensBezeugt)
                .map(|z| z.spell)
                .collect();
            let fixation_zeugen = tabelle
                .zeilen
                .iter()
                .filter(|z| SPELLS[z.spell].school == Some(FIXATION))
                .filter_map(|z| match &z.befund {
                    Befund::MindestensBezeugt(zeuge) => {
                        Some((z.spell, kontrolle_ausgegeben(boss(boss_id), &zeuge.liste)))
                    }
                    _ => None,
                })
                .collect();
            Schulboss {
                zeile,
                bezeugt,
                abgeschlossen: matches!(tabelle.laufabschluss(), Laufabschluss::Abgeschlossen),
                fixation_zeugen,
            }
        })
        .collect()
}

/// Die Zauber einer Schule mit der Zahl der Bosse, an denen sie vorkommen.
fn mit_bossen(zauber: &[usize], an: impl Fn(usize) -> usize) -> String {
    let teile: Vec<String> = zauber
        .iter()
        .map(|&i| format!("{} ({})", SPELLS[i].id, an(i)))
        .collect();
    if teile.is_empty() {
        "—".to_owned()
    } else {
        teile.join(", ")
    }
}

/// Der Bericht der Schul-Erhebung, Zeile für Zeile.
///
/// Zwei Zählungen nebeneinander, weil sie Verschiedenes sagen: **gewirkt**
/// ist, was die beste gefundene Rotation im Lauf wirkt (die Zählung oben);
/// **bezeugt** ist der Befund „mindestens bezeugt" — auch über eine
/// Projektion, wenn die Suche ihn nicht fand.
fn schulbericht(bosse: &[Schulboss], spielbar: &[usize]) -> Vec<String> {
    let mut out = vec![
        String::new(),
        "== AC3: Schulen ueber die gebauten Bosse (Issue #550) ==".to_owned(),
        format!(
            "  {} wertungsfaehige Bosse, {} spielbare Zauber, Laenge bis {MAX_LEN}, ohne \
             Ausruestung",
            bosse.len(),
            spielbar.len()
        ),
        "  gewirkt  im Lauf der besten Rotation gewirkt (dieselbe Zaehlung wie oben)".to_owned(),
        format!(
            "  bezeugt  Befund \"mindestens bezeugt\" (#946), Projektionen bis {BUDGET} Listen"
        ),
        String::new(),
    ];
    for b in bosse {
        out.push(format!(
            "  {:<16} gewirkt {:>2}   bezeugt {:>2}   Lauf {}",
            b.zeile.boss,
            b.zeile.gewirkt.len(),
            b.bezeugt.len(),
            if b.abgeschlossen {
                "abgeschlossen"
            } else {
                "NICHT abgeschlossen"
            }
        ));
        out.push(format!(
            "  {:<16}   gewirkt  {}",
            "",
            namen(&b.zeile.gewirkt)
        ));
        out.push(format!("  {:<16}   bezeugt  {}", "", namen(&b.bezeugt)));
    }

    let gewirkt_an = |i: usize| {
        bosse
            .iter()
            .filter(|b| b.zeile.gewirkt.contains(&i))
            .count()
    };
    let bezeugt_an = |i: usize| bosse.iter().filter(|b| b.bezeugt.contains(&i)).count();

    out.push(String::new());
    out.push(
        "  Schule          Zauber   gewirkt   bezeugt   (in Klammern: an wie vielen Bossen)"
            .to_owned(),
    );
    let mut leer_gewirkt = Vec::new();
    let mut leer_bezeugt = Vec::new();
    let mut reihen: Vec<Option<&str>> = schulen(spielbar).into_iter().map(Some).collect();
    reihen.push(None);
    for schule in reihen {
        let zauber: Vec<usize> = spielbar
            .iter()
            .copied()
            .filter(|&i| SPELLS[i].school == schule)
            .collect();
        let gewirkt: Vec<usize> = zauber
            .iter()
            .copied()
            .filter(|&i| gewirkt_an(i) > 0)
            .collect();
        let bezeugt: Vec<usize> = zauber
            .iter()
            .copied()
            .filter(|&i| bezeugt_an(i) > 0)
            .collect();
        let name = schule.unwrap_or("ohne Schule");
        out.push(format!(
            "  {name:<14} {:>7}   {:>7}   {:>7}",
            zauber.len(),
            gewirkt.len(),
            bezeugt.len()
        ));
        out.push(format!(
            "      gewirkt  {}",
            mit_bossen(&gewirkt, gewirkt_an)
        ));
        out.push(format!(
            "      bezeugt  {}",
            mit_bossen(&bezeugt, bezeugt_an)
        ));
        if schule == Some(FIXATION) {
            let zeugen: Vec<String> = bosse
                .iter()
                .flat_map(|b| {
                    b.fixation_zeugen
                        .iter()
                        .map(move |&(i, n)| format!("{} an {}: {n}", SPELLS[i].id, b.zeile.boss))
                })
                .collect();
            if !zeugen.is_empty() {
                out.push(format!(
                    "      Kontrollpunkte ausgegeben in der Zeugen-Liste  {}",
                    zeugen.join(", ")
                ));
            }
        }
        // Die schulunabhängigen Sonderzauber sind keine Schule (GAME-DESIGN
        // 7.4) — sie können nicht „leer" sein.
        if let Some(schule) = schule {
            if gewirkt.is_empty() {
                leer_gewirkt.push(schule);
            }
            if bezeugt.is_empty() {
                leer_bezeugt.push(schule);
            }
        }
    }
    let liste = |l: &[&str]| {
        if l.is_empty() {
            "keine".to_owned()
        } else {
            l.join(", ")
        }
    };
    out.push(String::new());
    out.push(format!("  LEER nach gewirkt:  {}", liste(&leer_gewirkt)));
    out.push(format!("  LEER nach bezeugt:  {}", liste(&leer_bezeugt)));
    out
}

// ------------------------------------------ AC3: der Prüfboss für FIXATION

/// Der Prüfboss, der die Anforderung für FIXATION stellt (AC3).
///
/// Ein Schlag über der Verteidigungs-Obergrenze, der fällt, wenn vom Kampf
/// noch genug übrig ist. **Zwilling von `test-lethal-strike`** (#797): dieselbe
/// Datei bis auf den Zeitpunkt des Schlags, 24 s statt 12 s. Warum der
/// Zeitpunkt die Anforderung ist, steht in `docs/ZAUBER-ABDECKUNG.md`.
const PRUEFBOSS: &str = "test-late-lethal-strike";

/// Sein Zeit-Zwilling: derselbe Schlag, 12 s früher.
const ZEIT_ZWILLING: &str = "test-lethal-strike";

/// Der Schaden des Schaden-Zwillings: derselbe wie bei
/// `test-survivable-strike` (#797) — die halbe Verteidigungs-Obergrenze, die
/// der Spieler mit seinen Leben allein aushält.
const UEBERLEBBAR: u32 = 4600;

/// Die Schwellenreihe über den Schaden, am Zeitpunkt des Prüfbosses.
///
/// Dicht an der Stelle, an der Aushalten aufhört zu tragen (9600 gegen 10000,
/// gemessen am 2026-10-29), weil sich dort entscheidet, ob es einen Bereich
/// gibt, in dem FIXATION die BESTE und nicht die EINZIGE Antwort ist.
const SCHADENSREIHE: [u32; 7] = [4600, 9300, 9600, 10000, 11500, 14000, 18400];

/// Die Zeitreihe über den Zeitpunkt des Schlags, mit dem Schaden des
/// Prüfbosses. 12000 und 24000 sind die beiden Zwillinge aus den Daten.
const ZEITREIHE: [Millis; 8] = [6000, 12000, 14000, 16000, 24000, 30000, 36000, 48000];

/// Ein Boss mit verschobenem oder abgeschwächtem einzigen Schlag.
///
/// **Zur Laufzeit gebildet, nicht als Datei abgelegt:** Jede Zeile der
/// Schwellenreihen wäre sonst eine eigene Bossdatei, die Spiel, Simulator und
/// Paritäts-Aufzeichnung mitführen müssten. `Box::leak`, weil
/// `BalanceConfig` einen `&'static Boss` verlangt; ein Lauf bildet höchstens
/// sechzehn davon.
fn mit_schlag(vorlage: &'static Boss, at_ms: Millis, schaden: u32) -> &'static Boss {
    assert_eq!(
        vorlage.timeline.len(),
        1,
        "{} traegt nicht genau einen Schlag — dann ist unklar, welcher verschoben wird",
        vorlage.id
    );
    let mut schlag = vorlage.timeline[0];
    schlag.at_ms = at_ms;
    schlag.damage = schaden;
    let timeline: &'static [TimelineEntry] = Box::leak(Box::new([schlag]));
    Box::leak(Box::new(Boss {
        timeline,
        ..*vorlage
    }))
}

/// Die FIXATION-Zauber, die der Lauf einer Zeile wirkt.
fn fixation_gewirkt(zeile: &Bosszeile) -> Vec<usize> {
    zeile
        .gewirkt
        .iter()
        .copied()
        .filter(|&i| SPELLS[i].school == Some(FIXATION))
        .collect()
}

/// Die beste Rotation, wenn FIXATION nicht zur Wahl steht — die Frage aus
/// GAME-DESIGN 11.3: „Kein Boss verlangt eine bestimmte Schule."
fn ohne_fixation(pool: &[Candidate], spielbar: &[usize], boss: &'static Boss) -> Bosszeile {
    let ohne: Vec<Candidate> = pool
        .iter()
        .filter(|k| SPELLS[k.spell].school != Some(FIXATION))
        .cloned()
        .collect();
    erhebe_boss(&ohne, spielbar, WornValues::none(), boss)
}

/// Eine Zeile des Prüfboss-Berichts: mit und ohne FIXATION.
fn pruefzeile(pool: &[Candidate], spielbar: &[usize], name: &str, boss: &'static Boss) -> String {
    let mit = erhebe_boss(pool, spielbar, WornValues::none(), boss);
    let ohne = ohne_fixation(pool, spielbar, boss);
    let schlag = boss.timeline[0];
    format!(
        "  {name:<26} {:>5} bei {:>5} ms   {:>7.1}  FIXATION {:<10} tot {:<5} unterbrochen {}   \
         ohne FIXATION {:>7.1} tot {}",
        schlag.damage,
        schlag.at_ms,
        mit.wert_frei,
        namen(&fixation_gewirkt(&mit)),
        mit.tot,
        mit.unterbrochen,
        ohne.wert_frei,
        ohne.tot
    )
}

/// Der Bericht zum Prüfboss, Zeile für Zeile.
fn pruefbericht(pool: &[Candidate], spielbar: &[usize]) -> Vec<String> {
    let pruefboss = boss(PRUEFBOSS);
    let schlag = pruefboss.timeline[0];
    let mut out = vec![
        String::new(),
        format!("== AC3: Pruefboss fuer {FIXATION} ({PRUEFBOSS}) =="),
        format!(
            "  ein Schlag ({}), {} Leben, Kampf {} ms; dieselbe Suche wie oben, Laenge bis \
             {MAX_LEN}, ohne Ausruestung",
            schlag.mechanic, pruefboss.hp, pruefboss.overpressure_ms
        ),
        String::new(),
        "  Zwillinge (je eine Zahl anders)".to_owned(),
        pruefzeile(pool, spielbar, PRUEFBOSS, pruefboss),
        pruefzeile(pool, spielbar, ZEIT_ZWILLING, boss(ZEIT_ZWILLING)),
        pruefzeile(
            pool,
            spielbar,
            "Schaden-Zwilling",
            mit_schlag(pruefboss, schlag.at_ms, UEBERLEBBAR),
        ),
        String::new(),
        format!("  Schadensreihe bei {} ms", schlag.at_ms),
    ];
    for schaden in SCHADENSREIHE {
        out.push(pruefzeile(
            pool,
            spielbar,
            "",
            mit_schlag(pruefboss, schlag.at_ms, schaden),
        ));
    }
    out.push(String::new());
    out.push(format!("  Zeitreihe mit {} Schaden", schlag.damage));
    for at_ms in ZEITREIHE {
        out.push(pruefzeile(
            pool,
            spielbar,
            "",
            mit_schlag(pruefboss, at_ms, schlag.damage),
        ));
    }
    out
}

/// Die spielbaren Zauber als Indizes in `SPELLS` — die Grundmenge der Zahl.
fn spielbare_zauber() -> Vec<usize> {
    (0..rota_sim::SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
        .collect()
}

fn main() {
    let pool = all_candidates();
    let spielbar = spielbare_zauber();

    let zeilen = erhebe(&pool, &spielbar, WornValues::none());
    for zeile in bericht(&Herkunft::jetzt(), &zeilen, &spielbar) {
        println!("{zeile}");
    }
    println!();

    let gewirkt = irgendwo_gewirkt(&zeilen, &spielbar);

    // Die Zusicherung, ohne die der Wächter nichts hält: Stünde hier eine
    // leere Liste spielbarer Zauber, wäre jede Schwelle trivial verfehlt oder
    // trivial erfüllt, je nach Vorzeichen.
    assert!(
        spielbar.len() > MINDESTENS,
        "es gibt nur {} spielbare Zauber — die Schwelle {} misst dann nichts",
        spielbar.len(),
        MINDESTENS
    );

    // **GEGEN DIE ANDERE RICHTUNG — und die ist die gefährlichere.**
    //
    // Eine Untergrenze fängt nur, dass die Zahl FÄLLT. Wird die Zählung
    // verwässert, steigt sie, und der Wächter meldet grün, ohne noch etwas zu
    // messen. Gemessen am 2026-08-14 vom Orchestrator, an genau einem Zeichen:
    //
    // ```text
    // if lauf.casts[i] > 0     ->     if lauf.casts[i] >= 0
    //   Nullprobe   6 / 6 / 5 / 6,  über alle 11
    //   Mutation    58 bei JEDEM Boss,  "BESTANDEN: 58 von 58 >= 11"
    // ```
    //
    // `>= 0` ist wahr für jeden Zauber, auch für die 47, die nie gewirkt
    // werden. Das ist ein Tippfehler, den ein Mensch macht — und danach läuft
    // der Wächter jede Nacht grün, ohne je wieder etwas zu prüfen. Niemand
    // merkt es, weil Grün der erwartete Zustand ist.
    //
    // **Und wenn eines Tages wirklich alle spielbaren Zauber vorkommen?** Dann
    // wird diese Zeile rot, und das ist richtig so: Leitplanke 2 wäre erfüllt,
    // das Issue erledigt, und dieser Wächter gehört dann überdacht statt
    // weitergeschleppt. Eine rote Zeile mit dieser Meldung ist der bessere Weg
    // dorthin als ein stilles Grün.
    assert!(
        gewirkt.len() < spielbar.len(),
        "ALLE {} spielbaren Zauber gelten als gewirkt — die Zaehlung misst \
         nichts mehr. Entweder ist die Bedingung in der Schleife oben \
         verwaessert (`> 0` gegen `>= 0`), oder Leitplanke 2 ist erfuellt und \
         dieser Waechter hat seinen Zweck verloren. Beides gehoert angesehen, \
         keines uebergangen.",
        spielbar.len()
    );

    // **Das Urteil steht hier, der Abbruch erst am Ende von `main`** (#550).
    //
    // Bis zum 2026-10-29 stand `exit(1)` an dieser Stelle. Solange die
    // Untergrenze hielt, fiel das nicht auf; an `2f0c0d77` ist sie rot, und
    // `--mit-ausruestung` lieferte dieselbe Ausgabe wie der Lauf ohne Schalter,
    // Exit 1 — die AC4-Messung war nicht mehr zu fahren, genau dann, wenn sie
    // erklären soll, was sich verschoben hat. Der Nightly sieht keinen
    // Unterschied: Er fährt ohne Schalter, Ausgabe und Exit-Code bleiben gleich.
    let unterschritten = gewirkt.len() < MINDESTENS;
    if unterschritten {
        eprintln!(
            "FEHLGESCHLAGEN: nur {} von {} spielbaren Zaubern werden ueber die {} \
             Bosse der ersten Instanz gewirkt — erwartet mindestens {}.",
            gewirkt.len(),
            spielbar.len(),
            BOSSE.len(),
            MINDESTENS
        );
        eprintln!(
            "Ein Zauber hat seine Nische verloren, oder ein Boss erzwingt sie nicht mehr. \
             Die Schwelle steht in `sim/examples/zauber-abdeckung.rs` (MINDESTENS) und \
             darf nur mit einer neuen Messung bewegt werden."
        );
    } else {
        println!(
            "BESTANDEN: {} von {} >= {} (Untergrenze vom 2026-08-14)",
            gewirkt.len(),
            spielbar.len(),
            MINDESTENS
        );
    }

    // **AC4 aus #550, hinter einem Schalter.** Die Frage lautet: Ändert
    // Ausrüstung, WELCHE Zauber vorkommen — oder nur, wie viel Schaden sie
    // machen?
    //
    // Hinter einem Schalter, weil der Nightly die Zahl täglich braucht und den
    // Vergleich nicht: Er verdoppelt die Laufzeit und beantwortet eine Frage,
    // die einmal gestellt war.
    //
    // **Zur Wortwahl der AC:** Sie verlangt „Faktoren ungleich NEUTRAL".
    // `factors` (`DamageFactors`) und `worn` (`WornValues`) sind zwei Felder;
    // Ausrüstung wirkt über `worn` — „der Weg, auf dem Ausruestung ueberhaupt
    // messbar wird" (`balance.rs`, Feld-Kommentar, Issue #177). `factors` von
    // Hand zu verstellen wäre eine Modellrechnung, keine Messung. Deshalb
    // bewegt dieser Vergleich `worn`.
    // **Welcher Topf traegt den Kollaps?** (Auftrag 7985)
    //
    // Der volle Satz auf Stufe 85 laesst 1 von 58 Zaubern uebrig. Welche der
    // fuenf Groessen das verursacht, war eine Vermutung — hier wird sie
    // gemessen, indem je Lauf nur EINE gesetzt wird.
    //
    // **Ein einzelner Topf ist kein realer Ausruestungsstand**, und das ist
    // Absicht: `distribute_budget` verteilt ein Budget ueber die belegten
    // Toepfe, und wer die Frage „welcher allein" stellt, muss sie einzeln
    // setzen. Die Werte sind die aus dem vollen Satz (Stufe 85, LEGENDARY),
    // damit der Vergleich derselbe ist.
    if std::env::args().any(|a| a == "--toepfe-einzeln") {
        let voll = voller_satz(85, QUALITY_LEGENDARY);
        println!();
        println!("== Welcher Topf traegt den Kollaps? (Auftrag 7985) ==");
        println!();
        println!("  Bezug: voller Satz Stufe 85 LEGENDARY = 1 von 58");
        println!("  Je Zeile ist NUR die genannte Groesse gesetzt, der Rest 0.");
        println!();
        println!("  Topf            Wert   gewirkt   Zauber");

        let einzeln: [(&str, WornValues); 5] = [
            (
                "catalysis",
                WornValues {
                    catalysis: voll.catalysis,
                    ..WornValues::none()
                },
            ),
            (
                "purity",
                WornValues {
                    purity: voll.purity,
                    ..WornValues::none()
                },
            ),
            (
                "yield_bonus",
                WornValues {
                    yield_bonus: voll.yield_bonus,
                    ..WornValues::none()
                },
            ),
            (
                // **Im vollen Satz steht essence auf 0** — `distribute_budget`
                // belegt bei drei Affix-Plaetzen die ersten drei Groessen. Hier
                // steht er trotzdem mit dem Wert der anderen, um die Frage zu
                // beantworten, ob er es KOENNTE.
                "essence (haette)",
                WornValues {
                    essence: voll.catalysis,
                    ..WornValues::none()
                },
            ),
            (
                "gear_effects",
                WornValues {
                    gear_effects: voll.gear_effects,
                    ..WornValues::none()
                },
            ),
        ];

        for (name, worn) in einzeln {
            let zauber = irgendwo_gewirkt(&erhebe(&pool, &spielbar, worn), &spielbar);
            let wert =
                worn.catalysis + worn.purity + worn.yield_bonus + worn.essence + worn.gear_effects;
            println!(
                "  {name:<16} {wert:>6}   {:>2} von {}   {}",
                zauber.len(),
                spielbar.len(),
                zauber.join(", ")
            );
        }

        // **Die Schwellenreihe** — vier Stuetzstellen ueber die
        // Groessenordnungen, damit sichtbar wird, WO es kippt und nicht nur
        // DASS es kippt. Sie laeuft auf dem Topf, der oben allein traegt;
        // steht dort keiner, sagt die Reihe das ebenso deutlich.
        println!();
        println!("  Schwellenreihe auf catalysis (die Vermutung aus 7985):");
        println!("  Wert     gewirkt   Zauber");
        for wert in [1000u32, 4000, 12000, 36000] {
            let worn = WornValues {
                catalysis: wert,
                ..WornValues::none()
            };
            let zauber = irgendwo_gewirkt(&erhebe(&pool, &spielbar, worn), &spielbar);
            println!(
                "  {wert:>6}    {:>2} von {}   {}",
                zauber.len(),
                spielbar.len(),
                zauber.join(", ")
            );
        }
    }

    if std::env::args().any(|a| a == "--mit-ausruestung") {
        println!();
        println!("== AC4: dieselbe Erhebung MIT Ausruestung ==");
        println!();
        println!(
            "  NEUTRAL                     {:>2} von {}   {}",
            gewirkt.len(),
            spielbar.len(),
            gewirkt.join(", ")
        );

        for (stufe, quality, warum) in AC4_STUFEN {
            let worn = voller_satz(stufe, quality);
            // Die Zusicherung je Stand: Ein Satz ohne Werte sähe wie
            // „verschiebt nichts" aus und wäre nur ein zweiter NEUTRAL-Lauf.
            assert!(
                worn.catalysis + worn.purity + worn.yield_bonus + worn.essence + worn.gear_effects
                    > 0,
                "der Satz auf Stufe {stufe} traegt keine Werte — dann ist der Vergleich \
                 zwei mal NEUTRAL"
            );

            let gewirkt_mit = irgendwo_gewirkt(&erhebe(&pool, &spielbar, worn), &spielbar);

            println!();
            println!("  Stufe {stufe}, {} — {warum}", QUALITY_NAMES[quality]);
            println!(
                "    worn: catalysis {}, purity {}, yield {}, essence {}, effects {}",
                worn.catalysis, worn.purity, worn.yield_bonus, worn.essence, worn.gear_effects
            );
            println!(
                "    ausgeruestet                {:>2} von {}   {}",
                gewirkt_mit.len(),
                spielbar.len(),
                gewirkt_mit.join(", ")
            );

            let nur_neutral: Vec<&str> = gewirkt
                .iter()
                .filter(|z| !gewirkt_mit.contains(z))
                .copied()
                .collect();
            let nur_mit: Vec<&str> = gewirkt_mit
                .iter()
                .filter(|z| !gewirkt.contains(z))
                .copied()
                .collect();

            if nur_neutral.is_empty() && nur_mit.is_empty() {
                println!("    -> VERSCHIEBT DIE AUSWAHL NICHT: dieselben Zauber.");
            } else {
                println!("    -> VERSCHIEBT DIE AUSWAHL:");
                if !nur_neutral.is_empty() {
                    println!("       faellt weg: {}", nur_neutral.join(", "));
                }
                if !nur_mit.is_empty() {
                    println!("       kommt dazu: {}", nur_mit.join(", "));
                }
            }
        }
    }

    // **AC3 aus #550, hinter einem Schalter:** Welche Schulen kommen über die
    // gebauten Bosse vor — gezählt nach Wurf und nach Befund (#946)?
    //
    // Hinter einem Schalter aus demselben Grund wie AC4: Die Viabilität rechnet
    // je Boss bis zu 58 erschöpfende Projektionen, über 21 Bosse. Der Nightly
    // braucht die Zahl oben täglich, diese Erhebung nicht.
    if std::env::args().any(|a| a == "--schulen") {
        let bosse = schulerhebung(&pool, &spielbar);
        for zeile in schulbericht(&bosse, &spielbar) {
            println!("{zeile}");
        }
        for zeile in pruefbericht(&pool, &spielbar) {
            println!("{zeile}");
        }
    }

    // Der Handschrift-Vergleich bleibt `rotation-search` überlassen; dieser
    // Läufer hält genau eine Zahl. `AplName` wird nur importiert, damit die
    // Signatur von `greedy` erfüllt ist — ohne Verwendung meldet clippy.
    let _ = AplName("zauber-abdeckung");

    // Der Abbruch der Untergrenze — nach allen Schalter-Teilen, siehe oben.
    if unterschritten {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zauber(id: &str) -> usize {
        rota_sim::data::index_of(id).unwrap_or_else(|| panic!("Zauber {id} fehlt"))
    }

    /// **AC1: Der Bericht nennt Datum und Stand, je Boss die gewirkten Zauber.**
    ///
    /// Auf festen Eingaben gerechnet, nicht auf der Uhr: Ein Prüffall, der am
    /// heutigen Datum hängt, wäre morgen rot (Auftrag vom 2026-10-29). Die
    /// Zeilen sind dieselben, die `main` druckt und der Nightly in seine
    /// Zusammenfassung schreibt.
    #[test]
    fn ac1_der_bericht_nennt_datum_stand_und_je_boss_die_zauber() {
        let zeilen = [
            Bosszeile {
                boss: "boss-1-1",
                bestliste_gewinnt: true,
                wert_frei: 1.0,
                wert_best: Some(2.0),
                gewirkt: vec![zauber("kindle"), zauber("rot")],
                tot: false,
                unterbrochen: 0,
            },
            Bosszeile {
                boss: "boss-1-2",
                bestliste_gewinnt: false,
                wert_frei: 3.0,
                wert_best: Some(1.0),
                gewirkt: vec![zauber("quench")],
                tot: false,
                unterbrochen: 0,
            },
        ];
        let spielbar = spielbare_zauber();
        let herkunft = Herkunft {
            datum: datum_utc(1_000_000_000),
            stand: Some("0123abcd".to_owned()),
        };
        let text = bericht(&herkunft, &zeilen, &spielbar);

        assert!(
            text.iter().any(|l| l == "  Datum  2001-09-09 01:46:40 UTC"),
            "kein Datum im Kopf:\n{}",
            text.join("\n")
        );
        assert!(
            text.iter().any(|l| l == "  Stand  0123abcd"),
            "kein Stand im Kopf:\n{}",
            text.join("\n")
        );
        for (boss, erwartet) in [("boss-1-1", "kindle, rot"), ("boss-1-2", "quench")] {
            let i = text
                .iter()
                .position(|l| l.trim_start().starts_with(boss))
                .unwrap_or_else(|| panic!("keine Zeile fuer {boss}:\n{}", text.join("\n")));
            assert_eq!(
                text.get(i + 1).map(|l| l.trim()),
                Some(erwartet),
                "unter {boss} stehen nicht seine Zauber:\n{}",
                text.join("\n")
            );
        }

        let ohne = bericht(
            &Herkunft {
                datum: datum_utc(0),
                stand: None,
            },
            &zeilen,
            &spielbar,
        );
        assert!(
            ohne.iter().any(|l| l.starts_with("  Stand  unbekannt")),
            "ohne Stand steht nicht `unbekannt` da:\n{}",
            ohne.join("\n")
        );
    }

    /// **AC1: Das Datum stimmt auch an den Rändern des Kalenders.**
    ///
    /// Bezugswerte aus `date -u -d @<sekunden>`; dabei die Schalttage 2000
    /// (durch 400 teilbar) und 2024 und das Nicht-Schaltjahr 2100.
    #[test]
    fn ac1_das_datum_rechnet_utc_aus_sekunden() {
        for (sekunden, erwartet) in [
            (0, "1970-01-01 00:00:00 UTC"),
            (951_782_400, "2000-02-29 00:00:00 UTC"),
            (951_868_800, "2000-03-01 00:00:00 UTC"),
            (1_000_000_000, "2001-09-09 01:46:40 UTC"),
            (1_709_251_199, "2024-02-29 23:59:59 UTC"),
            (1_793_283_910, "2026-10-29 14:25:10 UTC"),
            (4_107_456_000, "2100-02-28 00:00:00 UTC"),
            (4_107_542_400, "2100-03-01 00:00:00 UTC"),
        ] {
            assert_eq!(datum_utc(sekunden), erwartet, "{sekunden} s");
        }
    }

    /// **AC1: Der Stand kommt vom Schalter vor der Umgebung.**
    #[test]
    fn ac1_der_stand_kommt_vom_schalter_vor_github_sha() {
        let args = |a: &[&str]| -> Vec<String> { a.iter().map(|s| (*s).to_owned()).collect() };
        let sha = || Some("feedbeef".to_owned());
        assert_eq!(
            stand_aus(&args(&["x", "--stand", "0123abcd"]), sha()),
            Some("0123abcd".to_owned())
        );
        assert_eq!(stand_aus(&args(&["x"]), sha()), Some("feedbeef".to_owned()));
        assert_eq!(stand_aus(&args(&["x"]), None), None);
        assert_eq!(stand_aus(&args(&["x"]), Some("  ".to_owned())), None);
        assert_eq!(
            stand_aus(&args(&["x", "--stand"]), sha()),
            Some("feedbeef".to_owned()),
            "`--stand` ohne Wert ist keine Angabe"
        );
    }

    /// **AC3: Am Prüfboss kommt FIXATION in die beste Rotation — an beiden
    /// Zwillingen nicht.**
    ///
    /// Dieselbe Erhebung wie die Zählung über die Bosse (`erhebe_boss`): Länge
    /// [`MAX_LEN`], ohne Ausrüstung, nur Schaden. **Gewirkt allein genügt
    /// nicht:** Ein FIXATION-Zauber, der nur eine Wartezeit belegt, stünde auch
    /// in der Liste (so an `boss-4-3`, siehe `fixation_zeugen`). Verlangt ist,
    /// dass die Liste den Schlag abwendet: Der Spieler überlebt, und ein
    /// Boss-Zauber wird unterbrochen — den Schlag macht nur PIN unterbrechbar.
    ///
    /// Die Zwillinge unterscheiden sich vom Prüfboss in je EINER Zahl: dem
    /// Zeitpunkt (`test-lethal-strike`, 12 s) und dem Schaden (4600, gebildet
    /// mit [`mit_schlag`]). Bleibt FIXATION dort draußen, hängt die Wahl an der
    /// Anforderung und nicht an der Suche.
    #[test]
    fn ac3_am_pruefboss_kommt_fixation_in_die_beste_rotation() {
        let pool = all_candidates();
        let spielbar = spielbare_zauber();
        let pruefboss = boss(PRUEFBOSS);

        let mit = erhebe_boss(&pool, &spielbar, WornValues::none(), pruefboss);
        assert!(
            !fixation_gewirkt(&mit).is_empty(),
            "{PRUEFBOSS}: kein FIXATION-Zauber in der besten Rotation, gewirkt: {}",
            namen(&mit.gewirkt)
        );
        assert!(
            !mit.tot && mit.unterbrochen > 0,
            "{PRUEFBOSS}: FIXATION {} wendet den Schlag nicht ab — tot {}, unterbrochen {}",
            namen(&fixation_gewirkt(&mit)),
            mit.tot,
            mit.unterbrochen
        );

        let schaden_zwilling = mit_schlag(pruefboss, pruefboss.timeline[0].at_ms, UEBERLEBBAR);
        for (name, zwilling) in [
            (ZEIT_ZWILLING, boss(ZEIT_ZWILLING)),
            ("Schaden-Zwilling", schaden_zwilling),
        ] {
            let ohne = erhebe_boss(&pool, &spielbar, WornValues::none(), zwilling);
            assert!(
                fixation_gewirkt(&ohne).is_empty(),
                "{name}: FIXATION {} auch ohne die Anforderung — dann haengt die Wahl nicht \
                 an ihr",
                namen(&fixation_gewirkt(&ohne))
            );
        }
    }

    /// **Die Untergrenze hält auf dem heutigen Stand** (#550, Variante B).
    ///
    /// Derselbe Aufruf wie `main` ohne Schalter: `erhebe` über die vier Bosse,
    /// ohne Ausrüstung. Er läuft hier und nicht nur im Nightly, weil der
    /// Nightly erst am Morgen nach dem Merge rot wird — am 2026-10-28 kamen so
    /// zwei Änderungen am Entscheidungsmodell (#1135, #1154) unbemerkt durch.
    ///
    /// `cargo test` baut Beispiele nur; ausgeführt wird dieser Fall von
    /// `cargo test --all-targets` (CI-Stufe mit `--features diagnostics`).
    #[test]
    fn die_untergrenze_haelt_ohne_ausruestung() {
        let pool = all_candidates();
        let spielbar = spielbare_zauber();
        let namen = irgendwo_gewirkt(&erhebe(&pool, &spielbar, WornValues::none()), &spielbar);
        assert!(
            namen.len() >= MINDESTENS,
            "nur {} von {} spielbaren Zaubern ueber die vier Bosse gewirkt, erwartet mindestens \
             {MINDESTENS}: {}",
            namen.len(),
            spielbar.len(),
            namen.join(", ")
        );
    }
}
