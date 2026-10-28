//! Die Restzeit: gewichtet, simuliert und mit hergeleiteter Spanne (AC3, AC5–AC7).
//!
//! # Drei Fehler, gegen die dieses Modul gebaut ist
//!
//! 1. **Restkandidaten durch Kandidaten je Sekunde** (AC5). Eine Auswertung auf
//!    dem Weg „ein Kampf" kostet einen Kampf, eine feste Stichprobe kostet
//!    `iterations`. Die Division ohne Gewicht wird genau dann falsch, wenn der
//!    Verteiler gut arbeitet.
//! 2. **Restzyklen durch Zyklen je Sekunde** (AC6). Die gewichtete Division
//!    setzt voraus, dass jede Aufgabe auf jedem Strang gleich läuft und alles
//!    parallel geht. Hier gilt das ausdrücklich nicht: Eine Auswertung auf dem
//!    Weg „ein Kampf" ist EIN Kampf auf EINEM Strang, und die nächste beginnt
//!    erst nach dem Sammelpunkt. Vier Stränge machen diese Kette nicht viermal
//!    schneller.
//! 3. **Eine Spanne ohne Herleitung** (AC7). Sie wäre eine erfundene Zahl.
//!
//! # Die Kosten kommen aus einem `RateModel`
//!
//! Gewicht einer Aufgabe = ihre Einheiten geteilt durch die Rate der
//! zugehörigen [`Arbeitsart`] auf dem Strang, der sie rechnet. Die Rate ist ein
//! [`RateModel`] — derselbe Vertrag wie in der Fähigkeitserhebung (#912), mit
//! Streuung und Abschnittszahl. Heute rechnet jeder Weg in Kämpfen, also in
//! `ScalarFullFight`; ein späterer Weg in einer anderen Einheit bekommt seine
//! eigene Art, und ein Rechner ohne diese Art ist für ihn nicht zulässig.
//!
//! # Was die Rechnung NICHT enthält — gemessen am 2026-10-28
//!
//! **Die Nacharbeit einer Auswertung nach ihrem letzten Kampf** (Sortieren der
//! Stichprobe, Statistik über sechs Reihen). In einem Lauf aus vielen kleinen
//! Auswertungen steckt sie in der gemessenen Rate, weil die Arbeitsstränge
//! währenddessen warten. Bei einer einzigen großen fehlt sie: Mit
//! `bench --fights 3000000` über vier Stränge stand die Meldung mit allen
//! Kämpfen 0,61 s vor dem Ende, und die 16 von 89 Meldungen, deren Spanne die
//! tatsächliche Restzeit nicht einschloss, lagen alle in den letzten vier
//! Sekunden — alle zu kurz geschätzt.
//!
//! **Teurer werdende Kämpfe einer Suche**, soweit das Fenster sie noch nicht
//! gesehen hat. Der Trend erfasst, was sich in den letzten 30 Sekunden schon
//! verschoben hat; spätere Runden mit längeren Rotationen kann er nicht
//! vorhersehen. Bei einer Suche über vier Stränge lagen 13 von 21 Meldungen
//! in ihrer Spanne, die acht übrigen alle zu kurz und alle in den ersten
//! zwei Sekunden nach der ersten Restzeit.

use super::fenster::Raten;
use super::zaehlwerk::{Momentaufnahme, Weg};
use crate::kalibrierung::{Arbeitsart, RateModel};

/// Wahrscheinlichkeit, dass eine Aufgabe im lokalen Lauf wiederholt werden muss.
///
/// **Null, und das ist gemessen am Bau, nicht angenommen:** Ein lokaler Kampf
/// scheitert nicht. Ein Strang, der abstürzt, beendet den ganzen Lauf
/// (`expect("ein Strang ist abgestuerzt")` in [`crate::balance`]) — es gibt
/// keinen Weg, auf dem eine Aufgabe ein zweites Mal gerechnet würde. Der Term
/// steht trotzdem in der Herleitung, mit diesem Wert: Sobald Zusatzrechner
/// Pakete verlieren können (E15.5), ist er der erste, der wächst.
pub const WIEDERHOLUNG_LOKAL: f64 = 0.0;

