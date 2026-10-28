//! Was erhoben wird (AC1) — roh je Momentaufnahme, ausgewertet über das
//! Fenster.
//!
//! # Zwei Ebenen, wie beim Durchsatz
//!
//! [`Rohstand`] hält kumulierte Zähler ab Laufbeginn, so wie das Zählwerk aus
//! #933 seine Kämpfe zählt. [`Messwerte`] sind Differenzen zweier Rohstände —
//! derselben zwei Momentaufnahmen, aus denen die Anzeige ihre gleitende Rate
//! rechnet. Eine Lage, die mitten im Lauf beginnt, wird deshalb nicht vom
//! ganzen Lauf verdünnt, und die Diagnose spricht über dieselben dreißig
//! Sekunden wie die Durchsatzzeile darüber.
//!
//! # Fehlend ist nicht null
//!
//! Jede Größe, die in einem Lauf nicht erhoben werden kann, ist `None` und
//! trägt ihren Grund in der Regel, die sie braucht. Eine Netzwartezeit von 0
//! hieße „das Netz wartet nie"; es gibt aber kein Netz im Lauf.

use super::kernklasse::{Adapter, Kernklasse, Rolle};
use super::zaehler::Zaehlerstand;
use crate::fortschritt::{Momentaufnahme, Weg};

/// Arbeit gegen Warten eines Strangs in den parallelen Abschnitten, in
/// Nanosekunden.
///
/// **Was als Warten zählt.** Ein Abschnitt ist eine Auswertung, die
/// [`crate::balance`] über mehrere Stränge verteilt. Ein Strang arbeitet so
/// lange, wie sein Paket CPU-Zeit kostet, und wartet von da bis zum Ende des
/// langsamsten. Ein Strang des Laufs, der in diesem Abschnitt **kein** Paket
/// bekam, wartet den ganzen Abschnitt — das ist der Fall „zu wenige
/// Pakete". Die Zeit, bis ein Strang überhaupt gestartet ist, zählt bei
/// keinem: Sie ist Aufwand des Startens, nicht Warten auf Arbeit. Die
/// Rechnung steht in [`abschnitt`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Strangzeit {
    /// Gerechnet.
    pub arbeit_ns: u64,
    /// Fertig und auf die anderen gewartet.
    pub warten_ns: u64,
}

/// Arbeit gegen Warten je Strang für einen parallelen Abschnitt (#934).
///
/// `arbeit_ns` je Paket in der Reihenfolge der Bereiche — die CPU-Zeit
/// seines Strangs (`CLOCK_THREAD_CPUTIME_ID`), oder ohne lesbare Uhr seine
/// Dauer an der Wanduhr. `straenge` sind die Stränge des Laufs. Der
/// Abschnitt dauert so lange wie sein teuerstes Paket; jeder Strang wartet
/// den Rest, ein Strang ohne Paket den ganzen Abschnitt.
///
/// **Beides an der eigenen Arbeit gemessen, nicht an der Wanduhr** — aus
/// zwei Messungen vom 2026-10-28 auf dem Laptop:
///
/// - Verdrängt Fremdlast einen Strang, verlängert das seine Wanduhr, nicht
///   seine Arbeit. An der Wanduhr warteten bei PSI cpu 6–8 % vier Stränge mit
///   je 100 Kämpfen 22,6 bis 27,2 % ihrer Zeit.
/// - Startet ein Strang spät, weil er erst auf einen Kern wartet, endet er
///   spät, und die früh gestarteten „warten" auf ihn. Mit der Wanduhr als
///   Beginn blieben bei PSI cpu 16–22 % noch 15,6 und 18,0 %.
///
/// Beides ist kein zu großes und kein fehlendes Paket, und kein kleineres
/// Paket verkürzte es. Den belegten Kern meldet PSI.
#[must_use]
pub fn abschnitt(arbeit_ns: &[u64], straenge: usize) -> Vec<Strangzeit> {
    let ende = arbeit_ns.iter().copied().max().unwrap_or(0);
    (0..straenge.max(arbeit_ns.len()))
        .map(|nummer| {
            let arbeit = arbeit_ns.get(nummer).copied().unwrap_or(0);
            Strangzeit {
                arbeit_ns: arbeit,
                warten_ns: ende - arbeit,
            }
        })
        .collect()
}

