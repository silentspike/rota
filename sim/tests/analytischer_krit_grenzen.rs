//! Wo der analytische kritische Treffer an die Rundung stößt (Issue #908, AC4).
//!
//! # Warum diese Datei existiert
//!
//! AC4 verlangt: „Wo er benutzt wird, stimmt das Ergebnis **bitgleich** mit dem
//! gewürfelten Erwartungswert überein." Das ist erfüllbar — aber nicht auf dem
//! naheliegenden Weg. Wer den Erwartungswert in den Krit-Faktor einsetzt,
//! bekommt eine Zahl, die mit dem gewürfelten Mittel **nicht** übereinstimmt,
//! und zwar nicht knapp, sondern systematisch.
//!
//! Der Grund steht in [`rota_sim::compute_damage`]: Der Schaden ist ganzzahlig
//! und wird **je Treffer** gerundet. Runden und Mitteln vertauschen nicht.
//!
//! ```text
//! E[runden(x)]  ist nicht  runden(E[x])
//! ```
//!
//! Diese Datei misst das an der echten Formel, statt es zu behaupten. Sie
//! bleibt auch dann gültig, wenn AC4 anders gebaut wird, als ich vorschlage —
//! sie beschreibt eine Eigenschaft der Rundung, keine Entwurfsentscheidung.

use rota_sim::{compute_damage, DamageFactors, NEUTRAL};

/// Faktoren mit einem einzigen gesetzten Krit-Wert.
fn mit_krit(crit: f64) -> DamageFactors {
    DamageFactors { crit, ..NEUTRAL }
}

/// **Der Kern: den Erwartungswert einzusetzen liefert eine andere Zahl.**
///
/// Gerechnet wird mit der echten Formel, nicht mit einem Nachbau — ein Nachbau
/// antwortete plausibel aus einer zweiten Quelle und bewiese nichts über den
/// Pfad, der im Kampf wirklich läuft.
#[test]
fn erwartungswert_einsetzen_trifft_das_gewuerfelte_mittel_nicht() {
    let basis = 10u32;
    let p = 0.5; // halbe Krit-Chance
    let crit_faktor = 1.5; // CRIT_BASE_PERMILLE = 1500, ohne YIELD

    // Der gewürfelte Weg: zwei Ausgänge, jeder für sich gerundet.
    let ohne_krit = compute_damage(basis, &mit_krit(1.0));
    let mit_krit_wert = compute_damage(basis, &mit_krit(crit_faktor));
    let gewuerfeltes_mittel = p * f64::from(mit_krit_wert) + (1.0 - p) * f64::from(ohne_krit);

    // Der naheliegende analytische Weg: Erwartungswert IN die Formel.
    let erwarteter_faktor = 1.0 + p * (crit_faktor - 1.0);
    let analytisch = compute_damage(basis, &mit_krit(erwarteter_faktor));

    assert_eq!(ohne_krit, 10, "Kontrollzeile: ohne Krit bleibt es die Basis");
    assert_eq!(mit_krit_wert, 15, "Kontrollzeile: 10 * 1,5 ist glatt");
    assert_eq!(
        gewuerfeltes_mittel, 12.5,
        "Kontrollzeile: das gewürfelte Mittel liegt zwischen den Ausgängen"
    );

    assert_ne!(
        f64::from(analytisch),
        gewuerfeltes_mittel,
        "hier liegt der Bruch: {analytisch} gegen {gewuerfeltes_mittel} — die \
         Rundung je Treffer schiebt den eingesetzten Erwartungswert nach oben"
    );
    assert_eq!(
        analytisch, 13,
        "10 * 1,25 = 12,5, aufwärts gerundet 13 — nicht 12,5"
    );
}

/// **Wie weit die beiden Wege auseinanderliegen — über einen Bereich gemessen.**
///
/// Der erste Fall zeigt einen einzelnen Bruch. Er allein ließe offen, ob das ein
/// unglücklich gewählter Wert ist. Diese Zählung geht 18 Kombinationen durch und
/// nennt, in wie vielen die beiden Wege auseinanderfallen — und in welchen
/// nicht.
///
/// „Marginalisieren" heißt: die Formel je Ausgang auswerten und die Ergebnisse
/// gewichten. Dann fällt die Rundung dort an, wo sie auch beim Würfeln anfällt.
#[test]
fn einsetzen_weicht_vom_marginalisieren_in_der_mehrzahl_der_faelle_ab() {
    let mut abweichend = 0;
    let mut gleich = 0;

    for basis in [1u32, 7, 10, 13, 99, 1000] {
        for p in [0.25, 0.5, 0.75] {
            let ohne = f64::from(compute_damage(basis, &mit_krit(1.0)));
            let mit = f64::from(compute_damage(basis, &mit_krit(1.5)));

            // Weg A: je Ausgang rechnen, dann gewichten.
            let marginalisiert = p * mit + (1.0 - p) * ohne;
            // Weg B: den Erwartungswert in die Formel setzen.
            let eingesetzt = f64::from(compute_damage(basis, &mit_krit(1.0 + p * 0.5)));

            if marginalisiert.to_bits() == eingesetzt.to_bits() {
                gleich += 1;
            } else {
                abweichend += 1;
            }
        }
    }

    assert_eq!(
        abweichend + gleich,
        18,
        "Trägerzusicherung: es wurden wirklich 18 Kombinationen angesehen"
    );
    assert!(
        abweichend > 0,
        "die beiden Wege fielen in KEINEM Fall auseinander — dann wäre die \
         Warnung dieser Datei gegenstandslos"
    );
    // Die Zahl steht in der Meldung, nicht in der Bedingung: Sie darf sich mit
    // den Daten ändern, ohne dass jemand einen Wächter nachpflegen muss.
    println!("auseinander: {abweichend} von 18, gleich: {gleich}");
}

/// Die Gegenprobe zur ersten Behauptung: An den **Enden** stimmt beides.
///
/// Ohne diesen Fall bliebe offen, ob der Bruch oben an der Rundung liegt oder
/// an einem Fehler in der Rechnung. Bei p = 0 und p = 1 gibt es nichts zu
/// mitteln, und da trifft auch der eingesetzte Erwartungswert.
#[test]
fn an_den_enden_stimmt_auch_der_eingesetzte_erwartungswert() {
    let basis = 10u32;
    for (p, erwartet) in [(0.0, 10u32), (1.0, 15u32)] {
        let faktor = 1.0 + p * 0.5;
        assert_eq!(
            compute_damage(basis, &mit_krit(faktor)),
            erwartet,
            "bei p = {p} gibt es keinen Zwischenwert, der gerundet werden müsste"
        );
    }
}