/// Ein Rechner, wie die Ablaufsimulation ihn sieht.
#[derive(Debug, Clone)]
pub struct Rechner {
    /// Kennung, etwa der Hostname.
    pub id: String,
    /// Wie viele Stränge er für diesen Lauf rechnet.
    pub straenge: usize,
    /// Seine Rate je Strang und [`Arbeitsart`]. Fehlt eine Art, darf er
    /// Aufgaben dieser Art nicht rechnen — etwa gonet ohne AVX2.
    pub raten: Vec<(Arbeitsart, RateModel)>,
    /// Wie stark seine Rate gerade wandert, als Anteil — der gemessene Teil
    /// des Restkostenfehlers ([`super::Fenster::strangtrend`]). 0 ohne Trend.
    pub trend: f64,
}

/// Eine Stufe: Aufgaben, die gleichzeitig laufen dürfen. Zwischen zwei Stufen
/// liegt ein Sammelpunkt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stufe {
    /// Was gerechnet wird.
    pub art: Arbeitsart,
    /// Wie viele gleichartige Aufgaben die Stufe hat.
    pub aufgaben: u64,
    /// Einheiten der [`Arbeitsart`] je Aufgabe — bei Kämpfen: Kämpfe.
    pub einheiten_je_aufgabe: f64,
    /// Wie oft diese Stufe nacheinander kommt.
    pub wiederholungen: u64,
}

/// Was die Ablaufsimulation ergibt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ablauf {
    /// Simulierte Dauer bis zum letzten Sammelpunkt, in Sekunden.
    pub dauer_s: f64,
    /// Die Kette, die auch unendlich viele Stränge nicht verkürzen: je Stufe die
    /// längste einzelne Aufgabe auf dem schnellsten zulässigen Strang.
    pub kritischer_pfad_s: f64,
    /// Die Division, die AC6 verwirft: alle Einheiten durch alle Raten.
    pub naiv_s: f64,
    /// Wie viele Rechner Aufgaben bekommen konnten.
    pub rechner: usize,
}

/// Warum keine Ablaufsimulation möglich ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ablauffehler {
    /// Kein Rechner kann diese Arbeitsart rechnen.
    KeinRechner(Arbeitsart),
    /// Eine Rate ist null, negativ oder keine Zahl.
    UngueltigeRate(Arbeitsart),
}

/// Wie viele Kämpfe eine Auswertung kostet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kampfzahl {
    /// Genau so viele — der feste Weg.
    Fest(u64),
    /// Höchstens so viele — die gestaffelte Stichprobe endet früher, sobald die
    /// Aussage sicher ist.
    Hoechstens(u64),
}

/// Was eine Auswertung dieses Laufs kostet und wie sie sich verteilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kostenbild {
    /// Der Weg, den jede Auswertung dieses Laufs nimmt.
    pub weg: Weg,
    /// Kämpfe je Auswertung.
    pub kaempfe_je_kandidat: Kampfzahl,
}

/// Wie genau die verbleibende Arbeit feststeht — der Restkostenfehler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restkosten {
    /// Auf den Kampf genau.
    Exakt {
        /// Verbleibende Kämpfe.
        kaempfe: u64,
    },
    /// Zwischen zwei Grenzen.
    Spanne {
        /// So viele verbleiben mindestens.
        mindestens: u64,
        /// Mehr können es nicht werden.
        hoechstens: u64,
    },
}

