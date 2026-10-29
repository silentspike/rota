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
use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::{
    distribute_budget, WornValues, QUALITY_HAS_EFFECT, QUALITY_LEGENDARY, QUALITY_NAMES,
    QUALITY_RARE, SLOT_COUNT,
};
use rota_sim::rotation_search::{
    all_candidates, boss_kandidaten, greedy, greedy_from, Candidate, SearchResult,
};
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

/// Sucht je Boss die beste Rotation und trägt ein, welche Zauber im LAUF
/// vorkamen.
///
/// **Was im Lauf vorkam, nicht was in der Liste steht.** Bei `boss-1-3` stand
/// `kindle` am 2026-08-14 in der besten Liste und wurde trotzdem nie gewirkt:
/// Die Bedingungen davor greifen immer. Wer die Liste zählte statt den Lauf,
/// bekäme dort einen Zauber zu viel.
fn erhebe(
    pool: &[Candidate],
    spielbar: &[usize],
    irgendwo_gewirkt: &mut [bool],
    worn: WornValues,
    laut: bool,
) {
    for boss_id in BOSSE {
        let cfg = config(boss_id, worn);
        let (gefunden, bestliste_gewinnt, wert_frei, wert_best) = beste_liste(&cfg, pool, boss_id);

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
            let quelle = if bestliste_gewinnt {
                "Bestliste"
            } else {
                "freie Suche"
            };
            println!(
                "  {boss_id:<10} {hier:>2} von {} gewirkt   {quelle:<11}  frei {wert_frei:.1}, \
                 von der Bestliste {wert_best:.1}",
                spielbar.len()
            );
        }
    }
}

/// Die spielbaren Zauber als Indizes in `SPELLS` — die Grundmenge der Zahl.
fn spielbare_zauber() -> Vec<usize> {
    (0..rota_sim::SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
        .collect()
}

fn main() {
    println!("== Zauber-Abdeckung der ersten Instanz (Issue #550, AC5) ==");
    println!();

    let pool = all_candidates();
    let spielbar = spielbare_zauber();
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut gewirkt = vec![false; rota_sim::SPELL_COUNT];
        erhebe(&pool, &spielbar, &mut gewirkt, WornValues::none(), false);
        let namen: Vec<&str> = spielbar
            .iter()
            .filter(|&&i| gewirkt[i])
            .map(|&i| SPELLS[i].id)
            .collect();
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
