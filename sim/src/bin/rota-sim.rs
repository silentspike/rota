//! `rota-sim` — der Einstiegspunkt in den Simulator.
//!
//! ```text
//! rota-sim run   --boss boss-1-1
//! rota-sim data  bosses
//! rota-sim bench --fights 2000
//! ```
//!
//! # Warum es dieses Programm gibt
//!
//! Der Simulator hatte vierundzwanzig Beispiele und keinen Einstiegspunkt. Wer
//! etwas messen wollte, musste aus dem Quelltext erraten, welches Beispiel was
//! tut — `suchguete.rs` vergleicht Suchverfahren, `stufen-durchsatz.rs` misst
//! etwas ganz anderes, `parity.rs` prueft die Uebereinstimmung mit der
//! Browser-Seite. Fuer eine Sitzung, die diesen Quelltext nicht gelesen hat,
//! war das Werkzeug damit nicht vorhanden.
//!
//! Die Beispiele bleiben. Sie sind weiterhin die dokumentierten Messwege — sie
//! sind nur nicht mehr der Zugang.
//!
//! # Warum der Argument-Zerleger von Hand geschrieben ist
//!
//! Weil `clap` sonst in der Bibliothek laendete. In Cargo gilt
//! `[dependencies]` fuer ALLE Ziele eines Pakets; ein `[[bin]]` teilt sie mit
//! dem `lib`-Ziel. Wer `rota_sim` einbindet, bekaeme den Zerleger mit — und
//! `sim/Cargo.toml` sagt ueber seiner leeren Abhaengigkeitsliste ausdruecklich,
//! dass die Bibliothek abhaengigkeitsfrei bleibt, weil daran der Determinismus
//! aus GAME-DESIGN 18.4 haengt.
//!
//! Der zweite Grund ist der Nutzen: Dieses Programm zeigt in jeder Hilfe
//! **vollstaendige, lauffaehige Beispiele** statt einer Optionsliste. Genau die
//! erzeugt `clap` nicht — sie sind von Hand geschrieben, mit oder ohne Kiste.

use std::process::ExitCode;
use std::time::Instant;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, BalanceConfig, BalanceResult};
use rota_sim::data::{self, Boss};
use rota_sim::rotation_search::{all_candidates, greedy, Candidate, SearchResult};

/// Die gute Rotation aus Schritt 1 — die Voreinstellung fuer einen Lauf ohne
/// eigene Datei.
const GOOD_APL: &str = include_str!("../../apl/step1-good.apl");

// ---------------------------------------------------------------------------
// Einstieg
// ---------------------------------------------------------------------------

fn main() -> ExitCode {
    let argumente: Vec<String> = std::env::args().skip(1).collect();
    match fuehre_aus(&argumente) {
        Ok(code) => code,
        Err(fehler) => {
            eprintln!("{fehler}");
            ExitCode::FAILURE
        }
    }
}

/// Waehlt den Unterbefehl und fuehrt ihn aus.
///
/// Getrennt von [`main`], damit die Wahl **ohne Prozess** pruefbar ist: Ein
/// Test kann `fuehre_aus(&["--help".into()])` rufen. Ein `main`, das
/// `std::env::args` selbst liest, ist von aussen nur ueber einen Unterprozess
/// erreichbar — und dann prueft der Test den Bau mit, nicht die Wahl.
fn fuehre_aus(argumente: &[String]) -> Result<ExitCode, String> {
    let Some(befehl) = argumente.first() else {
        println!("{HILFE_WURZEL}");
        return Ok(ExitCode::FAILURE);
    };
    let rest = &argumente[1..];

    match befehl.as_str() {
        "--help" | "-h" | "help" => {
            println!("{HILFE_WURZEL}");
            Ok(ExitCode::SUCCESS)
        }
        "run" => befehl_run(rest),
        "bench" => befehl_bench(rest),
        "data" => befehl_data(rest),
        "verify" => Ok(platzhalter("verify", HILFE_VERIFY, GRUND_VERIFY, rest)),
        "hosts" => Ok(platzhalter("hosts", HILFE_HOSTS, GRUND_HOSTS, rest)),
        "status" => Ok(platzhalter("status", HILFE_STATUS, GRUND_STATUS, rest)),
        "explain" => Ok(platzhalter("explain", HILFE_EXPLAIN, GRUND_EXPLAIN, rest)),
        anderes => Err(fehler_mit_ausweg(
            &format!("Den Befehl '{anderes}' gibt es nicht."),
            aehnlichster_befehl(anderes).map(|treffer| format!("Meintest du '{treffer}'?")),
            "Alle anzeigen:  rota-sim --help",
        )),
    }
}

/// Die sieben Unterbefehle, in der Reihenfolge der Hilfe.
const BEFEHLE: [&str; 7] = [
    "run", "bench", "verify", "hosts", "status", "explain", "data",
];

/// Der Unterbefehl, der einer Fehleingabe am naechsten kommt.
fn aehnlichster_befehl(eingabe: &str) -> Option<&'static str> {
    BEFEHLE
        .iter()
        .map(|befehl| (*befehl, distanz(eingabe, befehl)))
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(_, d)| *d)
        .map(|(befehl, _)| befehl)
}

// ---------------------------------------------------------------------------
// Hilfetexte
// ---------------------------------------------------------------------------

/// Die Hilfe der obersten Ebene: jeder Unterbefehl mit einem Satz Zweck.
const HILFE_WURZEL: &str = "\
rota-sim

  Der Simulator von ROTA auf der Kommandozeile: dieselben Kampfregeln wie das
  Spiel, tausendfach schneller. Er sucht Rotationen, misst Durchsatz und zeigt,
  was in diesem Bau an Bossen und Zaubern steckt.

AUFRUF
  rota-sim BEFEHL [OPTIONEN]

BEFEHLE
  run        eine Suche oder einen Balance-Lauf fahren und die Zahlen ausgeben
  bench      den Durchsatz messen: wie viele Kaempfe je Sekunde dieser Rechner schafft
  verify     Paritaet gegen die Browser-Seite und Selbsttests fahren (noch nicht gebaut)
  hosts      die verfuegbaren Bau- und Rechen-Rechner auflisten (noch nicht gebaut)
  status     einen laufenden Lauf anzeigen (noch nicht gebaut)
  explain    zeigen, was ein Lauf TAETE, ohne ihn zu starten (noch nicht gebaut)
  data       die einkompilierten Inhalte pruefen: Bosse, Zauber, Kandidatenzeilen

BEISPIELE
  # schnellster Einstieg: die beste Rotation gegen einen Boss suchen
  rota-sim run --boss boss-1-1

  # welche Boss-Kennungen es gibt
  rota-sim data bosses

  # die Hilfe eines einzelnen Befehls, mit seinen eigenen Beispielen
  rota-sim run --help

WO DAS PROGRAMM LAEUFT
  Der Bau laeuft auf einem entfernten Rechner:
    cargo remote -c --manifest-path sim/Cargo.toml -- build --release
  Danach liegt das Programm unter target/release/rota-sim.
  Im Debug-Bau ist der Kern rund achtzehnmal langsamer — fuer einen Blick
  genuegt das, fuer tausend Durchgaenge nicht.";

