//! Zwei Ausgabewege aus EINER Anzeige (AC1, AC2, AC8).
//!
//! Die Zeile für Menschen und der JSON-Strom entstehen aus demselben Wert vom
//! Typ [`Anzeige`], und der hält genau eine [`Momentaufnahme`]. Keine der
//! beiden Ausgaben liest selbst einen Zähler — sie können also nicht
//! auseinanderlaufen, und ein Prüffall hält das fest.

use std::fmt::Write as _;
use std::time::Duration;

use super::fenster::Raten;
use super::restzeit::{Herleitung, Kampfzahl, Kostenbild, Restkosten, Restzeit};
use super::zaehlwerk::{Arbeitsmenge, Momentaufnahme, Weg, WEGE_OHNE_LAUF};
use crate::drosselung::ControlSnapshot;
use crate::kampagne::{json_number, json_string};

/// Breite des Fortschrittsbalkens in Zeichen.
const BALKEN: usize = 16;

/// Welche Art Meldung eine Zeile des Stroms ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ereignis {
    /// Der Lauf beginnt.
    Start,
    /// Ein Takt des Anzeigestrangs.
    Fortschritt,
    /// Der Lauf ist zu Ende.
    Ende,
}

impl Ereignis {
    /// Der Name im Feld `event`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Fortschritt => "progress",
            Self::Ende => "end",
        }
    }
}

/// Ein Rechner mit seinen Strängen, wie die Zeile ihn zeigt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RechnerZeile {
    /// Kennung, etwa der Hostname.
    pub id: String,
    /// Arbeitsstränge, die der Lauf bekommen hat.
    pub straenge: usize,
    /// Davon in der letzten Sekunde tätig.
    pub aktiv: usize,
    /// Kerne, die der Prozess benutzen darf — `None`, wenn nicht lesbar.
    pub erlaubt: Option<usize>,
}

/// Alles, was beide Ausgaben brauchen — aus einer Momentaufnahme.
///
/// Ohne `PartialEq`: [`ControlSnapshot`] trägt keines, und zwei Anzeigen
/// vergleicht ohnehin niemand — verglichen werden ihre beiden Ausgaben.
#[derive(Debug, Clone)]
pub struct Anzeige {
    /// Welche Meldung.
    pub ereignis: Ereignis,
    /// Laufende Nummer dieses Rechners, ab 1 — zeigt Lücken und Verspätungen.
    pub sequenz: u64,
    /// Weltzeit der Momentaufnahme, ISO 8601 in UTC mit Mikrosekunden.
    pub utc: String,
    /// Was gerade läuft: `greedy`, `fest`, `gestaffelt`, `bench` …
    pub phase: String,
    /// Gegen welchen Boss.
    pub boss: Option<String>,
    /// Die eine Momentaufnahme.
    pub probe: Momentaufnahme,
    /// Die Raten aus dem Fenster — `None` in der ersten Viertelsekunde.
    pub raten: Option<Raten>,
    /// Was eine Auswertung kostet.
    pub kosten: Kostenbild,
    /// Die Restzeit oder das Plateau.
    pub restzeit: Restzeit,
    /// Die Rechner mit ihren Strängen.
    pub rechner: Vec<RechnerZeile>,
    /// Die aktive Drosselung — `None` ohne Steuerdatei.
    pub drosselung: Option<ControlSnapshot>,
}

impl Anzeige {
    /// Die Zeilen für Menschen.
    #[must_use]
    pub fn zeilen(&self) -> Vec<String> {
        let p = &self.probe;
        vec![
            self.kopfzeile(),
            format!(
                "  Kandidaten   {:<34} seit {}",
                self.kandidaten_text(),
                hms(sekunden(p.elapsed_ns))
            ),
            self.durchsatz_zeile(),
            self.wege_zeile(),
            self.kernel_zeile(),
            format!("  Restzeit     {}", self.restzeit_text()),
            format!("  Systeme      {}", self.rechner_text()),
            format!("  Drosselung   {}", self.drossel_text()),
            "  Engpass      keine Diagnose — die Engpasserkennung folgt mit E15.3 (#934)"
                .to_owned(),
            format!("  Bestes       {}", self.bestes_text()),
            format!(
                "  Meldung      #{} · {} · {}",
                self.sequenz,
                self.utc,
                self.ereignis.name()
            ),
        ]
    }

