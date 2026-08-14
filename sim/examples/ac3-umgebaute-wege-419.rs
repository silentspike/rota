//! #419 AC3: Gewinnen die VIER UMGEBAUTEN Wege jetzt irgendwo?
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
//!   --example ac3-umgebaute-wege-419
//! ```
//!
//! # Warum dieser Laeufer noetig ist
//!
//! Am 2026-08-13 bekamen vier Wege eine andere Bauart, weil sie gegen ihren
//! Nachbarn im selben Baum verloren (Commit 347395f):
//!
//! ```text
//!   putrefaction build      hebt die Stapel-OBERGRENZE
//!   putrefaction duration   hebt den SCHUTZ je Stapel
//!   sublimation  reduction  hebt EFFLUX' UMWANDLUNG
//!   global       build      hebt den Schaden der AUFBAU-Zauber
//! ```
//!
//! Der Umbau sollte AC3 fuer sie erfuellen. **Gemessen wurde das nie.** Der
//! bestehende Fall `ac3_wege_gewinnen.rs` prueft weiterhin nur CALCINATION und
//! den globalen Baum, und die Liste in seinem Kopf beschreibt den Zustand VOR
//! dem Umbau.
//!
//! # Warum die Rotationen GESUCHT und nicht geschrieben werden
//!
//! Der erste Anlauf dieses Laeufers benutzte von Hand gebaute Listen. Er
//! meldete PUTREFACTION 68 % und SUBLIMATION 90 % zurueck — falsch, aus zwei
//! Gruenden, die beide bei mir lagen:
//!
//! ```text
//!   canker     15 Wirkungen     die Liste "canker/blight/fester/kindle"
//!   blight     15 Wirkungen     fuehrte ihren eigenen Kern NIE aus:
//!   fester      0 Wirkungen     die beiden oberen waren immer bereit
//!   kindle      0 Wirkungen
//! ```
//!
//! Dazu waren es REINE Schul-Listen, obwohl die Zauberleiste alle Schulen
//! fasst und SUBLIMATION-Zauber den Schaden gar nicht selbst tragen.
//!
//! **Wer Rotationen von Hand baut, bis eine passt, misst seine Ausdauer.** Die
//! Suche (`greedy_from`, aus #558) findet zu einem vorgegebenen Startpunkt die
//! beste Liste — sie kann nicht zugunsten einer These schlecht raten.
//!
//! # Der Aufbau in drei Schritten
//!
//! ```text
//!   1  freie Suche                    -> Bezugspunkt "beste Rotation"
//!   2  Suche MIT dem Zauber als Start -> liegt sie innerhalb der 10 %?
//!   3  in dieser Rotation:            -> gewinnt der Weg gegen die Nachbarn?
//! ```
//!
//! Erst wenn Schritt 2 durchfaellt, ist „der Weg ist tot" eine Aussage ueber
//! die Sache. Die Schwelle von 10 % steht in GAME-DESIGN 20.3.
//!
//! **Zu Schritt 2 gehoert eine Warnung, die in `rotation_search.rs` steht:**
//! Eine Suche von einem vorgegebenen Startpunkt kann SCHLECHTER ausfallen als
//! die freie. Der Vergleich mit dem Bezugspunkt aus Schritt 1 ist deshalb
//! Pflicht und nicht Beiwerk.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of};
use rota_sim::rotation_search::{all_candidates, greedy, greedy_from, Candidate};
use rota_sim::{Fight, FightConfig};

/// Fuenf Raenge, die Promille aus `data/player/talents.json`.
const RAENGE: u32 = 5;
/// Wie lang eine gesuchte Rotation hoechstens wird.
const MAX_LEN: usize = 8;

fn config(setzen: impl FnOnce(&mut BalanceConfig)) -> BalanceConfig {
    let mut c = BalanceConfig {
        iterations: 20,
        base_seed: 5,
        threads: 2,
        stop_on_kill: false,
        duration_ms: Some(60_000),
        boss: boss_by_id("boss-1-1").expect("boss-1-1 fehlt"),
        ..BalanceConfig::default()
    };
    setzen(&mut c);
    c
}

/// Der erste Kandidat zu einem Zauber — die Zeile ohne Bedingung.
fn zeile(id: &str, pool: &[Candidate]) -> Candidate {
    let index = index_of(id).unwrap_or_else(|| panic!("Zauber {id} fehlt"));
    pool.iter()
        .find(|k| k.spell == index)
        .cloned()
        .unwrap_or_else(|| panic!("kein Kandidat fuer {id}"))
}

/// Ein Weg: sein Name, der Schalter, und der Zauber, den er ueberhaupt braucht.
struct Weg {
    name: &'static str,
    setzen: fn(&mut BalanceConfig),
    /// Ohne diesen Zauber in der Liste kann der Weg nichts bewirken.
    traeger: &'static str,
}

