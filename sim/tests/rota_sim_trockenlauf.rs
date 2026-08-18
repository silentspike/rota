//! Der Trockenlauf sagt voraus, was ein Lauf kostet — und rechnet nichts (#969).
//!
//! # Was hier geprüft wird und was nicht
//!
//! **Geprüft**: dass die vorhergesagte Zahl der Messungen mit der übereinstimmt,
//! die eine echte Suche danach zählt. Das ist der Kern — eine Schätzung, die
//! nie gegen die Wirklichkeit gehalten wird, ist eine Behauptung.
//!
//! **Geprüft**: dass der Trockenlauf keinen Kampf startet, am Kampfzähler des
//! Programms. Die Laufzeit taugt dafür nicht: Ein schneller Lauf ist kein
//! Beweis, und ein belegter Rechner macht aus einem sauberen Trockenlauf einen
//! verdächtigen.
//!
//! **Nicht geprüft**: die zwei Messungen, die `greedy_opts` außerhalb von
//! `measured` fährt (Startliste und gefundene Liste). Sie sind aus dem
//! Quelltext abgelesen, nicht gemessen — es gibt keinen Zähler im Kern, der
//! sie sichtbar machte, und ihr Anteil liegt bei einer langen Suche unter
//! einem Promille. Bei einer kurzen liegt er über einem Prozent, und **deshalb
//! stehen sie überhaupt drin**.

mod gemeinsam;
use gemeinsam::rufe;

/// Liest eine Zahl aus einer JSON-Ausgabe, ohne einen JSON-Parser.
///
/// Die Bin-Datei schreibt JSON von Hand, in fester Reihenfolge und ohne
/// Leerzeichen — das reicht für ein Muster `"name":wert`. Ein Parser als
/// Abhängigkeit stünde in keinem Verhältnis.
fn zahl_aus(json: &str, name: &str) -> u64 {
    let muster = format!("\"{name}\":");
    let start = json
        .find(&muster)
        .unwrap_or_else(|| panic!("Feld {name} fehlt in:\n{json}"))
        + muster.len();
    let rest = &json[start..];
    let ende = rest
        .find(|z: char| !z.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..ende]
        .parse()
        .unwrap_or_else(|_| panic!("Feld {name} ist keine ganze Zahl: {}", &rest[..ende]))
}

/// Die Zahl hinter „Messungen" aus der Textausgabe eines Laufs.
fn messungen_aus_lauf(text: &str) -> u64 {
    text.lines()
        .find(|z| z.trim_start().starts_with("Messungen"))
        .and_then(|z| z.split_whitespace().nth(1))
        .and_then(|z| z.parse().ok())
        .unwrap_or_else(|| panic!("keine Zeile 'Messungen' in:\n{text}"))
}

/// Die Zahl hinter „Kaempfe gerechnet:".
fn kaempfe_aus(text: &str) -> u64 {
    text.lines()
        .find(|z| z.starts_with("Kaempfe gerechnet:"))
        .and_then(|z| z.split_whitespace().last())
        .and_then(|z| z.parse().ok())
        .unwrap_or_else(|| panic!("keine Zeile 'Kaempfe gerechnet' in:\n{text}"))
}

#[test]
fn der_trockenlauf_rechnet_keinen_kampf() {
    // AC2, und das ist die Abnahme dieses Issues.
    let (code, stdout, stderr) = rufe(&["explain", "--boss", "boss-1-1"]);
    assert_eq!(code, 0, "explain gab {code} zurueck:\n{stderr}");
    assert!(
        stderr.is_empty(),
        "explain schrieb auf den Fehlerkanal:\n{stderr}"
    );
    assert_eq!(
        kaempfe_aus(&stdout),
        0,
        "der Trockenlauf hat gerechnet:\n{stdout}"
    );

    // Auch mit Einstellungen, die einen sehr grossen Lauf beschreiben — dort
    // waere ein versehentlicher Kampf am teuersten.
    let (code, stdout, _) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "100000",
        "--max-len",
        "8",
    ]);
    assert_eq!(code, 0);
    assert_eq!(kaempfe_aus(&stdout), 0, "gross eingestellt und gerechnet");
}

