//! Messlauf für den Bestandsabgleich der Balance-Befunde (#509, #550, #598, #607).
//!
//! # Was das hier ist — und was nicht
//!
//! Das ist **kein Prüffall**. Er sichert keine Eigenschaft zu und wird nicht
//! rot, wenn sich das Spiel ändert. Er ist ein **Messwerkzeug**: Der Simulator
//! hat kein ausführbares Programm (`sim/src/bin/` existiert nicht), also ist
//! ein Test der einzige Weg, den Balance-Läufer mit eigenen Parametern zu
//! fahren und seine Zahlen zu sehen.
//!
//! Aufruf — die Fälle stehen hinter `#[ignore]` und laufen nur auf Zuruf:
//!
//! ```text
//! cargo remote -c -- test --test bestand_balance_messung -- --ignored --nocapture
//! ```
//!
//! # Warum `#[ignore]` hier richtig ist — und in `rotation_search.rs` falsch war
//!
//! `rotation_search.rs:44` warnt ausdrücklich vor genau diesem Attribut:
//!
//! ```text
//!   "Die Faelle liefen zunaechst hinter #[ignore] mit der Begruendung 'misst
//!    tausende Kaempfe'. Gemessen sind es 40 Millisekunden — die Begruendung
//!    war GESCHAETZT UND FALSCH, und die Faelle liefen deshalb in keinem Gate.
//!    Wer das Einfuegen auf Anhaengen zurueckgedreht haette, haette einen
//!    gruenen PR bekommen."
//!
//! Diese Datei ist der begründete Gegenfall, aus drei Gründen:
//!
//! **1. Die Zeit ist gemessen, nicht geschätzt.** Der volle Lauf über alle
//! Fälle brauchte 459,9 s (`--test-threads=1`); `messung_598` allein 153,0 s
//! (vom Orchestrator unabhängig nachgefahren, 152,97 s). Die Einzelfälle:
//! Verstärker-Paare 51,6 s, `flashpoint`-Vorprüfung 46,0 s, AC4-Kosten 74,9 s.
//! Das sind Minuten, keine 40 Millisekunden — der Unterschied zu dort ist
//! nicht die Haltung, sondern die Größenordnung.
//!
//! **2. Hier steht ein Werkzeug, dort standen Prüffälle.** Diese Datei sichert
//! nichts zu: Ihre `assert!`-Zeilen prüfen ausschließlich, dass der Lauf
//! überhaupt misst (nicht-leere Ergebnisse, bewegte Spannen, die erwartete
//! Abbruchbedingung). Keine einzige prüft eine Eigenschaft des Spiels, also
//! kann keine rot werden, wenn sich das Spiel ändert. **Ein Werkzeug in keinem
//! Gate kostet nichts; ein Prüffall in keinem Gate ist eine Lücke.** Genau
//! darin lag der Fehler dort — dort wäre ein Rückbau grün durchgelaufen.
//!
//! **3. Was hier grün ist, belegt nichts.** Wer diese Datei laufen lässt, will
//! die ausgegebenen ZAHLEN lesen, nicht ein Häkchen sehen. Deshalb ist
//! `--nocapture` im Aufruf keine Bequemlichkeit, sondern der Zweck.
//!
//! # Warum die Zahlen der Issues neu gemessen gehören
//!
//! #550 sagt „7 von 37 Zaubern kommen in der besten Rotation vor". Diese Zahl
//! stammt von **vor** Issue #553. Damals konnte die Suche keine Zeile der Art
//! `kindle if buff.flashpoint.active` bilden — der Parser las keine
//! Buff-Fenster, und `Origin::BuffWindow` gab es nicht. Eine Verstärkung auf
//! eine Nebengröße war für die Suche unsichtbar.
//!
//! `rotation_search.rs` sagt das über sich selbst:
//!
//! ```text
//!   "Das ist der Grund, warum #550 unter Vorbehalt stand: Ein Teil des
//!    Befundes konnte ein Werkzeug-Befund sein statt ein Balance-Befund."
//! ```
//!
//! Ein Teil des Befundes war also eine Eigenschaft des Werkzeugs. Wie groß
//! dieser Teil war, sagt nur eine Messung nach #553.
//!
//! # Die Parameter und ihr Preis
//!
//! `iterations` steht auf 200, nicht auf den 1000 des Standards. Die Suche
//! misst pro Runde `Pool × Einfügestellen` Rotationen; über neun Bosse wären
//! 1000 Durchgänge je Messung ein Lauf über Stunden. 200 Durchgänge tragen
//! einen größeren Fehlerbalken — für die Frage „welche Zauber kommen
//! überhaupt vor" reicht das, für einen Schadensvergleich zweier naher
//! Rotationen nicht.

