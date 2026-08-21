//! `rota-sim data` trennt Spielinstanzen von Pruefprofilen (Nachbereitung #897).
//!
//! # Warum dieser Fall am PROGRAMM prueft und nicht an der Funktion
//!
//! Die Trennung nuetzt niemandem, solange sie nur im Code steht. Was gelesen
//! wird, ist die **Ausgabe** — und genau dort stand bis zur Nachbereitung von
//! #897 eine einzelne Zahl `"instances": 7`, die fuer ihre Frage richtig war
//! und sich als „sieben Spielinstanzen" las. Es sind fuenf.
//!
//! # Was diesen Fall bei der ALTEN Zaehlweise rot macht
//!
//! Beides zusammen, und jedes fuer sich:
//!
//! - Die alte Ausgabe kannte die Schluessel `gameInstances` und
//!   `probeProfiles` nicht — sie fehlten schlicht.
//! - Sie fuehrte `instances` mit dem Wert 7. Dieser Fall verlangt, dass der
//!   Schluessel **verschwunden** ist, statt still eine neue Bedeutung zu
//!   tragen: Ein entfernter Schluessel bricht laut, ein umgedeuteter nicht.

mod gemeinsam;
use gemeinsam::rufe;

/// Die Zahlen des Bestands, gemessen gegen `origin/main` (siehe #981, AC6).
///
/// # Warum sie fest hier stehen und nicht aus den Daten abgeleitet werden
///
/// Eine Erwartung, die mit dem Prüfling mitwächst, kann nicht scheitern. Diese
/// hier sollen scheitern, sobald sich der Bestand ändert — dann ist die
/// Änderung sichtbar, statt still durchzugehen.
///
/// **`PRUEFBOSSE` ist am 2026-08-19 zum ersten Mal ausgelöst worden**, und die
/// Herkunft der neuen Zahl gehört dazu: 18 war der Stand aus #981; dazu kamen
/// `test-lethal-strike` und `test-survivable-strike` aus #797 (die beiden
/// Zwillinge für AC5, beide in der Instanz `testing`). Ohne diese Zeile wäre
/// die 20 in vier Wochen eine Zahl ohne Beleg.
///
/// **Am 2026-08-20 ein zweites Mal ausgelöst, auf 21:** `orch-maxload.json`, das
/// Lastprofil des Orchestrators, lag bis dahin ungetrackt in einem einzigen
/// Arbeitsbaum. Es ist auf ausdrückliche Freigabe hin aufgenommen worden
/// (Fragenseite `f-905-lastprofil`) — und zwar mit `instance: "testing"` statt
/// `"prologue"`. Genau daran hängt, dass diese Zeile die einzige ist, die sich
/// ändert: Mit `"prologue"` wäre ein Messwerkzeug als Spielboss gezählt worden,
/// `WERTUNGSFAEHIG` wäre auf 18 gesprungen und die Trennung, die dieser Test
/// misst, wäre stillschweigend aufgehoben.
///
/// **`WERTUNGSFAEHIG` bleibt bei 17, und das ist die eigentliche Probe:** Wäre
/// eines der beiden neuen Profile versehentlich in einer gewerteten Instanz
/// gelandet, müsste diese Zahl auf 19 springen. Sie tut es nicht — damit ist
/// die Trennung zwischen Prüfprofil und Spielboss gemessen statt behauptet.
const SPIELINSTANZEN: usize = 5;
const PRUEFPROFILE: usize = 2;
const WERTUNGSFAEHIG: usize = 17;
const PRUEFBOSSE: usize = 21;

#[test]
fn die_json_ausgabe_trennt_spielinstanzen_von_pruefprofilen() {
    let (code, aus, err) = rufe(&["data", "--json"]);
    assert_eq!(code, 0, "data --json soll gelingen; stderr: {err}");

    for (schluessel, wert) in [
        ("gameInstances", SPIELINSTANZEN),
        ("probeProfiles", PRUEFPROFILE),
        ("scoredBosses", WERTUNGSFAEHIG),
        ("probeBosses", PRUEFBOSSE),
    ] {
        let erwartet = format!("\"{schluessel}\":{wert}");
        assert!(
            aus.contains(&erwartet),
            "in der Ausgabe fehlt {erwartet}\n{aus}"
        );
    }

    // **Der Schluessel ist WEG, nicht umgedeutet.** Stuende dort weiterhin
    // `"instances"`, muesste jeder Leser raten, ob die Zahl daneben noch
    // dasselbe meint wie gestern. Ein entfernter Schluessel bricht laut.
    assert!(
        !aus.contains("\"instances\""),
        "der alte Sammelschluessel steht noch in der Ausgabe:\n{aus}"
    );
}

