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

use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{
    auswertungsweg, run, run_gestaffelt, run_vollstaendig, BalanceConfig, BalanceResult,
    Durchgangswahl, GestaffelterLauf, Staffelung,
};
use rota_sim::checksum::{mix_named_int, FNV_OFFSET};
use rota_sim::data::{self, Boss};
use rota_sim::drosselung::{ControlFileWatcher, ControlSettings, RuntimeControl};
use rota_sim::fortschritt::{
    Anzeigestrang, Arbeitsmenge, Ausgabe, Kampfzahl, Kostenbild, Laufbeschreibung, Weg, Zaehlwerk,
};
use rota_sim::gear::WornValues;
use rota_sim::replay::EmbeddedRecording;
use rota_sim::rotation_search::{all_candidates, greedy, paar_zuege, Candidate, SearchResult};
// Nachbereitung #897: die Klassifikation aus #981 statt einer eigenen Zaehlung.
// Eine Quelle fuer die Frage, was eine Spielinstanz ist — und die Bau-Bruecke
// aus #981 AC4 gilt damit auch fuer diese Ausgabe.
use rota_sim::scoring::{probe_bosses, ScoringSet};
use rota_sim::stichprobe::{noetige_versuche_ohne_fehler, Befund};
use rota_sim::{Fight, FightConfig};

// ---------------------------------------------------------------------------
// Der Kampfzaehler
// ---------------------------------------------------------------------------

/// Wie viele Kaempfe dieser Aufruf gerechnet hat — aus dem Zaehlwerk (#933).
///
/// # Wofuer er da ist
///
/// `rota-sim explain` verspricht, KEINEN Kampf zu starten (#969, AC2). Das ist
/// eine Aussage, die man nicht an der Laufzeit ablesen kann — ein schneller
/// Lauf ist kein Beweis, und ein langsamer Rechner macht aus einem sauberen
/// Trockenlauf einen verdaechtigen. Deshalb gibt jeder Befehl seinen Stand
/// aus: nach `explain` null, nach `run` mehr als null.
///
/// # Warum es keinen eigenen Zaehler mehr gibt
///
/// Bis #933 fuehrte dieses Programm einen. Er zaehlte, was es **anforderte**:
/// `iterations` je Lauf. Ohne Wuerfel rechnet ein Lauf aber genau einen Kampf
/// (#908) — der Zaehler meldete dann das Zwanzig- bis Tausendfache, und
/// `bench` teilte diese Zahl durch die Zeit des einen Kampfes. Neben der
/// Fortschrittsanzeige waere er die zweite Wahrheit, die #933 verbietet.
///
/// Gezaehlt wird jetzt im heissen Pfad von [`rota_sim::balance`], also an
/// JEDER Stelle, die dort rechnet — auch an einer, die hier jemand neu
/// hinzufuegt. Der Befehl muss dafuer nur ein [`Zaehlwerk`] anmelden, und
/// diese Funktion nimmt es als Argument: Ohne Zaehlwerk gibt es keine Zahl,
/// und eine vergessene Anmeldung baut nicht.
fn kaempfe_gerechnet(werk: &Zaehlwerk) -> u64 {
    werk.momentaufnahme().kaempfe
}

/// Die Messungen, die `greedy_opts` fährt, ohne sie in `measured` zu zaehlen:
/// die Startliste und die gefundene Liste.
const MESSUNGEN_AUSSERHALB: u64 = 2;

// ---------------------------------------------------------------------------
// Die Fortschrittsanzeige (#933)
// ---------------------------------------------------------------------------

/// Was `--progress` und `--jsonl` bestellt haben.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Anzeigewahl {
    /// Die Anzeige fuer Menschen, auf dem Fehlerkanal.
    progress: bool,
    /// Der JSON-Strom in diese Datei.
    jsonl: Option<String>,
}

/// Startet die Anzeige, wenn eine bestellt ist.
///
/// Der Bericht geht weiter nach stdout, die Anzeige nach stderr: Ein Skript,
/// das stdout liest, bekommt mit `--progress` denselben Text wie ohne.
fn starte_anzeige(
    werk: &Arc<Zaehlwerk>,
    wahl: &Anzeigewahl,
    beschreibung: Laufbeschreibung,
) -> Result<Option<Anzeigestrang>, String> {
    if !wahl.progress && wahl.jsonl.is_none() {
        return Ok(None);
    }
    let jsonl: Option<Box<dyn Write + Send>> = match &wahl.jsonl {
        Some(pfad) => {
            let datei = std::fs::File::create(pfad).map_err(|fehler| {
                fehler_mit_ausweg(
                    &format!("Die Datei '{pfad}' laesst sich nicht anlegen: {fehler}"),
                    None,
                    "Form und Beispiele:  rota-sim run --help",
                )
            })?;
            Some(Box::new(std::io::BufWriter::new(datei)))
        }
        None => None,
    };
    let zeile: Option<Box<dyn Write + Send>> = wahl
        .progress
        .then(|| Box::new(std::io::stderr()) as Box<dyn Write + Send>);
    Ok(Some(Anzeigestrang::starten(
        Arc::clone(werk),
        beschreibung,
        Ausgabe {
            zeile,
            terminal: wahl.progress && std::io::stderr().is_terminal(),
            jsonl,
        },
    )))
}

/// Haelt die Anzeige an. Ein Schreibfehler dort beendet nicht den Lauf — er
/// steht auf dem Fehlerkanal, und der Bericht folgt trotzdem.
fn beende_anzeige(anzeige: Option<Anzeigestrang>) {
    if let Some(anzeige) = anzeige {
        if let Err(fehler) = anzeige.beenden() {
            eprintln!("Fortschrittsausgabe abgebrochen: {fehler}");
        }
    }
}

/// Was eine Auswertung dieses Laufs kostet: der Weg aus der Bibliothek, nicht
/// nachgebaut.
fn kostenbild(config: &BalanceConfig) -> Kostenbild {
    let weg = auswertungsweg(config);
    Kostenbild {
        weg,
        kaempfe_je_kandidat: Kampfzahl::Fest(if weg == Weg::EinKampf {
            1
        } else {
            u64::from(config.iterations)
        }),
    }
}

/// Erkennt `--progress` und `--jsonl`, gemeinsam fuer `run` und `bench`.
fn zerlege_anzeige(
    argumente: &[String],
    i: &mut usize,
    name: &str,
    wahl: &mut Anzeigewahl,
) -> Result<bool, String> {
    match name {
        "--progress" => wahl.progress = true,
        "--jsonl" => wahl.jsonl = Some(hole_wert(argumente, i, name)?),
        _ => return Ok(false),
    }
    Ok(true)
}

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
        "verify" => befehl_verify(rest),
        "hosts" => Ok(platzhalter("hosts", HILFE_HOSTS, GRUND_HOSTS, rest)),
        "status" => Ok(platzhalter("status", HILFE_STATUS, GRUND_STATUS, rest)),
        "explain" => befehl_explain(rest),
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
  verify     der Selbsttest: habe ich richtig gemessen? Vier Pruefungen auf einmal
  hosts      die verfuegbaren Bau- und Rechen-Rechner auflisten (noch nicht gebaut)
  status     einen laufenden Lauf anzeigen (noch nicht gebaut)
  explain    zeigen, was ein Lauf TAETE, ohne ihn zu starten: Umfang und Kosten
  data       die einkompilierten Inhalte pruefen: Bosse, Zauber, Kandidatenzeilen

BEISPIELE
  # schnellster Einstieg: die beste Rotation gegen einen Boss suchen
  rota-sim run --boss boss-1-1 --iterations 20 --max-len 3

  # welche Boss-Kennungen es gibt
  rota-sim data bosses

  # die Hilfe eines einzelnen Befehls, mit seinen eigenen Beispielen
  rota-sim run --help

WO DAS PROGRAMM LAEUFT
  Der Bau laeuft auf einem entfernten Rechner:
    cargo remote -c -- build --release
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
  # schnellster Einstieg: ein Boss, klein gerechnet
  rota-sim run --boss boss-1-1 --iterations 20 --max-len 3

  # eine vorhandene Rotationsdatei messen statt zu suchen
  rota-sim run --boss boss-1-1 --apl sim/apl/step1-good.apl

  # dieselben Zahlen maschinenlesbar, fuer jq und Skripte
  rota-sim run --boss boss-1-1 --iterations 20 --max-len 3 --json

  # genauer: mehr Durchgaenge je Messung, ueber vier Straenge, laengere Liste
  rota-sim run --boss boss-1-1 --iterations 100 --threads 4 --max-len 4

  # mit Krit: die Siegwahrscheinlichkeit gestaffelt belegen statt fest zu zaehlen
  rota-sim run --boss boss-1-1 --search none --purity 500 --mode gestaffelt --threshold 0.9 --alpha 0.05 --budget 512

  Die Beispiele nennen ihre Zahlen ausdruecklich, statt die Voreinstellungen zu
  nehmen. Der Grund ist die Pruefstrecke: Sie fuehrt jedes Beispiel von hier aus,
  und zwar im Debug-Bau, wo der Kern rund achtzehnmal langsamer ist. Ohne Zahlen
  kostete allein dieser Abschnitt dort ueber dreissig Sekunden. Weglassen darf
  man sie natuerlich — dann gelten die Voreinstellungen, und der Lauf dauert
  laenger und misst genauer.