    /// Eine Zeile JSON Lines, ohne Zeilenende.
    ///
    /// Die Feldnamen der obersten Ebene sind die aus AC8, Wort für Wort. Was
    /// unter `metrics` steht, trägt dieselben Zahlen wie [`Anzeige::zeilen`],
    /// ungerundet.
    #[must_use]
    pub fn jsonl(&self, run_id: &str, host_id: &str) -> String {
        let mut m = String::from("{");
        let p = &self.probe;
        let s = &p.stand;
        let _ = write!(
            m,
            "\"boss\":{},\"round\":{},",
            self.boss
                .as_deref()
                .map_or_else(|| "null".to_owned(), json_string),
            ganz_oder_null(s.runde)
        );
        let _ = write!(
            m,
            "\"candidates\":{{\"done\":{},\"round_done\":{},\"round_planned\":{},\"amount\":{}}},",
            p.kandidaten_gesamt(),
            ganz_oder_null(
                s.runde
                    .map(|_| p.kandidaten_gesamt().saturating_sub(s.runde_ab_kandidaten))
            ),
            ganz_oder_null(s.runde.map(|_| s.runde_geplant)),
            menge_json(s.menge)
        );
        let _ = write!(
            m,
            "\"fights\":{},\"samples\":{},\"avoided_fights\":{},",
            p.kaempfe,
            p.durchgaenge,
            p.durchgaenge.saturating_sub(p.kaempfe)
        );
        let _ = write!(m, "\"throughput\":{},", self.durchsatz_json());
        let _ = write!(m, "\"paths\":{},", self.wege_json());
        let _ = write!(m, "\"cost\":{},", kosten_json(self.kosten));
        let _ = write!(m, "\"eta\":{},", restzeit_json(&self.restzeit));
        let _ = write!(m, "\"plateau\":{},", self.plateau_json());
        let _ = write!(m, "\"hosts\":{},", self.rechner_json());
        let _ = write!(
            m,
            "\"throttle\":{},",
            self.drosselung
                .map_or_else(|| "null".to_owned(), |d| drossel_json(&d))
        );
        m.push_str(
            "\"bottleneck\":{\"diagnosis\":null,\"reason\":\"keine Diagnose: E15.3 (#934) ist nicht gebaut\"}}",
        );
        format!(
            "{{\"schema_version\":1,\"run_id\":{},\"event\":{},\"timestamp_utc\":{},\
             \"elapsed_ns\":{},\"host_id\":{},\"host_sequence\":{},\"phase\":{},\"metrics\":{m}}}",
            json_string(run_id),
            json_string(self.ereignis.name()),
            json_string(&self.utc),
            p.elapsed_ns,
            json_string(host_id),
            self.sequenz,
            json_string(&self.phase),
        )
    }

    /// Wie viel erledigt ist, und wovon — oder `None`, wenn es kein Ganzes gibt.
    fn anteil(&self) -> Option<(f64, &'static str)> {
        let p = &self.probe;
        let s = &p.stand;
        let (zaehler, nenner, wovon) = match (s.menge, self.kosten.kaempfe_je_kandidat) {
            (Arbeitsmenge::Bekannt { kandidaten }, Kampfzahl::Fest(f)) => {
                (p.kaempfe, kandidaten.saturating_mul(f), "")
            }
            (Arbeitsmenge::Bekannt { kandidaten }, Kampfzahl::Hoechstens(b)) => {
                (p.kaempfe, kandidaten.saturating_mul(b), " des Budgets")
            }
            (Arbeitsmenge::Hoechstens { .. }, _) if s.runde.is_some() => (
                p.kandidaten_gesamt().saturating_sub(s.runde_ab_kandidaten),
                s.runde_geplant,
                " der Runde",
            ),
            _ => return None,
        };
        (nenner > 0).then(|| ((zaehler as f64 / nenner as f64).clamp(0.0, 1.0), wovon))
    }

