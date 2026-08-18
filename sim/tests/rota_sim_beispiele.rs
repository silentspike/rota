//! Jedes Beispiel aus der Hilfe wird ausgefuehrt (#968, AC4).
//!
//! # Warum das der eigentliche Test dieses Issues ist
//!
//! Eine Hilfe altert still. Wer eine Angabe umbenennt, aendert den Zerleger und
//! vergisst den Hilfetext — und dann steht dort ein Befehl, der nicht mehr
//! laeuft. Das faellt niemandem auf, weil eine Hilfe niemand ausfuehrt.
//!
//! Dieser Test fuehrt sie aus. Ein Beispiel, das nicht mehr geht, macht die
//! Pruefstrecke rot.
//!
//! # Wie ein kaputtes Beispiel erkannt wird
//!
//! **Am leeren Fehlerkanal, nicht am Rueckgabewert.** Die vier noch nicht
//! gebauten Unterbefehle geben absichtlich einen Fehlschlag zurueck — ein
//! Skript soll einen Platzhalter nicht fuer eine Messung halten. Wer nur den
//! Rueckgabewert prueft, muesste sie ausnehmen, und die Ausnahme waere genau
//! das Loch, durch das ein echter Fehler schluepft.
//!
//! Das Programm schreibt jede Fehlermeldung auf den Fehlerkanal und beginnt sie
//! mit `FEHLER`. Ein Beispiel, dessen Angabe es nicht mehr gibt, landet dort —
//! egal welchen Rueckgabewert es hat.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Das gebaute Programm. Cargo setzt die Variable fuer Integrationstests.
const PROGRAMM: &str = env!("CARGO_BIN_EXE_rota-sim");

/// Die Unterbefehle, deren Hilfe geprueft wird — plus die Wurzel.
///
/// **Aus der Wurzelhilfe gelesen waere schoener**, ist aber die schwaechere
/// Pruefung: Verschwindet ein Unterbefehl aus der Wurzelhilfe, verschwindet er
/// dann auch aus dem Test, und beide bleiben gruen. Die Liste hier ist der
/// unabhaengige zweite Zeuge.
const UNTERBEFEHLE: [&str; 7] = [
    "run", "bench", "verify", "hosts", "status", "explain", "data",
];

/// Das Wurzelverzeichnis des Projekts.
///
/// Cargo startet Integrationstests im Paketverzeichnis (`sim/`), die Beispiele
/// aus der Hilfe gelten aber vom Projekt-Wurzelverzeichnis aus — dort steht
/// `sim/apl/step1-good.apl`. Ohne diesen Wechsel scheitert genau das eine
/// Beispiel, das eine Datei nennt, und zwar mit einer Meldung ueber einen
/// Pfad, der voellig richtig ist.
fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sim/ hat ein Elternverzeichnis")
        .to_path_buf()
}

/// Ruft das Programm mit den Argumenten und liefert (Rueckgabewert, stdout, stderr).
fn rufe(argumente: &[&str]) -> (i32, String, String) {
    let ausgabe = Command::new(PROGRAMM)
        .args(argumente)
        .current_dir(wurzel())
        .output()
        .unwrap_or_else(|fehler| panic!("{PROGRAMM} liess sich nicht starten: {fehler}"));
    (
        ausgabe.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&ausgabe.stdout).into_owned(),
        String::from_utf8_lossy(&ausgabe.stderr).into_owned(),
    )
}

/// Ist die Zeile eine Abschnittsueberschrift?
///
/// Zweite Umsetzung derselben Regel wie in `src/bin/rota-sim.rs` — ein `[[bin]]`
/// laesst sich nicht einbinden. Siehe dort die Begruendung.
fn ist_ueberschrift(zeile: &str) -> bool {
    !zeile.is_empty()
        && !zeile.starts_with(' ')
        && zeile
            .chars()
            .all(|z| z.is_ascii_uppercase() || z == ' ' || z == '_')
}

/// Die Beispiele aus dem Abschnitt `BEISPIELE` einer Hilfe.
fn beispiele_aus(hilfe: &str) -> Vec<String> {
    let mut gefunden = Vec::new();
    let mut im_block = false;
    for zeile in hilfe.lines() {
        if ist_ueberschrift(zeile) {
            im_block = zeile == "BEISPIELE";
            continue;
        }
        if im_block {
            let inhalt = zeile.trim();
            if inhalt.starts_with("rota-sim ") {
                gefunden.push(inhalt.to_owned());
            }
        }
    }
    gefunden
}