#[test]
fn die_tabelle_nennt_beide_klassen_und_sagt_dass_pruefprofile_nicht_zaehlen() {
    let (code, aus, err) = rufe(&["data"]);
    assert_eq!(code, 0, "data soll gelingen; stderr: {err}");

    assert!(
        aus.contains("wertungsfaehig"),
        "die Tabelle nennt die wertungsfaehigen Bosse nicht:\n{aus}"
    );
    assert!(
        aus.contains("Pruefprofile"),
        "die Tabelle nennt die Pruefprofile nicht:\n{aus}"
    );
    // Die Zahl allein genuegt nicht: Wer „18 in 2 Profilen" liest, muss
    // erfahren, dass sie in KEINE Wertung eingehen. Sonst sieht es aus wie
    // eine weitere Sorte Spielinhalt.
    assert!(
        aus.contains("KEINER Wertung"),
        "die Tabelle sagt nicht, dass Pruefprofile nicht gewertet werden:\n{aus}"
    );

    // Und die alte Sammelzeile ist verschwunden.
    assert!(
        !aus.contains("Instanzen          "),
        "die alte Sammelzeile steht noch da:\n{aus}"
    );
}

/// JSON und Klartext nennen dieselbe Sache mit demselben Wort.
///
/// # Woher dieser Fall kommt
///
/// Ein erster Entwurf fuehrte im JSON `probeInstances`, waehrend die Tabelle
/// daneben von „Profilen" sprach — dieselbe Zahl, zwei Namen, in **derselben
/// Ausgabe**. Daneben stand dasselbe noch einmal: `scoredInstances` gegen
/// „Spielinstanzen".
///
/// Jetzt traegt jeder Schluessel das Wort seines Klartexts:
///
/// ```text
/// gameInstances  <->  "Spielinstanzen"
/// probeProfiles  <->  "Profilen"
/// scoredBosses   <->  "wertungsfaehig"
/// ```
///
/// # Warum nicht der Code-Begriff
///
/// `scoredInstances` waere naeher an `ScoringSet` aus #981 — und braeuchte
/// eine Uebersetzungstabelle zum Klartext. Eine Tabelle zwischen zwei Namen
/// fuer dieselbe Sache ist die Stelle, die spaeter auseinanderlaeuft. Der
/// Code-Begriff steht ohnehin im Code; die Ausgabe ist fuer den, der sie liest.
///
/// # Was dieser Fall verhindert
///
/// Ohne ihn faellt die Angleichung beim naechsten Umbau lautlos auseinander:
/// Beide Ausgaben waeren fuer sich genommen richtig, und genau deshalb wuerde
/// es niemand merken.
#[test]
fn json_und_klartext_nennen_dieselbe_sache_gleich() {
    let (_, json, _) = rufe(&["data", "--json"]);
    let (_, text, _) = rufe(&["data"]);

    for (schluessel, wert, im_text) in [
        ("gameInstances", SPIELINSTANZEN, "Spielinstanzen"),
        ("probeProfiles", PRUEFPROFILE, "Profilen"),
    ] {
        assert!(
            json.contains(&format!("\"{schluessel}\":{wert}")),
            "im JSON fehlt {schluessel} mit {wert}:\n{json}"
        );
        assert!(
            text.contains(&format!("{wert} {im_text}")),
            "im Klartext fehlt '{wert} {im_text}':\n{text}"
        );
    }

    // Die Gegenrichtung: der Schluessel, der Profile faelschlich „Instanzen"
    // nannte, darf nicht zurueckkommen.
    assert!(
        !json.contains("probeInstances"),
        "probeInstances ist zurueck — Profile sind keine Instanzen:\n{json}"
    );
}

/// Die Zahlen der Uebersicht stimmen mit `data bosses` ueberein.
///
/// # Warum das nicht ueberfluessig ist
///
/// Die Uebersicht koennte ihre Zahlen aus einer anderen Quelle beziehen als
/// die Liste darunter — dann stuende in der einen Zeile 17 und in der anderen
/// laegen 18 Bosse. Dieser Fall vergleicht beide Wege und faengt genau das.
#[test]
fn uebersicht_und_bossliste_zaehlen_dasselbe() {
    let (_, uebersicht, _) = rufe(&["data", "--json"]);
    let (_, bosse, _) = rufe(&["data", "bosses", "--json"]);

    let gezaehlt = |instanz: &str| {
        bosse
            .matches(&format!("\"instance\":\"{instanz}\""))
            .count()
    };
    let pruefbosse = gezaehlt("test") + gezaehlt("testing");
    let wertungsfaehig = bosse.matches("\"instance\":").count() - pruefbosse;

    assert_eq!(
        pruefbosse, PRUEFBOSSE,
        "die Bossliste enthaelt {pruefbosse} Pruefbosse, die Uebersicht nennt {PRUEFBOSSE}"
    );
    assert_eq!(wertungsfaehig, WERTUNGSFAEHIG);
    assert!(uebersicht.contains(&format!("\"probeBosses\":{pruefbosse}")));
    assert!(uebersicht.contains(&format!("\"scoredBosses\":{wertungsfaehig}")));
}
