//! Die Erhebung eines Laufs: Zähler, Schlüssel, Grundlage — und daraus ein
//! [`Engpassbild`] je Anzeige.
//!
//! Sie wird im Strang angelegt, der die Arbeitsstränge startet, bevor der
//! erste startet (siehe [`super::zaehler`], „Warum `inherit`"). Gelesen wird
//! sie aus dem Anzeigestrang.

use std::fmt::Write as _;

use super::grundlage::{Grundlage, Schluessel};
use super::kernklasse::{Adapter, Kernklasse, Prozessor};
use super::messwerte::{Druck, Messwerte, Rohstand, Zaehlerlage};
use super::regeln::{diagnose, Diagnose};
use super::zaehler::{prozess_cpu_ns, Leistungszaehler};
use crate::fortschritt::Momentaufnahme;
use crate::kampagne::{json_number, json_string};

/// Die Engpasserhebung eines Laufs.
#[derive(Debug)]
pub struct Erhebung {
    schluessel: Schluessel,
    grundlage: Option<Grundlage>,
    /// Die Zähler — oder warum es keine gibt, als Satz.
    zaehler: Result<Leistungszaehler, String>,
    prozessor: Option<String>,
}

impl Erhebung {
    /// Beginnt die Erhebung im laufenden Strang.
    ///
    /// Erkennt die Kernklasse, wählt ihren Adapter und öffnet die Zähler. Ist
    /// eines davon nicht möglich, läuft die Erhebung ohne Zähler weiter und
    /// sagt, warum — **sie fällt nicht aus** (AC4).
    #[must_use]
    pub fn starten(rechner: &str, lastprofil: &str, grundlage: Option<Grundlage>) -> Self {
        let prozessor = Prozessor::erheben();
        let kernklasse = prozessor
            .as_ref()
            .map_or(Kernklasse::Unbekannt, Prozessor::kernklasse);
        let zaehler = kernklasse
            .adapter()
            .map_err(ToOwned::to_owned)
            .and_then(|a| Leistungszaehler::oeffnen(a).map_err(|s| s.satz()));
        Self {
            schluessel: Schluessel {
                rechner: rechner.to_owned(),
                kernklasse,
                lastprofil: lastprofil.to_owned(),
            },
            grundlage,
            zaehler,
            prozessor: prozessor.map(|p| p.name),
        }
    }

    /// Eine Erhebung ohne Zähler — für einen Rechner, auf dem schon feststeht,
    /// dass es keine gibt, und für Prüffälle des Rückfalls.
    #[must_use]
    pub fn ohne_zaehler(schluessel: Schluessel, grundlage: Option<Grundlage>, grund: &str) -> Self {
        Self {
            schluessel,
            grundlage,
            zaehler: Err(grund.to_owned()),
            prozessor: None,
        }
    }

    /// Wofür gemessen wird.
    #[must_use]
    pub const fn schluessel(&self) -> &Schluessel {
        &self.schluessel
    }

    /// Die Grundlage, falls eine geladen ist.
    #[must_use]
    pub const fn grundlage(&self) -> Option<&Grundlage> {
        self.grundlage.as_ref()
    }

    /// Der Adapter, nach dem gezählt wird — oder warum nicht.
    ///
    /// # Errors
    ///
    /// Der Satz, warum es keine Zähler gibt.
    pub fn zaehlerlage(&self) -> Result<&'static Adapter, &str> {
        self.zaehler
            .as_ref()
            .map(Leistungszaehler::adapter)
            .map_err(String::as_str)
    }

    /// Trägt in den Rohstand ein, was das Zählwerk nicht kennt: Zähler,
    /// CPU-Zeit, Druck. Gerufen vom Anzeigestrang, direkt nach der
    /// Momentaufnahme.
    pub fn ergaenzen(&self, roh: &mut Rohstand) {
        roh.leistung = self.zaehler.as_ref().ok().and_then(|z| z.lesen().ok());
        roh.cpu_ns = prozess_cpu_ns();
        roh.druck = Druck::erheben();
    }

    /// Das Bild zwischen zwei Momentaufnahmen.
    #[must_use]
    pub fn bild(&self, alt: &Momentaufnahme, neu: &Momentaufnahme) -> Engpassbild {
        let messwerte = Messwerte::aus_fenster(alt, neu, self.zaehlerlage());
        self.bild_aus(messwerte)
    }

    /// Das Bild zu fertigen Messwerten — die Regeltabelle, angewandt.
    #[must_use]
    pub fn bild_aus(&self, messwerte: Messwerte) -> Engpassbild {
        let diagnose = diagnose(&messwerte, &self.schluessel, self.grundlage.as_ref());
        Engpassbild {
            schluessel: self.schluessel.clone(),
            grundlage: self.grundlage.as_ref().map(|g| g.schluessel.clone()),
            zaehler: self
                .zaehlerlage()
                .map(|a| a.hinweis)
                .map_err(ToOwned::to_owned),
            prozessor: self.prozessor.clone(),
            messwerte,
            diagnose,
        }
    }
}