/// Zerlegt ein Beispiel in seine Argumente, ohne den Programmnamen.
fn argumente_von(beispiel: &str) -> Vec<String> {
    beispiel
        .split_whitespace()
        .skip(1)
        .map(ToOwned::to_owned)
        .collect()
}

/// Holt die Hilfe eines Unterbefehls — oder die der Wurzel bei `None`.
fn hilfe_von(unterbefehl: Option<&str>) -> String {
    let argumente: Vec<&str> = match unterbefehl {
        Some(name) => vec![name, "--help"],
        None => vec!["--help"],
    };
    let (code, stdout, stderr) = rufe(&argumente);
    assert_eq!(
        code,
        0,
        "'{}' gab {code} zurueck statt null:\n{stderr}",
        argumente.join(" ")
    );
    assert!(
        stderr.is_empty(),
        "'{}' schrieb auf den Fehlerkanal:\n{stderr}",
        argumente.join(" ")
    );
    stdout
}

#[test]
fn jede_hilfe_zeigt_mindestens_zwei_beispiele() {
    let mut gesamt = 0usize;
    for unterbefehl in std::iter::once(None).chain(UNTERBEFEHLE.iter().map(|n| Some(*n))) {
        let hilfe = hilfe_von(unterbefehl);
        let beispiele = beispiele_aus(&hilfe);
        let name = unterbefehl.unwrap_or("(Wurzel)");
        assert!(
            beispiele.len() >= 2,
            "die Hilfe von {name} zeigt nur {} Beispiel(e): {beispiele:?}",
            beispiele.len()
        );
        gesamt += beispiele.len();
    }
    // ZUSICHERUNG GEGEN EINEN FILTER, DER NICHTS TRIFFT. Acht Hilfen mit je
    // mindestens zwei Beispielen sind sechzehn; findet der Filter weniger, hat
    // er nicht gefunden, sondern uebersehen — und dann liefe der Test unten
    // gruen, ohne einen einzigen Befehl auszufuehren.
    assert!(
        gesamt >= 16,
        "ueber alle Hilfen nur {gesamt} Beispiele gefunden — der Filter trifft nicht"
    );
    println!("{gesamt} Beispiele in acht Hilfen gefunden");
}

#[test]
fn jedes_beispiel_aus_der_hilfe_laeuft() {
    let mut ausgefuehrt = 0usize;
    for unterbefehl in std::iter::once(None).chain(UNTERBEFEHLE.iter().map(|n| Some(*n))) {
        let hilfe = hilfe_von(unterbefehl);
        let name = unterbefehl.unwrap_or("(Wurzel)");
        for beispiel in beispiele_aus(&hilfe) {
            let argumente = argumente_von(&beispiel);
            let als_str: Vec<&str> = argumente.iter().map(String::as_str).collect();
            let (code, _stdout, stderr) = rufe(&als_str);
            assert!(
                stderr.is_empty(),
                "das Beispiel aus der Hilfe von {name} schrieb auf den Fehlerkanal:\n  \
                 {beispiel}\n{stderr}"
            );
            assert_ne!(
                code, 101,
                "das Beispiel aus der Hilfe von {name} ist abgestuerzt:\n  {beispiel}"
            );
            ausgefuehrt += 1;
        }
    }
    assert!(
        ausgefuehrt >= 16,
        "nur {ausgefuehrt} Beispiele ausgefuehrt — zu wenige, um etwas zu belegen"
    );
    println!("{ausgefuehrt} Beispiele ausgefuehrt");
}