use rota_sim::apl::Value;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, MAX_LEVEL, SPELLS};
use rota_sim::rotation_search::{all_candidates, greedy, measure, Candidate, Origin};

/// Die spielbaren Bosse, gegen die gemessen wird.
///
/// Die `test-*`-Bosse bleiben draußen: Sie tragen 999999 Lebenspunkte oder
/// künstliche Zeitleisten und würden die Frage „welche Rotation gewinnt im
/// Spiel" mit einer Antwort über einen Prüfstand beantworten.
const BOSSE: &[&str] = &[
    "first-crucible",
    "boss-1-1",
    "boss-1-2",
    "boss-1-3",
    "boss-1-4",
    "boss-2-1",
    "boss-2-2",
    "boss-2-3",
    "boss-2-4",
];

/// Wie viele Durchgänge je gemessener Rotation. Siehe Kopf.
const DURCHGAENGE: u32 = 200;

/// Wie lang die gesuchte Rotation höchstens werden darf.
const MAX_LEN: usize = 6;

fn config_fuer(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} fehlt im Datensatz"));
    BalanceConfig {
        iterations: DURCHGAENGE,
        threads: 4,
        // **Nicht beim Sieg abbrechen.** `balance.rs` warnt ausdrücklich: Mit
        // `true` misst der Lauf, OB der Boss fällt, und schneidet den Schaden
        // ab. Vergleichbar werden zwei Rotationen nur über dieselbe Dauer.
        stop_on_kill: false,
        boss,
        ..BalanceConfig::default()
    }
}

/// Die Namen der Zauber einer gefundenen Rotation, in ihrer Reihenfolge.
fn zaubernamen(gewaehlt: &[rota_sim::rotation_search::Candidate]) -> Vec<&'static str> {
    gewaehlt.iter().map(|k| SPELLS[k.spell].id).collect()
}

