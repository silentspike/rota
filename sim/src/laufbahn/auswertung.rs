//! Viele Laufbahnen rechnen und zu Verteilungen verdichten (Issue #922).
//!
//! # Gleicher Startwert, gleiche Ausgabe — auf jeder Strangzahl
//!
//! Eine Laufbahn haengt allein an ihrer Nummer und dem Startwert
//! ([`super::schleife::mische`]); auf welchem Strang sie laeuft, erfaehrt sie
//! nicht. Die Straenge bekommen zusammenhaengende Nummernbereiche, die
//! Ergebnisse kommen in Nummernreihenfolge zurueck, und jede Summe wird in
//! dieser Reihenfolge gebildet. Damit ist die Ausgabe dieselbe Zeichenfolge
//! bei einem Strang wie bei acht — der Prueffall
//! `gleicher_startwert_gleiche_ausgabe_bei_jeder_strangzahl` haelt das fest.
//!
//! # Perzentile
//!
//! Nach dem **naechsten Rang**: Das `q`-Perzentil von `n` aufsteigend
//! sortierten Werten ist der Wert auf Platz `ceil(q * n)`. Kein Mitteln
//! zwischen zwei Plaetzen — jede Angabe ist ein Wert, den eine Laufbahn
//! wirklich hatte.

use core::fmt::{self, Write as _};

use crate::data::BOSSES;
use crate::gear::{QUALITY_COUNT, QUALITY_NAMES, SLOT_COUNT};

use super::orakel::{
    Kampfeinstellung, Kampforakel, OrakelFehler, Referenzspielweise, SpeicherOrakel, FASSUNG_A,
};
use super::regeln::{bessere_stuecke, erwarteter_stand, xp_for_victory, INSTANZEN, KATALOG};
use super::schleife::{laufbahn, Laufbahn, Laufbahnregeln, Messauftrag, VERLAUF_SIEGE};
use super::{kampffingerabdruck, Fortschrittsfingerabdruck, Kampffingerabdruck};

/// Die Siege, an denen die Verlaeufe ausgegeben werden.
pub const BERICHTS_SIEGE: [u32; 12] = [1, 2, 3, 5, 10, 15, 20, 30, 50, 75, 100, 150];

// Jeder berichtete Sieg muss im festgehaltenen Verlauf liegen — sonst griffe
// der Bericht ueber das Ende von `Laufbahn::siege` hinaus.
const _: () = assert!(BERICHTS_SIEGE[BERICHTS_SIEGE.len() - 1] as usize <= VERLAUF_SIEGE);

/// Die Spielerstufen, bis zu denen die Kaempfe gezaehlt werden (Auftrag
/// 29.10.: 10, 20, 30 — die Luecke der fehlenden Boss-Stufen als Zahl).
pub const BERICHTS_STUFEN: [u32; 8] = [5, 10, 15, 20, 25, 27, 28, 30];

/// Die Siege, nach denen der fehlende Fund gezaehlt wird (Masterplan: 10/20/30).
pub const FUND_SIEGE: [u32; 3] = [10, 20, 30];

/// Wie viele Standardfehler die Toleranz eines Anteils breit ist.
///
/// Vier: Bei einer Normalnaeherung faellt ein richtiger Anteil mit
/// Wahrscheinlichkeit 6,3 · 10⁻⁵ heraus. Die Toleranz folgt so aus `n` — bei
/// 10 000 Laufbahnen und 4,24 % sind es 0,81 Prozentpunkte.
pub const TOLERANZ_SIGMA: f64 = 4.0;

/// Ein Lauf ueber viele Laufbahnen.
#[derive(Debug, Clone)]
pub struct Auftrag {
    /// Wie viele Laufbahnen.
    pub laufbahnen: u32,
    /// Der Startwert des Laufs.
    pub startwert: u32,
    /// Auf wie vielen Straengen — **beruehrt kein Ergebnis**.
    pub straenge: usize,
    /// Die Regeln der Schleife.
    pub regeln: Laufbahnregeln,
    /// Laenge und verfolgter Fund.
    pub messung: Messauftrag,
    /// Wie das Orakel kaempft.
    pub kampf: Kampfeinstellung,
}