    fn kopfzeile(&self) -> String {
        let mut kopf = self.phase.clone();
        if let Some(boss) = &self.boss {
            let _ = write!(kopf, " · {boss}");
        }
        if let Some(runde) = self.probe.stand.runde {
            let _ = write!(kopf, " · Runde {runde}");
        }
        match self.anteil() {
            Some((anteil, wovon)) => {
                let voll = (anteil * BALKEN as f64).round() as usize;
                let _ = write!(
                    kopf,
                    "   {}{}  {:>3} %{wovon}",
                    "█".repeat(voll),
                    "░".repeat(BALKEN - voll),
                    (anteil * 100.0).floor() as u64
                );
            }
            None => kopf.push_str(match self.probe.stand.menge {
                Arbeitsmenge::Offen => "   ohne Balken: offene Suche, es gibt kein Ganzes",
                _ => "   ohne Balken: die Menge steht noch nicht fest",
            }),
        }
        kopf
    }

    fn kandidaten_text(&self) -> String {
        let fertig = format_ganzzahl(self.probe.kandidaten_gesamt());
        match self.probe.stand.menge {
            Arbeitsmenge::Bekannt { kandidaten } => {
                format!("{fertig} / {}", format_ganzzahl(kandidaten))
            }
            Arbeitsmenge::Hoechstens {
                mindestens,
                hoechstens,
            } => format!(
                "{fertig} / höchstens {} (mindestens {})",
                format_ganzzahl(hoechstens),
                format_ganzzahl(mindestens)
            ),
            Arbeitsmenge::Offen => format!("{fertig} (offene Menge)"),
            Arbeitsmenge::Unbekannt => format!("{fertig} (Menge noch unbekannt)"),
        }
    }

    fn durchsatz_zeile(&self) -> String {
        match &self.raten {
            Some(r) => format!(
                "  Durchsatz    {} /s   ⌀ {} /s über {} s   aufgelöste Kandidaten",
                rate_text(r.kandidaten_aktuell),
                rate_text(r.kandidaten_gleitend),
                // Unter zehn Sekunden mit einer Stelle: „über 0 s" sähe aus,
                // als gäbe es kein Fenster.
                if r.fenster_s < 10.0 {
                    dezimal(r.fenster_s, 1)
                } else {
                    format_ganzzahl(r.fenster_s.round() as u64)
                }
            ),
            None => "  Durchsatz    noch keine zwei Momentaufnahmen".to_owned(),
        }
    }

    fn wege_zeile(&self) -> String {
        let p = &self.probe;
        let anteile: Option<[f64; Weg::ANZAHL]> = match &self.raten {
            Some(r) if r.kandidaten_gleitend > 0.0 => {
                Some(r.wege_gleitend.map(|w| w / r.kandidaten_gleitend))
            }
            _ if p.kandidaten_gesamt() > 0 => {
                let gesamt = p.kandidaten_gesamt() as f64;
                Some(p.kandidaten.map(|k| k as f64 / gesamt))
            }
            _ => None,
        };
        let ohne = WEGE_OHNE_LAUF.join(", ");
        match anteile {
            Some(a) => {
                let teile: Vec<String> = Weg::ALLE
                    .iter()
                    .map(|w| format!("{} {:.0} %", w.name(), a[w.index()] * 100.0))
                    .collect();
                format!("     davon     {}   (ohne Lauf: {ohne})", teile.join(" · "))
            }
            None => {
                format!("     davon     noch keine Auswertung abgeschlossen   (ohne Lauf: {ohne})")
            }
        }
    }

    fn kernel_zeile(&self) -> String {
        let p = &self.probe;
        let anteil = if p.durchgaenge >= p.kaempfe && p.durchgaenge > 0 {
            format!(
                " — {} % der Durchgänge gerechnet",
                (p.kaempfe as f64 * 100.0 / p.durchgaenge as f64).round() as u64
            )
        } else {
            String::new()
        };
        let mengen = format!(
            "{} Kämpfe für {} Durchgänge{anteil}",
            format_ganzzahl(p.kaempfe),
            format_ganzzahl(p.durchgaenge)
        );
        match &self.raten {
            Some(r) => format!(
                "     Kernel    {} /s roh   ⌀ {} /s   {mengen}",
                rate_text(r.kaempfe_aktuell),
                rate_text(r.kaempfe_gleitend)
            ),
            None => format!("     Kernel    {mengen}"),
        }
    }