/// Die Zähler aus [`crate::diagnostics`], so weit sie die Regeltabelle
/// braucht — ohne das Histogramm, damit das Veröffentlichen je Auswertung
/// keinen Speicher anfordert.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kampfzaehlung {
    /// Wie oft die Schlangenlänge abgelesen wurde, einmal je Zeitpunkt.
    pub schritte: u64,
    /// Größte beobachtete Länge der Ereignisschlange.
    pub schlange_max: usize,
    /// Wie oft die Schlange in den Rückfall ausweichen musste (#909).
    pub schlange_ueberlaeufe: u64,
    /// Aufrufe von `expire_buffs`.
    pub ablauf_aufrufe: u64,
    /// Davon mit mindestens einer abgelaufenen Verstärkung.
    pub ablauf_treffer: u64,
}

/// Druckwerte aus `/proc/pressure`, `some avg10` in Prozent.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Druck {
    /// `cpu`.
    pub cpu: Option<f64>,
    /// `memory`.
    pub speicher: Option<f64>,
    /// `io`.
    pub io: Option<f64>,
}

impl Druck {
    /// Liest die drei Werte dieses Rechners — dieselbe Funktion, mit der
    /// [`crate::hostcaps`] die Umgebung prüft.
    #[must_use]
    pub fn erheben() -> Self {
        use crate::hostcaps::psi_some_avg10;
        Self {
            cpu: psi_some_avg10("/proc/pressure/cpu"),
            speicher: psi_some_avg10("/proc/pressure/memory"),
            io: psi_some_avg10("/proc/pressure/io"),
        }
    }

    /// Der höchste Wert je Quelle über mehrere Ablesungen — `None` je
    /// Quelle, wenn keine Ablesung sie hatte.
    ///
    /// **Der Höchststand, nicht der letzte Wert.** Am 2026-10-28 bestand eine
    /// Grundlage auf dem Laptop den Riegel mit PSI cpu 2,36 % am Ende des
    /// Laufs — während `avg60` dort noch bei 10 und `avg300` bei 22 stand. Ihr
    /// Befehlsdurchsatz lag bei 2,30 statt 2,91 in ruhiger Lage.
    #[must_use]
    pub fn hoechst(ablesungen: impl IntoIterator<Item = Self>) -> Self {
        let max = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        ablesungen.into_iter().fold(Self::default(), |h, d| Self {
            cpu: max(h.cpu, d.cpu),
            speicher: max(h.speicher, d.speicher),
            io: max(h.io, d.io),
        })
    }

