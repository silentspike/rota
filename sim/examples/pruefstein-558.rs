//! Den Pruefstein aus #558 reproduzieren, BEVOR daran gearbeitet wird.
//!
//! # Warum dieser Laeufer existiert
//!
//! Der Orchestrator hat mir ein Fertigkriterium als Zahl gegeben: Bei
//! Gegenstandsstufe 7, Qualitaet RARE, muss die Suche mindestens **15593**
//! erreichen; `greedy` holt dort **12671**. Gemessen hat das Worker-1 mit
//! einem Laeufer namens `stufen-durchsatz` — **und der liegt in keinem
//! Zweig**, weder in `main` noch in `mess/e1-791-gegenstandsstufen`.
//!
//! Gegen eine Zahl zu optimieren, deren Messaufbau ich nicht kenne, ist der
//! sicherste Weg zu einem Ergebnis, das keiner nachvollziehen kann: Ich
//! koennte 15593 „erreichen" und dabei etwas voellig anderes messen.
//!
//! # Was schon feststeht, ohne einen einzigen Lauf
//!
//! Die Budget-Reihe ist zuzuordnen. `QUALITY_BASE_BUDGET[RARE] = 140`, und
//! `budget_of` waechst je Stufe um 21/20 mit Ganzzahl-Division:
//!
//! ```text
//!   Stufe   1    7   13
//!   meine 140  185  245
//!   W-1   140  185  245
//! ```
//!
//! Damit ist belegt: Worker-1 rechnet mit `budget_of(item_level,
//! QUALITY_RARE)`. Offen ist nur, wie er das Budget auf [`WornValues`]
//! verteilt — und genau das misst dieser Laeufer, statt es zu raten.
//!
//! # Wie er das tut
//!
//! Er faehrt mehrere naheliegende Verteilungen desselben Budgets und druckt
//! je Variante den Durchsatz. **Die Variante, die bei Stufe 1 die 13655
//! trifft, ist Worker-1s Aufbau.** Trifft keine, ist das selbst der Befund —
//! dann fehlt mir sein Laeufer wirklich, und ich sage es, statt eine Zahl zu
//! erfinden, die zufaellig passt.
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- run --release --example pruefstein-558
//! ```

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::data::SPELLS;
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::{budget_of, distribute_budget, WornValues, QUALITY_AFFIX_SLOTS, QUALITY_RARE};
use rota_sim::rotation_search::{all_candidates, greedy, greedy_from, greedy_opts};
use rota_sim::sources::{mixed_build, permanent_max, pure_build, STAT_CATALYSIS, STAT_PURITY};

/// Eine benannte Verteilung des Budgets auf die getragenen Werte.
type Variante = (&'static str, fn(u32) -> WornValues);

/// Die Stufen, die Worker-1 gemeldet hat.
const STUFEN: &[u32] = &[1, 7, 13];

/// Was Worker-1 gemessen hat — als Zahl hier, damit der Vergleich im Bericht
/// steht und nicht im Kopf des Lesers.
const W1_DURCHSATZ: &[(u32, u64)] = &[(1, 13655), (7, 12671), (13, 19131)];
const W1_BUDGET: &[(u32, u64)] = &[(1, 140), (7, 185), (13, 245)];

/// Dieselben Einstellungen wie `rotation-search`, damit die Zahlen dorthin
/// passen.
const RUNS: u32 = 50;
const SEED: u32 = 0;
const MAX_LEN: usize = 6;

fn config(worn: WornValues) -> BalanceConfig {
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id("first-crucible").expect("first-crucible fehlt"),
        worn,
        ..BalanceConfig::default()
    }
}

/// Das Budget gleichmaessig auf die Affix-Plaetze der Qualitaet, und von dort
/// auf die drei Sonderwerte — die Reihenfolge aus `data/items.json` (`-mix`:
/// catalysis, purity, yield).
fn nach_affix_plaetzen(item_level: u32) -> WornValues {
    let mut plaetze = [0_u64; 4];
    let anzahl = distribute_budget(item_level, QUALITY_RARE, &mut plaetze);
    let mut worn = WornValues::none();
    for (i, &wert) in plaetze.iter().enumerate().take(anzahl) {
        let wert = u32::try_from(wert).unwrap_or(u32::MAX);
        match i % 3 {
            0 => worn.catalysis += wert,
            1 => worn.purity += wert,
            _ => worn.yield_bonus += wert,
        }
    }
    worn
}