/// Woraus eine Restzeit entstanden ist (AC7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Herleitung {
    /// Wie genau die Restmenge feststeht.
    pub restkosten: Restkosten,
    /// Kämpfe je Sekunde und Strang, aus dem Fenster.
    pub rate_je_strang: f64,
    /// Relative Streuung dieser Rate über die Sekundenabschnitte.
    pub rate_streuung: f64,
    /// Wie viele Sekundenabschnitte in die Streuung eingingen.
    pub rate_abschnitte: usize,
    /// Wie stark die Rate im Fenster wandert, als Anteil — der Trend.
    pub kosten_trend: f64,
    /// Der Anteil der Spanne aus diesem Trend, in Sekunden.
    pub sigma_kosten_s: f64,
    /// Simulierte Dauer der kleinsten Restmenge.
    pub ablauf_min_s: f64,
    /// Simulierte Dauer der größten Restmenge.
    pub ablauf_max_s: f64,
    /// Kritischer Pfad der größten Restmenge.
    pub kritischer_pfad_s: f64,
    /// Rechner, über die gemittelt werden darf.
    pub rechner: usize,
    /// Der Anteil der Spanne aus der Ratenstreuung, in Sekunden.
    pub sigma_rate_s: f64,
    /// Wahrscheinlichkeit einer Wiederholung je Aufgabe.
    pub wiederholung_p: f64,
    /// Der Anteil der Spanne aus Wiederholungen, in Sekunden.
    pub sigma_wiederholung_s: f64,
    /// Zum Vergleich, AC5: Restkandidaten durch Kandidaten je Sekunde.
    pub naiv_kandidaten_s: Option<f64>,
    /// Zum Vergleich, AC6: Restkämpfe durch alle Stränge mal Rate.
    pub naiv_division_s: f64,
}

/// Die Restzeit eines Laufs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Restzeit {
    /// Eine Restzeit mit Spanne.
    Spanne {
        /// Der Mittelwert — nur bei exakt bekannter Restmenge.
        mitte_s: Option<f64>,
        /// Untere Grenze, in Sekunden.
        unten_s: f64,
        /// Obere Grenze, in Sekunden.
        oben_s: f64,
        /// Woraus sie entstanden ist.
        herleitung: Herleitung,
    },
    /// Offene Suche: **keine Restzeit**, sondern das Plateau.
    Plateau,
    /// Noch nicht bestimmbar — mit Grund, nicht mit einer Null.
    Unbestimmt(&'static str),
    /// Der Lauf ist zu Ende.
    Fertig,
}

/// Die Ablaufsimulation (AC6): Listenzuteilung je Stufe, Sammelpunkt zwischen
/// den Stufen.
///
/// # Errors
///
/// [`Ablauffehler`], wenn eine Stufe keinen zulässigen Rechner hat oder eine
/// Rate unbrauchbar ist.
pub fn ablauf(stufen: &[Stufe], rechner: &[Rechner]) -> Result<Ablauf, Ablauffehler> {
    let mut dauer_s = 0.0;
    let mut kritischer_pfad_s = 0.0;
    let mut naiv_s = 0.0;
    let mut benutzt = vec![false; rechner.len()];
    for stufe in stufen {
        if stufe.aufgaben == 0 || stufe.wiederholungen == 0 {
            continue;
        }
        // Die zulässigen Stränge dieser Stufe, je mit ihrer Rate.
        let mut straenge: Vec<f64> = Vec::new();
        for (i, r) in rechner.iter().enumerate() {
            let Some(modell) = rate_fuer(r, stufe.art) else {
                continue;
            };
            if !(modell.rate_per_s.is_finite() && modell.rate_per_s > 0.0) {
                return Err(Ablauffehler::UngueltigeRate(stufe.art));
            }
            if r.straenge > 0 {
                benutzt[i] = true;
                straenge.extend(std::iter::repeat_n(modell.rate_per_s, r.straenge));
            }
        }
        if straenge.is_empty() {
            return Err(Ablauffehler::KeinRechner(stufe.art));
        }
        let mal = stufe.wiederholungen as f64;
        let einheit = stufe.einheiten_je_aufgabe;
        dauer_s += spanne(stufe.aufgaben, einheit, &straenge) * mal;
        let schnellster = straenge.iter().copied().fold(0.0, f64::max);
        kritischer_pfad_s += einheit / schnellster * mal;
        naiv_s += stufe.aufgaben as f64 * einheit / straenge.iter().sum::<f64>() * mal;
    }
    Ok(Ablauf {
        dauer_s,
        kritischer_pfad_s,
        naiv_s,
        rechner: benutzt.iter().filter(|b| **b).count(),
    })
}

