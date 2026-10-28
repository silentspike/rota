//! Die Grundlage: gegen wen relativ gemessen wird (AC2).
//!
//! # Warum relativ
//!
//! „Befehlsdurchsatz unter 2,0" bedeutet auf Zen 3 und Zen 5 etwas
//! Verschiedenes, und auf demselben Kern bei einem Boss mit neun
//! gleichzeitigen Systemen etwas anderes als bei einem mit zwei. Die Zeilen
//! der Regeltabelle, die Zählerwerte lesen, vergleichen deshalb **nur gegen
//! eine Grundlage desselben Rechners, derselben Kernklasse und desselben
//! Lastprofils** — und gegen keine andere. Passt der Schlüssel nicht, wird
//! die Zeile nicht geprüft; sie fällt nicht auf eine feste Zahl zurück.
//!
//! # Was hier NICHT steht
//!
//! Die Erhebung über die acht Lastprofile, die Speicherung je Kombination,
//! das bloße Bestätigen beim nächsten Lauf und die Erneuerung bei geändertem
//! Kernel sind E15.3b (#972). Hier steht eine Grundlage als Datei, so wie ein
//! einzelner Lauf sie schreibt und ein späterer sie liest.
//!
//! # Das Format
//!
//! Eine Zeile je Angabe, wie das Rechnerprofil in [`crate::kalibrierung`]:
//! lesbar, wenn etwas schiefgeht, und jede Zahl als Bitmuster **und** als
//! Dezimalzahl — das Bitmuster ist die Wahrheit, die Dezimalzahl der
//! Kommentar.

use std::fmt;

use super::kernklasse::Kernklasse;
use super::messwerte::{Messwerte, Zaehlerlage};

/// Wofür eine Grundlage gilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schluessel {
    /// Der Rechner — dieselbe Kennung wie im Fortschrittsstrom.
    pub rechner: String,
    /// Die Kernklasse.
    pub kernklasse: Kernklasse,
    /// Das Lastprofil: was gerechnet wird, beschrieben vom Aufrufer.
    pub lastprofil: String,
}

impl Schluessel {
    /// Worin `andere` abweicht — `None`, wenn beide übereinstimmen.
    ///
    /// Geprüft in der Reihenfolge Rechner, Kernklasse, Lastprofil; genannt
    /// wird die erste Abweichung.
    #[must_use]
    pub fn abweichung(&self, andere: &Self) -> Option<&'static str> {
        if self.rechner != andere.rechner {
            Some("anderer Rechner")
        } else if self.kernklasse != andere.kernklasse {
            Some("andere Kernklasse")
        } else if self.lastprofil != andere.lastprofil {
            Some("anderes Lastprofil")
        } else {
            None
        }
    }
}

/// Die Werte, gegen die relativ verglichen wird.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vergleichswerte {
    /// Befehle je Zyklus.
    pub ipc: Option<f64>,
    /// Zyklen je Kampf — gleich „Zyklen je Ereignis" relativ zur Grundlage,
    /// siehe [`super::Regel::Verzweigungsgebunden`].
    pub zyklen_je_kampf: Option<f64>,
    /// Fehlsprünge je Kampf.
    pub fehlspruenge_je_kampf: Option<f64>,
    /// Fehlgriffe der zweiten Ebene je Kampf.
    pub l2_je_kampf: Option<f64>,
    /// Fehlgriffe der dritten Ebene je Kampf.
    pub l3_je_kampf: Option<f64>,
    /// Speicher-Wartezyklen je Kampf.
    pub speicherwartezyklen_je_kampf: Option<f64>,
    /// CPU-Zeit je Kampf — die Vergleichsgröße des Rückfalls.
    pub cpu_ns_je_kampf: Option<f64>,
}

/// Die Namen der Werte im Format, in fester Reihenfolge.
const NAMEN: [&str; 7] = [
    "ipc",
    "zyklen_je_kampf",
    "fehlspruenge_je_kampf",
    "l2_je_kampf",
    "l3_je_kampf",
    "speicherwartezyklen_je_kampf",
    "cpu_ns_je_kampf",
];

impl Vergleichswerte {
    fn felder(&self) -> [Option<f64>; 7] {
        [
            self.ipc,
            self.zyklen_je_kampf,
            self.fehlspruenge_je_kampf,
            self.l2_je_kampf,
            self.l3_je_kampf,
            self.speicherwartezyklen_je_kampf,
            self.cpu_ns_je_kampf,
        ]
    }

    fn feld_mut(&mut self, name: &str) -> Option<&mut Option<f64>> {
        Some(match name {
            "ipc" => &mut self.ipc,
            "zyklen_je_kampf" => &mut self.zyklen_je_kampf,
            "fehlspruenge_je_kampf" => &mut self.fehlspruenge_je_kampf,
            "l2_je_kampf" => &mut self.l2_je_kampf,
            "l3_je_kampf" => &mut self.l3_je_kampf,
            "speicherwartezyklen_je_kampf" => &mut self.speicherwartezyklen_je_kampf,
            "cpu_ns_je_kampf" => &mut self.cpu_ns_je_kampf,
            _ => return None,
        })
    }
}

/// Eine Grundlage: ein Schlüssel und die Werte eines Laufs.
#[derive(Debug, Clone, PartialEq)]
pub struct Grundlage {
    /// Wofür sie gilt.
    pub schluessel: Schluessel,
    /// Die Werte.
    pub werte: Vergleichswerte,
    /// Über wie viele Kämpfe sie gemessen ist.
    pub kaempfe: u64,
    /// Über wie viele Sekunden.
    pub sekunden: f64,
}

