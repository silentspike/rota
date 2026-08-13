//! Der Bericht der Rotations-Suche (Issue #237, AC4 und AC1b).
//!
//! ```text
//! cargo remote -c -- run --release --example rotation-search
//! ```
//!
//! # Warum dieser Bericht seine eigenen Parameter druckt
//!
//! Regel aus Issue #242: Jede gemeldete Balance-Zahl braucht einen
//! eingecheckten Läufer, der seine Einstellungen mitdruckt. Eine Zahl ohne
//! ihren Aufruf ist nicht nachrechenbar, und „überliefert" ist kein Zustand,
//! den ein Bericht haben sollte.
//!
//! # Was hier ausdrücklich NICHT durchsucht wurde
//!
//! Das ist die Ehrlichkeitsauflage aus AC4 — und sie ist kein Beiwerk: Ohne
//! sie liest sich „gefunden" wie „bewiesen".

use rota_sim::apl::AplName;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy, greedy_from, Candidate, Origin, Stop};

/// Durchgänge je gemessener Rotation.
const RUNS: u32 = 50;
/// Startwert des ersten Durchgangs.
const SEED: u32 = 0;
/// Höchstlänge der gesuchten Listen — Voreinstellung, oder `--max-len N`.
const MAX_LEN: usize = 6;

/// Die Höchstlänge dieses Laufs.
///
/// **Wählbar, weil die AC4-Zahl sonst zwei Dinge misst.** „48 von 57 Zaubern
/// kommen in keiner Rotation vor" klingt nach einer Aussage über die Zauber —
/// bei sechs Zeilen je Liste können aber über fünf Bosse höchstens dreißig
/// überhaupt vorkommen. Ohne eine Gegenprobe mit längerer Liste wäre nicht zu
/// trennen, was am Kit liegt und was an der Suchlänge.
fn max_len_dieses_laufs() -> usize {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--max-len" {
            let wert = args.next().expect("Nach --max-len fehlt die Zahl");
            return wert.parse().expect("--max-len braucht eine Zahl");
        }
    }
    MAX_LEN
}

/// Ob der Nachlauf aus #558 laufen soll — abschaltbar mit `--kein-nachlauf`.
///
/// # Warum ein Schalter und nicht einfach „es laeuft immer"
///
/// Der Nachlauf bringt die ACHTUNG-Zeile des Kreuzvergleichs zum Schweigen —
/// das ist sein Zweck. Genau deshalb braucht es einen Weg, sie wieder zum
/// Anschlagen zu bringen: **Ein Waechter, der nie mehr ausloesen kann, ist
/// von einem toten nicht zu unterscheiden.** Ohne diesen Schalter waere die
/// einzige Gegenprobe, den Nachlauf im Quelltext auszubauen — also mit einem
/// Werkzeug zu messen, das es so nicht gibt.
///
/// Gemessen ueber dieselben fuenf Bosse bei `--max-len 12`:
///
/// ```text
///   mit Nachlauf     keine ACHTUNG-Zeile, mittlerer Verlust 0,0 %
///   --kein-nachlauf  ACHTUNG in 4 Paarungen, mittlerer Verlust 19,2 %
/// ```
fn nachlauf_gewuenscht() -> bool {
    !std::env::args().any(|a| a == "--kein-nachlauf")
}

/// Die Bosse, gegen die gesucht wird (Schnitt 3 und AC1b).
///
/// **Nicht irgendein zweiter Boss — einer mit anderer DAUER.** Mein erster
/// Versuch nahm `test-warden`, weil er Schaden und einen unterbrechbaren
/// Zauber hat. Beide Bosse lieferten exakt dieselbe Zahl, und der Grund stand
/// in meiner eigenen Einstellung: Mit `stop_on_kill = false` läuft jeder
/// Durchgang die volle Überdruckzeit, und `test-warden` hat dieselben 60 s wie
/// `first-crucible`. HP und Zeitleiste formen die Rotation nicht, solange
/// nicht gestoppt wird und der Spieler überlebt.
///
/// `test-metronome` hat 300 s. Dort kommt REFLUX (Abklingzeit 45 s) sechsmal
/// zum Zug statt einmal — die Werkzeug-Zauber haben dort tatsächlich
/// Gelegenheit.
///
/// **Mehr als einer, und das ist keine Gründlichkeit, sondern Notwendigkeit.**
/// Gegen einen Boss ohne Schaden und ohne eigene Zauber können FIXATION,
/// COAGULATION und QUENCH nichts beitragen; eine Suche dort würde sie zu Recht
/// nie wählen und danach „tote Zauber" melden, wo in Wahrheit steht: ein Boss,
/// an dem sie nichts zu tun haben.
const BOSSES: &[&str] = &["first-crucible", "test-metronome"];

