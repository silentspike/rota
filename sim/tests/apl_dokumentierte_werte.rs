//! Die Werteliste in der Modul-Doku beschreibt die Sprache, die es gibt
//! (Issue #797).
//!
//! # Warum das eine eigene Datei wert ist
//!
//! `sim/src/apl/mod.rs` führt in seinem Modul-Kommentar eine Tabelle „Werte,
//! die eine Bedingung lesen kann". Sie ist von Hand geführt, und der Parser
//! daneben ist es auch — zwei Aufzählungen derselben Sache.
//!
//! Beim Bau von `boss.next_strike_in` kam heraus, wie weit sie schon
//! auseinandergelaufen waren: **Sieben Werte fehlten**, manche seit Monaten.
//! `decay`, `shield` und `control` seit #526; `buff.<zauber>.active` und
//! `.remains` seit #553; `boss.casting` und `boss.cast_remains` seit #797 —
//! also aus demselben Issue, in dem sie gebaut wurden.
//!
//! Keiner dieser Fälle war ein Fehler beim Schreiben. Es ist die Bauart: Wer
//! einen Wert ergänzt, ändert `enum`, `eval`, `Display` und Parser, weil der
//! Übersetzer ihn dazu zwingt. Die Tabelle zwingt ihn zu nichts.
//!
//! # Was dieser Fall kann und was nicht
//!
//! Er prüft **eine Richtung**: Jeder Ausdruck, der in der Tabelle steht, muss
//! sich auch lesen lassen. Das fängt falsche Schreibweisen und Zeilen, die
//! einen entfernten Wert beschreiben.
//!
//! **Die andere Richtung fängt er nicht** — ein Wert, der existiert und in der
//! Tabelle fehlt, bleibt unbemerkt. Dafür bräuchte es eine vollständige Liste
//! aller `Value`-Varianten, und die wäre selbst wieder von Hand geführt: die
//! dritte Aufzählung neben den beiden, die schon auseinandergelaufen sind.
//!
//! Das steht hier, weil ein grüner Lauf sonst mehr zu belegen scheint, als er
//! belegt.

use rota_sim::apl::{parse, AplName};

/// Die Datei, in der die Tabelle steht.
const QUELLE: &str = "src/apl/mod.rs";

/// Liest die Ausdrücke aus der ersten Spalte der Werte-Tabelle.
///
/// **Aus der Datei gelesen, nicht abgeschrieben.** Eine zweite Liste hier wäre
/// genau der Fehler, den der Fall fangen soll.
fn dokumentierte_ausdruecke(quelle: &str) -> Vec<String> {
    let mut in_tabelle = false;
    let mut aus = Vec::new();

    for zeile in quelle.lines() {
        let zeile = zeile.trim_start_matches("//!").trim();

        if zeile.starts_with("| Ausdruck |") {
            in_tabelle = true;
            continue;
        }
        if in_tabelle {
            // Die Trennzeile unter dem Kopf.
            if zeile.starts_with("| ---") {
                continue;
            }
            if !zeile.starts_with('|') {
                break;
            }
            let Some(erste) = zeile.split('|').nth(1) else {
                continue;
            };
            let ausdruck = erste.trim().trim_matches('`').trim();
            if !ausdruck.is_empty() {
                aus.push(ausdruck.to_string());
            }
        }
    }

    aus
}

/// Setzt einen Platzhalter wie `<zauber>` durch einen echten Namen.
///
/// Die Tabelle beschreibt Familien (`dot.<zauber>.remains`), die Sprache liest
/// einzelne Namen. Ohne diesen Schritt prüfte der Fall die Schreibweise des
/// Platzhalters statt die des Werts.
fn einsetzbar(ausdruck: &str) -> Option<String> {
    // `searing` traegt einen Brand, `flashpoint` eine Verstaerkung — beide
    // stehen in den Daten und werden von den Pruefrotationen benutzt.
    let gesetzt = ausdruck
        .replace("dot.<zauber>", "dot.searing")
        .replace("buff.<zauber>", "buff.flashpoint")
        .replace("cooldown.<zauber>", "cooldown.searing");

    // Die Zeile fuer die Schul-Ressource nennt keinen Wert, sondern eine
    // Familie mit Beispiel im Fliesstext.
    if gesetzt.contains('<') {
        return None;
    }
    Some(gesetzt)
}

#[test]
fn alle_dokumentierten_werte_lassen_sich_lesen() {
    let quelle = std::fs::read_to_string(QUELLE)
        .unwrap_or_else(|e| panic!("{QUELLE} nicht lesbar: {e} (Lauf aus sim/ heraus?)"));
    let ausdruecke = dokumentierte_ausdruecke(&quelle);

    // **Träger-Zusicherung.** Findet das Muster die Tabelle nicht mehr — weil
    // jemand die Kopfzeile umbenennt oder die Tabelle verschiebt —, prüfte die
    // Schleife darunter nichts und bliebe grün.
    assert!(
        ausdruecke.len() >= 10,
        "in {QUELLE} wurden nur {} Tabellenzeilen gefunden; das Muster greift \
         nicht mehr, und der Fall prueft nichts. Gefunden: {ausdruecke:?}",
        ausdruecke.len()
    );

    let mut ungelesen = Vec::new();
    for ausdruck in &ausdruecke {
        let Some(gesetzt) = einsetzbar(ausdruck) else {
            continue;
        };
        // Eine Bedingung, damit der Wert an der Stelle steht, an der die
        // Sprache ihn liest.
        let text = format!("wisp if {gesetzt} >= 0\n");
        if parse(AplName("doku"), &text).is_err() {
            ungelesen.push(ausdruck.clone());
        }
    }

    assert!(
        ungelesen.is_empty(),
        "diese Ausdruecke stehen in der Tabelle von {QUELLE}, lassen sich aber \
         nicht lesen: {ungelesen:?}. Entweder ist die Schreibweise falsch, \
         oder die Zeile beschreibt einen Wert, den es nicht mehr gibt."
    );
}

/// Der neue Wert steht in der Tabelle.
///
/// # Warum das nicht der Fall oben mit abdeckt
///
/// Der prüft, dass alles Dokumentierte lesbar ist — eine fehlende Zeile fällt
/// ihm nicht auf, das steht im Dateikopf. Für den Wert aus diesem Issue wird
/// die zweite Richtung deshalb einzeln festgehalten, statt sich auf eine
/// Prüfung zu verlassen, die sie nicht leisten kann.
#[test]
fn der_neue_wert_steht_in_der_tabelle() {
    let quelle = std::fs::read_to_string(QUELLE)
        .unwrap_or_else(|e| panic!("{QUELLE} nicht lesbar: {e} (Lauf aus sim/ heraus?)"));
    let ausdruecke = dokumentierte_ausdruecke(&quelle);

    assert!(
        ausdruecke.iter().any(|a| a == "boss.next_strike_in"),
        "boss.next_strike_in fehlt in der Werteliste von {QUELLE}; gefunden \
         wurden {ausdruecke:?}"
    );
}
