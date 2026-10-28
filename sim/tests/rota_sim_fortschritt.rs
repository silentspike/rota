//! `--progress` und `--jsonl` am gebauten Programm (#933, E15.2).
//!
//! Die Bibliothek prüft `tests/fortschritt.rs`. Hier geht es um das, was nur
//! am Programm sichtbar wird: dass die Kampfzeile und der Strom aus DERSELBEN
//! Quelle zählen, dass `bench` misst, was es meldet, und dass die Anzeige am
//! Bericht nichts ändert.

mod gemeinsam;
use gemeinsam::rufe;

use std::path::PathBuf;

/// Eine Datei für den Strom, je Prozess und Fall eigen.
fn stromdatei(fall: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rota-933-{}-{fall}.jsonl", std::process::id()))
}

fn meldungen(pfad: &PathBuf) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(pfad)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", pfad.display()));
    let _ = std::fs::remove_file(pfad);
    text.lines()
        .map(|z| serde_json::from_str(z).unwrap_or_else(|e| panic!("kein JSON ({e}): {z}")))
        .collect()
}

fn ende(alle: &[serde_json::Value]) -> &serde_json::Value {
    let letzte = alle.last().expect("mindestens eine Meldung");
    assert_eq!(letzte["event"], "end", "die letzte Meldung ist nicht 'end'");
    letzte
}

/// Die Zahl hinter „Kaempfe gerechnet:".
fn kampfzeile(text: &str) -> u64 {
    text.lines()
        .find(|z| z.starts_with("Kaempfe gerechnet:"))
        .and_then(|z| z.split_whitespace().last())
        .and_then(|z| z.parse().ok())
        .unwrap_or_else(|| panic!("keine Zeile 'Kaempfe gerechnet' in:\n{text}"))
}