/// Die Rate eines Rechners für eine Arbeitsart.
fn rate_fuer(r: &Rechner, art: Arbeitsart) -> Option<&RateModel> {
    r.raten.iter().find(|(a, _)| *a == art).map(|(_, m)| m)
}

/// Wie lange `n` gleiche Aufgaben zu je `einheit` Einheiten auf Strängen mit
/// den Raten `raten` brauchen — Listenzuteilung, also die kürzeste Zeit `T`,
/// in der die Stränge zusammen `n` ganze Aufgaben schaffen.
///
/// Bei gleich schnellen Strängen ist das geschlossen `⌈n/W⌉ · einheit/rate`:
/// Fünf Aufgaben auf vier Strängen dauern zwei Aufgaben lang, nicht 1,25.
/// Bei verschiedenen Raten wird `T` gesucht und dann auf die Zuteilung
/// eingerastet, die es erreicht — sonst stünde dort die Genauigkeit der Suche
/// statt der Zeit einer Aufgabe.
fn spanne(n: u64, einheit: f64, raten: &[f64]) -> f64 {
    let erste = raten[0];
    if raten.iter().all(|r| r.to_bits() == erste.to_bits()) {
        let je_strang = n.div_ceil(raten.len() as u64);
        return je_strang as f64 * einheit / erste;
    }
    let schafft = |t: f64| -> u64 { raten.iter().map(|r| (t * r / einheit).floor() as u64).sum() };
    let schnellster = raten.iter().copied().fold(0.0, f64::max);
    let mut unten = 0.0;
    let mut oben = n as f64 * einheit / schnellster;
    for _ in 0..200 {
        let mitte = (unten + oben) / 2.0;
        if mitte <= unten || mitte >= oben {
            break;
        }
        if schafft(mitte) >= n {
            oben = mitte;
        } else {
            unten = mitte;
        }
    }
    raten
        .iter()
        .map(|r| (oben * r / einheit).floor() * einheit / r)
        .fold(0.0, f64::max)
}

/// Was auf einem Weg noch zu tun ist.
///
/// Ein Lauf von heute hat genau einen Anteil — alle seine Auswertungen nehmen
/// denselben Weg. Die Liste ist trotzdem eine Liste: Sobald ein Router
/// (E11.4b) Auswertungen auf verschiedene Wege schickt, steht jeder mit seinen
/// eigenen Kosten darin, und AC5 verlangt genau diese Gewichtung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Restanteil {
    /// Der Weg.
    pub weg: Weg,
    /// Kämpfe je Auswertung auf diesem Weg.
    pub kaempfe_je_kandidat: Kampfzahl,
    /// So viele Auswertungen stehen mindestens noch aus.
    pub mindestens: u64,
    /// Mehr können es nicht werden.
    pub hoechstens: u64,
}

