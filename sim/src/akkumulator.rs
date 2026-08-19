//! Durchgaenge ergaenzbar speichern statt fertig (Issue #967).
//!
//! # Warum kein Mittelwert
//!
//! **Ein Eintrag aus vier Durchgaengen ist wertlos, sobald zwoelf gebraucht
//! werden** — obwohl die vier bereits gerechnet sind. Wer einen Mittelwert
//! speichert, wirft die Moeglichkeit weg, darauf aufzubauen. Die gestaffelte
//! Stichprobe baut genau darauf: wenige Durchgaenge zuerst, mehr nur dort, wo
//! es noetig ist.
//!
//! Gespeichert wird deshalb ein **Akkumulator**: Zaehler, Summe, Quadratsumme,
//! Siege, Quantile — und die **Kennungen** der Durchgaenge.
//!
//! # Warum die Kennungen und nicht nur ihre Anzahl (AC2)
//!
//! Ohne sie ist die gepaarte Auswertung mit gemeinsamen Zufallszahlen
//! unmoeglich, und die hat deutlich weniger Streuung als zwei unabhaengige
//! Mittelwerte. Sie sind ausserdem der einzige Weg, eine **Doppelzaehlung** zu
//! erkennen: Zwei Pakete mit derselben Kennung duerfen nicht addiert werden,
//! und ohne die Kennungen faellt das niemandem auf.
//!
//! # Warum Festkomma (AC4)
//!
//! Mit Gleitkomma ist die Vereinigung **nicht** deterministisch: Kommen Pakete
//! anders geschnitten oder in anderer Reihenfolge zurueck, aendert sich das
//! letzte Bit der Summe — und damit womoeglich eine Abbruchentscheidung. Ein
//! ausgefallener Zusatzrechner wuerde das Ergebnis aendern.
//!
//! Das Issue laesst drei Wege zu; gebaut ist der **bevorzugte**: ein
//! Festkomma-Akkumulator. Ganzzahlige Addition ist assoziativ und kommutativ,
//! also ist die Vereinigung reihenfolgeunabhaengig **exakt** — nicht nur
//! meistens, und ohne dass jemand eine Reduktionsreihenfolge einhalten muss.
//!
//! > Das ist der Unterschied zu den beiden anderen Wegen: Sie machen die
//! > Reihenfolge zur **Voraussetzung**. Voraussetzungen sind Code, und Code hat
//! > Fehler. Hier gibt es nichts einzuhalten.
//!
//! # Was hier NICHT gebaut wird
//!
//! **Kein Speicher.** Dieses Modul baut den Typ; der kanonische
//! Ergebnisspeicher, der Akkumulatoren ablegt und wieder herausgibt, entsteht
//! im Auswertungs-Verteiler (#918) und nimmt den Typ von hier entgegen. Genau
//! deshalb muss er selbsttragend sein: vereinigbar, ergaenzbar und **ohne
//! Kenntnis des Speichers pruefbar**.
//!
//! Ebenfalls nicht hier: die gestaffelte Stichprobe (sie ist der Abnehmer) und
//! der dauerhafte Speicher auf Platte.

use core::fmt;

// ------------------------------------------------------------ Kennungen

/// Die Kennung eines einzelnen Durchgangs.
///
/// Im Simulator ist das der **Startwert** des Durchgangs: `balance::run`
/// vergibt `base_seed + offset + index`, und `Sample::seed` fuehrt ihn mit.
/// Ein Durchgang ist durch ihn vollstaendig bestimmt — das ist die Grundlage
/// dafuer, dass zwei Rechner denselben Durchgang meinen, wenn sie dieselbe
/// Zahl nennen.
pub type TrialId = u32;

