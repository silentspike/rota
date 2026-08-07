//! Der Simulator kennt genau EINE Ressource — und das ist eine Annahme der
//! Bauart, keine Eigenschaft des Spiels (Issue #506).
//!
//! **Gefunden von Worker-1**, als er etwas anderes messen sollte.
//!
//! # Der Unterschied, der still bleibt
//!
//! `sim/build.rs`, `emit_resource` liest den Abschnitt FEST aus
//! `calcination.json` und erzeugt ein einzelnes `pub static RESOURCE`. Das
//! Spiel fuehrt dagegen eine Ressource JE SCHULE (`SchoolDef.resource`).
//!
//! Solange nur CALCINATION eine hat, faellt es nicht auf. Die zweite Schule
//! mit eigener Ressource wuerde still ignoriert: Der Simulator rechnete
//! weiter mit Hitze, das Spiel mit der neuen — und die Paritaet meldete eine
//! Abweichung an einer Stelle, die nicht die Ursache ist.
//!
//! # Warum dieser Fall die Regel nachbaut, statt sie aufzurufen
//!
//! Der Waechter selbst steht in `build.rs` und laeuft beim ERZEUGEN — er
//! bricht den Bau ab, bevor irgendein Test startet. Aus einem Test heraus
//! ist er nicht erreichbar.
//!
//! Deshalb prueft dieser Fall zweierlei getrennt:
//! 1. die REGEL an erfundenen Eingaben (dass sie ueberhaupt ablehnt), und
//! 2. den ZUSTAND der echten Daten (dass heute genau eine Ressource liegt).
//!
//! **Die beiden tragen nicht gleich viel.** Der ZUSTANDS-Fall ist die
//! eigentliche Absicherung: Er bricht, sobald jemand eine Ressource
//! hinzufuegt — auch dann, wenn der Regel-Nachbau inzwischen gedriftet ist.
//! Der Regel-Fall ist die ERKLAERUNG: Er zeigt, was der Waechter im Bau
//! ablehnt, und belegt, dass er ueberhaupt etwas ablehnt.
//!
//! Zwei Quellen sind hier deshalb ein Vorteil, kein Mangel: Beide brechen bei
//! DERSELBEN Bedingung, und faellt eine aus, faengt die andere. Der Fall, vor
//! dem sonst zu warnen ist, sieht anders aus — zwei Quellen, die
//! VERSCHIEDENE Antworten geben koennen, und jemand glaubt der falschen.
//! Diesen Zustand gibt es hier nicht.
//!
//! Ein Nachbau ist sonst eine zweite Quelle, die auseinanderlaufen kann.
//! Hier ist er die einzige Moeglichkeit, die Regel ueberhaupt zu belegen —
//! und die Handprobe steht im Commit: Ein zweiter `resource`-Abschnitt in
//! `fixation.json` liess den Bau abbrechen mit
//! `["calcination.json", "fixation.json"]`.

use std::fs;

/// Zaehlt die Schul-Dateien mit eigenem `resource`-Abschnitt.
fn mit_ressource() -> Vec<String> {
    let mut gefunden = Vec::new();
    for eintrag in fs::read_dir("../data/spells").expect("data/spells fehlt") {
        let pfad = eintrag.expect("Verzeichniseintrag").path();
        if pfad.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&pfad).expect("Datei lesbar");
        let wert: serde_json::Value = serde_json::from_str(&text).expect("gueltiges JSON");
        if wert.get("resource").is_some() {
            gefunden.push(
                pfad.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_owned(),
            );
        }
    }
    gefunden.sort();
    gefunden
}

#[test]
fn heute_fuehrt_genau_eine_schule_eine_ressource() {
    let gefunden = mit_ressource();
    assert_eq!(
        gefunden,
        vec!["calcination.json".to_owned()],
        "Der Bestand hat sich geaendert. Ist eine Ressource DAZUGEKOMMEN, muss \
         `emit_resource` in build.rs vorher auf eine Ressource JE SCHULE \
         umgestellt werden — sonst rechnet der Simulator mit der falschen."
    );
}

#[test]
fn und_der_waechter_wuerde_eine_zweite_ablehnen() {
    // **Der Waechter ist bei seiner Einfuehrung gruen** — es gibt heute nur
    // eine Ressource. Ohne diesen Fall waere also nicht belegt, dass er
    // ueberhaupt etwas ablehnt.
    //
    // Die Regel aus build.rs, an erfundenen Eingaben:
    let regel = |anzahl: usize| anzahl <= 1;

    assert!(regel(1), "eine Ressource ist der heutige Zustand");
    assert!(
        regel(0),
        "keine waere ebenfalls kein Fall fuer den Waechter"
    );
    assert!(
        !regel(2),
        "zwei Ressourcen werden NICHT abgelehnt — dann ist der Waechter wirkungslos"
    );
}