#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn messung_509_550_welche_zauber_kommen_ueber_die_bosse_vor() {
    let spielbare: Vec<&'static str> = SPELLS
        .iter()
        .filter(|s| s.unlock_level <= MAX_LEVEL as u16)
        .map(|s| s.id)
        .collect();
    let pool = all_candidates();

    println!("\n=== GRUNDMENGE ===");
    println!("  spielbare Zauber: {}", spielbare.len());
    println!("  Kandidatenzeilen: {}", pool.len());
    println!("  Durchgaenge je Messung: {DURCHGAENGE}, Laengengrenze: {MAX_LEN}");

    let mut gesehen: Vec<&'static str> = Vec::new();
    let mut rotationen: Vec<(&str, Vec<&'static str>, f64)> = Vec::new();

    println!("\n=== #509: findet die Suche ueber die Bosse dieselbe Rotation? ===");
    for &boss_id in BOSSE {
        let config = config_fuer(boss_id);
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let namen = zaubernamen(&gefunden.chosen);
        for &n in &namen {
            if !gesehen.contains(&n) {
                gesehen.push(n);
            }
        }
        println!(
            "  {boss_id:<15} {:>9.1}  [{}]  ({} gemessen, Stop {:?})",
            gefunden.best.mean,
            namen.join(" > "),
            gefunden.measured,
            gefunden.stop
        );
        rotationen.push((boss_id, namen, gefunden.best.mean));
    }

    // #509 in einer Zahl: wie viele VERSCHIEDENE Rotationen kommen heraus?
    let mut verschiedene: Vec<&Vec<&'static str>> = Vec::new();
    for (_, namen, _) in &rotationen {
        if !verschiedene.contains(&namen) {
            verschiedene.push(namen);
        }
    }

    // **Reihenfolgen zaehlen ist die leichtere und die schwaechere Frage.**
    // Acht verschiedene Reihenfolgen klingen nach Vielfalt; bestehen sie aus
    // denselben sechs Zaubern in wechselnder Ordnung, ist es EINE Rotation mit
    // Umstellungen. #509 fragt nach dem Zauberkasten, nicht nach der Sortierung.
    let mut mengen: Vec<Vec<&'static str>> = Vec::new();
    for (_, namen, _) in &rotationen {
        let mut sortiert = namen.clone();
        sortiert.sort_unstable();
        if !mengen.contains(&sortiert) {
            mengen.push(sortiert);
        }
    }

    // Welche Zauber stehen in JEDER Rotation? Das ist der unbewegliche Kern.
    let kern: Vec<&'static str> = rotationen[0]
        .1
        .iter()
        .filter(|z| rotationen.iter().all(|(_, n, _)| n.contains(z)))
        .copied()
        .collect();

    println!("\n=== ANTWORT #509 ===");
    println!(
        "  {} Bosse ergeben {} verschiedene REIHENFOLGEN, aber nur {} verschiedene MENGEN",
        rotationen.len(),
        verschiedene.len(),
        mengen.len()
    );
    println!(
        "  In JEDER Rotation stehen {} Zauber: {}",
        kern.len(),
        kern.join(", ")
    );
    println!(
        "  Es variieren damit hoechstens {} von {} Plaetzen",
        MAX_LEN.saturating_sub(kern.len()),
        MAX_LEN
    );

    println!("\n=== ANTWORT #550 ===");
    println!(
        "  {} von {} spielbaren Zaubern kommen in mindestens einer besten Rotation vor",
        gesehen.len(),
        spielbare.len()
    );
    let mut fehlend: Vec<&&str> = spielbare.iter().filter(|s| !gesehen.contains(s)).collect();
    fehlend.sort_unstable();
    println!("  vorhanden: {}", gesehen.join(", "));
    println!("  fehlend:   {}", {
        let f: Vec<&str> = fehlend.iter().map(|s| **s).collect();
        f.join(", ")
    });

    // **Die Zusicherung, die den Lauf von einem leeren Lauf unterscheidet.**
    // Fände die Suche gar nichts, wäre alles oben eine Liste aus Nullen und
    // der Bericht sähe aus wie ein Befund.
    assert!(
        !gesehen.is_empty(),
        "keine einzige Rotation gefunden — der Lauf misst nichts"
    );
}

/// **Der Kontrolllauf, ohne den die anderen Zahlen nichts bedeuten.**
///
/// Beide Messungen unten vergleichen Rotationen als exakte Reihenfolgen. Wenn
/// die Suche schon bei gleicher Lage und nur anderem Startwert eine andere
/// Reihenfolge findet, dann heisst „die Rotation aendert sich mit dem Tempo"
/// gar nichts — es waere der Fehlerbalken, der spricht, und ich haette eine
/// richtige Antwort aus falschem Grund gemeldet.
///
/// 200 Durchgaenge tragen einen Fehlerbalken; zwei Rotationen koennen
/// innerhalb davon gleichwertig sein und trotzdem verschieden aussehen.
#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn kontrolle_streut_die_suche_schon_ohne_aenderung() {
    let pool = all_candidates();

    println!("\n=== KONTROLLE: dieselbe Lage, nur ein anderer Startwert ===");
    println!("  Aendert sich hier die Rotation, sagen die anderen Zahlen nichts.\n");

    let mut gefundene: Vec<Vec<&'static str>> = Vec::new();
    for seed in [0_u32, 1000, 2000] {
        let mut config = config_fuer("first-crucible");
        config.base_seed = seed;
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let namen = zaubernamen(&gefunden.chosen);
        println!(
            "  seed {seed:>5}  {:>9.1}  [{}]",
            gefunden.best.mean,
            namen.join(" > ")
        );
        gefundene.push(namen);
    }

    let stabil = gefundene.windows(2).all(|p| p[0] == p[1]);
    println!("\n=== ANTWORT KONTROLLE ===");
    if stabil {
        println!("  STABIL — drei Startwerte, dieselbe Reihenfolge.");
        println!("  Ein Unterschied in den anderen Laeufen ist damit echt.");
    } else {
        println!("  INSTABIL — schon der blosse Startwert bewegt die Reihenfolge.");
        println!("  Jede Aussage der Form 'die Rotation aendert sich' ist dann wertlos;");
        println!("  vergleichbar bleiben nur die MENGEN der Zauber, nicht ihre Ordnung.");
    }
}