/// Eine Menge von Durchgangskennungen als **Bereiche** (AC2b).
///
/// # Warum Bereiche und keine Liste
///
/// Bei tausenden Durchgaengen je Eintrag ist eine Liste sowohl im Speicher als
/// auch beim Vereinigen zu teuer — und dieser Typ wird von **jedem** Eintrag
/// des Ergebnisspeichers getragen, nicht nur von einigen.
///
/// Bereiche sind hier ausserdem die natuerliche Form: Der Laeufer vergibt
/// **fortlaufende** Kennungen (`base_seed + offset + index`), tausend
/// Durchgaenge sind also ein einziger Bereich statt tausend Eintraege.
///
/// # Zusicherungen
///
/// Die Bereiche sind jederzeit **sortiert, disjunkt und maximal verschmolzen**.
/// Zwei Mengen mit denselben Kennungen haben deshalb dieselbe Darstellung —
/// und das ist die Voraussetzung dafuer, dass der Fingerabdruck ueber sie
/// unabhaengig davon ist, in welcher Reihenfolge sie entstanden.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrialIdSet {
    /// Halboffene Bereiche `[start, ende)`, sortiert und disjunkt.
    bereiche: Vec<(TrialId, TrialId)>,
}

impl TrialIdSet {
    /// Die leere Menge.
    #[must_use]
    pub fn leer() -> Self {
        Self::default()
    }

    /// Aus einem halboffenen Bereich `[start, ende)`.
    ///
    /// Ein leerer oder verkehrter Bereich ergibt die leere Menge.
    #[must_use]
    pub fn aus_bereich(start: TrialId, ende: TrialId) -> Self {
        if start >= ende {
            return Self::leer();
        }
        Self {
            bereiche: vec![(start, ende)],
        }
    }

    /// Wie viele Kennungen enthalten sind.
    #[must_use]
    pub fn anzahl(&self) -> u64 {
        self.bereiche
            .iter()
            .map(|(a, b)| u64::from(*b) - u64::from(*a))
            .sum()
    }

    /// Ob die Menge leer ist.
    #[must_use]
    pub fn ist_leer(&self) -> bool {
        self.bereiche.is_empty()
    }

    /// Ob eine Kennung enthalten ist.
    #[must_use]
    pub fn enthaelt(&self, id: TrialId) -> bool {
        self.bereiche.iter().any(|(a, b)| id >= *a && id < *b)
    }

    /// Wie viele Bereiche die Darstellung braucht.
    ///
    /// Steht oeffentlich, damit ein Prueffall belegen kann, dass tausend
    /// fortlaufende Kennungen **einen** Bereich ergeben und nicht tausend.
    #[must_use]
    pub fn bereichszahl(&self) -> usize {
        self.bereiche.len()
    }

    /// Die Bereiche, in Ordnung.
    pub fn bereiche(&self) -> impl Iterator<Item = (TrialId, TrialId)> + '_ {
        self.bereiche.iter().copied()
    }

    /// Ob sich zwei Mengen ueberschneiden.
    ///
    /// **Die Frage, die eine Doppelzaehlung verhindert.** Zwei Pakete mit
    /// gemeinsamen Durchgaengen duerfen nicht addiert werden: Ihre Summen
    /// enthielten dieselben Kaempfe zweimal, und das Ergebnis saehe voellig
    /// plausibel aus.
    #[must_use]
    pub fn ueberschneidet(&self, other: &Self) -> bool {
        // Beide sind sortiert — ein Durchlauf genuegt.
        let (mut i, mut j) = (0, 0);
        while i < self.bereiche.len() && j < other.bereiche.len() {
            let (a0, a1) = self.bereiche[i];
            let (b0, b1) = other.bereiche[j];
            if a1 <= b0 {
                i += 1;
            } else if b1 <= a0 {
                j += 1;
            } else {
                return true;
            }
        }
        false
    }

    /// Vereinigt zwei Mengen.
    ///
    /// Das Ergebnis ist wieder sortiert, disjunkt und verschmolzen — auch dann,
    /// wenn die Eingaben aneinandergrenzen (`[0,4)` und `[4,16)` ergeben
    /// `[0,16)`, nicht zwei Bereiche). Ohne das Verschmelzen haetten „vier
    /// ergaenzt um zwoelf" und „sechzehn von vorn" verschiedene Darstellungen,
    /// und AC3 waere nicht erfuellbar.
    pub fn vereinige(&mut self, other: &Self) {
        let mut alle: Vec<(TrialId, TrialId)> = self
            .bereiche
            .iter()
            .chain(other.bereiche.iter())
            .copied()
            .collect();
        alle.sort_unstable();

        let mut aus: Vec<(TrialId, TrialId)> = Vec::with_capacity(alle.len());
        for (start, ende) in alle {
            match aus.last_mut() {
                // Grenzt an oder ueberlappt: verschmelzen.
                Some(letzter) if start <= letzter.1 => {
                    if ende > letzter.1 {
                        letzter.1 = ende;
                    }
                }
                _ => aus.push((start, ende)),
            }
        }
        self.bereiche = aus;
    }
}

