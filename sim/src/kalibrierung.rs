//! Was dieser Rechner je Arbeitsart leistet — gemessen (Issue #912, E15.0).
//!
//! # Warum eine Zahl je Rechner nicht genügt
//!
//! Die drei vorhandenen Rechner sind für verschiedene Arbeitsarten verschieden
//! geeignet. `gonet` ist bei verzweigtem skalarem Code brauchbar (121 689
//! Kämpfe je Sekunde gemessen), bei vektorisiertem **gar nicht** — er hat AVX2
//! nicht. Der i5-1235U dürfte in einem arithmetischen Stapel relativ stärker
//! sein als im verzweigten Vollkampf, weil dort die E-Kerne mithalten.
//!
//! **Ein einziger Durchsatzwert mittelt das weg und verteilt dann falsch.**
//! Deshalb trägt [`HostPerfVector`] eine Rate je [`Arbeitsart`].
//!
//! # Warum die Dauer adaptiv ist und nicht fest
//!
//! Feste zwei Sekunden wären genau der Fehler, der in diesem Projekt schon
//! fünfmal aufgetreten ist: Die erste Durchsatzmessung streute um 12 Prozent,
//! weil die Läufe zu kurz waren; der erste Wert lag um 30 Prozent daneben.
//!
//! **Eine Kalibrierung, die selbst zu kurz misst, verteilt die Arbeit dauerhaft
//! falsch** — alle folgenden Entscheidungen stehen auf ihr. Der Lauf hört
//! deshalb erst auf, wenn er stabil ist, und nicht, wenn eine Uhr abläuft:
//!
//! ```text
//! Aufwärmen -> messen -> weiter, bis ALLE erfüllt sind:
//!     Mindestdauer 5 s          ·  Höchstdauer 30 s
//!     Unsicherheit des Mittels unter 3 %  ·  keine neue Drosselung
//!     Speicherdruck akzeptabel
//! ```
//!
//! „Streuung" aus E15.0 ist seit dem 28.10. die Unsicherheit des MITTELS,
//! nicht die der einzelnen Abschnitte (Begründung bei [`STREUUNG_MAX`]).
//!
//! Die Höchstdauer ist **keine** Messdauer, sondern eine Notbremse: Wird sie
//! erreicht, ohne dass die Unsicherheit fällt, ist das Ergebnis
//! [`Abbruchgrund::Hoechstdauer`] — und der Aufrufer weiß, dass die Zahl nicht
//! zusichert, was eine stabile Messung zusichern würde.
//!
//! # Was hier NICHT zugesichert wird
//!
//! **Keine Rate.** Zugesichert wird, dass die gemessene Rate die tatsächliche
//! Arbeit trifft — nachprüfbar an der Abweichung.
//!
//! **Kein dauerhaft gültiges Profil.** Es wird bestätigt und bei Abweichung
//! über [`NEUERHEBUNG_AB`] neu erhoben.

use std::time::{Duration, Instant};

use crate::hostcaps::{
    drucksignale, erlaubte_kerne, jetzt, psi_some_avg10, takt_mhz, taktdrosselung, HostCaps,
    Timestamp,
};

/// Wie lange vor der Messung warmgelaufen wird, ohne dass es zählt.
///
/// Der erste Durchsatzwert dieses Projekts lag um 30 Prozent daneben, weil in
/// 5000 Kämpfen je Strang (rund 0,1 Sekunden) der Aufwärmvorgang dominiert.
pub const AUFWAERMEN: Duration = Duration::from_millis(500);

/// Kürzeste Messdauer. Darunter wird nicht abgebrochen, auch wenn die Streuung
/// schon klein aussieht — bei wenigen Abschnitten ist sie es zufällig.
pub const MINDESTDAUER: Duration = Duration::from_secs(5);

/// Längste Messdauer. **Eine Notbremse, keine Messdauer.**
pub const HOECHSTDAUER: Duration = Duration::from_secs(30);

/// Wie unsicher das MITTEL der Abschnitte höchstens sein darf, als Anteil:
/// Standardfehler durch Mittelwert (siehe [`relative_unsicherheit`]).
///
/// # Nicht die Streuung der Abschnitte — gemessen, warum
///
/// Bis zum 28.10. hing hier die Streuung der einzelnen Abschnitte. Auf node3,
/// dem geteilten Zielrechner, streuen sie beim Vollkampf um 8,8 bis 13,0 %;
/// fünf von fünf Läufen endeten an der Höchstdauer, obwohl das Mittel aus rund
/// 500 Abschnitten auf ein halbes Prozent bekannt war. Längere Abschnitte
/// halfen nicht: Dieselben Rohdaten, zu vier Sekunden zusammengefasst,
/// streuten noch um 6,2 %. Die Abschnitte messen die Unruhe der Maschine, das
/// Mittel misst die Rate.
///
/// Entscheidung des Orchestrators vom selben Tag: „Streuung unter 3 bis 5 %"
/// aus E15.0 ist die Unsicherheit des Mittels. Die Schwelle ist dieselbe
/// geblieben.
pub const STREUUNG_MAX: f64 = 0.03;

/// Wie viele Abschnitte mindestens vorliegen müssen, bevor die Streuung zählt.
///
/// **Zwei Abschnitte haben immer eine kleine Streuung**, wenn sie zufällig
/// ähnlich ausfallen — und eine große, wenn nicht. Beides sagt nichts. Ab vier
/// trägt die Zahl.
pub const ABSCHNITTE_MIN: usize = 4;

/// Wie stark der Takt zwischen der ersten und der zweiten Hälfte der Messung
/// schwanken darf, als Anteil.
///
/// Auf dem Laptop folgt der Takt der Last mit Verzögerung (Governor
/// `powersave`). Ein Lauf, der bei 2386 MHz anfängt und bei 4400 endet, misst
/// zwei verschiedene Maschinen.
///
/// # Zwei Hälften, nicht zwei Augenblicke
///
/// **Der Momentantakt ist keine brauchbare Größe.** Gemessen am 2026-08-22 auf
/// proxmox, im LEERLAUF, fünf Ablesungen im Sekundenabstand:
///
/// ```text
///   1863 · 1021 · 2230 · 1350 · 1106 MHz
/// ```
///
/// Mehr als hundert Prozent Schwankung, ohne dass etwas geschah. Ein Vergleich
/// „jetzt gegen vorhin" bricht dort jede Messung ab — in einem ersten Anlauf
/// meldeten alle sechs Raten `TaktDrift`, fünf davon nach genau einem
/// Abschnitt.
///
/// Verglichen wird deshalb das **Mittel der ersten Hälfte** gegen das der
/// zweiten. Das glättet die Regelschwankung und lässt einen Trend stehen —
/// und ein Trend ist es, was diese Prüfung meint.
///
/// **Die Schwelle ist unverändert geblieben.** Geändert hat sich die Größe,
/// die gegen sie gehalten wird; eine gelockerte Schwelle hätte den Fehler
/// zugedeckt statt ihn zu beheben.
pub const TAKT_DRIFT_MAX: f64 = 0.10;

/// Ab welcher Abweichung von der erwarteten Rate neu erhoben wird.
pub const NEUERHEBUNG_AB: f64 = 0.25;

/// Wie lange höchstens gewartet wird, bis der eigene Druck-Nachhall abgeklungen
/// ist.
///
/// `some avg10` mittelt über zehn Sekunden; nach einem Volllastlauf steht der
/// Wert noch hoch, obwohl die Maschine frei ist. Fünfzehn Sekunden geben dem
/// Mittel Zeit zu fallen — bleibt er danach oben, ist es keine eigene Spur mehr.
pub const ABKLINGEN_MAX: Duration = Duration::from_secs(15);

/// Wie lange ein einzelner Abschnitt mindestens dauern muss.
///
/// **Gemessen am 2026-08-22 auf gonet:** Der Ausrüstungsdurchlauf mit 400
/// Fällen brauchte weniger als eine Millisekunde, und die Rate stand daraufhin
/// bei 31 882 671 Varianten je Sekunde — eine Zahl, die vollständig aus der
/// Auflösung der Uhr stammt.
///
/// Zehn Millisekunden lassen den Messfehler der Uhr unter ein Prozent fallen.
/// Wer ein kleineres Paket wählt, misst nicht schneller, sondern ungenauer.
pub const ABSCHNITT_MIN: Duration = Duration::from_millis(10);

/// Warum der adaptive Lauf aufgehört hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abbruchgrund {
    /// Alle Bedingungen erfüllt — das ist der gute Ausgang.
    Stabil,
    /// Die Höchstdauer war erreicht, die Streuung aber noch zu groß.
    Hoechstdauer,
    /// Der Rechner stand unter fremdem Druck; die Zahl gilt nicht.
    Druck,
    /// Der Takt ist während der Messung weggelaufen.
    TaktDrift,
    /// Die CPU hat während der Messung wegen Hitze gedrosselt.
    Drosselung,
    /// Die Arbeitspakete waren zu klein, um die Zeit sinnvoll zu messen.
    PaketZuKlein,
    /// Ein Lauf mit **fester** Dauer: der kurze Bestätigungslauf oder die
    /// Kontrollprobe. Er misst, aber er kalibriert nicht.
    FesteDauer,
}

impl Abbruchgrund {
    /// Ob die Zahl als Kalibrierung taugt.
    ///
    /// **Nur [`Abbruchgrund::Stabil`].** Eine Rate, die unter Druck oder bei
    /// wegdriftendem Takt entstand, ist eine Zahl über diesen Augenblick und
    /// keine über die Maschine.
    #[must_use]
    pub const fn brauchbar(self) -> bool {
        matches!(self, Self::Stabil)
    }
}

/// Eine gemessene Rate mit allem, was zu ihrer Beurteilung nötig ist.
#[derive(Debug, Clone, Copy)]
pub struct RateModel {
    /// Einheiten je Sekunde.
    pub rate_per_s: f64,
    /// Relative Streuung über die Abschnitte (0,03 = drei Prozent).
    pub spread: f64,
    /// Wie viele Abschnitte gemessen wurden.
    pub samples: usize,
    /// Wie lange insgesamt gemessen wurde, in Millisekunden.
    pub duration_ms: u64,
    /// Warum aufgehört wurde.
    pub reason: Abbruchgrund,
    /// Wann.
    pub measured_at: Timestamp,
    /// `PSI cpu some avg10` am Ende der Messung, in Prozent — oder `None`.
    ///
    /// **Berichtet, nicht entschieden.** Der Wert enthält den Druck, den die
    /// Messung SELBST erzeugt: Wer zwölf Stränge auslastet, lässt zwangsläufig
    /// etwas warten. Ein Abbruch darauf beendete jede erfolgreiche Messung.
    pub psi_cpu: Option<f64>,
    /// Taktdrift zwischen erster und zweiter Hälfte, als Anteil — oder `None`.
    ///
    /// Ebenfalls berichtet und nicht entschieden; die Begründung steht bei
    /// [`TAKT_DRIFT_MAX`].
    pub takt_drift: Option<f64>,
}

impl RateModel {
    /// Ob diese Rate von der erwarteten so weit abweicht, dass neu erhoben
    /// werden muss.
    ///
    /// **Der Vergleich ist relativ zur ERWARTETEN Rate**, nicht zur gemessenen:
    /// Sonst hinge die Schwelle daran, welche der beiden Zahlen im Nenner
    /// steht, und 25 Prozent nach oben wären etwas anderes als 25 nach unten.
    #[must_use]
    pub fn weicht_ab(&self, erwartet: f64) -> bool {
        if !erwartet.is_finite()
            || erwartet <= 0.0
            || !self.rate_per_s.is_finite()
            || self.rate_per_s <= 0.0
        {
            return true;
        }
        (self.rate_per_s - erwartet).abs() / erwartet > NEUERHEBUNG_AB
    }