FORTSCHRITT
  Waehrend des Laufs zeigen, was er tut — Phase, Runde, Kandidaten, beide
  Durchsatzzahlen, Restzeit oder Plateau, Straenge, Drosselung:

    rota-sim run --boss boss-1-1 --iterations 100 --max-len 6 --progress

  Dieselben Zahlen viermal je Sekunde als JSON Lines, fuer Skripte und
  ein zweites Fenster mit tail -f:

    rota-sim run --boss boss-1-1 --iterations 100 --max-len 6 --jsonl lauf.jsonl

  Diese beiden stehen nicht unter BEISPIELE, weil die Pruefstrecke jedes
  Beispiel ausfuehrt und von ihm einen leeren Fehlerkanal und keine Datei
  verlangt. Die Anzeige schreibt auf den Fehlerkanal, der Bericht bleibt auf
  der Standardausgabe und aendert sich nicht. Die Felder des Stroms und was
  die Restzeit bedeutet:  docs/FORTSCHRITT.md

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
  --control-file <p> JSON-Steuerdatei fuer --search none oder --apl.
                     Alle Felder sind Pflicht: cpu_threads, cpu_percent, paused.
                     Beispielinhalt: {\"cpu_threads\":2,\"cpu_percent\":50,\"paused\":false}
                     Aenderungen werden alle 50 ms gelesen. Ein laufender Kampf
                     endet vor dem Parken. cpu_threads 0 oder paused true pausiert.
                     CPU-Prozent ist kooperativer Arbeits-/Pausentakt, keine cgroup-Quote.
  --seed <n>         Startwert des ersten Durchgangs.
  --purity <n>       PURITY der getragenen Ausruestung: Kritchance in Promille
                     (500 = halbe Chance, ab 1000 kritet jeder Treffer). Ohne
                     sie ist die Kritchance null, und nichts wird gewuerfelt.
  --mode <art>       fest (Voreinstellung) oder gestaffelt.
                     fest rechnet --iterations Kaempfe je Messung.
                     gestaffelt waehlt selbst: EINEN Kampf, wenn nirgends
                     gewuerfelt wird — dann ist der Ausgang exakt —, sonst eine
                     gestaffelte Stichprobe, die laeuft, bis die
                     Siegwahrscheinlichkeit sicher ueber oder unter --threshold
                     liegt. Sie misst eine vorhandene Rotation (--apl oder
                     --search none), in einem Strang, mit konstantem Speicher.
  --threshold <p>    gestaffelt: geforderte Siegwahrscheinlichkeit, echt
                     zwischen 0 und 1.
  --alpha <a>        gestaffelt: Irrtumswahrscheinlichkeit, ueber alle
                     Haltepunkte zusammen, echt zwischen 0 und 1.
  --budget <n>       gestaffelt: hoechstens so viele Kaempfe. Reicht es nicht,
                     lautet der Befund UNGEKLAERT, nicht eine Zahl.
                     Alle drei sind Pflicht; keiner hat eine Voreinstellung.
  --json             Ausgabe als JSON statt als Text. Dieselben Zahlen.
  --progress         zeigt den Fortschritt auf dem Fehlerkanal, im Terminal
                     viermal je Sekunde ueberschrieben, sonst alle zehn
                     Sekunden ein Block. Eine Restzeit steht nur dort, wo die
                     Menge feststeht; die Suche nennt Ober- und Untergrenze.
  --jsonl <datei>    schreibt den Fortschritt als JSON Lines in eine Datei.
                     Dieselben Zahlen wie --progress, aus derselben Quelle
                     wie 'Kaempfe gerechnet'.
  --help             dieser Text.

WAS AUSGEGEBEN WIRD
  Schaden je Kampf mit Streuung, Siege, Zeit bis zum Sieg, eingesteckter
  Schaden, Tode und die Wirkungen je Zauber. Bei einer Suche zusaetzlich die
  gefundene Rotation und was sie gekostet hat: die Messungen der Suche und die
  Auswertungen insgesamt, die zusaetzlich Start- und Fundliste enthalten.

  Mit --mode gestaffelt stattdessen: der gewaehlte Weg, der Befund (ERFUELLT,
  VERFEHLT oder UNGEKLAERT), wie viele Kaempfe er gekostet hat, die Siege und
  wie oft gewuerfelt wurde.";

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
  --fights <n>       wie viele Kaempfe gerechnet werden.
  --threads <n>      ueber wie viele Straenge.
  --boss <id>        gegen welchen Boss. Ohne ihn gegen den Standard.
  --json             Ausgabe als JSON statt als Text.
  --progress         den Fortschritt auf dem Fehlerkanal zeigen.
  --jsonl <datei>    den Fortschritt als JSON Lines in eine Datei schreiben.
  --help             dieser Text.

WAS DIE ZAHL BEDEUTET
  Kaempfe je Sekunde, gemessen an der Wanduhr — und JEDER davon gerechnet.
  Ohne Wuerfel rechnet ein Balance-Lauf sonst einen Kampf und vervielfacht
  ihn; bis #933 tat genau das auch dieser Befehl, und seine Zahl teilte die
  angeforderten Kaempfe durch die Zeit eines einzigen.

  Sie haengt am Bau: Im Debug-Bau ist der Kern rund achtzehnmal langsamer als
  im Release. Wer sie vergleicht, vergleicht deshalb nur Laeufe desselben Baus
  auf demselben Rechner — und sagt dazu, ob die Maschine ruhig war.";

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

  Beantwortet die Frage 'habe ich richtig gemessen?', ohne dass man den Aufbau
  des Simulators kennen muss. Vier Pruefungen in einem Aufruf: Paritaet gegen
  das Spiel, Stabilitaet der Trace-Hashes, Determinismus ueber Straenge und
  Bitgleichheit gegen einen zweiten Rechner.

  Ein bestandener Selbsttest ist KEINE Abnahme. Er prueft die Grundlage, nicht
  das Ergebnis.

BEISPIELE
  # der Selbsttest, so wie man ihn braucht
  rota-sim verify

  # nur den Fingerabdruck ausgeben — das fragt die Gegenstelle ab
  rota-sim verify --print-fingerprint

OPTIONEN
  --peer <ziel>          ein zweiter Rechner fuer die Bitgleichheit, als
                         BENUTZER@RECHNER — etwa 'root@10.0.0.155'. Er
                         braucht dasselbe Programm im Pfad. Ist er nicht
                         erreichbar, wird die Pruefung UEBERSPRUNGEN, nicht
                         als gescheitert gewertet.

                         **Dieser Aufruf steht bewusst NICHT unter
                         BEISPIELE.** Die Pruefstrecke fuehrt jedes Beispiel
                         von dort aus — und ein Test, der einen fremden
                         Rechner ueber das Netz anspricht, gehoert nicht in
                         eine Pruefstrecke. Gemessen kostete er dort achtzig
                         Sekunden und haette bei jedem Lauf eine SSH-Sitzung
                         auf den Bau-Server geoeffnet.
  --print-fingerprint    nur den Fingerabdruck dieses Baus, eine Zeile.
  --help                 dieser Text.

WAS ER NICHT LEISTET
  Er prueft die Aufzeichnungen mit der Pruefsumme des LETZTEN Takts. Eine
  Abweichung zeigt sich damit, aber nicht der Takt, in dem sie beginnt. Wer
  das braucht:

    cargo test --manifest-path sim/Cargo.toml --test parity_recording

  Und er prueft nur die Dateien, die eine abspielbare Runde beschreiben. Wie
  viele das von wie vielen sind, sagt er in seiner Ausgabe.";

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

  Den Fortschritt eines Laufs im Vordergrund zeigt heute schon
  'rota-sim run --progress', und 'rota-sim run --jsonl DATEI' schreibt ihn
  als Strom, den ein zweites Fenster mit tail -f liest.

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
fremden Rechnern laufen — siehe hosts. Den Fortschritt eines Laufs im
Vordergrund zeigen schon jetzt:  rota-sim run --progress  und  --jsonl DATEI

Was er ausserdem kann, steht in:  rota-sim status --help";

/// Die Hilfe von `explain`.
const HILFE_EXPLAIN: &str = "\
rota-sim explain

  Sagt, was ein Lauf TAETE, ohne ihn zu starten: wie viele Kandidatenzeilen zur
  Wahl stehen, wie viele Messungen und Kaempfe daraus werden, wie lange das
  dauert und wie viel Speicher es braucht. Bei Laeufen ueber Stunden ist das
  der Unterbefehl, der einen falsch eingestellten Lauf ueber Nacht verhindert.

BEISPIELE
  # was der Voreinstellungs-Lauf gegen einen Boss kosten wuerde
  rota-sim explain --boss boss-1-1

  # ein grosser Lauf, bevor man ihn ueber Nacht anwirft
  rota-sim explain --boss boss-1-1 --iterations 1000 --max-len 8

  # maschinenlesbar
  rota-sim explain --boss boss-1-1 --json

OPTIONEN
  --boss <id>        gegen welchen Boss gerechnet wuerde.
  --max-len <n>      wie viele Zeilen die gesuchte Rotation hoechstens haette.
  --iterations <n>   wie viele Kaempfe je Messung.
  --threads <n>      ueber wie viele Straenge.
  --rate <n>         Kaempfe je Sekunde dieser Maschine. Ohne die Angabe gilt
                     ein gemessener Erfahrungswert — siehe UNSICHERHEIT.
  --json             Ausgabe als JSON statt als Text.
  --help             dieser Text.

UNSICHERHEIT
  Zwei der ausgegebenen Zahlen sind exakt und zwei sind geschaetzt, und die
  Ausgabe sagt bei jeder, welche von beiden sie ist.

  EXAKT sind die Kandidatenzeilen und die Zahl der Messungen. Sie folgen aus
  der Bauart der Suche, nicht aus Messwerten.

  GESCHAETZT sind Dauer und Speicher. Die Dauer haengt an der Rate dieser
  Maschine und dieses Baus — im Debug-Bau ist der Kern rund achtzehnmal
  langsamer als im Release. Die eigene Rate bestimmt:  rota-sim bench

  UND EINE ZAHL IST EINE SPANNE, keine Zahl: Die Suche kann jederzeit
  abbrechen, wenn keine Zeile mehr traegt. Die Ausgabe nennt deshalb ein
  Minimum (die erste Runde) und ein Maximum (alle Runden).";

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
///
/// Nur `PartialEq`, nicht `Eq`: Schwelle und alpha sind Gleitkommazahlen.
#[derive(Debug, Clone, PartialEq)]
struct RunOptionen {
    boss_id: Option<String>,
    apl_pfad: Option<String>,
    control_file: Option<String>,
    suche: Suchart,
    max_len: usize,
    iterations: u32,
    /// Ob `--iterations` ausdruecklich genannt wurde — im gestaffelten Lauf
    /// gilt es nicht, und eine stillschweigend uebergangene Angabe waere eine
    /// Messung, die jemand anders bestellt hat, als sie lief.
    iterations_genannt: bool,
    threads: usize,
    seed: u32,
    /// PURITY der getragenen Ausruestung, in Promille Kritchance.
    purity: u32,
    modus: Modus,
    schwelle: Option<f64>,
    alpha: Option<f64>,
    budget: Option<u64>,
    json: bool,
    /// `--progress` und `--jsonl` (#933).
    anzeige: Anzeigewahl,
    hilfe: bool,
}

/// Wie `run` die Zahl der Durchgaenge bestimmt (#915).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modus {
    /// Eine feste Zahl, `--iterations`.
    Fest,
    /// Die Durchgangssteuerung: ein Kampf, wenn nirgends gewuerfelt wird,
    /// sonst die gestaffelte Stichprobe.
    Gestaffelt,
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
            control_file: None,
            suche: Suchart::Greedy,
            // Vier Zeilen sind die Laenge, mit der die Suchguete-Reihe misst.
            max_len: 4,
            // **Bewusst weniger als die tausend von `BalanceConfig::default`.**
            // Eine Suche misst nicht einmal, sondern hunderte Male; mit tausend
            // Durchgaengen je Messung waere der Aufruf aus der Hilfe keiner,
            // den man ausprobiert. Wer genauer will, setzt --iterations.
            //
            // # Warum hier keine kleinere Zahl steht, obwohl die Pruefstrecke
            //   davon profitierte
            //
            // Sie stand kurzzeitig auf zwanzig, begruendet damit, dass der
            // Standardfehler ohnehin null sei. Das stimmt — aber nur, solange
            // `WornValues::purity` null ist, und das ist ein VORGABEWERT
            // (`gear.rs`, "Kritchance"), kein Zustand des Simulators. Sobald
            // jemand Ausruestung setzbar macht, gaebe es Krit, Zufallsziehungen
            // und eine Streuung — und zwanzig Durchgaenge waeren zu wenig, ohne
            // dass irgendetwas rot wuerde. Der Lauf meldete dann einen zu
            // kleinen Standardfehler, keinen Fehler.
            //
            // Ein Waechter dagegen liesse sich heute nicht pruefen: Er koennte
            // erst anschlagen, wenn es die Aenderung gibt, die er abfangen soll.
            // Deshalb steht hier der Wert, der ohne Begruendung auskommt — und
            // die Beispiele in der Hilfe nennen ihre Zahlen ausdruecklich,
            // damit die Pruefstrecke trotzdem schnell bleibt.
            iterations: 50,
            iterations_genannt: false,
            threads: 1,
            seed: 0,
            // Wie `BalanceConfig::default`: ohne Ausruestung, also ohne Krit.
            purity: 0,
            modus: Modus::Fest,
            schwelle: None,
            alpha: None,
            budget: None,
            json: false,
            anzeige: Anzeigewahl::default(),
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
    // Ohne `--purity` ist das `WornValues::none()` — dieselbe Ausruestung wie
    // in `BalanceConfig::default`, also aendert sich an keinem Lauf ohne die
    // Angabe etwas.
    let mut worn = WornValues::none();
    worn.purity = optionen.purity;
    let config = BalanceConfig {
        iterations: optionen.iterations,
        base_seed: optionen.seed,
        threads: optionen.threads,
        boss,
        worn,
        ..BalanceConfig::default()
    };

    // Das Zaehlwerk dieses Aufrufs: angemeldet, bevor der erste Kampf laeuft,
    // mit einem Platz je Strang.
    let werk = Zaehlwerk::neu(config.threads);
    let _anmeldung = werk.anmelden();

    if optionen.modus == Modus::Gestaffelt {
        return lauf_gestaffelt(&optionen, &config, boss, &werk);
    }

    // Eine eigene Datei heisst immer "messen", nie "suchen" — sonst wuerde der
    // Aufruf die Datei stillschweigend ignorieren.
    let sucht = optionen.suche == Suchart::Greedy && optionen.apl_pfad.is_none();
    if sucht && optionen.control_file.is_some() {
        return Err(fehler_mit_ausweg(
            "'--control-file' ist noch nicht an die adaptive Suche angeschlossen.",
            Some("Eine vorhandene Rotation steuern: --search none oder --apl DATEI".to_owned()),
            "Steuerdatei und Grenzen: rota-sim run --help",
        ));
    }

    let start = Instant::now();
    if sucht {
        let pool = all_candidates();
        // Die Menge setzt die Suche selbst, Runde fuer Runde — vorher steht
        // sie nicht fest.
        let anzeige = starte_anzeige(
            &werk,
            &optionen.anzeige,
            Laufbeschreibung::neu("greedy", Some(boss.id), config.threads, kostenbild(&config)),
        )?;
        let ergebnis = greedy(&config, &pool, optionen.max_len);
        beende_anzeige(anzeige);
        let dauer = start.elapsed().as_secs_f64();
        // Beide Zahlen aus EINER Momentaufnahme des Zaehlwerks.
        let stand = werk.momentaufnahme();
        let zaehlung = Zaehlung {
            auswertungen: stand.kandidaten_gesamt(),
            kaempfe: stand.kaempfe,
        };
        if optionen.json {
            println!("{}", such_json(&ergebnis, boss, dauer, zaehlung));
        } else {
            berichte_suche(&ergebnis, boss, &config, dauer, zaehlung);
        }
    } else {
        let (name, apl) = lade_apl(optionen.apl_pfad.as_deref())?;
        let mut runtime = if let Some(path) = &optionen.control_file {
            let max_workers = config.threads.max(1);
            let control = Arc::new(
                RuntimeControl::new(
                    max_workers,
                    ControlSettings {
                        cpu_threads: max_workers,
                        cpu_percent: 100,
                        paused: false,
                    },
                )
                .map_err(|e| e.to_string())?,
            );
            let watcher = ControlFileWatcher::start_with_error_handler(
                path,
                Arc::clone(&control),
                Duration::from_millis(50),
                |error| {
                    eprintln!(
                        "Control-file update rejected: {error}; previous valid settings retained."
                    );
                },
            )
            .map_err(|e| format!("Steuerdatei {path}: {e}; Format: rota-sim run --help"))?;
            eprintln!(
                "Runtime control active: {path}; polling every 50 ms between complete fights."
            );
            Some((control, watcher))
        } else {
            None
        };
        werk.setze_menge(Arbeitsmenge::Bekannt { kandidaten: 1 });
        let mut beschreibung =
            Laufbeschreibung::neu("fest", Some(boss.id), config.threads, kostenbild(&config));
        beschreibung.drosselung = runtime.as_ref().map(|(control, _)| Arc::clone(control));
        let anzeige = starte_anzeige(&werk, &optionen.anzeige, beschreibung)?;
        let ergebnis = if let Some((control, _)) = &runtime {
            rota_sim::balance::run_controlled(&config, &apl, control)
        } else {
            run(&config, &apl)
        };
        beende_anzeige(anzeige);
        if let Some((control, watcher)) = &mut runtime {
            watcher.stop().map_err(|e| e.to_string())?;
            let state = control.snapshot();
            let changes = watcher.status();
            eprintln!(
                "Runtime control: {} jobs completed, peak {} workers, {} accepted / {} rejected file updates; longest job {:.3} ms.",
                state.completed_jobs, state.peak_active_workers,
                changes.applied_updates, changes.rejected_updates,
                state.max_job_time.as_secs_f64() * 1000.0,
            );
            if let Some(error) = changes.last_error {
                eprintln!("Last control-file error (previous valid settings retained): {error}");
            }
        }
        let dauer = start.elapsed().as_secs_f64();
        if optionen.json {
            println!(
                "{{\"boss\":{:?},\"elapsedMs\":{:?},\"result\":{}}}",
                boss.id,
                dauer * 1000.0,
                ergebnis.to_json()
            );
        } else {
            berichte_lauf(&name, &ergebnis, boss, dauer, kaempfe_gerechnet(&werk));
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
            "--control-file" => optionen.control_file = Some(hole_wert(argumente, &mut i, name)?),
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
            "--iterations" => {
                optionen.iterations = hole_zahl(argumente, &mut i, name)?;
                optionen.iterations_genannt = true;
            }
            "--threads" => optionen.threads = hole_zahl(argumente, &mut i, name)?,
            "--seed" => optionen.seed = hole_zahl(argumente, &mut i, name)?,
            "--purity" => optionen.purity = hole_zahl(argumente, &mut i, name)?,
            "--mode" => {
                let wert = hole_wert(argumente, &mut i, name)?;
                optionen.modus = match wert.as_str() {
                    "fest" => Modus::Fest,
                    "gestaffelt" => Modus::Gestaffelt,
                    anderes => {
                        return Err(fehler_mit_ausweg(
                            &format!("Den Modus '{anderes}' gibt es nicht."),
                            Some("Moeglich sind: 'fest', 'gestaffelt'".to_owned()),
                            "Mit Beispielen:  rota-sim run --help",
                        ))
                    }
                };
            }
            "--threshold" => optionen.schwelle = Some(hole_anteil(argumente, &mut i, name)?),
            "--alpha" => optionen.alpha = Some(hole_anteil(argumente, &mut i, name)?),
            "--budget" => optionen.budget = Some(hole_zahl(argumente, &mut i, name)?),
            "--progress" | "--jsonl" => {
                zerlege_anzeige(argumente, &mut i, name, &mut optionen.anzeige)?;
            }
            anderes => return Err(unbekannte_angabe("run", anderes, &ANGABEN_RUN)),
        }
        i += 1;
    }
    pruefe_modus(&optionen)?;
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

/// Prueft, ob die Angaben zum gewaehlten Modus passen (#915).
///
/// **Was nur der andere Modus liest, wird abgelehnt statt uebergangen.** Ein
/// `--iterations` im gestaffelten Lauf oder ein `--budget` im festen haette
/// sonst keine Wirkung, und die Ausgabe saehe aus wie die bestellte Messung.
///
/// `--max-len` gehoert nicht dazu: Es gilt nur fuer die Suche und wird bei
/// einer vorhandenen Rotation schon vor #915 in jedem Lauf nicht gelesen.
fn pruefe_modus(optionen: &RunOptionen) -> Result<(), String> {
    let staffel_angaben = [
        ("--threshold", optionen.schwelle.is_some()),
        ("--alpha", optionen.alpha.is_some()),
        ("--budget", optionen.budget.is_some()),
    ];
    let mit_beispielen = "Mit Beispielen:  rota-sim run --help";
    match optionen.modus {
        Modus::Fest => {
            if let Some((name, _)) = staffel_angaben.iter().find(|(_, genannt)| *genannt) {
                return Err(fehler_mit_ausweg(
                    &format!("'{name}' gilt nur mit '--mode gestaffelt'."),
                    Some(
                        "Ein fester Lauf rechnet --iterations Kaempfe und prueft keine Schwelle."
                            .to_owned(),
                    ),
                    mit_beispielen,
                ));
            }
            Ok(())
        }
        Modus::Gestaffelt => {
            let fehlend: Vec<&str> = staffel_angaben
                .iter()
                .filter(|(_, genannt)| !*genannt)
                .map(|(name, _)| *name)
                .collect();
            if !fehlend.is_empty() {
                return Err(fehler_mit_ausweg(
                    &format!("'--mode gestaffelt' braucht {}.", fehlend.join(", ")),
                    Some(
                        "Schwelle, alpha und Budget haben mit Absicht keine Voreinstellung."
                            .to_owned(),
                    ),
                    mit_beispielen,
                ));
            }
            if optionen.iterations_genannt {
                return Err(fehler_mit_ausweg(
                    "'--iterations' gilt nicht mit '--mode gestaffelt'.",
                    Some(
                        "Die Stichprobe bestimmt ihre Zahl selbst; die Obergrenze ist --budget."
                            .to_owned(),
                    ),
                    mit_beispielen,
                ));
            }
            if optionen.threads != 1 {
                return Err(fehler_mit_ausweg(
                    "'--threads' gilt nicht mit '--mode gestaffelt'.",
                    Some("Die Stichprobe rechnet in einem Strang.".to_owned()),
                    mit_beispielen,
                ));
            }
            if optionen.control_file.is_some() {
                return Err(fehler_mit_ausweg(
                    "'--control-file' ist an '--mode gestaffelt' nicht angeschlossen.",
                    None,
                    mit_beispielen,
                ));
            }
            if optionen.suche == Suchart::Greedy && optionen.apl_pfad.is_none() {
                return Err(fehler_mit_ausweg(
                    "'--mode gestaffelt' misst eine vorhandene Rotation, es sucht keine.",
                    Some("Eine Rotation waehlen: --apl DATEI oder --search none".to_owned()),
                    mit_beispielen,
                ));
            }
            staffelung_aus(optionen)
                .pruefe()
                .map_err(|fehler| fehler_mit_ausweg(&fehler.to_string(), None, mit_beispielen))
        }
    }
}

/// Die Staffelung aus den Angaben.
///
/// Fehlt eine, steht dort ein Wert, den [`Staffelung::pruefe`] ablehnt — ein
/// fehlender Wert wird also nie still zu einer Voreinstellung.
fn staffelung_aus(optionen: &RunOptionen) -> Staffelung {
    Staffelung {
        schwelle: optionen.schwelle.unwrap_or(f64::NAN),
        alpha: optionen.alpha.unwrap_or(f64::NAN),
        budget: optionen.budget.unwrap_or(0),
    }
}

/// `run --mode gestaffelt`: ein Lauf mit Durchgangssteuerung (#915).
fn lauf_gestaffelt(
    optionen: &RunOptionen,
    config: &BalanceConfig,
    boss: &Boss,
    werk: &Arc<Zaehlwerk>,
) -> Result<ExitCode, String> {
    let (name, apl) = lade_apl(optionen.apl_pfad.as_deref())?;
    let staffelung = staffelung_aus(optionen);
    // Der Weg kommt aus derselben Wahl, die der Lauf gleich trifft: Ohne Wuerfel
    // ist es EIN Kampf, sonst hoechstens das Budget.
    let kosten = match Durchgangswahl::fuer(config) {
        Durchgangswahl::EinDurchgang => Kostenbild {
            weg: Weg::EinKampf,
            kaempfe_je_kandidat: Kampfzahl::Fest(1),
        },
        Durchgangswahl::Gestaffelt => Kostenbild {
            weg: Weg::Gestaffelt,
            kaempfe_je_kandidat: Kampfzahl::Hoechstens(staffelung.budget),
        },
    };
    werk.setze_menge(Arbeitsmenge::Bekannt { kandidaten: 1 });
    let anzeige = starte_anzeige(
        werk,
        &optionen.anzeige,
        Laufbeschreibung::neu("gestaffelt", Some(boss.id), 1, kosten),
    )?;
    let start = Instant::now();
    let lauf = run_gestaffelt(config, &apl, staffelung);
    beende_anzeige(anzeige);
    let lauf = lauf.map_err(|fehler| {
        fehler_mit_ausweg(
            &fehler.to_string(),
            None,
            "Mit Beispielen:  rota-sim run --help",
        )
    })?;
    let dauer = start.elapsed().as_secs_f64();
    if optionen.json {
        println!("{}", gestaffelt_json(&lauf, boss, &name, config, dauer));
    } else {
        berichte_gestaffelt(&lauf, boss, &name, config, dauer, kaempfe_gerechnet(werk));
    }
    Ok(ExitCode::SUCCESS)
}

/// Die Versuchszahl eines Befunds und sein Name, wie ihn `--json` ausgibt.
fn befund_teile(befund: Befund) -> (&'static str, u64) {
    match befund {
        Befund::Erfuellt { versuche } => ("Erfuellt", versuche),
        Befund::Verfehlt { versuche } => ("Verfehlt", versuche),
        Befund::Ungeklaert { versuche, .. } => ("Ungeklaert", versuche),
    }
}

/// Der Befund in Worten.
///
/// **Beim Weg „ein Durchgang" steht dort „exakt", nicht „sicher".** Der eine
/// Kampf ist die ganze Verteilung; es gibt keinen Irrtum, den alpha begrenzen
/// muesste. Beim gestaffelten Weg dagegen gilt die Aussage nur bis auf alpha.
fn befund_satz(lauf: &GestaffelterLauf) -> String {
    let schwelle = lauf.staffelung.schwelle;
    let alpha = lauf.staffelung.alpha;
    match (lauf.wahl, lauf.befund) {
        (Durchgangswahl::EinDurchgang, Befund::Erfuellt { .. }) => {
            format!("ERFUELLT — gewonnen; die Siegwahrscheinlichkeit ist exakt 1, ueber {schwelle}")
        }
        (Durchgangswahl::EinDurchgang, Befund::Verfehlt { .. }) => {
            format!("VERFEHLT — verloren; die Siegwahrscheinlichkeit ist exakt 0, unter {schwelle}")
        }
        (_, Befund::Erfuellt { .. }) => format!(
            "ERFUELLT — die Siegwahrscheinlichkeit liegt sicher ueber {schwelle} \
             (Irrtum hoechstens {alpha})"
        ),
        (_, Befund::Verfehlt { .. }) => format!(
            "VERFEHLT — die Siegwahrscheinlichkeit liegt sicher unter {schwelle} \
             (Irrtum hoechstens {alpha})"
        ),
        (_, Befund::Ungeklaert { versuche, fehler }) => format!(
            "UNGEKLAERT — das Budget ist aufgebraucht, keine Seite ist sicher \
             ({fehler} verloren in {versuche} Kaempfen)"
        ),
    }
}

fn berichte_gestaffelt(
    lauf: &GestaffelterLauf,
    boss: &Boss,
    name: &str,
    config: &BalanceConfig,
    dauer: f64,
    kaempfe: u64,
) {
    let vorgabe = lauf.staffelung;
    println!("{}", "=".repeat(72));
    println!(
        "ROTA — Durchgangssteuerung gegen {} ({})",
        boss.name, boss.id
    );
    println!(
        "Schwelle {}, alpha {}, Budget {}, Startwert ab {}, PURITY {}, Liste {name}",
        vorgabe.schwelle, vorgabe.alpha, vorgabe.budget, config.base_seed, config.worn.purity
    );
    println!("{}", "=".repeat(72));
    println!();
    let weg = match lauf.wahl {
        Durchgangswahl::EinDurchgang => {
            "ein Durchgang — nirgends wird gewuerfelt, der Ausgang ist exakt"
        }
        Durchgangswahl::Gestaffelt => "gestaffelt — es wird gewuerfelt",
    };
    println!("Weg                  {weg}");
    println!("Befund               {}", befund_satz(lauf));
    println!(
        "Kaempfe              {} von hoechstens {}",
        lauf.kaempfe, vorgabe.budget
    );
    println!("Siege                {} von {}", lauf.siege, lauf.kaempfe);
    println!(
        "Schaden je Kampf     {:.1}",
        lauf.schaden as f64 / lauf.kaempfe as f64
    );
    println!("Gewuerfelt           {}-mal", lauf.wuerfe);
    if lauf.wahl == Durchgangswahl::Gestaffelt {
        println!(
            "Zum Vergleich        ungestaffelt genuegten ohne einen Fehler {} Kaempfe",
            noetige_versuche_ohne_fehler(vorgabe.schwelle, vorgabe.alpha)
        );
    }
    println!("Speicher             konstant: ein Kampf und vier Zaehler, kein Einzelergebnis");
    println!();
    println!("Gerechnet in {dauer:.2} Sekunden, in einem Strang.");
    println!("Kaempfe gerechnet: {kaempfe}");
}

fn gestaffelt_json(
    lauf: &GestaffelterLauf,
    boss: &Boss,
    name: &str,
    config: &BalanceConfig,
    dauer: f64,
) -> String {
    let (urteil, versuche) = befund_teile(lauf.befund);
    let wahl = format!("{:?}", lauf.wahl);
    format!(
        "{{\"boss\":{:?},\"elapsedMs\":{:?},\"apl\":{:?},\"purity\":{},\"baseSeed\":{},\
         \"selection\":{:?},\"threshold\":{:?},\"alpha\":{:?},\"budget\":{},\
         \"verdict\":{:?},\"trials\":{},\"fights\":{},\"wins\":{},\"losses\":{},\
         \"draws\":{},\"damage\":{}}}",
        boss.id,
        dauer * 1000.0,
        name,
        config.worn.purity,
        config.base_seed,
        wahl,
        lauf.staffelung.schwelle,
        lauf.staffelung.alpha,
        lauf.staffelung.budget,
        urteil,
        versuche,
        lauf.kaempfe,
        lauf.siege,
        lauf.kaempfe - lauf.siege,
        lauf.wuerfe,
        lauf.schaden
    )
}

/// Die Angaben, die `run` kennt — Grundlage fuer den Vorschlag bei Tippfehlern.
const ANGABEN_RUN: [&str; 18] = [
    "--progress",
    "--jsonl",
    "--boss",
    "--apl",
    "--control-file",
    "--search",
    "--max-len",
    "--iterations",
    "--threads",
    "--seed",
    "--purity",
    "--mode",
    "--threshold",
    "--alpha",
    "--budget",
    "--json",
    "--help",
    "-h",
];

/// Die Angaben, die `bench` kennt.
const ANGABEN_BENCH: [&str; 8] = [
    "--boss",
    "--fights",
    "--threads",
    "--json",
    "--progress",
    "--jsonl",
    "--help",
    "-h",
];

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

fn berichte_lauf(name: &str, ergebnis: &BalanceResult, boss: &Boss, dauer: f64, kaempfe: u64) {
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
    println!("Kaempfe gerechnet: {kaempfe}");
}

/// Was das Zaehlwerk nach einer Suche meldet — zwei Zahlen aus einer
/// Momentaufnahme.
#[derive(Debug, Clone, Copy)]
struct Zaehlung {
    /// Jede abgeschlossene Auswertung, auch Start- und Fundliste.
    auswertungen: u64,
    /// Gerechnete Kaempfe.
    kaempfe: u64,
}

/// Der Bericht einer Suche.
///
/// # Warum zwei Zahlen fuer Messungen dastehen (#933)
///
/// „Messungen" ist [`SearchResult::measured`]: die Zuege, die die Suche
/// gegeneinander gehalten hat. „Auswertungen" zaehlt das Zaehlwerk: jede
/// abgeschlossene Auswertung, dazu die Startliste vor der ersten Runde und die
/// gefundene Liste nach der letzten (`MESSUNGEN_AUSSERHALB`). Das sind zwei
/// Groessen, keine zweite Zaehlung derselben Sache; die Anzeige und der Strom
/// zeigen die zweite. Stuende nur die erste da, laese ein Bericht „1615" und die
/// Anzeige daneben „1617", und niemand wuesste, welche stimmt.
fn berichte_suche(
    ergebnis: &SearchResult,
    boss: &Boss,
    config: &BalanceConfig,
    dauer: f64,
    zaehlung: Zaehlung,
) {
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
    println!(
        "Messungen            {}  (Zuege der Suche, ohne Start- und Fundliste)",
        ergebnis.measured
    );
    println!(
        "Auswertungen         {}  (alle, mit Start- und Fundliste — so zaehlen --progress und --jsonl)",
        zaehlung.auswertungen
    );
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
    println!("Kaempfe gerechnet: {}", zaehlung.kaempfe);
}

fn such_json(ergebnis: &SearchResult, boss: &Boss, dauer: f64, zaehlung: Zaehlung) -> String {
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
         \"poolSize\":{},\"measured\":{},\"evaluations\":{},\"fightsRun\":{},\
         \"stop\":{:?},\"rotation\":[{}]}}",
        boss.id,
        dauer * 1000.0,
        ergebnis.best.mean,
        ergebnis.best.stderr,
        ergebnis.pool_size,
        ergebnis.measured,
        zaehlung.auswertungen,
        zaehlung.kaempfe,
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
    /// `--progress` und `--jsonl` (#933).
    anzeige: Anzeigewahl,
    hilfe: bool,
}

impl Default for BenchOptionen {
    fn default() -> Self {
        Self {
            fights: 2000,
            threads: 1,
            boss_id: None,
            json: false,
            anzeige: Anzeigewahl::default(),
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

    let werk = Zaehlwerk::neu(config.threads);
    let _anmeldung = werk.anmelden();
    werk.setze_menge(Arbeitsmenge::Bekannt { kandidaten: 1 });
    let anzeige = starte_anzeige(
        &werk,
        &optionen.anzeige,
        Laufbeschreibung::neu(
            "bench",
            Some(boss.id),
            config.threads,
            Kostenbild {
                weg: Weg::Stichprobe,
                kaempfe_je_kandidat: Kampfzahl::Fest(u64::from(optionen.fights)),
            },
        ),
    )?;

    // **Jeder Durchgang ein gerechneter Kampf.** Ohne Wuerfel rechnete `run`
    // EINEN Kampf und vervielfachte ihn (#908); bis #933 stand hier genau das,
    // und die Rate darunter teilte die angeforderte Zahl durch die Zeit dieses
    // einen. Eine Durchsatzmessung muss rechnen, was sie meldet.
    let start = Instant::now();
    let ergebnis = run_vollstaendig(&config, &apl);
    let sekunden = start.elapsed().as_secs_f64();
    beende_anzeige(anzeige);
    // Zaehler und Nenner aus derselben Quelle wie die Kampfzeile darunter.
    let kaempfe = kaempfe_gerechnet(&werk);
    // Nie durch null teilen: Ein Lauf, der unter der Aufloesung der Uhr bleibt,
    // hat keinen messbaren Durchsatz — und eine unendliche Zahl waere die
    // schlechteste aller Antworten.
    let je_sekunde = if sekunden > 0.0 {
        kaempfe as f64 / sekunden
    } else {
        f64::NAN
    };

    if optionen.json {
        println!(
            "{{\"boss\":{:?},\"fights\":{},\"threads\":{},\"elapsedMs\":{:?},\
             \"fightsPerSecond\":{:?},\"damageMean\":{:?}}}",
            boss.id,
            kaempfe,
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
        println!("Kaempfe              {kaempfe}");
        println!("Straenge             {}", optionen.threads);
        println!("Gerechnet in         {sekunden:.3} Sekunden");
        println!("Kaempfe je Sekunde   {je_sekunde:.0}");
        println!();
        println!(
            "Zur Kontrolle: Schaden je Kampf {:.1} — weicht der zwischen zwei",
            ergebnis.damage.mean
        );
        println!("Laeufen ab, hat der Lauf nicht dasselbe gerechnet.");
        println!();
        println!("Kaempfe gerechnet: {kaempfe}");
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
            "--progress" | "--jsonl" => {
                zerlege_anzeige(argumente, &mut i, name, &mut optionen.anzeige)?;
            }
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
// verify — der Selbsttest
// ---------------------------------------------------------------------------

/// Wie eine Einzelpruefung ausgegangen ist.
///
/// **Drei Zustaende, nicht zwei.** Eine Pruefung, die nicht laufen konnte, ist
/// weder bestanden noch durchgefallen — und sie als bestanden zu zaehlen waere
/// genau der Fehler, den dieser Befehl aufdecken soll (#970, AC5).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pruefergebnis {
    Bestanden(String),
    Gescheitert(String),
    Uebersprungen(String),
}

impl Pruefergebnis {
    const fn marke(&self) -> &'static str {
        match self {
            Self::Bestanden(_) => "BESTANDEN",
            Self::Gescheitert(_) => "GESCHEITERT",
            Self::Uebersprungen(_) => "UEBERSPRUNGEN",
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Bestanden(t) | Self::Gescheitert(t) | Self::Uebersprungen(t) => t,
        }
    }

    const fn ist_gescheitert(&self) -> bool {
        matches!(self, Self::Gescheitert(_))
    }
}

/// Alle Aufzeichnungen einmal abgespielt, mit ihrer Pruefsumme.
///
/// # Warum das gesammelt wird und nicht je Pruefung neu
///
/// Drei der vier Pruefungen brauchen dieselben Werte: die Paritaet vergleicht
/// sie mit der Datei, der Fingerabdruck mischt sie zu einer Zahl, und die
/// Trace-Hashes brauchen sie als ersten von zwei Laeufen. Sie einzeln zu
/// rechnen hiess, dieselben 65 Runden VIERMAL abzuspielen.
///
/// Gemessen im Debug-Bau: 28,5 Sekunden vorher. Da die Pruefstrecke jedes
/// Beispiel aus der Hilfe ausfuehrt und `rota-sim verify` eines davon ist,
/// zahlt das jeder Testlauf.
fn spiele_alle_ab() -> Vec<(&'static EmbeddedRecording, Option<u32>)> {
    data::RECORDINGS
        .iter()
        .map(|eintrag| (eintrag, spiele_ab(eintrag)))
        .collect()
}

/// Spielt eine eingebettete Aufzeichnung ab und liefert die Pruefsumme des
/// letzten Takts.
fn spiele_ab(eintrag: &EmbeddedRecording) -> Option<u32> {
    let boss = match eintrag.boss_id {
        Some(id) => data::boss_by_id(id)?,
        None => FightConfig::default().boss,
    };
    let recording = eintrag.to_recording();
    let mut fight = Fight::new(FightConfig {
        seed: eintrag.seed,
        boss,
        ..FightConfig::default()
    });
    fight.replay(&recording, true).checksums.last().copied()
}

/// Aufzeichnungen, die sich aus der DATEI ALLEIN nicht abspielen lassen.
///
/// # Der Befund dahinter
///
/// Die Dateien unter `tests/parity/recordings/` sind nicht selbsttragend. Ein
/// Teil ihrer Einstellungen steht im zugehoerigen Test, nicht in der Datei —
/// `parity_decay_ward.rs` setzt `boss_by_id("test-onslaught")` fest, obwohl
/// `decay-ward.json` keinen Boss nennt. Gemessen am 2026-08-18: 32 der
/// `parity_*`-Tests setzen einen Boss auf diese Weise.
///
/// Wer sie ohne dieses Wissen abspielt, bekommt eine Abweichung, die wie ein
/// Fehler im Simulator aussieht und keiner ist.
///
/// # Warum eine Liste und keine Zahl
///
/// Jeder Eintrag hat einen EIGENEN Grund. Eine Zahl ("acht weichen ab") waere
/// erfuellt, sobald irgendwelche acht abweichen — auch andere.
///
/// # Warum sie nicht still altert
///
/// [`pruefe_paritaet`] meldet auch den umgekehrten Fall: Steht eine
/// Aufzeichnung hier und stimmt sie ploetzlich, ist die Ausnahme ueberholt und
/// gehoert heraus. Ohne das waere die Liste nach dem ersten Ausbau eine
/// Sammlung von Behauptungen, die niemand mehr prueft.
const BEKANNTE_AUSNAHMEN: [(&str, &str); 12] = [
    (
        "decay-ward",
        "der Boss steht im Test (test-onslaught), nicht in der Datei",
    ),
    (
        "pin-befreiung",
        "der Boss steht im Test (test-caster), nicht in der Datei",
    ),
    ("krit-umwandlung", "braucht getragene Ausruestung (worn)"),
    ("synergy-haste", "braucht getragene Ausruestung (worn)"),
    ("tick-rundung", "braucht getragene Ausruestung (worn)"),
    ("worn-values", "braucht getragene Ausruestung (worn)"),
    ("level-30", "braucht eine Spielerstufe (playerLevel)"),
    (
        "retort-erstattung",
        "braucht eine Spielerstufe (playerLevel)",
    ),
    (
        "kontrolle-preis-talent",
        "braucht eine Talent-Verteilung (talentPermille)",
    ),
    (
        "schluessel-nachlage",
        "braucht ein Schluesseltalent (keystones)",
    ),
    (
        "schluessel-nachlage-ablauf",
        "braucht ein Schluesseltalent (keystones)",
    ),
    (
        "schluessel-nachlauf",
        "braucht ein Schluesseltalent (keystones)",
    ),
];

/// Der Grund, aus dem eine Aufzeichnung bekanntermassen abweicht.
fn ausnahmegrund(name: &str) -> Option<&'static str> {
    BEKANNTE_AUSNAHMEN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, grund)| *grund)
}

/// **Paritaet**: Der Simulator muss dieselben Zahlen liefern wie das Spiel.
///
/// Geprueft wird jede eingebettete Aufzeichnung gegen die Pruefsumme ihres
/// letzten Takts, wie das Spiel sie geschrieben hat.
fn pruefe_paritaet(gespielt: &[(&EmbeddedRecording, Option<u32>)]) -> Pruefergebnis {
    let mut abweichungen = Vec::new();
    let mut bekannte = 0usize;
    let mut ueberholte = Vec::new();
    let mut geprueft = 0usize;

    for (eintrag, ergebnis) in gespielt {
        let Some(gemessen) = *ergebnis else {
            abweichungen.push(format!("{}: nicht abspielbar", eintrag.name));
            continue;
        };
        geprueft += 1;
        let stimmt = gemessen == eintrag.final_checksum;
        match (stimmt, ausnahmegrund(eintrag.name)) {
            (true, None) => {}
            // **Der umgekehrte Fall, und er ist der wichtigere**: Eine
            // Ausnahme, die nicht mehr noetig ist, verdeckt ab jetzt eine
            // echte Abweichung.
            (true, Some(grund)) => ueberholte.push(format!("{} ({grund})", eintrag.name)),
            (false, Some(_)) => bekannte += 1,
            (false, None) => abweichungen.push(format!(
                "{}: erwartet {}, gerechnet {}",
                eintrag.name, eintrag.final_checksum, gemessen
            )),
        }
    }

    let bekannt_satz = if bekannte == 0 {
        String::new()
    } else {
        format!(
            "\n      {bekannte} weitere weichen bekanntermassen ab, weil ihre Einstellungen \
             im Test stehen und nicht in der Datei — siehe BEKANNTE_AUSNAHMEN"
        )
    };

    if !abweichungen.is_empty() {
        return Pruefergebnis::Gescheitert(format!(
            "{} von {geprueft} Runden weichen UNERWARTET ab:\n      {}{bekannt_satz}",
            abweichungen.len(),
            abweichungen.join("\n      ")
        ));
    }
    if !ueberholte.is_empty() {
        return Pruefergebnis::Gescheitert(format!(
            "{} Ausnahme(n) sind ueberholt — sie stimmen jetzt und gehoeren aus der Liste:\
             \n      {}\n      Solange sie drinstehen, verdecken sie eine echte Abweichung.",
            ueberholte.len(),
            ueberholte.join("\n      ")
        ));
    }
    Pruefergebnis::Bestanden(format!(
        "{} von {geprueft} aufgezeichneten Runden stimmen ueberein{bekannt_satz}",
        geprueft - bekannte
    ))
}

/// **Trace-Hashes**: Derselbe Lauf muss dieselbe Pruefsummen-Kette ergeben.
///
/// Das ist NICHT dasselbe wie die Paritaet. Dort wird gegen das Spiel geprueft,
/// hier gegen den Simulator selbst: Zweimal abgespielt muss Zeichen fuer
/// Zeichen dasselbe herauskommen. Ein nichtdeterministischer Kern faellt hier
/// auf, auch wenn er zufaellig die richtige Endzahl trifft.
fn pruefe_trace_hashes() -> Pruefergebnis {
    let mut abweichungen = Vec::new();
    let mut takte = 0usize;
    for eintrag in &data::RECORDINGS {
        let boss = match eintrag.boss_id {
            Some(id) => match data::boss_by_id(id) {
                Some(b) => b,
                None => continue,
            },
            None => FightConfig::default().boss,
        };
        let recording = eintrag.to_recording();
        let lauf = |()| {
            let mut fight = Fight::new(FightConfig {
                seed: eintrag.seed,
                boss,
                ..FightConfig::default()
            });
            fight.replay(&recording, true)
        };
        let erst = lauf(());
        let zweit = lauf(());
        takte += erst.checksums.len();
        if erst.checksums != zweit.checksums {
            let stelle = erst
                .checksums
                .iter()
                .zip(zweit.checksums.iter())
                .position(|(a, b)| a != b);
            abweichungen.push(format!(
                "{}: zwei Laeufe gehen auseinander, zuerst im Takt {}",
                eintrag.name,
                stelle.map_or_else(|| "?".to_owned(), |s| s.to_string())
            ));
        }
        if erst.sequence_checksum != zweit.sequence_checksum {
            abweichungen.push(format!("{}: Folgen-Pruefsumme instabil", eintrag.name));
        }
    }
    if abweichungen.is_empty() {
        Pruefergebnis::Bestanden(format!(
            "{takte} Takt-Pruefsummen zweimal gerechnet, alle gleich"
        ))
    } else {
        Pruefergebnis::Gescheitert(abweichungen.join("\n      "))
    }
}

/// **Determinismus unter Drosselung**: Das Ergebnis darf nicht davon abhaengen,
/// wie schnell oder ueber wie viele Straenge gerechnet wird.
///
/// Ein Balance-Lauf wird ueber eine, zwei und vier Straenge gefahren. Kommen
/// verschiedene Zahlen heraus, haengt das Ergebnis an der Aufteilung — und
/// jede Messung auf einer anderen Maschine waere damit unvergleichbar.
fn pruefe_determinismus() -> Pruefergebnis {
    let Ok((_, apl)) = lade_apl(None) else {
        return Pruefergebnis::Uebersprungen(
            "die mitgelieferte Rotation liess sich nicht lesen".to_owned(),
        );
    };
    let mut werte = Vec::new();
    for threads in [1usize, 2, 4] {
        let config = BalanceConfig {
            iterations: 40,
            threads,
            ..BalanceConfig::default()
        };
        let ergebnis = run(&config, &apl);
        werte.push((threads, ergebnis.damage.mean, ergebnis.kills));
    }
    let (_, erste_mean, erste_kills) = werte[0];
    let abweichend: Vec<String> = werte
        .iter()
        .filter(|(_, mean, kills)| {
            // Bitgleich, nicht "nahe genug": Der Simulator rechnet in
            // ganzen Zahlen, ein Unterschied waere ein echter Fehler.
            (*mean - erste_mean).abs() > f64::EPSILON || *kills != erste_kills
        })
        .map(|(t, mean, kills)| format!("{t} Straenge: {mean} / {kills} Siege"))
        .collect();
    if abweichend.is_empty() {
        Pruefergebnis::Bestanden(format!(
            "ein, zwei und vier Straenge liefern {erste_mean} und {erste_kills} Siege"
        ))
    } else {
        Pruefergebnis::Gescheitert(format!(
            "ein Strang liefert {erste_mean} / {erste_kills} Siege, aber:\n      {}",
            abweichend.join("\n      ")
        ))
    }
}

/// Der Fingerabdruck dieses Baus — eine Zahl ueber alle Aufzeichnungen.
///
/// # Warum selbst gebildet
///
/// Der in #970 genannte Wert `0x42210c39918563b9` steht nirgends im Bestand,
/// und er ist 63-bittig, waehrend `checksum.rs` durchgehend in `u32` rechnet.
/// Er kann also nicht aus diesem Mechanismus stammen. Der Orchestrator hat ihn
/// daraufhin zurueckgezogen; dieser Fingerabdruck ist der belegte Ersatz.
fn fingerabdruck(gespielt: &[(&EmbeddedRecording, Option<u32>)]) -> u32 {
    let mut hash = FNV_OFFSET;
    for (eintrag, ergebnis) in gespielt {
        // Der NAME geht mit ein: Sonst ergaeben zwei Baue mit derselben
        // Zahlenmenge, aber verschiedenen Aufzeichnungen denselben Abdruck.
        hash = mix_named_int(hash, "recording", eintrag.name.len() as i32);
        match ergebnis {
            Some(wert) => hash = mix_named_int(hash, "checksum", *wert as i32),
            None => hash = mix_named_int(hash, "unplayable", 0),
        }
    }
    hash
}

/// **Bitgleichheit ueber Rechner**: Derselbe Bau muss anderswo dieselbe Zahl
/// ergeben.
///
/// Ist kein zweiter Rechner erreichbar, wird die Pruefung UEBERSPRUNGEN und
/// sagt das — sie scheitert nicht (#970, AC5). In der CI ist das der
/// Normalfall.
fn pruefe_bitgleichheit(
    gegenstelle: Option<&str>,
    gespielt: &[(&EmbeddedRecording, Option<u32>)],
) -> Pruefergebnis {
    let eigener = fingerabdruck(gespielt);
    let Some(host) = gegenstelle else {
        return Pruefergebnis::Uebersprungen(format!(
            "kein zweiter Rechner angegeben; eigener Fingerabdruck 0x{eigener:08x}\n      \
             Mit einem zweiten:  rota-sim verify --peer BENUTZER@RECHNER"
        ));
    };
    let ausgabe = std::process::Command::new("ssh")
        .args([
            // BatchMode: nie nach einem Passwort fragen — ein Selbsttest, der
            // auf eine Eingabe wartet, blockiert die Pruefstrecke still.
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            // **Ohne diese Zeile dauert ein unerreichbarer NAME achtzig
            // Sekunden**, gemessen: ConnectTimeout greift erst nach der
            // Namensaufloesung, und die laeuft ueber jeden Nameserver einzeln.
            // Mit `-o AddressFamily=inet` bleibt es bei einem Durchgang.
            "-o",
            "AddressFamily=inet",
            host,
            "rota-sim verify --print-fingerprint",
        ])
        .output();
    let Ok(ausgabe) = ausgabe else {
        return Pruefergebnis::Uebersprungen(format!(
            "{host} nicht erreichbar; eigener Fingerabdruck 0x{eigener:08x}"
        ));
    };
    if !ausgabe.status.success() {
        return Pruefergebnis::Uebersprungen(format!(
            "{host} antwortete nicht mit einem Fingerabdruck; eigener 0x{eigener:08x}"
        ));
    }
    let fremd_roh = String::from_utf8_lossy(&ausgabe.stdout).trim().to_owned();
    vergleiche_fingerabdruecke(eigener, &fremd_roh, host)
}

/// Entscheidet, ob zwei Fingerabdruecke zusammenpassen.
///
/// **Getrennt von der Beschaffung**, damit die Entscheidung ohne einen zweiten
/// Rechner pruefbar ist. Ein Vergleich, den man nur mit SSH ausloesen kann,
/// laesst sich nicht absichtlich brechen — und eine Pruefung, die nie
/// nachweislich angeschlagen hat, ist keine (#970, AC4).
fn vergleiche_fingerabdruecke(eigener: u32, fremd_roh: &str, host: &str) -> Pruefergebnis {
    let Ok(fremd) = u32::from_str_radix(fremd_roh.trim().trim_start_matches("0x"), 16) else {
        return Pruefergebnis::Uebersprungen(format!(
            "{host} lieferte '{fremd_roh}', keine Zahl; eigener 0x{eigener:08x}"
        ));
    };
    if fremd == eigener {
        Pruefergebnis::Bestanden(format!("0x{eigener:08x} auf beiden Rechnern"))
    } else {
        Pruefergebnis::Gescheitert(format!(
            "hier 0x{eigener:08x}, auf {host} 0x{fremd:08x} — die Baue sind nicht bitgleich"
        ))
    }
}

fn befehl_verify(argumente: &[String]) -> Result<ExitCode, String> {
    let mut gegenstelle: Option<String> = None;
    let mut nur_fingerabdruck = false;
    let mut i = 0;
    while i < argumente.len() {
        let name = argumente[i].as_str();
        match name {
            "--help" | "-h" => {
                println!("{HILFE_VERIFY}");
                return Ok(ExitCode::SUCCESS);
            }
            "--peer" => gegenstelle = Some(hole_wert(argumente, &mut i, name)?),
            // Fuer die Gegenstelle: nur die Zahl, damit ssh sie lesen kann.
            "--print-fingerprint" => nur_fingerabdruck = true,
            anderes => return Err(unbekannte_angabe("verify", anderes, &ANGABEN_VERIFY)),
        }
        i += 1;
    }

    // Vier Plaetze: Die Determinismuspruefung rechnet ueber bis zu vier Straenge.
    let werk = Zaehlwerk::neu(4);
    let _anmeldung = werk.anmelden();

    // **Einmal abspielen, dreimal auswerten.** Die Reihenfolge ist wichtig:
    // Ohne die gesammelten Werte rechnete jede Pruefung dieselben Runden neu.
    let gespielt = spiele_alle_ab();

    if nur_fingerabdruck {
        println!("0x{:08x}", fingerabdruck(&gespielt));
        return Ok(ExitCode::SUCCESS);
    }

    let pruefungen = [
        ("Paritaet gegen das Spiel", pruefe_paritaet(&gespielt)),
        ("Trace-Hashes", pruefe_trace_hashes()),
        ("Determinismus ueber Straenge", pruefe_determinismus()),
        (
            "Bitgleichheit ueber Rechner",
            pruefe_bitgleichheit(gegenstelle.as_deref(), &gespielt),
        ),
    ];

    println!("{}", "=".repeat(72));
    println!("ROTA — Selbsttest");
    println!("{}", "=".repeat(72));
    println!();
    for (name, ergebnis) in &pruefungen {
        println!("{:<13} {name}", ergebnis.marke());
        println!("      {}", ergebnis.text());
        println!();
    }

    let gescheitert: Vec<&str> = pruefungen
        .iter()
        .filter(|(_, e)| e.ist_gescheitert())
        .map(|(n, _)| *n)
        .collect();

    println!("{}", "-".repeat(72));
    if gescheitert.is_empty() {
        println!("ERGEBNIS: BESTANDEN");
    } else {
        println!("ERGEBNIS: NICHT BESTANDEN");
        println!("Gescheitert: {}", gescheitert.join(", "));
    }
    println!();
    println!("WAS DIESER SELBSTTEST NICHT IST");
    println!(
        "  Er prueft {} der {} Dateien in tests/parity/recordings/. Die uebrigen {}",
        data::RECORDINGS.len(),
        data::RECORDING_FILES_TOTAL,
        data::RECORDINGS_SKIPPED
    );
    println!("  beschreiben keine abspielbare Runde — es sind Pruefdaten fuer");
    println!("  Ausruestung, Ereignisstroeme und Buff-Ablaeufe.");
    println!();
    println!("  Und er traegt nur die Pruefsumme des LETZTEN Takts. Eine Abweichung");
    println!("  zeigt sich, aber nicht der Takt, in dem sie beginnt. Dafuer:");
    println!("    cargo test --manifest-path sim/Cargo.toml --test parity_recording");
    println!();
    println!("  Ein bestandener Selbsttest ist keine Abnahme eines Issues.");
    println!();
    println!("Kaempfe gerechnet: {}", kaempfe_gerechnet(&werk));

    Ok(if gescheitert.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Die Angaben, die `verify` kennt.
const ANGABEN_VERIFY: [&str; 4] = ["--peer", "--print-fingerprint", "--help", "-h"];

// ---------------------------------------------------------------------------
// explain — der Trockenlauf
// ---------------------------------------------------------------------------

/// Die Einstellungen von `explain`.
#[derive(Debug, Clone, PartialEq)]
struct ExplainOptionen {
    boss_id: Option<String>,
    max_len: usize,
    iterations: u32,
    threads: usize,
    rate: Option<f64>,
    json: bool,
    hilfe: bool,
}

impl Default for ExplainOptionen {
    fn default() -> Self {
        let vorgabe = RunOptionen::default();
        Self {
            boss_id: None,
            max_len: vorgabe.max_len,
            iterations: vorgabe.iterations,
            threads: vorgabe.threads,
            rate: None,
            json: false,
            hilfe: false,
        }
    }
}

/// Kaempfe je Sekunde, wenn niemand eine eigene Rate nennt.
///
/// **Sie gilt fuer EINEN Strang**, und ohne diese Angabe ist eine Rate nicht
/// deutbar. Das ist kein Formalismus, sondern am 2026-08-18 teuer geworden:
/// Ich hielt die Planzahl aus GAME-DESIGN (350 000 bis 365 000) fuer einen
/// Widerspruch zu meinen gemessenen 38 474 — Faktor neun. Sie ist keiner. Die
/// Planzahl meint den Vollausbau mit sechzehn Straengen; nachgemessen wurden
/// dort 258 354. Zwei Zahlen ohne ihre Strangzahl sahen aus wie ein Fehler.
///
/// **Gemessen, nicht angenommen** — aber die Messung ist der Grund, warum
/// diese Zahl mit Vorsicht zu geniessen ist. Fuenf Laeufe von
/// `rota-sim bench --fights 20000` im Release-Bau auf EINEM Strang, alle am
/// 2026-08-18 auf derselben Maschine:
///
/// ```text
///   38474   39913   50979   56970   51957
/// ```
///
/// **Faktor 1,5 zwischen der langsamsten und der schnellsten Messung.** Der
/// Unterschied ist die Last: Die beiden niedrigen entstanden, waehrend
/// nebenher gebaut wurde, die drei hohen bei `load average` um fuenf auf
/// sechzehn Kernen. Die Kontrollzeile (Schaden je Kampf) war in allen fuenf
/// Laeufen gleich — gerechnet wurde also jedes Mal dasselbe.
///
/// Der Wert hier ist die Mitte, gerundet. Er taugt fuer eine Groessenordnung,
/// nicht fuer eine Zusage, und [`RATE_SCHWANKUNG`] sagt das in der Ausgabe.
///
/// Die Zahl steht bewusst hier und nicht in der Bibliothek — sie ist eine
/// Eigenschaft dieses Rechners, keine des Simulators.
const RATE_VORGABE: f64 = 45_000.0;

/// Fuer wie viele Straenge [`RATE_VORGABE`] gilt.
///
/// Steht als eigene Konstante da, damit die Zahl und ihre Bedingung nicht
/// auseinanderlaufen koennen — und sie geht in jede Ausgabe mit ein.
const RATE_VORGABE_STRAENGE: usize = 1;

/// Um welchen Faktor die gemessene Rate an einem Tag geschwankt hat.
///
/// Steht in der Ausgabe, damit niemand die geschaetzte Dauer fuer eine Zusage
/// haelt. Wer es genauer braucht, misst die eigene Rate mit `rota-sim bench`
/// **auf der Maschine, die den Lauf dann faehrt** — und moeglichst dann, wenn
/// sie so belegt ist wie waehrend des Laufs.
const RATE_SCHWANKUNG: f64 = 1.5;

/// Ab welcher geschaetzten Dauer der Trockenlauf ablehnt (#969, AC5).
///
/// Ein Jahr ist nicht die Grenze der Geduld, sondern die der Sinnhaftigkeit:
/// Ein Lauf, der so lange braucht, wird nicht anders eingestellt, sondern
/// anders gestellt. Die Schwelle ist grosszuegig, damit ein ernst gemeinter
/// Lauf ueber ein Wochenende nicht abgelehnt wird.
const DAUER_ABLEHNUNG_SEKUNDEN: f64 = 365.0 * 24.0 * 3600.0;

/// Was ein Lauf kosten wuerde — ohne einen einzigen Kampf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Schaetzung {
    /// Kandidatenzeilen, die zur Wahl stehen.
    pool: usize,
    /// Messungen, wenn die Suche bis zur Laengengrenze laeuft.
    messungen_max: u64,
    /// Messungen, wenn sie nach der ersten Runde abbricht.
    messungen_min: u64,
    /// Einzelzuege im Maximum — der Anteil, der exakt feststeht.
    einzelzuege: u64,
    /// Paar-Zuege im Maximum.
    paarzuege: u64,
}

/// Rechnet aus, was eine Suche kosten wuerde.
///
/// # Warum das exakt geht und nicht geschaetzt werden muss
///
/// `greedy_opts` misst in Runde k fuer jeden offenen Kandidaten an jeder
/// Einfuegeposition. Bei Pool n und Runde k sind das `(n - k + 1) * k`
/// Einzelzuege — beides folgt aus der Schleife, nicht aus Messwerten.
///
/// Die Paar-Zuege kommen aus [`paar_zuege`], das ebenfalls keinen Kampfwert
/// liest. Nur WELCHE Kandidaten in spaeteren Runden noch offen sind, haengt
/// am Ergebnis — deshalb ist ihre Zahl fuer Runde eins exakt und darueber eine
/// Obergrenze.
///
/// # Die zwei Messungen, die `measured` nicht zaehlt
///
/// Die Startliste und die gefundene Liste. Sie stehen als
/// [`MESSUNGEN_AUSSERHALB`] in beiden Zahlen — sonst laege der Trockenlauf bei
/// einer kurzen Suche mehrere Prozent zu niedrig, und zwar systematisch.
fn schaetze(pool: &[Candidate], max_len: usize) -> Schaetzung {
    let n = pool.len();
    let offen: Vec<usize> = (0..n).collect();
    // Fuer Runde eins exakt; fuer spaetere eine Obergrenze, weil dort weniger
    // Kandidaten offen sind.
    let paare_runde_eins = paar_zuege(pool, &offen).len() as u64;

    let mut einzelzuege = 0u64;
    let mut paarzuege = 0u64;
    let mut erste_runde = 0u64;
    for k in 1..=max_len {
        if k > n {
            break;
        }
        let einzeln = (n - k + 1) as u64 * k as u64;
        einzelzuege += einzeln;
        // Die Paar-Betrachtung laeuft nur, solange zwei Zeilen noch in die
        // Laengengrenze passen.
        let paare = if (k - 1) + 2 <= max_len {
            paare_runde_eins * k as u64
        } else {
            0
        };
        paarzuege += paare;
        if k == 1 {
            erste_runde = einzeln + paare;
        }
    }

    Schaetzung {
        pool: n,
        messungen_max: einzelzuege + paarzuege + MESSUNGEN_AUSSERHALB,
        messungen_min: erste_runde + MESSUNGEN_AUSSERHALB,
        einzelzuege,
        paarzuege,
    }
}

/// Der Speicher, den die Stichprobe eines Laufs belegt.
///
/// Der groesste Einzelposten ist `BalanceResult::samples` — ein Eintrag je
/// Durchgang. Andere Puffer (der Kampf selbst, die Kandidatenlisten) sind
/// dagegen klein und konstant; sie sind hier NICHT enthalten, und die Ausgabe
/// sagt das.
fn speicher_bytes(iterations: u32) -> u64 {
    std::mem::size_of::<rota_sim::balance::Sample>() as u64 * u64::from(iterations)
}

/// Eine Groesse in der Einheit, in der sie noch etwas aussagt.
///
/// „0,0 MB" fuer zweitausend Bytes sieht aus wie ein nicht gebauter Posten und
/// ist genau der Eindruck, den ein Trockenlauf nicht erwecken darf.
fn groesse(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} Bytes");
    }
    let kb = bytes as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{kb:.1} KB");
    }
    let mb = kb / 1024.0;
    if mb < 1024.0 {
        return format!("{mb:.1} MB");
    }
    format!("{:.1} GB", mb / 1024.0)
}

