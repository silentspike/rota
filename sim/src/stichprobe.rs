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

/// Die Wahrscheinlichkeit, bei `n` Versuchen mit Erfolgsrate `p` **höchstens**
/// `k` Fehler zu sehen.
///
/// Die Binomial-Verteilungsfunktion, aufsummiert — **im Log-Raum** und über die
/// **kürzere Flanke** der Verteilung.
///
/// # Warum im Log-Raum (Issue #915, AC3)
///
/// Die erste Fassung rechnete die Glieder direkt und begann mit `p^n`. Bei
/// kleiner Erfolgsrate unterläuft dieser Startterm zu `0.0`, und alle weiteren
/// Glieder entstehen als Vielfache davon — die ganze Summe blieb null. Genau
/// das trifft die gespiegelte Obergrenze in [`gestaffelt`]: Dort ist die
/// Erfolgsrate der Spiegelung die FEHLERrate, also klein. Gemessen
/// (`tests/stichprobe_grenzen.rs`): Nach EINEM Fehler bei Versuch 600 meldete
/// die Folge die Schwelle 0,999 bei 1024 Versuchen als sicher verfehlt, und
/// die Obergrenze stand dort bei 0,517 statt bei 0,99999993.
///
/// Dazu kam `n as i32`: Ab `n = 2^31` kippte der Exponent ins Negative, ab
/// `2^32` auf null. Die Stichprobe hat keine feste Obergrenze, also ist das
/// kein theoretischer Rand.
///
/// Im Log-Raum gibt es weder das eine noch das andere: Der Startterm ist
/// `n · ln p`, eine gewöhnliche negative Zahl, und `n` geht als `f64` ein.
///
/// # Warum über die kürzere Flanke
///
/// Summiert werden höchstens `min(k + 1, n - k)` Glieder. Liegt `k` in der
/// oberen Hälfte, wird das Gegenereignis gerechnet — „mindestens `k + 1`
/// Fehler" ist dasselbe wie „höchstens `n - k - 1` Erfolge" — und von eins
/// abgezogen, über `exp_m1`, damit kleine Ergebnisse nicht in der Subtraktion
/// verschwinden. Die gespiegelte Obergrenze nach einem Fehler kostet dadurch
/// ein Glied statt `n`.
///
/// Die Laufzeit ist damit linear in `min(k, n - k)`, je Aufruf; [`untergrenze`]
/// ruft die Funktion 200-mal.
#[must_use]
pub fn hoechstens_k_fehler(n: u64, k: u64, p: f64) -> f64 {
    if k >= n {
        return 1.0;
    }
    if p >= 1.0 {
        // Bei sicherer Erfolgsrate gibt es null Fehler, also höchstens k.
        return 1.0;
    }
    if p <= 0.0 {
        // Bei sicherem Fehlschlag sind es n Fehler — mehr als k.
        return 0.0;
    }
    // **`p` ist die ERFOLGSrate.** Ein Fehler tritt mit `1 - p` ein, also ist
    // die Wahrscheinlichkeit für genau `i` Fehler `C(n,i)·(1−p)^i·p^(n−i)`.
    //
    // Die erste Fassung stand mit vertauschten Rollen da (`(1−p)^n` als Term
    // für null Fehler) und lieferte bei `n = 1, k = 0, alpha = 0,01` den Wert
    // 0,99 statt 0,01 — die Gegenprobe gegen die geschlossene Formel hat es
    // gefangen, das Lesen nicht.
    //
    // Beide Logarithmen werden aus `p` selbst gebildet: `ln(1 - p)` über
    // `ln_1p(-p)`, damit eine kleine Erfolgsrate nicht erst in `1 - p` gerundet
    // wird. Beim Gegenereignis tauschen sie nur die Rollen.
    let ln_erfolg = p.ln();
    let ln_fehler = (-p).ln_1p();
    if k < n - k {
        ln_hoechstens(n, k, ln_erfolg, ln_fehler).exp().min(1.0)
    } else {
        // `k < n` ist oben geprüft, also ist `n - k - 1` kein Unterlauf.
        (-ln_hoechstens(n, n - k - 1, ln_fehler, ln_erfolg).exp_m1()).clamp(0.0, 1.0)
    }
}