    /// Relative Unsicherheit des Mittelwerts — die Größe, gegen die
    /// [`STREUUNG_MAX`] gehalten wird.
    ///
    /// Aus `spread` und `samples` abgeleitet statt gespeichert: Beide stehen
    /// schon im Profil, und eine dritte Zahl, die aus zweien folgt, könnte
    /// ihnen widersprechen.
    #[must_use]
    pub fn unsicherheit(&self) -> f64 {
        unsicherheit_aus(self.spread, self.samples)
    }
}

/// Welche Art von Arbeit gemessen wird.
///
/// Die Namen folgen dem Vertrag aus #912 und sind **nicht frei gewählt** — ein
/// Worker, der eigene wählt, erzeugt Felder, die kein anderes Bauteil findet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arbeitsart {
    /// Vollständig gerechnete Kämpfe, skalarer Pfad.
    ScalarFullFight,
    /// Vollständig gerechnete Kämpfe, vektorisierter Pfad.
    AvxFullFight,
    /// Bereichsrechnung auf dem Durchgangs-Register.
    LedgerEval,
    /// Exakte Krit-Verteilung, Dynamikprogrammierung.
    CritDp,
    /// Die Suche über Kandidatenzeilen.
    BranchAndBound,
    /// Der Ausrüstungsdurchlauf.
    GearSweep,
}

impl Arbeitsart {
    /// Der Name, unter dem die Rate gespeichert wird.
    ///
    /// **Eigene Methode statt `Debug`**, weil `Debug` eine Darstellung für
    /// Menschen ist und sich ändern darf. Ein Profil, dessen Schlüssel an
    /// `Debug` hängt, wird beim nächsten `derive` unlesbar.
    #[must_use]
    pub const fn schluessel(self) -> &'static str {
        match self {
            Self::ScalarFullFight => "scalar_full_fight",
            Self::AvxFullFight => "avx_full_fight",
            Self::LedgerEval => "ledger_eval",
            Self::CritDp => "crit_dp",
            Self::BranchAndBound => "branch_and_bound",
            Self::GearSweep => "gear_sweep",
        }
    }

    /// Was je Einheit gezählt wird — für den Bericht.
    #[must_use]
    pub const fn einheit(self) -> &'static str {
        match self {
            Self::ScalarFullFight | Self::AvxFullFight => "Kaempfe/s",
            Self::LedgerEval => "Bereichspruefungen/s",
            Self::CritDp => "DP-Zustaende/s",
            Self::BranchAndBound => "Kandidatenzeilen/s",
            Self::GearSweep => "Varianten/s",
        }
    }

    /// Alle Arten, in fester Reihenfolge.
    #[must_use]
    pub const fn alle() -> [Self; 6] {
        [
            Self::ScalarFullFight,
            Self::AvxFullFight,
            Self::LedgerEval,
            Self::CritDp,
            Self::BranchAndBound,
            Self::GearSweep,
        ]
    }
}

/// Alle Raten eines Rechners.
///
/// Die Feldnamen sind der Vertrag aus #912.
#[derive(Debug, Clone)]
pub struct HostPerfVector {
    /// Vollständig gerechnete Kämpfe, skalarer Pfad. Der Bezugswert.
    pub scalar_full_fight: RateModel,
    /// `None`, wenn die Maschine keinen vektorisierten Pfad anbietet — oder
    /// wenn es ihn im Simulator nicht gibt.
    ///
    /// **Beide Gründe führen auf `None`, und das ist eine Schwäche**, die hier
    /// benannt gehört: Heute ist es immer der zweite. Der Simulator hat keinen
    /// vektorisierten Vollkampf; `IsaSet::vektorisierbar` sagt nur, ob einer
    /// gebaut werden KÖNNTE. Wer die Unterscheidung braucht, liest
    /// [`HostCaps::isa`] daneben.
    pub avx_full_fight: Option<RateModel>,
    /// Bereichsrechnung auf dem Durchgangs-Register.
    pub ledger_eval: RateModel,
    /// Exakte Krit-Verteilung, Dynamikprogrammierung.
    pub crit_dp: RateModel,
    /// Die Suche über Kandidatenzeilen.
    pub branch_and_bound: RateModel,
    /// Der Ausrüstungsdurchlauf.
    pub gear_sweep: RateModel,
    /// `None`, solange es keinen Grafikeinheit-Auswerter gibt (#912, out of
    /// scope).
    pub gpu_bulk: Option<RateModel>,
    /// Umlaufzeit zu den Zusatzrechnern in Millisekunden, oder `None`.
    pub network_rtt: Option<f64>,
    /// Bandbreite zu den Zusatzrechnern in Bytes je Sekunde, oder `None`.
    pub network_bandwidth: Option<f64>,
    /// Der höchste PSI-Wert während der Erhebung, als Anteil.
    pub current_pressure: f64,
}

impl HostPerfVector {
    /// Die Rate einer Art, falls sie gemessen wurde.
    #[must_use]
    pub const fn rate(&self, art: Arbeitsart) -> Option<&RateModel> {
        match art {
            Arbeitsart::ScalarFullFight => Some(&self.scalar_full_fight),
            Arbeitsart::AvxFullFight => self.avx_full_fight.as_ref(),
            Arbeitsart::LedgerEval => Some(&self.ledger_eval),
            Arbeitsart::CritDp => Some(&self.crit_dp),
            Arbeitsart::BranchAndBound => Some(&self.branch_and_bound),
            Arbeitsart::GearSweep => Some(&self.gear_sweep),
        }
    }
}

/// Ein Messabschnitt: gefahrene Einheiten, gebrauchte Zeit und der Takt dabei.
#[derive(Debug, Clone, Copy)]
struct Abschnitt {
    einheiten: f64,
    dauer: Duration,
    /// Mittlerer Takt der erlaubten Kerne am Ende dieses Abschnitts, in MHz.
    takt_mhz: Option<f64>,
}

impl Abschnitt {
    fn rate(self) -> f64 {
        let s = self.dauer.as_secs_f64();
        if s <= 0.0 {
            0.0
        } else {
            self.einheiten / s
        }
    }
}

/// Wie stark der Takt zwischen der ersten und der zweiten Hälfte gedriftet ist.
///
/// `None`, wenn zu wenige Ablesungen vorliegen oder keine gelang — dann ist die
/// Frage nicht beantwortbar, und das ist etwas anderes als „keine Drift".
#[must_use]
pub fn takt_drift(werte: &[f64]) -> Option<f64> {
    if werte.len() < ABSCHNITTE_MIN {
        return None;
    }
    let mitte = werte.len() / 2;
    #[allow(clippy::cast_precision_loss)]
    let mittel = |teil: &[f64]| teil.iter().sum::<f64>() / teil.len() as f64;
    let erste = mittel(&werte[..mitte]);
    let zweite = mittel(&werte[mitte..]);
    if erste <= 0.0 {
        return None;
    }
    Some((zweite - erste).abs() / erste)
}

/// Relative Streuung einer Reihe von Raten.
///
/// **Standardabweichung durch Mittelwert**, nicht Spannweite durch Mittelwert:
/// Die Spannweite hängt an genau zwei Werten und wächst mit jedem weiteren
/// Abschnitt, auch wenn die Messung ruhiger wird.
///
/// Sie beschreibt die **Abschnitte**, nicht das Mittel; die Abbruchregel hängt
/// an [`relative_unsicherheit`].
#[must_use]
pub fn streuung(raten: &[f64]) -> f64 {
    if raten.len() < 2 {
        return f64::INFINITY;
    }
    #[allow(clippy::cast_precision_loss)]
    let n = raten.len() as f64;
    let mittel = raten.iter().sum::<f64>() / n;
    if mittel <= 0.0 {
        return f64::INFINITY;
    }
    let varianz = raten.iter().map(|r| (r - mittel).powi(2)).sum::<f64>() / n;
    varianz.sqrt() / mittel
}

/// Relative Unsicherheit des Mittelwerts einer Reihe von Raten:
/// Standardfehler durch Mittelwert.
///
/// Aus [`streuung`] abgeleitet, damit beide Größen auf derselben Rechnung
/// stehen: Die Streuung teilt durch `n`, der Standardfehler braucht die
/// Stichproben-Standardabweichung (`n - 1`) geteilt durch `sqrt(n)` — zusammen
/// `streuung / sqrt(n - 1)`.
///
/// **Unendlich bei weniger als zwei Werten**, aus demselben Grund wie bei
/// `streuung`: Eine einzige Messung hat keine bekannte Unsicherheit, und eine
/// Null hier beendete den Lauf nach dem ersten Abschnitt.
#[must_use]
pub fn relative_unsicherheit(raten: &[f64]) -> f64 {
    unsicherheit_aus(streuung(raten), raten.len())
}

/// `streuung / sqrt(n - 1)`, unendlich unter zwei Werten.
fn unsicherheit_aus(streuung: f64, n: usize) -> f64 {
    if n < 2 {
        return f64::INFINITY;
    }
    #[allow(clippy::cast_precision_loss)]
    let freiheitsgrade = (n - 1) as f64;
    streuung / freiheitsgrade.sqrt()
}

/// Ob ein adaptiver Lauf nach `gelaufen` aufhören darf, und warum.
///
/// `None` heißt: weitermessen. Alle `reihen` müssen die Bedingung erfüllen —
/// beim verschränkten Lauf die Kalibrierung UND die Referenz.
#[must_use]
pub fn urteil_nach(gelaufen: Duration, reihen: &[&[f64]]) -> Option<Abbruchgrund> {
    let stabil = !reihen.is_empty()
        && reihen
            .iter()
            .all(|r| r.len() >= ABSCHNITTE_MIN && relative_unsicherheit(r) < STREUUNG_MAX);
    if gelaufen >= MINDESTDAUER && stabil {
        return Some(Abbruchgrund::Stabil);
    }
    if gelaufen >= HOECHSTDAUER {
        return Some(Abbruchgrund::Hoechstdauer);
    }
    None
}