/// Die Restzeit aus Restanteilen (AC5–AC7).
///
/// `begonnen` sind die Kämpfe, die die laufende Auswertung des ersten Anteils
/// schon gerechnet hat. `raten` dienen nur dem Vergleich mit der naiven
/// Division in der [`Herleitung`].
///
/// # Die Herleitung der Spanne (AC7)
///
/// Vier Quellen, jede mit ihrem eigenen Term:
///
/// ```text
///   Restkostenfehler     a) die Restmenge selbst: exakt, oder zwischen min und max
///                        b) die Kosten je Kampf: Trend τ der Strangrate im Fenster
///   Ratenstreuung        σ_r aus dem RateModel (Std/Mittel der Sekundenabschnitte)
///   kritischer Pfad      K aus der Ablaufsimulation
///   Wiederholung         p je Kampf, N Kämpfe
///
///   g        = 1 / (1 − p)                     erwartete Versuche je Kampf
///   T        = Ablaufdauer · g                 Mitte (bei exakter Menge)
///   σ_rate   = σ_r · (K + (T − K) / √H)        H = beteiligte Rechner
///   σ_kosten = τ · T
///   σ_wdh    = T · √(p / N)
///   σ        = √((σ_rate + σ_kosten)² + σ_wdh²)
/// ```
///
/// **Warum `σ_rate + σ_kosten` und nicht in Quadratur:** Beide sind
/// Eigenschaften derselben Rate. Wandert sie, ist sie auch unruhig; unabhängig
/// sind die beiden nicht, und die Quadratur rechnete die Spanne schmaler, als
/// die Messung hergibt. Wiederholungen dagegen hängen nicht an der Rate.
///
/// **Warum `√H` und nicht `√(Stränge)`:** Die Stränge eines Rechners drosseln
/// gemeinsam — wird der Laptop warm, werden alle zwölf langsamer. Unabhängig
/// voneinander schwanken nur verschiedene Rechner, und nur über die mittelt
/// sich der parallele Teil. Der kritische Pfad mittelt sich nie: Er läuft auf
/// einem Strang nach dem anderen, und eine Verzögerung dort verschiebt das
/// Ende ganz. Mit einem Rechner ist `σ_rate = σ_r · T` — die volle Streuung.
///
/// **Warum `T · √(p/N)`:** Bei Wiederholungen mit Wahrscheinlichkeit `p`
/// braucht ein Kampf im Mittel `g` Versuche, mit Varianz `p·g²`. Über `N`
/// unabhängige Kämpfe ist die relative Standardabweichung der Summe `√(p/N)`.
///
/// Die Spanne ist **eine** Standardabweichung. Bei einer nur nach oben
/// beschränkten Menge gibt es keine Mitte: Unten steht die kleinste Restmenge
/// minus σ, oben die größte plus σ.
#[must_use]
pub fn restzeit(
    anteile: &[Restanteil],
    begonnen: u64,
    rechner: &[Rechner],
    raten: Option<&Raten>,
    wiederholung_p: f64,
) -> Restzeit {
    if !(0.0..1.0).contains(&wiederholung_p) {
        return Restzeit::Unbestimmt("die Wiederholungswahrscheinlichkeit liegt nicht in [0, 1)");
    }
    let (stufen_min, kaempfe_min) = stufen_fuer(anteile, begonnen, Grenze::Mindestens);
    let (stufen_max, kaempfe_max) = stufen_fuer(anteile, begonnen, Grenze::Hoechstens);
    let (a_min, a_max) = match (ablauf(&stufen_min, rechner), ablauf(&stufen_max, rechner)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return Restzeit::Unbestimmt(grund(e)),
    };
    let genutzt: Vec<&RateModel> = rechner
        .iter()
        .filter_map(|r| rate_fuer(r, Arbeitsart::ScalarFullFight))
        .collect();
    let rate_streuung = genutzt.iter().map(|m| m.spread).fold(0.0, f64::max);
    if !rate_streuung.is_finite() {
        return Restzeit::Unbestimmt("die Rate streut unbestimmt — zu wenige Abschnitte");
    }
    let kosten_trend = rechner.iter().map(|r| r.trend).fold(0.0, f64::max);
    if !kosten_trend.is_finite() {
        return Restzeit::Unbestimmt("der Trend der Rate ist keine Zahl");
    }
    let exakt = kaempfe_min == kaempfe_max
        && anteile.iter().all(|a| {
            a.mindestens == a.hoechstens && matches!(a.kaempfe_je_kandidat, Kampfzahl::Fest(_))
        });

    let g = 1.0 / (1.0 - wiederholung_p);
    let h = (a_max.rechner.max(1) as f64).sqrt();
    let sigma_rate = |a: &Ablauf| {
        let t = a.dauer_s * g;
        let k = (a.kritischer_pfad_s * g).min(t);
        rate_streuung * (k + (t - k) / h)
    };
    let sigma_wdh = |a: &Ablauf, n: u64| {
        if wiederholung_p > 0.0 && n > 0 {
            a.dauer_s * g * (wiederholung_p / n as f64).sqrt()
        } else {
            0.0
        }
    };
    let sigma_kosten = |a: &Ablauf| kosten_trend * a.dauer_s * g;
    let sigma = |a: &Ablauf, n: u64| (sigma_rate(a) + sigma_kosten(a)).hypot(sigma_wdh(a, n));

    let herleitung = Herleitung {
        restkosten: if exakt {
            Restkosten::Exakt {
                kaempfe: kaempfe_max,
            }
        } else {
            Restkosten::Spanne {
                mindestens: kaempfe_min,
                hoechstens: kaempfe_max,
            }
        },
        rate_je_strang: genutzt.first().map_or(f64::NAN, |m| m.rate_per_s),
        rate_streuung,
        rate_abschnitte: genutzt.iter().map(|m| m.samples).min().unwrap_or(0),
        kosten_trend,
        sigma_kosten_s: sigma_kosten(&a_max),
        ablauf_min_s: a_min.dauer_s,
        ablauf_max_s: a_max.dauer_s,
        kritischer_pfad_s: a_max.kritischer_pfad_s * g,
        rechner: a_max.rechner,
        sigma_rate_s: sigma_rate(&a_max),
        wiederholung_p,
        sigma_wiederholung_s: sigma_wdh(&a_max, kaempfe_max),
        naiv_kandidaten_s: raten.filter(|r| r.kandidaten_gleitend > 0.0).map(|r| {
            anteile.iter().map(|a| a.hoechstens).sum::<u64>() as f64 / r.kandidaten_gleitend
        }),
        naiv_division_s: a_max.naiv_s,
    };

    if exakt {
        let mitte = a_max.dauer_s * g;
        let s = sigma(&a_max, kaempfe_max);
        Restzeit::Spanne {
            mitte_s: Some(mitte),
            unten_s: (mitte - s).max(0.0),
            oben_s: mitte + s,
            herleitung,
        }
    } else {
        Restzeit::Spanne {
            mitte_s: None,
            unten_s: (a_min.dauer_s * g - sigma(&a_min, kaempfe_min)).max(0.0),
            oben_s: a_max.dauer_s * g + sigma(&a_max, kaempfe_max),
            herleitung,
        }
    }
}