/// Eine Dauer in einer Form, die man lesen kann.
fn dauer_lesbar(sekunden: f64) -> String {
    if !sekunden.is_finite() {
        return "nicht bestimmbar".to_owned();
    }
    if sekunden < 90.0 {
        return format!("{sekunden:.1} Sekunden");
    }
    let minuten = sekunden / 60.0;
    if minuten < 90.0 {
        return format!("{minuten:.1} Minuten");
    }
    let stunden = minuten / 60.0;
    if stunden < 48.0 {
        return format!("{stunden:.1} Stunden");
    }
    let tage = stunden / 24.0;
    if tage < 730.0 {
        return format!("{tage:.1} Tage");
    }
    format!("{:.1} Jahre", tage / 365.0)
}

fn befehl_explain(argumente: &[String]) -> Result<ExitCode, String> {
    let optionen = zerlege_explain(argumente)?;
    if optionen.hilfe {
        println!("{HILFE_EXPLAIN}");
        return Ok(ExitCode::SUCCESS);
    }

    let boss = waehle_boss(optionen.boss_id.as_deref())?;
    let pool = all_candidates();
    // Angemeldet, damit die Zusicherung „kein Kampf" dieselbe Quelle liest wie
    // jeder andere Befehl — eine Null ohne Zaehlwerk belegte nichts.
    let werk = Zaehlwerk::neu(1);
    let _anmeldung = werk.anmelden();

    // AC5, erster Fall: strukturell unerfuellbar. Eine Rotation kann nicht
    // mehr Zeilen haben, als es Kandidaten gibt — das faellt hier auf und
    // nicht nach der halben Nacht.
    if optionen.max_len > pool.len() {
        return Err(fehler_mit_ausweg(
            &format!(
                "Eine Rotation mit {} Zeilen gibt es nicht — es stehen nur {} Kandidatenzeilen zur Wahl.",
                optionen.max_len,
                pool.len()
            ),
            Some(format!(
                "Hoechstens '--max-len {}' ist erfuellbar.",
                pool.len()
            )),
            "Die Kandidatenzeilen zaehlt:  rota-sim data candidates",
        ));
    }

    let schaetzung = schaetze(&pool, optionen.max_len);
    let rate = optionen.rate.unwrap_or(RATE_VORGABE);
    let kaempfe_max = schaetzung.messungen_max * u64::from(optionen.iterations);
    let kaempfe_min = schaetzung.messungen_min * u64::from(optionen.iterations);
    // **Die Spanne traegt BEIDE Unsicherheiten**, nicht nur eine.
    //
    // Die erste ist die Suche selbst: Sie kann nach der ersten Runde abbrechen
    // oder bis zur Laengengrenze laufen. Die zweite ist die Rate — und die
    // wurde beim Bauen dieses Befehls teuer gelernt: Die erste Fassung teilte
    // nur durch die Vorgabe-Rate, sagte "0,13 bis 1,18 Sekunden" und der
    // tatsaechliche Lauf brauchte 1,31. Eine Schaetzung, deren Spanne die
    // bekannte Unsicherheit nicht enthaelt, ist zu schmal — und eine zu schmale
    // Spanne ist schlimmer als eine weite, weil sie Genauigkeit vortaeuscht.
    //
    // Wer eine eigene Rate nennt, bekommt die Weitung NICHT: Er hat auf seiner
    // Maschine gemessen und weiss, was er tut.
    let weitung = if optionen.rate.is_some() {
        1.0
    } else {
        RATE_SCHWANKUNG
    };
    let dauer_max = kaempfe_max as f64 / rate * weitung;
    let dauer_min = kaempfe_min as f64 / rate / weitung;
    let speicher = speicher_bytes(optionen.iterations);

    // AC5, zweiter Fall: rechnerisch moeglich, praktisch nicht.
    if dauer_max > DAUER_ABLEHNUNG_SEKUNDEN {
        return Err(fehler_mit_ausweg(
            &format!(
                "Dieser Lauf braeuchte hoechstens {} — das lehnt der Trockenlauf ab.",
                dauer_lesbar(dauer_max)
            ),
            Some(format!(
                "Die Grenze liegt bei {}. Kleiner werden laesst sich ueber --max-len oder --iterations.",
                dauer_lesbar(DAUER_ABLEHNUNG_SEKUNDEN)
            )),
            "Was die Stellschrauben tun:  rota-sim explain --help",
        ));
    }

    if optionen.json {
        println!(
            "{{\"boss\":{:?},\"poolSize\":{},\"maxLen\":{},\"iterations\":{},\"threads\":{},\
             \"measurements\":{{\"min\":{},\"max\":{},\"singles\":{},\"pairs\":{}}},\
             \"fights\":{{\"min\":{},\"max\":{}}},\
             \"seconds\":{{\"min\":{:?},\"max\":{:?}}},\
             \"rate\":{{\"fightsPerSecond\":{:?},\"source\":{:?},\"threads\":{}}},\
             \"memoryBytes\":{},\"fightsRun\":{}}}",
            boss.id,
            schaetzung.pool,
            optionen.max_len,
            optionen.iterations,
            optionen.threads,
            schaetzung.messungen_min,
            schaetzung.messungen_max,
            schaetzung.einzelzuege,
            schaetzung.paarzuege,
            kaempfe_min,
            kaempfe_max,
            dauer_min,
            dauer_max,
            rate,
            if optionen.rate.is_some() {
                "angegeben"
            } else {
                "Vorgabe"
            },
            // **Die Strangzahl gehoert zur Rate**, sonst ist sie nicht
            // deutbar. Eine angegebene Rate gilt fuer die Strangzahl des
            // Aufrufs; die Vorgabe fuer die, auf der sie gemessen wurde.
            if optionen.rate.is_some() {
                optionen.threads
            } else {
                RATE_VORGABE_STRAENGE
            },
            speicher,
            kaempfe_gerechnet(&werk),
        );
        return Ok(ExitCode::SUCCESS);
    }

    println!("{}", "=".repeat(72));
    println!("ROTA — Trockenlauf gegen {} ({})", boss.name, boss.id);
    println!("{}", "=".repeat(72));
    println!();
    println!("WAS FESTSTEHT");
    println!("  Kandidatenzeilen     {}", schaetzung.pool);
    println!(
        "  Messungen            {} bis {}",
        schaetzung.messungen_min, schaetzung.messungen_max
    );
    println!(
        "  davon Einzelzuege    {}  ({:.0} Prozent)",
        schaetzung.einzelzuege,
        anteil(schaetzung.einzelzuege, schaetzung.messungen_max)
    );
    println!(
        "  davon Paar-Zuege     {}  ({:.0} Prozent)",
        schaetzung.paarzuege,
        anteil(schaetzung.paarzuege, schaetzung.messungen_max)
    );
    println!(
        "  Kaempfe              {} bis {}   ({} Durchgaenge je Messung)",
        kaempfe_min, kaempfe_max, optionen.iterations
    );
    println!();
    println!("WAS GESCHAETZT IST");
    println!(
        "  Dauer                {} bis {}",
        dauer_lesbar(dauer_min),
        dauer_lesbar(dauer_max)
    );
    let raten_straenge = if optionen.rate.is_some() {
        optionen.threads
    } else {
        RATE_VORGABE_STRAENGE
    };
    println!(
        "  bei einer Rate von   {rate:.0} Kaempfen je Sekunde auf {raten_straenge} {} ({})",
        if raten_straenge == 1 {
            "Strang"
        } else {
            "Straengen"
        },
        if optionen.rate.is_some() {
            "von dir angegeben"
        } else {
            "Vorgabe, gemessen"
        }
    );
    println!(
        "  Speicher             {} fuer die Stichprobe",
        groesse(speicher)
    );
    println!();
    println!("RECHNER");
    println!("  Beteiligt            einer, dieser hier");
    println!("  Mehr als einer geht noch nicht — siehe:  rota-sim hosts");
    if optionen.threads > 1 {
        println!(
            "  Die Dauer oben gilt fuer EINEN Strang. Mit {} Straengen wird sie",
            optionen.threads
        );
        println!("  kleiner, aber nicht um den Faktor — die eigene Zahl misst:");
        println!("    rota-sim bench --threads {}", optionen.threads);
    }
    println!();
    println!("UNSICHERHEIT");
    println!("  Die Spanne bei Messungen und Kaempfen ist keine Ungenauigkeit,");
    println!("  sondern die Suche selbst: Sie bricht ab, sobald keine Zeile mehr");
    println!("  traegt. Das Minimum ist die erste Runde, das Maximum sind alle.");
    println!();
    println!("  Die Dauer haengt zusaetzlich an Maschine und Bau. Im Debug-Bau ist");
    println!("  der Kern rund achtzehnmal langsamer als im Release.");
    if optionen.rate.is_none() {
        println!();
        println!("  DIE VORGABE-RATE IST EINE GROESSENORDNUNG, KEINE ZUSAGE: Fuenf",);
        println!(
            "  Messungen an einem Tag auf derselben Maschine lagen um Faktor {RATE_SCHWANKUNG}"
        );
        println!("  auseinander, je nach Last. Die Dauer oben kann also entsprechend");
        println!("  daneben liegen. Die eigene Rate misst:  rota-sim bench");
    }
    println!();
    println!("  Der Speicher nennt nur die Stichprobe — den groessten Posten. Was");
    println!("  der Kampf selbst belegt, ist klein und konstant und fehlt hier.");
    println!();
    println!("Kaempfe gerechnet: {}", kaempfe_gerechnet(&werk));
    Ok(ExitCode::SUCCESS)
}