/// **Kein zweiter Zähler.** Die Kampfzeile des Berichts und der Strom lesen
/// dasselbe Zählwerk.
///
/// Der Fall ist so gebaut, dass ein zweiter Zähler hier auffiele: Ohne Würfel
/// rechnet jede Messung EINEN Kampf, angefordert sind zwanzig. Der Zähler, den
/// `rota-sim` bis #933 führte, zählte die angeforderten — er hätte hier das
/// Zwanzigfache gemeldet.
#[test]
fn kampfzeile_und_strom_zaehlen_aus_derselben_quelle() {
    let pfad = stromdatei("quelle");
    let (code, stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "20",
        "--max-len",
        "2",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let schluss = ende(&alle);
    let kaempfe = schluss["metrics"]["fights"].as_u64().expect("Zahl");
    let kandidaten = schluss["metrics"]["candidates"]["done"]
        .as_u64()
        .expect("Zahl");
    assert_eq!(
        kampfzeile(&stdout),
        kaempfe,
        "Bericht und Strom nennen verschiedene Kampfzahlen"
    );
    assert_eq!(
        kaempfe, kandidaten,
        "ohne Würfel rechnet jede Messung genau einen Kampf"
    );
    assert_eq!(
        schluss["metrics"]["samples"].as_u64().expect("Zahl"),
        kandidaten * 20,
        "jede Messung liefert zwanzig Durchgänge"
    );
}

/// **`bench` misst, was es meldet.** Bis #933 griff dort die Reduktion aus
/// #908: Angefordert waren 2000 Kämpfe, gerechnet wurde einer, und die Rate
/// teilte 2000 durch die Zeit dieses einen.
#[test]
fn bench_rechnet_die_kaempfe_die_es_meldet() {
    let pfad = stromdatei("bench");
    let (code, stdout, stderr) = rufe(&[
        "bench",
        "--fights",
        "100",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let schluss = ende(&alle);
    assert_eq!(schluss["metrics"]["fights"], 100, "{schluss}");
    assert_eq!(kampfzeile(&stdout), 100);
    assert_eq!(schluss["metrics"]["paths"]["fixed_sample"]["done"], 1);
    assert_eq!(schluss["phase"], "bench");
}

/// **Die Anzeige ändert am Bericht nichts** — sie liest nur.
#[test]
fn progress_schreibt_auf_den_fehlerkanal_und_laesst_den_bericht_stehen() {
    let grund = [
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "10",
        "--max-len",
        "2",
        "--json",
    ];
    let (code, ohne, stderr_ohne) = rufe(&grund);
    assert_eq!(code, 0, "{stderr_ohne}");
    let mut mit_anzeige = grund.to_vec();
    mit_anzeige.push("--progress");
    let (code, mit, stderr_mit) = rufe(&mit_anzeige);
    assert_eq!(code, 0, "{stderr_mit}");

    // Verglichen wird ALLES außer der Wanduhr. Ein einzelnes Feld genügte
    // nicht: Die Suche gibt kein `result` aus, und zwei fehlende Felder wären
    // gleich, ohne dass etwas verglichen wurde.
    let ohne_uhr = |text: &str| {
        let mut wert: serde_json::Value = serde_json::from_str(text.trim()).expect("JSON");
        let objekt = wert.as_object_mut().expect("ein JSON-Objekt");
        assert!(
            objekt.remove("elapsedMs").is_some(),
            "keine Wanduhr im Bericht"
        );
        assert!(
            objekt.contains_key("measured"),
            "Zusicherung: ein Suchbericht"
        );
        wert
    };
    assert_eq!(
        ohne_uhr(&ohne),
        ohne_uhr(&mit),
        "mit --progress kam ein anderes Ergebnis heraus"
    );
    for angabe in ["Kandidaten", "Durchsatz", "Restzeit", "Engpass", "Bestes"] {
        assert!(
            stderr_mit.contains(angabe),
            "'{angabe}' fehlt in der Anzeige:\n{stderr_mit}"
        );
    }
    assert!(
        stderr_ohne.is_empty(),
        "ohne --progress schrieb der Lauf auf den Fehlerkanal:\n{stderr_ohne}"
    );
}

/// **Die Schranken der Suche halten in jeder Meldung** (AC3, dritte Zeile).
#[test]
fn die_schranken_der_suche_halten_in_jeder_meldung() {
    let pfad = stromdatei("schranken");
    let (code, _stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "10",
        "--max-len",
        "3",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let gesamt = ende(&alle)["metrics"]["candidates"]["done"]
        .as_u64()
        .expect("Zahl");
    let mut geprueft = 0;
    for m in &alle {
        let menge = &m["metrics"]["candidates"]["amount"];
        if menge["kind"] == "bounded" {
            let min = menge["min"].as_u64().expect("Zahl");
            let max = menge["max"].as_u64().expect("Zahl");
            assert!(
                min <= gesamt && gesamt <= max,
                "Meldung {}: {min}..={max} schließt {gesamt} nicht ein",
                m["host_sequence"]
            );
            geprueft += 1;
        }
    }
    assert!(
        geprueft > 0,
        "keine Meldung mit Schranken — der Fall prüfte nichts"
    );
}

/// **Eine Runde überschreitet ihren Plan nie** — in keiner Meldung.
///
/// Befund des Orchestrators auf 2361f2c1: Die end-Meldung zeigte `round_done
/// 361` gegen `round_planned 360`. Die Messung der gefundenen Liste nach der
/// letzten Runde stand in beiden Grenzen der Menge, aber nicht im Rundenplan,
/// und wurde trotzdem der letzten Runde zugerechnet.
#[test]
fn in_jeder_meldung_liegt_die_runde_in_ihrem_plan() {
    let pfad = stromdatei("rundenplan");
    let (code, _stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "20",
        "--max-len",
        "4",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let mut mit_runde = 0;
    for m in &alle {
        let c = &m["metrics"]["candidates"];
        let (Some(erledigt), Some(geplant)) =
            (c["round_done"].as_u64(), c["round_planned"].as_u64())
        else {
            continue;
        };
        assert!(
            erledigt <= geplant,
            "Meldung {} ({}): Runde {} hat {erledigt} von {geplant} geplanten Auswertungen",
            m["host_sequence"],
            m["event"],
            m["metrics"]["round"]
        );
        mit_runde += 1;
    }
    assert!(
        ende(&alle)["metrics"]["round"].as_u64().is_some(),
        "Zusicherung: die end-Meldung trägt eine Runde"
    );
    assert!(
        mit_runde > 0,
        "keine Meldung mit Runde — der Fall prüfte nichts"
    );
}

/// Die Zahl hinter der Zeilenmarke `marke` im Textbericht.
fn zahl_hinter(text: &str, marke: &str) -> u64 {
    text.lines()
        .find(|z| z.split_whitespace().next() == Some(marke))
        .and_then(|z| z.split_whitespace().nth(1))
        .and_then(|z| z.parse().ok())
        .unwrap_or_else(|| panic!("keine Zeile '{marke}' mit Zahl in:\n{text}"))
}

/// **Messungen der Suche und Auswertungen sind zwei Größen, und der Bericht
/// nennt beide.**
///
/// Befund des Orchestrators: „Messungen 1615" im Bericht gegen 1617 in der
/// Anzeige und in „Kaempfe gerechnet". Die zwei sind die Start- und die
/// Fundliste: `SearchResult::measured` zählt die Züge der Suche, das Zählwerk
/// jede abgeschlossene Auswertung. Ohne Würfel kostet jede Auswertung genau
/// einen Kampf — daher stimmen Auswertungen, Strom und Kampfzeile überein.
#[test]
fn bericht_nennt_messungen_und_auswertungen_getrennt() {
    let pfad = stromdatei("auswertungen");
    let (code, stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "20",
        "--max-len",
        "3",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let strom = ende(&alle)["metrics"]["candidates"]["done"]
        .as_u64()
        .expect("Zahl");
    let messungen = zahl_hinter(&stdout, "Messungen");
    let auswertungen = zahl_hinter(&stdout, "Auswertungen");
    assert_eq!(
        auswertungen, strom,
        "Bericht und Strom nennen verschiedene Auswertungszahlen"
    );
    assert_eq!(
        auswertungen,
        messungen + 2,
        "Auswertungen sind die Messungen der Suche plus Start- und Fundliste"
    );
    assert_eq!(
        kampfzeile(&stdout),
        auswertungen,
        "ohne Würfel ein Kampf je Auswertung"
    );

    // Dieselbe Beziehung im JSON-Bericht.
    let (code, json, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "20",
        "--max-len",
        "3",
        "--json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let wert: serde_json::Value = serde_json::from_str(json.trim()).expect("JSON");
    let gemessen = wert["measured"].as_u64().expect("measured");
    assert_eq!(wert["evaluations"].as_u64(), Some(gemessen + 2), "{wert}");
}

/// **Der gestaffelte Lauf meldet seinen Weg und seine Kämpfe.**
#[test]
fn gestaffelt_meldet_den_gestaffelten_weg() {
    let pfad = stromdatei("gestaffelt");
    let (code, stdout, stderr) = rufe(&[
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
        "--purity",
        "500",
        "--json",
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let bericht: serde_json::Value = serde_json::from_str(stdout.trim()).expect("JSON");
    let alle = meldungen(&pfad);
    let schluss = ende(&alle);
    assert_eq!(schluss["metrics"]["fights"], bericht["fights"]);
    assert_eq!(schluss["metrics"]["paths"]["sequential"]["done"], 1);
    assert_eq!(schluss["phase"], "gestaffelt");
}

/// **Mit Steuerdatei steht die Drosselung im Strom.**
#[test]
fn mit_steuerdatei_steht_die_drosselung_im_strom() {
    let steuerung =
        std::env::temp_dir().join(format!("rota-933-{}-steuer.json", std::process::id()));
    std::fs::write(
        &steuerung,
        "{\"cpu_threads\":1,\"cpu_percent\":100,\"paused\":false}",
    )
    .expect("Steuerdatei");
    let pfad = stromdatei("steuer");
    let (code, _stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--search",
        "none",
        "--iterations",
        "20",
        "--threads",
        "2",
        "--purity",
        "500",
        "--control-file",
        steuerung.to_str().expect("Pfad"),
        "--jsonl",
        pfad.to_str().expect("Pfad"),
    ]);
    let _ = std::fs::remove_file(&steuerung);
    assert_eq!(code, 0, "{stderr}");
    let alle = meldungen(&pfad);
    let drossel = &ende(&alle)["metrics"]["throttle"];
    assert_eq!(drossel["cpu_threads"], 1, "{drossel}");
    assert_eq!(drossel["cpu_percent"], 100, "{drossel}");
    assert_eq!(ende(&alle)["metrics"]["fights"], 20);
}