/// Das ganze Budget auf CATALYSIS, ohne Umweg ueber die Plaetze.
fn alles_auf_catalysis(item_level: u32) -> WornValues {
    WornValues {
        catalysis: u32::try_from(budget_of(item_level, QUALITY_RARE)).unwrap_or(u32::MAX),
        ..WornValues::none()
    }
}

/// Das ganze Budget zu gleichen Teilen auf die drei Sonderwerte.
fn budget_gedrittelt(item_level: u32) -> WornValues {
    let drittel = u32::try_from(budget_of(item_level, QUALITY_RARE) / 3).unwrap_or(u32::MAX);
    WornValues {
        catalysis: drittel,
        purity: drittel,
        yield_bonus: drittel,
        ..WornValues::none()
    }
}

fn durchsatz(worn: WornValues) -> f64 {
    let pool = all_candidates();
    greedy(&config(worn), &pool, MAX_LEN).best.mean
}

fn main() {
    println!("== Pruefstein #558: Worker-1s Messaufbau rekonstruieren ==");
    println!("Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={MAX_LEN}, Boss=first-crucible");
    println!(
        "           Qualitaet RARE ({} Affix-Plaetze)",
        QUALITY_AFFIX_SLOTS[QUALITY_RARE]
    );
    println!();

    // **Der Anker zuerst.** Passt schon die Budget-Reihe nicht, ist jede
    // Durchsatz-Zahl darunter ein Vergleich zweier verschiedener Sachen.
    println!("Budget je Stufe (RARE)");
    let mut budget_passt = true;
    for &(stufe, erwartet) in W1_BUDGET {
        let meins = budget_of(stufe, QUALITY_RARE);
        let urteil = if meins == erwartet {
            "passt"
        } else {
            budget_passt = false;
            "WEICHT AB"
        };
        println!("  Stufe {stufe:<3} meins {meins:<5} Worker-1 {erwartet:<5} {urteil}");
    }
    println!();
    assert!(
        budget_passt,
        "die Budget-Reihe stimmt nicht mit Worker-1 ueberein — dann messe ich eine \
         andere Groesse als er, und jeder Durchsatz-Vergleich darunter waere wertlos"
    );

    println!("Durchsatz je Verteilung — welche trifft Worker-1s Zahl?");
    println!(
        "  {:<26} {:>10} {:>10} {:>10}",
        "Verteilung", "Stufe 1", "Stufe 7", "Stufe 13"
    );
    let varianten: &[Variante] = &[
        ("Affix-Plaetze (items.json)", nach_affix_plaetzen),
        ("alles auf CATALYSIS", alles_auf_catalysis),
        ("Budget gedrittelt", budget_gedrittelt),
        ("pure_build(CATALYSIS)", |l| pure_build(STAT_CATALYSIS, l)),
        ("pure_build(PURITY)", |l| pure_build(STAT_PURITY, l)),
        ("mixed_build(CATALYSIS)", |l| mixed_build(STAT_CATALYSIS, l)),
    ];
    for (name, bauen) in varianten {
        print!("  {name:<26}");
        for &stufe in STUFEN {
            print!("{:>10.0}", durchsatz(bauen(stufe)));
        }
        println!();
    }
    println!("  {:<26}", "Worker-1 gemeldet");
    print!("  {:<26}", "");
    for &(_, wert) in W1_DURCHSATZ {
        print!("{wert:>10}");
    }
    println!();
    println!();

    // Zum Einordnen: `permanent_max` ist eine ANDERE Groesse als `budget_of`,
    // und beide heissen im Gespraech „Budget". Wer sie verwechselt, vergleicht
    // zwei Zahlen, die nie zusammengehoert haben.
    println!("Zum Abgrenzen: permanent_max(CATALYSIS) je Stufe");
    for &stufe in STUFEN {
        println!(
            "  Stufe {stufe:<3} permanent_max {:<6} budget_of {}",
            permanent_max(STAT_CATALYSIS, stufe),
            budget_of(stufe, QUALITY_RARE)
        );
    }
    println!();

    paar_suche_gegen_einzel();
    traegt_das_paar_ueberhaupt();
}