    /// Warum eine Zeitmessung unter diesem Druck nichts über den Lauf sagt —
    /// `None`, wenn sie es tut.
    ///
    /// Dieselben Grenzen, unter denen das Projekt überhaupt misst
    /// ([`crate::hostcaps::PSI_CPU_MAX`] für Rechenzeit,
    /// [`crate::hostcaps::PSI_IO_MEM_MAX`] für Speicher). Ohne lesbaren
    /// CPU-Druck ist eine Verlangsamung von Fremdlast nicht zu trennen.
    ///
    /// **PSI zählt auch die eigenen Stränge.** Am 2026-10-28 auf node3, ohne
    /// ein fremdes Programm im Container: vier Arbeitsstränge auf vier
    /// erlaubten Kernen, tausende Auswertungen je Sekunde — Leit- und
    /// Anzeigestrang warteten auf einen Kern, und PSI cpu stieg von 1,43 auf
    /// 12,15 %. Der Satz nennt deshalb beide Ursachen.
    ///
    /// Was keine Ablesung zeigt: einen gedrosselten Takt. Unter Volllast lief
    /// der Laptop am 2026-10-28 mit dreieinhalbfacher CPU-Zeit je Kampf bei nur
    /// +27 % Zyklen. Zähler sehen das nicht, eine Uhr schon.
    #[must_use]
    pub fn stoert_zeitmessung(&self) -> Option<String> {
        use crate::hostcaps::{PSI_CPU_MAX, PSI_IO_MEM_MAX};
        let zahl = |x: f64| format!("{x:.2}").replace('.', ",");
        match (self.cpu, self.speicher) {
            (None, _) => Some(
                "kein Druckwert lesbar — ohne ihn ist eine Zeitmessung unter Fremdlast nicht \
                 von einer Verlangsamung zu trennen"
                    .to_owned(),
            ),
            (Some(cpu), _) if cpu > PSI_CPU_MAX => Some(format!(
                "Rechendruck: PSI cpu some avg10 bis {} % im Fenster, über {} % — Fremdlast \
                 oder eigene Stränge ohne freien Kern; die Zeitmessung ist nicht aussagekräftig",
                zahl(cpu),
                zahl(PSI_CPU_MAX)
            )),
            (_, Some(speicher)) if speicher > PSI_IO_MEM_MAX => Some(format!(
                "Speicherdruck: PSI memory some avg10 bis {} % im Fenster, über {} % — die \
                 Zeitmessung ist nicht aussagekräftig",
                zahl(speicher),
                zahl(PSI_IO_MEM_MAX)
            )),
            _ => None,
        }
    }
}

/// Was eine Momentaufnahme für die Engpasserkennung festhält, kumuliert ab
/// Laufbeginn.
///
/// Die ersten drei Felder füllt das Zählwerk, die letzten drei der
/// Anzeigestrang im selben Takt ([`super::Erhebung::ergaenzen`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rohstand {
    /// Je Arbeitsplatz des Zählwerks (ohne den Leitstrang).
    pub straenge: Vec<Strangzeit>,
    /// Wie oft eine Auswertung geprüft hat, ob **ein** Kampf genügt (#908).
    pub reduktion_versuche: u64,
    /// Die zuletzt veröffentlichten Kampfzähler — `None`, solange keine
    /// Auswertung mit eingeschalteter Erfassung fertig wurde.
    pub kampf: Option<Kampfzaehlung>,
    /// Die Leistungszähler — `None` ohne Zähler.
    pub leistung: Option<Zaehlerstand>,
    /// CPU-Zeit des Prozesses.
    pub cpu_ns: Option<u64>,
    /// Druckwerte im selben Augenblick.
    pub druck: Druck,
}

/// Was die Leistungszähler über das Fenster ergeben.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zaehlerwerte {
    /// Nach welchem Adapter.
    pub kernklasse: Kernklasse,
    /// Befehle je Zyklus.
    pub ipc: Option<f64>,
    /// Zyklen je Kampf, aus der ersten Gruppe.
    pub zyklen_je_kampf: Option<f64>,
    /// Fehlsprünge je Kampf.
    pub fehlspruenge_je_kampf: Option<f64>,
    /// Fehlgriffe der ersten Ebene je Kampf.
    pub l1_je_kampf: Option<f64>,
    /// Fehlgriffe der zweiten Ebene je Kampf.
    pub l2_je_kampf: Option<f64>,
    /// Fehlgriffe der dritten Ebene je Kampf.
    pub l3_je_kampf: Option<f64>,
    /// Zyklen, die auf Speicher warten, je Kampf.
    pub speicherwartezyklen_je_kampf: Option<f64>,
    /// Derselbe Wert als Anteil an den Zyklen derselben Gruppe.
    pub speicher_wartezeit_anteil: Option<f64>,
    /// Je Gruppe: welcher Anteil der eingeschalteten Zeit wirklich gezählt
    /// wurde. Unter 1, wenn der Kernel die Gruppen abwechselnd plant; die
    /// Werte oben sind dann hochgerechnet, wie `perf stat` es tut.
    pub abdeckung: [Option<f64>; 2],
}

