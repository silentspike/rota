//! Der Suchbericht nennt seine eigenen Grenzen (Issue #558, AC4).
//!
//! **Woher die Forderung kommt.** Der Autor von #558 hat das Ergebnis der
//! Rotations-Suche als „diese Zauber taugen nichts" gelesen und ein Issue
//! darauf gebaut. Die Suche hatte damals einen blinden Fleck, den ihr Bericht
//! nicht nannte — und ein Ergebnis ohne seine Grenze wird als vollstaendig
//! gelesen.
//!
//! **Warum als Pruefall und nicht als Absatz im Bericht.** Ein Satz in der
//! Ausgabe verschwindet beim naechsten Umbau, ohne dass es jemand merkt; der
//! Lauf bleibt gruen, weil er nichts prueft. Diese Datei faerbt.
//!
//! **Was hier NICHT geprueft wird:** ob die Grenzen inhaltlich stimmen. Das
//! tun die Messungen zu #558. Hier steht nur, dass der Laeufer sie ausspricht.

use std::fs;

/// Der Quelltext des Laeufers, dessen Ausgabe hier geprueft wird.
const LAEUFER: &str = "examples/rotation-search.rs";

fn quelle() -> String {
    fs::read_to_string(LAEUFER).unwrap_or_else(|e| panic!("{LAEUFER} nicht lesbar: {e}"))
}

/// **Traeger-Zusicherung.** Alles Weitere sucht Zeichenfolgen in dieser Datei.
/// Ist sie leer oder am falschen Ort, waere jede Suche unten erfuellt und
/// nichtssagend.
#[test]
fn der_laeufer_ist_ueberhaupt_da() {
    assert!(
        quelle().len() > 10_000,
        "{LAEUFER} ist unerwartet klein — pruefe den Pfad, bevor du den Testen glaubst"
    );
}

/// Die Laengengrenze. Sie stand schon vor #558 da und soll nicht verschwinden.
#[test]
fn der_bericht_nennt_die_laengengrenze() {
    assert!(
        quelle().contains("UNTERGRENZE, KEIN BEWIESENES OPTIMUM"),
        "Der Laeufer nennt seine Laengengrenze nicht mehr"
    );
}

/// **Der Kern von AC4.** Die Suche findet nur eine Sorte Zusammenspiel; wer
/// das nicht weiss, liest eine Abwesenheit als Schwaeche.
#[test]
fn der_bericht_nennt_die_paar_grenze() {
    let q = quelle();
    assert!(
        q.contains("NUR EINE SORTE ZUSAMMENSPIEL"),
        "Der Laeufer nennt seine Paar-Grenze nicht — genau der Zustand, der zu #558 gefuehrt hat"
    );
    assert!(
        q.contains("NICHT belegt schwach"),
        "Der Bericht sagt nicht, was aus der Grenze FOLGT: dass ein fehlender Zauber \
         kein schwacher Zauber ist. Ohne den Satz ist die Grenze eine Fussnote."
    );
}

/// Die drei Faelle, die die Paar-Suche nicht abdeckt, stehen einzeln da.
///
/// **Warum namentlich und nicht als Gesamtzahl.** Faellt einer der drei weg,
/// bricht dieser Fall — eine Zahl „drei Faelle" wuerde still weiterzaehlen,
/// auch wenn der wichtigste fehlt.
#[test]
fn die_drei_unentdeckten_faelle_stehen_namentlich() {
    let q = quelle();
    for fall in [
        "ohne Buff-Bedingung",
        "Dreiergruppen",
        "beide Haelften ein Fenster",
    ] {
        assert!(q.contains(fall), "Im Bericht fehlt der Fall: {fall}");
    }
}

/// Die Zahl der Kandidatenzeilen wird abgelesen, nicht geschrieben.
///
/// Hier stand „siebzehn", gemessen sind es 93. Eine feste Zahl im Fliesstext
/// altert still — dieser Fall haelt fest, dass sie aus dem Bestand kommt.
#[test]
fn die_kandidatenzahl_ist_nicht_fest_verdrahtet() {
    let q = quelle();
    assert!(
        !q.contains("siebzehn Kandidaten"),
        "Die alte feste Zahl ist zurueck — sie war schon einmal um Faktor fuenf daneben"
    );
    assert!(
        q.contains("{} Kandidatenzeilen unerreichbar"),
        "Die Kandidatenzahl wird nicht mehr aus `pool.len()` abgelesen"
    );
}