#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn messung_598_aendert_tempo_die_rotation() {
    let pool = all_candidates();

    println!("\n=== #598: ist Tempo eine spielbare Entscheidung? ===");
    println!("  CATALYSIS in Promille, gegen first-crucible\n");

    let mut ergebnisse: Vec<(u32, Vec<&'static str>, f64)> = Vec::new();
    for tempo in [0_u32, 200, 500] {
        let mut config = config_fuer("first-crucible");
        config.worn.catalysis = tempo;
        let gefunden = greedy(&config, &pool, MAX_LEN);
        let namen = zaubernamen(&gefunden.chosen);
        println!(
            "  catalysis {tempo:>4}  {:>9.1}  [{}]",
            gefunden.best.mean,
            namen.join(" > ")
        );
        ergebnisse.push((tempo, namen, gefunden.best.mean));
    }

    let alle_gleich = ergebnisse.windows(2).all(|p| p[0].1 == p[1].1);
    println!("\n=== ANTWORT #598 ===");
    if alle_gleich {
        println!("  Die beste Rotation ist bei 0, 200 und 500 Promille DIESELBE.");
        println!("  Tempo skaliert den Schaden, aendert aber keine Entscheidung.");
    } else {
        println!("  Die beste Rotation AENDERT sich mit dem Tempo — Tempo ist eine Entscheidung.");
    }

    assert!(
        ergebnisse.iter().all(|(_, n, _)| !n.is_empty()),
        "mindestens ein Tempo-Lauf fand keine Rotation"
    );
}