/// Die Bosse dieses Laufs — Voreinstellung [`BOSSES`], oder `--bosses a,b,c`.
///
/// **Waehlbar seit #509**, und das ist keine Bequemlichkeit: Der Pruefstein
/// fragt, ob ueber VERSCHIEDENE Bosse dieselbe Rotation gefunden wird. Wer
/// dafuer jedes Mal eine Zeile im Quelltext aendert, misst mit einem Werkzeug,
/// das nach der Messung anders aussieht als davor — und der Bericht nennt
/// einen Befehl, den niemand nachfahren kann.
fn bosse_dieses_laufs() -> Vec<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--bosses" {
            let liste = args.next().unwrap_or_else(|| {
                panic!(
                    "Nach --bosses fehlt die Liste, etwa: --bosses first-crucible,test-onslaught"
                )
            });
            let gewaehlt: Vec<String> = liste.split(',').map(|s| s.trim().to_owned()).collect();
            // **Jeder Name wird geprueft, bevor gerechnet wird.** Ein Tippfehler
            // darf nicht stillschweigend einen Boss weglassen: Der Bericht
            // saehe dann vollstaendig aus und waere es nicht.
            for id in &gewaehlt {
                assert!(
                    boss_by_id(id).is_some(),
                    "Boss {id} kennt der Simulator nicht"
                );
            }
            return gewaehlt;
        }
    }
    BOSSES.iter().map(|s| (*s).to_owned()).collect()
}

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        // Beide Rotationen rechnen gleich lang — sonst gewinnt die, die am
        // frühesten aufhört, und die Zahlen wären nicht vergleichbar.
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

/// Die handgeschriebene Liste, gegen die gemessen wird (AC1).
const STEP1_GOOD: &str = include_str!("../apl/step1-good.apl");

fn parameter_zeile_mit(bosse: &[String], max_len: usize) {
    println!("Parameter: RUNS={RUNS}, SEED={SEED}, MAX_LEN={max_len}, Bosse={bosse:?}");
    println!("           stop_on_kill=false, Faktoren=NEUTRAL, threads=1");
}

fn zeilen_nach_herkunft(pool: &[Candidate]) -> (usize, usize, usize, usize, usize) {
    let mut a = 0;
    let mut d = 0;
    let mut s = 0;
    let mut b = 0;
    let mut c = 0;
    for k in pool {
        match k.origin {
            Origin::Always => a += 1,
            Origin::DotRefresh => d += 1,
            Origin::SpenderThreshold => s += 1,
            // Neu mit #553 — vorher konnte die Suche diese Zeile gar nicht
            // bilden, weil der Parser keine Buff-Fenster kannte.
            Origin::BuffWindow => b += 1,
            // Neu mit #797 — vorher kannte die Sprache keinen laufenden
            // Boss-Zauber, also konnte die Suche keine Zeile bilden, die auf
            // einen antwortet.
            Origin::BossCastWindow => c += 1,
        }
    }
    (a, d, s, b, c)
}

/// Was an einem Boss gefunden wurde.
///
/// **Die gewaehlten Zeilen stehen daneben, nicht nur die fertige Liste.** Der
/// Nachlauf aus #558 braucht sie als Startpunkt, und aus einer `Apl` waeren
/// sie nur durch erneutes Zerlegen zurueckzugewinnen — eine zweite Quelle fuer
/// etwas, das die Suche schon weiss.
struct Gefunden {
    boss: String,
    apl: rota_sim::apl::Apl,
    wert: f64,
    zeilen: Vec<Candidate>,
}