/// Die Hilfe von `run` — Zweck zuerst, dann vollstaendige, lauffaehige Beispiele.
const HILFE_RUN: &str = "\
rota-sim run

  Sucht die beste Rotation gegen einen Boss — oder misst eine, die es schon
  gibt. Die Suche baut die Prioritaetenliste Zeile fuer Zeile auf und behaelt
  an jeder Stelle die Ergaenzung, die den Durchsatz am staerksten hebt.

BEISPIELE
  # schnellster Einstieg: ein Boss, Voreinstellungen
  rota-sim run --boss boss-1-1

  # eine vorhandene Rotationsdatei messen statt zu suchen
  rota-sim run --boss boss-1-1 --apl sim/apl/step1-good.apl

  # dieselben Zahlen maschinenlesbar, fuer jq und Skripte
  rota-sim run --boss boss-1-1 --json

  # genauer: mehr Durchgaenge je Messung, ueber vier Straenge, laengere Liste
  rota-sim run --boss boss-1-1 --iterations 100 --threads 4 --max-len 4

OPTIONEN
  --boss <id>        gegen welchen Boss gerechnet wird.
                     Ohne ihn gegen den Standard, der KEINEN Schaden austeilt —
                     dann ist jeder Verteidigungszauber unmessbar.
                     Alle Kennungen:  rota-sim data bosses
  --apl <datei>      eine Rotationsdatei messen statt zu suchen.
                     Der Pfad ist relativ zum Wurzelverzeichnis des Projekts,
                     also sim/apl/DATEI.apl.
  --search <art>     greedy (Voreinstellung) oder none.
                     none entspricht --apl mit der mitgelieferten Rotation.
  --max-len <n>      wie viele Zeilen die gesuchte Rotation hoechstens hat.
  --iterations <n>   wie viele Kaempfe je Messung gerechnet werden.
  --threads <n>      ueber wie viele Straenge gerechnet wird.
  --seed <n>         Startwert des ersten Durchgangs.
  --json             Ausgabe als JSON statt als Text. Dieselben Zahlen.
  --help             dieser Text.

WAS AUSGEGEBEN WIRD
  Schaden je Kampf mit Streuung, Siege, Zeit bis zum Sieg, eingesteckter
  Schaden, Tode und die Wirkungen je Zauber. Bei einer Suche zusaetzlich die
  gefundene Rotation und wie viele Messungen sie gekostet hat.";

/// Die Hilfe von `bench` — Zweck zuerst, dann Beispiele.
const HILFE_BENCH: &str = "\
rota-sim bench

  Misst, wie viele Kaempfe dieser Rechner je Sekunde schafft. Die Zahl sagt,
  wie gross ein Lauf werden darf, bevor er die Geduld ueberschreitet — und sie
  verraet, ob eine Maschine gerade belegt ist.

BEISPIELE
  # schnellster Einstieg: Voreinstellungen
  rota-sim bench

  # genauer, mit mehr Kaempfen und ueber vier Straenge
  rota-sim bench --fights 20000 --threads 4

  # maschinenlesbar
  rota-sim bench --json

OPTIONEN
  --fights <n>    wie viele Kaempfe gerechnet werden.
  --threads <n>   ueber wie viele Straenge.
  --boss <id>     gegen welchen Boss. Ohne ihn gegen den Standard.
  --json          Ausgabe als JSON statt als Text.
  --help          dieser Text.

WAS DIE ZAHL BEDEUTET
  Kaempfe je Sekunde, gemessen an der Wanduhr. Sie haengt am Bau: Im
  Debug-Bau ist der Kern rund achtzehnmal langsamer als im Release. Wer sie
  vergleicht, vergleicht deshalb nur Laeufe desselben Baus auf demselben
  Rechner — und sagt dazu, ob die Maschine ruhig war.";

/// Die Hilfe von `data` — Zweck zuerst, dann Beispiele.
const HILFE_DATA: &str = "\
rota-sim data

  Zeigt, was in diesem Bau steckt: welche Bosse und Zauber es gibt und wie
  viele Kandidatenzeilen die Suche zu waehlen hat. Der erste Griff, wenn eine
  Kennung nicht angenommen wird oder eine Zahl unerwartet aussieht.

BEISPIELE
  # schnellster Einstieg: die Uebersicht
  rota-sim data

  # alle Boss-Kennungen, wie --boss sie erwartet
  rota-sim data bosses

  # alle Zauber mit Kennung und Name
  rota-sim data spells

  # wie viele Kandidatenzeilen die Suche zu waehlen hat
  rota-sim data candidates

UNTERPUNKTE
  (ohne)        Uebersicht mit allen Zahlen
  bosses        Kennung, Name und Instanz je Boss
  spells        Kennung und Name je Zauber
  candidates    Anzahl der Kandidatenzeilen, aufgeschluesselt nach Herkunft

OPTIONEN
  --json        Ausgabe als JSON statt als Text. Dieselben Zahlen.
  --help        dieser Text.

