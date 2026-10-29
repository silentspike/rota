//! Die Fortschritts-Simulation ausfuehren (Issue #922, E11.4e, Fassung A).
//!
//! ```text
//! cargo remote -c -- build --release --example laufbahn
//! target/release/examples/laufbahn --laufbahnen 10000 --straenge 8
//! target/release/examples/laufbahn --laufbahnen 10000 --fallrate 100 --katalog-nur focus-mix --siege 30
//! target/release/examples/laufbahn --laufbahnen 10000 --vergleich-aufwertung
//! ```
//!
//! Ausgegeben werden fuer `--laufbahnen N`:
//!
//! * Punkte je Sieg ueber die Laufbahn (Mittel, p10, Median, p90),
//! * die Gegenstandsstufe ueber die Siege (Mittel der zehn Plaetze),
//! * der Anteil der Laufbahnen, die den verfolgten Fund nach 10/20/30 Siegen
//!   noch nicht haben, mit der Toleranz, die aus `n` folgt,
//! * Kaempfe bis Instanz N (geoeffnet und abgeschlossen),
//! * die Ausruestung beim Betreten neben dem gesetzten erwarteten Stand,
//! * mit `--vergleich-aufwertung` beide Wege nebeneinander (AC7).
//!
//! **Ein Startwert, eine Ausgabe** — gleich auf wie vielen Straengen. Die
//! Strangzahl steht deshalb nicht im Bericht, nur auf dem Fehlerkanal.
//!
//! Die Simulation rechnet; sie laeuft nach Vorgabe des Auftraggebers auf
//! node3 (CT 105), nicht auf dem Laptop.

use std::process::ExitCode;
use std::time::Instant;

use rota_sim::apl::{parse, AplName};
use rota_sim::laufbahn::auswertung::{bericht, kampf_von, rechne, vergleich, Auftrag};
use rota_sim::laufbahn::orakel::{Kampfeinstellung, Referenzspielweise};
use rota_sim::laufbahn::regeln::KATALOG;
use rota_sim::laufbahn::schleife::{Aufwerten, Fundziel, Laufbahnregeln, Messauftrag};

const REFERENZ: &str = include_str!("../apl/laufbahn-referenz.apl");

const HILFE: &str = "laufbahn — Fortschritts-Simulation (#922, Fassung A)

  --laufbahnen N        wie viele Laufbahnen (10000)
  --startwert S         Startwert des Laufs (1)
  --straenge T          Straenge; beruehrt kein Ergebnis (1)
  --siege W             Siege je Laufbahn (100)
  --kaempfe F           hoechstens so viele Kaempfe je Laufbahn (2000)
  --durchgaenge K       Durchgaenge je Orakel-Zelle (1)
  --apl DATEI           Referenz-Spielweise (sim/apl/laufbahn-referenz.apl)
  --fund ID[:GUETE]     verfolgter Fund, Guete 0-3 (focus-mix:0)
  --fallrate PROMILLE   Fallrate statt data/items.json
  --katalog-nur ID      nur diese Vorlage faellt
  --ohne-aufwertung     Material bleibt liegen
  --vergleich-aufwertung  beide Wege, gleiche Laufbahnen (AC7)
";

struct Angaben {
    laufbahnen: u32,
    startwert: u32,
    straenge: usize,
    siege: u32,
    kaempfe: u32,
    durchgaenge: u32,
    apl: Option<String>,
    fund: (String, usize),
    fallrate: Option<u32>,
    katalog_nur: Option<String>,
    ohne_aufwertung: bool,
    vergleich: bool,
}

fn zahl<T: std::str::FromStr>(name: &str, wert: Option<String>) -> Result<T, String> {
    let wert = wert.ok_or_else(|| format!("{name} braucht einen Wert"))?;
    wert.parse()
        .map_err(|_| format!("{name}: \"{wert}\" ist keine passende Zahl"))
}

fn angaben() -> Result<Option<Angaben>, String> {
    let mut a = Angaben {
        laufbahnen: 10_000,
        startwert: 1,
        straenge: 1,
        siege: 100,
        kaempfe: 2000,
        durchgaenge: 1,
        apl: None,
        fund: ("focus-mix".to_owned(), 0),
        fallrate: None,
        katalog_nur: None,
        ohne_aufwertung: false,
        vergleich: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--laufbahnen" => a.laufbahnen = zahl(&arg, args.next())?,
            "--startwert" => a.startwert = zahl(&arg, args.next())?,
            "--straenge" => a.straenge = zahl(&arg, args.next())?,
            "--siege" => a.siege = zahl(&arg, args.next())?,
            "--kaempfe" => a.kaempfe = zahl(&arg, args.next())?,
            "--durchgaenge" => a.durchgaenge = zahl(&arg, args.next())?,
            "--apl" => a.apl = Some(args.next().ok_or("--apl braucht eine Datei")?),
            "--fund" => {
                let wert = args.next().ok_or("--fund braucht eine Kennung")?;
                let (id, guete) = match wert.split_once(':') {
                    Some((id, g)) => (id.to_owned(), zahl("--fund", Some(g.to_owned()))?),
                    None => (wert, 0),
                };
                a.fund = (id, guete);
            }
            "--fallrate" => a.fallrate = Some(zahl(&arg, args.next())?),
            "--katalog-nur" => {
                a.katalog_nur = Some(args.next().ok_or("--katalog-nur braucht eine Kennung")?);
            }
            "--ohne-aufwertung" => a.ohne_aufwertung = true,
            "--vergleich-aufwertung" => a.vergleich = true,
            anders => return Err(format!("unbekannte Angabe: {anders}; siehe --help")),
        }
    }
    if a.laufbahnen == 0 || a.siege == 0 || a.durchgaenge == 0 {
        return Err("--laufbahnen, --siege und --durchgaenge muessen groesser als 0 sein".into());
    }
    Ok(Some(a))
}