/// `ln` der Wahrscheinlichkeit für höchstens `k` Fehler in `n` Versuchen —
/// gegeben als die beiden Logarithmen von Erfolgs- und Fehlerrate.
///
/// Die Glieder entstehen über denselben fortlaufenden Faktor wie vorher, nur
/// addiert statt multipliziert: Glied `i` ist Glied `i - 1` plus
/// `ln((n - i + 1) / i) + ln(Fehlerrate) - ln(Erfolgsrate)`. Fakultäten kommen
/// nicht vor, `C(n,i)` liefe sonst schon bei dreistelligem `n` über.
///
/// Summiert wird als log-sum-exp mit laufendem Maximum: Jedes Glied wird
/// relativ zum größten bisher gesehenen genommen, also bleibt jede einzelne
/// Exponentialfunktion im Bereich bis eins, und die Summe läuft weder über noch
/// unter.
fn ln_hoechstens(n: u64, k: u64, ln_erfolg: f64, ln_fehler: f64) -> f64 {
    let n_f = n as f64;
    let schritt = ln_fehler - ln_erfolg;
    // i = 0: alle n Versuche gelingen, ln(p^n) = n · ln p.
    let mut glied = n_f * ln_erfolg;
    let mut groesstes = glied;
    let mut summe = 1.0f64;
    for i in 1..=k {
        let i_f = i as f64;
        glied += ((n_f - i_f + 1.0) / i_f).ln() + schritt;
        if glied > groesstes {
            summe = summe * (groesstes - glied).exp() + 1.0;
            groesstes = glied;
        } else {
            summe += (glied - groesstes).exp();
        }
    }
    groesstes + summe.ln()
}

/// Die einseitige untere Schranke für die Erfolgsrate bei `k` Fehlern in `n`
/// Versuchen, zur Irrtumswahrscheinlichkeit `alpha`.
///
/// Gesucht ist das kleinste `p`, für das `hoechstens_k_fehler(n, k, p) <= alpha`
/// noch **nicht** gilt — also der Punkt, an dem die beobachtete Fehlerzahl
/// gerade noch verträglich ist. Gefunden per Bisektion, weil die Umkehrung der
/// Verteilungsfunktion keine geschlossene Form hat.
///
/// **Für `k = 0` muss das Ergebnis `alpha^(1/n)` sein** — und genau das prüft
/// `die_bisektion_trifft_die_geschlossene_formel_bei_null_fehlern`. Ohne diese
/// Gegenprobe wäre die Bisektion eine Zahl ohne Bezugspunkt.
#[must_use]
pub fn untergrenze(n: u64, k: u64, alpha: f64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    // **`hoechstens_k_fehler` STEIGT mit `p`**: Je höher die Erfolgsrate, desto
    // wahrscheinlicher sind wenige Fehler. Gesucht ist das kleinste `p`, das
    // die beobachteten `k` Fehler noch mit `alpha` verträglich macht — also die
    // Stelle, an der die Funktion `alpha` von unten erreicht.
    //
    // Die Suchrichtung war in der ersten Fassung verkehrt (sie behandelte die
    // Funktion als fallend). Zusammen mit den vertauschten Rollen in
    // `hoechstens_k_fehler` hoben sich die beiden Fehler bei manchen Werten
    // fast auf — genau deshalb fiel nur der Randfall `n = 1` sofort auf.
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..200 {
        let mitte = 0.5 * (lo + hi);
        if hoechstens_k_fehler(n, k, mitte) >= alpha {
            hi = mitte;
        } else {
            lo = mitte;
        }
    }
    hi
}

