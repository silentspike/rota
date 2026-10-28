//! Die Regeltabelle (AC2, AC5): aus Messwerten ein Satz.
//!
//! # Die Tabelle
//!
//! | Beobachtung | Diagnose | nächster Schritt |
//! | --- | --- | --- |
//! | Durchsatz deutlich unter, Fehlsprünge deutlich über der Grundlage | verzweigungsgebunden | Kernel-Spezialisierung, profilgeführter Bau |
//! | Fehlgriffe der zweiten Ebene hoch, dritte niedrig | Zustand zu groß für die zweite Ebene | Suchzustand weiter verdichten |
//! | Stränge warten über 15 % | Pakete zu groß oder zu wenige | Paketlänge senken |
//! | Netzwartezeit über 10 % | Pakete zu klein | Paketgröße erhöhen |
//! | Trefferquote unter der eigenen Schwelle | dieser Weg lohnt nicht | Weg abschalten |
//! | Ablaufprüfung trifft über 20 % | die Kernel-Annahme gilt nicht mehr | neu messen, Endausbau erreicht |
//! | Ereignisschlange über 80 % voll | Kapazität wird knapp | **vor** dem Überlauf handeln |
//!
//! Wort für Wort aus Issue #934, AC5. Dazu eine achte Zeile, die nur im
//! Rückfall gilt: [`Regel::ZeitOhneZaehler`].
//!
//! # Welche Schwellen relativ sind und welche Anteile
//!
//! **Relativ zur Grundlage** sind genau die vier Größen aus AC2 — die, deren
//! Betrag an der Kernklasse hängt: Fehlsprünge, Befehlsdurchsatz, Zyklen je
//! Ereignis, Speicher-Wartezeit. Für sie gibt es keine feste Zahl, auch keine
//! Voreinstellung: Ohne passende Grundlage werden die beiden Zeilen, die sie
//! lesen, nicht geprüft.
//!
//! **Anteile** sind die übrigen fünf Schwellen, so wie AC5 sie nennt: welcher
//! Teil der Strangzeit Warten war, welcher Teil der Paketzeit Netz, welcher
//! Teil der Aufrufe traf, welcher Teil der Kapazität belegt ist. Ein Anteil
//! der eigenen Zeit bedeutet auf jeder Kernklasse dasselbe.
//!
//! # Zwei Stellen, die die Tabelle offen lässt, und wie sie hier gefüllt sind
//!
//! 1. **„Zyklen je Ereignis +30 %"** steht in AC2, aber in keiner Zeile. Hier
//!    ist es die gemeinsame Vorbedingung der beiden Zählerzeilen: Erst wenn
//!    ein Kampf messbar mehr Zyklen kostet als in der Grundlage, wird gefragt,
//!    woran. Ohne sie schlüge eine Zeile auf ein Verhältnis zweier kleiner
//!    Zahlen an, das nichts kostet. Gezählt wird je Kampf: Für ein festes
//!    Lastprofil ist die Zahl der Ereignisse je Kampf im deterministischen
//!    Simulator dieselbe, die relative Abweichung also gleich.
//! 2. **„hoch" und „niedrig" der zweiten Zeile** nennen keine eigene Zahl.
//!    „Hoch" ist die Speicherschwelle aus AC2: Fehlgriffe der zweiten Ebene
//!    und Speicher-Wartezeit je mindestens +50 %. „Niedrig" heißt: **Von den
//!    zusätzlichen Fehlgriffen der zweiten Ebene geht weniger als die Hälfte
//!    auch an der dritten vorbei** ([`DRITTE_EBENE_ANTEIL`]) — die Mehrheit
//!    trifft dort, der Zustand passt in L3, nicht in L2.
//!
//!    Hier stand zuerst „die dritte Ebene unter +50 % gegen die Grundlage".
//!    Die Lage „Speicher" auf dem Laptop hat das widerlegt (2026-10-28):
//!    Fehlgriffe der zweiten Ebene 4,6 → 142,4 je Kampf, der dritten
//!    2,27 → 6,32. Relativ waren das +178 % — bei einer dritten Ebene, die in
//!    der Grundlage fast nichts zu tun hat, ist jede Handvoll Fehlgriffe ein
//!    großer Anteil. Von den 138 zusätzlichen gingen 4 an L3 vorbei, also 3 %.
//!    Ein Zustand, der auch für L3 zu groß ist, schickt dagegen fast jeden
//!    zusätzlichen Fehlgriff weiter — dann schweigt die Zeile, denn die
//!    Tabelle hat für ihn keine.