/// Fährt einen adaptiven Kalibrierlauf.
///
/// `arbeit` bekommt keine Argumente und liefert die Zahl der Einheiten, die sie
/// gefahren hat. Sie wird so oft aufgerufen, bis der Lauf stabil ist — die
/// Funktion selbst entscheidet also **nicht** über die Dauer.
///
/// # Warum die Arbeit ihre Einheiten zurückgibt
///
/// Weil nur sie sie kennt. Ein Kalibrierer, der die Menge vorschreibt, misst
/// entweder zu kurz (bei schneller Arbeit) oder blockiert zu lange (bei
/// langsamer). Die Arbeit wählt ein Paket, das auf jeder Maschine in etwa
/// dieselbe Größenordnung braucht, und sagt hinterher, wie viel es war.
pub fn kalibriere<F>(mut arbeit: F) -> RateModel
where
    F: FnMut() -> f64,
{
    let kerne = erlaubte_kerne().unwrap_or_default();
    let drossel_vorher = taktdrosselung();
    if let Some(abbruch) = vorpruefung() {
        return abbruch;
    }

    // Aufwärmen zählt nicht mit.
    let warm_bis = Instant::now() + AUFWAERMEN;
    while Instant::now() < warm_bis {
        let _ = arbeit();
    }

    // **Der Bezugstakt wird NACH dem Aufwärmen gelesen, nicht davor.**
    //
    // Davor steht der Leerlauftakt. Auf einer Maschine mit `performance`-Regler
    // und Turbo springt er beim ersten Paket von 800 auf 4400 MHz — das sind
    // 450 Prozent, und jede Messung bräche sofort mit `TaktDrift` ab.
    //
    // **Gemessen am 2026-08-22 auf proxmox**, erster Lauf dieses Programms:
    // Alle sechs Raten meldeten `TaktDrift`, fünf davon nach genau EINEM
    // Abschnitt. Die Maschine war ruhig; gedriftet war die Rampe, die das
    // Aufwärmen gerade beseitigen soll.
    //
    // Der Bezug steht seither NICHT mehr in einer Variablen: Verglichen werden
    // die zwei Hälften der Messreihe, und die entstehen erst im Lauf.

    let start = Instant::now();
    let mut abschnitte: Vec<Abschnitt> = Vec::new();
    // **Kein Anfangswert mit Bedeutung.** Jeder Ausgang der Schleife setzt ihn;
    // ein vorbelegter Wert sähe aus wie ein Vorgabefall und wäre in Wahrheit
    // unerreichbar — clippy meldet ihn zu Recht als nie gelesen.
    let grund;

    loop {
        abschnitte.push(abschnitt_messen(&mut arbeit, &kerne));
        let gelaufen = start.elapsed();

        // **Störungen erst, wenn genug Abschnitte vorliegen.** Ein Abbruch nach
        // dem ersten liefert eine Rate mit Streuung `inf` — eine Zahl, die
        // niemand deuten kann, und der Grund daneben verdeckt, dass gar nicht
        // gemessen wurde.
        if abschnitte.len() < ABSCHNITTE_MIN {
            continue;
        }

        // **Von den Störungen bricht nur EINE ab: die Drosselung.**
        //
        // Sie ist ein diskretes Ereignis — der Zähler springt oder er springt
        // nicht —, und sie verfälscht die Rate nachweislich. Druck und Takt
        // sind etwas anderes, und die Messung hat es an zwei Rechnern gezeigt:
        //
        // `PSI cpu some avg10` steigt, WEIL gemessen wird. Wer zwölf Stränge
        // auslastet, lässt zwangsläufig etwas warten; auf proxmox brachen zwei
        // Raten mit `Druck` ab, nachdem der eigene Lauf den Wert über die
        // Schwelle geschoben hatte. Ein Abbruch darauf beendet jede
        // erfolgreiche Messung.
        //
        // Der Takt schwankt auf einem modernen Regler im LEERLAUF um über
        // hundert Prozent (gemessen, siehe `TAKT_DRIFT_MAX`). Eine
        // Abbruchbedingung darauf misst den Regler, nicht die Güte.
        //
        // **Was beides bereits enthält, ist die Streuung der Raten.** Ein Takt,
        // der schwankt, ohne die Raten zu bewegen, hat die Messung nicht
        // gestört — und einer, der sie bewegt, treibt die Streuung hoch. Die
        // zwei Größen werden deshalb GEMELDET und nicht zur Entscheidung
        // benutzt.
        //
        // Die Fremdlast VOR der Messung ist ein anderer Fall; sie steht oben
        // und bricht ab, bevor überhaupt gerechnet wird.
        if gedrosselt(drossel_vorher) {
            grund = Abbruchgrund::Drosselung;
            break;
        }

        // Ein Paket, das die Uhr nicht auflöst, macht jede Rate darüber zu
        // einer Aussage über die Auflösung.
        if abschnitte.iter().all(|a| a.dauer < ABSCHNITT_MIN) {
            grund = Abbruchgrund::PaketZuKlein;
            break;
        }

        if let Some(g) = urteil_nach(gelaufen, &[&raten_von(&abschnitte)]) {
            grund = g;
            break;
        }
    }

    let takte: Vec<f64> = abschnitte.iter().filter_map(|a| a.takt_mhz).collect();
    modell(&abschnitte, grund, start.elapsed(), takt_drift(&takte))
}

/// Die Vorprüfung vor jeder Messung. `Some` ist der Abbruch mit
/// [`Abbruchgrund::Druck`], `None` heißt: messen.
fn vorpruefung() -> Option<RateModel> {
    // **Fremdlast VOR der Messung ist ein Abbruchgrund, eigener Druck nicht.**
    //
    // Der Unterschied ist der ganze Punkt: Was hier anliegt, hat mit dieser
    // Messung nichts zu tun und verfälscht sie. Was später anliegt, hat sie
    // selbst erzeugt.
    //
    // Die Ein- und Ausgabe bleibt draußen — der Simulator liest nichts von der
    // Platte. Auf gonet stand `PSI io some avg10` am 2026-08-22 bei 16,3
    // Prozent, dauerhaft; drei von sechs Raten brachen daran ab, ohne dass
    // irgendetwas gestört war.
    // **Erst abklingen lassen, dann urteilen.** `some avg10` mittelt über zehn
    // Sekunden; nach einem eigenen Volllastlauf steht der Wert noch hoch,
    // obwohl die Maschine bereits frei ist.
    //
    // Gemessen am 2026-08-22 auf proxmox: Nach der ersten Arbeitsart (30 s
    // Volllast) stand `PSI cpu some avg10` bei 25,7 Prozent — und die restlichen
    // VIER Arbeitsarten brachen sofort mit `Druck` ab, ohne einen einzigen
    // Abschnitt. Der Druck war der eigene Nachhall.
    //
    // Der Unterschied zu echter Fremdlast ist, dass Nachhall FÄLLT. Wer wartet,
    // trennt beides — und wer nicht wartet, hält jede zweite Messung für
    // gestört.
    let abkling_bis = Instant::now() + ABKLINGEN_MAX;
    while Instant::now() < abkling_bis {
        let hoch = drucksignale()
            .iter()
            .any(|s| s.ueberschritten() && s.quelle != "/proc/pressure/io");
        if !hoch {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let fremdlast = drucksignale()
        .into_iter()
        .find(|s| s.ueberschritten() && s.quelle != "/proc/pressure/io");
    fremdlast.map(|_| RateModel {
        rate_per_s: 0.0,
        spread: f64::INFINITY,
        samples: 0,
        duration_ms: 0,
        reason: Abbruchgrund::Druck,
        measured_at: jetzt(),
        psi_cpu: psi_some_avg10("/proc/pressure/cpu"),
        takt_drift: None,
    })
}

/// Misst einen Abschnitt: einen Aufruf der Arbeit, mit Dauer und Takt.
fn abschnitt_messen<F: FnMut() -> f64>(arbeit: &mut F, kerne: &[usize]) -> Abschnitt {
    let t0 = Instant::now();
    let einheiten = arbeit();
    Abschnitt {
        einheiten,
        dauer: t0.elapsed(),
        takt_mhz: takt_mhz(kerne),
    }
}

/// Ob die CPU seit `vorher` neu gedrosselt hat.
fn gedrosselt(vorher: Option<u64>) -> bool {
    matches!((vorher, taktdrosselung()), (Some(v), Some(n)) if n > v)
}

/// Die Raten der Abschnitte, in ihrer Reihenfolge.
fn raten_von(abschnitte: &[Abschnitt]) -> Vec<f64> {
    abschnitte.iter().map(|a| a.rate()).collect()
}

/// Das [`RateModel`] über eine Reihe von Abschnitten.
fn modell(
    abschnitte: &[Abschnitt],
    grund: Abbruchgrund,
    dauer: Duration,
    takt_drift: Option<f64>,
) -> RateModel {
    let raten = raten_von(abschnitte);
    #[allow(clippy::cast_precision_loss)]
    let mittel = if raten.is_empty() {
        0.0
    } else {
        raten.iter().sum::<f64>() / raten.len() as f64
    };
    RateModel {
        rate_per_s: mittel,
        spread: streuung(&raten),
        samples: raten.len(),
        duration_ms: u64::try_from(dauer.as_millis()).unwrap_or(u64::MAX),
        reason: grund,
        measured_at: jetzt(),
        psi_cpu: psi_some_avg10("/proc/pressure/cpu"),
        takt_drift,
    }
}

/// Fährt einen Lauf mit **fester** Dauer — nur für die Kontrollprobe.
///
/// # Warum es diese Funktion gibt, obwohl das Issue feste Dauern verbietet
///
/// #912 verlangt eine Kontrollprobe: „Ein absichtlich auf zwei Sekunden
/// verkürzter Lauf muss eine sichtbar größere Streuung zeigen als der
/// adaptive." Ohne diesen Vergleich ist **nicht belegt, dass die Adaptivität
/// etwas bringt** — sie wäre eine Behauptung mit Code daneben.
///
/// Die Funktion ist deshalb ausdrücklich als Kontrolle benannt und liefert ein
/// [`RateModel`] mit [`Abbruchgrund::Hoechstdauer`]: Ihre Zahl darf nirgends
/// als Kalibrierung durchgehen.
pub fn kontrolllauf_feste_dauer<F>(mut arbeit: F, dauer: Duration) -> RateModel
where
    F: FnMut() -> f64,
{
    let start = Instant::now();
    let mut abschnitte: Vec<Abschnitt> = Vec::new();
    while start.elapsed() < dauer {
        let t0 = Instant::now();
        let einheiten = arbeit();
        abschnitte.push(Abschnitt {
            einheiten,
            dauer: t0.elapsed(),
            // Der Kontrolllauf braucht den Takt nicht — er prüft nichts, er
            // vergleicht nur seine Streuung mit der des adaptiven Laufs.
            takt_mhz: None,
        });
    }
    let raten: Vec<f64> = abschnitte.iter().map(|a| a.rate()).collect();
    #[allow(clippy::cast_precision_loss)]
    let mittel = if raten.is_empty() {
        0.0
    } else {
        raten.iter().sum::<f64>() / raten.len() as f64
    };
    RateModel {
        rate_per_s: mittel,
        spread: streuung(&raten),
        samples: raten.len(),
        duration_ms: u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
        reason: Abbruchgrund::Hoechstdauer,
        measured_at: jetzt(),
        psi_cpu: psi_some_avg10("/proc/pressure/cpu"),
        takt_drift: None,
    }
}

/// Der Schlüssel, unter dem ein Profil gilt.
///
/// # Warum vier Größen und nicht nur der Rechnername
///
/// Ein Profil gilt für **eine bestimmte Maschine in einem bestimmten Zustand**.
/// Ändert sich der gebaute Simulator, der Kernel oder das Taktregime, ist die
/// alte Rate eine Aussage über etwas anderes.
///
/// Der Artefakt-Hash ist dabei der wichtigste: Er ändert sich bei jeder
/// Optimierung — und genau dann ist die alte Rate am irreführendsten, weil sie
/// plausibel aussieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profilschluessel {
    /// Rechner-Kennung, etwa der Hostname.
    pub host: String,
    /// Hash des gebauten Programms.
    pub artefakt: String,
    /// Kernelklasse, etwa `6.19`.
    pub kernel: String,
    /// Taktregime, etwa `powersave` oder `performance`.
    pub taktregime: String,
}

impl Profilschluessel {
    /// Eine Zeile, die sich speichern und wieder einlesen lässt.
    #[must_use]
    pub fn zeile(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.host, self.artefakt, self.kernel, self.taktregime
        )
    }

    /// Die Umkehrung. `None`, wenn die Zeile nicht vier Felder hat.
    #[must_use]
    pub fn aus_zeile(zeile: &str) -> Option<Self> {
        let mut teile = zeile.split('|');
        let s = Self {
            host: teile.next()?.to_owned(),
            artefakt: teile.next()?.to_owned(),
            kernel: teile.next()?.to_owned(),
            taktregime: teile.next()?.to_owned(),
        };
        // **Ein fünftes Feld ist ein Fehler, keine Erweiterung.** Wer die Zeile
        // später erweitert, ohne das Format zu versionieren, bekommt hier ein
        // `None` statt eines still falsch gelesenen Profils.
        if teile.next().is_some() {
            return None;
        }
        Some(s)
    }
}

/// Erhebt den Schlüssel dieses Rechners.
#[must_use]
pub fn schluessel_erheben(artefakt: &str) -> Profilschluessel {
    Profilschluessel {
        host: std::fs::read_to_string("/proc/sys/kernel/hostname")
            .map(|s| s.trim().to_owned())
            .unwrap_or_else(|_| "unbekannt".to_owned()),
        artefakt: artefakt.to_owned(),
        kernel: std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().split('-').next().unwrap_or("").to_owned())
            .unwrap_or_else(|_| "unbekannt".to_owned()),
        taktregime: std::fs::read_to_string(
            "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
        )
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "unbekannt".to_owned()),
    }
}