#[test]
fn ein_kaputtes_beispiel_wuerde_auffallen() {
    // DIE GEGENPROBE ZUM TEST DARUEBER. Sie ist kein Beiwerk: Ein Test, der
    // jedes Beispiel durchwinkt, saehe genauso gruen aus wie einer, der prueft.
    //
    // Nachgestellt wird der wahrscheinlichste Alterungsfall — eine Angabe, die
    // es nicht mehr gibt, weil sie umbenannt wurde.
    let (code, _stdout, stderr) = rufe(&["run", "--boss", "boss-1-1", "--iterationen", "20"]);
    assert!(
        !stderr.is_empty(),
        "eine unbekannte Angabe blieb still — dann faengt der Test oben nichts"
    );
    assert!(
        stderr.starts_with("FEHLER"),
        "die Meldung faengt nicht mit FEHLER an:\n{stderr}"
    );
    assert_ne!(code, 0, "eine unbekannte Angabe gab null zurueck");

    // Und derselbe Aufruf OHNE den Tippfehler muss sauber durchlaufen — sonst
    // belegt die Gegenprobe nur, dass irgendetwas an dem Befehl kaputt ist.
    let (code, _stdout, stderr) = rufe(&["run", "--boss", "boss-1-1", "--iterations", "20"]);
    assert!(stderr.is_empty(), "der richtige Aufruf meldete:\n{stderr}");
    assert_eq!(code, 0, "der richtige Aufruf gab {code} zurueck");
}

#[test]
fn jede_fehlermeldung_nennt_einen_ausweg() {
    // AC3. Geprueft werden alle Wege, auf denen das Programm ablehnt — jeder
    // muss die Form 'FEHLER ... Alle anzeigen: <Befehl>' tragen.
    let faelle: [(&[&str], &str); 6] = [
        (&["gibtsnicht"], "rota-sim --help"),
        (&["run", "--boss", "boss-2-5"], "rota-sim data bosses"),
        (&["run", "--gibtsnicht"], "rota-sim run --help"),
        (&["bench", "--gibtsnicht"], "rota-sim bench --help"),
        (&["data", "gibtsnicht"], "rota-sim data --help"),
        (&["run", "--boss"], "rota-sim run --help"),
    ];
    for (argumente, ausweg) in faelle {
        let (code, _stdout, stderr) = rufe(argumente);
        let aufruf = argumente.join(" ");
        assert_ne!(code, 0, "'{aufruf}' gab null zurueck");
        assert!(
            stderr.starts_with("FEHLER"),
            "'{aufruf}' faengt nicht mit FEHLER an:\n{stderr}"
        );
        assert!(
            stderr.contains(ausweg),
            "'{aufruf}' nennt den Ausweg '{ausweg}' nicht:\n{stderr}"
        );
    }
}

#[test]
fn ein_tippfehler_im_bossnamen_schlaegt_den_nachbarn_vor() {
    // Die Kontrollprobe, die das Issue ausdruecklich verlangt.
    let (_code, _stdout, stderr) = rufe(&["run", "--boss", "boss-2-5"]);
    assert!(
        stderr.contains("Meintest du 'boss-2-4'?"),
        "kein oder der falsche Vorschlag:\n{stderr}"
    );
}

#[test]
fn ein_tippfehler_im_befehl_schlaegt_den_nachbarn_vor() {
    let (_code, _stdout, stderr) = rufe(&["bnech"]);
    assert!(
        stderr.contains("Meintest du 'bench'?"),
        "kein oder der falsche Vorschlag:\n{stderr}"
    );
}

#[test]
fn die_platzhalter_nennen_beim_aufruf_keine_zahl() {
    // Uebernommen aus #897 und hier verschaerft: Auch die HILFE der Platzhalter
    // bleibt ohne Ziffer, damit die Zusicherung nicht zwischen zwei Aufrufarten
    // unterscheiden muss.
    for name in ["verify", "hosts", "status", "explain"] {
        let (code, stdout, stderr) = rufe(&[name]);
        assert_eq!(code, 1, "'{name}' gab {code} zurueck statt eins");
        assert!(stderr.is_empty(), "'{name}' schrieb auf den Fehlerkanal");
        assert!(
            !stdout.chars().any(|z| z.is_ascii_digit()),
            "'{name}' nennt eine Ziffer:\n{stdout}"
        );

        let (code, stdout, _stderr) = rufe(&[name, "--help"]);
        assert_eq!(code, 0, "'{name} --help' gab {code} zurueck statt null");
        assert!(
            !stdout.chars().any(|z| z.is_ascii_digit()),
            "'{name} --help' nennt eine Ziffer:\n{stdout}"
        );
    }
}