impl fmt::Display for TrialIdSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let teile: Vec<String> = self
            .bereiche
            .iter()
            .map(|(a, b)| format!("{a}..{b}"))
            .collect();
        write!(f, "[{}]", teile.join(","))
    }
}

// ------------------------------------------------------------ Festkomma

/// Nachkommastellen des Festkomma-Akkumulators.
///
/// Sechs Stellen: Die gemessenen Groessen des Simulators sind heute
/// **ganzzahlig** (Schaden, Millisekunden), die Skala kostet dort also nichts
/// und wird exakt. Sie steht trotzdem da, damit ein spaeterer Gleitkomma-Wert
/// nicht die Zusicherung bricht, sondern sichtbar gerundet wird.
pub const SKALA: i128 = 1_000_000;

/// Ein Messwert in Festkomma-Darstellung.
///
/// # Warum ein eigener Typ und kein `i128`
///
/// Damit an der Verwendungsstelle steht, dass die Zahl **skaliert** ist. Eine
/// nackte `i128` haette irgendwann jemand mit einem rohen Schadenswert addiert,
/// und die Skala waere um sechs Zehnerpotenzen daneben — plausibel aussehend
/// und falsch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Festwert(pub i128);

impl Festwert {
    /// Aus einer Ganzzahl — **exakt**, ohne jede Rundung.
    #[must_use]
    pub const fn von_ganzzahl(wert: u32) -> Self {
        Self(wert as i128 * SKALA)
    }

    /// Aus einer Gleitkommazahl — **gerundet** auf die Skala.
    ///
    /// Hier und nur hier entsteht ein Rundungsfehler. Er entsteht **einmal je
    /// Messwert** und danach nie wieder: Alles Weitere ist ganzzahlige
    /// Addition. Genau darum geht es in AC4 — der Fehler darf nicht davon
    /// abhaengen, wie die Pakete geschnitten waren.
    ///
    /// Nicht endliche Werte ergeben 0; ein `NaN` im Akkumulator waere sonst
    /// ansteckend und der ganze Eintrag verloren.
    #[must_use]
    pub fn von_gleitkomma(wert: f64) -> Self {
        if !wert.is_finite() {
            return Self(0);
        }
        // `round` statt `as`-Abschneiden: Abschneiden waere in beide Richtungen
        // verzerrt, je nach Vorzeichen.
        Self((wert * SKALA as f64).round() as i128)
    }

    /// Zurueck als Gleitkommazahl — nur fuer die **Ausgabe**.
    ///
    /// Nie als Zwischenschritt einer Rechnung, die wieder in den Akkumulator
    /// geht: Damit waere die Exaktheit weg.
    #[must_use]
    pub fn als_gleitkomma(self) -> f64 {
        self.0 as f64 / SKALA as f64
    }
}

// ------------------------------------------------------------ Histogramm