/// Anteil in Prozent, ohne Sturz bei null.
fn anteil(teil: u64, ganzes: u64) -> f64 {
    if ganzes == 0 {
        return 0.0;
    }
    teil as f64 * 100.0 / ganzes as f64
}

fn zerlege_explain(argumente: &[String]) -> Result<ExplainOptionen, String> {
    let mut optionen = ExplainOptionen::default();
    let mut i = 0;
    while i < argumente.len() {
        let name = argumente[i].as_str();
        match name {
            "--help" | "-h" => optionen.hilfe = true,
            "--json" => optionen.json = true,
            "--boss" => optionen.boss_id = Some(hole_wert(argumente, &mut i, name)?),
            "--max-len" => optionen.max_len = hole_zahl(argumente, &mut i, name)?,
            "--iterations" => optionen.iterations = hole_zahl(argumente, &mut i, name)?,
            "--threads" => optionen.threads = hole_zahl(argumente, &mut i, name)?,
            "--rate" => {
                let roh = hole_wert(argumente, &mut i, name)?;
                let wert: f64 = roh.parse().map_err(|_| {
                    fehler_mit_ausweg(
                        &format!("'--rate' braucht eine Zahl, bekommen hat es '{roh}'."),
                        None,
                        "Die eigene Rate misst:  rota-sim bench",
                    )
                })?;
                if wert <= 0.0 || !wert.is_finite() {
                    return Err(fehler_mit_ausweg(
                        "'--rate' muss groesser als null sein.",
                        Some(
                            "Bei null Kaempfen je Sekunde dauert jeder Lauf unendlich.".to_owned(),
                        ),
                        "Die eigene Rate misst:  rota-sim bench",
                    ));
                }
                optionen.rate = Some(wert);
            }
            anderes => return Err(unbekannte_angabe("explain", anderes, &ANGABEN_EXPLAIN)),
        }
        i += 1;
    }
    if optionen.iterations == 0 {
        return Err(fehler_mit_ausweg(
            "'--iterations' muss groesser als null sein.",
            Some("Ein Lauf ohne Durchgang hat nichts gerechnet.".to_owned()),
            "Mit Beispielen:  rota-sim explain --help",
        ));
    }
    if optionen.max_len == 0 {
        return Err(fehler_mit_ausweg(
            "'--max-len' muss groesser als null sein.",
            Some("Eine Rotation ohne Zeile ist keine Rotation.".to_owned()),
            "Mit Beispielen:  rota-sim explain --help",
        ));
    }
    Ok(optionen)
}