/// Rechnet `anzahl` Laufbahnen ab Nummer 0 auf `straenge` Straengen, je
/// Laufbahn mit einem frischen Orakel aus `fabrik`.
///
/// **Ein Orakel je Laufbahn**: Eine Zelle ist eine reine Funktion ihres
/// Schluessels, das Ergebnis haengt also nicht daran, wie geteilt wird; der
/// Speicher waechst so nur mit einer Laufbahn und nicht mit allen.
///
/// # Errors
///
/// Den ersten [`OrakelFehler`] in Nummernreihenfolge.
///
/// # Panics
///
/// Wenn ein Strang abstuerzt — ein stillschweigend gekuerzter Satz waere
/// schlimmer als ein Abbruch.
pub fn rechne_mit<O, F>(
    anzahl: u32,
    startwert: u32,
    straenge: usize,
    regelsaetze: &[Laufbahnregeln],
    messung: &Messauftrag,
    fabrik: F,
) -> Result<Lauf, OrakelFehler>
where
    O: Kampforakel,
    F: Fn() -> O + Sync,
{
    let straenge = straenge.clamp(1, anzahl.max(1) as usize);
    let je_strang = (anzahl as usize).div_ceil(straenge);
    let teile: Vec<Result<Vec<Teilergebnis>, OrakelFehler>> = std::thread::scope(|bereich| {
        let griffe: Vec<_> = (0..straenge)
            .map(|s| {
                let fabrik = &fabrik;
                let von = (s * je_strang).min(anzahl as usize) as u32;
                let bis = ((s + 1) * je_strang).min(anzahl as usize) as u32;
                bereich.spawn(move || {
                    (von..bis)
                        .map(|nummer| {
                            let mut orakel = fabrik();
                            let mut ergebnisse = Vec::with_capacity(regelsaetze.len());
                            for regeln in regelsaetze {
                                ergebnisse.push(laufbahn(
                                    nummer,
                                    startwert,
                                    regeln,
                                    messung,
                                    &mut orakel,
                                )?);
                            }
                            Ok(Teilergebnis {
                                ergebnisse,
                                gerechnet: orakel.gerechnet(),
                                abgefragt: orakel.abgefragt(),
                            })
                        })
                        .collect::<Result<Vec<_>, OrakelFehler>>()
                })
            })
            .collect();
        griffe
            .into_iter()
            .map(|g| {
                g.join()
                    .expect("ein Strang der Fortschritts-Simulation ist abgestuerzt")
            })
            .collect()
    });
    let mut lauf = Lauf {
        je_regelsatz: vec![Vec::with_capacity(anzahl as usize); regelsaetze.len()],
        gerechnet: 0,
        abgefragt: 0,
    };
    for teil in teile {
        for je_laufbahn in teil? {
            lauf.gerechnet += je_laufbahn.gerechnet;
            lauf.abgefragt += je_laufbahn.abgefragt;
            for (ziel, ergebnis) in lauf.je_regelsatz.iter_mut().zip(je_laufbahn.ergebnisse) {
                ziel.push(ergebnis);
            }
        }
    }
    Ok(lauf)
}

struct Teilergebnis {
    ergebnisse: Vec<Laufbahn>,
    gerechnet: u64,
    abgefragt: u64,
}

/// Die gerechneten Laufbahnen eines Laufs, je Regelsatz in Nummernreihenfolge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lauf {
    /// Je Regelsatz eine Liste, Laufbahn `i` an Stelle `i`.
    pub je_regelsatz: Vec<Vec<Laufbahn>>,
    /// Wie viele Kaempfe das Orakel wirklich gerechnet hat, alle Laufbahnen.
    pub gerechnet: u64,
    /// Wie oft es gefragt wurde.
    pub abgefragt: u64,
}