/// Ein exaktes Histogramm mit fester Klassenbreite (AC5).
///
/// # Warum bestimmt und nicht „optional"
///
/// Eine Quantilstruktur, die je nach Lage anders gewaehlt wird, ist zwischen
/// zwei Rechnern nicht vereinigbar. Hier ist sie festgelegt: feste Untergrenze,
/// feste Klassenbreite, fester Ueberlauf-Eimer.
///
/// # Wann sie EXAKT ist, und wann nur beschraenkt genau
///
/// Bei Klassenbreite 1 und ganzzahligen Messwerten ist jedes Quantil **exakt**
/// — jede Klasse enthaelt genau einen Wert. Bei groesserer Breite ist der
/// Fehler durch die Breite beschraenkt, und [`ExactHistogram::quantil_fehler`]
/// gibt ihn heraus, statt ihn zu verschweigen.
///
/// Der Ueberlauf wird **mitgezaehlt und ausgewiesen**, nicht still gedeckelt —
/// dasselbe Vorgehen wie im Diagnose-Histogramm ([`crate::diagnostics`]): Ein
/// Wert oberhalb der Obergrenze ist selbst ein Befund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactHistogram {
    /// Untergrenze der ersten Klasse.
    untergrenze: u64,
    /// Breite jeder Klasse, in Einheiten des Messwerts.
    breite: u64,
    /// Besetzung je Klasse.
    klassen: Vec<u64>,
    /// Werte oberhalb der letzten Klasse.
    ueberlauf: u64,
    /// Werte unterhalb der Untergrenze.
    unterlauf: u64,
}

impl ExactHistogram {
    /// Ein leeres Histogramm.
    ///
    /// # Panics
    ///
    /// Wenn `breite` oder `klassen` null ist — ein Histogramm ohne Klassen
    /// koennte nichts aufnehmen, und Breite 0 waere eine Division durch null.
    /// Beides ist ein Programmierfehler, kein Laufzeitfall.
    #[must_use]
    pub fn neu(untergrenze: u64, breite: u64, klassen: usize) -> Self {
        assert!(breite > 0, "Klassenbreite 0 ist keine Einteilung");
        assert!(klassen > 0, "ein Histogramm ohne Klassen nimmt nichts auf");
        Self {
            untergrenze,
            breite,
            klassen: vec![0; klassen],
            ueberlauf: 0,
            unterlauf: 0,
        }
    }

    /// Ob zwei Histogramme dieselbe Einteilung haben.
    ///
    /// Nur dann sind sie vereinigbar. Verschiedene Einteilungen zu addieren
    /// waere die stille Variante des Fehlers, den dieses Modul verhindert.
    #[must_use]
    pub fn gleiche_einteilung(&self, other: &Self) -> bool {
        self.untergrenze == other.untergrenze
            && self.breite == other.breite
            && self.klassen.len() == other.klassen.len()
    }

    /// Traegt einen Messwert ein.
    pub fn trage_ein(&mut self, wert: u64) {
        if wert < self.untergrenze {
            self.unterlauf += 1;
            return;
        }
        let index = ((wert - self.untergrenze) / self.breite) as usize;
        match self.klassen.get_mut(index) {
            Some(eimer) => *eimer += 1,
            None => self.ueberlauf += 1,
        }
    }

    /// Wie viele Werte eingetragen sind, einschliesslich Ueber- und Unterlauf.
    #[must_use]
    pub fn anzahl(&self) -> u64 {
        self.klassen.iter().sum::<u64>() + self.ueberlauf + self.unterlauf
    }

    /// Werte oberhalb der letzten Klasse.
    #[must_use]
    pub const fn ueberlauf(&self) -> u64 {
        self.ueberlauf
    }

    /// Werte unterhalb der ersten Klasse.
    #[must_use]
    pub const fn unterlauf(&self) -> u64 {
        self.unterlauf
    }