use super::grundlage::{Grundlage, Schluessel};
use super::messwerte::{Messwerte, Zaehlerlage, Zaehlerwerte};

/// Fehlsprünge je Kampf: mindestens so viel über der Grundlage (AC2).
pub const FEHLSPRUENGE_PLUS: f64 = 0.40;
/// Befehlsdurchsatz: mindestens so viel unter der Grundlage (AC2).
pub const DURCHSATZ_MINUS: f64 = 0.25;
/// Zyklen je Ereignis: mindestens so viel über der Grundlage (AC2).
pub const ZYKLEN_JE_EREIGNIS_PLUS: f64 = 0.30;
/// Speicher-Wartezeit: mindestens so viel über der Grundlage (AC2).
pub const SPEICHER_WARTEZEIT_PLUS: f64 = 0.50;
/// Anteil der Strangzeit, der Warten war: mehr als das (AC5).
pub const STRANG_WARTEN_ANTEIL: f64 = 0.15;
/// Anteil der Paketzeit, der Netzwartezeit war: mehr als das (AC5).
pub const NETZ_WARTEN_ANTEIL: f64 = 0.10;
/// Anteil der Ablaufprüfungen, die etwas abräumen: mehr als das (AC5).
pub const ABLAUF_TREFFER_ANTEIL: f64 = 0.20;
/// Belegung der Ereignisschlange: mehr als das ihrer Kapazität (AC5).
pub const SCHLANGE_BELEGUNG_ANTEIL: f64 = 0.80;
/// Zeile 2, „dritte niedrig": Von den zusätzlichen Fehlgriffen der zweiten
/// Ebene gehen weniger als dieser Anteil auch an der dritten vorbei — die
/// Mehrheit trifft dort. Ein Anteil der eigenen Fehlgriffe, kein Betrag; was
/// er ersetzt und warum, steht im Kopf dieser Datei.
pub const DRITTE_EBENE_ANTEIL: f64 = 0.50;

/// Eine Zeile der Tabelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regel {
    /// Durchsatz deutlich unter, Fehlsprünge deutlich über der Grundlage.
    Verzweigungsgebunden,
    /// Fehlgriffe der zweiten Ebene hoch, der dritten niedrig.
    ZweiteEbene,
    /// Stränge warten über 15 %.
    StraengeWarten,
    /// Netzwartezeit über 10 %.
    NetzWarten,
    /// Trefferquote eines Wegs unter seiner eigenen Schwelle.
    WegLohntNicht,
    /// Die Ablaufprüfung trifft über 20 %.
    AblaufAnnahme,
    /// Die Ereignisschlange ist über 80 % voll.
    SchlangeKnapp,
    /// **Nur im Rückfall:** Ohne Leistungszähler kostet ein Kampf mindestens
    /// +30 % CPU-Zeit gegenüber der Grundlage. Die Ursache lässt sich ohne
    /// Zähler nicht eingrenzen — gesagt wird, dass es eine gibt.
    ZeitOhneZaehler,
}

impl Regel {
    /// Die sieben Zeilen aus AC5, in der Reihenfolge der Tabelle.
    pub const TABELLE: [Self; 7] = [
        Self::Verzweigungsgebunden,
        Self::ZweiteEbene,
        Self::StraengeWarten,
        Self::NetzWarten,
        Self::WegLohntNicht,
        Self::AblaufAnnahme,
        Self::SchlangeKnapp,
    ];

