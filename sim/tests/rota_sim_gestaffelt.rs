//! `rota-sim run --mode gestaffelt`, über das gebaute Programm (#915, AC3).
//!
//! Die Prüffälle in `tests/durchgangssteuerung.rs` prüfen den Läufer in der
//! Bibliothek. Diese hier prüfen, dass die Kommandozeile ihn **erreicht**: Ein
//! Modus, den nur die Bibliothek kennt, wäre für jede Sitzung, die
//! `rota-sim --help` liest, nicht vorhanden.
//!
//! # Woher die Sollwerte kommen
//!
//! Aus der Bibliothek, mit der Konfiguration, die `befehl_run` baut:
//! `BalanceConfig::default()` mit Boss, Startwert und PURITY, dazu die
//! mitgelieferte Rotation `step1-good` (`--search none`). Befund, Siege,
//! Niederlagen und Schaden zählt ein fester Lauf auf denselben Startwerten
//! nach, jede Zahl für sich. Die Wurfzahl kommt aus `run_gestaffelt`; deren
//! Zählung hält `balance::tests::die_wurfzahl_der_stichprobe_ist_die_des_festen_laeufers`
//! gegen den festen Läufer fest.

mod gemeinsam;
use gemeinsam::rufe;

use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::balance::{run, run_gestaffelt, siege, BalanceConfig, Staffelung};
use rota_sim::gear::WornValues;
use rota_sim::stichprobe::{gestaffelt, Befund};

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

/// Die Vorgabe von [`GESTAFFELT`], als Bibliotheksaufruf.
const VORGABE: Staffelung = Staffelung {
    schwelle: 0.9,
    alpha: 0.05,
    budget: 512,
};

/// Die Konfiguration, die `rota-sim run --boss boss-1-1` mit diesem Startwert
/// und dieser PURITY baut (`befehl_run`). `iterations` und `threads` liest
/// der gestaffelte Lauf nicht.
fn konfiguration(base_seed: u32, purity: u32) -> BalanceConfig {
    let mut worn = WornValues::none();
    worn.purity = purity;
    BalanceConfig {
        base_seed,
        boss: rota_sim::data::boss_by_id("boss-1-1").expect("den Boss gibt es"),
        worn,
        ..BalanceConfig::default()
    }
}

/// Die Rotation von `--search none`.
fn gute_liste() -> Apl {
    parse(AplName("step1-good"), include_str!("../apl/step1-good.apl"))
        .expect("die eingebaute Liste ist gültig")
}

/// Was ein fester Lauf auf den Startwerten der Stichprobe zählt.
struct Gegenzaehlung {
    befund: Befund,
    siege: u64,
    niederlagen: u64,
    schaden: u64,
}

/// Zählt auf einem festen Lauf über das ganze Budget nach.
///
/// Der Befund kommt aus `gestaffelt` über die Ausgänge des festen Laufs,
/// Versuch `i` mit dem Startwert `base_seed + i`. Siege, Niederlagen und
/// Schaden werden über die Kämpfe bis zu diesem Befund gezählt, **jede Zahl
/// für sich** — die Niederlagen also nicht als Kämpfe minus Siege, so wie das
/// Programm sie ausgibt.
///
/// Gesucht wird nach dem Startwert, nicht nach dem Index: `run` sortiert nach
/// Startwert, und am Überlauf steht der erste Versuch nicht mehr vorn.
fn gegenzaehlung(base_seed: u32, purity: u32) -> Gegenzaehlung {
    let config = konfiguration(base_seed, purity);
    let budget = u32::try_from(VORGABE.budget).expect("passt");
    let fest = run(
        &BalanceConfig {
            iterations: budget,
            threads: 1,
            ..config
        },
        &gute_liste(),
    );
    let probe = |i: u64| {
        let seed = base_seed.wrapping_add(u32::try_from(i).expect("passt"));
        fest.samples
            .iter()
            .find(|kandidat| kandidat.seed == seed)
            .unwrap_or_else(|| panic!("der feste Lauf hat keinen Durchgang mit Startwert {seed}"))
    };
    let gewonnen = |i: u64| siege(std::slice::from_ref(probe(i)), config.boss) == 1;

    let befund = gestaffelt(VORGABE.schwelle, VORGABE.alpha, VORGABE.budget, &gewonnen);
    let n = befund_teile(befund).1;
    Gegenzaehlung {
        befund,
        siege: (0..n).filter(|&i| gewonnen(i)).count() as u64,
        niederlagen: (0..n).filter(|&i| !gewonnen(i)).count() as u64,
        schaden: (0..n).map(|i| u64::from(probe(i).damage)).sum(),
    }
}