/// **Die Frage, ohne die der Befund oben nichts wert ist.**
///
/// Die Paar-Suche findet nichts — 225 Paare geprueft, keines gewinnt. Daraus
/// folgt aber NICHT, dass die Verstaerker nichts taugen. Zwei Lagen sehen von
/// aussen gleich aus:
///
/// | Lage | was zu tun waere |
/// | --- | --- |
/// | Es ist nichts zu finden | Balance-Befund: die Verstaerker sind zu schwach |
/// | Es ist da, die Suche findet es nicht | Werkzeug-Befund: mein Verfahren ist zu eng |
///
/// Getrennt werden sie nur von HAND: dieselbe Liste einmal mit und einmal ohne
/// das Paar, beide fest geschrieben, ohne jede Suche. Ist die Liste mit Paar
/// besser, existiert der Gewinn — und meine Suche hat ihn uebersehen.
fn traegt_das_paar_ueberhaupt() {
    println!("== Traegt das Paar ueberhaupt? (von Hand, ohne Suche) ==");
    println!();

    // Die gefundene Rotation als Grundlage, damit der Vergleich auf dem steht,
    // was die Suche fuer das Beste haelt.
    let basis = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nkindle\n";
    let nur_verstaerker = "pyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nflashpoint\nkindle\n";
    // **Die Fenster-Zeile gehoert nach OBEN.** Mein erster Entwurf setzte sie
    // direkt ueber das unbedingte `kindle`:
    //
    //     flashpoint
    //     kindle if buff.flashpoint.active
    //     kindle
    //
    // Beide Zeilen wirken denselben Zauber, und die untere faengt ohnehin
    // alles ab — die Fenster-Zeile konnte gar nichts aendern. Gemessen kam
    // zweimal exakt 8621 heraus, und das war der Hinweis: Ein Unterschied von
    // NULL zwischen zwei angeblich verschiedenen Listen ist keine Messung,
    // sondern ein Aufbaufehler.
    //
    // Eine Fenster-Zeile ist eine PRIORITAET. Sie kann nur etwas bewirken, wo
    // sie etwas verdraengt.
    let paar_oben = "flashpoint\nkindle if buff.flashpoint.active\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nkindle\n";
    // Und dieselbe Anordnung OHNE die Fenster-Zeile — damit der Unterschied
    // der Zeile gehoert und nicht der neuen Position von `flashpoint`.
    let verstaerker_oben = "flashpoint\npyre if heat >= 15\nsearing if dot.searing.remains < 1500\nrot\nsmoulder if dot.smoulder.remains < 1500\nkindle\n";

    let messen = |text: &str| -> f64 {
        let apl = rota_sim::apl::parse(rota_sim::apl::AplName("hand"), text).expect("APL gueltig");
        rota_sim::balance::run(&config(WornValues::none()), &apl)
            .damage
            .mean
    };

    let a = messen(basis);
    let c = messen(nur_verstaerker);
    let d = messen(verstaerker_oben);
    let b = messen(paar_oben);
    println!("  ohne alles                  {a:>9.0}");
    println!(
        "  FLASHPOINT unten            {c:>9.0}   ({:+.1} %)",
        (c - a) / a * 100.0
    );
    println!(
        "  FLASHPOINT oben             {d:>9.0}   ({:+.1} %)",
        (d - a) / a * 100.0
    );
    println!(
        "  FLASHPOINT oben + Fenster   {b:>9.0}   ({:+.1} %)",
        (b - a) / a * 100.0
    );
    println!(
        "  davon die Fenster-Zeile:    {:+.1} %  (gegen FLASHPOINT oben)",
        (b - d) / d * 100.0
    );
    println!();
    // **Die Zusicherung gegen den Aufbaufehler von vorhin.** Zwei Listen, die
    // sich in einer Zeile unterscheiden und exakt dasselbe liefern, haben
    // nichts gemessen.
    assert!(
        (b - d).abs() > f64::EPSILON,
        "die Fenster-Zeile aendert exakt nichts — dann steht sie wieder an einer \
         Stelle, an der sie nichts verdraengen kann, und dieser Vergleich misst nicht, \
         was er behauptet"
    );
    if b > a {
        println!("  -> Der Gewinn EXISTIERT. Meine Suche hat ihn uebersehen: Werkzeug-Befund.");
    } else {
        println!("  -> Der Gewinn existiert an dieser Stelle NICHT. Dann ist „kein Paar");
        println!("     gefunden\" die richtige Antwort, und die Frage ist eine der Balance:");
        println!("     ein Verstaerker, der einen GCD kostet und ihn nicht einspielt.");
    }
    println!();

    alle_verstaerker();
}