    /// Der Schlüssel im JSON-Strom.
    #[must_use]
    pub const fn schluessel(self) -> &'static str {
        match self {
            Self::Verzweigungsgebunden => "branch_bound",
            Self::ZweiteEbene => "state_exceeds_l2",
            Self::StraengeWarten => "threads_waiting",
            Self::NetzWarten => "network_wait",
            Self::WegLohntNicht => "path_below_break_even",
            Self::AblaufAnnahme => "expire_assumption_expired",
            Self::SchlangeKnapp => "event_queue_near_capacity",
            Self::ZeitOhneZaehler => "slower_without_counters",
        }
    }

    /// Die Diagnose, Wort für Wort aus der Tabelle.
    #[must_use]
    pub const fn diagnose(self) -> &'static str {
        match self {
            Self::Verzweigungsgebunden => "verzweigungsgebunden",
            Self::ZweiteEbene => "Zustand zu groß für die zweite Ebene",
            Self::StraengeWarten => "Pakete zu groß oder zu wenige",
            Self::NetzWarten => "Pakete zu klein",
            Self::WegLohntNicht => "dieser Weg lohnt nicht",
            Self::AblaufAnnahme => "die Kernel-Annahme gilt nicht mehr",
            Self::SchlangeKnapp => "Kapazität der Ereignisschlange wird knapp",
            Self::ZeitOhneZaehler => {
                "langsamer als die Grundlage — ohne Leistungszähler nicht eingegrenzt"
            }
        }
    }

    /// Der nächste Schritt, aus der Tabelle samt dem Abschnitt des Plans.
    #[must_use]
    pub const fn naechster_schritt(self) -> &'static str {
        match self {
            Self::Verzweigungsgebunden => "Kernel-Spezialisierung, profilgeführter Bau (E11.7b)",
            Self::ZweiteEbene => "Suchzustand weiter verdichten (E11.4)",
            Self::StraengeWarten => "Paketlänge senken",
            Self::NetzWarten => "Paketgröße erhöhen (Teil 2.1)",
            Self::WegLohntNicht => "Weg abschalten (E11.4b)",
            Self::AblaufAnnahme => "neu messen, Endausbau erreicht (E11.2)",
            Self::SchlangeKnapp => "vor dem Überlauf handeln (E11.0b)",
            Self::ZeitOhneZaehler => "auf einem Rechner mit Leistungszählern nachmessen",
        }
    }
}

/// Eine Zeile, die angeschlagen hat — mit den Zahlen, die sie belegen.
#[derive(Debug, Clone, PartialEq)]
pub struct Befund {
    /// Welche Zeile.
    pub regel: Regel,
    /// Was beobachtet wurde, mit Zahlen.
    pub beobachtung: String,
}

impl Befund {
    /// Der Satz: Diagnose, Beobachtung, nächster Schritt.
    #[must_use]
    pub fn satz(&self) -> String {
        format!(
            "{}: {} — nächster Schritt: {}",
            self.regel.diagnose(),
            self.beobachtung,
            self.regel.naechster_schritt()
        )
    }
}

/// Was die Tabelle über einen Lauf sagt.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagnose {
    /// Die Zeilen, die angeschlagen haben, in der Reihenfolge der Tabelle.
    pub befunde: Vec<Befund>,
    /// Die Zeilen, die geprüft wurden — angeschlagen oder nicht.
    pub geprueft: Vec<Regel>,
    /// Die Zeilen, die nicht geprüft werden konnten, mit dem Grund.
    pub ungeprueft: Vec<(Regel, String)>,
}

