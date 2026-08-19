//! Der Selbsttest prüft vier Dinge — und muss bei jedem einzeln rot werden (#970).
//!
//! # Warum AC4 die Abnahme ist
//!
//! Ein Selbsttest, bei dem nur eine Prüfung nachweislich anschlägt, ist zu drei
//! Vierteln ungeprüft. Er sähe genauso grün aus wie einer, der alles fängt.
//!
//! Die vier Fehlschläge lassen sich nicht alle im Test erzeugen — zwei hängen
//! an Werten, die zur Bauzeit einkompiliert werden. Was hier steht, ist der
//! Teil, der sich **ohne Mutation am Quelltext** prüfen lässt; die übrigen sind
//! als Mutationsprobe gefahren und im PR belegt.

mod gemeinsam;
use gemeinsam::rufe;

use std::sync::OnceLock;

/// Die Ausgabe von `rota-sim verify` — **einmal geholt, von allen geteilt**.
///
/// Der Selbsttest spielt fünfundsechzig aufgezeichnete Runden ab und braucht
/// dafür im Debug-Bau rund zweiundzwanzig Sekunden. Fünf Tests prüfen
/// verschiedene Stellen derselben Ausgabe; sie einzeln zu holen kostete
/// fünfmal so lange, ohne mehr zu prüfen.
///
/// Gemessen: 55 s vorher, mit dieser Zwischenspeicherung deutlich darunter.
fn verify_ausgabe() -> &'static (i32, String, String) {
    static EINMAL: OnceLock<(i32, String, String)> = OnceLock::new();
    EINMAL.get_or_init(|| rufe(&["verify"]))
}

#[test]
fn der_selbsttest_faehrt_vier_pruefungen_in_einem_aufruf() {
    // AC1.
    let (code, stdout, stderr) = verify_ausgabe();
    let (code, stdout, stderr) = (*code, stdout.as_str(), stderr.as_str());
    assert!(
        stderr.is_empty(),
        "verify schrieb auf den Fehlerkanal:\n{stderr}"
    );
    for pruefung in [
        "Paritaet gegen das Spiel",
        "Trace-Hashes",
        "Determinismus ueber Straenge",
        "Bitgleichheit ueber Rechner",
    ] {
        assert!(
            stdout.contains(pruefung),
            "die Pruefung '{pruefung}' fehlt in:\n{stdout}"
        );
    }
    assert_eq!(code, 0, "auf gesundem Stand muss er bestehen:\n{stdout}");
}

#[test]
fn das_ergebnis_ist_eindeutig_und_nennt_die_gescheiterte_pruefung() {
    // AC2. Auf gesundem Stand: genau eine Ergebniszeile, und sie ist positiv.
    let stdout = verify_ausgabe().1.as_str();
    let ergebniszeilen: Vec<&str> = stdout
        .lines()
        .filter(|z| z.starts_with("ERGEBNIS:"))
        .collect();
    assert_eq!(
        ergebniszeilen.len(),
        1,
        "es gibt {} Ergebniszeilen statt einer:\n{stdout}",
        ergebniszeilen.len()
    );
    assert_eq!(ergebniszeilen[0], "ERGEBNIS: BESTANDEN");
    // Und jede Prüfung trägt eine der drei Marken — nicht bloß Fließtext.
    let marken = stdout
        .lines()
        .filter(|z| {
            z.starts_with("BESTANDEN")
                || z.starts_with("GESCHEITERT")
                || z.starts_with("UEBERSPRUNGEN")
        })
        .count();
    assert_eq!(
        marken, 4,
        "vier Marken erwartet, {marken} gefunden:\n{stdout}"
    );
}