/// Ob es Zählerwerte gibt.
#[derive(Debug, Clone, PartialEq)]
pub enum Zaehlerlage {
    /// Gezählt.
    Gemessen(Zaehlerwerte),
    /// Nicht gezählt, mit dem Grund — gesperrt, kein Adapter, oder noch kein
    /// Stand im Fenster.
    Fehlt(String),
}

/// Die Trefferquote eines Auswertungswegs.
#[derive(Debug, Clone, PartialEq)]
pub struct Wegtreffer {
    /// Welcher Weg.
    pub weg: &'static str,
    /// Wie oft er versucht wurde.
    pub versuche: u64,
    /// Wie oft er die Auswertung übernommen hat.
    pub treffer: u64,
    /// Die Quote, unter der er sich nicht lohnt — die **eigene** Schwelle des
    /// Wegs. `None`, wenn er keine hat.
    pub schwelle: Option<f64>,
    /// Warum er keine hat, wenn er keine hat.
    pub ohne_schwelle: &'static str,
}

impl Wegtreffer {
    /// Treffer durch Versuche — `None` ohne Versuch.
    #[must_use]
    pub fn quote(&self) -> Option<f64> {
        (self.versuche > 0).then(|| self.treffer as f64 / self.versuche as f64)
    }
}

/// Netzwartezeit und verworfene Pakete einer Verteilung über mehrere Rechner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Netzwerte {
    /// Pakete, die über das Netz gingen.
    pub pakete: u64,
    /// Wie lange auf das Netz gewartet wurde, summiert über alle Pakete.
    pub wartezeit_ns: u64,
    /// Wie lange die Pakete insgesamt unterwegs und in Arbeit waren.
    pub laufzeit_ns: u64,
    /// Pakete, die verworfen und neu vergeben wurden.
    pub verworfen: u64,
}

/// Die Belegung der Ereignisschlange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schlangenbelegung {
    /// Der höchste Stand, ab Laufbeginn — eine Kapazität läuft am
    /// Höchststand über, nicht am Mittel.
    pub hoechststand: usize,
    /// Die feste Kapazität.
    pub kapazitaet: usize,
    /// Ausweichen in den Rückfall (#909) im Fenster.
    pub ueberlaeufe: u64,
    /// Ablesungen im Fenster — die Zusicherung, dass die Zahl gemessen ist.
    pub schritte: u64,
}

/// Die Trefferquote der Ablaufprüfung `expire_buffs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ablauftreffer {
    /// Aufrufe im Fenster.
    pub aufrufe: u64,
    /// Davon mit Treffer.
    pub treffer: u64,
}

impl Ablauftreffer {
    /// Treffer durch Aufrufe — `None` ohne Aufruf.
    #[must_use]
    pub fn quote(&self) -> Option<f64> {
        (self.aufrufe > 0).then(|| self.treffer as f64 / self.aufrufe as f64)
    }
}

/// Alles aus AC1, über ein Fenster.
#[derive(Debug, Clone, PartialEq)]
pub struct Messwerte {
    /// Wie lang das Fenster ist.
    pub sekunden: f64,
    /// Gerechnete Kämpfe im Fenster — der Nenner aller Werte „je Kampf".
    pub kaempfe: u64,
    /// Befehlsdurchsatz, Fehlsprünge, drei Ebenen, Speicher-Wartezeit.
    pub zaehler: Zaehlerlage,
    /// CPU-Zeit des Prozesses je Kampf — die Zeitmessung des Rückfalls.
    pub cpu_ns_je_kampf: Option<f64>,
    /// Arbeit gegen Warten je Strang.
    pub straenge: Vec<Strangzeit>,
    /// Trefferquote je Auswertungsweg.
    pub wege: Vec<Wegtreffer>,
    /// Netzwartezeit je Paket und verworfene Pakete.
    pub netz: Option<Netzwerte>,
    /// Belegung der Ereignisschlange.
    pub schlange: Option<Schlangenbelegung>,
    /// Trefferquote der Ablaufprüfung.
    pub ablauf: Option<Ablauftreffer>,
    /// Der höchste Druck je Quelle über alle Proben des Fensters
    /// ([`Druck::hoechst`]); [`Messwerte::aus_fenster`] allein kennt nur die
    /// jüngste Probe und setzt deren Werte.
    pub druck: Druck,
}