/// Welche Grenze der Restmenge in Stufen übersetzt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grenze {
    Mindestens,
    Hoechstens,
}

/// Die Restanteile als Stufen, dazu die Zahl der Kämpfe darin.
///
/// # Welcher Weg wie verteilt wird
///
/// Nur die feste Stichprobe verteilt ihre Kämpfe auf Stränge
/// ([`crate::balance`] teilt die Durchgänge in Bereiche). Der Weg „ein Kampf"
/// ist ein Kampf auf dem Leitstrang, und die gestaffelte Stichprobe rechnet in
/// einem Strang, Versuch für Versuch — beide sind **eine** Aufgabe je
/// Auswertung, und zwischen zwei Auswertungen liegt ein Sammelpunkt.
///
/// Bei [`Kampfzahl::Hoechstens`] kostet eine Auswertung höchstens das Budget,
/// mindestens aber einen Kampf — und die schon begonnene keinen weiteren mehr,
/// denn sie kann an ihrem nächsten Haltepunkt enden.
fn stufen_fuer(anteile: &[Restanteil], begonnen: u64, grenze: Grenze) -> (Vec<Stufe>, u64) {
    let mut stufen = Vec::new();
    let mut kaempfe = 0;
    for (i, a) in anteile.iter().enumerate() {
        let anzahl = match grenze {
            Grenze::Mindestens => a.mindestens,
            Grenze::Hoechstens => a.hoechstens,
        };
        if anzahl == 0 {
            continue;
        }
        let schon = if i == 0 { begonnen } else { 0 };
        let (je, erste) = match (a.kaempfe_je_kandidat, grenze) {
            (Kampfzahl::Fest(f), _) | (Kampfzahl::Hoechstens(f), Grenze::Hoechstens) => {
                (f, f.saturating_sub(schon))
            }
            (Kampfzahl::Hoechstens(_), Grenze::Mindestens) => (1, u64::from(schon == 0)),
        };
        let parallel = a.weg == Weg::Stichprobe;
        let stufe = |kaempfe_je: u64, wiederholungen: u64| {
            if parallel {
                Stufe {
                    art: Arbeitsart::ScalarFullFight,
                    aufgaben: kaempfe_je,
                    einheiten_je_aufgabe: 1.0,
                    wiederholungen,
                }
            } else {
                Stufe {
                    art: Arbeitsart::ScalarFullFight,
                    aufgaben: 1,
                    einheiten_je_aufgabe: kaempfe_je as f64,
                    wiederholungen,
                }
            }
        };
        if erste > 0 {
            stufen.push(stufe(erste, 1));
        }
        if anzahl > 1 && je > 0 {
            stufen.push(stufe(je, anzahl - 1));
        }
        kaempfe += erste + je * (anzahl - 1);
    }
    (stufen, kaempfe)
}