/// **Die Vorpruefung zu #558: existiert der Paar-Fall ueberhaupt?**
///
/// Bevor ich eine Suche umbaue, muss belegt sein, dass es etwas zu finden
/// gibt. Der Verdacht faellt auf `flashpoint`:
///
/// * Er richtet **keinen Schaden** an (`damage: null`) und kostet einen GCD.
///   Allein eingefuegt kann er die Rotation nur verschlechtern.
/// * Er verdoppelt den Ressourcengewinn (`buff.target: resource`, Faktor 2)
///   fuer drei GCDs. Das zahlt sich nur aus, wenn eine Zeile den Vorrat
///   **im Fenster** verbraucht.
/// * Die Gegenzeile `X if buff.flashpoint.active >= 1` steht im Pool, aber
///   ihre Bedingung ist **nie wahr**, solange `flashpoint` nicht geworfen
///   wird. Allein eingefuegt ist sie eine tote Zeile.
///
/// Beide einzeln also wertlos oder schaedlich, zusammen moeglicherweise ein
/// Gewinn — genau die Lage, an der `greedy` mit `Stop::NoImprovement`
/// aussteigt. `flashpoint` steht denn auch in der Liste der 32 Zauber, die in
/// KEINER besten Rotation vorkommen.
///
/// Faellt diese Messung anders aus — traegt `flashpoint` schon allein —, dann
/// ist es kein Paar-Fall, und AC1 aus 5210 braucht einen anderen Traeger.
#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn vorpruefung_558_traegt_flashpoint_nur_im_paar() {
    let pool = all_candidates();
    let config = config_fuer("first-crucible");

    let flashpoint = SPELLS.iter().position(|s| s.id == "flashpoint");
    let Some(flashpoint) = flashpoint else {
        println!("flashpoint fehlt im Datensatz — Vorpruefung nicht anwendbar");
        return;
    };

    // Die Basis: was die heutige Suche findet.
    let basis = greedy(&config, &pool, MAX_LEN);
    let basis_zeilen = basis.chosen.clone();
    println!("\n=== VORPRUEFUNG #558 — traegt flashpoint nur im Paar? ===");
    println!(
        "  Basis (heutige Suche):        {:>9.1}  [{}]",
        basis.best.mean,
        zaubernamen(&basis_zeilen).join(" > ")
    );

    // Die beiden Haelften des verdaechtigen Paares.
    let werfer: Vec<&Candidate> = pool
        .iter()
        .filter(|k| k.spell == flashpoint && k.origin == Origin::Always)
        .collect();
    // Die Nutzer-Zeilen erkennt man an ihrer Bedingung: `buff.flashpoint.active`
    // steht dort als `Value::BuffActive(index)` auf der linken Seite.
    let nutzer: Vec<&Candidate> = pool
        .iter()
        .filter(|k| {
            k.origin == Origin::BuffWindow
                && k.condition
                    .groups
                    .iter()
                    .flatten()
                    .any(|t| t.left == Value::BuffActive(flashpoint))
        })
        .collect();
    println!(
        "  Im Pool: {} Werfer-Zeile(n), {} Nutzer-Zeile(n) mit flashpoint-Fenster",
        werfer.len(),
        nutzer.len()
    );

    // **Jede Haelfte an ihrer BESTEN Stelle**, nicht an einer beliebigen.
    // Sonst misst der Vergleich eine schlechte Einfuegestelle statt der Frage.
    //
    // **Und die Spanne gehoert mitgemessen, nicht nur das Beste.** Die Basis
    // endet mit `kindle` ohne Bedingung; jede Zeile DARUNTER ist tot, und ihre
    // Messung ergibt exakt den Basiswert. Gaebe ich nur das Maximum aus und
    // waere die Messung in Wahrheit kaputt, saehe beides gleich aus: lauter
    // Basiswerte. Bewegt sich zwischen bester und schlechtester Stelle etwas,
    // dann misst der Lauf nachweislich die Einfuegung und nicht sich selbst.
    let spanne = |zeile: &Candidate| -> (f64, f64) {
        (0..=basis_zeilen.len())
            .map(|stelle| {
                let mut versuch = basis_zeilen.clone();
                versuch.insert(stelle, zeile.clone());
                measure(&config, "einzeln", &versuch).mean
            })
            .fold((f64::MAX, f64::MIN), |(lo, hi), w| (lo.min(w), hi.max(w)))
    };
    let bester_einzelzug = |zeile: &Candidate| -> f64 { spanne(zeile).1 };

    let Some(werfer_zeile) = werfer.first() else {
        println!("  keine Werfer-Zeile — Vorpruefung nicht anwendbar");
        return;
    };
    let (werfer_min, nur_werfer) = spanne(werfer_zeile);
    println!("  nur flashpoint: beste Stelle {nur_werfer:>9.1}, schlechteste {werfer_min:>9.1}");

    // **Die Zusicherung, dass ueberhaupt gemessen wird.** Sind beste und
    // schlechteste Stelle identisch, hat das Einfuegen nirgends gewirkt — dann
    // ist jede Aussage unten eine ueber ein totes Werkzeug, nicht ueber
    // flashpoint.
    assert!(
        (nur_werfer - werfer_min).abs() > f64::EPSILON,
        "das Einfuegen von flashpoint aendert an KEINER Stelle etwas \
         ({werfer_min} an jeder Position) — die Messung misst nicht, was sie soll"
    );

    let mut bestes_paar = f64::MIN;
    let mut bester_nutzer_allein = f64::MIN;
    let mut paar_name = String::new();
    for nutzer_zeile in &nutzer {
        let allein = bester_einzelzug(nutzer_zeile);
        bester_nutzer_allein = bester_nutzer_allein.max(allein);

        // Beide zusammen, an allen Stellenkombinationen.
        for a in 0..=basis_zeilen.len() {
            let mut mit_werfer = basis_zeilen.clone();
            mit_werfer.insert(a, (*werfer_zeile).clone());
            for b in 0..=mit_werfer.len() {
                let mut beide = mit_werfer.clone();
                beide.insert(b, (*nutzer_zeile).clone());
                let wert = measure(&config, "paar", &beide).mean;
                if wert > bestes_paar {
                    bestes_paar = wert;
                    paar_name = format!("flashpoint + {}", SPELLS[nutzer_zeile.spell].id);
                }
            }
        }
    }
    println!("  nur Nutzer-Zeile (beste):      {bester_nutzer_allein:>9.1}");
    println!("  BEIDE zusammen (bestes Paar):  {bestes_paar:>9.1}  [{paar_name}]");

    println!("\n=== ANTWORT VORPRUEFUNG ===");
    let einzeln_besser = nur_werfer.max(bester_nutzer_allein);
    if bestes_paar > basis.best.mean && einzeln_besser <= basis.best.mean {
        println!("  PAAR-FALL BELEGT: einzeln bringt keine Haelfte einen Gewinn");
        println!(
            "  ({einzeln_besser:.1} <= Basis {:.1}), zusammen {bestes_paar:.1}.",
            basis.best.mean
        );
        println!("  Genau hier steigt greedy mit Stop::NoImprovement aus.");
    } else if bestes_paar <= basis.best.mean {
        println!("  KEIN GEWINN: auch zusammen schlaegt das Paar die Basis nicht");
        println!("  ({bestes_paar:.1} <= {:.1}).", basis.best.mean);
        println!("  Dann ist flashpoint kein Traeger fuer AC1 und #558 braucht einen anderen.");
    } else {
        println!("  KEIN PAAR-FALL: eine Haelfte traegt schon allein");
        println!(
            "  ({einzeln_besser:.1} > Basis {:.1}) — greedy findet sie ohne Umbau.",
            basis.best.mean
        );
    }
}