impl Messwerte {
    /// Der Anteil der Strangzeit, der Warten war — `None` ohne parallelen
    /// Abschnitt im Fenster.
    #[must_use]
    pub fn warteanteil(&self) -> Option<f64> {
        let arbeit: u64 = self.straenge.iter().map(|s| s.arbeit_ns).sum();
        let warten: u64 = self.straenge.iter().map(|s| s.warten_ns).sum();
        let gesamt = arbeit + warten;
        (gesamt > 0).then(|| warten as f64 / gesamt as f64)
    }

    /// Die Messwerte zwischen zwei Momentaufnahmen.
    ///
    /// `adapter` ist der Plan, nach dem gezählt wurde — oder der Grund, warum
    /// nicht.
    #[must_use]
    pub fn aus_fenster(
        alt: &Momentaufnahme,
        neu: &Momentaufnahme,
        adapter: Result<&Adapter, &str>,
    ) -> Self {
        let kaempfe = neu.kaempfe.saturating_sub(alt.kaempfe);
        let je_kampf = |wert: f64| (kaempfe > 0).then(|| wert / kaempfe as f64);
        let (a, n) = (&alt.engpass, &neu.engpass);

        let zaehler = match (adapter, &a.leistung, &n.leistung) {
            (Err(grund), _, _) => Zaehlerlage::Fehlt(grund.to_owned()),
            (Ok(adapter), Some(a), Some(n)) => {
                Zaehlerlage::Gemessen(zaehlerwerte(adapter, a, n, kaempfe))
            }
            (Ok(_), _, _) => Zaehlerlage::Fehlt("noch kein Zählerstand im Fenster".to_owned()),
        };

        let cpu_ns_je_kampf = match (a.cpu_ns, n.cpu_ns) {
            (Some(a), Some(n)) => je_kampf(n.saturating_sub(a) as f64),
            _ => None,
        };

        let straenge = n
            .straenge
            .iter()
            .enumerate()
            .map(|(i, neu)| {
                let vorher = a.straenge.get(i).copied().unwrap_or_default();
                Strangzeit {
                    arbeit_ns: neu.arbeit_ns.saturating_sub(vorher.arbeit_ns),
                    warten_ns: neu.warten_ns.saturating_sub(vorher.warten_ns),
                }
            })
            .collect();

        let ein_kampf = Weg::EinKampf.index();
        let wege = vec![Wegtreffer {
            weg: "ein Kampf (Reduktion, #908)",
            versuche: n.reduktion_versuche.saturating_sub(a.reduktion_versuche),
            treffer: neu.kandidaten[ein_kampf].saturating_sub(alt.kandidaten[ein_kampf]),
            schwelle: None,
            ohne_schwelle: "die Prüfung kostet keinen Kampf — es gibt keine Quote, unter der \
                            sie sich nicht lohnt",
        }];

        let (schlange, ablauf) = match (n.kampf, crate::diagnostics::im_bau()) {
            (Some(nk), true) => {
                let ak = a.kampf.unwrap_or_default();
                let schritte = nk.schritte.saturating_sub(ak.schritte);
                let aufrufe = nk.ablauf_aufrufe.saturating_sub(ak.ablauf_aufrufe);
                (
                    (schritte > 0).then_some(Schlangenbelegung {
                        hoechststand: nk.schlange_max,
                        kapazitaet: crate::diagnostics::QUEUE_BUCKETS - 1,
                        ueberlaeufe: nk
                            .schlange_ueberlaeufe
                            .saturating_sub(ak.schlange_ueberlaeufe),
                        schritte,
                    }),
                    (aufrufe > 0).then_some(Ablauftreffer {
                        aufrufe,
                        treffer: nk.ablauf_treffer.saturating_sub(ak.ablauf_treffer),
                    }),
                )
            }
            _ => (None, None),
        };

        Self {
            sekunden: neu.elapsed_ns.saturating_sub(alt.elapsed_ns) as f64 / 1e9,
            kaempfe,
            zaehler,
            cpu_ns_je_kampf,
            straenge,
            wege,
            // Es gibt heute keine Verteilung über das Netz (E15.5). Eine Zahl
            // hier wäre erfunden; die Regel nennt den Grund.
            netz: None,
            schlange,
            ablauf,
            druck: n.druck,
        }
    }
}