/// Ein gespeichertes Profil.
#[derive(Debug, Clone)]
pub struct Profil {
    /// Wofür dieses Profil gilt.
    pub schluessel: Profilschluessel,
    /// Was der Rechner meldet.
    pub caps: HostCaps,
    /// Was er je Arbeitsart leistet.
    pub raten: Vec<(Arbeitsart, RateModel)>,
}

impl Profil {
    /// Das Speicherformat: eine Zeile je Angabe, `schluessel wert`.
    ///
    /// **Von Hand geschrieben, ohne Serde.** Die Bibliothek führt bewusst keine
    /// Laufzeit-Abhängigkeit (`sim/Cargo.toml` nennt den Grund: Determinismus).
    /// Ein Zeilenformat ist außerdem lesbar, wenn etwas schiefgeht — ein
    /// abgeschnittenes JSON ist es nicht.
    #[must_use]
    pub fn speichern(&self) -> String {
        let mut s = String::new();
        s.push_str("rota-profil 1\n");
        s.push_str(&format!("schluessel {}\n", self.schluessel.zeile()));
        s.push_str(&format!("threads_total {}\n", self.caps.threads_total));
        s.push_str(&format!("threads_usable {}\n", self.caps.threads_usable));
        s.push_str(&format!("l2_per_core {}\n", self.caps.l2_per_core));
        s.push_str(&format!("l3 {}\n", self.caps.l3));
        s.push_str(&format!("ram_total {}\n", self.caps.ram_total));
        s.push_str(&format!("measured_at {}\n", self.caps.measured_at));
        for (art, rate) in &self.raten {
            // Die Rate als Bitmuster **und** als lesbare Zahl: Das Bitmuster
            // ist die Wahrheit, die Dezimalzahl der Kommentar. Wer nur den
            // Text speichert, verliert beim Zurücklesen die letzten Bits —
            // dieselbe Klasse wie ein Fingerabdruck aus `{:.10}`.
            s.push_str(&format!(
                "rate {} {:016x} {:.3} {:.5} {} {}\n",
                art.schluessel(),
                rate.rate_per_s.to_bits(),
                rate.rate_per_s,
                rate.spread,
                rate.samples,
                rate.measured_at
            ));
        }
        s
    }

    /// Liest die Raten aus einem gespeicherten Profil.
    ///
    /// Gibt die Paare zurück, die sich lesen ließen. **Eine kaputte Zeile wird
    /// übersprungen und nicht geraten** — ein Profil mit einer erfundenen Rate
    /// wäre schlimmer als keines.
    #[must_use]
    pub fn raten_lesen(text: &str) -> Vec<(String, f64)> {
        let mut out = Vec::new();
        for zeile in text.lines() {
            let mut f = zeile.split_whitespace();
            if f.next() != Some("rate") {
                continue;
            }
            let (Some(name), Some(bits)) = (f.next(), f.next()) else {
                continue;
            };
            if let Ok(b) = u64::from_str_radix(bits, 16) {
                let rate = f64::from_bits(b);
                if rate.is_finite() && rate > 0.0 {
                    out.push((name.to_owned(), rate));
                }
            }
        }
        out
    }

    /// Ergänzt fehlende Raten aus demselben Profil, ohne deren Messbeleg
    /// durch Metadaten eines verworfenen neuen Laufs zu ersetzen (#912).
    #[must_use]
    pub fn speichern_mit_vorherigem(&self, vorher: Option<&str>) -> String {
        let mut text = self.speichern();
        let Some(vorher) = vorher else { return text };
        let schluessel = format!("schluessel {}", self.schluessel.zeile());
        if vorher.lines().next() != Some("rota-profil 1")
            || !vorher.lines().any(|zeile| zeile == schluessel)
        {
            return text;
        }
        let mut vorhanden: Vec<String> = self
            .raten
            .iter()
            .map(|(art, _)| art.schluessel().to_owned())
            .collect();
        for zeile in vorher.lines() {
            let felder: Vec<&str> = zeile.split_whitespace().collect();
            if felder.len() != 7
                || felder[0] != "rate"
                || vorhanden.iter().any(|name| name == felder[1])
                || !Arbeitsart::alle()
                    .iter()
                    .any(|art| art.schluessel() == felder[1])
                || Self::raten_lesen(zeile).len() != 1
                || !felder[4]
                    .parse::<f64>()
                    .is_ok_and(|v| v.is_finite() && v >= 0.0)
                || !felder[5].parse::<usize>().is_ok_and(|v| v > 0)
                || felder[6].parse::<Timestamp>().is_err()
            {
                continue;
            }
            text.push_str(zeile);
            text.push('\n');
            vorhanden.push(felder[1].to_owned());
        }
        text
    }
}

/// Was AC1 verlangt: höchstens zehn Prozent Abweichung.
///
/// # Wogegen — und warum nicht mehr gegen eine Tabelle
///
/// Bis zum 28.10. stand hier eine Funktion mit drei Referenzwerten aus #912,
/// nach Hostname: 210 094 für den Laptop, 121 689 für gonet, 221 688 für
/// proxmox. Das waren **Momentanwerte unter drei verschiedenen Lasten** vom
/// 16.08., und ein 10-Prozent-Kriterium gegen einen Momentanwert ist nur durch
/// Lastglück erfüllbar (#912, AC1, Umformulierung vom 22.08.). Dazu traf der
/// Name auf keinem der zwei Zielrechner mehr: Der Laptop heißt seit einem
/// Neustart `localhost.localdomain`, der Container auf node3 `rota-sim`.
///
/// Verglichen wird deshalb mit [`Verschraenkt::ac1`] gegen eine Referenz, die
/// im selben Lauf auf derselben Maschine gemessen wird — verschränkt mit der
/// Kalibrierung, Abschnitt um Abschnitt ([`kalibriere_verschraenkt`]).
pub const AC1_TOLERANZ: f64 = 0.10;

/// Was der Vergleich aus AC1 über eine Kalibrierung sagt.
///
/// # Warum vier Zustände und nicht zwei
///
/// AC1 fragt, ob die Kalibrierung die **gleichzeitig auf derselben Maschine**
/// erhobene Referenz trifft — gleiche Konfiguration, gleicher Zeitraum,
/// gleiche Last. Zeitraum und Last sichert die Verschränkung
/// ([`kalibriere_verschraenkt`]); ob die zwei Mittel überhaupt genau genug
/// sind, lässt sich am Lauf selbst nachprüfen. Wo nicht, ist „erfüllt" wie
/// „nicht erfüllt" eine Behauptung über etwas, das nicht gemessen wurde.
///
/// # Warum nicht mehr davor und danach
///
/// Bis zum 28.10. lief die Referenz einmal vor und einmal nach der
/// Kalibrierung. Auf node3 lagen die zwei 10,7 % auseinander (P1, 400 814
/// gegen 446 023 Kämpfe je Sekunde): Unter dieser Drift kann ein
/// Vorher/Nachher-Vergleich das 10-Prozent-Kriterium nicht tragen, gleich wie
/// gut die Kalibrierung ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ac1Urteil {
    /// Die Kalibrierung trifft die Referenz innerhalb von [`AC1_TOLERANZ`].
    Erfuellt,
    /// Sie verfehlt sie.
    NichtErfuellt,
    /// Mindestens eine der zwei Reihen endete nicht [`Abbruchgrund::Stabil`]:
    /// Ihr Mittel ist nicht genau genug, um gegen das andere gehalten zu
    /// werden, und eine solche Kalibrierung wird ohnehin nicht abgelegt.
    KeineKalibrierung,
    /// Eine der zwei Zahlen ist keine Rate.
    Ungueltig,
}

/// Der Vergleich aus AC1 mit den Zahlen, aus denen das Urteil entsteht.
///
/// Die Zahlen stehen auch dann da, wenn das Urteil keines ist: Wer einen
/// unruhigen Lauf verwirft, will trotzdem sehen, wie weit er gelegen hätte.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ac1Vergleich {
    /// Mittel der Referenzabschnitte.
    pub referenz: f64,
    /// `(kalibriert - referenz) / referenz`.
    pub abweichung: f64,
    /// Das Urteil.
    pub urteil: Ac1Urteil,
}

/// Ein verschränkter Lauf: Kalibrier- und Referenzabschnitte abwechselnd,
/// gleich lang, auf denselben Kernen (#912, AC1).
#[derive(Debug, Clone, Copy)]
pub struct Verschraenkt {
    /// Die Rate über die Kalibrierabschnitte.
    pub kalibrierung: RateModel,
    /// Die Rate über die Referenzabschnitte.
    pub referenz: RateModel,
}

impl Verschraenkt {
    /// Der Vergleich aus AC1: Mittel der Kalibrierabschnitte gegen Mittel der
    /// Referenzabschnitte.
    #[must_use]
    pub fn ac1(&self) -> Ac1Vergleich {
        let rate = |x: f64| x.is_finite() && x > 0.0;
        if !rate(self.kalibrierung.rate_per_s) || !rate(self.referenz.rate_per_s) {
            return Ac1Vergleich {
                referenz: f64::NAN,
                abweichung: f64::NAN,
                urteil: Ac1Urteil::Ungueltig,
            };
        }
        let referenz = self.referenz.rate_per_s;
        let abweichung = (self.kalibrierung.rate_per_s - referenz) / referenz;
        let urteil = if !self.kalibrierung.reason.brauchbar() || !self.referenz.reason.brauchbar() {
            Ac1Urteil::KeineKalibrierung
        } else if abweichung.abs() <= AC1_TOLERANZ {
            Ac1Urteil::Erfuellt
        } else {
            Ac1Urteil::NichtErfuellt
        };
        Ac1Vergleich {
            referenz,
            abweichung,
            urteil,
        }
    }
}

/// Fährt Kalibrierung und Referenz **verschränkt**: ein Abschnitt der einen,
/// ein Abschnitt der anderen, bis beide Mittel sicher genug sind.
///
/// # Was der Aufrufer zusichert
///
/// Dass beide Abschnitte **gleich viel Arbeit** sind — dann sind sie auf
/// derselben Maschine auch gleich lang, und eine Drift trifft beide Reihen
/// gleich. Die Funktion prüft das nicht; sie kann es nicht, ohne die Arbeit
/// zu kennen.
///
/// # Wie aufgehört wird
///
/// Wie bei [`kalibriere`], nur über zwei Reihen: Vorprüfung auf Fremdlast,
/// Aufwärmen (ebenfalls abwechselnd), dann Paare, bis [`urteil_nach`] für
/// BEIDE Reihen `Stabil` sagt oder die Höchstdauer erreicht ist. Drosselung und
/// zu kleine Pakete brechen ab wie dort. Beide Raten tragen denselben
/// Abbruchgrund und dieselbe Dauer — sie sind ein Lauf.
pub fn kalibriere_verschraenkt<A, B>(mut kalibrierung: A, mut referenz: B) -> Verschraenkt
where
    A: FnMut() -> f64,
    B: FnMut() -> f64,
{
    let kerne = erlaubte_kerne().unwrap_or_default();
    let drossel_vorher = taktdrosselung();
    if let Some(abbruch) = vorpruefung() {
        return Verschraenkt {
            kalibrierung: abbruch,
            referenz: abbruch,
        };
    }

    // Auch das Aufwärmen wechselt ab, damit beide Arbeiten warm sind.
    let warm_bis = Instant::now() + AUFWAERMEN;
    loop {
        let _ = kalibrierung();
        let _ = referenz();
        if Instant::now() >= warm_bis {
            break;
        }
    }

    let start = Instant::now();
    let mut ka: Vec<Abschnitt> = Vec::new();
    let mut re: Vec<Abschnitt> = Vec::new();
    let grund;
    loop {
        ka.push(abschnitt_messen(&mut kalibrierung, &kerne));
        re.push(abschnitt_messen(&mut referenz, &kerne));
        let gelaufen = start.elapsed();
        if ka.len() < ABSCHNITTE_MIN {
            continue;
        }
        if gedrosselt(drossel_vorher) {
            grund = Abbruchgrund::Drosselung;
            break;
        }
        if ka.iter().chain(re.iter()).all(|a| a.dauer < ABSCHNITT_MIN) {
            grund = Abbruchgrund::PaketZuKlein;
            break;
        }
        if let Some(g) = urteil_nach(gelaufen, &[&raten_von(&ka), &raten_von(&re)]) {
            grund = g;
            break;
        }
    }

    // Der Takt über beide Reihen in zeitlicher Folge: Er gehört zur Maschine,
    // nicht zu einer der zwei Arbeiten.
    let takte: Vec<f64> = ka
        .iter()
        .zip(re.iter())
        .flat_map(|(a, b)| [a.takt_mhz, b.takt_mhz])
        .flatten()
        .collect();
    let drift = takt_drift(&takte);
    let dauer = start.elapsed();
    Verschraenkt {
        kalibrierung: modell(&ka, grund, dauer, drift),
        referenz: modell(&re, grund, dauer, drift),
    }
}