    /// Die groesstmoegliche Abweichung eines Quantils, in Einheiten des
    /// Messwerts.
    ///
    /// Bei Breite 1 ist sie **0** — das Quantil ist dann exakt. Sonst liegt der
    /// wahre Wert irgendwo in der zurueckgegebenen Klasse, also hoechstens
    /// `breite - 1` daneben.
    ///
    /// Gibt `None`, wenn Werte im Ueber- oder Unterlauf liegen: Dann ist die
    /// Abweichung **unbeschraenkt**, und eine Zahl auszugeben waere eine
    /// Behauptung ohne Deckung.
    #[must_use]
    pub const fn quantil_fehler(&self) -> Option<u64> {
        if self.ueberlauf > 0 || self.unterlauf > 0 {
            return None;
        }
        Some(self.breite - 1)
    }

    /// Das Quantil zu `p` (0.0 bis 1.0) als Untergrenze seiner Klasse.
    ///
    /// Gibt `None` bei leerem Histogramm. Werte im Ueber- oder Unterlauf
    /// zaehlen fuer die Position mit — sonst verschoebe sich das Quantil
    /// stillschweigend.
    #[must_use]
    pub fn quantil(&self, p: f64) -> Option<u64> {
        let gesamt = self.anzahl();
        if gesamt == 0 {
            return None;
        }
        let p = p.clamp(0.0, 1.0);
        // Rang ab 1, damit p = 0 den kleinsten Wert trifft.
        let ziel = ((p * gesamt as f64).ceil() as u64).max(1);

        let mut gesehen = self.unterlauf;
        if gesehen >= ziel {
            return Some(self.untergrenze);
        }
        for (i, anzahl) in self.klassen.iter().enumerate() {
            gesehen += anzahl;
            if gesehen >= ziel {
                return Some(self.untergrenze + i as u64 * self.breite);
            }
        }
        Some(self.untergrenze + self.klassen.len() as u64 * self.breite)
    }

    /// Vereinigt zwei Histogramme.
    ///
    /// # Errors
    ///
    /// [`Vereinigungsfehler::EinteilungVerschieden`], wenn die Einteilungen
    /// nicht uebereinstimmen.
    pub fn vereinige(&mut self, other: &Self) -> Result<(), Vereinigungsfehler> {
        if !self.gleiche_einteilung(other) {
            return Err(Vereinigungsfehler::EinteilungVerschieden);
        }
        for (eigen, fremd) in self.klassen.iter_mut().zip(other.klassen.iter()) {
            *eigen += *fremd;
        }
        self.ueberlauf += other.ueberlauf;
        self.unterlauf += other.unterlauf;
        Ok(())
    }
}

// ------------------------------------------------------------ Der Akkumulator

/// Warum eine Vereinigung abgelehnt wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vereinigungsfehler {
    /// Beide Seiten enthalten denselben Durchgang.
    ///
    /// **Der teuerste stille Fehler dieses Moduls, wenn er durchginge:** Die
    /// Summen enthielten dieselben Kaempfe zweimal, der Mittelwert bliebe
    /// plausibel, und nur die Streuung waere falsch.
    DurchgaengeUeberschneidenSich,
    /// Die Quantilstrukturen sind verschieden eingeteilt.
    EinteilungVerschieden,
}

impl fmt::Display for Vereinigungsfehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DurchgaengeUeberschneidenSich => {
                write!(f, "die Durchgangskennungen ueberschneiden sich")
            }
            Self::EinteilungVerschieden => {
                write!(f, "die Quantilstrukturen sind verschieden eingeteilt")
            }
        }
    }
}

impl std::error::Error for Vereinigungsfehler {}