#[test]
fn nach_einem_echten_lauf_steht_der_zaehler_ueber_null() {
    // DIE GEGENPROBE ZUM TEST DARUEBER. Ein Zaehler, der immer null zeigt,
    // belegt nichts — er waere dekorativ, und AC2 waere damit erfuellt, ohne
    // etwas zu bedeuten.
    let (code, stdout, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "5",
        "--max-len",
        "1",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let kaempfe = kaempfe_aus(&stdout);
    assert!(
        kaempfe > 0,
        "der Zaehler blieb bei null, obwohl gerechnet wurde:\n{stdout}"
    );

    // Und `bench` ebenso — er rechnet Kaempfe, also muss er sie ausweisen.
    // Die Zeile fehlte dort zuerst: Der Zaehler wurde erhoeht und nie
    // ausgegeben, was von aussen wie „kein Kampf" aussah.
    let (code, stdout, stderr) = rufe(&["bench", "--fights", "100"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        kaempfe_aus(&stdout),
        100,
        "bench nennt seine Kaempfe nicht oder falsch:\n{stdout}"
    );
}

#[test]
fn die_vorhersage_trifft_den_echten_lauf() {
    // DIE KONTROLLPROBE, die das Issue verlangt: Die Schaetzung wird gegen
    // einen kurzen echten Lauf gehalten.
    //
    // Verglichen werden die MESSUNGEN, nicht die Dauer. Die Dauer haengt an
    // Maschine und Last und schwankt zwischen zwei Laeufen; die Messungszahl
    // folgt aus der Bauart und muss auf die Einheit stimmen.
    let (code, trocken, stderr) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "5",
        "--max-len",
        "2",
        "--json",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let vorhergesagt_max = zahl_aus(&trocken, "max");
    let einzeln = zahl_aus(&trocken, "singles");
    let paare = zahl_aus(&trocken, "pairs");

    let (code, lauf, stderr) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "5",
        "--max-len",
        "2",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let gemessen = messungen_aus_lauf(&lauf);

    // Die Suche laeuft hier bis zur Laengengrenze (max-len 2 ist klein genug,
    // dass sie nicht vorher aufgibt) — dann ist die Obergrenze exakt.
    assert_eq!(
        einzeln + paare,
        gemessen,
        "Vorhersage {} (Einzelzuege {einzeln} + Paare {paare}) gegen gemessene {gemessen}\n\
         Trockenlauf:\n{trocken}\nLauf:\n{lauf}",
        einzeln + paare
    );

    // Und die Obergrenze liegt genau zwei darueber — die Startliste und die
    // gefundene Liste, die `measured` nicht mitzaehlt.
    assert_eq!(
        vorhergesagt_max,
        gemessen + 2,
        "die zwei Messungen ausserhalb von measured fehlen oder sind doppelt"
    );
}

#[test]
fn die_vorhersage_ist_auch_bei_laenge_eins_genau() {
    // Der Randfall: eine einzige Runde. Hier ist der Anteil der zwei
    // Messungen ausserhalb am groessten — bei 93 Kandidaten sind es zwei von
    // 95, also gut zwei Prozent. Wer sie weglaesst, sieht es genau hier.
    let (_, trocken, _) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "5",
        "--max-len",
        "1",
        "--json",
    ]);
    let (_, lauf, _) = rufe(&[
        "run",
        "--boss",
        "boss-1-1",
        "--iterations",
        "5",
        "--max-len",
        "1",
    ]);
    assert_eq!(
        zahl_aus(&trocken, "singles") + zahl_aus(&trocken, "pairs"),
        messungen_aus_lauf(&lauf)
    );
    // Bei Laenge eins sind Minimum und Maximum dasselbe: Es gibt nur eine Runde.
    assert_eq!(zahl_aus(&trocken, "min"), zahl_aus(&trocken, "max"));
}

#[test]
fn der_trockenlauf_weist_seine_unsicherheit_aus() {
    // AC4. Geprueft wird nicht, DASS dort Worte stehen, sondern dass die
    // Ausgabe zwischen dem unterscheidet, was feststeht, und dem, was
    // geschaetzt ist — und dass die Spanne wirklich eine Spanne ist.
    let (_, stdout, _) = rufe(&["explain", "--boss", "boss-1-1"]);
    for stueck in ["WAS FESTSTEHT", "WAS GESCHAETZT IST", "UNSICHERHEIT"] {
        assert!(stdout.contains(stueck), "'{stueck}' fehlt in:\n{stdout}");
    }
    let (_, json, _) = rufe(&["explain", "--boss", "boss-1-1", "--json"]);
    assert!(
        zahl_aus(&json, "min") < zahl_aus(&json, "max"),
        "Minimum und Maximum sind gleich — dann ist es keine Spanne:\n{json}"
    );
    assert!(
        json.contains("\"source\":\"Vorgabe\""),
        "die Herkunft der Rate fehlt:\n{json}"
    );
    let (_, json, _) = rufe(&["explain", "--boss", "boss-1-1", "--rate", "12345", "--json"]);
    assert!(
        json.contains("\"source\":\"angegeben\""),
        "eine angegebene Rate wird nicht als solche ausgewiesen:\n{json}"
    );
}