/// **Ein Verstaerker ist kein Befund.** Alle sieben, an zwei Kampflaengen.
///
/// Die Messung oben gilt fuer FLASHPOINT an `first-crucible` (60 s). Daraus
/// „die Verstaerker tragen nicht" zu folgern, waere derselbe Fehlschluss, den
/// dieses Issue aufklaeren soll — nur in die andere Richtung.
///
/// Zwei Achsen, an denen es kippen koennte:
///
/// 1. **Der Verstaerker.** DISTIL und AETHER wirken auf Abklingzeiten, SLAG
///    auf die Vorrats-Obergrenze — andere Wirkwege als FLASHPOINTs
///    Ressourcen-Gewinn.
/// 2. **Die Kampflaenge.** Ein Verstaerker mit 60 s Abklingzeit kommt in einem
///    60-s-Kampf einmal dran, in einem 300-s-Kampf fuenfmal. Der GCD-Preis
///    faellt einmal an, der Nutzen mehrfach.
fn alle_verstaerker() {
    println!("== Alle Verstaerker, zwei Kampflaengen (von Hand) ==");
    println!();
    println!("  Der Nutzer kommt aus den DATEN, nicht aus einer Liste: der erste");
    println!("  spielbare Zauber, den `candidates_for` zu diesem Verstaerker paart.");
    println!();

    let basis_zeilen = [
        "pyre if heat >= 15",
        "searing if dot.searing.remains < 1500",
        "rot",
        "smoulder if dot.smoulder.remains < 1500",
        "kindle",
    ];

    for boss_id in ["first-crucible", "test-metronome"] {
        let boss = boss_by_id(boss_id).expect("Boss fehlt");
        let cfg = |apl_text: &str| -> f64 {
            let apl =
                rota_sim::apl::parse(rota_sim::apl::AplName("hand"), apl_text).expect("gueltig");
            let mut c = config(WornValues::none());
            c.boss = boss;
            rota_sim::balance::run(&c, &apl).damage.mean
        };
        let basis = format!("{}\n", basis_zeilen.join("\n"));
        let ohne = cfg(&basis);
        println!("  Boss {boss_id} — ohne Verstaerker {ohne:.0}");
        println!(
            "    {:<14} {:>10} {:>10} {:>10}",
            "Verstaerker", "oben", "+Fenster", "bester"
        );

        for (v_index, v) in SPELLS.iter().enumerate() {
            if v.buff.is_none() || v.unlock_level > 100 {
                continue;
            }
            // Der Partner: irgendeine BuffWindow-Zeile, die genau diesen
            // Verstaerker nennt. Sie stammt aus `candidates_for`, also aus den
            // Daten — nicht aus meiner Vorstellung, was zusammenpasst.
            let nutzer = all_candidates().into_iter().find(|k| {
                k.origin == rota_sim::rotation_search::Origin::BuffWindow
                    && k.rule().to_string().contains(&format!("buff.{}.", v.id))
            });
            let Some(nutzer) = nutzer else { continue };

            let mut oben = vec![v.id.to_owned()];
            oben.extend(basis_zeilen.iter().map(|z| (*z).to_owned()));
            let wert_oben = cfg(&format!("{}\n", oben.join("\n")));

            let mut mit_fenster = vec![v.id.to_owned(), nutzer.rule().to_string()];
            mit_fenster.extend(basis_zeilen.iter().map(|z| (*z).to_owned()));
            let wert_fenster = cfg(&format!("{}\n", mit_fenster.join("\n")));

            let bester = wert_oben.max(wert_fenster);
            let zeichen = if bester > ohne { "  GEWINNT" } else { "" };
            println!(
                "    {:<14} {:>10.0} {:>10.0} {:>+9.1} %{zeichen}",
                v.id,
                wert_oben,
                wert_fenster,
                (bester - ohne) / ohne * 100.0
            );
            let _ = v_index;
        }
        println!();
    }

    verstaerker_in_bester_umgebung();
}