/// Ein ergaenzbarer Satz Durchgaenge — **kein Mittelwert** (AC1).
///
/// # Was er traegt
///
/// | Feld | wofuer |
/// | --- | --- |
/// | `trial_ids` | welche Durchgaenge, nicht nur wie viele (AC2) |
/// | `count` | wie viele |
/// | `sum`, `sum_squares` | Mittelwert und Streuung **spaeter** ausrechenbar |
/// | `win_count` | wie oft gewonnen |
/// | `quantiles` | die Verteilung, nicht nur ihr Mittel |
///
/// Ein Mittelwert steht hier **nirgends**. Er ist eine Ableitung
/// ([`TrialAccumulator::mittelwert`]) und wird bei jedem Aufruf neu gerechnet —
/// so kann er nicht veralten, wenn Durchgaenge dazukommen.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialAccumulator {
    /// Welche Durchgaenge enthalten sind.
    pub trial_ids: TrialIdSet,
    /// Wie viele Durchgaenge.
    pub count: u64,
    /// Summe der Messwerte, in Festkomma.
    pub sum: Festwert,
    /// Summe der Quadrate, in Festkomma **zum Quadrat** (Skala mal Skala).
    ///
    /// Eine eigene Skala, damit auch das Quadrat exakt bleibt. Wer sie auf
    /// [`SKALA`] zurueckrechnete, verlore genau die Stellen, wegen derer der
    /// Akkumulator ganzzahlig ist.
    pub sum_squares: i128,
    /// Wie oft der Durchgang gewonnen wurde.
    pub win_count: u64,
    /// Die Verteilung.
    pub quantiles: ExactHistogram,
}

impl TrialAccumulator {
    /// Ein leerer Akkumulator mit der gegebenen Quantil-Einteilung.
    #[must_use]
    pub fn neu(quantiles: ExactHistogram) -> Self {
        Self {
            trial_ids: TrialIdSet::leer(),
            count: 0,
            sum: Festwert(0),
            sum_squares: 0,
            win_count: 0,
            quantiles,
        }
    }

    /// Traegt einen einzelnen Durchgang ein.
    ///
    /// Der Messwert ist ganzzahlig, weil der Simulator ganzzahlig misst
    /// (Schaden, Millisekunden). Damit ist der Eintrag exakt und die spaetere
    /// Vereinigung reihenfolgeunabhaengig.
    ///
    /// # Errors
    ///
    /// [`Vereinigungsfehler::DurchgaengeUeberschneidenSich`], wenn diese
    /// Kennung schon enthalten ist — sonst zaehlte derselbe Kampf zweimal.
    pub fn trage_ein(
        &mut self,
        id: TrialId,
        wert: u32,
        gewonnen: bool,
    ) -> Result<(), Vereinigungsfehler> {
        if self.trial_ids.enthaelt(id) {
            return Err(Vereinigungsfehler::DurchgaengeUeberschneidenSich);
        }
        self.trial_ids
            .vereinige(&TrialIdSet::aus_bereich(id, id.saturating_add(1)));
        self.count += 1;
        let fest = Festwert::von_ganzzahl(wert);
        self.sum = Festwert(self.sum.0 + fest.0);
        self.sum_squares += fest.0 * fest.0;
        if gewonnen {
            self.win_count += 1;
        }
        self.quantiles.trage_ein(u64::from(wert));
        Ok(())
    }