/// Was beide Ausgaben über den Engpass zeigen — aus einer Momentaufnahme.
#[derive(Debug, Clone, PartialEq)]
pub struct Engpassbild {
    /// Wofür gemessen wurde.
    pub schluessel: Schluessel,
    /// Der Schlüssel der geladenen Grundlage.
    pub grundlage: Option<Schluessel>,
    /// Der Hinweis des Adapters — oder warum es keine Zähler gibt.
    pub zaehler: Result<&'static str, String>,
    /// `model name` des Prozessors.
    pub prozessor: Option<String>,
    /// Die Messwerte des Fensters.
    pub messwerte: Messwerte,
    /// Was die Regeltabelle daraus macht.
    pub diagnose: Diagnose,
}

impl Engpassbild {
    /// Die Zeilen für Menschen: der Satz, dann woher er kommt.
    #[must_use]
    pub fn zeilen(&self) -> Vec<String> {
        let mut zeilen = vec![format!("  Engpass      {}", self.diagnose.satz())];
        let zaehler = match &self.zaehler {
            Ok(_) => match &self.messwerte.zaehler {
                Zaehlerlage::Gemessen(z) => format!(
                    "Zähler {} ({} / {} der Zeit gezählt)",
                    self.schluessel.kernklasse.kennung(),
                    abdeckung(z.abdeckung[0]),
                    abdeckung(z.abdeckung[1])
                ),
                Zaehlerlage::Fehlt(g) => {
                    format!("Zähler {}: {g}", self.schluessel.kernklasse.kennung())
                }
            },
            Err(grund) => format!("ohne Zähler ({grund}) — Zeitmessung und Druckwerte"),
        };
        let grundlage = match &self.grundlage {
            None => "ohne Grundlage".to_owned(),
            Some(g) => match g.abweichung(&self.schluessel) {
                None => "Grundlage passt".to_owned(),
                Some(was) => format!("Grundlage passt nicht: {was}"),
            },
        };
        zeilen.push(format!("     Messung   {zaehler} · {grundlage}"));
        zeilen
    }

    /// Das Feld `bottleneck` des JSON-Stroms.
    #[must_use]
    pub fn json(&self) -> String {
        let d = &self.diagnose;
        let zustand = if !d.befunde.is_empty() {
            "diagnosis"
        } else if d.geprueft.is_empty() {
            "pending"
        } else {
            "none"
        };
        let mut j = format!(
            "{{\"state\":\"{zustand}\",\"diagnosis\":{},\"sentence\":{},",
            d.befunde
                .first()
                .map_or_else(|| "null".to_owned(), |b| json_string(b.regel.diagnose())),
            json_string(&d.satz())
        );
        let befunde: Vec<String> = d
            .befunde
            .iter()
            .map(|b| {
                format!(
                    "{{\"rule\":\"{}\",\"diagnosis\":{},\"observation\":{},\"next_step\":{}}}",
                    b.regel.schluessel(),
                    json_string(b.regel.diagnose()),
                    json_string(&b.beobachtung),
                    json_string(b.regel.naechster_schritt())
                )
            })
            .collect();
        let _ = write!(j, "\"findings\":[{}],", befunde.join(","));
        let geprueft: Vec<String> = d
            .geprueft
            .iter()
            .map(|r| format!("\"{}\"", r.schluessel()))
            .collect();
        let _ = write!(j, "\"checked\":[{}],", geprueft.join(","));
        let ungeprueft: Vec<String> = d
            .ungeprueft
            .iter()
            .map(|(r, g)| {
                format!(
                    "{{\"rule\":\"{}\",\"reason\":{}}}",
                    r.schluessel(),
                    json_string(g)
                )
            })
            .collect();
        let _ = write!(j, "\"unchecked\":[{}],", ungeprueft.join(","));
        let _ = write!(j, "\"key\":{},", schluessel_json(&self.schluessel));
        let _ = write!(
            j,
            "\"baseline\":{},",
            self.grundlage.as_ref().map_or_else(
                || "null".to_owned(),
                |g| format!(
                    "{{\"matches\":{},\"key\":{}}}",
                    g.abweichung(&self.schluessel).is_none(),
                    schluessel_json(g)
                )
            )
        );
        let _ = write!(
            j,
            "\"counters\":{},",
            match &self.zaehler {
                Ok(hinweis) => format!(
                    "{{\"available\":true,\"core_class\":\"{}\",\"cpu\":{},\"note\":{}}}",
                    self.schluessel.kernklasse.kennung(),
                    self.prozessor
                        .as_deref()
                        .map_or_else(|| "null".to_owned(), json_string),
                    json_string(hinweis)
                ),
                Err(grund) => format!(
                    "{{\"available\":false,\"core_class\":\"{}\",\"cpu\":{},\"reason\":{},\
                     \"fallback\":\"cpu_time_and_psi\"}}",
                    self.schluessel.kernklasse.kennung(),
                    self.prozessor
                        .as_deref()
                        .map_or_else(|| "null".to_owned(), json_string),
                    json_string(grund)
                ),
            }
        );
        let _ = write!(j, "\"measured\":{}}}", messwerte_json(&self.messwerte));
        j
    }
}