/// Wo Profile liegen.
///
/// `ROTA_PROFILVERZEICHNIS` schlägt alles; sonst `$XDG_CACHE_HOME/rota-sim`,
/// sonst `$HOME/.cache/rota-sim`.
///
/// **Nicht im Repository.** Ein Profil beschreibt eine Maschine; wer es
/// eincheckt, verteilt die Rate eines fremden Rechners an alle — und sie sieht
/// gemessen aus.
///
/// **Und nicht in `/work/tmp`.** Das Verzeichnis wird von mehreren Sitzungen
/// geteilt und regelmäßig geleert; ein Profil, das beim nächsten Lauf fehlt,
/// erzwingt eine Vollerhebung, die AC4 gerade sparen soll.
#[must_use]
pub fn profilverzeichnis() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("ROTA_PROFILVERZEICHNIS") {
        return std::path::PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("XDG_CACHE_HOME") {
        return std::path::PathBuf::from(p).join("rota-sim");
    }
    std::env::var("HOME").map_or_else(
        |_| std::path::PathBuf::from(".rota-sim"),
        |h| std::path::PathBuf::from(h).join(".cache").join("rota-sim"),
    )
}

/// Der Dateiname eines Profils.
///
/// Der Schlüssel steht **im Dateinamen**, nicht nur in der Datei: Sonst müsste
/// man jede Datei öffnen, um zu wissen, wofür sie gilt — und ein Profil mit
/// falschem Schlüssel fiele erst beim Lesen auf.
///
/// Alles, was kein Buchstabe, keine Ziffer und kein Bindestrich ist, wird zu
/// `_`. Ein Hostname mit einem Schrägstrich darin gäbe sonst einen Pfad.
#[must_use]
pub fn profildatei(s: &Profilschluessel) -> String {
    let sauber = |t: &str| -> String {
        t.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    format!(
        "{}--{}--{}--{}.profil",
        sauber(&s.host),
        sauber(&s.artefakt),
        sauber(&s.kernel),
        sauber(&s.taktregime)
    )
}

/// Was beim Bestätigen eines Profils herauskam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bestaetigung {
    /// Es gibt kein Profil für diesen Schlüssel — voll erheben.
    Fehlt,
    /// Die gemessene Rate passt zur gespeicherten.
    Bestaetigt,
    /// Sie weicht um mehr als [`NEUERHEBUNG_AB`] ab — neu erheben.
    Abweichung,
}

/// Vergleicht eine frisch gemessene Rate mit dem gespeicherten Profil.
///
/// # Warum die Antwort drei Zustände hat und nicht zwei
///
/// „Neu erheben" hat zwei Gründe, und sie sind verschieden viel wert: Ein
/// **fehlendes** Profil ist der Normalfall beim ersten Lauf auf einem Rechner.
/// Eine **Abweichung** heißt, dass sich an der Maschine etwas geändert hat —
/// das ist ein Befund, und wer ihn als „fehlt" führt, verliert ihn.
#[must_use]
pub fn bestaetigen(gespeichert: Option<f64>, gemessen: &RateModel) -> Bestaetigung {
    match gespeichert {
        None => Bestaetigung::Fehlt,
        Some(erwartet) if gemessen.weicht_ab(erwartet) => Bestaetigung::Abweichung,
        Some(_) => Bestaetigung::Bestaetigt,
    }
}

/// Die gespeicherte Rate einer Arbeitsart, falls es eine gibt.
#[must_use]
pub fn gespeicherte_rate(text: &str, art: Arbeitsart) -> Option<f64> {
    Profil::raten_lesen(text)
        .into_iter()
        .find(|(name, _)| name == art.schluessel())
        .map(|(_, wert)| wert)
}

/// Der kurze Bestätigungslauf aus AC4: [`MINDESTDAUER`] lang, ohne adaptive
/// Verlängerung.
///
/// # Warum eine feste Dauer hier richtig ist
///
/// Er **kalibriert nicht**, er prüft, ob ein gespeichertes Profil noch gilt —
/// gegen eine Schwelle von 25 %, nicht von 3. Seine Rate trägt
/// [`Abbruchgrund::FesteDauer`], gilt also nie als Kalibrierung und wird nie
/// abgelegt. Gemessen am 28.10. auf node3: Mit Profil lief bis dahin dieselbe
/// volle Erhebung wie ohne (2:13 gegen 2:39) — genau der Aufwand, den AC4
/// sparen soll.
///
/// Vorprüfung und Aufwärmen sind dieselben wie bei [`kalibriere`]: Unter
/// Fremdlast vor der Messung gibt es keinen Abschnitt.
pub fn bestaetigungslauf<F>(mut arbeit: F) -> RateModel
where
    F: FnMut() -> f64,
{
    let kerne = erlaubte_kerne().unwrap_or_default();
    if let Some(abbruch) = vorpruefung() {
        return abbruch;
    }
    let warm_bis = Instant::now() + AUFWAERMEN;
    while Instant::now() < warm_bis {
        let _ = arbeit();
    }
    let start = Instant::now();
    let mut abschnitte: Vec<Abschnitt> = Vec::new();
    while start.elapsed() < MINDESTDAUER {
        abschnitte.push(abschnitt_messen(&mut arbeit, &kerne));
    }
    let takte: Vec<f64> = abschnitte.iter().filter_map(|a| a.takt_mhz).collect();
    modell(
        &abschnitte,
        Abbruchgrund::FesteDauer,
        start.elapsed(),
        takt_drift(&takte),
    )
}

/// Welcher Lauf gefragt ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lauf {
    /// Der kurze Bestätigungslauf ([`bestaetigungslauf`]).
    Kurz,
    /// Die volle Erhebung ([`kalibriere`]).
    Voll,
}

/// Was eine Arbeitsart in diesem Lauf erfahren hat (AC4, AC5).
#[derive(Debug, Clone, Copy)]
pub enum Erhebung {
    /// Kein Profil: volle Erhebung.
    Fehlt {
        /// Die volle Erhebung.
        voll: RateModel,
    },
    /// Der Bestätigungslauf lag innerhalb von [`NEUERHEBUNG_AB`]: Das Profil
    /// gilt, eine volle Erhebung gab es nicht.
    Bestaetigt {
        /// Der Bestätigungslauf.
        kurz: RateModel,
    },
    /// Der Bestätigungslauf wich ab, also wurde neu erhoben (AC5).
    NeuErhoben {
        /// Der Bestätigungslauf, der abwich.
        kurz: RateModel,
        /// Die volle Erhebung danach.
        voll: RateModel,
        /// Wie die volle Erhebung zum Profil steht.
        urteil: Bestaetigung,
    },
    /// Der Bestätigungslauf kam nicht zum Messen, etwa wegen Fremdlast vor
    /// der Messung. Weder bestätigt noch neu erhoben.
    NichtGemessen {
        /// Der Bestätigungslauf ohne Abschnitt.
        kurz: RateModel,
    },
}

impl Erhebung {
    /// Die maßgebliche Messung: die volle, wenn es eine gab, sonst die kurze.
    #[must_use]
    pub const fn rate(&self) -> &RateModel {
        match self {
            Self::Fehlt { voll } | Self::NeuErhoben { voll, .. } => voll,
            Self::Bestaetigt { kurz } | Self::NichtGemessen { kurz } => kurz,
        }
    }

    /// Was davon ins Profil gehört: eine volle Erhebung, die stabil ist und
    /// nicht abwich — sonst nichts, und das alte Profil bleibt stehen.
    ///
    /// **Eine abgewichene Rate legt NICHT ab.** Sie ist der Befund, nicht die
    /// neue Wahrheit (gemessen am 2026-08-22 auf gonet: Unter Fremdlast war
    /// `crit_dp` um 43,5 % gedrückt und dabei `Stabil` — gleichmäßiger Druck
    /// erzeugt gleichmäßig falsche Raten). Ein Bestätigungslauf legt nie ab:
    /// Er kalibriert nicht.
    #[must_use]
    pub fn abzulegen(&self) -> Option<&RateModel> {
        match self {
            Self::Fehlt { voll }
            | Self::NeuErhoben {
                voll,
                urteil: Bestaetigung::Bestaetigt,
                ..
            } if voll.reason.brauchbar() => Some(voll),
            _ => None,
        }
    }
}