fn vorlage(id: &str) -> Result<usize, String> {
    KATALOG
        .iter()
        .position(|v| v.id == id)
        .ok_or_else(|| format!("keine Vorlage \"{id}\" in data/items.json"))
}

fn main() -> ExitCode {
    let a = match angaben() {
        Ok(Some(a)) => a,
        Ok(None) => {
            print!("{HILFE}");
            return ExitCode::SUCCESS;
        }
        Err(fehler) => {
            eprintln!("FEHLER: {fehler}");
            return ExitCode::from(2);
        }
    };
    match laufen(&a) {
        Ok(()) => ExitCode::SUCCESS,
        Err(fehler) => {
            eprintln!("FEHLER: {fehler}");
            ExitCode::from(1)
        }
    }
}

fn laufen(a: &Angaben) -> Result<(), String> {
    let (name, text) = match &a.apl {
        Some(pfad) => (
            pfad.clone(),
            std::fs::read_to_string(pfad).map_err(|e| format!("{pfad}: {e}"))?,
        ),
        None => ("laufbahn-referenz.apl".to_owned(), REFERENZ.to_owned()),
    };
    let apl = parse(AplName(&name), &text).map_err(|e| e.to_string())?;
    let referenz = Referenzspielweise::fuer_alle(apl);

    let mut regeln = Laufbahnregeln::default();
    if let Some(rate) = a.fallrate {
        if rate > 1000 {
            return Err(format!("--fallrate {rate}: erlaubt sind 0 bis 1000"));
        }
        regeln.fallrate_promille = rate;
    }
    if let Some(id) = &a.katalog_nur {
        regeln.katalog = vec![vorlage(id)?];
    }
    if a.ohne_aufwertung {
        regeln.aufwerten = Aufwerten::Nie;
    }
    let fund = Fundziel {
        vorlage: vorlage(&a.fund.0)?,
        mindest_guete: a.fund.1,
    };
    let auftrag = Auftrag {
        laufbahnen: a.laufbahnen,
        startwert: a.startwert,
        straenge: a.straenge,
        regeln: regeln.clone(),
        messung: Messauftrag {
            siege: a.siege,
            max_kaempfe: a.kaempfe,
            fund,
        },
        kampf: Kampfeinstellung {
            durchgaenge: a.durchgaenge,
            startwert: 0,
        },
    };
    let mut saetze = vec![regeln];
    let mut namen = vec![if a.ohne_aufwertung {
        "ohne Aufwertung"
    } else {
        "mit Aufwertung"
    }];
    if a.vergleich {
        let mut ohne = saetze[0].clone();
        ohne.aufwerten = if ohne.aufwerten == Aufwerten::Nie {
            Aufwerten::NiedrigsteZuerst
        } else {
            Aufwerten::Nie
        };
        namen.push(if ohne.aufwerten == Aufwerten::Nie {
            "ohne Aufwertung"
        } else {
            "mit Aufwertung"
        });
        saetze.push(ohne);
    }

    let beginn = Instant::now();
    let lauf = rechne(&auftrag, &saetze, &referenz).map_err(|e| e.to_string())?;
    let dauer = beginn.elapsed();

    let kampf = kampf_von(&auftrag, &referenz);
    for (index, regeln) in saetze.iter().enumerate() {
        if saetze.len() > 1 {
            println!("\n==== Regelsatz {}: {} ====", index + 1, namen[index]);
        }
        print!(
            "{}",
            bericht(&auftrag, regeln, &lauf.je_regelsatz[index], &name, kampf)
        );
    }
    print!("{}", vergleich(&namen, &lauf));
    eprintln!(
        "Wanduhr {:.3} s auf {} Strang/Straengen fuer {} Laufbahnen x {} Regelsatz/-saetze",
        dauer.as_secs_f64(),
        a.straenge,
        a.laufbahnen,
        saetze.len()
    );
    Ok(())
}