/// **Gibt es im Bestand ueberhaupt ein Paar, das nur gemeinsam traegt?**
///
/// `flashpoint` war der naheliegendste Verdacht und hat sich nicht bestaetigt.
/// Bevor die Suche paar-faehig umgebaut wird, gehoert die Frage breiter
/// gestellt: Verstaerker sind die Klasse, in der ein Paar-Effekt strukturell
/// zu erwarten ist — der Werfer richtet keinen Schaden an, der Nutzen faellt
/// bei einem anderen Zauber an.
///
/// Geprueft wird jedes Verstaerker-Paar `(Werfer, Nutzer im Fenster)` gegen
/// die gefundene Rotation:
///
/// * beide Haelften einzeln an ihrer besten Stelle
/// * beide zusammen an allen Stellenkombinationen
///
/// Findet sich kein Paar, bei dem *zusammen* die Basis schlaegt und *einzeln*
/// nicht, dann ist die Paar-Blindheit aus #558 zwar als Mechanismus belegt,
/// hat aber im heutigen Bestand **kein Opfer** — und das ist ein Befund, der
/// vor den Umbau gehoert, nicht danach.
#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn vorpruefung_558_gibt_es_irgendein_paar_das_nur_gemeinsam_traegt() {
    let pool = all_candidates();
    // **Der Boss ist hier nicht beliebig, sondern der Kern der Frage.**
    // Gegen `first-crucible` endet die Suche mit `Stop::LengthLimit` — sie
    // hoert auf, weil sechs Zeilen voll sind, nicht weil ihr nichts mehr
    // einfaellt. Paar-Blindheit tritt aber genau dort auf, wo sie von SELBST
    // aufgibt (`Stop::NoImprovement`). Gegen die volle Liste zu pruefen misst
    // die Laengengrenze und nennt das Ergebnis dann "kein Paar".
    // `boss-2-4` ist der einzige der neun Bosse, dessen Suche von selbst
    // ausläuft — nach fuenf Zeilen.
    let config = config_fuer("boss-2-4");
    let basis = greedy(&config, &pool, MAX_LEN);
    let basis_zeilen = basis.chosen.clone();

    println!("\n=== #558: sucht ALLE Verstaerker-Paare ===");
    println!(
        "  Basis {:.1}  [{}]  Stop {:?}\n",
        basis.best.mean,
        zaubernamen(&basis_zeilen).join(" > "),
        basis.stop
    );

    // **Ohne diese Zeile misst der Lauf die falsche Sache.** Endet die Suche
    // an der Laengengrenze, hat sie nie gesagt „mir faellt nichts mehr ein" —
    // und genau dieser Satz ist die Lage, in der ein Paar verlorengeht. Ein
    // „kein Paar gefunden" waere dann eine Aussage ueber MAX_LEN.
    assert_eq!(
        basis.stop,
        rota_sim::rotation_search::Stop::NoImprovement,
        "die Suche endete mit {:?}, nicht mit NoImprovement — an diesem Boss \
         ist Paar-Blindheit gar nicht die bindende Grenze, der Lauf misst die \
         Laengengrenze",
        basis.stop
    );

    let bestes = |zeile: &Candidate| -> f64 {
        (0..=basis_zeilen.len())
            .map(|stelle| {
                let mut versuch = basis_zeilen.clone();
                versuch.insert(stelle, zeile.clone());
                measure(&config, "einzeln", &versuch).mean
            })
            .fold(f64::MIN, f64::max)
    };

    let mut treffer: Vec<String> = Vec::new();
    let mut geprueft = 0_usize;

    for (verstaerker, spell) in SPELLS.iter().enumerate() {
        if spell.buff.is_none() || spell.unlock_level > MAX_LEVEL as u16 {
            continue;
        }
        let Some(werfer) = pool
            .iter()
            .find(|k| k.spell == verstaerker && k.origin == Origin::Always)
        else {
            continue;
        };
        let nutzer: Vec<&Candidate> = pool
            .iter()
            .filter(|k| {
                k.origin == Origin::BuffWindow
                    && k.condition
                        .groups
                        .iter()
                        .flatten()
                        .any(|t| t.left == Value::BuffActive(verstaerker))
            })
            .collect();
        if nutzer.is_empty() {
            continue;
        }

        let werfer_allein = bestes(werfer);
        let mut paar_bestes = f64::MIN;
        let mut nutzer_bestes = f64::MIN;
        let mut welcher = "";
        for nutzer_zeile in &nutzer {
            nutzer_bestes = nutzer_bestes.max(bestes(nutzer_zeile));
            for a in 0..=basis_zeilen.len() {
                let mut mit = basis_zeilen.clone();
                mit.insert(a, werfer.clone());
                for b in 0..=mit.len() {
                    let mut beide = mit.clone();
                    beide.insert(b, (*nutzer_zeile).clone());
                    let wert = measure(&config, "paar", &beide).mean;
                    if wert > paar_bestes {
                        paar_bestes = wert;
                        welcher = SPELLS[nutzer_zeile.spell].id;
                    }
                }
            }
        }
        geprueft += 1;

        let einzeln = werfer_allein.max(nutzer_bestes);
        let nur_gemeinsam = paar_bestes > basis.best.mean && einzeln <= basis.best.mean;
        println!(
            "  {:<12} einzeln {:>8.1}  Paar {:>8.1} (+{})  {}",
            spell.id,
            einzeln,
            paar_bestes,
            welcher,
            if nur_gemeinsam { "<== NUR IM PAAR" } else { "" }
        );
        if nur_gemeinsam {
            treffer.push(format!("{} + {welcher}", spell.id));
        }
    }

    println!("\n=== ANTWORT ===");
    println!("  {geprueft} Verstaerker mit Nutzer-Zeilen geprueft");
    if treffer.is_empty() {
        println!("  KEIN Paar traegt nur gemeinsam.");
        println!("  Die Paar-Blindheit aus #558 ist als Mechanismus belegt, hat im");
        println!("  heutigen Bestand aber kein Opfer. Ein Umbau der Suche wuerde an");
        println!("  diesen Daten nichts finden, was sie heute verfehlt.");
    } else {
        println!(
            "  {} Paar(e) tragen NUR gemeinsam: {}",
            treffer.len(),
            treffer.join(", ")
        );
        println!("  Das sind die Traeger fuer AC1 aus 5210.");
    }

    assert!(
        geprueft > 0,
        "kein einziger Verstaerker geprueft — der Lauf misst nichts"
    );
}