/// Die Angaben, die `explain` kennt.
const ANGABEN_EXPLAIN: [&str; 8] = [
    "--boss",
    "--max-len",
    "--iterations",
    "--threads",
    "--rate",
    "--json",
    "--help",
    "-h",
];

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
    let wertung = ScoringSet::all_scored();
    let spielinstanzen = wertung.groups().len();
    let profile = probe_bosses();
    let pruefprofile = belegte_instanzen(&profile);

    if json {
        println!(
            // **Jeder Schluessel traegt das Wort, das im Klartext daneben
            // steht** — die Ausgabe wird zusammengehalten, nicht der Code:
            //
            //     gameInstances  <->  "Spielinstanzen"
            //     probeProfiles  <->  "Profilen"
            //     scoredBosses   <->  "wertungsfaehig"
            //
            // Ein Schluessel mit dem Code-Begriff (`scoredInstances`, nach
            // `ScoringSet` aus #981) braeuchte eine Uebersetzungstabelle zum
            // Klartext — und eine Tabelle zwischen zwei Namen fuer dieselbe
            // Sache ist die Stelle, die spaeter auseinanderlaeuft. Der
            // Code-Begriff steht ohnehin im Code; die Ausgabe ist fuer den, der
            // sie liest.
            //
            // `probeProfiles` und nicht `probeInstances`: `test` und `testing`
            // sind keine Spielorte, die jemand betritt.
            "{{\"bosses\":{},\"spells\":{},\"scoredBosses\":{},\"gameInstances\":{},\
             \"probeBosses\":{},\"probeProfiles\":{},\"candidates\":{}}}",
            data::BOSSES.len(),
            data::SPELL_COUNT,
            wertung.len(),
            spielinstanzen,
            profile.len(),
            pruefprofile,
            kandidaten.len()
        );
        return;
    }
    println!("Einkompilierte Inhalte dieses Baus");
    println!();
    println!("  Bosse              {}", data::BOSSES.len());
    println!(
        "    wertungsfaehig   {} in {spielinstanzen} Spielinstanzen",
        wertung.len()
    );
    println!(
        "    Pruefprofile     {} in {pruefprofile} Profilen — in KEINER Wertung",
        profile.len()
    );
    println!("  Zauber             {}", data::SPELL_COUNT);
    println!("  Kandidatenzeilen   {}", kandidaten.len());
    println!();
    println!("Einzeln:  rota-sim data bosses | spells | candidates");
}

