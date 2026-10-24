//! `rota-sim run --mode gestaffelt`, über das gebaute Programm (#915, AC3).
//!
//! Die Prüffälle in `tests/durchgangssteuerung.rs` prüfen den Läufer in der
//! Bibliothek. Diese hier prüfen, dass die Kommandozeile ihn **erreicht**: Ein
//! Modus, den nur die Bibliothek kennt, wäre für jede Sitzung, die
//! `rota-sim --help` liest, nicht vorhanden.

mod gemeinsam;
use gemeinsam::rufe;

/// Ruft das Programm und liest seine JSON-Ausgabe.
fn json_von(argumente: &[&str]) -> serde_json::Value {
    let (code, stdout, stderr) = rufe(argumente);
    let aufruf = argumente.join(" ");
    assert_eq!(code, 0, "'{aufruf}' gab {code} zurück:\n{stderr}");
    assert!(
        stderr.is_empty(),
        "'{aufruf}' schrieb auf den Fehlerkanal:\n{stderr}"
    );
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|fehler| panic!("'{aufruf}' gab kein JSON aus ({fehler}):\n{stdout}"))
}

fn zahl(wert: &serde_json::Value, feld: &str) -> u64 {
    wert[feld]
        .as_u64()
        .unwrap_or_else(|| panic!("das Feld '{feld}' fehlt oder ist keine Zahl: {wert}"))
}

const GESTAFFELT: [&str; 13] = [
    "run",
    "--boss",
    "boss-1-1",
    "--search",
    "none",
    "--mode",
    "gestaffelt",
    "--threshold",
    "0.9",
    "--alpha",
    "0.05",
    "--budget",
    "512",
];

/// **Mit Krit läuft die Stichprobe — und ihre Zahlen passen zueinander.**
#[test]
fn mit_krit_faehrt_run_die_gestaffelte_stichprobe() {
    let mut argumente = GESTAFFELT.to_vec();
    argumente.extend(["--purity", "500", "--json"]);
    let wert = json_von(&argumente);

    assert_eq!(wert["selection"], "Gestaffelt", "{wert}");
    assert!(
        zahl(&wert, "draws") > 0,
        "Trägerzusicherung: gewürfelt: {wert}"
    );
    let kaempfe = zahl(&wert, "fights");
    assert_eq!(
        zahl(&wert, "trials"),
        kaempfe,
        "Versuche und Kämpfe: {wert}"
    );
    assert!((1..=512).contains(&kaempfe), "höchstens das Budget: {wert}");
    assert_eq!(
        zahl(&wert, "wins") + zahl(&wert, "losses"),
        kaempfe,
        "Siege und Niederlagen ergeben die Kämpfe: {wert}"
    );
    assert!(zahl(&wert, "damage") > 0, "Schaden gemacht: {wert}");
    let urteil = wert["verdict"].as_str().expect("verdict ist Text");
    assert!(
        ["Erfuellt", "Verfehlt", "Ungeklaert"].contains(&urteil),
        "unbekannter Befund: {wert}"
    );
    println!("gestaffelt über die Kommandozeile: {wert}");
}

/// **Ohne Krit genügt ein Kampf — dieselbe Angabe, der andere Weg.**
#[test]
fn ohne_krit_waehlt_run_einen_kampf() {
    let mut argumente = GESTAFFELT.to_vec();
    argumente.push("--json");
    let wert = json_von(&argumente);

    assert_eq!(wert["selection"], "EinDurchgang", "{wert}");
    assert_eq!(zahl(&wert, "fights"), 1, "{wert}");
    assert_eq!(zahl(&wert, "draws"), 0, "{wert}");
    assert_eq!(wert["purity"], 0, "{wert}");
}

/// **Die Textausgabe nennt Weg, Befund und die gezählten Kämpfe.**
#[test]
fn die_textausgabe_nennt_weg_befund_und_kampfzahl() {
    let mut mit_json = GESTAFFELT.to_vec();
    mit_json.extend(["--purity", "500", "--json"]);
    let kaempfe = zahl(&json_von(&mit_json), "fights");

    let mut argumente = GESTAFFELT.to_vec();
    argumente.extend(["--purity", "500"]);
    let (code, stdout, stderr) = rufe(&argumente);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    for zeile in ["Weg", "gestaffelt", "Befund", "Siege", "Gewuerfelt"] {
        assert!(stdout.contains(zeile), "'{zeile}' fehlt in:\n{stdout}");
    }
    assert!(
        stdout.contains(&format!("Kaempfe gerechnet: {kaempfe}")),
        "der Kampfzähler zeigt nicht die {kaempfe} Kämpfe des Laufs:\n{stdout}"
    );
}

/// **Eine unvollständige Vorgabe wird abgelehnt, mit Ausweg.**
#[test]
fn eine_unvollstaendige_vorgabe_wird_abgelehnt() {
    let (code, _stdout, stderr) = rufe(&[
        "run",
        "--search",
        "none",
        "--mode",
        "gestaffelt",
        "--threshold",
        "0.9",
    ]);
    assert_ne!(code, 0, "eine unvollständige Vorgabe gab null zurück");
    assert!(stderr.starts_with("FEHLER"), "{stderr}");
    assert!(
        stderr.contains("--alpha") && stderr.contains("--budget"),
        "die fehlenden Angaben werden nicht genannt:\n{stderr}"
    );
    assert!(
        stderr.contains("rota-sim run --help"),
        "kein Ausweg:\n{stderr}"
    );
}