/// **Der faire Test:** das Paar erzwingen, die Umgebung suchen lassen.
///
/// # Was an der Messung davor falsch war
///
/// Sie setzte jeden Verstaerker in dieselbe CALCINATION-Basis. Aber
/// `virulence` verstaerkt die Wertigkeit von STAPELN, `griplock`
/// Kontroll-Punkte, `quicksilver` die Umwandlung in Schild, `slag` die
/// Vorrats-Obergrenze. Ein Verstaerker auf einen Wirkweg, den die Rotation
/// gar nicht benutzt, kann nicht gewinnen — er kostet nur den GCD.
///
/// Fuer vier von sieben mass ich damit, dass etwas nicht wirkt, das an dieser
/// Stelle nicht wirken KANN. Nur FLASHPOINT (Ressourcen-Gewinn) stand in
/// einer Umgebung, die seine Verstaerkung wirklich nutzt.
///
/// # Wie es richtig geht
///
/// Das Paar wird als Startpunkt ERZWUNGEN, und dieselbe Suche baut die beste
/// Liste darum herum — mit `greedy_from`, dem Baustein aus dem Nachlauf.
/// Verglichen wird gegen die freie Suche derselben Laenge.
///
/// | Lauf | was er misst |
/// | --- | --- |
/// | `greedy` | das Beste ohne Zwang |
/// | `greedy_from([verstaerker, nutzer])` | das Beste MIT dem Paar |
///
/// Traegt das Paar irgendwo, ist die zweite Zahl groesser. Ist sie es nie,
/// ist der Befund ein Balance-Befund und kein Werkzeug-Befund — und zwar
/// einer, der die beste Umgebung gesehen hat, die das Verfahren bauen kann.
///
/// # Ein Zwang, der keiner ist — und warum das Ergebnis trotzdem gilt
///
/// Sieben Verstaerker lieferten an `first-crucible` exakt dieselbe Zahl wie
/// die freie Suche: 9742. Identische Zahlen ueber verschiedene Eingaben sind
/// nie ein Ergebnis, sondern ein Hinweis. Die ausgeschriebene Liste zeigt,
/// worauf:
///
/// ```text
///   pyre if heat >= 15
///   ... sechs weitere Zeilen ...
///   kindle
///   flashpoint                          <- ans ENDE geschoben
///   kindle if buff.flashpoint.active
/// ```
///
/// **Die Suche kann ein erzwungenes Paar nicht ablehnen, aber sie kann es
/// neutralisieren.** Sie fuegt ihre Zeilen an jeder Position ein, auch DAVOR;
/// das Paar rutscht dorthin, wo das unbedingte `kindle` ohnehin alles abfaengt
/// — und ist damit folgenlos. Die exakt gleiche Zahl ist der Beleg dafuer.
///
/// Das entwertet den Befund nicht, es schaerft ihn: Die Suche hat JEDE
/// Position geprueft und keine gefunden, an der das Paar besser waere als
/// unten. An `test-metronome`, wo die Laengengrenze das Wegschieben verhindert
/// (12 Zeilen voll), kostet dasselbe Paar 11,9 % — dort verdraengt es zwei
/// Zeilen, die etwas beitragen.
fn verstaerker_in_bester_umgebung() {
    println!("== Jeder Verstaerker in seiner BESTEN Umgebung (erzwungenes Paar) ==");
    println!();

    let pool = all_candidates();
    for boss_id in ["first-crucible", "test-metronome"] {
        let boss = boss_by_id(boss_id).expect("Boss fehlt");
        let mut cfg = config(WornValues::none());
        cfg.boss = boss;

        let frei = greedy(&cfg, &pool, 12);
        println!("  Boss {boss_id} — freie Suche {:.0}", frei.best.mean);
        println!(
            "    {:<14} {:>12} {:>10}",
            "Verstaerker", "mit Zwang", "Abstand"
        );

        for v in SPELLS.iter() {
            if v.buff.is_none() || v.unlock_level > 100 {
                continue;
            }
            let verstaerker_zeile = pool.iter().find(|k| {
                SPELLS[k.spell].id == v.id && k.origin == rota_sim::rotation_search::Origin::Always
            });
            let nutzer_zeile = pool.iter().find(|k| {
                k.origin == rota_sim::rotation_search::Origin::BuffWindow
                    && k.rule().to_string().contains(&format!("buff.{}.", v.id))
            });
            let (Some(a), Some(b)) = (verstaerker_zeile, nutzer_zeile) else {
                continue;
            };
            let start = vec![a.clone(), b.clone()];
            let erzwungen = greedy_from(&cfg, &pool, 12, &start);

            // **Die Zusicherung:** Das Paar steht wirklich in der Liste. Ohne
            // sie belegte „kein Gewinn" nichts — die Suche koennte es
            // unterwegs verloren haben.
            let paar_drin = erzwungen.chosen.contains(a) && erzwungen.chosen.contains(b);
            assert!(
                paar_drin,
                "das erzwungene Paar ist aus der Liste verschwunden"
            );

            let abstand = (erzwungen.best.mean - frei.best.mean) / frei.best.mean * 100.0;
            let zeichen = if erzwungen.best.mean > frei.best.mean {
                "  GEWINNT"
            } else {
                ""
            };
            println!(
                "    {:<14} {:>12.0} {:>+9.1} %{zeichen}  ({} Zeilen, Abbruch {:?})",
                v.id,
                erzwungen.best.mean,
                abstand,
                erzwungen.chosen.len(),
                erzwungen.stop
            );
            // **Die erste Liste ausgeschrieben.** Sieben Verstaerker mit
            // exakt derselben Zahl waren der Hinweis, dass hier etwas nicht
            // misst, was es behauptet — und eine Zahl ohne die Liste dahinter
            // laesst sich nicht nachpruefen.
            if v.id == "flashpoint" {
                for zeile in erzwungen.best.apl.to_string().lines() {
                    println!("        {zeile}");
                }
            }
        }
        println!();
    }
}