impl Diagnose {
    /// Der Satz für die Anzeige.
    ///
    /// **Drei Ausgänge, nicht zwei.** Ein Befund ist ein Satz mit Ursache und
    /// Schritt. Keine Zeile angeschlagen heißt nur dann „keine Diagnose",
    /// wenn überhaupt eine geprüft wurde — sonst sagt der Satz, warum nichts
    /// geprüft ist. Ein Wächter, der nichts prüfen kann, darf nicht wie einer
    /// aussehen, der nichts gefunden hat.
    #[must_use]
    pub fn satz(&self) -> String {
        if let Some(erster) = self.befunde.first() {
            let mut s = erster.satz();
            if self.befunde.len() > 1 {
                s.push_str(&format!(" (+{} weitere)", self.befunde.len() - 1));
            }
            return s;
        }
        if self.geprueft.is_empty() {
            let grund = self
                .ungeprueft
                .first()
                .map_or("keine Messwerte", |(_, g)| g.as_str());
            return format!("noch keine Diagnose — {grund}");
        }
        format!(
            "keine Diagnose — {} von {} Zeilen geprüft, keine schlägt an",
            self.geprueft.len(),
            self.geprueft.len() + self.ungeprueft.len()
        )
    }
}

/// Relative Abweichung von der Grundlage: `x / g − 1`.
fn relativ(x: Option<f64>, g: Option<f64>) -> Option<f64> {
    match (x, g) {
        (Some(x), Some(g)) if g > 0.0 && x.is_finite() && g.is_finite() => Some(x / g - 1.0),
        _ => None,
    }
}

fn prozent(x: f64) -> String {
    format!("{:+.0} %", x * 100.0).replace('-', "−")
}

fn anteil(x: f64) -> String {
    format!("{:.1} %", x * 100.0).replace('.', ",")
}

fn zahl(x: f64) -> String {
    if x.abs() >= 100.0 {
        format!("{x:.0}")
    } else {
        format!("{x:.2}").replace('.', ",")
    }
}