    fn restzeit_text(&self) -> String {
        match &self.restzeit {
            Restzeit::Spanne {
                mitte_s: Some(mitte),
                oben_s,
                ..
            } => format!(
                "{} ± {}   (eine Standardabweichung; Herleitung im JSON-Strom)",
                hms(*mitte),
                hms(oben_s - mitte)
            ),
            Restzeit::Spanne {
                mitte_s: None,
                unten_s,
                oben_s,
                ..
            } => format!(
                // Nicht „höchstens": Die MENGE hat eine harte Obergrenze, die
                // ZEIT nicht — sie hängt an der Rate, und die kann fallen.
                "zwischen {} und {}   (eine Standardabweichung; die Suche kann früher enden)",
                hms(*unten_s),
                hms(*oben_s)
            ),
            Restzeit::Plateau => "keine — offene Suche bis Konvergenz (siehe Bestes)".to_owned(),
            Restzeit::Unbestimmt(grund) => format!("noch keine: {grund}"),
            Restzeit::Fertig => "fertig".to_owned(),
        }
    }

    fn rechner_text(&self) -> String {
        let teile: Vec<String> = self
            .rechner
            .iter()
            .map(|r| {
                let erlaubt = r
                    .erlaubt
                    .map_or_else(String::new, |n| format!(" ({n} Kerne erlaubt)"));
                format!("{} {}/{} Stränge aktiv{erlaubt}", r.id, r.aktiv, r.straenge)
            })
            .collect();
        teile.join(" · ")
    }

    fn drossel_text(&self) -> String {
        match &self.drosselung {
            Some(d) => format!(
                "cpu_threads {} von {} · cpu_percent {} % · {} · {} Wartefälle",
                d.settings.cpu_threads,
                self.rechner.first().map_or(0, |r| r.straenge),
                d.settings.cpu_percent,
                if d.settings.paused {
                    "pausiert"
                } else {
                    "läuft"
                },
                format_ganzzahl(d.throttle_events)
            ),
            None => "keine (ohne --control-file)".to_owned(),
        }
    }

    fn bestes_text(&self) -> String {
        let p = &self.probe;
        let Some(b) = p.stand.bestes else {
            return "— (keine Suche)".to_owned();
        };
        let ohne = p.kandidaten_gesamt().saturating_sub(b.bei_kandidaten);
        let mut text = format!(
            "{} seit {} unverändert · {} {} ohne Verbesserung",
            dezimal(b.wert, 1),
            hms(sekunden(p.elapsed_ns.saturating_sub(b.seit_ns))),
            format_ganzzahl(ohne),
            if ohne == 1 {
                "Auswertung"
            } else {
                "Auswertungen"
            }
        );
        if let Some(runde) = p.stand.runde {
            let _ = write!(text, " · Runde {runde}");
        }
        text
    }

    fn durchsatz_json(&self) -> String {
        match &self.raten {
            Some(r) => format!(
                "{{\"window_s\":{},\"resolved_candidates_per_s\":{{\"current\":{},\"window\":{}}},\
                 \"full_fights_per_s\":{{\"current\":{},\"window\":{}}},\"active_threads\":{}}}",
                zahl(r.fenster_s),
                zahl(r.kandidaten_aktuell),
                zahl(r.kandidaten_gleitend),
                zahl(r.kaempfe_aktuell),
                zahl(r.kaempfe_gleitend),
                r.aktive_straenge
            ),
            None => "null".to_owned(),
        }
    }

    fn wege_json(&self) -> String {
        let mut j = String::from("{");
        for weg in Weg::ALLE {
            let rate = self
                .raten
                .as_ref()
                .map_or_else(|| "null".to_owned(), |r| zahl(r.wege_gleitend[weg.index()]));
            let _ = write!(
                j,
                "\"{}\":{{\"done\":{},\"per_s\":{rate}}},",
                weg.schluessel(),
                self.probe.kandidaten[weg.index()]
            );
        }
        let ohne: Vec<String> = WEGE_OHNE_LAUF.iter().map(|w| json_string(w)).collect();
        let _ = write!(j, "\"not_in_run\":[{}]}}", ohne.join(","));
        j
    }

    fn plateau_json(&self) -> String {
        let p = &self.probe;
        match p.stand.bestes {
            Some(b) => format!(
                "{{\"best\":{},\"best_unchanged_s\":{},\"evaluations_since_best\":{},\"round\":{}}}",
                zahl(b.wert),
                zahl(sekunden(p.elapsed_ns.saturating_sub(b.seit_ns))),
                p.kandidaten_gesamt().saturating_sub(b.bei_kandidaten),
                ganz_oder_null(p.stand.runde)
            ),
            None => "null".to_owned(),
        }
    }