/// Der Grund, warum keine Ablaufsimulation möglich war — als Satz.
const fn grund(fehler: Ablauffehler) -> &'static str {
    match fehler {
        Ablauffehler::KeinRechner(_) => {
            "noch keine Rate je Strang: kein Rechner mit gemessener Kampfrate"
        }
        Ablauffehler::UngueltigeRate(_) => "die gemessene Rate ist null oder keine Zahl",
    }
}

/// Die Restzeit eines Laufs aus seiner Momentaufnahme (AC3).
///
/// Hier fällt die Entscheidung aus AC3: [`Arbeitsmenge::Offen`] ergibt
/// [`Restzeit::Plateau`] — vor jeder Rechnung, nicht als deren Ergebnis.
///
/// [`Arbeitsmenge::Offen`]: super::zaehlwerk::Arbeitsmenge::Offen
///
/// # Was schon begonnen ist
///
/// Kämpfe zählt der heiße Pfad sofort, Auswertungen erst, wenn sie fertig
/// sind. Bei fester Kampfzahl `f` hat die laufende Auswertung also
/// `Kämpfe − f · fertige Auswertungen` schon gerechnet. Bei der gestaffelten
/// Stichprobe ist das nur für die erste bestimmbar; danach zählt die Rechnung
/// keinen Vorlauf ab und bleibt damit eine obere Schranke.
#[must_use]
pub fn restzeit_aus(
    probe: &Momentaufnahme,
    kosten: &Kostenbild,
    rechner: &[Rechner],
    raten: Option<&Raten>,
    wiederholung_p: f64,
) -> Restzeit {
    use super::zaehlwerk::Arbeitsmenge;
    let fertig = probe.kandidaten_gesamt();
    let (mindestens, hoechstens) = match probe.stand.menge {
        Arbeitsmenge::Offen => return Restzeit::Plateau,
        Arbeitsmenge::Unbekannt => {
            return Restzeit::Unbestimmt("die Arbeitsmenge steht noch nicht fest")
        }
        Arbeitsmenge::Bekannt { kandidaten } => {
            let rest = kandidaten.saturating_sub(fertig);
            (rest, rest)
        }
        Arbeitsmenge::Hoechstens {
            mindestens,
            hoechstens,
        } => (
            mindestens.saturating_sub(fertig),
            hoechstens.saturating_sub(fertig),
        ),
    };
    let begonnen = match kosten.kaempfe_je_kandidat {
        Kampfzahl::Fest(f) => probe
            .kaempfe
            .saturating_sub(f.saturating_mul(fertig))
            .min(f),
        Kampfzahl::Hoechstens(b) if fertig == 0 => probe.kaempfe.min(b),
        Kampfzahl::Hoechstens(_) => 0,
    };
    restzeit(
        &[Restanteil {
            weg: kosten.weg,
            kaempfe_je_kandidat: kosten.kaempfe_je_kandidat,
            mindestens,
            hoechstens,
        }],
        begonnen,
        rechner,
        raten,
        wiederholung_p,
    )
}