/// Rechnet einen Auftrag mit dem Orakel ueber dem Ergebnisspeicher, fuer
/// einen oder mehrere Regelsaetze. Mehrere Regelsaetze **teilen** je
/// Laufbahn ein Orakel — eine geaenderte Schleifenregel kostet dann nur die
/// Zellen, die der erste Satz nicht schon gerechnet hat (AC5).
///
/// # Errors
///
/// [`OrakelFehler`] wie bei [`rechne_mit`].
pub fn rechne(
    auftrag: &Auftrag,
    regelsaetze: &[Laufbahnregeln],
    referenz: &Referenzspielweise,
) -> Result<Lauf, OrakelFehler> {
    rechne_mit(
        auftrag.laufbahnen,
        auftrag.startwert,
        auftrag.straenge,
        regelsaetze,
        &auftrag.messung,
        || SpeicherOrakel::neu(referenz, auftrag.kampf),
    )
}

/// Mittel und Perzentile einer Reihe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Verteilung {
    /// Wie viele Werte.
    pub n: usize,
    /// Mittelwert.
    pub mittel: f64,
    /// 10. Perzentil.
    pub p10: f64,
    /// Median.
    pub p50: f64,
    /// 90. Perzentil.
    pub p90: f64,
}

impl Verteilung {
    /// Aus Werten in beliebiger Reihenfolge; das Mittel wird in der
    /// uebergebenen Reihenfolge summiert.
    #[must_use]
    pub fn aus(werte: &[f64]) -> Option<Self> {
        if werte.is_empty() {
            return None;
        }
        let mut summe = 0.0;
        for w in werte {
            summe += w;
        }
        let mut sortiert = werte.to_vec();
        sortiert.sort_by(f64::total_cmp);
        Some(Self {
            n: werte.len(),
            mittel: summe / werte.len() as f64,
            p10: perzentil(&sortiert, 0.10),
            p50: perzentil(&sortiert, 0.50),
            p90: perzentil(&sortiert, 0.90),
        })
    }
}

/// Das `q`-Perzentil nach dem naechsten Rang. `sortiert` darf nicht leer sein.
#[must_use]
pub fn perzentil(sortiert: &[f64], q: f64) -> f64 {
    let rang = (q * sortiert.len() as f64).ceil() as usize;
    sortiert[rang.clamp(1, sortiert.len()) - 1]
}

/// Ein Anteil mit der Toleranz, die aus seiner Grundmenge folgt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anteil {
    /// Wie viele zaehlen.
    pub treffer: usize,
    /// Wie viele es insgesamt sind.
    pub n: usize,
}

impl Anteil {
    /// Der Anteil, `0` bei leerer Grundmenge.
    #[must_use]
    pub fn wert(&self) -> f64 {
        if self.n == 0 {
            return 0.0;
        }
        self.treffer as f64 / self.n as f64
    }

    /// Die Toleranz um einen erwarteten Anteil `p`:
    /// [`TOLERANZ_SIGMA`] mal `sqrt(p (1 - p) / n)`.
    #[must_use]
    pub fn toleranz_um(&self, p: f64) -> f64 {
        if self.n == 0 {
            return f64::INFINITY;
        }
        TOLERANZ_SIGMA * (p * (1.0 - p) / self.n as f64).sqrt()
    }
}

/// Wie viele Laufbahnen den Fund nach `k` Siegen noch nicht haben — unter
/// denen, die `k` Siege erreicht haben.
#[must_use]
pub fn fehlt_nach(laufbahnen: &[Laufbahn], k: u32) -> Anteil {
    let mit_k: Vec<&Laufbahn> = laufbahnen.iter().filter(|l| l.siege_gesamt >= k).collect();
    Anteil {
        treffer: mit_k
            .iter()
            .filter(|l| l.fund_beim_sieg.is_none_or(|s| s > k))
            .count(),
        n: mit_k.len(),
    }
}

/// Die Rate je Sieg, wenn sie sich ohne Lauf angeben laesst: genau ein
/// Eintrag im Katalog, und der Fund zaehlt ab `common`. Dann ist sie die
/// Fallrate selbst.
#[must_use]
pub fn bekannte_rate(regeln: &Laufbahnregeln, messung: &Messauftrag) -> Option<f64> {
    (regeln.katalog == [messung.fund.vorlage] && messung.fund.mindest_guete == 0)
        .then(|| f64::from(regeln.fallrate_promille) / 1000.0)
}