/// Erhebt eine Arbeitsart: mit Profil kurz bestätigen, ohne voll erheben
/// (AC4, AC5).
///
/// `messen` bekommt den gefragten [`Lauf`] und liefert seine Rate —
/// gewöhnlich [`bestaetigungslauf`] für `Kurz` und [`kalibriere`] für `Voll`.
/// Als Funktion übergeben, damit sich prüfen lässt, welcher Lauf gefragt
/// wurde: Dass mit passendem Profil KEINE volle Erhebung läuft, ist die
/// Eigenschaft, um die es in AC4 geht.
pub fn erheben_mit<M>(gespeichert: Option<f64>, mut messen: M) -> Erhebung
where
    M: FnMut(Lauf) -> RateModel,
{
    let Some(erwartet) = gespeichert else {
        return Erhebung::Fehlt {
            voll: messen(Lauf::Voll),
        };
    };
    let kurz = messen(Lauf::Kurz);
    // Kein Abschnitt heißt: nicht gemessen. Eine volle Erhebung träfe dieselbe
    // Vorprüfung, und `bestaetigen` hielte die Rate 0 für eine Abweichung.
    if kurz.samples == 0 {
        return Erhebung::NichtGemessen { kurz };
    }
    if bestaetigen(Some(erwartet), &kurz) == Bestaetigung::Bestaetigt {
        return Erhebung::Bestaetigt { kurz };
    }
    let voll = messen(Lauf::Voll);
    let urteil = bestaetigen(Some(erwartet), &voll);
    Erhebung::NeuErhoben { kurz, voll, urteil }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streuung_ist_null_bei_gleichen_werten() {
        assert!((streuung(&[100.0, 100.0, 100.0, 100.0]) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn streuung_waechst_mit_der_schwankung() {
        let eng = streuung(&[100.0, 101.0, 99.0, 100.0]);
        let weit = streuung(&[100.0, 140.0, 60.0, 100.0]);
        assert!(weit > eng, "weit {weit} nicht groesser als eng {eng}");
        assert!(eng < STREUUNG_MAX, "die enge Reihe gilt als unruhig: {eng}");
        assert!(
            weit > STREUUNG_MAX,
            "die weite Reihe gilt als ruhig: {weit}"
        );
    }

    #[test]
    fn streuung_ist_unendlich_bei_zu_wenig_werten() {
        // **Nicht null.** Eine einzige Messung hat keine Streuung, und wer sie
        // als 0 liest, bricht den adaptiven Lauf nach dem ersten Abschnitt ab
        // — genau der Fehler, gegen den ABSCHNITTE_MIN steht.
        assert!(streuung(&[100.0]).is_infinite());
        assert!(streuung(&[]).is_infinite());
    }

    #[test]
    fn weicht_ab_misst_gegen_die_erwartete_rate() {
        let r = |x: f64| RateModel {
            rate_per_s: x,
            spread: 0.0,
            samples: 4,
            duration_ms: 5000,
            reason: Abbruchgrund::Stabil,
            measured_at: 0,
            psi_cpu: None,
            takt_drift: None,
        };
        assert!(!r(100.0).weicht_ab(100.0));
        assert!(!r(120.0).weicht_ab(100.0), "20 Prozent sind noch drin");
        assert!(r(130.0).weicht_ab(100.0), "30 Prozent muessen ausloesen");
        assert!(r(70.0).weicht_ab(100.0), "auch nach unten");
        // Die Gegenprobe zur Nenner-Frage: 25 Prozent nach oben und nach unten
        // sind gegen dieselbe Bezugsgröße gerechnet.
        assert!(r(126.0).weicht_ab(100.0));
        assert!(r(74.0).weicht_ab(100.0));
    }

    #[test]
    fn weicht_ab_bei_erwarteter_null() {
        // Ohne Erwartung gibt es nichts zu bestätigen — dann wird erhoben.
        let r = RateModel {
            rate_per_s: 1000.0,
            spread: 0.0,
            samples: 4,
            duration_ms: 5000,
            reason: Abbruchgrund::Stabil,
            measured_at: 0,
            psi_cpu: None,
            takt_drift: None,
        };
        assert!(r.weicht_ab(0.0));
    }

    #[test]
    fn nur_stabil_gilt_als_brauchbar() {
        assert!(Abbruchgrund::Stabil.brauchbar());
        for g in [
            Abbruchgrund::Hoechstdauer,
            Abbruchgrund::Druck,
            Abbruchgrund::TaktDrift,
            Abbruchgrund::Drosselung,
            Abbruchgrund::PaketZuKlein,
        ] {
            assert!(!g.brauchbar(), "{g:?} gilt faelschlich als brauchbar");
        }
    }

    #[test]
    fn profilschluessel_geht_hin_und_zurueck() {
        let s = Profilschluessel {
            host: "laptop".to_owned(),
            artefakt: "abc123".to_owned(),
            kernel: "6.19".to_owned(),
            taktregime: "powersave".to_owned(),
        };
        assert_eq!(Profilschluessel::aus_zeile(&s.zeile()), Some(s));
    }

    #[test]
    fn profilschluessel_weist_zu_viele_felder_zurueck() {
        assert!(Profilschluessel::aus_zeile("a|b|c").is_none());
        assert!(Profilschluessel::aus_zeile("a|b|c|d|e").is_none());
    }

    #[test]
    fn rate_geht_bitgleich_durch_den_speicher() {
        // **Der Punkt der Bitspeicherung.** Eine Rate mit vielen Nachkommastellen
        // überlebt `{:.3}` nicht — und ein Profil, das beim Zurücklesen eine
        // andere Zahl liefert, löst die Neuerhebung aus, obwohl sich nichts
        // geändert hat.
        let rate = 210_094.123_456_789_1_f64;
        let profil = Profil {
            schluessel: Profilschluessel {
                host: "h".to_owned(),
                artefakt: "a".to_owned(),
                kernel: "k".to_owned(),
                taktregime: "t".to_owned(),
            },
            caps: HostCaps::erheben(),
            raten: vec![(
                Arbeitsart::ScalarFullFight,
                RateModel {
                    rate_per_s: rate,
                    spread: 0.01,
                    samples: 5,
                    duration_ms: 6000,
                    reason: Abbruchgrund::Stabil,
                    measured_at: 42,
                    psi_cpu: None,
                    takt_drift: None,
                },
            )],
        };
        let gelesen = Profil::raten_lesen(&profil.speichern());
        assert_eq!(gelesen.len(), 1);
        assert_eq!(gelesen[0].0, "scalar_full_fight");
        assert!(
            gelesen[0].1.to_bits() == rate.to_bits(),
            "Bitmuster verloren: {:016x} statt {:016x}",
            gelesen[0].1.to_bits(),
            rate.to_bits()
        );
    }

    #[test]
    fn raten_lesen_ueberspringt_kaputte_zeilen() {
        let text = "rota-profil 1\n\
                    rate scalar_full_fight ZZZZ 1.0 0.0 4 0\n\
                    rate crit_dp 4059000000000000 100.0 0.0 4 0\n";
        let gelesen = Profil::raten_lesen(text);
        assert_eq!(
            gelesen.len(),
            1,
            "die kaputte Zeile wurde nicht uebersprungen"
        );
        assert_eq!(gelesen[0].0, "crit_dp");
    }

    #[test]
    fn ungueltige_raten_werden_nie_bestaetigt() {
        for ungueltig in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
            let mut rate = RateModel {
                rate_per_s: ungueltig,
                spread: 0.0,
                samples: 4,
                duration_ms: 5000,
                reason: Abbruchgrund::Stabil,
                measured_at: 42,
                psi_cpu: None,
                takt_drift: None,
            };
            assert!(rate.weicht_ab(100.0));
            rate.rate_per_s = 100.0;
            assert!(rate.weicht_ab(ungueltig));
            let text = format!("rate crit_dp {:016x} 0 0 4 42\n", ungueltig.to_bits());
            assert!(Profil::raten_lesen(&text).is_empty());
        }
    }

    #[test]
    fn alter_messbeleg_bleibt_beim_teilweisen_erneuern_unveraendert() {
        let mut profil = Profil {
            schluessel: Profilschluessel {
                host: "h".to_owned(),
                artefakt: "bau-a".to_owned(),
                kernel: "k".to_owned(),
                taktregime: "t".to_owned(),
            },
            caps: HostCaps::erheben(),
            raten: vec![],
        };
        let alte_zeile = "rate scalar_full_fight 4059000000000000 100.000 0.01234 17 42";
        let vorher = format!("{}{}\n", profil.speichern(), alte_zeile);
        profil.raten.push((
            Arbeitsart::CritDp,
            RateModel {
                rate_per_s: 200.0,
                spread: 0.02,
                samples: 9,
                duration_ms: 6000,
                reason: Abbruchgrund::Stabil,
                measured_at: 999,
                psi_cpu: None,
                takt_drift: None,
            },
        ));
        let text = profil.speichern_mit_vorherigem(Some(&vorher));
        assert!(text.lines().any(|zeile| zeile == alte_zeile));
        assert_eq!(Profil::raten_lesen(&text).len(), 2);
        // Ein anderer Bau darf den alten Messwert samt Herkunft nicht erben.
        profil.schluessel.artefakt = "bau-b".to_owned();
        let anderer_bau = profil.speichern_mit_vorherigem(Some(&vorher));
        assert_eq!(Profil::raten_lesen(&anderer_bau).len(), 1);
        assert!(!anderer_bau.contains(alte_zeile));
    }

    #[test]
    fn neue_rate_ersetzt_den_gesamten_alten_messbeleg() {
        let profil = Profil {
            schluessel: Profilschluessel {
                host: "h".to_owned(),
                artefakt: "a".to_owned(),
                kernel: "k".to_owned(),
                taktregime: "t".to_owned(),
            },
            caps: HostCaps::erheben(),
            raten: vec![(
                Arbeitsart::CritDp,
                RateModel {
                    rate_per_s: 200.0,
                    spread: 0.02,
                    samples: 9,
                    duration_ms: 6000,
                    reason: Abbruchgrund::Stabil,
                    measured_at: 999,
                    psi_cpu: None,
                    takt_drift: None,
                },
            )],
        };
        let vorher = "rota-profil 1\nschluessel h|a|k|t\nrate crit_dp 4059000000000000 100.000 0.01234 17 42\n";
        let text = profil.speichern_mit_vorherigem(Some(vorher));
        let raten = Profil::raten_lesen(&text);
        assert_eq!(raten.len(), 1);
        assert_eq!(raten[0].1, 200.0);
        assert!(text.contains("0.02000 9 999"));
        assert!(!text.contains("0.01234 17 42"));
    }

    #[test]
    fn die_schluessel_sind_eindeutig() {
        // Zwei Arten mit demselben Schlüssel überschrieben sich im Profil
        // gegenseitig — und die zweite Rate stünde still unter dem Namen der
        // ersten.
        let mut gesehen = Vec::new();
        for art in Arbeitsart::alle() {
            assert!(
                !gesehen.contains(&art.schluessel()),
                "Schluessel {} kommt zweimal vor",
                art.schluessel()
            );
            gesehen.push(art.schluessel());
        }
        assert_eq!(gesehen.len(), 6);
    }

    #[test]
    fn kalibriere_haelt_die_mindestdauer_ein() {
        // Eine Arbeit, die immer gleich lange braucht: Der Lauf muss trotzdem
        // MINDESTDAUER laufen, auch wenn die Streuung sofort null ist.
        //
        // **Die Zusicherung hängt am Abbruchgrund**, und das ist kein Ausweichen:
        // Auf einem belasteten Rechner — einem CI-Läufer zum Beispiel — bricht
        // `kalibriere` absichtlich früher ab, weil `drucksignale()` anschlägt.
        // Ein Fall, der dort trotzdem fünf Sekunden verlangt, wäre nicht
        // strenger, sondern nur unzuverlässig; er meldete die Umgebung als
        // Fehler im Code.
        //
        // Geprüft wird deshalb in beiden Zweigen etwas: bei `Stabil` die Dauer
        // und die Abschnittszahl, sonst dass der Grund die Umgebung benennt.
        let start = Instant::now();
        let r = kalibriere(|| {
            std::thread::sleep(Duration::from_millis(20));
            1000.0
        });
        let gelaufen = start.elapsed();

        // **Der Vorprüfungs-Abbruch ist ein eigener Zweig, kein Fehler.**
        //
        // Liegt schon vor der Messung Fremdlast an, kehrt `kalibriere` sofort
        // mit null Abschnitten zurück — genau so gebaut. Auf dem CI-Läufer ist
        // das der Normalfall: Er ist eine geteilte Maschine unter Dauerlast.
        //
        // Gemessen am 2026-08-22: Die CI-Stufe `sim` fiel an
        // `assert!(r.rate_per_s > 0.0)`, während dieselbe Zeile lokal grün war.
        // Ein Fall, der die Umgebung als Fehler im Code meldet, ist nicht
        // strenger — er ist unbrauchbar.
        if r.reason == Abbruchgrund::Druck && r.samples == 0 {
            assert_eq!(r.rate_per_s, 0.0, "Rate ohne Abschnitt");
            assert!(r.spread.is_infinite(), "Streuung ohne Abschnitt");
            assert!(!r.reason.brauchbar());
            return;
        }

        assert!(r.rate_per_s > 0.0, "keine Rate gemessen");
        assert!(r.samples > 0, "kein einziger Abschnitt");

        if r.reason == Abbruchgrund::Stabil {
            assert!(
                gelaufen >= MINDESTDAUER,
                "stabil nach nur {gelaufen:?}, MINDESTDAUER ist {MINDESTDAUER:?}"
            );
            assert!(r.samples >= ABSCHNITTE_MIN, "nur {} Abschnitte", r.samples);
            // Seit dem 28.10. hängt `Stabil` an der Unsicherheit des Mittels
            // (siehe `STREUUNG_MAX`); dieselbe Größe wird hier gefragt.
            assert!(
                r.unsicherheit() < STREUUNG_MAX,
                "als stabil gemeldet bei Unsicherheit {}",
                r.unsicherheit()
            );
        } else {
            // Der frühe Abbruch ist nur unter einer Störung erlaubt. `Hoechstdauer`
            // gehört ausdrücklich dazu: Sie kann nur nach HOECHSTDAUER fallen,
            // und die ist länger als MINDESTDAUER.
            assert!(
                matches!(
                    r.reason,
                    Abbruchgrund::Druck
                        | Abbruchgrund::TaktDrift
                        | Abbruchgrund::Drosselung
                        | Abbruchgrund::Hoechstdauer
                        | Abbruchgrund::PaketZuKlein
                ),
                "unerwarteter Abbruchgrund {:?}",
                r.reason
            );
            assert!(
                !r.reason.brauchbar(),
                "ein gestoerter Lauf gilt als brauchbar: {:?}",
                r.reason
            );
        }
    }

    #[test]
    fn eine_arbeit_ohne_einheiten_ergibt_keine_rate() {
        // **Die Gegenprobe zur Zeile `rate_per_s > 0` oben.** Ohne sie wäre
        // nicht belegt, dass die Rate überhaupt an der Arbeit hängt — eine
        // Kalibrierung, die immer eine positive Zahl liefert, sagt nichts.
        //
        // Der Lauf bricht hier nach HOECHSTDAUER ab, weil die Streuung bei
        // lauter Nullen unendlich bleibt. Das dauert; deshalb wird die
        // Höchstdauer NICHT abgewartet, sondern die Eigenschaft an `streuung`
        // selbst geprüft — dieselbe Funktion, an der die Abbruchbedingung hängt.
        assert!(streuung(&[0.0, 0.0, 0.0, 0.0]).is_infinite());
    }

    // ------------------------------------------------------------ AC4 und AC5

    fn rate(x: f64) -> RateModel {
        RateModel {
            rate_per_s: x,
            spread: 0.01,
            samples: 5,
            duration_ms: 6000,
            reason: Abbruchgrund::Stabil,
            measured_at: 0,
            psi_cpu: None,
            takt_drift: None,
        }
    }

    #[test]
    fn bestaetigen_trennt_fehlend_von_abweichend() {
        // **Der Kern des Falls.** Beide führen auf „neu erheben", und wer sie
        // zusammenwirft, verliert den Befund: Ein fehlendes Profil ist der
        // erste Lauf auf einem Rechner, eine Abweichung heißt, dass sich dort
        // etwas geändert hat.
        assert_eq!(bestaetigen(None, &rate(100.0)), Bestaetigung::Fehlt);
        assert_eq!(
            bestaetigen(Some(100.0), &rate(100.0)),
            Bestaetigung::Bestaetigt
        );
        assert_eq!(
            bestaetigen(Some(100.0), &rate(200.0)),
            Bestaetigung::Abweichung
        );
    }

    #[test]
    fn bestaetigen_loest_nicht_dauernd_aus() {
        // **Die Kontrollprobe, die #912 ausdrücklich verlangt.** Ein Wächter,
        // der immer anschlägt, ist so nutzlos wie einer, der nie anschlägt —
        // und ohne diesen Fall belegte das Auslösen unter Fremdlast nichts.
        //
        // Zwanzig Prozent Schwankung um den gespeicherten Wert dürfen NICHT
        // auslösen; die Schwelle steht bei 25.
        for abweichung in [-0.20, -0.10, -0.01, 0.0, 0.01, 0.10, 0.20] {
            let gemessen = 100.0 * (1.0 + abweichung);
            assert_eq!(
                bestaetigen(Some(100.0), &rate(gemessen)),
                Bestaetigung::Bestaetigt,
                "{:.0} Prozent Abweichung haben faelschlich ausgeloest",
                abweichung * 100.0
            );
        }
        // Und die Gegenprobe: 30 Prozent lösen aus. Ohne sie prüfte der Fall
        // oben nur, dass die Funktion IMMER `Bestaetigt` sagt.
        for abweichung in [-0.30, 0.30, 1.0] {
            assert_eq!(
                bestaetigen(Some(100.0), &rate(100.0 * (1.0 + abweichung))),
                Bestaetigung::Abweichung,
                "{:.0} Prozent haben NICHT ausgeloest",
                abweichung * 100.0
            );
        }
    }

    #[test]
    fn gespeicherte_rate_findet_die_richtige_art() {
        let profil = Profil {
            schluessel: Profilschluessel {
                host: "h".to_owned(),
                artefakt: "a".to_owned(),
                kernel: "k".to_owned(),
                taktregime: "t".to_owned(),
            },
            caps: HostCaps::erheben(),
            raten: vec![
                (Arbeitsart::ScalarFullFight, rate(210_094.0)),
                (Arbeitsart::CritDp, rate(40.0)),
            ],
        };
        let text = profil.speichern();
        assert_eq!(
            gespeicherte_rate(&text, Arbeitsart::ScalarFullFight),
            Some(210_094.0)
        );
        assert_eq!(gespeicherte_rate(&text, Arbeitsart::CritDp), Some(40.0));
        // Eine Art, die nicht gespeichert wurde, ist `None` — **nicht** die
        // erstbeste Zahl aus der Datei.
        assert_eq!(gespeicherte_rate(&text, Arbeitsart::GearSweep), None);
    }

    #[test]
    fn takt_drift_braucht_genug_werte() {
        // **Nicht null bei zu wenigen.** Eine Drift, die man aus zwei
        // Ablesungen rechnet, ist Rauschen — und wer sie als 0 liest, hält eine
        // schwankende Maschine für ruhig.
        assert_eq!(takt_drift(&[]), None);
        assert_eq!(takt_drift(&[3000.0, 3000.0]), None);
        assert!(takt_drift(&[3000.0; ABSCHNITTE_MIN]).is_some());
    }

    #[test]
    fn takt_drift_misst_den_trend_und_nicht_das_rauschen() {
        // Diese Reihe schwankt stark, hat aber keinen Trend: Beide Hälften
        // mitteln auf denselben Wert. Genau das ist der Unterschied zum alten
        // Vergleich „jetzt gegen vorhin", der hier über 100 Prozent gemeldet
        // hätte.
        let rauschen = [1000.0, 2200.0, 2200.0, 1000.0];
        let d = takt_drift(&rauschen).expect("genug Werte");
        assert!(d < 0.01, "Rauschen als Drift gelesen: {d}");

        // Und ein echter Trend wird erkannt.
        let trend = [1000.0, 1000.0, 2000.0, 2000.0];
        let d = takt_drift(&trend).expect("genug Werte");
        assert!((d - 1.0).abs() < 1e-9, "Trend falsch gerechnet: {d}");
    }

    #[test]
    fn profildatei_macht_aus_dem_schluessel_einen_dateinamen() {
        let s = Profilschluessel {
            host: "laptop/eins".to_owned(),
            artefakt: "0.1.0".to_owned(),
            kernel: "6.19".to_owned(),
            taktregime: "powersave".to_owned(),
        };
        let name = profildatei(&s);
        assert!(
            !name.contains('/'),
            "der Schraegstrich ist im Dateinamen geblieben: {name}"
        );
        assert!(name.starts_with("laptop_eins--"), "{name}");
        assert!(name.ends_with(".profil"), "{name}");
    }

    #[test]
    fn zwei_verschiedene_schluessel_geben_zwei_dateien() {
        // Sonst überschriebe ein Lauf auf demselben Rechner mit anderem
        // Artefakt das alte Profil — und die Rate stünde unter einem Programm,
        // das sie nie gemessen hat.
        let mk = |artefakt: &str| Profilschluessel {
            host: "h".to_owned(),
            artefakt: artefakt.to_owned(),
            kernel: "k".to_owned(),
            taktregime: "t".to_owned(),
        };
        assert_ne!(profildatei(&mk("aaa")), profildatei(&mk("bbb")));
    }

    // ------------------------------------- AC2: Abbruch über den Standardfehler

    /// `k`-mal das Paar 90/110: Mittel 100, Streuung der Abschnitte 10 %.
    fn breit(k: usize) -> Vec<f64> {
        (0..k).flat_map(|_| [90.0, 110.0]).collect()
    }

    #[test]
    fn relative_unsicherheit_faellt_mit_der_wurzel_der_anzahl() {
        // Die Streuung der Abschnitte bleibt bei 10 %, gleich wie lang man
        // misst. Die Unsicherheit des Mittels fällt mit 1/sqrt(n-1) — genau der
        // Unterschied, um den es in der Abbruchregel geht.
        for (k, erwartet) in [(2usize, 0.1 / 3f64.sqrt()), (50, 0.1 / 99f64.sqrt())] {
            let r = breit(k);
            assert!(
                (streuung(&r) - 0.1).abs() < 1e-12,
                "Streuung {} bei n {}",
                streuung(&r),
                r.len()
            );
            let u = relative_unsicherheit(&r);
            assert!(
                (u - erwartet).abs() < 1e-12,
                "Unsicherheit {u} statt {erwartet} bei n {}",
                r.len()
            );
        }
    }

    #[test]
    fn relative_unsicherheit_ist_unendlich_ohne_genug_werte() {
        assert!(relative_unsicherheit(&[100.0]).is_infinite());
        assert!(relative_unsicherheit(&[]).is_infinite());
        assert!(relative_unsicherheit(&[0.0, 0.0, 0.0]).is_infinite());
    }

    #[test]
    fn rate_model_leitet_die_unsicherheit_aus_streuung_und_anzahl_ab() {
        let mut r = rate(100.0);
        r.spread = 0.10;
        r.samples = 101;
        assert!(
            (r.unsicherheit() - 0.01).abs() < 1e-12,
            "Unsicherheit {}",
            r.unsicherheit()
        );
        // Ein einziger Abschnitt hat keine Unsicherheit, die man kennt.
        r.samples = 1;
        assert!(
            r.unsicherheit().is_infinite(),
            "Unsicherheit {}",
            r.unsicherheit()
        );
    }

    #[test]
    fn urteil_nach_haelt_eine_breite_aber_lange_reihe_fuer_stabil() {
        // **Der Fall von node3** (2026-10-28): Abschnitte streuen um 10 %, das
        // Mittel aus 400 davon ist auf ein halbes Prozent bekannt. Die alte
        // Regel hätte bis zur Höchstdauer gemessen und verworfen.
        let r = breit(200);
        assert_eq!(urteil_nach(MINDESTDAUER, &[&r]), Some(Abbruchgrund::Stabil));
        // Vor der Mindestdauer nicht, so sicher das Mittel auch ist.
        assert_eq!(
            urteil_nach(MINDESTDAUER - Duration::from_millis(1), &[&r]),
            None
        );
    }

    #[test]
    fn urteil_nach_meldet_hoechstdauer_wenn_das_mittel_unsicher_bleibt() {
        // Vier Abschnitte 50/150: Unsicherheit 0,5/sqrt(3) = 29 %.
        let r = [50.0, 150.0, 50.0, 150.0];
        assert_eq!(urteil_nach(Duration::from_secs(10), &[&r]), None);
        assert_eq!(
            urteil_nach(HOECHSTDAUER, &[&r]),
            Some(Abbruchgrund::Hoechstdauer)
        );
        // Und zu wenige Abschnitte zählen nicht, auch wenn sie gleich sind.
        let drei = [100.0, 100.0, 100.0];
        assert_eq!(urteil_nach(Duration::from_secs(10), &[&drei]), None);
    }

    #[test]
    fn urteil_nach_verlangt_jede_reihe() {
        // Beim verschränkten Lauf müssen Kalibrierung UND Referenz sicher
        // sein: Ein genaues Mittel gegen ein ungenaues ist kein Vergleich.
        let sicher = breit(200);
        let unsicher = [50.0, 150.0, 50.0, 150.0];
        assert_eq!(
            urteil_nach(Duration::from_secs(10), &[&sicher, &unsicher]),
            None
        );
        assert_eq!(
            urteil_nach(Duration::from_secs(10), &[&sicher, &sicher]),
            Some(Abbruchgrund::Stabil)
        );
        assert_eq!(urteil_nach(Duration::from_secs(10), &[]), None);
    }

    // ---------------------------------------------- AC4: nur kurz bestätigen

    /// Ein Bestätigungslauf mit der Rate `x`.
    fn kurz(x: f64) -> RateModel {
        RateModel {
            reason: Abbruchgrund::FesteDauer,
            ..rate(x)
        }
    }

    /// Fährt `erheben_mit` gegen gefälschte Messungen und schreibt mit, welche
    /// Läufe gefragt wurden.
    fn erhebe(
        gespeichert: Option<f64>,
        kurz_rate: RateModel,
        voll_rate: RateModel,
    ) -> (Erhebung, Vec<Lauf>) {
        let mut gefragt = Vec::new();
        let e = erheben_mit(gespeichert, |lauf| {
            gefragt.push(lauf);
            match lauf {
                Lauf::Kurz => kurz_rate,
                Lauf::Voll => voll_rate,
            }
        });
        (e, gefragt)
    }

    #[test]
    fn erheben_ohne_profil_misst_voll_und_nie_kurz() {
        let (e, gefragt) = erhebe(None, kurz(100.0), rate(100.0));
        assert_eq!(gefragt, [Lauf::Voll]);
        assert!(matches!(e, Erhebung::Fehlt { .. }), "{e:?}");
    }

    #[test]
    fn erheben_mit_passendem_profil_misst_nur_kurz() {
        // **Der Kern von AC4**: Ein vollständiger Lauf bei jedem Start ist zu
        // teuer. Liegt der Bestätigungslauf innerhalb von 25 %, wird NICHT
        // voll erhoben.
        for kurz_rate in [100.0, 80.0, 124.0] {
            let (e, gefragt) = erhebe(Some(100.0), kurz(kurz_rate), rate(100.0));
            assert_eq!(gefragt, [Lauf::Kurz], "kurz {kurz_rate}");
            assert!(
                matches!(e, Erhebung::Bestaetigt { .. }),
                "kurz {kurz_rate}: {e:?}"
            );
        }
    }

    #[test]
    fn erheben_mit_abweichendem_kurzlauf_erhebt_neu() {
        // AC5: Weicht der Bestätigungslauf um mehr als 25 % ab, folgt die volle
        // Erhebung — und erst sie entscheidet.
        let (e, gefragt) = erhebe(Some(100.0), kurz(140.0), rate(140.0));
        assert_eq!(gefragt, [Lauf::Kurz, Lauf::Voll]);
        assert!(
            matches!(
                e,
                Erhebung::NeuErhoben {
                    urteil: Bestaetigung::Abweichung,
                    ..
                }
            ),
            "{e:?}"
        );
        // Bestätigt die volle Erhebung das Profil, war der Kurzlauf ein
        // Ausreißer.
        let (e, _) = erhebe(Some(100.0), kurz(60.0), rate(105.0));
        assert!(
            matches!(
                e,
                Erhebung::NeuErhoben {
                    urteil: Bestaetigung::Bestaetigt,
                    ..
                }
            ),
            "{e:?}"
        );
    }

    #[test]
    fn erheben_ohne_kurzmessung_erhebt_nicht_neu() {
        // Fremdlast vor dem Bestätigungslauf: kein Abschnitt. Eine volle
        // Erhebung träfe dieselbe Vorprüfung — und „Abweichung" wäre ein
        // Befund, wo nicht gemessen wurde.
        let leer = RateModel {
            rate_per_s: 0.0,
            spread: f64::INFINITY,
            samples: 0,
            reason: Abbruchgrund::Druck,
            ..kurz(0.0)
        };
        let (e, gefragt) = erhebe(Some(100.0), leer, rate(100.0));
        assert_eq!(gefragt, [Lauf::Kurz]);
        assert!(matches!(e, Erhebung::NichtGemessen { .. }), "{e:?}");
    }

    #[test]
    fn abgelegt_wird_nur_eine_stabile_volle_erhebung_ohne_abweichung() {
        let mut instabil = rate(100.0);
        instabil.reason = Abbruchgrund::Hoechstdauer;
        let faelle = [
            (Erhebung::Fehlt { voll: rate(100.0) }, true),
            (Erhebung::Fehlt { voll: instabil }, false),
            (Erhebung::Bestaetigt { kurz: kurz(100.0) }, false),
            (
                Erhebung::NeuErhoben {
                    kurz: kurz(140.0),
                    voll: rate(140.0),
                    urteil: Bestaetigung::Abweichung,
                },
                false,
            ),
            (
                Erhebung::NeuErhoben {
                    kurz: kurz(60.0),
                    voll: rate(105.0),
                    urteil: Bestaetigung::Bestaetigt,
                },
                true,
            ),
            (Erhebung::NichtGemessen { kurz: kurz(0.0) }, false),
        ];
        for (e, ablegen) in faelle {
            assert_eq!(e.abzulegen().is_some(), ablegen, "{e:?}");
            if let Some(r) = e.abzulegen() {
                assert!(
                    std::ptr::eq(r, e.rate()),
                    "abgelegt wird nicht die volle Erhebung"
                );
            }
        }
    }

    #[test]
    fn bestaetigungslauf_verlaengert_nicht() {
        // Abschnitte zwischen 2 und 40 ms: Das Mittel bleibt lange unsicher,
        // eine volle Erhebung liefe weiter. Der Bestätigungslauf hört nach
        // der Mindestdauer auf — das ist der ganze Punkt.
        let mut lang = false;
        let start = Instant::now();
        let r = bestaetigungslauf(|| {
            lang = !lang;
            std::thread::sleep(Duration::from_millis(if lang { 40 } else { 2 }));
            1.0
        });
        let gelaufen = start.elapsed();
        if r.reason == Abbruchgrund::Druck && r.samples == 0 {
            return;
        }
        assert_eq!(r.reason, Abbruchgrund::FesteDauer);
        assert!(
            !r.reason.brauchbar(),
            "ein Bestaetigungslauf gilt als Kalibrierung"
        );
        assert!(
            r.duration_ms >= 5000 && r.duration_ms < 5200,
            "Messdauer {} ms",
            r.duration_ms
        );
        // Mit Aufwärmen und ohne Abklingen: deutlich unter der Höchstdauer.
        assert!(gelaufen < Duration::from_secs(10), "gelaufen {gelaufen:?}");
        assert!(r.samples >= ABSCHNITTE_MIN, "nur {} Abschnitte", r.samples);
    }

    // --------------------------------------------- AC1: verschränkte Referenz

    fn verschraenkt(kalibriert: f64, referenz: f64) -> Verschraenkt {
        Verschraenkt {
            kalibrierung: rate(kalibriert),
            referenz: rate(referenz),
        }
    }

    #[test]
    fn verschraenkt_ac1_trifft_innerhalb_der_toleranz_und_nicht_darueber() {
        for (kalibriert, erwartet) in [
            (100.0, Ac1Urteil::Erfuellt),
            (105.0, Ac1Urteil::Erfuellt),
            (90.5, Ac1Urteil::Erfuellt),
            (109.5, Ac1Urteil::Erfuellt),
            (89.0, Ac1Urteil::NichtErfuellt),
            (111.0, Ac1Urteil::NichtErfuellt),
            (50.0, Ac1Urteil::NichtErfuellt),
        ] {
            let v = verschraenkt(kalibriert, 100.0).ac1();
            assert_eq!(v.urteil, erwartet, "kalibriert {kalibriert}");
            assert!(
                (v.referenz - 100.0).abs() < 1e-12,
                "Referenz {}",
                v.referenz
            );
            assert!(
                (v.abweichung - (kalibriert - 100.0) / 100.0).abs() < 1e-12,
                "Abweichung {} bei {kalibriert}",
                v.abweichung
            );
        }
    }

    #[test]
    fn verschraenkt_ac1_verlangt_kalibrierung_und_referenz() {
        // Beide Reihen müssen stabil sein: ein genaues Mittel gegen ein
        // ungenaues ist kein Vergleich. Die Abweichung steht trotzdem da.
        for (kal, refz) in [
            (Abbruchgrund::Hoechstdauer, Abbruchgrund::Stabil),
            (Abbruchgrund::Stabil, Abbruchgrund::Hoechstdauer),
            (Abbruchgrund::Drosselung, Abbruchgrund::Drosselung),
        ] {
            let mut v = verschraenkt(100.0, 100.0);
            v.kalibrierung.reason = kal;
            v.referenz.reason = refz;
            let a = v.ac1();
            assert_eq!(a.urteil, Ac1Urteil::KeineKalibrierung, "{kal:?} {refz:?}");
            assert!(a.abweichung.abs() < 1e-12, "Abweichung {}", a.abweichung);
        }
        // Gegenprobe: beide stabil ist erfüllt.
        assert_eq!(verschraenkt(100.0, 100.0).ac1().urteil, Ac1Urteil::Erfuellt);
    }

    #[test]
    fn verschraenkt_ac1_ohne_rate_ist_ungueltig() {
        for (k, r) in [
            (0.0, 100.0),
            (100.0, 0.0),
            (100.0, f64::NAN),
            (f64::INFINITY, 100.0),
            (100.0, -5.0),
        ] {
            assert_eq!(
                verschraenkt(k, r).ac1().urteil,
                Ac1Urteil::Ungueltig,
                "kalibriert {k}, Referenz {r}"
            );
        }
    }

    #[test]
    fn verschraenkt_wechselt_kalibrierung_und_referenz_ab() {
        // ABAB…, vom ersten Aufwärmabschnitt an, und jede Reihe gleich lang.
        let folge = std::cell::RefCell::new(Vec::<char>::new());
        let v = kalibriere_verschraenkt(
            || {
                folge.borrow_mut().push('A');
                std::thread::sleep(Duration::from_millis(10));
                10.0
            },
            || {
                folge.borrow_mut().push('B');
                std::thread::sleep(Duration::from_millis(10));
                10.0
            },
        );
        // Derselbe Ausweg wie bei `kalibriere_haelt_die_mindestdauer_ein`:
        // Fremdlast vor der Messung bricht ab, bevor gerechnet wird.
        if v.kalibrierung.reason == Abbruchgrund::Druck && v.kalibrierung.samples == 0 {
            assert!(folge.borrow().is_empty(), "gerechnet trotz Druck-Abbruch");
            return;
        }
        let folge = folge.into_inner();
        assert!(
            folge.len() >= 2 * ABSCHNITTE_MIN,
            "nur {} Aufrufe",
            folge.len()
        );
        assert_eq!(folge[0], 'A', "die Kalibrierung beginnt nicht");
        assert!(
            folge.windows(2).all(|w| w[0] != w[1]),
            "nicht abwechselnd: {}",
            folge.iter().collect::<String>()
        );
        assert_eq!(folge.last(), Some(&'B'), "endet mitten im Paar");
        assert_eq!(
            v.kalibrierung.samples, v.referenz.samples,
            "verschieden viele Abschnitte"
        );
        assert!(v.kalibrierung.samples >= ABSCHNITTE_MIN);
    }

    #[test]
    fn verschraenkt_hebt_eine_drift_auf() {
        // **Der Grund für die Verschränkung** (P1 auf node3, 28.10.): Die
        // Referenz davor und danach lag 10,7 % auseinander. Hier wird dieselbe
        // Arbeit während des Laufs stetig langsamer — wie eine Maschine, deren
        // Last steigt. Verschränkt sehen beide Reihen dieselbe Drift, und ihre
        // Mittel treffen sich; nacheinander gemessen lägen sie um die Drift
        // auseinander.
        let start = Instant::now();
        let arbeit = || {
            let faktor = 1.0 + start.elapsed().as_secs_f64() / 5.0;
            std::thread::sleep(Duration::from_secs_f64(0.010 * faktor));
            1.0
        };
        let v = kalibriere_verschraenkt(arbeit, arbeit);
        if v.kalibrierung.reason == Abbruchgrund::Druck && v.kalibrierung.samples == 0 {
            return;
        }
        // Positive Kontrolle: Die Drift ist im Lauf angekommen — die
        // Abschnitte streuen weit. Ohne sie prüfte der Fall gleiche Arbeit
        // unter gleichen Bedingungen und könnte nicht scheitern.
        assert!(
            v.kalibrierung.spread > 0.10,
            "keine Drift im Lauf, Streuung {}",
            v.kalibrierung.spread
        );
        let a = v.ac1();
        assert!(
            a.abweichung.abs() < 0.03,
            "verschränkte Mittel liegen auseinander: Abweichung {}",
            a.abweichung
        );
    }
}