/// Die Regeltabelle: eine reine Funktion der Messwerte, des Schlüssels des
/// Laufs und seiner Grundlage.
#[must_use]
pub fn diagnose(m: &Messwerte, schluessel: &Schluessel, grundlage: Option<&Grundlage>) -> Diagnose {
    let mut d = Diagnose::default();

    // Dieselbe Grenze wie bei der Restzeit (#933): Unter vier Messsekunden
    // ist eine Abweichung Zufall, und der Anfang eines Laufs lädt Code und
    // Daten — er sieht aus wie ein Speicherengpass und ist keiner.
    let mindestens = crate::kalibrierung::ABSCHNITTE_MIN as f64;
    if m.sekunden < mindestens {
        for regel in Regel::TABELLE {
            d.ungeprueft.push((
                regel,
                format!(
                    "erst ab {mindestens:.0} Messsekunden, das Fenster hat {}",
                    zahl(m.sekunden)
                ),
            ));
        }
        return d;
    }

    // Gegen welche Grundlage — oder warum gegen keine.
    let vergleich = match grundlage {
        None => Err(
            "keine Grundlage für diesen Rechner, diese Kernklasse und dieses Lastprofil".to_owned(),
        ),
        Some(g) => match g.schluessel.abweichung(schluessel) {
            Some(was) => Err(format!(
                "die Grundlage passt nicht: {was} (Grundlage: {} · {} · {})",
                g.schluessel.rechner,
                g.schluessel.kernklasse.kennung(),
                g.schluessel.lastprofil
            )),
            None => Ok(g),
        },
    };

    match (&m.zaehler, &vergleich) {
        (Zaehlerlage::Gemessen(z), Ok(g)) => zaehlerzeilen(&mut d, z, g),
        (Zaehlerlage::Gemessen(_), Err(grund)) => {
            d.ungeprueft
                .push((Regel::Verzweigungsgebunden, grund.clone()));
            d.ungeprueft.push((Regel::ZweiteEbene, grund.clone()));
        }
        (Zaehlerlage::Fehlt(grund), _) => {
            let satz = format!(
                "Leistungszähler fehlen: {grund} — Rückfall auf Zeitmessung und Druckwerte"
            );
            d.ungeprueft
                .push((Regel::Verzweigungsgebunden, satz.clone()));
            d.ungeprueft.push((Regel::ZweiteEbene, satz));
        }
    }

    match m.warteanteil() {
        None => d.ungeprueft.push((
            Regel::StraengeWarten,
            "kein über mehrere Stränge verteilter Abschnitt im Fenster".to_owned(),
        )),
        Some(w) => {
            d.geprueft.push(Regel::StraengeWarten);
            if w > STRANG_WARTEN_ANTEIL {
                d.befunde.push(Befund {
                    regel: Regel::StraengeWarten,
                    beobachtung: format!(
                        "Stränge warten {} ihrer Zeit (über {}), {} Stränge",
                        anteil(w),
                        anteil(STRANG_WARTEN_ANTEIL),
                        m.straenge.len()
                    ),
                });
            }
        }
    }

    match &m.netz {
        None => d.ungeprueft.push((
            Regel::NetzWarten,
            "kein Netz im Lauf — die Verteilung über mehrere Rechner (E15.5) ist nicht gebaut"
                .to_owned(),
        )),
        Some(n) if n.laufzeit_ns == 0 => d
            .ungeprueft
            .push((Regel::NetzWarten, "kein Paket im Fenster".to_owned())),
        Some(n) => {
            d.geprueft.push(Regel::NetzWarten);
            let w = n.wartezeit_ns as f64 / n.laufzeit_ns as f64;
            if w > NETZ_WARTEN_ANTEIL {
                d.befunde.push(Befund {
                    regel: Regel::NetzWarten,
                    beobachtung: format!(
                        "Netzwartezeit {} der Paketzeit (über {}), {} Pakete, {} verworfen",
                        anteil(w),
                        anteil(NETZ_WARTEN_ANTEIL),
                        n.pakete,
                        n.verworfen
                    ),
                });
            }
        }
    }

    let mit_schwelle: Vec<_> = m
        .wege
        .iter()
        .filter(|w| w.schwelle.is_some() && w.versuche > 0)
        .collect();
    if mit_schwelle.is_empty() {
        let ohne: Vec<String> = m
            .wege
            .iter()
            .map(|w| format!("{}: {}", w.weg, w.ohne_schwelle))
            .collect();
        d.ungeprueft.push((
            Regel::WegLohntNicht,
            if ohne.is_empty() {
                "kein Weg mit eigener Schwelle im Lauf".to_owned()
            } else {
                format!(
                    "kein Weg mit eigener Schwelle im Lauf ({})",
                    ohne.join("; ")
                )
            },
        ));
    } else {
        d.geprueft.push(Regel::WegLohntNicht);
        for w in mit_schwelle {
            let (Some(q), Some(s)) = (w.quote(), w.schwelle) else {
                continue;
            };
            if q < s {
                d.befunde.push(Befund {
                    regel: Regel::WegLohntNicht,
                    beobachtung: format!(
                        "{} trifft {} von {} ({}), seine Schwelle ist {}",
                        w.weg,
                        w.treffer,
                        w.versuche,
                        anteil(q),
                        anteil(s)
                    ),
                });
            }
        }
    }

    let ohne_diagnostics = "nicht erhoben — der Bau hat das Merkmal 'diagnostics' nicht";
    match &m.ablauf {
        None => d.ungeprueft.push((
            Regel::AblaufAnnahme,
            if crate::diagnostics::im_bau() {
                "keine Ablaufprüfung im Fenster".to_owned()
            } else {
                ohne_diagnostics.to_owned()
            },
        )),
        Some(a) => {
            d.geprueft.push(Regel::AblaufAnnahme);
            if let Some(q) = a.quote().filter(|q| *q > ABLAUF_TREFFER_ANTEIL) {
                d.befunde.push(Befund {
                    regel: Regel::AblaufAnnahme,
                    beobachtung: format!(
                        "expire_buffs trifft {} von {} Aufrufen ({}, über {})",
                        a.treffer,
                        a.aufrufe,
                        anteil(q),
                        anteil(ABLAUF_TREFFER_ANTEIL)
                    ),
                });
            }
        }
    }

    match &m.schlange {
        None => d.ungeprueft.push((
            Regel::SchlangeKnapp,
            if crate::diagnostics::im_bau() {
                "keine Ablesung der Schlange im Fenster".to_owned()
            } else {
                ohne_diagnostics.to_owned()
            },
        )),
        Some(s) => {
            d.geprueft.push(Regel::SchlangeKnapp);
            let belegt = s.hoechststand as f64 / s.kapazitaet.max(1) as f64;
            if belegt > SCHLANGE_BELEGUNG_ANTEIL || s.ueberlaeufe > 0 {
                d.befunde.push(Befund {
                    regel: Regel::SchlangeKnapp,
                    beobachtung: format!(
                        "Höchststand {} von {} Plätzen ({}, über {}), {} Ausweichfälle",
                        s.hoechststand,
                        s.kapazitaet,
                        anteil(belegt),
                        anteil(SCHLANGE_BELEGUNG_ANTEIL),
                        s.ueberlaeufe
                    ),
                });
            }
        }
    }

    // Der Rückfall: nur, wenn die Zähler fehlen.
    if let Zaehlerlage::Fehlt(_) = &m.zaehler {
        rueckfall(&mut d, m, &vergleich);
    }

    // Die Befunde in der Reihenfolge der Tabelle, der Rückfall zuletzt.
    let rang = |r: Regel| Regel::TABELLE.iter().position(|t| *t == r).unwrap_or(7);
    d.befunde.sort_by_key(|b| rang(b.regel));
    d
}