/// Der Name eines Befunds, wie ihn `--json` ausgibt, und seine Versuchszahl.
fn befund_teile(befund: Befund) -> (&'static str, u64) {
    match befund {
        Befund::Erfuellt { versuche } => ("Erfuellt", versuche),
        Befund::Verfehlt { versuche } => ("Verfehlt", versuche),
        Befund::Ungeklaert { versuche, .. } => ("Ungeklaert", versuche),
    }
}

/// Hält die JSON-Ausgabe Feld für Feld gegen die Gegenzählung.
fn pruefe_gegen(wert: &serde_json::Value, soll: &Gegenzaehlung) {
    let (name, versuche) = befund_teile(soll.befund);
    assert_eq!(wert["verdict"], name, "Befund: {wert}");
    assert_eq!(zahl(wert, "trials"), versuche, "Versuche: {wert}");
    assert_eq!(zahl(wert, "fights"), versuche, "Kämpfe: {wert}");
    assert_eq!(zahl(wert, "wins"), soll.siege, "Siege: {wert}");
    assert_eq!(
        zahl(wert, "losses"),
        soll.niederlagen,
        "Niederlagen: {wert}"
    );
    assert_eq!(zahl(wert, "damage"), soll.schaden, "Schaden: {wert}");
}

/// **Mit Krit läuft die Stichprobe — mit den Zahlen eines festen Laufs auf
/// denselben Startwerten.**
///
/// Eine frühere Fassung sicherte hier `wins + losses == fights` zu und einen
/// Befund aus der Menge der drei Namen. Beides konnte nicht scheitern: Das
/// Programm rechnet `losses` als `fights - wins`, und es kennt nur diese drei
/// Namen. Jetzt steht jede Zahl gegen eine Zählung aus der Bibliothek.
#[test]
fn mit_krit_faehrt_run_die_gestaffelte_stichprobe() {
    let mut argumente = GESTAFFELT.to_vec();
    argumente.extend(["--purity", "500", "--json"]);
    let wert = json_von(&argumente);

    assert_eq!(wert["selection"], "Gestaffelt", "{wert}");
    assert_eq!(zahl(&wert, "baseSeed"), 0, "{wert}");
    let soll = gegenzaehlung(0, 500);
    assert!(
        soll.niederlagen > 0,
        "Trägerzusicherung: mindestens ein Kampf geht verloren, sonst wäre eine \
         falsche Niederlagenzahl von null nicht zu unterscheiden"
    );
    pruefe_gegen(&wert, &soll);

    let lauf =
        run_gestaffelt(&konfiguration(0, 500), &gute_liste(), VORGABE).expect("gültige Vorgabe");
    assert!(lauf.wuerfe > 0, "Trägerzusicherung: es wurde gewürfelt");
    assert_eq!(zahl(&wert, "draws"), lauf.wuerfe, "Würfe: {wert}");
    println!("gestaffelt über die Kommandozeile: {wert}");
}