WOHER DIE ZAHLEN KOMMEN
  Aus den Werten, die build.rs zur Bauzeit aus data/*.json erzeugt hat — nicht
  aus den JSON-Dateien selbst. Weicht eine Zahl von dem ab, was in data/ steht,
  ist der Bau alt und nicht die Datei falsch.

PHAENOTYPEN
  Die Faltung des Suchraums — wie viele verschiedene Einstellungen dieselbe
  Wirkung haben — zaehlt dieses Programm NICHT. Die Laeufer dafuer liegen als
  JavaScript neben dem Simulator:
    node tools/phaenotypen-gear.mjs
    node tools/phaenotypen-talente-analytisch.mjs
  Sie hier nachzubauen waere eine zweite Quelle fuer dieselbe Zahl.";

// ---------------------------------------------------------------------------
// Platzhalter
// ---------------------------------------------------------------------------

/// Die Hilfe von `verify`.
///
/// **Ohne eine einzige Ziffer**, wie die Ausgabe des Befehls selbst — dann ist
/// die Zusicherung aus #897 auch fuer den Hilfe-Aufruf erfuellt und muss nicht
/// zwischen zwei Aufrufarten unterscheiden.
const HILFE_VERIFY: &str = "\
rota-sim verify

  Wird pruefen, ob der Simulator dieselben Zahlen liefert wie das Spiel im
  Browser, und dabei auch, ob mehrere Rechner untereinander uebereinstimmen.
  Weicht einer ab, ist das ein Hinweis auf einen Fehler — hier oder dort, und
  er wird gemeldet statt weggerechnet.

  NOCH NICHT GEBAUT.

BEISPIELE
  # was der Aufruf heute tut: er sagt, was ihm fehlt
  rota-sim verify

  # und der Griff, der heute schon prueft, ob dieser Bau die Daten hat
  rota-sim data

WEG HEUTE OHNE DIESEN BEFEHL
  Der Vergleich fuer EINEN Rechner liegt als Beispiel vor:

    cargo remote -c --manifest-path sim/Cargo.toml -- run --release --example parity";

/// Was `verify` braucht und noch nicht hat.
const GRUND_VERIFY: &str = "\
Was fehlt: der Vergleich mit der Browser-Seite ueber mehrere Rechner. Die
Pruefdateien und der Vergleich fuer EINEN Rechner liegen bereits als Beispiel
vor und laufen heute so:

  cargo remote -c --manifest-path sim/Cargo.toml -- run --release --example parity

Der Unterbefehl fasst das zusammen, sobald es die Rechner-Verwaltung gibt —
siehe hosts. Beides gehoert in ein eigenes Issue des Epics rota-sim.

Was er ausserdem kann, steht in:  rota-sim verify --help";

/// Die Hilfe von `hosts` — ebenfalls ohne Ziffer.
const HILFE_HOSTS: &str = "\
rota-sim hosts

  Wird die Rechner auflisten, auf die ein Lauf verteilt werden kann, jeden mit
  seiner gemessenen Leistung. Ohne diese Messung waere die Liste irrefuehrend:
  Zwei Rechner mit gleicher Kernzahl schaffen nicht gleich viele Kaempfe.

  NOCH NICHT GEBAUT.

BEISPIELE
  # was der Aufruf heute tut: er sagt, was ihm fehlt
  rota-sim hosts

  # den Durchsatz DIESES Rechners misst heute schon
  rota-sim bench";

/// Was `hosts` braucht und noch nicht hat.
const GRUND_HOSTS: &str = "\
Was fehlt: eine Liste der Rechen-Rechner mit ihrer Kalibrierung. Ohne die
Kalibrierung waere eine Liste irrefuehrend — zwei Rechner mit gleicher
Kernzahl schaffen nicht gleich viele Kaempfe, und wer ohne diese Zahl
verteilt, verteilt falsch.

Den Durchsatz DIESES Rechners misst heute schon:

  rota-sim bench

Was er ausserdem kann, steht in:  rota-sim hosts --help";

/// Die Hilfe von `status` — ebenfalls ohne Ziffer.
const HILFE_STATUS: &str = "\
rota-sim status

  Wird zeigen, wie weit ein laufender Lauf gekommen ist, ohne ihn zu stoeren.
  Sinnvoll erst, wenn Laeufe im Hintergrund oder auf fremden Rechnern
  stattfinden — heute laeuft jede Messung im Vordergrund und endet mit ihrer
  eigenen Ausgabe.

  NOCH NICHT GEBAUT.

BEISPIELE
  # was der Aufruf heute tut: er sagt, was ihm fehlt
  rota-sim status

  # wie lange ein Lauf ueberhaupt dauern wird, schaetzt heute
  rota-sim bench";

/// Was `status` braucht und noch nicht hat.
const GRUND_STATUS: &str = "\
Was fehlt: ein laufender Lauf, den man anzeigen koennte. Heute laeuft jede
Messung im Vordergrund und endet mit ihrer Ausgabe; es gibt nichts, was
zwischendurch nach seinem Zustand gefragt werden koennte.

Der Unterbefehl bekommt erst einen Sinn, wenn Laeufe im Hintergrund oder auf
fremden Rechnern laufen — siehe hosts.

Was er ausserdem kann, steht in:  rota-sim status --help";

/// Die Hilfe von `explain` — ebenfalls ohne Ziffer.
const HILFE_EXPLAIN: &str = "\
rota-sim explain

  Wird sagen, was ein Lauf taete, ohne ihn zu starten: wie viele Messungen er
  braucht und wie lange sie dauern. Die zweite Haelfte ist die interessante —
  ein Trockenlauf, der nur die Einstellungen wiederholt, beantwortet die Frage
  nicht.

  NOCH NICHT GEBAUT.

BEISPIELE
  # was der Aufruf heute tut: er sagt, was ihm fehlt
  rota-sim explain

  # wie viele Kandidatenzeilen zur Wahl stehen, sagt heute schon
  rota-sim data candidates";

/// Was `explain` braucht und noch nicht hat.
const GRUND_EXPLAIN: &str = "\
Was fehlt: die Kostenschaetzung. Ein Trockenlauf, der nur die Einstellungen
wiederholt, ist keine Antwort auf die Frage 'was taete dieser Lauf' — die
interessante Haelfte ist, wie viele Messungen er braucht und wie lange das
dauert. Dafuer fehlt die Kalibrierung aus hosts.

Wie viele Kandidatenzeilen zur Wahl stehen, sagt heute schon:

  rota-sim data candidates

Was er ausserdem kann, steht in:  rota-sim explain --help";

/// Gibt aus, was einem noch nicht gebauten Unterbefehl fehlt — oder seine Hilfe.
///
/// **Ohne eine einzige Ziffer**, in beiden Faellen. Ein Platzhalter, der eine
/// Zahl nennt, ist von einem Ergebnis nicht zu unterscheiden — und eine
/// plausible falsche Zahl ist schlimmer als keine. Der Test
/// `platzhalter_nennen_keine_zahl` haelt das fest; er prueft die AUSGABE,
/// nicht die Absicht.
///
/// Der Rueckgabewert ohne `--help` ist bewusst ein Fehlschlag: Ein Skript, das
/// den Befehl ruft und nur den Rueckgabewert prueft, soll nicht denken, es
/// habe gemessen. **Mit `--help` ist er Erfolg** — wer nach der Hilfe fragt,
/// hat keinen Fehler gemacht.
fn platzhalter(name: &str, hilfe: &str, grund: &str, argumente: &[String]) -> ExitCode {
    if argumente.iter().any(|a| a == "--help" || a == "-h") {
        println!("{hilfe}");
        return ExitCode::SUCCESS;
    }
    println!("rota-sim {name} — noch nicht gebaut.\n");
    println!("{grund}");
    ExitCode::FAILURE
}

// DIE HILFE WIRD NICHT HIER GEPRUEFT, sondern in `tests/rota_sim_beispiele.rs`.
//
// Das ist Absicht. Wer die Beispiele aus einer KONSTANTE liest, prueft den
// Text; wer sie aus der AUSGABE des gebauten Programms liest, prueft das
// Werkzeug. Nur die zweite Fassung faengt einen Fehler in der Ausgabe selbst —
// und sie kann die Beispiele anschliessend ausfuehren, was #968 AC4 verlangt.
//
// Eine Kopie der Zerlegeregel hier waere eine zweite Quelle fuer dieselbe
// Frage, und die beiden liefen irgendwann auseinander.

/// Baut eine Fehlermeldung in der Form aus #968, AC3.
///
/// ```text
/// FEHLER  Boss 'boss-2-5' gibt es nicht.
///         Meintest du 'boss-2-4'?
///         Alle anzeigen:  rota-sim data bosses
/// ```
///
/// **Der Ausweg ist Pflicht, der Vorschlag nicht.** Ein Vorschlag entsteht nur,
/// wenn etwas nahe genug liegt; ein geratener fuehrt weiter weg als keiner.
/// Ein Ausweg dagegen laesst sich immer nennen — und ohne ihn ist die Meldung
/// nur die Feststellung, dass es nicht ging.
fn fehler_mit_ausweg(was: &str, vorschlag: Option<String>, ausweg: &str) -> String {
    let mut zeilen = vec![format!("FEHLER  {was}")];
    if let Some(satz) = vorschlag {
        zeilen.push(format!("        {satz}"));
    }
    zeilen.push(format!("        {ausweg}"));
    zeilen.join("\n")
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

/// Die Einstellungen von `run`, wie sie aus der Kommandozeile kommen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunOptionen {
    boss_id: Option<String>,
    apl_pfad: Option<String>,
    suche: Suchart,
    max_len: usize,
    iterations: u32,
    threads: usize,
    seed: u32,
    json: bool,
    hilfe: bool,
}

/// Welches Suchverfahren `run` faehrt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suchart {
    /// Gar keine Suche — eine vorhandene Rotation messen.
    Keine,
    /// Zeile fuer Zeile die beste Ergaenzung waehlen.
    Greedy,
}

impl Default for RunOptionen {
    fn default() -> Self {
        Self {
            boss_id: None,
            apl_pfad: None,
            suche: Suchart::Greedy,
            // DIE VOREINSTELLUNGEN SIND AN DER PRUEFSTRECKE GEMESSEN, nicht
            // geschaetzt (#968, AC4): Jedes Beispiel aus der Hilfe wird
            // ausgefuehrt, und Tests laufen im DEBUG-Bau, wo der Kern rund
            // achtzehnmal langsamer ist.
            //
            //   iterations 50, max_len 4   10,0 s im Debug
            //   iterations 20, max_len 3    1,9 s im Debug
            //
            // Die Genauigkeit kostet das kaum: Solange kein Krit existiert
            // (PURITY = 0), zieht kein Kampf aus den Zufallsstroemen und alle
            // Durchgaenge sind gleich — der Standardfehler ist null, ob man
            // zwanzigmal rechnet oder tausendmal. Wer genauer will, setzt
            // --iterations; die Hilfe zeigt genau das als Beispiel.
            max_len: 3,
            iterations: 20,
            threads: 1,
            seed: 0,
            json: false,
            hilfe: false,
        }
    }
}

fn befehl_run(argumente: &[String]) -> Result<ExitCode, String> {
    let optionen = zerlege_run(argumente)?;
    if optionen.hilfe {
        println!("{HILFE_RUN}");
        return Ok(ExitCode::SUCCESS);
    }

    let boss = waehle_boss(optionen.boss_id.as_deref())?;
    let config = BalanceConfig {
        iterations: optionen.iterations,
        base_seed: optionen.seed,
        threads: optionen.threads,
        boss,
        ..BalanceConfig::default()
    };

    // Eine eigene Datei heisst immer "messen", nie "suchen" — sonst wuerde der
    // Aufruf die Datei stillschweigend ignorieren.
    let sucht = optionen.suche == Suchart::Greedy && optionen.apl_pfad.is_none();

    let start = Instant::now();
    if sucht {
        let pool = all_candidates();
        let ergebnis = greedy(&config, &pool, optionen.max_len);
        let dauer = start.elapsed().as_secs_f64();
        if optionen.json {
            println!("{}", such_json(&ergebnis, boss, dauer));
        } else {
            berichte_suche(&ergebnis, boss, &config, dauer);
        }
    } else {
        let (name, apl) = lade_apl(optionen.apl_pfad.as_deref())?;
        let ergebnis = run(&config, &apl);
        let dauer = start.elapsed().as_secs_f64();
        if optionen.json {
            println!(
                "{{\"boss\":{:?},\"elapsedMs\":{:?},\"result\":{}}}",
                boss.id,
                dauer * 1000.0,
                ergebnis.to_json()
            );
        } else {
            berichte_lauf(&name, &ergebnis, boss, dauer);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn zerlege_run(argumente: &[String]) -> Result<RunOptionen, String> {
    let mut optionen = RunOptionen::default();
    let mut i = 0;
    while i < argumente.len() {
        let name = argumente[i].as_str();
        match name {
            "--help" | "-h" => optionen.hilfe = true,
            "--json" => optionen.json = true,
            "--boss" => optionen.boss_id = Some(hole_wert(argumente, &mut i, name)?),
            "--apl" => optionen.apl_pfad = Some(hole_wert(argumente, &mut i, name)?),
            "--search" => {
                let wert = hole_wert(argumente, &mut i, name)?;
                optionen.suche = match wert.as_str() {
                    "greedy" => Suchart::Greedy,
                    "none" => Suchart::Keine,
                    anderes => {
                        return Err(fehler_mit_ausweg(
                            &format!("Die Suchart '{anderes}' gibt es nicht."),
                            Some("Moeglich sind: 'greedy', 'none'".to_owned()),
                            "Mit Beispielen:  rota-sim run --help",
                        ))
                    }
                };
            }
            "--max-len" => optionen.max_len = hole_zahl(argumente, &mut i, name)?,
            "--iterations" => optionen.iterations = hole_zahl(argumente, &mut i, name)?,
            "--threads" => optionen.threads = hole_zahl(argumente, &mut i, name)?,
            "--seed" => optionen.seed = hole_zahl(argumente, &mut i, name)?,
            anderes => return Err(unbekannte_angabe("run", anderes, &ANGABEN_RUN)),
        }
        i += 1;
    }
    if optionen.iterations == 0 {
        return Err(fehler_mit_ausweg(
            "'--iterations' muss groesser als null sein.",
            Some("Ein Lauf ohne Durchgang hat nichts gerechnet, und sein Mittelwert waere trotzdem eine Zahl.".to_owned()),
            "Mit Beispielen:  rota-sim run --help",
        ));
    }
    if optionen.max_len == 0 {
        return Err(fehler_mit_ausweg(
            "'--max-len' muss groesser als null sein.",
            Some("Eine Rotation ohne Zeile ist keine Rotation.".to_owned()),
            "Mit Beispielen:  rota-sim run --help",
        ));
    }
    Ok(optionen)
}

/// Die Angaben, die `run` kennt — Grundlage fuer den Vorschlag bei Tippfehlern.
const ANGABEN_RUN: [&str; 10] = [
    "--boss",
    "--apl",
    "--search",
    "--max-len",
    "--iterations",
    "--threads",
    "--seed",
    "--json",
    "--help",
    "-h",
];

/// Die Angaben, die `bench` kennt.
const ANGABEN_BENCH: [&str; 6] = ["--boss", "--fights", "--threads", "--json", "--help", "-h"];

/// Die Angaben, die `data` kennt.
const ANGABEN_DATA: [&str; 3] = ["--json", "--help", "-h"];

/// Die Unterpunkte von `data`.
const UNTERPUNKTE_DATA: [&str; 3] = ["bosses", "spells", "candidates"];

/// Laedt die zu messende Rotation — aus einer Datei oder die mitgelieferte.
fn lade_apl(pfad: Option<&str>) -> Result<(String, Apl), String> {
    match pfad {
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|fehler| {
                fehler_mit_ausweg(
                    &format!("Die Datei '{p}' ist nicht lesbar: {fehler}"),
                    Some(
                        "Der Pfad gilt vom Wurzelverzeichnis des Projekts aus, also sim/apl/DATEI.apl."
                            .to_owned(),
                    ),
                    "Vorhandene Rotationen:  ls sim/apl/",
                )
            })?;
            let apl = parse(AplName(p), &text).map_err(|fehler| {
                fehler_mit_ausweg(
                    &format!("Die Datei '{p}' ist fehlerhaft."),
                    Some(fehler.to_string()),
                    "Die Form mit Beispielen:  rota-sim run --help",
                )
            })?;
            Ok((p.to_owned(), apl))
        }
        None => {
            let apl = parse(AplName("step1-good"), GOOD_APL).map_err(|fehler| {
                format!("Die mitgelieferte Rotation ist fehlerhaft:\n{fehler}")
            })?;
            Ok(("step1-good".to_owned(), apl))
        }
    }
}

fn berichte_lauf(name: &str, ergebnis: &BalanceResult, boss: &Boss, dauer: f64) {
    println!("{}", "=".repeat(72));
    println!("ROTA — Balance-Lauf gegen {} ({})", boss.name, boss.id);
    println!(
        "Durchgaenge {}, Straenge {}, Startwert ab {}, Liste {name}",
        ergebnis.iterations, ergebnis.threads, ergebnis.base_seed
    );
    println!("{}", "=".repeat(72));
    println!();
    println!(
        "Schaden je Kampf     {:.1}  (Streuung {:.1}, Standardfehler {:.2})",
        ergebnis.damage.mean, ergebnis.damage.stddev, ergebnis.damage.stderr
    );
    println!(
        "Siege                {} von {}  ({:.1} Prozent)",
        ergebnis.kills,
        ergebnis.samples.len(),
        ergebnis.kill_rate() * 100.0
    );
    if ergebnis.kills > 0 {
        println!(
            "Zeit bis zum Sieg    {:.0} ms  (kuerzeste {:.0}, laengste {:.0})",
            ergebnis.kill_time_ms.mean, ergebnis.kill_time_ms.min, ergebnis.kill_time_ms.max
        );
    }
    println!(
        "Schaden eingesteckt  {:.1}  (davon abgefangen {:.1})",
        ergebnis.damage_taken.mean, ergebnis.absorbed.mean
    );
    println!("Tode                 {}", ergebnis.deaths);
    println!();
    println!("Wirkungen je Zauber");
    for (i, anzahl) in ergebnis.casts.iter().enumerate() {
        if *anzahl > 0 {
            println!("  {:<20} {anzahl}", data::SPELLS[i].id);
        }
    }
    println!();
    println!("Gerechnet in {dauer:.2} Sekunden.");
}

fn berichte_suche(ergebnis: &SearchResult, boss: &Boss, config: &BalanceConfig, dauer: f64) {
    println!("{}", "=".repeat(72));
    println!("ROTA — Suche gegen {} ({})", boss.name, boss.id);
    println!(
        "Durchgaenge je Messung {}, Straenge {}, Startwert ab {}",
        config.iterations, config.threads, config.base_seed
    );
    println!("{}", "=".repeat(72));
    println!();
    println!(
        "Bester Wert          {:.1}  (Standardfehler {:.2})",
        ergebnis.best.mean, ergebnis.best.stderr
    );
    println!("Kandidatenzeilen     {}", ergebnis.pool_size);
    println!("Messungen            {}", ergebnis.measured);
    println!("Abbruchgrund         {:?}", ergebnis.stop);
    println!();
    println!("Gefundene Rotation");
    if ergebnis.chosen.is_empty() {
        println!("  (leer — keine Zeile war besser als gar keine)");
    } else {
        for (nummer, kandidat) in ergebnis.chosen.iter().enumerate() {
            println!(
                "  {}. {:<20} {}",
                nummer + 1,
                data::SPELLS[kandidat.spell].id,
                kandidat.origin.label()
            );
        }
    }
    println!();
    println!("Gerechnet in {dauer:.2} Sekunden.");
}

fn such_json(ergebnis: &SearchResult, boss: &Boss, dauer: f64) -> String {
    let zeilen: Vec<String> = ergebnis
        .chosen
        .iter()
        .map(|k: &Candidate| {
            format!(
                "{{\"spell\":{:?},\"origin\":{:?}}}",
                data::SPELLS[k.spell].id,
                k.origin.label()
            )
        })
        .collect();
    let stop = format!("{:?}", ergebnis.stop);
    format!(
        "{{\"boss\":{:?},\"elapsedMs\":{:?},\"mean\":{:?},\"stderr\":{:?},\
         \"poolSize\":{},\"measured\":{},\"stop\":{:?},\"rotation\":[{}]}}",
        boss.id,
        dauer * 1000.0,
        ergebnis.best.mean,
        ergebnis.best.stderr,
        ergebnis.pool_size,
        ergebnis.measured,
        stop,
        zeilen.join(",")
    )
}

// ---------------------------------------------------------------------------
// bench
// ---------------------------------------------------------------------------

/// Die Einstellungen von `bench`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchOptionen {
    fights: u32,
    threads: usize,
    boss_id: Option<String>,
    json: bool,
    hilfe: bool,
}

impl Default for BenchOptionen {
    fn default() -> Self {
        Self {
            fights: 2000,
            threads: 1,
            boss_id: None,
            json: false,
            hilfe: false,
        }
    }
}

fn befehl_bench(argumente: &[String]) -> Result<ExitCode, String> {
    let optionen = zerlege_bench(argumente)?;
    if optionen.hilfe {
        println!("{HILFE_BENCH}");
        return Ok(ExitCode::SUCCESS);
    }

    let boss = waehle_boss(optionen.boss_id.as_deref())?;
    let (_, apl) = lade_apl(None)?;
    let config = BalanceConfig {
        iterations: optionen.fights,
        threads: optionen.threads,
        boss,
        ..BalanceConfig::default()
    };

    let start = Instant::now();
    let ergebnis = run(&config, &apl);
    let sekunden = start.elapsed().as_secs_f64();
    // Nie durch null teilen: Ein Lauf, der unter der Aufloesung der Uhr bleibt,
    // hat keinen messbaren Durchsatz — und eine unendliche Zahl waere die
    // schlechteste aller Antworten.
    let je_sekunde = if sekunden > 0.0 {
        f64::from(optionen.fights) / sekunden
    } else {
        f64::NAN
    };

    if optionen.json {
        println!(
            "{{\"boss\":{:?},\"fights\":{},\"threads\":{},\"elapsedMs\":{:?},\
             \"fightsPerSecond\":{:?},\"damageMean\":{:?}}}",
            boss.id,
            optionen.fights,
            optionen.threads,
            sekunden * 1000.0,
            je_sekunde,
            ergebnis.damage.mean
        );
    } else {
        println!("{}", "=".repeat(72));
        println!("ROTA — Durchsatz gegen {} ({})", boss.name, boss.id);
        println!("{}", "=".repeat(72));
        println!();
        println!("Kaempfe              {}", optionen.fights);
        println!("Straenge             {}", optionen.threads);
        println!("Gerechnet in         {sekunden:.3} Sekunden");
        println!("Kaempfe je Sekunde   {je_sekunde:.0}");
        println!();
        println!(
            "Zur Kontrolle: Schaden je Kampf {:.1} — weicht der zwischen zwei",
            ergebnis.damage.mean
        );
        println!("Laeufen ab, hat der Lauf nicht dasselbe gerechnet.");
    }
    Ok(ExitCode::SUCCESS)
}

fn zerlege_bench(argumente: &[String]) -> Result<BenchOptionen, String> {
    let mut optionen = BenchOptionen::default();
    let mut i = 0;
    while i < argumente.len() {
        let name = argumente[i].as_str();
        match name {
            "--help" | "-h" => optionen.hilfe = true,
            "--json" => optionen.json = true,
            "--boss" => optionen.boss_id = Some(hole_wert(argumente, &mut i, name)?),
            "--fights" => optionen.fights = hole_zahl(argumente, &mut i, name)?,
            "--threads" => optionen.threads = hole_zahl(argumente, &mut i, name)?,
            anderes => return Err(unbekannte_angabe("bench", anderes, &ANGABEN_BENCH)),
        }
        i += 1;
    }
    if optionen.fights == 0 {
        return Err(fehler_mit_ausweg(
            "'--fights' muss groesser als null sein.",
            Some("Null Kaempfe je Sekunde waere durch null geteilt, nicht gemessen.".to_owned()),
            "Mit Beispielen:  rota-sim bench --help",
        ));
    }
    Ok(optionen)
}

// ---------------------------------------------------------------------------
// data
// ---------------------------------------------------------------------------

fn befehl_data(argumente: &[String]) -> Result<ExitCode, String> {
    let mut json = false;
    let mut unterpunkt: Option<String> = None;
    for argument in argumente {
        match argument.as_str() {
            "--help" | "-h" => {
                println!("{HILFE_DATA}");
                return Ok(ExitCode::SUCCESS);
            }
            "--json" => json = true,
            anderes if anderes.starts_with('-') => {
                return Err(unbekannte_angabe("data", anderes, &ANGABEN_DATA))
            }
            anderes => unterpunkt = Some(anderes.to_owned()),
        }
    }

    match unterpunkt.as_deref() {
        None => data_uebersicht(json),
        Some("bosses") => data_bosses(json),
        Some("spells") => data_spells(json),
        Some("candidates") => data_candidates(json),
        Some(anderes) => {
            let vorschlag = UNTERPUNKTE_DATA
                .iter()
                .map(|kandidat| (*kandidat, distanz(anderes, kandidat)))
                .filter(|(_, d)| *d <= 3)
                .min_by_key(|(_, d)| *d)
                .map(|(treffer, _)| format!("Meintest du '{treffer}'?"));
            return Err(fehler_mit_ausweg(
                &format!("Den Unterpunkt '{anderes}' kennt 'data' nicht."),
                vorschlag,
                "Alle anzeigen:  rota-sim data --help",
            ));
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn data_uebersicht(json: bool) {
    let kandidaten = all_candidates();
    let instanzen = anzahl_instanzen();
    if json {
        println!(
            "{{\"bosses\":{},\"spells\":{},\"instances\":{},\"candidates\":{}}}",
            data::BOSSES.len(),
            data::SPELL_COUNT,
            instanzen,
            kandidaten.len()
        );
        return;
    }
    println!("Einkompilierte Inhalte dieses Baus");
    println!();
    println!("  Bosse              {}", data::BOSSES.len());
    println!("  Zauber             {}", data::SPELL_COUNT);
    println!("  Instanzen          {instanzen}");
    println!("  Kandidatenzeilen   {}", kandidaten.len());
    println!();
    println!("Einzeln:  rota-sim data bosses | spells | candidates");
}

fn data_bosses(json: bool) {
    if json {
        let eintraege: Vec<String> = data::BOSSES
            .iter()
            .map(|b| {
                format!(
                    "{{\"id\":{:?},\"name\":{:?},\"instance\":{:?},\"hp\":{}}}",
                    b.id, b.name, b.instance, b.hp
                )
            })
            .collect();
        println!("[{}]", eintraege.join(","));
        return;
    }
    // Zwanzig Zeichen fuer die Kennung, weil die laengste im Bestand
    // (test-crucible-trial) neunzehn hat — bei vierzehn verschob sie die
    // ganze Zeile nach rechts, und die Spalte daneben war keine mehr.
    println!("{:<20} {:<28} {:<20} LEBEN", "KENNUNG", "NAME", "INSTANZ");
    for boss in data::BOSSES {
        println!(
            "{:<20} {:<28} {:<20} {}",
            boss.id, boss.name, boss.instance, boss.hp
        );
    }
    println!();
    println!(
        "{} Bosse. Als Wert fuer --boss dient die KENNUNG.",
        data::BOSSES.len()
    );
}

fn data_spells(json: bool) {
    if json {
        let eintraege: Vec<String> = data::SPELLS
            .iter()
            .map(|s| format!("{{\"id\":{:?},\"name\":{:?}}}", s.id, s.name))
            .collect();
        println!("[{}]", eintraege.join(","));
        return;
    }
    println!("{:<22} NAME", "KENNUNG");
    for zauber in data::SPELLS {
        println!("{:<22} {}", zauber.id, zauber.name);
    }
    println!();
    println!("{} Zauber.", data::SPELL_COUNT);
}

fn data_candidates(json: bool) {
    let kandidaten = all_candidates();
    // Nach Herkunft zaehlen, ueber die Beschriftung — sie ist die einzige
    // oeffentliche Sicht auf `Origin` und bleibt gueltig, wenn eine Art
    // hinzukommt.
    let mut arten: Vec<(&'static str, usize)> = Vec::new();
    for kandidat in &kandidaten {
        let beschriftung = kandidat.origin.label();
        if let Some(eintrag) = arten.iter_mut().find(|(name, _)| *name == beschriftung) {
            eintrag.1 += 1;
        } else {
            arten.push((beschriftung, 1));
        }
    }

    if json {
        let eintraege: Vec<String> = arten
            .iter()
            .map(|(name, anzahl)| format!("{{\"origin\":{name:?},\"count\":{anzahl}}}"))
            .collect();
        println!(
            "{{\"total\":{},\"byOrigin\":[{}]}}",
            kandidaten.len(),
            eintraege.join(",")
        );
        return;
    }
    println!("Kandidatenzeilen, die der Suche zur Wahl stehen");
    println!();
    for (name, anzahl) in &arten {
        println!("  {name:<28} {anzahl}");
    }
    println!();
    println!("  {:<28} {}", "zusammen", kandidaten.len());
}

/// Zaehlt die verschiedenen Instanzen ueber die Bosse.
///
/// Ohne Container mit Streuung — GAME-DESIGN 18.4 verbietet Hash-Abbildungen
/// dort, wo die Reihenfolge in ein Ergebnis einfliessen koennte. Bei
/// zweistelligen Zahlen ist die lineare Suche ohnehin nicht messbar langsamer.
fn anzahl_instanzen() -> usize {
    let mut gesehen: Vec<&'static str> = Vec::new();
    for boss in data::BOSSES {
        if !gesehen.contains(&boss.instance) {
            gesehen.push(boss.instance);
        }
    }
    gesehen.len()
}

// ---------------------------------------------------------------------------
// Gemeinsames
// ---------------------------------------------------------------------------

/// Sucht den Boss zur Kennung — und nennt bei einem Fehlgriff einen Ausweg.
///
/// Der Ausweg ist ein **Vorschlag plus der Befehl, der alle auflistet**. Eine
/// Liste aller Kennungen allein taugt dafuer nicht: Bei zweistelligen Zahlen
/// ist sie eine Wand, durch die man den einen Tippfehler nicht sieht.
fn waehle_boss(id: Option<&str>) -> Result<&'static Boss, String> {
    let Some(id) = id else {
        return Ok(BalanceConfig::default().boss);
    };
    if let Some(boss) = data::boss_by_id(id) {
        return Ok(boss);
    }
    Err(fehler_mit_ausweg(
        &format!("Boss '{id}' gibt es nicht."),
        aehnlichster_boss(id).map(|treffer| format!("Meintest du '{treffer}'?")),
        "Alle anzeigen:  rota-sim data bosses",
    ))
}

/// Der Boss mit der kleinsten Editierdistanz zur Eingabe.
///
/// Nur bis zu einer Distanz von drei — darueber ist der Vorschlag geraten und
/// fuehrt weiter weg als gar keiner.
///
/// # Warum die Editierdistanz allein nicht reicht
///
/// Der Fall aus AC4 ist genau der, an dem sie versagt: Zu `boss-2-5` haben
/// `boss-2-1`, `boss-2-2`, `boss-2-3` und `boss-2-4` **dieselbe** Distanz von
/// eins. Wer nur nach ihr sortiert, bekommt den, der zufaellig zuerst in den
/// Daten steht — gemessen war das `boss-2-1`, und der ist vier Plaetze vom
/// gemeinten entfernt.
///
/// Deshalb entscheidet bei Gleichstand der [`ziffernabstand`]. Er ist keine
/// Annahme ueber das Kennungsformat: Wo keine Ziffern stehen, ist er fuer alle
/// gleich und aendert nichts.
fn aehnlichster_boss(id: &str) -> Option<&'static str> {
    data::BOSSES
        .iter()
        .map(|boss| (boss.id, distanz(id, boss.id)))
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(kennung, d)| (*d, ziffernabstand(id, kennung)))
        .map(|(kennung, _)| kennung)
}

/// Wie weit die Zahlen zweier Kennungen auseinanderliegen.
///
/// Beide Zeichenketten werden in ihre Ziffernfolgen zerlegt und paarweise
/// verglichen. Unterschiedlich viele Zahlen heissen "weit weg" — dann sagt der
/// Vergleich nichts, und ein Kandidat mit passender Struktur soll gewinnen.
fn ziffernabstand(a: &str, b: &str) -> u64 {
    let zahlen = |text: &str| -> Vec<u64> {
        let mut gefunden = Vec::new();
        let mut aktuelle: Option<u64> = None;
        for zeichen in text.chars() {
            if let Some(ziffer) = zeichen.to_digit(10) {
                aktuelle = Some(aktuelle.unwrap_or(0) * 10 + u64::from(ziffer));
            } else if let Some(zahl) = aktuelle.take() {
                gefunden.push(zahl);
            }
        }
        if let Some(zahl) = aktuelle {
            gefunden.push(zahl);
        }
        gefunden
    };
    let (links, rechts) = (zahlen(a), zahlen(b));
    if links.len() != rechts.len() {
        return u64::MAX;
    }
    links
        .iter()
        .zip(rechts.iter())
        .map(|(x, y)| x.abs_diff(*y))
        .sum()
}

/// Editierdistanz nach Levenshtein.
///
/// Zwei Zeilen statt einer Matrix: Es wird immer nur die vorige Zeile
/// gebraucht. Bei Kennungen von unter zwanzig Zeichen ist das keine
/// Optimierung, sondern nur weniger Code.
fn distanz(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut vorige: Vec<usize> = (0..=b.len()).collect();
    let mut aktuelle = vec![0usize; b.len() + 1];
    for (i, zeichen_a) in a.iter().enumerate() {
        aktuelle[0] = i + 1;
        for (j, zeichen_b) in b.iter().enumerate() {
            let kosten = usize::from(zeichen_a != zeichen_b);
            aktuelle[j + 1] = (vorige[j + 1] + 1)
                .min(aktuelle[j] + 1)
                .min(vorige[j] + kosten);
        }
        std::mem::swap(&mut vorige, &mut aktuelle);
    }
    vorige[b.len()]
}

/// Holt den Wert hinter einer Angabe und schiebt den Zeiger weiter.
fn hole_wert(argumente: &[String], i: &mut usize, name: &str) -> Result<String, String> {
    *i += 1;
    argumente.get(*i).cloned().ok_or_else(|| {
        fehler_mit_ausweg(
            &format!("Nach '{name}' fehlt der Wert."),
            None,
            "Die Form mit Beispielen:  rota-sim run --help",
        )
    })
}

/// Wie [`hole_wert`], nur als Zahl.
fn hole_zahl<T: std::str::FromStr>(
    argumente: &[String],
    i: &mut usize,
    name: &str,
) -> Result<T, String> {
    let roh = hole_wert(argumente, i, name)?;
    roh.parse().map_err(|_| {
        fehler_mit_ausweg(
            &format!("'{name}' braucht eine ganze Zahl, bekommen hat es '{roh}'."),
            None,
            "Die Form mit Beispielen:  rota-sim run --help",
        )
    })
}

/// Die Meldung fuer eine Angabe, die es an dieser Stelle nicht gibt.
///
/// Der Vorschlag kommt aus den Angaben, die DIESER Unterbefehl kennt — ein
/// Treffer aus einem fremden Unterbefehl waere schlimmer als keiner, weil er
/// zum naechsten Fehlversuch fuehrt.
fn unbekannte_angabe(befehl: &str, angabe: &str, bekannte: &[&str]) -> String {
    let vorschlag = bekannte
        .iter()
        .map(|kandidat| (*kandidat, distanz(angabe, kandidat)))
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(_, d)| *d)
        .map(|(treffer, _)| format!("Meintest du '{treffer}'?"));
    fehler_mit_ausweg(
        &format!("Die Angabe '{angabe}' kennt '{befehl}' nicht."),
        vorschlag,
        &format!("Alle anzeigen:  rota-sim {befehl} --help"),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn worte(zeile: &str) -> Vec<String> {
        zeile.split_whitespace().map(ToOwned::to_owned).collect()
    }

    #[test]
    fn hilfe_der_wurzel_nennt_jeden_unterbefehl() {
        for befehl in [
            "run", "bench", "verify", "hosts", "status", "explain", "data",
        ] {
            assert!(
                HILFE_WURZEL.contains(befehl),
                "die Hilfe der Wurzel nennt {befehl} nicht"
            );
        }
    }

    // DIE DREI HILFE-PRUEFUNGEN STEHEN IN tests/rota_sim_beispiele.rs:
    // Zahl der Beispiele, Zweck vor dem ersten Abschnitt, und dass die
    // Aufrufzeile nicht als Beispiel zaehlt. Dort lesen sie die AUSGABE des
    // gebauten Programms statt dieser Konstanten — und koennen die Beispiele
    // anschliessend ausfuehren, was #968 AC4 verlangt.

    #[test]
    fn platzhalter_nennen_keine_zahl() {
        // Wortwoertlich: keine ZIFFER. Ein Platzhalter, der eine Zahl nennt,
        // ist von einem Ergebnis nicht zu unterscheiden.
        for (name, grund) in [
            ("verify", GRUND_VERIFY),
            ("hosts", GRUND_HOSTS),
            ("status", GRUND_STATUS),
            ("explain", GRUND_EXPLAIN),
        ] {
            assert!(
                !grund.chars().any(|z| z.is_ascii_digit()),
                "der Grund von {name} enthaelt eine Ziffer"
            );
            assert!(
                grund.contains("Was fehlt"),
                "der Grund von {name} sagt nicht, was fehlt"
            );
        }
    }

    #[test]
    fn zerlege_run_nimmt_die_voreinstellungen() {
        let optionen = zerlege_run(&[]).expect("leer ist gueltig");
        assert_eq!(optionen, RunOptionen::default());
        assert_eq!(optionen.suche, Suchart::Greedy);
    }

    #[test]
    fn zerlege_run_liest_jede_angabe() {
        let optionen = zerlege_run(&worte(
            "--boss boss-1-1 --apl sim/apl/step1-good.apl --search none \
             --max-len 6 --iterations 200 --threads 4 --seed 7 --json",
        ))
        .expect("alle Angaben sind gueltig");
        assert_eq!(optionen.boss_id.as_deref(), Some("boss-1-1"));
        assert_eq!(optionen.apl_pfad.as_deref(), Some("sim/apl/step1-good.apl"));
        assert_eq!(optionen.suche, Suchart::Keine);
        assert_eq!(optionen.max_len, 6);
        assert_eq!(optionen.iterations, 200);
        assert_eq!(optionen.threads, 4);
        assert_eq!(optionen.seed, 7);
        assert!(optionen.json);
    }

    #[test]
    fn zerlege_run_meldet_fehlenden_wert() {
        let fehler = zerlege_run(&worte("--boss")).expect_err("nach --boss fehlt der Wert");
        assert!(fehler.contains("--boss"), "{fehler}");
    }

    #[test]
    fn zerlege_run_meldet_null_durchgaenge() {
        // Null Durchgaenge waeren ein Lauf, der nichts gerechnet hat — und
        // dessen Mittelwert trotzdem eine Zahl waere.
        let fehler =
            zerlege_run(&worte("--iterations 0")).expect_err("null Durchgaenge sind ungueltig");
        assert!(fehler.contains("--iterations"), "{fehler}");
    }

    #[test]
    fn zerlege_run_weist_unbekannte_angabe_ab() {
        let fehler = zerlege_run(&worte("--gibtsnicht")).expect_err("die Angabe gibt es nicht");
        assert!(fehler.contains("--gibtsnicht"), "{fehler}");
        assert!(fehler.contains("rota-sim run --help"), "{fehler}");
    }

    #[test]
    fn zerlege_bench_liest_jede_angabe() {
        let optionen = zerlege_bench(&worte("--fights 500 --threads 2 --boss boss-1-1 --json"))
            .expect("alle Angaben sind gueltig");
        assert_eq!(optionen.fights, 500);
        assert_eq!(optionen.threads, 2);
        assert_eq!(optionen.boss_id.as_deref(), Some("boss-1-1"));
        assert!(optionen.json);
    }

    #[test]
    fn unbekannter_boss_schlaegt_den_naechsten_vor() {
        // Der Fall aus AC4: boss-2-5 gibt es nicht, boss-2-4 schon.
        //
        // DIESER TEST WAR ROT, BEVOR ES DEN ZIFFERNABSTAND GAB. Nach der
        // Editierdistanz allein sind boss-2-1 bis boss-2-4 alle gleich weit
        // entfernt (Distanz eins), und der Vorschlag war boss-2-1 — der, der
        // zufaellig zuerst in den Daten steht.
        let fehler = waehle_boss(Some("boss-2-5")).expect_err("boss-2-5 gibt es nicht");
        assert!(fehler.contains("boss-2-5"), "{fehler}");
        assert!(fehler.contains("boss-2-4"), "{fehler}");
        assert!(
            !fehler.contains("boss-2-1"),
            "vorgeschlagen wurde der falsche Nachbar:\n{fehler}"
        );
        assert!(fehler.contains("rota-sim data bosses"), "{fehler}");
    }

    #[test]
    fn ziffernabstand_entscheidet_nur_bei_gleicher_struktur() {
        assert_eq!(ziffernabstand("boss-2-5", "boss-2-4"), 1);
        assert_eq!(ziffernabstand("boss-2-5", "boss-2-1"), 4);
        assert_eq!(ziffernabstand("boss-2-5", "boss-2-5"), 0);
        // Mehrstellig, damit ein Zeichenvergleich hier nicht durchkaeme:
        // 9 gegen 10 ist ein Schritt, als Text waeren es zwei Aenderungen.
        assert_eq!(ziffernabstand("boss-1-9", "boss-1-10"), 1);
        // Ohne Ziffern ist er fuer alle gleich und entscheidet nichts.
        assert_eq!(ziffernabstand("abc", "xyz"), 0);
        // Verschieden viele Zahlen heisst "weit weg", nicht "gleich".
        assert_eq!(ziffernabstand("boss-2-5", "boss-2"), u64::MAX);
    }

    #[test]
    fn ohne_boss_kommt_der_standard() {
        let boss = waehle_boss(None).expect("ohne Kennung gibt es den Standard");
        assert_eq!(boss.id, BalanceConfig::default().boss.id);
    }

    #[test]
    fn bekannter_boss_kommt_durch() {
        let boss = waehle_boss(Some("boss-1-1")).expect("boss-1-1 gibt es");
        assert_eq!(boss.id, "boss-1-1");
    }

    #[test]
    fn kein_vorschlag_bei_voelliger_fremdheit() {
        // Ein geratener Vorschlag fuehrt weiter weg als gar keiner.
        assert_eq!(aehnlichster_boss("xxxxxxxxxxxxxxxxxxxx"), None);
    }

    #[test]
    fn distanz_zaehlt_die_einzelnen_aenderungen() {
        assert_eq!(distanz("", ""), 0);
        assert_eq!(distanz("boss-2-4", "boss-2-4"), 0);
        assert_eq!(distanz("boss-2-5", "boss-2-4"), 1);
        assert_eq!(distanz("kitten", "sitting"), 3);
        assert_eq!(distanz("", "abc"), 3);
    }

    #[test]
    fn unbekannter_befehl_nennt_die_moeglichen() {
        let fehler = fuehre_aus(&worte("gibtsnicht")).expect_err("den Befehl gibt es nicht");
        assert!(fehler.contains("gibtsnicht"), "{fehler}");
        assert!(fehler.contains("rota-sim --help"), "{fehler}");
    }

    #[test]
    fn die_instanzen_werden_ohne_doppelte_gezaehlt() {
        let anzahl = anzahl_instanzen();
        assert!(anzahl > 0, "es gibt keine Instanz");
        assert!(
            anzahl <= data::BOSSES.len(),
            "es gibt mehr Instanzen als Bosse: {anzahl}"
        );
    }
}