/// Die beiden Zählerzeilen gegen eine passende Grundlage.
fn zaehlerzeilen(d: &mut Diagnose, z: &Zaehlerwerte, g: &Grundlage) {
    let zyklen = relativ(z.zyklen_je_kampf, g.werte.zyklen_je_kampf);

    let ipc = relativ(z.ipc, g.werte.ipc);
    let spruenge = relativ(z.fehlspruenge_je_kampf, g.werte.fehlspruenge_je_kampf);
    match (zyklen, ipc, spruenge) {
        (Some(zy), Some(ipc), Some(sp)) => {
            d.geprueft.push(Regel::Verzweigungsgebunden);
            if zy >= ZYKLEN_JE_EREIGNIS_PLUS && ipc <= -DURCHSATZ_MINUS && sp >= FEHLSPRUENGE_PLUS {
                d.befunde.push(Befund {
                    regel: Regel::Verzweigungsgebunden,
                    beobachtung: format!(
                        "Befehlsdurchsatz {} statt {} ({}), Fehlsprünge je Kampf {} statt {} ({}), \
                         Zyklen je Kampf {}",
                        zahl(z.ipc.unwrap_or(f64::NAN)),
                        zahl(g.werte.ipc.unwrap_or(f64::NAN)),
                        prozent(ipc),
                        zahl(z.fehlspruenge_je_kampf.unwrap_or(f64::NAN)),
                        zahl(g.werte.fehlspruenge_je_kampf.unwrap_or(f64::NAN)),
                        prozent(sp),
                        prozent(zy)
                    ),
                });
            }
        }
        _ => d.ungeprueft.push((
            Regel::Verzweigungsgebunden,
            "Zyklen, Befehle oder Fehlsprünge fehlen im Fenster oder in der Grundlage".to_owned(),
        )),
    }

    let speicher = relativ(
        z.speicherwartezyklen_je_kampf,
        g.werte.speicherwartezyklen_je_kampf,
    );
    let l2 = relativ(z.l2_je_kampf, g.werte.l2_je_kampf);
    // Welcher Teil der ZUSÄTZLICHEN Fehlgriffe der zweiten Ebene auch an der
    // dritten vorbeiging — nur bestimmt, wenn es zusätzliche gibt.
    let weiter = match (
        z.l2_je_kampf,
        g.werte.l2_je_kampf,
        z.l3_je_kampf,
        g.werte.l3_je_kampf,
    ) {
        (Some(l2), Some(g2), Some(l3), Some(g3)) if l2 > g2 => {
            Some(((l3 - g3) / (l2 - g2)).max(0.0))
        }
        (Some(_), Some(_), Some(_), Some(_)) => Some(f64::INFINITY),
        _ => None,
    };
    match (zyklen, speicher, l2, weiter) {
        (Some(zy), Some(sp), Some(l2), Some(weiter)) => {
            d.geprueft.push(Regel::ZweiteEbene);
            if zy >= ZYKLEN_JE_EREIGNIS_PLUS
                && sp >= SPEICHER_WARTEZEIT_PLUS
                && l2 >= SPEICHER_WARTEZEIT_PLUS
                && weiter < DRITTE_EBENE_ANTEIL
            {
                d.befunde.push(Befund {
                    regel: Regel::ZweiteEbene,
                    beobachtung: format!(
                        "Fehlgriffe der zweiten Ebene je Kampf {} statt {} ({}), davon {} auch an \
                         der dritten vorbei ({} statt {}), Speicher-Wartezeit {}, Zyklen je Kampf {}",
                        zahl(z.l2_je_kampf.unwrap_or(f64::NAN)),
                        zahl(g.werte.l2_je_kampf.unwrap_or(f64::NAN)),
                        prozent(l2),
                        anteil(weiter),
                        zahl(z.l3_je_kampf.unwrap_or(f64::NAN)),
                        zahl(g.werte.l3_je_kampf.unwrap_or(f64::NAN)),
                        prozent(sp),
                        prozent(zy)
                    ),
                });
            }
        }
        _ => d.ungeprueft.push((
            Regel::ZweiteEbene,
            "Zyklen, Speicher-Wartezeit oder Fehlgriffe fehlen im Fenster oder in der Grundlage"
                .to_owned(),
        )),
    }
}

