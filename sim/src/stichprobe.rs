//! Gestaffelte Stichprobe mit jederzeit gültiger Aussage (Issue #915, AC3).
//!
//! # Warum eine feste Obergrenze nicht geht
//!
//! Das Issue rechnet es vor, und der Prüffall unten rechnet es nach: Bei **null**
//! beobachteten Fehlern liegt die einseitige 99-Prozent-Untergrenze nach 256
//! Versuchen bei
//!
//! ```text
//! 0,01^(1/256) = 98,22 %
//! ```
//!
//! Für eine Untergrenze von 99,9 Prozent braucht es
//!
//! ```text
//! n ≥ ln(0,01) / ln(0,999) ≈ 4 603
//! ```
//!
//! fehlerfreie Versuche. Eine harte Bedingung wie „Mechanikerfolg mindestens
//! 99,9 Prozent" ist mit 256 Versuchen also **nicht** zertifizierbar — nicht
//! knapp, sondern um den Faktor 18.
//!
//! # Warum „jederzeit gültig" nicht dasselbe ist wie „gültig"
//!
//! Die übliche Schranke (Clopper-Pearson und Verwandte) gilt für ein **vorher
//! festgelegtes** `n`. Wer sie nach jedem Durchgang prüft und beim ersten
//! Überschreiten stehenbleibt, hat sie damit ungültig gemacht: Das wiederholte
//! Hinsehen ist selbst eine Auswahl, und die Irrtumswahrscheinlichkeit summiert
//! sich über alle Blicke.
//!
//! > Genau das braucht dieses Issue aber — der Lauf soll laufen, **bis** die
//! > Schwelle sicher über- oder unterschritten ist. Also muss die Schranke das
//! > Hinsehen aushalten.
//!
//! Der hier gewählte Weg ist der einfachste, der nachweisbar trägt: **Prüfen nur
//! an geometrisch wachsenden Haltepunkten** (1, 2, 4, 8, …) und das Irrtums-
//! budget über eine konvergente Reihe darauf verteilen. Die Summe der Einzel-
//! risiken bleibt dann unter `alpha`, egal wie oft geprüft wird.
//!
//! ```text
//! alpha_k = alpha / (k · (k+1))        Σ = alpha · Σ 1/(k(k+1)) = alpha
//! ```
//!
//! Das ist konservativer als eine moderne e-Wert-Folge und dafür in drei Zeilen
//! nachrechenbar — und der Prüffall rechnet die Reihe nach, statt ihr zu
//! glauben.

/// Das Ergebnis einer gestaffelten Stichprobe.
///
/// **`Ungeklaert` ist ein vollwertiges Ergebnis, kein Fehler.** Wer bei
/// Budgetende eine Zahl liefert, macht aus „wir wissen es nicht" ein „so ist es"
/// — und dem Wert sieht danach niemand mehr an, welcher von beiden er war.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Befund {
    /// Die Schwelle ist sicher **über**schritten.
    Erfuellt {
        /// Wie viele Versuche dafür nötig waren.
        versuche: u64,
    },
    /// Die Schwelle ist sicher **unter**schritten.
    Verfehlt {
        /// Wie viele Versuche dafür nötig waren.
        versuche: u64,
    },
    /// Das Budget ist aufgebraucht, ohne dass eine Seite sicher war.
    Ungeklaert {
        /// Wie viele Versuche gefahren wurden.
        versuche: u64,
        /// Wie viele Fehler dabei auftraten.
        fehler: u64,
    },
}

/// Wie viele fehlerfreie Versuche nötig sind, um `schwelle` mit Sicherheit
/// `1 - alpha` zu belegen.
///
/// Die Umkehrung der Regel „bei null Fehlern ist die einseitige Untergrenze
/// `alpha^(1/n)`".
#[must_use]
pub fn noetige_versuche_ohne_fehler(schwelle: f64, alpha: f64) -> u64 {
    if !(0.0..1.0).contains(&schwelle) || alpha <= 0.0 || alpha >= 1.0 {
        return 0;
    }
    (alpha.ln() / schwelle.ln()).ceil() as u64
}

/// Die einseitige Untergrenze bei `n` fehlerfreien Versuchen.
#[must_use]
pub fn untergrenze_ohne_fehler(n: u64, alpha: f64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    alpha.powf(1.0 / n as f64)
}