fn main() {
    let pool = all_candidates();

    // --- Schritt 1: der Bezugspunkt ---------------------------------------
    let frei = greedy(&config(|_| {}), &pool, MAX_LEN);
    let beste = frei.best.mean;
    println!("== Schritt 1: die freie Suche ==\n");
    println!("  bester Schaden ohne Vorgabe: {beste:.0}");
    println!("  Kandidaten im Pool: {}", frei.pool_size);
    println!(
        "  Schwelle fuer Spielbarkeit: {:.0} (10 % darunter)\n",
        beste * 0.9
    );

    let baeume: [(&str, [Weg; 3]); 3] = [
        (
            "PUTREFACTION",
            [
                Weg {
                    name: "build (Deckel)",
                    setzen: |c| c.talent_decay_cap_bonus = 2,
                    traeger: "putrid-bloom",
                },
                Weg {
                    name: "effect",
                    setzen: |c| c.talent_decay_effect_permille = 100 * RAENGE,
                    traeger: "fester",
                },
                Weg {
                    name: "duration (Schutz)",
                    setzen: |c| c.talent_decay_ward_permille = 60 * RAENGE,
                    traeger: "decay",
                },
            ],
        ),
        (
            "SUBLIMATION",
            [
                Weg {
                    name: "reduction (Umwandlung)",
                    setzen: |c| c.talent_convert_permille = 100 * RAENGE,
                    traeger: "efflux",
                },
                Weg {
                    name: "charges",
                    setzen: |c| c.talent_cooldown_charges_bonus = RAENGE,
                    traeger: "reflux",
                },
                Weg {
                    name: "threshold",
                    setzen: |c| c.talent_cooldown_threshold_permille = 100 * RAENGE,
                    traeger: "vapour",
                },
            ],
        ),
        (
            "GLOBAL",
            [
                Weg {
                    name: "build (Aufbau)",
                    setzen: |c| c.talent_build_damage_permille = 40 * RAENGE,
                    traeger: "kindle",
                },
                Weg {
                    name: "release",
                    setzen: |c| c.talent_spender_damage_permille = 40 * RAENGE,
                    traeger: "pyre",
                },
                Weg {
                    name: "hold",
                    setzen: |c| c.talent_dot_duration_permille = 100 * RAENGE,
                    traeger: "smoulder",
                },
            ],
        ),
    ];

    for (baum, wege) in &baeume {
        println!("== {baum} ==\n");
        for (i, weg) in wege.iter().enumerate() {
            // --- Schritt 2: die beste Rotation MIT dem Traeger-Zauber ------
            let start = [zeile(weg.traeger, &pool)];
            let mit = greedy_from(&config(|_| {}), &pool, MAX_LEN, &start);
            let abstand = (beste - mit.best.mean) / beste * 100.0;

            // **Die Zusicherung, ohne die alles darunter wertlos waere:** Wird
            // der Traeger in der gefundenen Liste ueberhaupt GEWIRKT?
            //
            // Gemessen, warum sie noetig ist: Die Suche laesst einen Zauber,
            // der nichts beitraegt, an letzter Stelle stehen — er ist dann in
            // der Liste und kommt trotzdem nie dran, weil die Zeilen darueber
            // immer bereit sind. Fuenf von sechs Traegern ergaben dabei EXAKT
            // denselben Schaden (10729), was der eigentliche Hinweis war:
            // gleiche Zahlen, wo verschiedene stehen muessten.
            //
            // Dasselbe Muster wie in #558, wo ein ans Ende geschobenes Paar
            // fuer sieben Verstaerker 9742 ergab.
            let index = index_of(weg.traeger).expect("Traeger");
            let mut probe = Fight::new(FightConfig {
                seed: 5,
                boss: boss_by_id("boss-1-1").expect("boss-1-1 fehlt"),
                duration_ms: Some(60_000),
                stop_on_kill: false,
                ..FightConfig::default()
            });
            let wirkungen = probe.run_apl(&mit.best.apl).casts[index];
            let spielbar = abstand <= 10.0 && wirkungen > 0;

            println!(
                "  {:<24} Traeger {:<14} beste Liste damit: {:>7.0}  ({abstand:>5.1} % zurueck)                   Wirkungen {wirkungen:>3}  {}",
                weg.name,
                weg.traeger,
                mit.best.mean,
                if spielbar { "spielbar" } else { "FAELLT DURCH" }
            );

            if wirkungen == 0 {
                println!(
                    "      -> Der Traeger steht in der Liste und wird NIE gewirkt. Die Suche \
                     laesst ihn an letzter Stelle, weil er nichts beitraegt — dieser Weg hat \
                     in einer schadensoptimalen Rotation keine Gelegenheit."
                );
                continue;
            }

            if !spielbar {
                println!(
                    "      -> AC3 fuer diesen Weg NICHT belegbar: es gibt keine spielbare \
                     Rotation, in der sein Traeger ueberhaupt vorkommt."
                );
                continue;
            }

            // --- Schritt 3: gewinnt er dort gegen seine Nachbarn? ----------
            let werte: Vec<f64> = wege
                .iter()
                .map(|w| {
                    let mut c = config(w.setzen);
                    c.iterations = 60;
                    rota_sim::balance::run(&c, &mit.best.apl).damage.mean
                })
                .collect();
            let (sieger, _) = werte
                .iter()
                .enumerate()
                .fold(
                    (0, f64::MIN),
                    |(bi, bv), (j, v)| {
                        if *v > bv {
                            (j, *v)
                        } else {
                            (bi, bv)
                        }
                    },
                );
            let liste: Vec<String> = wege
                .iter()
                .zip(werte.iter())
                .map(|(w, v)| format!("{}={v:.0}", w.name.split(' ').next().unwrap_or(w.name)))
                .collect();
            println!(
                "      in seiner eigenen besten Rotation: {}",
                liste.join("  ")
            );
            println!(
                "      -> Sieger: {}{}",
                wege[sieger].name,
                if sieger == i {
                    "   AC3 ERFUELLT fuer diesen Weg"
                } else {
                    ""
                }
            );
        }
        println!();
    }
}