/// Eine gestaffelte Stichprobe fahren (AC3).
///
/// Geprüft wird nur an **Haltepunkten** (1, 2, 4, 8, …), und jeder bekommt sein
/// eigenes Irrtumsbudget aus [`budget_am_haltepunkt`]. Damit bleibt die Aussage
/// gültig, egal wie oft hingesehen wird.
///
/// `budget` ist eine **Obergrenze für die Versuche**, keine Voreinstellung für
/// die Stichprobengröße: Wird sie erreicht, ohne dass eine Seite sicher ist,
/// kommt [`Befund::Ungeklaert`] heraus — **keine Zahl**.
///
/// `versuch(i)` liefert `true` für einen Erfolg. Der Aufrufer kann darin auch
/// seinen Fortschritt wegschreiben; das Modul tut es nicht selbst, weil es
/// nicht weiß, wohin.
pub fn gestaffelt<F>(schwelle: f64, alpha: f64, budget: u64, mut versuch: F) -> Befund
where
    F: FnMut(u64) -> bool,
{
    let mut fehler = 0u64;
    let mut naechster_halt = 1u64;
    let mut haltepunkt = 1u32;

    for i in 0..budget {
        if !versuch(i) {
            fehler += 1;
        }
        let n = i + 1;
        if n < naechster_halt {
            continue;
        }
        naechster_halt = naechster_halt.saturating_mul(2);
        let a = budget_am_haltepunkt(haltepunkt, alpha);
        haltepunkt += 1;

        if untergrenze(n, fehler, a) >= schwelle {
            return Befund::Erfuellt { versuche: n };
        }
        // Spiegelbildlich: Ist selbst die OBERE Schranke unter der Forderung,
        // ist die Sache in die andere Richtung entschieden.
        let obergrenze = 1.0 - untergrenze(n, n.saturating_sub(fehler), a);
        if obergrenze < schwelle {
            return Befund::Verfehlt { versuche: n };
        }
    }

    Befund::Ungeklaert {
        versuche: budget,
        fehler,
    }
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

    /// **Die Bisektion trifft die geschlossene Formel — bei null Fehlern.**
    ///
    /// Ohne diesen Vergleich wäre [`untergrenze`] eine Zahl ohne Bezugspunkt:
    /// Sie käme aus einer Schleife, und niemand wüsste, ob die Schleife das
    /// Richtige sucht. Für `k = 0` ist die Antwort bekannt (`alpha^(1/n)`), und
    /// genau daran wird sie gemessen.
    #[test]
    fn die_bisektion_trifft_die_geschlossene_formel_bei_null_fehlern() {
        for n in [1u64, 10, 256, 1000, 4603] {
            let bisektion = untergrenze(n, 0, 0.01);
            let formel = untergrenze_ohne_fehler(n, 0.01);
            assert!(
                (bisektion - formel).abs() < 1e-9,
                "n = {n}: Bisektion {bisektion}, Formel {formel}"
            );
        }
        println!("Bisektion und geschlossene Formel stimmen über fünf Größen überein");
    }

    /// Mehr Fehler senken die Schranke — sonst misst sie die Fehler nicht.
    #[test]
    fn mehr_fehler_senken_die_untergrenze() {
        let n = 1000;
        let mut vorher = 1.0;
        for k in [0u64, 1, 5, 20, 100] {
            let g = untergrenze(n, k, 0.01);
            assert!(
                g < vorher,
                "bei {k} Fehlern ist die Schranke {g}, vorher {vorher} — sie \
                 müsste FALLEN"
            );
            vorher = g;
        }
        println!("Untergrenze fällt monoton mit der Fehlerzahl");
    }

    /// **AC3: bei Budgetende kommt UNGEKLÄRT heraus, keine Zahl.**
    #[test]
    fn ein_zu_kleines_budget_liefert_ungeklaert() {
        // 99,9 Prozent verlangen 4603 fehlerfreie Versuche — mit 256 ist das
        // nicht zu schaffen, auch wenn ALLE gelingen.
        let befund = gestaffelt(0.999, 0.01, 256, |_| true);
        assert_eq!(
            befund,
            Befund::Ungeklaert {
                versuche: 256,
                fehler: 0
            },
            "mit 256 fehlerfreien Versuchen darf die Schwelle NICHT als erfüllt \
             gelten — das ist die Rechnung aus dem Issue"
        );

        // Und die Gegenprobe: Mit genug Budget kippt derselbe Aufruf.
        //
        // **8192 genügen NICHT**, und das ist kein Rechenfehler, sondern der
        // Preis der jederzeit gültigen Aussage: Am k-ten Haltepunkt steht nur
        // noch `alpha/(k·(k+1))` zur Verfügung. Bei 8192 Versuchen ist der 14.
        // Haltepunkt erreicht, das Budget also auf ein Zweihundertzehntel
        // geschrumpft — und dafür reichen 8192 fehlerfreie Versuche nicht.
        let genug = gestaffelt(0.999, 0.01, 1 << 17, |_| true);
        match genug {
            Befund::Erfuellt { versuche } => {
                let ohne_staffelung = noetige_versuche_ohne_fehler(0.999, 0.01);
                assert!(
                    versuche >= ohne_staffelung,
                    "erfüllt nach nur {versuche} Versuchen, ungestaffelt bräuchte \
                     es schon {ohne_staffelung} — das wäre zu früh"
                );

                // **Der Preis, gemessen statt geschätzt.** AC3 verlangt, dass im
                // Bericht steht, wie viele Versuche für die verlangte Sicherheit
                // tatsächlich nötig waren — nicht nur, dass die Schwelle
                // erreicht wurde.
                let preis = versuche as f64 / ohne_staffelung as f64;
                assert!(
                    preis > 1.0,
                    "die Staffelung kostet nichts (Faktor {preis}) — dann wäre \
                     das Irrtumsbudget nicht wirklich aufgeteilt"
                );
                println!(
                    "erfüllt nach {versuche} Versuchen; ungestaffelt genügten \
                     {ohne_staffelung}, Faktor {preis:.2}"
                );
            }
            anderer => {
                panic!("mit 2^17 fehlerfreien Versuchen erwartet: Erfuellt, war: {anderer:?}")
            }
        }
    }

    /// Eine Rate deutlich UNTER der Schwelle wird als verfehlt erkannt.
    #[test]
    fn eine_schlechte_rate_wird_als_verfehlt_erkannt() {
        // Jeder zweite Versuch scheitert — 99,9 Prozent sind ausgeschlossen.
        let befund = gestaffelt(0.999, 0.01, 4096, |i| i % 2 == 0);
        match befund {
            Befund::Verfehlt { versuche } => {
                assert!(
                    versuche < 4096,
                    "erst am Budgetende erkannt — dann hätte die Folge nichts \
                     gespart"
                );
                println!("verfehlt nach {versuche} Versuchen statt nach 4096");
            }
            anderer => panic!("bei halber Erfolgsrate erwartet: Verfehlt, war: {anderer:?}"),
        }
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