/// Das Irrtumsbudget für den `k`-ten Haltepunkt (k ab 1).
///
/// `alpha / (k·(k+1))` — die Reihe summiert sich zu `alpha`, unabhängig davon,
/// wie viele Haltepunkte tatsächlich erreicht werden.
#[must_use]
pub fn budget_am_haltepunkt(k: u32, alpha: f64) -> f64 {
    let k = f64::from(k);
    alpha / (k * (k + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Die Zahlen des Issues, nachgerechnet statt übernommen.**
    #[test]
    fn die_beiden_zahlen_des_issues_stimmen() {
        let nach_256 = untergrenze_ohne_fehler(256, 0.01);
        assert!(
            (nach_256 - 0.9822).abs() < 0.0001,
            "nach 256 fehlerfreien Versuchen: {nach_256}, erwartet rund 0,9822"
        );
        assert!(
            nach_256 < 0.999,
            "und das ist die Aussage: {nach_256} reicht NICHT für 99,9 Prozent"
        );

        let noetig = noetige_versuche_ohne_fehler(0.999, 0.01);
        assert_eq!(
            noetig, 4603,
            "für eine 99-Prozent-Untergrenze von 99,9 Prozent"
        );

        // Die Gegenprobe zur Umkehrung: Mit genau dieser Zahl wird die Schwelle
        // erreicht, mit einem Versuch weniger nicht.
        assert!(untergrenze_ohne_fehler(noetig, 0.01) >= 0.999);
        assert!(untergrenze_ohne_fehler(noetig - 1, 0.01) < 0.999);

        println!("256 -> {nach_256:.4}; für 0,999 nötig: {noetig}");
    }

    /// **Der Faktor, der die feste Obergrenze widerlegt.**
    ///
    /// Nicht „knapp zu wenig", sondern um eine Größenordnung.
    #[test]
    fn eine_feste_obergrenze_von_256_ist_nicht_knapp_daneben() {
        let noetig = noetige_versuche_ohne_fehler(0.999, 0.01);
        let faktor = noetig as f64 / 256.0;
        assert!(
            faktor > 15.0,
            "Faktor nur {faktor} — dann wäre die feste Obergrenze eine Frage der \
             Bequemlichkeit und nicht der Zertifizierbarkeit"
        );
        println!("nötig {noetig}, erlaubt 256, Faktor {faktor:.1}");
    }

    /// **Die Reihe summiert sich zu `alpha` — nachgerechnet, nicht behauptet.**
    ///
    /// Darauf steht die ganze Aussage „jederzeit gültig": Wenn die Summe der
    /// Einzelrisiken über `alpha` läge, wäre die Folge beim häufigen Hinsehen
    /// wertlos, und niemand sähe es der einzelnen Schranke an.
    #[test]
    fn das_irrtumsbudget_summiert_sich_zu_alpha() {
        let alpha = 0.01;
        let mut summe = 0.0;
        for k in 1..=100_000u32 {
            summe += budget_am_haltepunkt(k, alpha);
        }
        // Σ 1/(k(k+1)) = 1 - 1/(N+1), also knapp unter alpha.
        assert!(
            summe < alpha,
            "die Summe {summe} überschreitet das Budget {alpha}"
        );
        assert!(
            summe > alpha * 0.9999,
            "die Summe {summe} ist viel kleiner als {alpha} — dann verschenkt \
             die Aufteilung Aussagekraft"
        );
        println!("Summe über 100 000 Haltepunkte: {summe:.9} von {alpha}");
    }

    /// Ein Budget, das sich NICHT summieren ließe, fiele auf.
    ///
    /// Ohne diesen Fall bliebe offen, ob der Fall darüber die Reihe prüft oder
    /// nur eine kleine Zahl.
    #[test]
    fn ein_konstantes_budget_je_haltepunkt_wuerde_das_alpha_sprengen() {
        let alpha = 0.01;
        let konstant = alpha / 10.0;
        let summe: f64 = (1..=100).map(|_| konstant).sum();
        assert!(
            summe > alpha,
            "ein konstantes Budget müsste das alpha überschreiten, sonst prüft \
             der Vergleich oben nichts"
        );
        println!("konstantes Budget über 100 Haltepunkte: {summe} statt {alpha}");
    }
}