    fn rechner_json(&self) -> String {
        let teile: Vec<String> = self
            .rechner
            .iter()
            .map(|r| {
                format!(
                    "{{\"id\":{},\"threads\":{},\"active\":{},\"allowed\":{}}}",
                    json_string(&r.id),
                    r.straenge,
                    r.aktiv,
                    ganz_oder_null(r.erlaubt.map(|n| n as u64))
                )
            })
            .collect();
        format!("[{}]", teile.join(","))
    }
}

fn menge_json(menge: Arbeitsmenge) -> String {
    match menge {
        Arbeitsmenge::Unbekannt => "{\"kind\":\"unknown\"}".to_owned(),
        Arbeitsmenge::Bekannt { kandidaten } => {
            format!("{{\"kind\":\"known\",\"total\":{kandidaten}}}")
        }
        Arbeitsmenge::Hoechstens {
            mindestens,
            hoechstens,
        } => format!("{{\"kind\":\"bounded\",\"min\":{mindestens},\"max\":{hoechstens}}}"),
        Arbeitsmenge::Offen => "{\"kind\":\"open\"}".to_owned(),
    }
}

fn kosten_json(kosten: Kostenbild) -> String {
    let (art, wert) = match kosten.kaempfe_je_kandidat {
        Kampfzahl::Fest(f) => ("fixed", f),
        Kampfzahl::Hoechstens(b) => ("at_most", b),
    };
    format!(
        "{{\"path\":{},\"fights_per_candidate\":{{\"kind\":\"{art}\",\"value\":{wert}}}}}",
        json_string(kosten.weg.schluessel())
    )
}

fn restzeit_json(restzeit: &Restzeit) -> String {
    match restzeit {
        Restzeit::Spanne {
            mitte_s,
            unten_s,
            oben_s,
            herleitung,
        } => format!(
            "{{\"kind\":\"estimate\",\"mid_s\":{},\"low_s\":{},\"high_s\":{},\
             \"band\":\"one_sigma\",\"derivation\":{}}}",
            mitte_s.map_or_else(|| "null".to_owned(), zahl),
            zahl(*unten_s),
            zahl(*oben_s),
            herleitung_json(herleitung)
        ),
        Restzeit::Plateau => "{\"kind\":\"none\",\"reason\":\"open_search\"}".to_owned(),
        Restzeit::Unbestimmt(grund) => {
            format!("{{\"kind\":\"unknown\",\"reason\":{}}}", json_string(grund))
        }
        Restzeit::Fertig => "{\"kind\":\"done\"}".to_owned(),
    }
}

fn herleitung_json(h: &Herleitung) -> String {
    let rest = match h.restkosten {
        Restkosten::Exakt { kaempfe } => format!("{{\"kind\":\"exact\",\"value\":{kaempfe}}}"),
        Restkosten::Spanne {
            mindestens,
            hoechstens,
        } => format!("{{\"kind\":\"range\",\"min\":{mindestens},\"max\":{hoechstens}}}"),
    };
    format!(
        "{{\"remaining_fights\":{rest},\"rate_per_thread\":{},\"rate_spread\":{},\
         \"rate_sections\":{},\"cost_trend\":{},\"sigma_cost_s\":{},\
         \"schedule_min_s\":{},\"schedule_max_s\":{},\
         \"critical_path_s\":{},\"hosts\":{},\"sigma_rate_s\":{},\
         \"retry_probability\":{},\"sigma_retry_s\":{},\"naive_candidates_s\":{},\
         \"naive_division_s\":{}}}",
        zahl(h.rate_je_strang),
        zahl(h.rate_streuung),
        h.rate_abschnitte,
        zahl(h.kosten_trend),
        zahl(h.sigma_kosten_s),
        zahl(h.ablauf_min_s),
        zahl(h.ablauf_max_s),
        zahl(h.kritischer_pfad_s),
        h.rechner,
        zahl(h.sigma_rate_s),
        zahl(h.wiederholung_p),
        zahl(h.sigma_wiederholung_s),
        h.naiv_kandidaten_s.map_or_else(|| "null".to_owned(), zahl),
        zahl(h.naiv_division_s)
    )
}