/// Ein ausgewerteter Lauf, fertig zur Ausgabe.
#[derive(Debug, Clone, PartialEq)]
pub struct Bericht {
    /// Text der Ausgabe — dieselbe Zeichenfolge bei jeder Strangzahl.
    pub text: String,
}

impl fmt::Display for Bericht {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

fn zahl(x: f64) -> String {
    format!("{x:.1}")
}

fn verteilung_zeile(v: Option<Verteilung>) -> String {
    v.map_or_else(
        || "      -".to_owned(),
        |v| {
            format!(
                "{:>7} {:>9} {:>8} {:>8} {:>8}",
                v.n,
                zahl(v.mittel),
                zahl(v.p10),
                zahl(v.p50),
                zahl(v.p90)
            )
        },
    )
}

/// Wertet einen Regelsatz eines Laufs aus.
///
/// `straenge` steht NICHT im Text — sonst waere die Ausgabe bei anderer
/// Strangzahl eine andere, und gerade das darf sie nicht sein.
#[must_use]
pub fn bericht(
    auftrag: &Auftrag,
    regeln: &Laufbahnregeln,
    laufbahnen: &[Laufbahn],
    referenz_name: &str,
    kampf: Kampffingerabdruck,
) -> Bericht {
    let mut t = String::new();
    let messung = &auftrag.messung;
    let fund = &KATALOG[messung.fund.vorlage];
    let _ = writeln!(t, "Fortschritts-Simulation (#922, E11.4e)");
    let _ = writeln!(t, "{FASSUNG_A}");
    let _ = writeln!(
        t,
        "Referenz-Spielweise: {referenz_name}, ohne Talente, Kampfstufe 1, {} Durchgang/Zelle, \
         Startwert {}",
        auftrag.kampf.durchgaenge, auftrag.kampf.startwert
    );
    let _ = writeln!(
        t,
        "Laufbahnen {}  Startwert {}  Siege je Laufbahn {}  hoechstens {} Kaempfe",
        laufbahnen.len(),
        auftrag.startwert,
        messung.siege,
        messung.max_kaempfe
    );
    let _ = writeln!(
        t,
        "Regeln: Fallrate {} Promille, Katalog {} Vorlagen, Material {:?}, Stufe kostet {}, \
         {:?}, {:?}, {:?}",
        regeln.fallrate_promille,
        regeln.katalog.len(),
        regeln.material_je_qualitaet,
        regeln.kosten_je_stufe,
        regeln.freischaltung,
        regeln.anlegen,
        regeln.aufwerten
    );
    let _ = writeln!(
        t,
        "Fingerabdruck Fortschritt {}  Kampf-Tabelle {}",
        Fortschrittsfingerabdruck::aus(regeln),
        kampf
    );

    // ---- Punkte, Stufe, Spielerstufe je Sieg
    let _ = writeln!(t, "\nJe Sieg (n = Laufbahnen mit so vielen Siegen):");
    let _ = writeln!(
        t,
        "  Sieg | Punkte      n    mittel      p10      p50      p90 | Gegenstandsstufe (Mittel der 10 Plaetze)      n    mittel      p10      p50      p90 | Spielerstufe mittel"
    );
    for k in BERICHTS_SIEGE.iter().filter(|k| **k <= messung.siege) {
        let mit_k: Vec<&Laufbahn> = laufbahnen.iter().filter(|l| l.siege_gesamt >= *k).collect();
        let sieg = |l: &&Laufbahn| l.siege[*k as usize - 1];
        let punkte: Vec<f64> = mit_k.iter().map(|l| sieg(l).punkte as f64).collect();
        let stufe: Vec<f64> = mit_k
            .iter()
            .map(|l| f64::from(sieg(l).stufensumme) / SLOT_COUNT as f64)
            .collect();
        let spieler: Vec<f64> = mit_k
            .iter()
            .map(|l| f64::from(sieg(l).spielerstufe))
            .collect();
        let _ = writeln!(
            t,
            "  {k:>4} | {} | {:>41} {} | {:>8}",
            verteilung_zeile(Verteilung::aus(&punkte)),
            "",
            verteilung_zeile(Verteilung::aus(&stufe)),
            Verteilung::aus(&spieler).map_or_else(|| "-".to_owned(), |v| zahl(v.mittel))
        );
    }

    // ---- Der verfolgte Fund
    let guete = QUALITY_NAMES
        .get(messung.fund.mindest_guete)
        .copied()
        .unwrap_or("?");
    let _ = writeln!(
        t,
        "\nFund {} ab {guete}: Anteil der Laufbahnen, die ihn nach k Siegen NOCH NICHT haben",
        fund.id
    );
    let rate = bekannte_rate(regeln, messung);
    if let Some(p) = rate {
        let _ = writeln!(
            t,
            "  Rate je Sieg bekannt: {:.4} (ein Eintrag im Katalog, ab common) — Erwartung (1-p)^k, \
             Toleranz {TOLERANZ_SIGMA} * sqrt(e(1-e)/n)",
            p
        );
    } else {
        let _ = writeln!(
            t,
            "  Rate je Sieg nicht geschlossen angebbar (sie haengt an Instanz und Gewichten); \
             Toleranz {TOLERANZ_SIGMA} * sqrt(a(1-a)/n) um den gemessenen Anteil a"
        );
    }
    for k in FUND_SIEGE {
        let a = fehlt_nach(laufbahnen, k);
        match rate {
            Some(p) => {
                let erwartet = (1.0 - p).powi(k as i32);
                let tol = a.toleranz_um(erwartet);
                let _ = writeln!(
                    t,
                    "  nach {k:>2} Siegen: {:>6} von {:>6} = {:>7.3} %   erwartet {:>7.3} %   \
                     Toleranz +-{:.3} Pp   {}",
                    a.treffer,
                    a.n,
                    100.0 * a.wert(),
                    100.0 * erwartet,
                    100.0 * tol,
                    if (a.wert() - erwartet).abs() <= tol {
                        "innerhalb"
                    } else {
                        "AUSSERHALB"
                    }
                );
            }
            None => {
                let _ = writeln!(
                    t,
                    "  nach {k:>2} Siegen: {:>6} von {:>6} = {:>7.3} %   Toleranz +-{:.3} Pp",
                    a.treffer,
                    a.n,
                    100.0 * a.wert(),
                    100.0 * a.toleranz_um(a.wert())
                );
            }
        }
    }

    // ---- Kaempfe bis Spielerstufe L, mit den heutigen Boss-Stufen
    let _ = writeln!(
        t,
        "\nKaempfe bis Spielerstufe L (Boss-Stufen aus den Daten; Siege je Laufbahn {}):",
        messung.siege
    );
    let _ = writeln!(
        t,
        "  Stufe | erreicht      n    mittel      p10      p50      p90 | nicht erreicht | Erfahrung je Sieg auf Stufe L-1 gegen Boss-Stufe 1"
    );
    for stufe in BERICHTS_STUFEN {
        let index = stufe as usize - 1;
        let werte: Vec<f64> = laufbahnen
            .iter()
            .filter_map(|l| {
                l.stufe_nach_kaempfen
                    .get(index)
                    .copied()
                    .flatten()
                    .map(f64::from)
            })
            .collect();
        // **Aus der Regel, nicht nachgerechnet** (`xp_for_victory`, gegen
        // `xpForVictory` aufgezeichnet): Fuer die Stufe davor — dort wird
        // die Erfahrung verdient, die diese Stufe braucht.
        let xp = xp_for_victory(1, i64::from(stufe) - 1);
        let _ = writeln!(
            t,
            "  {stufe:>5} | {:>8} {} | {:>14} | {xp}",
            "",
            verteilung_zeile(Verteilung::aus(&werte)),
            laufbahnen.len() - werte.len()
        );
    }

    // ---- Kaempfe bis Instanz N (AC3) und erwarteter Stand daneben (AC4)
    let _ = writeln!(
        t,
        "\nKaempfe bis Instanz N (Median, p10, p90 ueber die Laufbahnen, die sie erreicht haben):"
    );
    let _ = writeln!(
        t,
        "  Instanz      | geoeffnet      n   mittel    p10    p50    p90 | abgeschlossen      n   mittel    p10    p50    p90 | nicht abgeschlossen"
    );
    for (i, instanz) in INSTANZEN.iter().enumerate() {
        let auf: Vec<f64> = laufbahnen
            .iter()
            .filter_map(|l| l.geoeffnet_nach[i].map(f64::from))
            .collect();
        let zu: Vec<f64> = laufbahnen
            .iter()
            .filter_map(|l| l.abgeschlossen_nach[i].map(f64::from))
            .collect();
        let _ = writeln!(
            t,
            "  {:<12} | {:>8} {} | {:>12} {} | {}",
            instanz.id,
            "",
            verteilung_zeile(Verteilung::aus(&auf)),
            "",
            verteilung_zeile(Verteilung::aus(&zu)),
            laufbahnen.len() - zu.len()
        );
    }
    let _ = writeln!(
        t,
        "\nAusruestung beim Betreten — gemessen gegen den gesetzten erwarteten Stand \
         (`expected-gear.ts`, 20.5):"
    );
    let _ = writeln!(
        t,
        "  Instanz      | erwartet: Stufe  ueber common | gemessen:     n  Stufe mittel  ueber common mittel  belegt mittel"
    );
    for (i, instanz) in INSTANZEN.iter().enumerate() {
        let Some(verlangt) = instanz.requires.and_then(super::regeln::instanz) else {
            continue;
        };
        let vorstufe = i32::try_from(verlangt.number).unwrap_or(1);
        let erwartet = erwarteter_stand(vorstufe, SLOT_COUNT);
        let stufe = erwartet.first().map_or(0, |s| s.item_level);
        let besser = bessere_stuecke(vorstufe, SLOT_COUNT);
        let staende: Vec<_> = laufbahnen
            .iter()
            .filter_map(|l| l.stand_bei_oeffnung[i])
            .collect();
        let n = staende.len();
        let mittel = |f: &dyn Fn(&super::schleife::Standbild) -> u32, teiler: f64| {
            if n == 0 {
                "-".to_owned()
            } else {
                let mut s = 0.0;
                for st in &staende {
                    s += f64::from(f(st)) / teiler;
                }
                format!("{:.2}", s / n as f64)
            }
        };
        let _ = writeln!(
            t,
            "  {:<12} |          {:>6} {:>13} |          {:>6} {:>13} {:>20} {:>14}",
            instanz.id,
            stufe,
            besser,
            n,
            mittel(&|s| s.stufensumme, SLOT_COUNT as f64),
            mittel(&|s| s.ueber_common, 1.0),
            mittel(&|s| s.belegt, 1.0)
        );
    }

    // ---- Wo sie stecken bleiben, und die Obergrenze
    let mut steckt: Vec<(usize, usize)> = Vec::new();
    for l in laufbahnen
        .iter()
        .filter(|l| l.abgeschlossen_nach.iter().any(Option::is_none))
    {
        let offen_bis = INSTANZEN
            .iter()
            .enumerate()
            .filter(|(i, _)| l.geoeffnet_nach[*i].is_some() && l.abgeschlossen_nach[*i].is_none())
            .map(|(i, _)| i)
            .next();
        if let Some(i) = offen_bis {
            match steckt.iter_mut().find(|(j, _)| *j == i) {
                Some(e) => e.1 += 1,
                None => steckt.push((i, 1)),
            }
        }
    }
    steckt.sort_unstable();
    let _ = writeln!(
        t,
        "\nNicht abgeschlossen nach {} Siegen / {} Kaempfen, nach erster offener Instanz:",
        messung.siege, messung.max_kaempfe
    );
    if steckt.is_empty() {
        let _ = writeln!(t, "  keine");
    }
    for (i, n) in steckt {
        let _ = writeln!(t, "  {:<12} {n}", INSTANZEN[i].id);
    }
    let ueber = laufbahnen
        .iter()
        .map(|l| l.ueber_deckel)
        .max()
        .unwrap_or(i64::MIN);
    let _ = writeln!(
        t,
        "Obergrenze der Aufwertung: groesster Abstand UEBER der Inhaltsstufe = {} \
         (hoechstens 0 heisst: die Regel aus 13.6 hat gehalten)",
        if ueber == i64::MIN {
            "- (nie etwas getragen)".to_owned()
        } else {
            ueber.to_string()
        }
    );
    let material: Vec<f64> = laufbahnen
        .iter()
        .map(|l| l.material_ungenutzt as f64)
        .collect();
    let punkte_gesamt: Vec<f64> = laufbahnen.iter().map(|l| l.punkte_gesamt as f64).collect();
    let kaempfe: Vec<f64> = laufbahnen.iter().map(|l| f64::from(l.kaempfe)).collect();
    let _ = writeln!(
        t,
        "Summen je Laufbahn          n    mittel      p10      p50      p90"
    );
    let _ = writeln!(
        t,
        "  Punkte gesamt      {}",
        verteilung_zeile(Verteilung::aus(&punkte_gesamt))
    );
    let _ = writeln!(
        t,
        "  Kaempfe            {}",
        verteilung_zeile(Verteilung::aus(&kaempfe))
    );
    let _ = writeln!(
        t,
        "  Material am Ende   {}",
        verteilung_zeile(Verteilung::aus(&material))
    );
    let _ = writeln!(
        t,
        "Qualitaeten im Spiel: {}; Bosse im Inhalt: {}",
        QUALITY_COUNT,
        BOSSES
            .iter()
            .filter(|b| INSTANZEN.iter().any(|i| i.id == b.instance))
            .count()
    );
    Bericht { text: t }
}

/// Wertet einen Lauf mit mehreren Regelsaetzen aus und haengt den Vergleich
/// an — die Frage aus AC7: **Lohnt Aufwerten gegenueber Weiterspielen?**
#[must_use]
pub fn vergleich(namen: &[&str], lauf: &Lauf) -> String {
    let mut t = String::new();
    let _ = writeln!(
        t,
        "\nVergleich der Regelsaetze (gleiche Laufbahnen, gleiche Startwerte):"
    );
    let _ = write!(t, "  {:<12} |", "Instanz");
    for name in namen {
        let _ = write!(t, " {name:>26} |");
    }
    let _ = writeln!(t);
    let _ = write!(t, "  {:<12} |", "");
    for _ in namen {
        let _ = write!(t, " {:>12} {:>13} |", "abgeschl. n", "Median Kaempfe");
    }
    let _ = writeln!(t);
    for (i, instanz) in INSTANZEN.iter().enumerate() {
        let _ = write!(t, "  {:<12} |", instanz.id);
        for laufbahnen in &lauf.je_regelsatz {
            let zu: Vec<f64> = laufbahnen
                .iter()
                .filter_map(|l| l.abgeschlossen_nach[i].map(f64::from))
                .collect();
            let median = Verteilung::aus(&zu).map_or_else(|| "-".to_owned(), |v| zahl(v.p50));
            let _ = write!(t, " {:>12} {:>13} |", zu.len(), median);
        }
        let _ = writeln!(t);
    }
    let _ = write!(t, "  {:<12} |", "Punkte Summe");
    for laufbahnen in &lauf.je_regelsatz {
        let summen: Vec<f64> = laufbahnen.iter().map(|l| l.punkte_gesamt as f64).collect();
        let median = Verteilung::aus(&summen).map_or_else(|| "-".to_owned(), |v| zahl(v.p50));
        let _ = write!(t, " {:>12} {:>13} |", "Median", median);
    }
    let _ = writeln!(t);
    let _ = writeln!(
        t,
        "Orakel: {} Kaempfe gerechnet bei {} Abfragen ({:.1} % nachgeschlagen)",
        lauf.gerechnet,
        lauf.abgefragt,
        if lauf.abgefragt == 0 {
            0.0
        } else {
            100.0 * (1.0 - lauf.gerechnet as f64 / lauf.abgefragt as f64)
        }
    );
    t
}

/// Der Kampf-Fingerabdruck eines Auftrags — Bequemlichkeit fuer die Ausgabe.
#[must_use]
pub fn kampf_von(auftrag: &Auftrag, referenz: &Referenzspielweise) -> Kampffingerabdruck {
    kampffingerabdruck(referenz, auftrag.kampf)
}