#[test]
fn die_bitgleichheit_ueberspringt_einen_fehlenden_rechner_statt_zu_scheitern() {
    // AC5, erster Fall: gar kein Rechner angegeben.
    let (code, stdout) = (verify_ausgabe().0, verify_ausgabe().1.as_str());
    assert_eq!(code, 0, "ohne zweiten Rechner darf er nicht scheitern");
    assert!(
        stdout.contains("UEBERSPRUNGEN") && stdout.contains("kein zweiter Rechner"),
        "die Bitgleichheit wurde nicht als uebersprungen ausgewiesen:\n{stdout}"
    );

    // AC5, zweiter Fall: ein Rechner, den es nicht gibt. Auch das ist ein
    // Ueberspringen und kein Fehlschlag — sonst waere jede Messung ohne Netz
    // ein roter Selbsttest.
    // **Eine IP aus TEST-NET-1 (RFC 5737), kein Name.** Ein unerreichbarer
    // NAME kostete gemessen achtzig Sekunden — die Namensaufloesung laeuft vor
    // dem ConnectTimeout und fragt jeden Nameserver einzeln. Eine Adresse, die
    // garantiert nicht geroutet wird, scheitert nach dem ConnectTimeout.
    let (code, stdout, _) = rufe(&["verify", "--peer", "192.0.2.1"]);
    assert_eq!(
        code, 0,
        "ein unerreichbarer Rechner liess den Selbsttest scheitern:\n{stdout}"
    );
    assert!(
        stdout.contains("UEBERSPRUNGEN"),
        "der unerreichbare Rechner wurde nicht uebersprungen:\n{stdout}"
    );
}

#[test]
fn der_fingerabdruck_ist_eine_zahl_und_bleibt_gleich() {
    // Die Gegenstelle liest genau diese Ausgabe. Ändert sich ihre Form, kann
    // die Bitgleichheitsprüfung sie nicht mehr deuten — und würde still
    // übersprungen statt zu prüfen.
    let (code, erst, _) = rufe(&["verify", "--print-fingerprint"]);
    assert_eq!(code, 0);
    let erst = erst.trim().to_owned();
    assert!(
        erst.starts_with("0x") && erst.len() == 10,
        "der Fingerabdruck hat nicht die Form 0xXXXXXXXX: {erst:?}"
    );
    assert!(
        u32::from_str_radix(erst.trim_start_matches("0x"), 16).is_ok(),
        "der Fingerabdruck ist keine Zahl: {erst:?}"
    );

    // **Und er muss derselbe sein wie der, den der volle Selbsttest nennt.**
    //
    // Das prüft mehr als zwei Abfragen desselben Befehls und kostet weniger:
    // Ein zweiter `--print-fingerprint`-Aufruf spielt alle 65 Runden erneut ab
    // (rund zwanzig Sekunden im Debug-Bau) und belegt nur, dass derselbe Weg
    // zweimal dasselbe liefert. Der Vergleich mit der `verify`-Ausgabe belegt,
    // dass ZWEI Wege übereinstimmen — und genau darauf verlässt sich die
    // Gegenstelle.
    let aus_selbsttest = verify_ausgabe()
        .1
        .lines()
        .find_map(|z| z.split("Fingerabdruck ").nth(1))
        .map(|rest| {
            rest.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .expect("der Selbsttest nennt seinen Fingerabdruck");
    assert_eq!(
        erst, aus_selbsttest,
        "'--print-fingerprint' und der Selbsttest nennen verschiedene Zahlen"
    );
}

#[test]
fn der_selbsttest_sagt_was_er_nicht_prueft() {
    // AC3: Er muss ohne Kenntnis der Messumgebung deutbar sein. Dazu gehört,
    // dass er seine eigenen Grenzen nennt — sonst liest ihn jemand als
    // vollständige Prüfung.
    let stdout = verify_ausgabe().1.as_str();
    for satz in [
        "WAS DIESER SELBSTTEST NICHT IST",
        "parity_recording",
        "keine Abnahme",
    ] {
        assert!(stdout.contains(satz), "'{satz}' fehlt in:\n{stdout}");
    }
    // Und die Zahlen, die den Umfang beziffern: geprüfte von vorhandenen.
    assert!(
        stdout.contains("der 86 Dateien"),
        "die Bezugsgroesse fehlt — 'x geprueft' ohne 'von wie vielen':\n{stdout}"
    );
}

#[test]
fn eine_unbekannte_angabe_wird_abgelehnt() {
    let (code, _, stderr) = rufe(&["verify", "--gibtsnicht"]);
    assert_ne!(code, 0);
    assert!(stderr.starts_with("FEHLER"), "{stderr}");
    assert!(stderr.contains("rota-sim verify --help"), "{stderr}");
}