/// Der zweite Teil: Findet die Paar-Suche, was die Einzel-Suche nicht findet?
///
/// **Das haengt NICHT an Worker-1s Laeufer** und ist deshalb der Teil, den ich
/// ohne ihn belegen kann. Gefragt ist die Eigenschaft selbst: ein Paar, dessen
/// Haelften einzeln verlieren.
fn paar_suche_gegen_einzel() {
    println!("== Paar-Suche gegen Einzel-Suche (Issue #558 AC1/AC3) ==");
    println!();

    // FLASHPOINT verdoppelt den Ressourcen-Gewinn (target=resource, Faktor 2,
    // Abklingzeit 12 s). Der Nutzer ist ein Zauber mit `resourceGain > 0`.
    let pool = all_candidates();
    for &(name, stufen) in &[
        ("nur der Fall selbst", &["flashpoint", "kindle", "pyre"][..]),
        (
            "voller Kandidatenraum",
            &[][..], // leer = `pool` unveraendert
        ),
    ] {
        let teil: Vec<_> = if stufen.is_empty() {
            pool.clone()
        } else {
            pool.iter()
                .filter(|k| stufen.contains(&SPELLS[k.spell].id))
                .cloned()
                .collect()
        };
        // **Zwei Laengen, und die zweite ist die entscheidende.** Bei sechs
        // Zeilen ist die Liste voll, bevor die Einzelzuege aufhoeren zu
        // tragen — das Paar kommt dann NIE zur Wahl, und „kein Unterschied"
        // saehe genauso aus wie „geprueft und verloren".
        for laenge in [MAX_LEN, 12] {
            let ohne = greedy_opts(&config(WornValues::none()), &teil, laenge, &[], false);
            let mit = greedy_opts(&config(WornValues::none()), &teil, laenge, &[], true);

            println!(
                "  {name} ({} Kandidatenzeilen, Laenge {laenge})",
                teil.len()
            );
            println!(
                "    ohne Paare {:>9.0}   ({} Messungen, Abbruch {:?})",
                ohne.best.mean, ohne.measured, ohne.stop
            );
            println!(
                "    mit Paaren {:>9.0}   ({} Messungen, {} Paare geprueft, Abbruch {:?})",
                mit.best.mean, mit.measured, mit.paare_geprueft, mit.stop
            );
            let zuwachs = (mit.best.mean - ohne.best.mean) / ohne.best.mean * 100.0;
            println!("    Unterschied {zuwachs:+.1} %");
            println!("    ohne Paare gefunden:");
            for zeile in ohne.best.apl.to_string().lines() {
                println!("      {zeile}");
            }
            println!("    mit Paaren gefunden:");
            for zeile in mit.best.apl.to_string().lines() {
                println!("      {zeile}");
            }
            println!();
        }
    }
}
