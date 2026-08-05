//! Der Bericht druckt seine Parameter (Issue #242, AC4).
//!
//! # Warum das ein Prueffall sein muss
//!
//! Die Parameter-Ausgabe ist Text in einer `main`-Funktion. Beim naechsten
//! Umbau des Berichts faellt sie heraus, ohne dass etwas rot wird — und dann
//! stehen die Zahlen wieder ohne ihre Herkunft da.
//!
//! Genau so ist #118 entstanden: Der Aufruf, der die Zahl erzeugt hat, wurde
//! nie committet. Weder Durchgangszahl noch Startwert, Kampfdauer oder Boss
//! sind festgehalten, und auf dieser Zahl beruhen zwei offene
//! Design-Entscheidungen.
//!
//! # WIE MAN SIE GEGENPRUEFT — bitte lesen, bevor man es versucht
//!
//! **Der Begriff muss nur EINMAL vorkommen, oder alle Fundstellen muessen
//! weg.** Sonst bleibt die Pruefung gruen, und man haelt sie faelschlich fuer
//! kaputt.
//!
//! Gemessen am 2026-08-05, Fundstellen in `println!`-Zeilen:
//!
//! ```text
//! "Durchgaenge je Zeile"   2      "Kampfdauer"          1   <- geeignet
//! "Startwert"              2      "Abbruch beim Sieg"   1   <- geeignet
//! "Boss "                  4
//! ```
//!
//! Mein erster Versuch entfernte EINE "Startwert"-Zeile von zweien. Die
//! Pruefung blieb gruen — richtig, denn der Begriff stand noch da. Ich habe
//! daraus geschlossen, der Waechter faenge nichts, und danach zwanzig Minuten
//! an der falschen Stelle gesucht.
//!
//! Mit einem Begriff, der nur einmal vorkommt, schlaegt er sofort an —
//! Rueckgabewert 101, und die Meldung nennt den fehlenden Parameter.
//!
//! # UND EIN ZWEITER SATZ, der mich weitere zwanzig Minuten gekostet hat
//!
//! **`cargo test --test X` baut NUR diesen Test, nicht die `examples/`.**
//!
//! Ich wollte pruefen, ob der Bau-Server die geaenderte Datei ueberhaupt
//! bekommt, und habe dafuer einen Syntaxfehler in `rotation-balance.rs`
//! eingebaut. Der Bau lief durch — woraus ich schloss, die Datei komme nicht
//! an.
//!
//! Falsch. Sie kommt an; sie wird von diesem Befehl nur nicht ueberSETZT.
//! `include_str!` liest sie trotzdem, naemlich zur Compile-Zeit des Tests.
//!
//! Wer wirklich pruefen will, ob eine Datei ankommt, muss sie in etwas
//! einbauen, das der Befehl auch uebersetzt — oder `cargo test` ohne
//! `--test` laufen lassen.
//!
//! # Was diese Pruefung NICHT kann
//!
//! Sie liest den Quelltext, nicht die Ausgabe. Wer die Zeilen auskommentiert
//! oder in einen nie aufgerufenen Zweig verschiebt, kommt daran vorbei. Die
//! Grenze steht hier, damit niemand die Pruefung fuer mehr haelt, als sie ist.

/// Der unveraenderte Quelltext des Berichts.
const BERICHT: &str = include_str!("../examples/rotation-balance.rs");

/// Nur die Zeilen, die tatsaechlich etwas AUSGEBEN.
///
/// # Der erste Entwurf suchte im ganzen Quelltext und fand nichts heraus
///
/// Er prueft, ob "Startwert" irgendwo vorkommt — und das tut es auch in den
/// Kommentaren, die erklaeren, warum der Startwert wichtig ist. Die Gegenprobe
/// (Ausgabe-Zeile entfernt) blieb deshalb GRUEN.
///
/// Ein Waechter, der die Erklaerung fuer die Sache haelt, faengt nichts. Das
/// ist dieselbe Klasse wie in #229, nur andersherum: Dort fing die Suche zu
/// viel, weil sie Kommentare mitlas — hier fing sie zu wenig, aus demselben
/// Grund.
fn ausgabe_zeilen() -> String {
    BERICHT
        .lines()
        .map(str::trim)
        // **Auch Fortsetzungszeilen.** Ein mehrzeiliges `println!` beginnt
        // nur in seiner ERSTEN Zeile mit dem Namen; die Zeichenkette steht
        // darunter und faengt mit einem Anfuehrungszeichen an. Mein erster
        // Filter nahm nur die erste Zeile — und uebersah damit ausgerechnet
        // die Kampfdauer-Zeile, also den einzigen Begriff, mit dem sich diese
        // Pruefung sauber gegenpruefen laesst.
        .filter(|zeile| zeile.starts_with("println!") || zeile.starts_with('"'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn der_bericht_ist_ueberhaupt_lesbar() {
    // Ohne diese Zeile waere eine leere Datei die stillste Art, gruen zu sein.
    assert!(
        BERICHT.len() > 1000,
        "der Bericht ist leer oder nicht geladen — die Pruefung wuerde nichts messen"
    );
}

#[test]
fn er_druckt_jeden_parameter_der_zu_einer_zahl_gehoert() {
    // **Vier Groessen, die eine Balance-Zahl erst nachfahrbar machen.** Fehlt
    // eine davon, laesst sich das Ergebnis nicht wiederholen — und eine Zahl,
    // die niemand wiederholen kann, ist eine Behauptung.
    let ausgabe = ausgabe_zeilen();
    assert!(
        ausgabe.len() > 500,
        "keine Ausgabe-Zeilen gefunden — die Pruefung wuerde nichts messen"
    );

    for begriff in [
        "Durchgaenge je Zeile",
        "Startwert",
        "Boss ",
        "Kampfdauer",
        "Abbruch beim Sieg",
    ] {
        assert!(
            ausgabe.contains(begriff),
            "der Bericht druckt \"{begriff}\" nicht mehr — dann tragen seine \
             Zahlen ihre Herkunft nicht mit, sobald jemand sie herauskopiert"
        );
    }
}

#[test]
fn die_parameter_sind_konstanten_und_keine_vorgaben() {
    // Ein Lauf, der Startwert oder Durchgangszahl dem Standard ueberlaesst,
    // ist nicht reproduzierbar — auch wenn er sie druckt: Der Standard kann
    // sich aendern, die gedruckte Zahl waere dann von gestern.
    assert!(BERICHT.contains("const RUNS"), "RUNS ist keine Konstante");
    assert!(BERICHT.contains("const SEED"), "SEED ist keine Konstante");
    assert!(
        BERICHT.contains("base_seed: SEED"),
        "der Lauf benutzt SEED nicht — dann steht die Konstante da und wirkt nicht"
    );
}

#[test]
fn die_nicht_reproduzierbare_zahl_ist_als_solche_vermerkt() {
    // **Die alte Zahl bleibt stehen, statt ersetzt zu werden** (AC3-Auflage).
    // Wer in drei Wochen eine Abweichung sieht, muss erkennen koennen, ob sich
    // das Spiel geaendert hat oder nur die Messung.
    let ausgabe = ausgabe_zeilen();
    assert!(
        ausgabe.contains("nie committet"),
        "der Vermerk fehlt, dass #118s Aufruf nicht committet wurde"
    );
    assert!(
        ausgabe.contains("9892,7"),
        "die alte Zahl ist verschwunden — sie sollte daneben stehen bleiben"
    );
}