/// Der Nachlauf: jede fremde Liste als Startpunkt (Issue #558).
///
/// # Was gemessen wurde und warum es das braucht
///
/// Bei `--max-len 12` schlaegt die Liste von `test-metronome` an vier von
/// fuenf Bossen deren EIGENE gefundene Liste — um bis zu 37,3 %. Die Referenz
/// des Kreuzvergleichs ist damit nicht das Optimum, und jede Verlust-Zahl
/// misst zum Teil die Schwaeche der Suche statt der Rotations-Tiefe.
///
/// # Das Verfahren
///
/// Fuer jeden Boss wird jede an einem ANDEREN Boss gefundene Liste als
/// Startpunkt genommen und von dort weitergesucht. Das Beste aus allen
/// Anlaeufen — einschliesslich des urspruenglichen — gewinnt.
///
/// **Uebernehmen allein reicht nicht.** Eine fremde Liste kann besser sein und
/// trotzdem nicht das Beste, was von ihr aus erreichbar ist: `greedy` bricht
/// bei Laenge 12 schon nach acht Zeilen ab, es ist also Platz. Deshalb
/// `greedy_from` und nicht ein blosser Vergleich.
///
/// # Und EINE Runde reicht auch nicht — gemessen
///
/// Der erste Entwurf lief genau einmal durch. Er hob vier von fuenf Bossen um
/// 21 bis 40 %, und die ACHTUNG-Zeile meldete danach trotzdem wieder vier
/// Paarungen:
///
/// ```text
///   test-resistant  38337 -> 53817  (+40,4 %)
///   ...
///   ACHTUNG: In 4 Paarungen ist die FREMDE Liste besser als die eigene.
///     test-resistant schlaegt an test-onslaught dessen eigene um 10,8 %
/// ```
///
/// Der Grund ist die Reihenfolge, nicht das Verfahren: `test-resistant` wurde
/// IN DIESER RUNDE auf 53817 gehoben, und diese neue Liste stand keinem
/// anderen Boss mehr als Startpunkt zur Verfuegung. **Eine Verbesserung ist
/// selbst ein neuer Startpunkt** — deshalb laeuft es bis zum Fixpunkt.
fn nachlauf(bosse: &[String], pool: &[Candidate], max_len: usize, gefundene: &mut [Gefunden]) {
    println!("== Nachlauf mit fremden Startpunkten (Issue #558) ==");
    println!();

    // **Nach oben beschraenkt, aber nicht still.** Jede Runde hebt mindestens
    // einen Wert, und der Suchraum ist endlich — das Verfahren terminiert von
    // selbst. Die Grenze faengt nur den Fall ab, dass zwei Listen sich in
    // winzigen Schritten gegenseitig hochschaukeln; greift sie, steht das als
    // Zeile im Bericht statt als stille Kuerzung.
    const MAX_RUNDEN: usize = 10;
    let mut runde = 0;
    loop {
        runde += 1;
        let veraendert = nachlauf_runde(bosse, pool, max_len, gefundene, runde);
        if !veraendert {
            println!("  Runde {runde}: keine Verbesserung mehr — Fixpunkt erreicht.");
            break;
        }
        if runde >= MAX_RUNDEN {
            println!("  ABBRUCH nach {MAX_RUNDEN} Runden, OHNE Fixpunkt. Die Zahlen unten sind");
            println!("  damit nicht ausgereizt — die letzte Runde hat noch etwas gehoben.");
            break;
        }
    }
    println!();
}

/// Eine Runde des Nachlaufs. Gibt zurueck, ob sich etwas verbessert hat.
fn nachlauf_runde(
    bosse: &[String],
    pool: &[Candidate],
    max_len: usize,
    gefundene: &mut [Gefunden],
    runde: usize,
) -> bool {
    // Erst alle Startpunkte sammeln, dann anwenden: Sonst nimmt Boss 2 bereits
    // die im selben Durchgang verbesserte Liste von Boss 1 als Startpunkt, und
    // das Ergebnis haengt an der Reihenfolge der Bosse statt an der Sache.
    let ausgangslisten: Vec<Vec<Candidate>> = gefundene.iter().map(|g| g.zeilen.clone()).collect();
    let mut veraendert = false;

    for (i, boss_id) in bosse.iter().enumerate() {
        let config = config(boss_id);
        let mut bestes: Option<(usize, rota_sim::rotation_search::SearchResult)> = None;
        for (j, start) in ausgangslisten.iter().enumerate() {
            if i == j {
                continue;
            }
            let versuch = greedy_from(&config, pool, max_len, start);
            let besser = match &bestes {
                None => versuch.best.mean > gefundene[i].wert,
                Some((_, bisher)) => versuch.best.mean > bisher.best.mean,
            };
            if besser {
                bestes = Some((j, versuch));
            }
        }
        if let Some((j, ergebnis)) = bestes {
            let zuwachs = (ergebnis.best.mean - gefundene[i].wert) / gefundene[i].wert * 100.0;
            println!(
                "  Runde {runde}  {boss_id:<20} {:.0} -> {:.0} ({zuwachs:+.1} %) ueber {}",
                gefundene[i].wert, ergebnis.best.mean, bosse[j]
            );
            gefundene[i].wert = ergebnis.best.mean;
            gefundene[i].apl = ergebnis.best.apl.clone();
            gefundene[i].zeilen = ergebnis.chosen;
            veraendert = true;
        }
    }
    veraendert
}