/// **AC4 aus 5210: was kostet eine paar-faehige Suche wirklich?**
///
/// In 5232 stand dazu eine Hochrechnung — 68.000 Messungen, knapp zwei
/// Stunden — abgeleitet aus der Rate einer kleineren Messung. Eine
/// Hochrechnung ist keine Zahl, und AC4 verlangt Sekunden.
///
/// Zwei Stunden zu verbrennen, nur um eine Zahl zu bestaetigen, waere aber
/// Verschwendung. Deshalb wird hier eine **Stichprobe** gefahren: eine feste
/// Zahl echter Paar-Messungen aus dem echten Pool, gegen die echte Basis,
/// mit der Uhr daneben. Aus der gemessenen Rate folgt die volle Runde.
///
/// Der Rechenweg steht in der Ausgabe, damit die Zahl nachrechenbar ist,
/// ohne den vollen Lauf zu wiederholen.
#[test]
#[ignore = "Messwerkzeug, Laufzeit in Minuten (gemessen, siehe Dateikopf) — sichert nichts zu"]
fn ac4_was_kostet_eine_paar_runde() {
    use std::time::Instant;

    let pool = all_candidates();
    let config = config_fuer("boss-2-4");
    let basis = greedy(&config, &pool, MAX_LEN);
    let basis_zeilen = basis.chosen.clone();

    // Die volle Runde: jedes ungeordnete Paar aus dem Pool, an jeder
    // Stellenkombination der bisherigen Liste.
    let n = pool.len();
    let paare = n * (n - 1) / 2;
    let stellen = (basis_zeilen.len() + 1) * (basis_zeilen.len() + 2);
    let voll = paare * stellen;

    println!("\n=== AC4: Kosten einer vollstaendigen Paar-Runde ===");
    println!("  Pool {n} Kandidaten -> {paare} Paare");
    println!(
        "  Basis {} Zeilen -> {stellen} Stellenkombinationen",
        basis_zeilen.len()
    );
    println!("  Volle Runde: {voll} Messungen");

    // Die Stichprobe: so viele echte Messungen, wie in vertretbarer Zeit gehen.
    const STICHPROBE: usize = 300;
    let mut gemessen = 0_usize;
    let start = Instant::now();
    'aussen: for a in 0..n {
        for b in (a + 1)..n {
            let mut versuch = basis_zeilen.clone();
            versuch.insert(0, pool[a].clone());
            versuch.insert(1, pool[b].clone());
            let _ = measure(&config, "kosten", &versuch);
            gemessen += 1;
            if gemessen >= STICHPROBE {
                break 'aussen;
            }
        }
    }
    let dauer = start.elapsed();
    let je_messung = dauer.as_secs_f64() / gemessen as f64;
    let hochgerechnet = je_messung * voll as f64;

    println!(
        "\n  Stichprobe: {gemessen} echte Messungen in {:.1} s",
        dauer.as_secs_f64()
    );
    // **Die Rate haengt an der Last der Maschine, nicht nur am Verfahren.**
    // Am 2026-08-09 zweimal gemessen, mit demselben Code: 0,0425 s je Messung,
    // als dieser Fall neben anderen lief, und 0,0206 s allein — Faktor zwei,
    // also 68 gegen 33 Minuten fuer dieselbe Runde. Wer die Hochrechnung
    // zitiert, muss die Last dazusagen; sonst vergleicht der Naechste zwei
    // Zahlen, die nie dieselbe Frage beantwortet haben.
    println!("  (Rate abhaengig von der Parallelitaet: allein rund doppelt so");
    println!("   schnell wie neben anderen Faellen — beide Zahlen im Code)");
    println!("  -> {:.4} s je Messung", je_messung);
    println!(
        "  -> volle Runde {voll} x {:.4} s = {:.0} s ({:.1} Minuten)",
        je_messung,
        hochgerechnet,
        hochgerechnet / 60.0
    );
    println!("\n  Zum Vergleich die heutige Suche:");
    println!(
        "  {} Messungen fuer die ganze Suche, Stop {:?}",
        basis.measured, basis.stop
    );
    println!(
        "  -> eine EINZIGE Paar-Runde kostet das {:.0}-fache der gesamten heutigen Suche",
        voll as f64 / basis.measured as f64
    );

    assert!(
        gemessen > 0,
        "keine Messung gefahren — die Zahl waere erfunden"
    );
}