/// Die Zählerwerte zwischen zwei Ständen.
///
/// Je Gruppe wird hochgerechnet wie bei `perf stat`: gezählter Wert mal
/// eingeschaltete durch gezählte Zeit. Eine Gruppe, die im Fenster nie zum
/// Zug kam, liefert keine Werte statt einer Null.
fn zaehlerwerte(
    adapter: &Adapter,
    alt: &Zaehlerstand,
    neu: &Zaehlerstand,
    kaempfe: u64,
) -> Zaehlerwerte {
    let mut abdeckung = [None; 2];
    let mut summen: [Option<Vec<f64>>; 2] = [None, None];
    for g in 0..2 {
        let (Some(a), Some(n)) = (alt.gruppen.get(g), neu.gruppen.get(g)) else {
            continue;
        };
        let an = n.eingeschaltet_ns.saturating_sub(a.eingeschaltet_ns);
        let gezaehlt = n.gezaehlt_ns.saturating_sub(a.gezaehlt_ns);
        if an == 0 || gezaehlt == 0 {
            abdeckung[g] = Some(0.0);
            continue;
        }
        let faktor = an as f64 / gezaehlt as f64;
        abdeckung[g] = Some(gezaehlt as f64 / an as f64);
        summen[g] = Some(
            n.werte
                .iter()
                .zip(&a.werte)
                .map(|(n, a)| n.saturating_sub(*a) as f64 * faktor)
                .collect(),
        );
    }
    let wert = |rolle: Rolle| -> Option<f64> {
        let (g, p) = adapter.platz(rolle)?;
        summen[g].as_ref()?.get(p).copied()
    };
    // Die Zyklen der zweiten Gruppe — für den Anteil der Speicher-Wartezeit,
    // der nur durch Zyklen DERSELBEN Gruppe geteilt werden darf.
    let zyklen_zweite = summen[1].as_ref().and_then(|w| w.first().copied());
    let je_kampf = |x: Option<f64>| x.filter(|_| kaempfe > 0).map(|x| x / kaempfe as f64);
    let zyklen = wert(Rolle::Zyklen);
    let speicher = wert(Rolle::Speicherwartezeit);
    Zaehlerwerte {
        kernklasse: adapter.kernklasse,
        ipc: match (wert(Rolle::Befehle), zyklen) {
            (Some(b), Some(z)) if z > 0.0 => Some(b / z),
            _ => None,
        },
        zyklen_je_kampf: je_kampf(zyklen),
        fehlspruenge_je_kampf: je_kampf(wert(Rolle::Fehlspruenge)),
        l1_je_kampf: je_kampf(wert(Rolle::FehlgriffeL1)),
        l2_je_kampf: je_kampf(wert(Rolle::FehlgriffeL2)),
        l3_je_kampf: je_kampf(wert(Rolle::FehlgriffeL3)),
        speicherwartezyklen_je_kampf: je_kampf(speicher),
        speicher_wartezeit_anteil: match (speicher, zyklen_zweite) {
            (Some(s), Some(z)) if z > 0.0 => Some(s / z),
            _ => None,
        },
        abdeckung,
    }
}