/// Der Kreuzvergleich — der Kern von Issue #509.
///
/// # Die Frage
///
/// „Findet der Balance-Laeufer ueber verschiedene Bosse dieselbe Rotation?"
/// Wenn ja, ist die Tiefe nicht da: Dann gibt es EINE beste Reihenfolge, und
/// der Rest des Spiels ist Ausfuehrung (GAME-DESIGN 20.3).
///
/// # Zwei Zahlen, und die zweite ist die belastbare
///
/// **Gemeinsame Zeilen** sagen, wie aehnlich die gefundenen Listen aussehen.
/// Das ist die schwaechere Zahl: Zwei Listen koennen sich in einer Zeile
/// unterscheiden und trotzdem gleich viel wert sein.
///
/// **Was es KOSTET, die Rotation von Boss A gegen Boss B zu spielen** — das
/// ist die Zahl, auf die es ankommt. Kostet es nichts, ist die Tiefe nicht da,
/// und zwar unabhaengig davon, wie verschieden die Listen aussehen.
///
/// # Warum das Urteil nicht am groessten Verlust haengt
///
/// **Der erste Entwurf tat genau das — und log.** Gemessen ueber fuenf Bosse:
///
/// ```text
///   Groesster Verlust   61,6 %   -> „Tiefe ist vorhanden"
///   Mittlerer Verlust    1,7 %
///   Paare ueber 10 %     8 von 20
/// ```
///
/// Alle acht teuren Paarungen haben `test-onslaught` auf einer der beiden
/// Seiten — den einzigen Boss der Auswahl, der den Spieler toeten kann. Die
/// uebrigen vier sind untereinander praktisch austauschbar (0,0 bis 1,7 %). Ein
/// Maximum sagt, dass es EINEN Fall gibt; es sagt nichts darueber, ob die Sache
/// traegt.
///
/// Deshalb steht jetzt die Verteilung da, und der Urteilssatz nennt, wie viele
/// Paare ueberhaupt etwas kosten. Ein Befund aus einem von fuenf Bossen ist ein
/// anderer Befund als einer aus fuenf von fuenf — und die frohere Lesart war
/// genau die, die dieses Issue verhindern soll.
fn kreuzvergleich(gefundene: &[Gefunden]) {
    if gefundene.len() < 2 {
        println!("Kreuzvergleich uebersprungen — dafuer braucht es mindestens zwei Bosse.");
        println!("  Beispiel: --bosses first-crucible,test-onslaught,test-metronome");
        println!();
        return;
    }

    println!("== Kreuzvergleich (Issue #509 AC2 und AC3) ==");
    println!();
    println!("Wie aehnlich sehen die gefundenen Listen aus?");
    for i in 0..gefundene.len() {
        for j in (i + 1)..gefundene.len() {
            let a: Vec<String> = gefundene[i]
                .apl
                .to_string()
                .lines()
                .map(str::to_owned)
                .collect();
            let b: Vec<String> = gefundene[j]
                .apl
                .to_string()
                .lines()
                .map(str::to_owned)
                .collect();
            let gemeinsam = a.iter().filter(|z| b.contains(z)).count();
            let groesser = a.len().max(b.len()).max(1);
            println!(
                "  {:<20} gegen {:<20} {gemeinsam} von {groesser} Zeilen gleich ({:.0} %)",
                gefundene[i].boss,
                gefundene[j].boss,
                gemeinsam as f64 / groesser as f64 * 100.0
            );
        }
    }
    println!();

    println!("Was kostet die FREMDE Rotation? (Zeile = gespielte Liste, Spalte = Boss)");
    print!("  {:<22}", "");
    for g in gefundene {
        print!("{:>22}", g.boss);
    }
    println!();

    // **Einmal rechnen, zweimal lesen.** Der erste Entwurf fuhr `run` fuer die
    // Tabelle und danach ein zweites Mal fuers Urteil — zwei Quellen fuer
    // dieselbe Zahl, die auseinanderlaufen koennen, ohne dass es auffaellt.
    // Jetzt trifft die Tabelle und das Urteil dieselbe Sammlung.
    let mut fremde_verluste: Vec<(String, String, f64)> = Vec::new();

    for quelle in gefundene {
        let herkunft = &quelle.boss;
        print!("  {:<22}", format!("von {herkunft}"));
        for ziel in gefundene {
            let (boss_id, eigen) = (&ziel.boss, ziel.wert);
            let ergebnis = rota_sim::balance::run(&config(boss_id), &quelle.apl);
            // **Der Verlust gegen die EIGENE beste Rotation dieses Bosses.**
            // Nicht gegen die eigene Herkunft: Gefragt ist, was der Spieler an
            // DIESEM Boss verliert, wenn er die fremde Liste mitbringt.
            let verlust = (eigen - ergebnis.damage.mean) / eigen * 100.0;
            print!(
                "{:>22}",
                format!("{:.0} ({verlust:+.1} %)", ergebnis.damage.mean)
            );
            if herkunft != boss_id {
                fremde_verluste.push(((*herkunft).clone(), (*boss_id).clone(), verlust));
            }
        }
        println!();
    }
    println!();

    urteil_ueber_die_tiefe(&fremde_verluste);
}