/// **`--seed` wirkt: Das Programm rechnet die Kämpfe ab diesem Startwert.**
///
/// Ab Startwert 0 fiele ein übergangenes `--seed` nicht auf, `base_seed + i`
/// ist dort `i`. Deshalb ab 1000 und am Überlauf des `u32`; die
/// Trägerzusicherung hält fest, dass ab 0 ein anderer Schaden herauskäme.
#[test]
fn mit_startwert_rechnet_run_die_kaempfe_ab_diesem_startwert() {
    let ab_null = gegenzaehlung(0, 500);
    for base_seed in [1000u32, u32::MAX - 50] {
        let startwert = base_seed.to_string();
        let mut argumente: Vec<&str> = GESTAFFELT.to_vec();
        argumente.extend(["--purity", "500", "--seed", startwert.as_str(), "--json"]);
        let wert = json_von(&argumente);

        assert_eq!(wert["selection"], "Gestaffelt", "{wert}");
        assert_eq!(
            zahl(&wert, "baseSeed"),
            u64::from(base_seed),
            "Startwert: {wert}"
        );
        let soll = gegenzaehlung(base_seed, 500);
        assert_ne!(
            soll.schaden, ab_null.schaden,
            "Trägerzusicherung: ab Startwert {base_seed} und ab 0 derselbe Schaden, \
             dann fiele ein übergangenes --seed nicht auf"
        );
        pruefe_gegen(&wert, &soll);

        let lauf = run_gestaffelt(&konfiguration(base_seed, 500), &gute_liste(), VORGABE)
            .expect("gültige Vorgabe");
        assert_eq!(zahl(&wert, "draws"), lauf.wuerfe, "Würfe: {wert}");
        println!("ab Startwert {base_seed}: {wert}");
    }
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

/// **Die Textausgabe nennt Weg, Befund, die gezählten Kämpfe und die Würfe.**
#[test]
fn die_textausgabe_nennt_weg_befund_und_kampfzahl() {
    let mut mit_json = GESTAFFELT.to_vec();
    mit_json.extend(["--purity", "500", "--json"]);
    let wert = json_von(&mit_json);
    let kaempfe = zahl(&wert, "fights");
    let wuerfe = zahl(&wert, "draws");

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
    // Die Wurfzahl der Textzeile ist die der JSON-Ausgabe; die wiederum hält
    // `mit_krit_faehrt_run_die_gestaffelte_stichprobe` gegen die Bibliothek.
    let gewuerfelt = stdout
        .lines()
        .find(|zeile| zeile.starts_with("Gewuerfelt"))
        .unwrap_or_else(|| panic!("keine Zeile 'Gewuerfelt' in:\n{stdout}"));
    assert!(
        gewuerfelt.ends_with(&format!(" {wuerfe}-mal")),
        "die Textzeile nennt nicht die {wuerfe} Würfe der JSON-Ausgabe: {gewuerfelt}"
    );
}

/// Breite der Bezeichnungsspalte der Textausgabe (`"Befund               "`).
const SPALTE: usize = 21;

/// **Jeder der vier Befund-Sätze steht lesbar da.**
///
/// Ein Lauf je Satz: erfüllt, verfehlt und ungeklärt gestaffelt, dazu der
/// exakte Weg ohne Zufall. Hinter der Bezeichnungsspalte steht nirgends ein
/// doppeltes Leerzeichen.
///
/// > Gefunden am fertigen Programm, nicht von einem Prüffall: In der ersten
/// > Fassung standen in drei Sätzen 14 Leerzeichen mitten im Text. Ein
/// > Einfügeskript hatte das `\` am Zeilenende des Rust-Strings als eigene
/// > Zeilenfortsetzung gelesen und mitsamt dem Umbruch entfernt — der String
/// > blieb gültig, und alle Prüffälle suchten nur nach Teilstrings.
#[test]
fn jeder_befund_satz_steht_lesbar_da() {
    let faelle: [(&[&str], &str); 4] = [
        (&["--purity", "500"], "ERFUELLT"),
        (
            &[
                "--purity",
                "500",
                "--threshold",
                "0.99",
                "--alpha",
                "0.01",
                "--budget",
                "2048",
            ],
            "VERFEHLT",
        ),
        (
            &[
                "--purity",
                "500",
                "--threshold",
                "0.99",
                "--alpha",
                "0.01",
                "--budget",
                "64",
            ],
            "UNGEKLAERT",
        ),
        (&[], "VERFEHLT — verloren"),
    ];
    for (zusatz, erwartet) in faelle {
        // Spätere Angaben überschreiben frühere, also gewinnt der Zusatz.
        let mut argumente = GESTAFFELT.to_vec();
        argumente.extend(zusatz);
        let (code, stdout, stderr) = rufe(&argumente);
        let aufruf = argumente.join(" ");
        assert_eq!(code, 0, "'{aufruf}': {stderr}");
        assert!(stderr.is_empty(), "'{aufruf}': {stderr}");
        let befund = stdout
            .lines()
            .find(|zeile| zeile.starts_with("Befund"))
            .unwrap_or_else(|| panic!("'{aufruf}': keine Befund-Zeile in:\n{stdout}"));
        assert!(
            befund.contains(erwartet),
            "'{aufruf}': erwartet {erwartet}, steht da: {befund}"
        );
        for zeile in stdout.lines() {
            // Zeichen, nicht Bytes: `get(SPALTE..)` lieferte vor einem
            // Mehrbyte-Zeichen wie `—` still `None`, und die Zeile fiele aus
            // der Prüfung.
            let wert: String = zeile.chars().skip(SPALTE).collect();
            assert!(
                !wert.contains("  "),
                "'{aufruf}': doppeltes Leerzeichen hinter der Spalte:\n{zeile}"
            );
        }
        println!("{befund}");
    }
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