/// Die Zeile des Rückfalls: CPU-Zeit je Kampf gegen die Grundlage, sofern
/// der Druck die Zeitmessung nicht verfälscht.
fn rueckfall(d: &mut Diagnose, m: &Messwerte, vergleich: &Result<&Grundlage, String>) {
    let g = match vergleich {
        Ok(g) => g,
        Err(grund) => {
            d.ungeprueft.push((Regel::ZeitOhneZaehler, grund.clone()));
            return;
        }
    };
    // Fremdlast verlängert die CPU-Zeit je Kampf über geteilte Kerne und
    // Zwischenspeicher, ohne dass der Lauf etwas falsch macht. Darüber
    // entscheidet derselbe Wert, unter dem das Projekt überhaupt misst.
    let grenze = crate::hostcaps::PSI_CPU_MAX;
    match m.druck.cpu {
        Some(psi) if psi > grenze => {
            d.ungeprueft.push((
                Regel::ZeitOhneZaehler,
                format!(
                    "Fremdlast: PSI cpu some avg10 {} % über {} % — die Zeitmessung ist nicht \
                     aussagekräftig",
                    zahl(psi),
                    zahl(grenze)
                ),
            ));
            return;
        }
        None => {
            d.ungeprueft.push((
                Regel::ZeitOhneZaehler,
                "kein Druckwert lesbar — ohne ihn ist eine Zeitmessung unter Fremdlast nicht \
                 von einer Verlangsamung zu trennen"
                    .to_owned(),
            ));
            return;
        }
        Some(_) => {}
    }
    match relativ(m.cpu_ns_je_kampf, g.werte.cpu_ns_je_kampf) {
        None => d.ungeprueft.push((
            Regel::ZeitOhneZaehler,
            "CPU-Zeit je Kampf fehlt im Fenster oder in der Grundlage".to_owned(),
        )),
        Some(r) => {
            d.geprueft.push(Regel::ZeitOhneZaehler);
            if r >= ZYKLEN_JE_EREIGNIS_PLUS {
                d.befunde.push(Befund {
                    regel: Regel::ZeitOhneZaehler,
                    beobachtung: format!(
                        "CPU-Zeit je Kampf {} µs statt {} µs ({}), PSI cpu {} %",
                        zahl(m.cpu_ns_je_kampf.unwrap_or(f64::NAN) / 1e3),
                        zahl(g.werte.cpu_ns_je_kampf.unwrap_or(f64::NAN) / 1e3),
                        prozent(r),
                        zahl(m.druck.cpu.unwrap_or(f64::NAN))
                    ),
                });
            }
        }
    }
}