/// Welche Zauber sind ueber ALLE Bosse hinweg nie drangekommen (#509 AC4).
///
/// # Warum das nicht die Summe der Einzelzahlen ist
///
/// Je Boss steht oben „nie gewirkt 52 von 57". Diese Zahlen zu addieren oder
/// die kleinste zu nehmen waere beides falsch: Gefragt ist die SCHNITTMENGE —
/// wer bei keinem einzigen Boss die beste Wahl war. Ein Zauber, der nur gegen
/// `test-onslaught` gewirkt wird, taucht in vier von fuenf Einzellisten auf und
/// gehoert hier trotzdem nicht hin.
///
/// Das ist Leitplanke 2 aus GAME-DESIGN: „jeder Zauber muss irgendwo die beste
/// Wahl sein." Die Zahl hier ist das direkte Mass dafuer.
fn nirgends_die_beste_wahl(irgendwo_gewirkt: &[bool], anzahl_bosse: usize) {
    let spielbar: Vec<usize> = (0..rota_sim::SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
        .collect();
    let tot: Vec<&str> = spielbar
        .iter()
        .filter(|&&i| !irgendwo_gewirkt[i])
        .map(|&i| SPELLS[i].id)
        .collect();
    let lebendig = spielbar.len() - tot.len();

    println!("Welche Zauber sind NIRGENDS die beste Wahl? (Issue #509 AC4)");
    println!(
        "  ueber alle {anzahl_bosse} Bosse gewirkt   {lebendig} von {}",
        spielbar.len()
    );
    println!(
        "  in KEINER Rotation      {} von {}",
        tot.len(),
        spielbar.len()
    );
    if tot.is_empty() {
        println!("  -> Leitplanke 2 ist erfuellt: jeder spielbare Zauber kommt irgendwo vor.");
    } else {
        println!("    {}", tot.join(", "));
        println!("  -> Leitplanke 2 ist NICHT erfuellt. Die Zahl faellt erst, wenn es einen");
        println!("     Boss gibt, an dem diese Zauber gebraucht werden — nicht durch mehr");
        println!("     Zauber, sondern durch mehr Anlaesse.");
    }
    println!();
}

/// Das Urteil ueber die Rotations-Tiefe — aus der Verteilung, nicht aus dem
/// groessten Wert.
///
/// Die Diagonale steht nicht in `verluste`: Eine Liste gegen ihren eigenen Boss
/// verliert per Definition nichts und wuerde den Mittelwert nach unten ziehen.
fn urteil_ueber_die_tiefe(verluste: &[(String, String, f64)]) {
    if verluste.is_empty() {
        return;
    }
    let mut sortiert: Vec<f64> = verluste.iter().map(|(_, _, v)| *v).collect();
    sortiert.sort_by(f64::total_cmp);
    let groesster = sortiert.last().copied().unwrap_or(0.0);
    let mittlerer = sortiert[sortiert.len() / 2];
    let ueber_zehn = sortiert.iter().filter(|v| **v >= 10.0).count();

    // **Die Zusicherung VOR dem Urteil: taugt die Referenz ueberhaupt?**
    //
    // Der ganze Kreuzvergleich rechnet den Verlust gegen die je Boss GEFUNDENE
    // Liste. Das setzt voraus, dass sie an ihrem Boss die beste ist. Findet
    // eine FREMDE Liste dort mehr Schaden, ist genau das widerlegt — dann misst
    // die Matrix nicht mehr die Rotations-Tiefe, sondern wie weit die Suche am
    // Optimum vorbeigegriffen hat.
    //
    // Gemessen bei `--max-len 12`: Die Liste von `test-metronome` ist an vier
    // von fuenf Bossen besser als deren eigene, um bis zu 37 %. Das Urteil
    // lautete trotzdem „Tiefe ist vorhanden" — aus einem mittleren Verlust von
    // 19,2 %, der zum grossen Teil aus schwachen Referenzen stammte. Bei Laenge
    // 6 tritt der Fall nicht auf; greedy legt sich im groesseren Raum frueher
    // falsch fest.
    let besser_als_referenz: Vec<&(String, String, f64)> =
        verluste.iter().filter(|(_, _, v)| *v < -0.5).collect();

    println!("Was der Kreuzvergleich ergibt (ohne die Diagonale)");
    println!("  Fremde Paarungen        {}", sortiert.len());
    println!("  Groesster Verlust       {groesster:.1} %");
    println!("  Mittlerer Verlust       {mittlerer:.1} %");
    println!(
        "  Paare ueber 10 %        {ueber_zehn} von {}",
        sortiert.len()
    );

    if !besser_als_referenz.is_empty() {
        println!();
        println!(
            "  ACHTUNG: In {} Paarungen ist die FREMDE Liste besser als die eigene.",
            besser_als_referenz.len()
        );
        for (von, gegen, v) in &besser_als_referenz {
            println!(
                "    {von} schlaegt an {gegen} dessen eigene Liste um {:.1} %",
                -v
            );
        }
        println!("  Damit ist die Referenz je Boss NICHT das Optimum, und die Zahlen oben");
        println!("  messen zum Teil die Schwaeche der Suche statt der Rotations-Tiefe.");
        println!("  -> KEIN Urteil. Erst die Suche verbessern, dann erneut vergleichen.");
        println!();
        return;
    }

    // **Der Mittelwert entscheidet, nicht das Maximum.** Ein einzelner Boss mit
    // Todesgefahr hebt das Maximum auf 61 %, waehrend alle anderen Paare unter
    // 2 % liegen — „Tiefe ist vorhanden" waere daraus die falsche Lesart.
    if mittlerer < 1.0 {
        println!("  -> Es kostet in der MEHRZAHL der Paarungen praktisch nichts, ueberall");
        println!("     dieselbe Liste zu spielen. Das ist der Befund aus GAME-DESIGN 20.3:");
        println!("     Rotations-Tiefe entsteht nicht von selbst.");
    } else if mittlerer < 10.0 {
        println!("  -> Es kostet in der Regel etwas, aber wenig. Ob das reicht, ist eine");
        println!("     Design-Frage.");
    } else {
        println!("  -> Die Rotation lohnt sich je Boss zu ueberdenken. Tiefe ist vorhanden.");
    }

    if ueber_zehn > 0 {
        // **Woher die Ausreisser kommen, gehoert daneben** — sonst liest man
        // die Spanne als Eigenschaft aller Bosse.
        //
        // Und zwar von BEIDEN Seiten: Mein erster Entwurf sah nur die Spalte
        // an und haette vier der acht teuren Paare uebersehen. Eine fremde
        // Liste gegen `test-onslaught` zu spielen kostet 61 %, und SEINE Liste
        // gegen einen anderen Boss kostet 19 % — beide Male ist derselbe Boss
        // der Grund, einmal als Ziel und einmal als Herkunft.
        let teuer: Vec<&(String, String, f64)> =
            verluste.iter().filter(|(_, _, v)| *v >= 10.0).collect();
        let mut beteiligt: Vec<&str> = teuer
            .iter()
            .flat_map(|(von, gegen, _)| [von.as_str(), gegen.as_str()])
            .collect();
        beteiligt.sort_unstable();
        beteiligt.dedup();
        let gemeinsam: Vec<&str> = beteiligt
            .iter()
            .copied()
            .filter(|b| teuer.iter().all(|(von, gegen, _)| von == b || gegen == b))
            .collect();
        if gemeinsam.is_empty() {
            println!("  Kein Boss kommt in allen teuren Paarungen vor — die Spanne verteilt sich.");
        } else {
            println!("  In JEDER teuren Paarung steckt: {}", gemeinsam.join(", "));
            println!("     Die Spanne ist damit die Eigenschaft dieses Bosses, nicht des Spiels.");
        }
    }
    println!();
}

fn main() {
    println!("== Rotations-Suche (Issue #237, Kreuzvergleich #509) ==");
    let bosse = bosse_dieses_laufs();
    let max_len = max_len_dieses_laufs();
    parameter_zeile_mit(&bosse, max_len);
    println!();

    let pool = all_candidates();
    let (a, d, s, b, c) = zeilen_nach_herkunft(&pool);
    println!("Suchraum");
    println!("  Kandidatenzeilen  {}", pool.len());
    // **Die Bezeichnungen kommen aus `Origin::label`, nicht aus diesen
    // Zeilen** (Issue #815). Bis hierher standen sie doppelt: einmal in
    // `rotation_search.rs` als Methode, einmal hier woertlich. Die Methode
    // hatte deshalb keinen Aufrufer und galt als toter Code — sie war es
    // nicht, sie war nur umgangen.
    //
    // Zwei Listen derselben vier Namen laufen beim naechsten Herkunfts-Typ
    // auseinander: Wer ihn in `Origin` ergaenzt, bekommt hier keine Zeile und
    // merkt es nicht, weil die Zahl einfach fehlt.
    for (origin, zahl) in [
        (Origin::Always, a),
        (Origin::DotRefresh, d),
        (Origin::SpenderThreshold, s),
        (Origin::BuffWindow, b),
        (Origin::BossCastWindow, c),
    ] {
        println!("    {:22} {zahl}", origin.label());
    }
    println!();

    // Je Boss die gefundene Rotation — fuer den Kreuzvergleich unten (#509 AC3).
    let mut gefundene: Vec<Gefunden> = Vec::new();
    // Wer ueber ALLE Bosse mindestens einmal drankam (#509 AC4). Die Zahl je
    // Boss steht schon unten je Abschnitt; gefragt ist aber die andere: Welcher
    // Zauber ist NIRGENDS die beste Wahl. Von Hand aus fuenf Listen gebildet
    // waere das eine zweite Rechnung neben der ersten.
    let mut irgendwo_gewirkt: Vec<bool> = vec![false; rota_sim::SPELL_COUNT];

    for boss_id in &bosse {
        let config = config(boss_id);
        let gefunden = greedy(&config, &pool, max_len);
        let hand = rota_sim::apl::parse(AplName("step1-good"), STEP1_GOOD)
            .expect("step1-good ist gueltig");
        let hand_result = rota_sim::balance::run(&config, &hand);

        println!("Boss {boss_id}");
        println!(
            "  gefunden     {:9.1} +/- {:.1}   ({} Listen gemessen)",
            gefunden.best.mean, gefunden.best.stderr, gefunden.measured
        );
        println!(
            "  step1-good   {:9.1} +/- {:.1}",
            hand_result.damage.mean, hand_result.damage.stderr
        );

        // AC1: erreicht oder uebertrifft — und der Bericht sagt, welches von
        // beiden. „Erreicht" ist ein Befund ueber die Handschrift, kein Mangel
        // der Suche.
        let unschaerfe = gefunden.best.stderr + hand_result.damage.stderr;
        let urteil = if gefunden.best.mean - hand_result.damage.mean > unschaerfe {
            "UEBERTRIFFT die Handschrift messbar"
        } else if hand_result.damage.mean - gefunden.best.mean > unschaerfe {
            "BLEIBT unter der Handschrift — die Suche findet sie nicht"
        } else {
            "ERREICHT die Handschrift (Unterschied im Rauschen)"
        };
        println!("  Urteil       {urteil}");

        println!("  Abbruch      {:?}", gefunden.stop);
        println!("  Rotation:");
        for zeile in gefunden.best.apl.to_string().lines() {
            println!("    {zeile}");
        }

        // Welche Zauber kamen im Lauf vor — nicht: welche stehen in der Liste.
        let boss = boss_by_id(boss_id).expect("Boss");
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            seed: SEED,
            boss,
            ..rota_sim::FightConfig::default()
        });
        let lauf = fight.run_apl(&gefunden.best.apl);
        // **Nur spielbare Zauber.** Ein Pruef-Zauber, der gar nicht zur Wahl
        // stand, als „nie gewirkt" zu melden, waere eine Zahl, die etwas
        // anderes zaehlt als sie behauptet — sie wuerde die Zahl der toten
        // Zauber um einen zu hoch ausweisen.
        let spielbar: Vec<usize> = (0..rota_sim::SPELL_COUNT)
            .filter(|&i| u32::from(SPELLS[i].unlock_level) <= rota_sim::data::MAX_LEVEL)
            .collect();
        for &i in &spielbar {
            if lauf.casts[i] > 0 {
                irgendwo_gewirkt[i] = true;
            }
        }
        let nie: Vec<&str> = spielbar
            .iter()
            .filter(|&&i| lauf.casts[i] == 0)
            .map(|&i| SPELLS[i].id)
            .collect();
        println!("  nie gewirkt  {} von {}", nie.len(), spielbar.len());
        println!("    {}", nie.join(", "));
        println!(
            "  (ausgenommen  {} Pruef-Zauber ueber Stufe {})",
            rota_sim::SPELL_COUNT - spielbar.len(),
            rota_sim::data::MAX_LEVEL
        );
        println!();
        gefundene.push(Gefunden {
            boss: (*boss_id).clone(),
            apl: gefunden.best.apl.clone(),
            wert: gefunden.best.mean,
            zeilen: gefunden.chosen,
        });
    }

    if nachlauf_gewuenscht() {
        nachlauf(&bosse, &pool, max_len, &mut gefundene);
    } else {
        println!("== Nachlauf UEBERSPRUNGEN (--kein-nachlauf) ==");
        println!("  Die Zahlen unten sind der Stand VOR #558 — zur Gegenprobe, dass die");
        println!("  ACHTUNG-Zeile des Kreuzvergleichs noch anschlagen kann.");
        println!();
    }
    kreuzvergleich(&gefundene);
    nirgends_die_beste_wahl(&irgendwo_gewirkt, bosse.len());

    println!("Was NICHT durchsucht wurde");
    println!("  1  Freie Bedingungssuche. Nicht der Aufwand ist der Grund, sondern die");
    println!("     Ueberanpassung: Eine Suche ueber alle Werte, Vergleiche und Zahlen faende");
    println!("     Zeilen wie \"if time > 17340\", die zufaellig zur Messung passen und sonst");
    println!("     nichts. Sie liefert NIE ein leeres Ergebnis — und genau das macht sie");
    println!("     wertlos: Man kann sie nicht widerlegen.");
    println!("  2  Erschoepfende Reihenfolgen-Suche. 15! = 1.307.674.368.000.");
    println!("     Erschoepfend gesucht wird nur in der Gegenprobe bei n=5, 7 und 8 — dort");
    println!("     ist es kein Verfahren, sondern ein Massstab.");
    println!("  3  Zufall im Verfahren. Simulated Annealing oder genetische Verfahren");
    println!("     lieferten zwischen zwei Laeufen verschiedene Ergebnisse; eine gemeldete");
    println!("     Zahl waere nicht reproduzierbar (GAME-DESIGN 18.4, wie Issue #242).");
    println!();
    println!("  4  GIERIGES ANHAENGEN — geprueft und VERWORFEN, mit Zahlen:");
    println!("       anhaengen    4000,0");
    println!("       erschoepfend 5960,0     (der step1-good-Wert)");
    println!("     Es verfehlt das Optimum um ein Drittel. Der Grund ist derselbe Satz, mit");
    println!("     dem es begruendet war: Weil das Obere alles Darunterliegende verdeckt,");
    println!("     ist die erste Wahl die folgenreichste — und sie wird mit der");
    println!("     schlechtesten Information getroffen, naemlich einem Zauber ohne Liste.");
    println!("     Steht hier, weil es die naheliegendste Bauart ist und von aussen");
    println!("     vernuenftig aussieht.");
    println!();
    // **Die Auflage aus #558.** Solange die Suche ihre eigene Blindheit nicht
    // ausweist, liest jeder ihr Ergebnis als „diese Zauber taugen nichts" —
    // und genau das ist im Projekt schon einmal passiert (#550 stand deshalb
    // unter Vorbehalt).
    println!("  5  PAARE, deren Haelften einzeln verlieren. Die Suche bewertet jede Zeile");
    println!("     EINZELN. Eine Verstaerker-Zeile allein bringt nichts — erst das Paar aus");
    println!("     Verstaerker und dem Zauber, der die Verstaerkung nutzt. Ein gieriges");
    println!("     Verfahren kann ein solches Paar nicht finden.");
    // **Beide Zahlen kommen aus DIESEM Lauf, nicht aus dem Fliesstext.** Eine
    // verdrahtete 25 stimmte, bis jemand einen Verstaerker ergaenzt — und
    // stuende dann als Beleg neben einer Zahl, die sie nicht mehr belegt.
    //
    // **Und die Zusicherung, ohne die die Null nichts bedeutet:** Sie zaehlt
    // ueber ein Textmuster. Schreibt die APL-Ausgabe eines Tages nicht mehr
    // `if buff.`, meldet dieser Bericht still „null Verstaerker-Zeilen" —
    // richtig aussehend und aus dem falschen Grund. Deshalb wird an einer
    // echten Kandidatenzeile geprueft, dass das Muster ueberhaupt entsteht.
    let muster_traegt = pool
        .iter()
        .filter(|k| k.origin == Origin::BuffWindow)
        .any(|k| k.rule().to_string().contains("if buff."));
    assert!(
        muster_traegt || b == 0,
        "keine der {b} Verstaerkungs-Kandidatenzeilen enthaelt `if buff.` — dann zaehlt \
         die Zahl unten die Schreibweise der APL-Ausgabe, nicht die gewaehlten Zeilen"
    );
    let gewaehlte_buff_zeilen: usize = gefundene
        .iter()
        .map(|g| {
            g.apl
                .to_string()
                .lines()
                .filter(|z| z.contains("if buff."))
                .count()
        })
        .sum();
    println!("     Gemessen in DIESEM Lauf: {b} der Kandidatenzeilen stammen aus Verstaerkungs-");
    println!(
        "     Fenstern, und ueber alle gefundenen Rotationen zusammen stehen {gewaehlte_buff_zeilen}"
    );
    println!("     davon in einer Liste. Der Nachlauf aus #558 aendert das nicht — er nimmt");
    println!("     fremde Startpunkte, findet aber nichts, was an keinem Boss gefunden wurde.");
    println!("     WER DIE LISTE DER NIE GEWIRKTEN ZAUBER ALS BALANCE-AUSSAGE LIEST, ZIEHT");
    println!("     DESHALB EINEN ZU WEITEN SCHLUSS: Ein Verstaerker kann tragen und hier");
    println!("     trotzdem stehen.");
    println!();
    println!("Wie diese Zahlen zu lesen sind");
    println!("  Der Vorsprung der Suche ist klein — bei test-metronome 120 von 31000, also");
    println!("  0,4 %. Das ist KEIN magerer Erfolg der Suche, sondern ein Befund ueber die");
    println!("  Handschrift:");
    println!();
    println!("    Eine von Hand geschriebene Liste, die eine erschoepfend geprueft");
    println!("    arbeitende Suche nur um 0,4 % unterbietet, ist sehr gut.");
    println!();
    println!("  Der Wert dieser Suche liegt deshalb nicht darin, bessere Rotationen zu");
    println!("  finden. Er liegt darin, dass wir kuenftig WISSEN, ob eine Handschrift gut");
    println!("  ist, statt es zu hoffen. Vorher war jede Balance-Zahl eine Untergrenze mit");
    println!("  unbekanntem Abstand nach oben.");
    println!();
    println!("Grenzen dieser Zahlen");
    println!("  Dieser Bericht sucht Listen bis Laenge {max_len}.");
    println!();
    println!("  Die Laengengrenze ist SEPARAT geprueft, und zwar bei einer anderen Zahl:");
    println!("  In sim/tests/rotation_search.rs laeuft die Gegenprobe mit Laenge 4, weil");
    println!("  dort erschoepfend mitgemessen wird. Dort ergab eine Laenge mehr keinen");
    println!("  Unterschied — 5960,0 (205 Listen) gegen 5960,0 (325 Listen).");
    println!();
    println!("  Die beiden Zahlen {max_len} und 4 gehoeren also zu verschiedenen Laeufen und");
    println!("  duerfen nicht verwechselt werden. Fuer Laenge {max_len} selbst ist NICHT");
    println!("  geprueft, ob eine laengere Liste besser waere — erschoepfend waere das bei");
    println!("  siebzehn Kandidaten unerreichbar.");
    println!();
    println!("  DAS ERGEBNIS IST EINE UNTERGRENZE, KEIN BEWIESENES OPTIMUM.");
    println!("  Es sagt: besser als das hier wurde im abgesuchten Raum nichts gefunden.");
    println!("  Es sagt nicht, dass es nichts Besseres gibt.");
    let _ = index_of("kindle");
    let _ = Stop::NoImprovement;
}