fn drossel_json(d: &ControlSnapshot) -> String {
    format!(
        "{{\"cpu_threads\":{},\"cpu_percent\":{},\"paused\":{},\"active_workers\":{},\
         \"throttle_events\":{},\"updates\":{}}}",
        d.settings.cpu_threads,
        d.settings.cpu_percent,
        d.settings.paused,
        d.active_workers,
        d.throttle_events,
        d.updates
    )
}

/// Eine Zahl als JSON — **`null` statt NaN**. Eine nicht endliche Zahl ist im
/// Strom keine Angabe; als Text geschrieben machte sie die ganze Zeile
/// unlesbar.
fn zahl(x: f64) -> String {
    json_number(x, "fortschritt").unwrap_or_else(|_| "null".to_owned())
}

fn ganz_oder_null(x: Option<u64>) -> String {
    x.map_or_else(|| "null".to_owned(), |n| n.to_string())
}

fn sekunden(ns: u64) -> f64 {
    ns as f64 / 1e9
}

/// Sekunden als `hh:mm:ss`, abgerundet. Mehr als 99 Stunden werden mehr Ziffern.
fn hms(s: f64) -> String {
    if !s.is_finite() || s < 0.0 {
        return "--:--:--".to_owned();
    }
    let ganz = s as u64;
    format!("{:02}:{:02}:{:02}", ganz / 3600, ganz / 60 % 60, ganz % 60)
}

/// Eine Rate für die Zeile: ab 100 ganzzahlig, darunter mit einer Stelle.
///
/// Eine Suche mit vollen Stichproben schafft wenige Auswertungen je Sekunde;
/// „0 /s" wäre dort keine Rate, sondern ein Rundungsfehler.
fn rate_text(x: f64) -> String {
    if !x.is_finite() {
        return "—".to_owned();
    }
    if x >= 100.0 {
        format_ganzzahl(x.round() as u64)
    } else {
        dezimal(x, 1)
    }
}

/// Eine Kommazahl mit Tausendertrenner: `14 208,0`.
fn dezimal(x: f64, stellen: usize) -> String {
    if !x.is_finite() {
        return "—".to_owned();
    }
    let text = format!("{:.stellen$}", x.abs());
    let (ganz, rest) = text.split_once('.').unwrap_or((&text, ""));
    let mut aus = if x < 0.0 {
        "-".to_owned()
    } else {
        String::new()
    };
    aus.push_str(&format_ganzzahl(ganz.parse().unwrap_or(0)));
    if !rest.is_empty() {
        aus.push(',');
        aus.push_str(rest);
    }
    aus
}

/// Eine Zeit seit der Unix-Epoche als ISO 8601 in UTC, mit Mikrosekunden.
///
/// Von Hand, weil die Bibliothek keine Abhängigkeit trägt (`sim/Cargo.toml`).
/// Die Umrechnung von Tagen in ein Datum ist das Verfahren `civil_from_days`
/// von Howard Hinnant — ganzzahlig, ohne Tabelle, gültig für den ganzen
/// gregorianischen Kalender.
#[must_use]
pub fn utc_iso8601(seit_epoche: Duration) -> String {
    let sekunden = seit_epoche.as_secs();
    let tage = (sekunden / 86_400) as i64;
    let im_tag = sekunden % 86_400;
    let z = tage + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tag = doy - (153 * mp + 2) / 5 + 1;
    let monat = if mp < 10 { mp + 3 } else { mp - 9 };
    let jahr = yoe + era * 400 + i64::from(monat <= 2);
    format!(
        "{jahr:04}-{monat:02}-{tag:02}T{:02}:{:02}:{:02}.{:06}Z",
        im_tag / 3600,
        im_tag / 60 % 60,
        im_tag % 60,
        seit_epoche.subsec_micros()
    )
}

/// Eine ganze Zahl mit Leerzeichen als Tausendertrenner: `1 084 000`.
#[must_use]
pub fn format_ganzzahl(n: u64) -> String {
    let ziffern = n.to_string();
    let mut aus = String::with_capacity(ziffern.len() + ziffern.len() / 3);
    for (i, c) in ziffern.chars().enumerate() {
        if i > 0 && (ziffern.len() - i).is_multiple_of(3) {
            aus.push(' ');
        }
        aus.push(c);
    }
    aus
}