/// Wie viele verschiedene `instance`-Werte diese Bosse belegen.
///
/// **Belegte, nicht erlaubte.** [`data::SCORED_INSTANCES`] nennt die Werte, die
/// eine Spielinstanz benennen DUERFEN; hier steht, wie viele davon tatsaechlich
/// Bosse haben. Eine Instanz ohne Bosse waere sonst eine Zahl in der Uebersicht,
/// hinter der nichts steht.
///
/// Ohne Container mit Streuung — GAME-DESIGN 18.4 verbietet Hash-Abbildungen
/// dort, wo die Reihenfolge in ein Ergebnis einfliessen koennte.
fn belegte_instanzen(bosse: &[&'static data::Boss]) -> usize {
    let mut gesehen: Vec<&'static str> = Vec::new();
    for boss in bosse {
        if !gesehen.contains(&boss.instance) {
            gesehen.push(boss.instance);
        }
    }
    gesehen.len()
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

/// Wie [`hole_wert`], nur als Gleitkommazahl — fuer Schwelle und alpha.
///
/// Ob der Wert im zulaessigen Bereich liegt, prueft [`Staffelung::pruefe`],
/// nicht diese Funktion: Eine Pruefung an zwei Stellen liefe auseinander.
fn hole_anteil(argumente: &[String], i: &mut usize, name: &str) -> Result<f64, String> {
    let roh = hole_wert(argumente, i, name)?;
    roh.parse().map_err(|_| {
        fehler_mit_ausweg(
            &format!("'{name}' braucht eine Zahl wie 0.99, bekommen hat es '{roh}'."),
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
        //
        // `explain` stand hier, bis #969 ihn gebaut hat, `verify` bis #970.
        // Beide geben jetzt Zahlen aus, und das ist ihr Zweck — die
        // Zusicherung gilt nur noch fuer die zwei, die wirklich leer sind.
        //
        // **Diese Liste ist die Stelle, an der ein Ausbau sichtbar wird.** Wer
        // einen Platzhalter fuellt und sie vergisst, bekommt einen roten Test
        // statt einer Zusicherung, die nichts mehr sichert.
        for (name, grund) in [("hosts", GRUND_HOSTS), ("status", GRUND_STATUS)] {
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
    fn zerlege_run_liest_die_staffelung() {
        let optionen = zerlege_run(&worte(
            "--search none --purity 500 --mode gestaffelt --threshold 0.9 \
             --alpha 0.05 --budget 512 --threads 1",
        ))
        .expect("alle Angaben sind gueltig");
        assert_eq!(optionen.modus, Modus::Gestaffelt);
        assert_eq!(optionen.purity, 500);
        assert_eq!(
            staffelung_aus(&optionen),
            Staffelung {
                schwelle: 0.9,
                alpha: 0.05,
                budget: 512,
            }
        );
    }

    #[test]
    fn die_voreinstellung_bleibt_der_feste_lauf_ohne_krit() {
        // Wer keine der neuen Angaben nennt, bekommt denselben Lauf wie vor
        // #915: feste Zahl, keine Ausruestung, keine Schwelle.
        let optionen = RunOptionen::default();
        assert_eq!(optionen.modus, Modus::Fest);
        assert_eq!(optionen.purity, 0);
        assert_eq!(
            (optionen.schwelle, optionen.alpha, optionen.budget),
            (None, None, None)
        );
        assert!(!optionen.iterations_genannt);
    }

    #[test]
    fn zerlege_run_verlangt_alle_drei_staffelangaben() {
        let fehler = zerlege_run(&worte("--search none --mode gestaffelt --threshold 0.9"))
            .expect_err("alpha und Budget fehlen");
        assert!(
            fehler.contains("--alpha") && fehler.contains("--budget"),
            "{fehler}"
        );
        assert!(fehler.starts_with("FEHLER"), "{fehler}");
    }

    #[test]
    fn zerlege_run_weist_widerspruechliche_angaben_ab() {
        let basis = "--search none --mode gestaffelt --threshold 0.9 --alpha 0.05 --budget 64";
        assert!(
            zerlege_run(&worte(basis)).is_ok(),
            "Kontrollzeile: die Basis allein ist gueltig"
        );
        for (zusatz, erwartet) in [
            ("--iterations 20", "--iterations"),
            ("--threads 4", "--threads"),
            ("--control-file steuer.json", "--control-file"),
        ] {
            let fehler = zerlege_run(&worte(&format!("{basis} {zusatz}"))).expect_err(zusatz);
            assert!(fehler.contains(erwartet), "{zusatz}: {fehler}");
            assert!(fehler.contains("rota-sim run --help"), "{zusatz}: {fehler}");
        }

        // Gestaffelt misst eine vorhandene Rotation; ohne --apl und ohne
        // --search none waere es eine Suche.
        let fehler = zerlege_run(&worte(
            "--mode gestaffelt --threshold 0.9 --alpha 0.05 --budget 64",
        ))
        .expect_err("ohne Rotation");
        assert!(fehler.contains("vorhandene Rotation"), "{fehler}");

        // Und umgekehrt: Staffelangaben ohne den Modus wirkten nicht.
        let fehler = zerlege_run(&worte("--budget 64")).expect_err("ohne Modus");
        assert!(fehler.contains("--mode gestaffelt"), "{fehler}");
    }

    #[test]
    fn zerlege_run_prueft_die_werte_der_staffelung() {
        let fehler = zerlege_run(&worte(
            "--search none --mode gestaffelt --threshold 1 --alpha 0.05 --budget 64",
        ))
        .expect_err("Schwelle 1 ist mit einer Stichprobe nie belegbar");
        assert!(fehler.contains("Schwelle"), "{fehler}");

        let fehler = zerlege_run(&worte(
            "--search none --mode gestaffelt --threshold hoch --alpha 0.05 --budget 64",
        ))
        .expect_err("keine Zahl");
        assert!(fehler.contains("--threshold"), "{fehler}");

        let fehler = zerlege_run(&worte("--mode gestaffel")).expect_err("Tippfehler");
        assert!(fehler.contains("'gestaffelt'"), "{fehler}");
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
    fn run_und_bench_lesen_die_fortschrittsangaben() {
        let erwartet = Anzeigewahl {
            progress: true,
            jsonl: Some("strom.jsonl".to_owned()),
        };
        let run = zerlege_run(&worte("--progress --jsonl strom.jsonl")).expect("gueltig");
        assert_eq!(run.anzeige, erwartet);
        let bench = zerlege_bench(&worte("--jsonl strom.jsonl --progress")).expect("gueltig");
        assert_eq!(bench.anzeige, erwartet);
        // Ohne Angabe: keine Anzeige, und das ist die Voreinstellung.
        assert_eq!(RunOptionen::default().anzeige, Anzeigewahl::default());
        assert!(zerlege_run(&worte("--jsonl")).is_err(), "der Pfad fehlt");
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

    /// Die Zaehlung TRENNT, statt zu addieren — geprueft an der Methode.
    ///
    /// # Woher die alte Sieben kam
    ///
    /// Bis zur Nachbereitung von #897 zaehlte `anzahl_instanzen()` die
    /// verschiedenen `instance`-Werte ueber ALLE Bosse und kam auf 7. Die Zahl
    /// war fuer ihre Frage richtig und las sich trotzdem falsch: Es gibt
    /// **fuenf** Spielinstanzen; die zwei uebrigen Werte sind Pruefprofile, die
    /// kein Spieler je sieht.
    ///
    /// # Warum hier keine Zahl allein steht
    ///
    /// Ein `assert_eq!(spielinstanzen, 5)` wuerde auch rot, wenn jemand eine
    /// echte Instanz **entfernt** — das ist eine Aussage ueber den Bestand,
    /// nicht ueber die Zaehlweise. Dieser Fall prueft stattdessen die
    /// Eigenschaft, die der alten Methode fehlte: Es gibt **zwei** belegte
    /// Klassen, beide sind echt kleiner als das Ganze, und zusammen ergeben sie
    /// es genau. Eine Zaehlung, die addiert, liefert fuer die eine Klasse das
    /// Ganze — und faellt an der dritten Zusicherung.
    ///
    /// Die Bestandszahlen stehen darunter, ausdruecklich getrennt.
    #[test]
    fn die_zaehlung_trennt_statt_zu_addieren() {
        let alle: Vec<&'static Boss> = data::BOSSES.iter().collect();
        let insgesamt = belegte_instanzen(&alle);
        let spielinstanzen = ScoringSet::all_scored().groups().len();
        let profile = probe_bosses();
        let pruefprofile = belegte_instanzen(&profile);

        assert!(
            spielinstanzen > 0 && pruefprofile > 0,
            "beide Klassen muessen belegt sein: {spielinstanzen} und {pruefprofile}"
        );
        assert!(
            spielinstanzen < insgesamt && pruefprofile < insgesamt,
            "eine Klasse umfasst alle {insgesamt} belegten Werte — dann wird addiert \
             statt getrennt: {spielinstanzen} und {pruefprofile}"
        );
        assert_eq!(
            spielinstanzen + pruefprofile,
            insgesamt,
            "die Trennung teilt dieselbe Menge auf, ohne Rest und ohne Doppel"
        );
    }

    /// Die Zahlen des Bestands — getrennt vom Fall darueber, weil sie eine
    /// andere Frage beantworten.
    ///
    /// Faellt dieser Fall, hat sich der Datenbestand geaendert. Faellt der
    /// darueber, hat sich die Zaehlweise geaendert. Zwei Ursachen, zwei
    /// Meldungen.
    #[test]
    fn die_zahlen_des_bestands() {
        assert_eq!(ScoringSet::all_scored().groups().len(), 6);
        assert_eq!(belegte_instanzen(&probe_bosses()), 2);
    }
}
