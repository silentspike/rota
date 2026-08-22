//! Wächter: Eine paritätsgeprüfte Prioritätenliste darf nur Werte benutzen,
//! die die TypeScript-Seite nachbildet (Issue #1074, AC4).
//!
//! # Warum das ein Wächter sein muss und kein Vermerk
//!
//! Die Paritätskette vergleicht denselben Kampf auf beiden Seiten. Die Listen
//! liegen dabei **verschieden vor**: In Rust als geparster Text
//! (`sim/apl/step1-good.apl`), auf der Browser-Seite als handgeschriebene
//! Funktion (`src/devtools/reference-run.ts`, `aplGood`). Der Dateikopf dort
//! sagt es selbst — die Listen sind *wortgleich zu apl_good und apl_bad der
//! Referenz*.
//!
//! Mit #1074 kennt die Sprache vier Werte, die es auf der TS-Seite nicht gibt:
//! `doom`, `doom_charges`, `afflicted`, `race_progress`. Schreibt jemand eine
//! paritätsgeprüfte Liste mit `purge if doom >= 60`, hat sie **keine
//! wortgleiche Gegenseite mehr**.
//!
//! **Und das bricht die Bitgleichheit nicht — es macht die Prüfung stumm.**
//! Sie vergleicht weiter, was auf beiden Seiten steht, und meldet nicht, dass
//! sie etwas nicht mehr abdeckt. Genau diese Klasse hat #1062 in der
//! Deckungstabelle aufgedeckt: eine Deckung, die aussieht wie eine Aussage und
//! keine ist.
//!
//! # Was dieser Wächter kann und was nicht
//!
//! Er prüft **eine Richtung**: Kein Wert in einer paritätsgeprüften Liste ohne
//! Entsprechung drüben. Er prüft **nicht**, ob die Entsprechung dasselbe tut —
//! dafür ist der Schadensvergleich der Paritätsprüfung da.
//!
//! Die Zuordnung in `in_typescript_nachgebildet` ist **von Hand geführt**, und das
//! ist ihre Schwäche: Sie kann von `reference-run.ts` abweichen, ohne dass es
//! auffällt. Automatisch ableiten ließe sie sich nicht — drüben stehen
//! TypeScript-Funktionen, keine Werteliste. Wer `reference-run.ts` um einen
//! Wert erweitert, muss sie hier nachtragen; wer es vergisst, bekommt einen
//! roten Wächter statt einer stillen Lücke, und das ist die richtige Richtung
//! des Fehlers.

use rota_sim::apl::{parse, AplName, Value};

/// Die Liste, die in der Paritätskette gefahren wird
/// (`tests/parity/step1.test.ts` über `runReference('good')`).
/// Der Pfad ist relativ zum Crate-Wurzelverzeichnis `sim/` — dort setzt
/// `cargo test` das Arbeitsverzeichnis, und dieselbe Form benutzt
/// `apl_dokumentierte_werte.rs`.
const PARITAETSGEPRUEFT: &str = "apl/step1-good.apl";

/// Werte, die `src/devtools/reference-run.ts` nachbildet.
///
/// Abgelesen an `aplGood`/`aplBad` (Stand 2026-08-22):
///
/// | Rust | TypeScript |
/// | --- | --- |
/// | `DotRemains` | `combat.dotRemainingMs(index, timeMs)` |
/// | `Gcd` | `GCD_MS` |
/// | `Resource` | `combat.resource` |
/// | `Literal` | Zahlen im Quelltext |
///
/// Die Bereitschaft (`isReady`) steht drüben ausdrücklich in jeder Regel; in
/// der APL ist sie stillschweigend und braucht deshalb keinen Wert.
fn in_typescript_nachgebildet(value: Value) -> bool {
    matches!(
        value,
        Value::Literal(_) | Value::Resource | Value::DotRemains(_) | Value::Gcd
    )
}

#[test]
fn kein_wert_ohne_entsprechung_auf_der_browserseite() {
    let text = std::fs::read_to_string(PARITAETSGEPRUEFT).unwrap_or_else(|e| {
        panic!("{PARITAETSGEPRUEFT} nicht lesbar: {e} (Lauf aus sim/ heraus?)")
    });
    let apl = parse(AplName("paritaet"), &text).expect("Liste ist fehlerhaft");

    // ZUSICHERUNG: Die Liste muss überhaupt Bedingungen haben. Eine Liste ohne
    // Werte bestünde diesen Fall, ohne dass er etwas geprüft hätte — und eine
    // leere oder falsch gelesene Datei sähe genauso aus.
    let werte: Vec<Value> = apl
        .rules
        .iter()
        .flat_map(|r| r.condition.groups.iter())
        .flatten()
        .flat_map(|t| [t.left, t.right])
        .collect();
    assert!(
        werte.len() >= 4,
        "Nur {} Werte in {PARITAETSGEPRUEFT} gefunden. Die Liste hat zwei \
         Bedingungen mit je zwei Seiten — weniger heißt, sie wurde nicht \
         richtig gelesen, und dann prüft dieser Fall nichts.",
        werte.len()
    );

    let ohne: Vec<Value> = werte
        .into_iter()
        .filter(|v| !in_typescript_nachgebildet(*v))
        .collect();
    assert!(
        ohne.is_empty(),
        "Diese Werte stehen in einer paritätsgeprüften Liste, werden aber von \
         src/devtools/reference-run.ts nicht nachgebildet: {ohne:?}\n\n\
         Die Paritätsprüfung wird davon nicht rot — sie vergleicht weiter, was \
         auf beiden Seiten steht, und meldet nicht, dass sie etwas nicht mehr \
         abdeckt. Entweder die Gegenseite nachziehen oder die Liste aus der \
         Paritätskette nehmen."
    );
}

/// **Die Gegenrichtung.** Ohne sie wäre ein Wächter grün, der jeden Wert
/// durchwinkt — und dann belegte der Fall oben nichts.
#[test]
fn ein_wert_ohne_entsprechung_wuerde_gefangen() {
    for neu in [
        Value::Doom,
        Value::DoomCharges,
        Value::Afflicted,
        Value::RaceProgress,
    ] {
        assert!(
            !in_typescript_nachgebildet(neu),
            "{neu:?} gilt als nachgebildet, obwohl es die vier neuen Werte aus \
             #1074 sind — dann fängt der Wächter genau den Fall nicht, für den \
             er gebaut wurde."
        );
    }
    // Und die Positiv-Kontrolle: Was wirklich drüben steht, muss durchgehen.
    assert!(
        in_typescript_nachgebildet(Value::Gcd),
        "`gcd` wird drüben als GCD_MS gelesen und muss durchgehen. Tut es das \
         nicht, ist der Wächter zu streng und würde jede Liste ablehnen."
    );
}