/// Warum sich eine Grundlage nicht lesen ließ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grundlagenfehler {
    /// Die erste Zeile ist nicht `rota-engpass-grundlage 1`.
    Kopf,
    /// Eine Pflichtangabe fehlt.
    Fehlt(&'static str),
    /// Eine Zeile ist nicht lesbar.
    Zeile(String),
}

impl fmt::Display for Grundlagenfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kopf => write!(f, "die erste Zeile ist nicht 'rota-engpass-grundlage 1'"),
            Self::Fehlt(was) => write!(f, "die Angabe '{was}' fehlt"),
            Self::Zeile(z) => write!(f, "die Zeile '{z}' ist nicht lesbar"),
        }
    }
}

/// Die erste Zeile jeder Grundlage.
const KOPF: &str = "rota-engpass-grundlage 1";

impl Grundlage {
    /// Eine Grundlage aus den Messwerten eines Laufs.
    #[must_use]
    pub fn aus_messwerten(schluessel: Schluessel, m: &Messwerte) -> Self {
        let mut werte = Vergleichswerte {
            cpu_ns_je_kampf: m.cpu_ns_je_kampf,
            ..Vergleichswerte::default()
        };
        if let Zaehlerlage::Gemessen(z) = &m.zaehler {
            werte.ipc = z.ipc;
            werte.zyklen_je_kampf = z.zyklen_je_kampf;
            werte.fehlspruenge_je_kampf = z.fehlspruenge_je_kampf;
            werte.l2_je_kampf = z.l2_je_kampf;
            werte.l3_je_kampf = z.l3_je_kampf;
            werte.speicherwartezyklen_je_kampf = z.speicherwartezyklen_je_kampf;
        }
        Self {
            schluessel,
            werte,
            kaempfe: m.kaempfe,
            sekunden: m.sekunden,
        }
    }

    /// Der Text der Datei.
    #[must_use]
    pub fn speichern(&self) -> String {
        let mut s = format!(
            "{KOPF}\nrechner {}\nkernklasse {}\nlastprofil {}\nkaempfe {}\nsekunden {:016x} {:.3}\n",
            self.schluessel.rechner,
            self.schluessel.kernklasse.kennung(),
            self.schluessel.lastprofil,
            self.kaempfe,
            self.sekunden.to_bits(),
            self.sekunden
        );
        for (name, wert) in NAMEN.iter().zip(self.werte.felder()) {
            if let Some(w) = wert {
                s.push_str(&format!("wert {name} {:016x} {w}\n", w.to_bits()));
            }
        }
        s
    }

    /// Liest eine Grundlage.
    ///
    /// # Errors
    ///
    /// Ein [`Grundlagenfehler`] — **eine kaputte Zeile macht die ganze
    /// Grundlage ungültig.** Eine Grundlage mit einem still übersprungenen
    /// Wert vergliche gegen weniger, als sie zu enthalten behauptet.
    pub fn lesen(text: &str) -> Result<Self, Grundlagenfehler> {
        let mut zeilen = text.lines();
        if zeilen.next() != Some(KOPF) {
            return Err(Grundlagenfehler::Kopf);
        }
        let mut rechner = None;
        let mut kernklasse = None;
        let mut lastprofil = None;
        let mut kaempfe = None;
        let mut sekunden = None;
        let mut werte = Vergleichswerte::default();
        let kaputt = |z: &str| Grundlagenfehler::Zeile(z.to_owned());
        for zeile in zeilen.filter(|z| !z.trim().is_empty()) {
            let (art, rest) = zeile.split_once(' ').ok_or_else(|| kaputt(zeile))?;
            match art {
                "rechner" => rechner = Some(rest.to_owned()),
                "kernklasse" => {
                    kernklasse = Some(Kernklasse::aus_kennung(rest).ok_or_else(|| kaputt(zeile))?);
                }
                "lastprofil" => lastprofil = Some(rest.to_owned()),
                "kaempfe" => kaempfe = Some(rest.parse().map_err(|_| kaputt(zeile))?),
                "sekunden" => sekunden = Some(bits(rest).ok_or_else(|| kaputt(zeile))?),
                "wert" => {
                    let (name, zahl) = rest.split_once(' ').ok_or_else(|| kaputt(zeile))?;
                    let feld = werte.feld_mut(name).ok_or_else(|| kaputt(zeile))?;
                    *feld = Some(bits(zahl).ok_or_else(|| kaputt(zeile))?);
                }
                _ => return Err(kaputt(zeile)),
            }
        }
        Ok(Self {
            schluessel: Schluessel {
                rechner: rechner.ok_or(Grundlagenfehler::Fehlt("rechner"))?,
                kernklasse: kernklasse.ok_or(Grundlagenfehler::Fehlt("kernklasse"))?,
                lastprofil: lastprofil.ok_or(Grundlagenfehler::Fehlt("lastprofil"))?,
            },
            werte,
            kaempfe: kaempfe.ok_or(Grundlagenfehler::Fehlt("kaempfe"))?,
            sekunden: sekunden.ok_or(Grundlagenfehler::Fehlt("sekunden"))?,
        })
    }
}

/// Das Bitmuster am Anfang von `text` als Zahl; der Rest ist Kommentar.
fn bits(text: &str) -> Option<f64> {
    let hex = text.split_whitespace().next()?;
    let wert = f64::from_bits(u64::from_str_radix(hex, 16).ok()?);
    wert.is_finite().then_some(wert)
}