    /// Vereinigt einen zweiten Akkumulator hinein (AC3, AC4).
    ///
    /// **Reihenfolgeunabhaengig exakt:** Alle Felder sind ganzzahlig, und
    /// ganzzahlige Addition ist assoziativ und kommutativ. `a.vereinige(b)`
    /// und `b.vereinige(a)` liefern denselben Fingerabdruck, ebenso jede
    /// Klammerung ueber drei und mehr Pakete.
    ///
    /// # Errors
    ///
    /// [`Vereinigungsfehler::DurchgaengeUeberschneidenSich`] bei gemeinsamen
    /// Durchgaengen, [`Vereinigungsfehler::EinteilungVerschieden`] bei
    /// verschiedenen Quantil-Einteilungen. In beiden Faellen bleibt `self`
    /// **unveraendert** — eine halb vollzogene Vereinigung waere schlimmer als
    /// gar keine.
    pub fn vereinige(&mut self, other: &Self) -> Result<(), Vereinigungsfehler> {
        if self.trial_ids.ueberschneidet(&other.trial_ids) {
            return Err(Vereinigungsfehler::DurchgaengeUeberschneidenSich);
        }

        // Absichtlich doppelt: `ExactHistogram::vereinige` prueft dasselbe noch
        // einmal. Hier steht es, damit die Ablehnung **vor** jeder Aenderung
        // faellt und `self` unberuehrt bleibt; dort, damit der Histogramm-Typ
        // fuer sich allein sicher ist. Gemessen mit der Mutationsprobe: Faellt
        // nur eine der beiden, faengt die andere den Fall — belegen laesst sich
        // die Deckung deshalb nur, wenn man beide entfernt.
        if !self.quantiles.gleiche_einteilung(&other.quantiles) {
            return Err(Vereinigungsfehler::EinteilungVerschieden);
        }
        // Ab hier kann nichts mehr fehlschlagen.
        self.quantiles.vereinige(&other.quantiles)?;
        self.trial_ids.vereinige(&other.trial_ids);
        self.count += other.count;
        self.sum = Festwert(self.sum.0 + other.sum.0);
        self.sum_squares += other.sum_squares;
        self.win_count += other.win_count;
        Ok(())
    }

    /// Der Mittelwert — **abgeleitet, nicht gespeichert**.
    #[must_use]
    pub fn mittelwert(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        Some(self.sum.als_gleitkomma() / self.count as f64)
    }

    /// Die Streuung der Einzelwerte (Stichprobe, mit n−1).
    #[must_use]
    pub fn streuung(&self) -> Option<f64> {
        if self.count < 2 {
            return None;
        }
        let n = self.count as f64;
        let skala2 = (SKALA * SKALA) as f64;
        let summe = self.sum.0 as f64 / SKALA as f64;
        let quadrate = self.sum_squares as f64 / skala2;
        let varianz = (quadrate - summe * summe / n) / (n - 1.0);
        Some(varianz.max(0.0).sqrt())
    }

    /// Der Anteil gewonnener Durchgaenge.
    #[must_use]
    pub fn siegquote(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        Some(self.win_count as f64 / self.count as f64)
    }

    /// Ein Fingerabdruck ueber den **exakten** Inhalt (AC4).
    ///
    /// # Warum FNV-1a ueber Ganzzahlen und nicht ueber `to_bits`
    ///
    /// Er darf nur von dem abhaengen, WAS enthalten ist — nicht davon, in
    /// welcher Reihenfolge oder in welchen Paketen es hereinkam. Alle
    /// eingehenden Groessen sind ganzzahlig, und die Kennungen liegen in einer
    /// eindeutigen, verschmolzenen Bereichsform vor. Damit ist gleicher Inhalt
    /// gleicher Fingerabdruck, ohne jede Toleranz.
    ///
    /// Genau das ist die Abnahme von #967: Zwei Laeufe mit vertauschter
    /// Paketreihenfolge muessen denselben Wert liefern.
    #[must_use]
    pub fn fingerabdruck(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;
        let mut h = OFFSET;
        let mut fuettere = |wert: u128| {
            for byte in wert.to_le_bytes() {
                h ^= u64::from(byte);
                h = h.wrapping_mul(PRIME);
            }
        };

        fuettere(u128::from(self.count));
        fuettere(self.sum.0 as u128);
        fuettere(self.sum_squares as u128);

        fuettere(u128::from(self.win_count));

        for (a, b) in self.trial_ids.bereiche() {
            fuettere(u128::from(a));
            fuettere(u128::from(b));
        }
        fuettere(u128::from(self.quantiles.untergrenze));
        fuettere(u128::from(self.quantiles.breite));
        for eimer in &self.quantiles.klassen {
            fuettere(u128::from(*eimer));
        }
        fuettere(u128::from(self.quantiles.ueberlauf));
        fuettere(u128::from(self.quantiles.unterlauf));
        h
    }
}
