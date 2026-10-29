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
//! # Kosten
//!
//! Gemessen am 2026-08-14: **21 s** auf rustbuild (`--release`, vier Bosse,
//! `--max-len 6`, je 2258 Listen). Das ist der Grund, warum AC5 einen Wächter
//! bekommt und nicht den Absatz „zu teuer" — die Alternative stand in der AC
//! ausdrücklich offen.
use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::{
    distribute_budget, WornValues, QUALITY_HAS_EFFECT, QUALITY_LEGENDARY, QUALITY_NAMES,
    QUALITY_RARE, SLOT_COUNT,
};
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::sources::gear_effects_permille;

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

fn config(boss_id: &str, worn: WornValues) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
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

/// Sucht je Boss die beste Rotation und trägt ein, welche Zauber im LAUF
/// vorkamen.
///
/// **Was im Lauf vorkam, nicht was in der Liste steht.** Bei `boss-1-3` stand
/// `kindle` am 2026-08-14 in der besten Liste und wurde trotzdem nie gewirkt:
/// Die Bedingungen davor greifen immer. Wer die Liste zählte statt den Lauf,
/// bekäme dort einen Zauber zu viel.
fn erhebe(
    pool: &[rota_sim::rotation_search::Candidate],
    spielbar: &[usize],
    irgendwo_gewirkt: &mut [bool],
    worn: WornValues,
    laut: bool,
) {
    for boss_id in BOSSE {
        let cfg = config(boss_id, worn);
        let gefunden = greedy(&cfg, pool, MAX_LEN);

        let boss = boss_by_id(boss_id).expect("Boss");
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            seed: SEED,
            boss,
            worn,
            ..rota_sim::FightConfig::default()
        });
        let lauf = fight.run_apl(&gefunden.best.apl);
        let mut hier = 0;
        for &i in spielbar {
            if lauf.casts[i] > 0 {
                irgendwo_gewirkt[i] = true;
                hier += 1;
            }
        }
        if laut {
            println!("  {boss_id:<10} {hier:>2} von {} gewirkt", spielbar.len());
        }
    }
}

fn main() {
    println!("== Zauber-Abdeckung der ersten Instanz (Issue #550, AC5) ==");
    println!();

    let pool = all_candidates();
    let spielbar: Vec<usize> = (0..rota_sim::SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
        .collect();
    let mut irgendwo_gewirkt: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];

    erhebe(
        &pool,
        &spielbar,
        &mut irgendwo_gewirkt,
        WornValues::none(),
        true,
    );

    let gewirkt: Vec<&str> = spielbar
        .iter()
        .filter(|&&i| irgendwo_gewirkt[i])
        .map(|&i| SPELLS[i].id)
        .collect();

    println!();
    println!(
        "  ueber alle {} Bosse zusammen   {} von {}",
        BOSSE.len(),
        gewirkt.len(),
        spielbar.len()
    );
    println!("    {}", gewirkt.join(", "));
    println!();

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
            let mut mit: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];
            erhebe(&pool, &spielbar, &mut mit, worn, false);
            let zauber: Vec<&str> = spielbar
                .iter()
                .filter(|&&i| mit[i])
                .map(|&i| SPELLS[i].id)
                .collect();
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
            let mut mit: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];
            erhebe(&pool, &spielbar, &mut mit, worn, false);
            let zauber: Vec<&str> = spielbar
                .iter()
                .filter(|&&i| mit[i])
                .map(|&i| SPELLS[i].id)
                .collect();
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

            let mut mit: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];
            erhebe(&pool, &spielbar, &mut mit, worn, false);
            let gewirkt_mit: Vec<&str> = spielbar
                .iter()
                .filter(|&&i| mit[i])
                .map(|&i| SPELLS[i].id)
                .collect();

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

    // Der Handschrift-Vergleich bleibt `rotation-search` überlassen; dieser
    // Läufer hält genau eine Zahl. `AplName` wird nur importiert, damit die
    // Signatur von `greedy` erfüllt ist — ohne Verwendung meldet clippy.
    let _ = AplName("zauber-abdeckung");

    // Der Abbruch der Untergrenze — nach beiden Schalter-Teilen, siehe oben.
    if unterschritten {
        std::process::exit(1);
    }
}