fn abdeckung(a: Option<f64>) -> String {
    a.map_or_else(|| "—".to_owned(), |a| format!("{:.0} %", a * 100.0))
}

fn schluessel_json(s: &Schluessel) -> String {
    format!(
        "{{\"host\":{},\"core_class\":\"{}\",\"load_profile\":{}}}",
        json_string(&s.rechner),
        s.kernklasse.kennung(),
        json_string(&s.lastprofil)
    )
}

fn zahl(x: Option<f64>) -> String {
    x.and_then(|x| json_number(x, "engpass").ok())
        .unwrap_or_else(|| "null".to_owned())
}

fn messwerte_json(m: &Messwerte) -> String {
    let mut j = format!(
        "{{\"window_s\":{},\"fights\":{},\"cpu_ns_per_fight\":{},",
        zahl(Some(m.sekunden)),
        m.kaempfe,
        zahl(m.cpu_ns_je_kampf)
    );
    match &m.zaehler {
        Zaehlerlage::Gemessen(z) => {
            let _ = write!(
                j,
                "\"counters\":{{\"ipc\":{},\"cycles_per_fight\":{},\"branch_misses_per_fight\":{},\
                 \"l1d_misses_per_fight\":{},\"l2_misses_per_fight\":{},\"l3_misses_per_fight\":{},\
                 \"memory_wait_cycles_per_fight\":{},\"memory_wait_share\":{},\"coverage\":[{},{}]}},",
                zahl(z.ipc),
                zahl(z.zyklen_je_kampf),
                zahl(z.fehlspruenge_je_kampf),
                zahl(z.l1_je_kampf),
                zahl(z.l2_je_kampf),
                zahl(z.l3_je_kampf),
                zahl(z.speicherwartezyklen_je_kampf),
                zahl(z.speicher_wartezeit_anteil),
                zahl(z.abdeckung[0]),
                zahl(z.abdeckung[1])
            );
        }
        Zaehlerlage::Fehlt(grund) => {
            let _ = write!(j, "\"counters\":{{\"missing\":{}}},", json_string(grund));
        }
    }
    let straenge: Vec<String> = m
        .straenge
        .iter()
        .map(|s| {
            format!(
                "{{\"work_ns\":{},\"wait_ns\":{}}}",
                s.arbeit_ns, s.warten_ns
            )
        })
        .collect();
    let _ = write!(
        j,
        "\"threads\":[{}],\"wait_share\":{},",
        straenge.join(","),
        zahl(m.warteanteil())
    );
    let wege: Vec<String> = m
        .wege
        .iter()
        .map(|w| {
            format!(
                "{{\"path\":{},\"attempts\":{},\"hits\":{},\"hit_rate\":{},\"break_even\":{},\
                 \"without_break_even\":{}}}",
                json_string(w.weg),
                w.versuche,
                w.treffer,
                zahl(w.quote()),
                zahl(w.schwelle),
                json_string(w.ohne_schwelle)
            )
        })
        .collect();
    let _ = write!(j, "\"paths\":[{}],", wege.join(","));
    let _ = write!(
        j,
        "\"network\":{},",
        m.netz.map_or_else(
            || "null".to_owned(),
            |n| format!(
                "{{\"packets\":{},\"wait_ns\":{},\"busy_ns\":{},\"dropped\":{}}}",
                n.pakete, n.wartezeit_ns, n.laufzeit_ns, n.verworfen
            )
        )
    );
    let _ = write!(
        j,
        "\"event_queue\":{},",
        m.schlange.map_or_else(
            || "null".to_owned(),
            |s| format!(
                "{{\"max\":{},\"capacity\":{},\"overflows\":{},\"samples\":{}}}",
                s.hoechststand, s.kapazitaet, s.ueberlaeufe, s.schritte
            )
        )
    );
    let _ = write!(
        j,
        "\"expire_buffs\":{},",
        m.ablauf.map_or_else(
            || "null".to_owned(),
            |a| format!(
                "{{\"calls\":{},\"hits\":{},\"hit_rate\":{}}}",
                a.aufrufe,
                a.treffer,
                zahl(a.quote())
            )
        )
    );
    let _ = write!(
        j,
        "\"psi_some_avg10\":{{\"cpu\":{},\"memory\":{},\"io\":{}}}}}",
        zahl(m.druck.cpu),
        zahl(m.druck.speicher),
        zahl(m.druck.io)
    );
    j
}