#[test]
fn jede_rate_traegt_ihre_strangzahl() {
    // EINE RATE OHNE STRANGZAHL IST NICHT DEUTBAR, und das ist am 2026-08-18
    // teuer geworden: Die Planzahl aus GAME-DESIGN (350 000) und ein gemessener
    // Wert von 38 474 sahen nach Faktor neun aus. Sie sind es nicht — die eine
    // gilt für sechzehn Stränge, die andere für einen. Zwei Zahlen ohne ihre
    // Bedingung sahen aus wie ein Fehler.
    let (_, json, _) = rufe(&["explain", "--boss", "boss-1-1", "--json"]);
    assert!(
        json.contains("\"threads\":1"),
        "die Vorgabe-Rate nennt ihre Strangzahl nicht:\n{json}"
    );

    // Eine angegebene Rate gilt für die Strangzahl des Aufrufs.
    let (_, json, _) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--rate",
        "260000",
        "--threads",
        "16",
        "--json",
    ]);
    assert!(
        json.contains("\"threads\":16"),
        "eine angegebene Rate uebernimmt die Strangzahl nicht:\n{json}"
    );

    // Und in der Textausgabe steht sie ebenfalls — sie ist der Ort, an dem
    // die meisten sie lesen.
    let (_, text, _) = rufe(&["explain", "--boss", "boss-1-1"]);
    assert!(
        text.contains("Kaempfen je Sekunde auf 1 Strang"),
        "die Textausgabe nennt die Strangzahl nicht:\n{text}"
    );
}

#[test]
fn der_trockenlauf_laeuft_in_sekunden() {
    // AC3. Gemessen wird der GROESSTE sinnvolle Fall — ein Lauf, der Tage
    // braeuchte, muss trotzdem in Sekunden vorhergesagt sein.
    let start = std::time::Instant::now();
    let (code, _, _) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "1000000",
        "--max-len",
        "12",
    ]);
    let dauer = start.elapsed();
    assert_eq!(code, 0);
    assert!(
        dauer.as_secs() < 5,
        "der Trockenlauf brauchte {dauer:?} — das ist keiner mehr"
    );
}

#[test]
fn eine_unerfuellbare_anfrage_wird_hier_schon_abgelehnt() {
    // AC5, beide Faelle.
    //
    // Erstens strukturell: mehr Zeilen als Kandidaten.
    let (code, _, stderr) = rufe(&["explain", "--boss", "boss-1-1", "--max-len", "5000"]);
    assert_ne!(code, 0, "eine unmoegliche Laenge wurde angenommen");
    assert!(stderr.starts_with("FEHLER"), "{stderr}");
    assert!(
        stderr.contains("Kandidatenzeilen"),
        "die Meldung nennt den Grund nicht:\n{stderr}"
    );

    // Zweitens rechnerisch: eine Dauer jenseits der Grenze.
    let (code, _, stderr) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "4000000000",
        "--max-len",
        "12",
    ]);
    assert_ne!(code, 0, "ein Lauf ueber Jahrhunderte wurde angenommen");
    assert!(stderr.starts_with("FEHLER"), "{stderr}");

    // UND DIE GEGENPROBE: Ein grosser, aber machbarer Lauf muss durchgehen.
    // Ohne sie belegte der Test nur, dass irgendetwas abgelehnt wird.
    let (code, stdout, stderr) = rufe(&[
        "explain",
        "--boss",
        "boss-1-1",
        "--iterations",
        "10000",
        "--max-len",
        "8",
    ]);
    assert_eq!(code, 0, "ein machbarer Lauf wurde abgelehnt:\n{stderr}");
    assert_eq!(kaempfe_aus(&stdout), 0);
}

#[test]
fn der_trockenlauf_nennt_alle_geforderten_groessen() {
    // AC1, an der Textausgabe UND am JSON — beide müssen dieselben Größen
    // tragen, sonst ist eine der beiden Ausgaben ärmer als die andere.
    let (_, text, _) = rufe(&["explain", "--boss", "boss-1-1"]);
    for stueck in [
        "Kandidatenzeilen",
        "Messungen",
        "Einzelzuege",
        "Paar-Zuege",
        "Kaempfe",
        "Dauer",
        "Speicher",
        "RECHNER",
    ] {
        assert!(text.contains(stueck), "'{stueck}' fehlt in:\n{text}");
    }
    let (_, json, _) = rufe(&["explain", "--boss", "boss-1-1", "--json"]);
    for feld in [
        "poolSize",
        "measurements",
        "singles",
        "pairs",
        "fights",
        "seconds",
        "memoryBytes",
        "fightsRun",
    ] {
        assert!(json.contains(feld), "'{feld}' fehlt in:\n{json}");
    }
}
