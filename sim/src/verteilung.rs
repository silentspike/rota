//! Modell-exakte Krit-Verteilung und zählerbasierter Zufall (Issues #915, #965).
//!
//! # Was hier entsteht, und was bewusst NICHT
//!
//! Dieses Modul trägt die Verfahren für den Fall, dass der kritische Treffer
//! **wirklich würfelt**: eine exakte Verteilungs-Dynamikprogrammierung und eine
//! gestaffelte Stichprobe. Beides läuft **neben** dem Kampf, nicht darin — aus
//! demselben Grund wie [`crate::analytisch`]: Der Kampf und das Spiel teilen
//! ihren Zufall, und Auflage 1 verlangt, dass jede Prüfdatei bitgleich bleibt.
//!
//! Gemessen am Bestand: Drei der 86 Paritäts-Aufzeichnungen würfeln wirklich
//! (`worn.purity = 375`, also Krit-Chance 0,375), und `worn-values.json` prüft
//! `expected.randomDraws = 25` ausdrücklich. Ein anderer Zufall im Kampfpfad
//! bräche sie beim ersten Lauf.
//!
//! # Die ganzzahlige Schwelle (AC5)
//!
//! Das Issue verlangt, Wahrscheinlichkeiten über **ganzzahlige** Schwellen zu
//! entscheiden statt über Gleitkomma-Vergleiche:
//!
//! ```text
//! random_u32 < probability_threshold_u32
//! ```
//!
//! Der Grund ist nicht Geschwindigkeit, sondern Bitgleichheit über
//! Hauptprozessor, Vektorpfad und Grafikeinheit. Ein Gleitkomma-Vergleich ist
//! genau die Stelle, an der ein Grafikpfad still abweicht.
//!
//! **Die Umrechnung ist exakt, und das ist keine Selbstverständlichkeit.**
//! `next_f64()` liefert `x / 2³²` mit ganzzahligem `x < 2³²`; dieser Quotient
//! ist in `f64` exakt darstellbar (53 Bit Mantisse reichen für 32 Bit Zähler
//! und eine Zweierpotenz im Nenner). Damit gilt
//!
//! ```text
//! x / 2³² < p   ⟺   x < p · 2³²   ⟺   x < ⌈p · 2³²⌉
//! ```
//!
//! Die letzte Umformung gilt, weil `x` ganzzahlig ist. Ob `p · 2³²` seinerseits
//! exakt gerechnet wird, hängt an `p` — und genau das prüft
//! `schwelle_trifft_denselben_ausgang_wie_der_gleitkomma_vergleich`.

mod numerik;
pub use numerik::{MassenNachweis, NumerikFehler, QuantilGrenze, StatistikGrenze};

/// Die ganzzahlige Schwelle zu einer Wahrscheinlichkeit.
///
/// Gibt den Wert `t` zurück, für den `random_u32 < t` genau dann gilt, wenn der
/// bisherige Vergleich `next_f64() < p` zutrifft.
///
/// Die Ränder sind Sonderfälle und **nicht** gerundet: `p <= 0` ergibt 0 (kein
/// Wert liegt darunter), `p >= 1` ergibt `2³²` als `u64` (jeder `u32` liegt
/// darunter). Beides entspricht den Kurzschlüssen in
/// [`crate::rng::RandomStream::chance`], die dort **nicht** würfeln.
#[must_use]
pub fn schwelle(p: f64) -> u64 {
    const ZWEI_HOCH_32: f64 = 4_294_967_296.0;
    if p <= 0.0 {
        return 0;
    }
    if p >= 1.0 {
        return ZWEI_HOCH_32 as u64;
    }
    // `ceil`, nicht `round`: Bei exakt aufgehendem Produkt darf der Wert
    // `p · 2³²` selbst NICHT mehr treffen — `x / 2³² < p` ist echt kleiner.
    (p * ZWEI_HOCH_32).ceil() as u64
}

/// Trifft der Wurf `wert` die Wahrscheinlichkeit — ganzzahlig entschieden?
#[must_use]
pub fn trifft(wert: u32, schwelle: u64) -> bool {
    u64::from(wert) < schwelle
}

// ------------------------------------------------ Zählerbasierter Zufall (AC4)

/// Woraus ein Wurf stammt.
///
/// **Getrennte Bereiche, weil ein gemeinsamer Zähler koppelt:** Zwei
/// Zufallsquellen desselben Zaubers, Auslöser **und** kritischer Treffer am
/// selben Treffer, Boss- und Legendär-Zufall — sie alle würden sich sonst
/// gegenseitig verschieben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quellbereich {
    /// Kritischer Treffer.
    Krit,
    /// Auslöser eines Effekts.
    Ausloeser,
    /// Zufall des Bosses.
    Boss,
    /// Zufall eines legendären Effekts.
    Legendaer,
    /// Zielauswahl.
    Zielwahl,
}

impl Quellbereich {
    /// Die Kennzahl im Schlüssel.
    ///
    /// # Kein Auffangzweig
    ///
    /// Ein neuer Bereich bricht hier die Übersetzung. Ein `_ => 0` wäre bequem
    /// und würde ihn mit einem bestehenden **koppeln** — also genau den Fehler
    /// bauen, gegen den die Trennung gerichtet ist.
    #[must_use]
    pub const fn kennzahl(self) -> i32 {
        match self {
            Quellbereich::Krit => 1,
            Quellbereich::Ausloeser => 2,
            Quellbereich::Boss => 3,
            Quellbereich::Legendaer => 4,
            Quellbereich::Zielwahl => 5,
        }
    }
}

/// Die drei Versionen, die in **jedem** Schlüssel stecken (AC4).
///
/// Ohne sie ließe sich ein Lauf von heute nicht von einem nach einer
/// Algorithmus-Änderung unterscheiden — beide lieferten Zahlen, und niemand
/// sähe, dass sie aus verschiedenen Welten stammen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Versionen {
    /// Version des Misch-Algorithmus.
    pub algorithmus: u16,
    /// Version des numerischen Vertrags.
    pub vertrag: u16,
    /// Version der Wahrscheinlichkeits-Abbildung.
    pub abbildung: u16,
}

/// Der Schlüssel eines einzelnen Wurfs (Issue #915, AC4).
///
/// **Elf Felder plus drei Versionen, und keines ist Zierde.** Ein Schlüssel aus
/// Boss, Durchgang, Zauber, Wirkung und Treffer — also fünf — koppelt Würfe,
/// die getrennt sein müssen: mehrere Akteure, mehrere Ziele, zwei Quellen
/// desselben Zaubers, verzögerte Wirkungen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wurfschluessel {
    /// Startwert des ganzen Laufs.
    pub lauf_startwert: u32,
    /// Kennung der Begegnung (Boss, Schwierigkeit, Aufbau).
    pub begegnung: u32,
    /// Welcher Durchgang.
    pub durchgang: u32,
    /// Aus welchem Bereich der Wurf stammt.
    pub bereich: Quellbereich,
    /// Welcher Effekt.
    pub effekt: u32,
    /// Wer wirkt.
    pub akteur: u32,
    /// Auf wen.
    pub ziel: u32,
    /// Der wievielte Zauber.
    pub zauber_ordinal: u32,
    /// Das wievielte Ereignis.
    pub ereignis_ordinal: u32,
    /// Der wievielte Treffer.
    pub treffer_ordinal: u32,
    /// Die drei Versionen.
    pub versionen: Versionen,
}

impl Wurfschluessel {
    /// Der Wurf zu diesem Schlüssel — **allein aus dem Schlüssel**.
    ///
    /// Kein Zustand, keine Historie: Derselbe Schlüssel liefert immer denselben
    /// Wert, und ein zusätzlicher Wurf anderswo verschiebt nichts. Genau darauf
    /// steht der Beweis in [`Zustand`], dass der DP ohne
    /// Zufalls-Fortsetzungszustand auskommt.
    ///
    /// Gemischt wird mit [`crate::checksum::mix_int`] — derselben FNV-1a-Stufe,
    /// die auch die Zustands-Prüfsumme benutzt. Eine zweite Mischfunktion wäre
    /// eine zweite Stelle, an der Simulator und Spiel auseinanderlaufen können.
    #[must_use]
    pub fn wurf(&self) -> u32 {
        use crate::checksum::{mix_int, FNV_OFFSET};
        let mut h = FNV_OFFSET;
        h = mix_int(h, self.lauf_startwert as i32);
        h = mix_int(h, self.begegnung as i32);
        h = mix_int(h, self.durchgang as i32);
        h = mix_int(h, self.bereich.kennzahl());
        h = mix_int(h, self.effekt as i32);
        h = mix_int(h, self.akteur as i32);
        h = mix_int(h, self.ziel as i32);
        h = mix_int(h, self.zauber_ordinal as i32);
        h = mix_int(h, self.ereignis_ordinal as i32);
        h = mix_int(h, self.treffer_ordinal as i32);
        h = mix_int(h, i32::from(self.versionen.algorithmus));
        h = mix_int(h, i32::from(self.versionen.vertrag));
        mix_int(h, i32::from(self.versionen.abbildung))
    }

    /// Trifft dieser Wurf die Wahrscheinlichkeit `p`?
    #[must_use]
    pub fn kritet(&self, p: f64) -> bool {
        trifft(self.wurf(), schwelle(p))
    }
}

// ------------------------------------------------------- Verteilungs-DP (AC1)

/// Ein Treffer, wie ihn der DP sieht.
///
/// Mehr braucht er nicht: Was aus dem Treffer wird, hängt nur am Grundschaden
/// und daran, ob er kritet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Treffer {
    /// Grundschaden ohne Krit.
    pub ohne_krit: u32,
    /// Schaden mit Krit — **fertig gerundet**, nicht als Faktor.
    ///
    /// Der Faktor gehört nicht hierher: `compute_damage` rundet je Treffer, und
    /// wer hier einen Faktor führte, müsste die Rundung nachbauen. Beide Werte
    /// kommen aus derselben Formel wie im Kampf.
    pub mit_krit: u32,
}

/// Der Zustandsschlüssel des DP (Issue #915, AC1).
///
/// # Warum der Zufalls-Fortsetzungszustand FEHLT — und warum das erlaubt ist
///
/// AC1 verlangt ihn im Schlüssel **oder** den Beweis, dass alle künftigen
/// Zähler aus dem zusammengeführten Zustand ableitbar sind. Hier gilt das
/// zweite, und der Grund ist der zählerbasierte Zufall:
///
/// Ein Wurf hängt allein an seinem **Schlüssel** — bei einer Krit-Entscheidung
/// also an der Stelle `(Durchgang, Effekt, Treffer-Ordinal)`. Er hängt **nicht**
/// daran, wie viele Würfe vorher stattgefunden haben. Zwei Pfade, die dieselbe
/// DP-Stufe erreichen, haben damit denselben nächsten Wurf-Schlüssel, egal
/// welchen Weg sie genommen haben.
///
/// > Bei einem **zustandsbasierten** Strom wäre das falsch: Dort verschiebt
/// > jede zusätzliche Ziehung alles Folgende, und zwei Pfade mit gleicher
/// > Trefferzahl, aber verschiedener Zahl an Kurzschlüssen, würden verschieden
/// > weiterwürfeln. Genau deshalb hängt AC1 an AC4.
///
/// Die DP-Stufe ist der Treffer-Index, und der ist keine Zustandskomponente,
/// sondern die Schleifenvariable — deshalb steht er auch nicht im Schlüssel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Zustand {
    /// Kumulierter Schaden bis hierher.
    pub schaden: u64,
    /// In welcher Phase der Boss ist.
    pub phase: u32,
}

/// Eine Wahrscheinlichkeitsverteilung über Zuständen.
///
/// **`BTreeMap` und nicht `HashMap`** — GAME-DESIGN 18.4 verlangt eine feste
/// Reihenfolge ohne Hash-Container. Die Summationsreihenfolge ist damit über
/// Läufe und Rechner hinweg dieselbe, und das ist bei Gleitkomma keine
/// Feinheit, sondern der Unterschied zwischen reproduzierbar und nicht.
#[derive(Debug, Clone, Default)]
pub struct Verteilung {
    masse: std::collections::BTreeMap<Zustand, f64>,
    // None for Default (an empty, uncertified distribution). Only transitions
    // from the exact initial mass construct a numerical certificate.
    numerik: Option<numerik::Fortpflanzung>,
}

impl Verteilung {
    /// Die Startverteilung: alles auf einem Zustand.
    #[must_use]
    pub fn am_anfang() -> Self {
        let mut masse = std::collections::BTreeMap::new();
        masse.insert(
            Zustand {
                schaden: 0,
                phase: 0,
            },
            1.0,
        );
        Self {
            masse,
            numerik: Some(numerik::Fortpflanzung::default()),
        }
    }

    /// Wie viele Zustände geführt werden.
    #[must_use]
    pub fn zustaende(&self) -> usize {
        self.masse.len()
    }

    /// Binary64 point estimate; [`Self::massen_nachweis`] checks conservation
    /// against the propagated error instead of a fixed epsilon.
    #[must_use]
    pub fn gesamtmasse(&self) -> f64 {
        self.masse.values().sum()
    }

    /// Die Wahrscheinlichkeit, mindestens Phase `k` erreicht zu haben
    /// (AC6, Nachweis 4).
    #[must_use]
    pub fn phase_erreicht(&self, k: u32) -> f64 {
        self.masse
            .iter()
            .filter(|(z, _)| z.phase >= k)
            .map(|(_, m)| m)
            .sum()
    }

    /// Binary64 point estimate. Use [`Self::erwarteter_schaden_grenze`] for a
    /// certified absolute-error bound (including integer conversion).
    #[must_use]
    pub fn erwarteter_schaden(&self) -> f64 {
        self.masse.iter().map(|(z, m)| m * z.schaden as f64).sum()
    }

    /// Binary64 point estimate. Use [`Self::varianz_grenze`] for its enclosure.
    #[must_use]
    pub fn varianz(&self) -> f64 {
        let mittel = self.erwarteter_schaden();
        self.masse
            .iter()
            .map(|(z, m)| {
                let d = z.schaden as f64 - mittel;
                m * d * d
            })
            .sum()
    }

    /// Einen Treffer auf die ganze Verteilung anwenden.
    ///
    /// Jeder Zustand verzweigt in zwei: kritet (`p`) und kritet nicht (`1 - p`).
    /// Zustände, die danach übereinstimmen, werden zusammengelegt — und genau
    /// das ist der Gewinn gegenüber der Aufzählung.
    ///
    /// `phasen_bei` sind die kumulierten Schadensschwellen, an denen die Phase
    /// wechselt. Sie werden **nach** dem Treffer ausgewertet, so wie der Kampf
    /// es tut.
    #[must_use]
    pub fn nach_treffer(&self, treffer: Treffer, p: f64, phasen_bei: &[u64]) -> Verteilung {
        self.nach_treffer_geprueft(treffer, p, phasen_bei)
            .expect("valid probabilities, damage ranges and binary64 DP contract")
    }

    /// Checked transition preserving the old point masses and their order.
    /// `p` is the exact real value of the supplied binary64 number, not the
    /// quantized probability `schwelle(p) / 2^32` of a modeled uniform u32
    /// draw. Neither uniformity nor independence of an actual RNG is proved.
    pub fn nach_treffer_geprueft(
        &self,
        treffer: Treffer,
        p: f64,
        phasen_bei: &[u64],
    ) -> Result<Verteilung, NumerikFehler> {
        numerik::wahrscheinlichkeit(p)?;
        let alt = self.massen_nachweis()?;
        u32::try_from(phasen_bei.len()).map_err(|_| NumerikFehler::Zahlenbereich)?;
        let mut nachweis = numerik::Fortpflanzung {
            fehler: alt.l1_fehler,
            schritte: alt
                .schritte
                .checked_add(1)
                .ok_or(NumerikFehler::Zahlenbereich)?,
            uebergaenge: alt.zustandsuebergaenge,
        };
        let komplement = 1.0 - p;
        let komplement_fehler = if p == 0.0 || p == 1.0 {
            0.0
        } else {
            numerik::rundungsfehler(komplement)?
        };
        let mut neu: std::collections::BTreeMap<Zustand, f64> = std::collections::BTreeMap::new();
        for (z, m) in &self.masse {
            // A true stochastic transition is L1-nonexpansive. Only the
            // rounded complement and the local multiply/merge operations add
            // error; see the induction in docs/KRIT-DP-NUMERIK.md.
            nachweis.fehler =
                numerik::add_oben(nachweis.fehler, numerik::mul_oben(*m, komplement_fehler)?)?;
            for (schaden, anteil) in [
                (u64::from(treffer.mit_krit), p),
                (u64::from(treffer.ohne_krit), komplement),
            ] {
                if anteil == 0.0 {
                    continue;
                }
                nachweis.uebergaenge = nachweis
                    .uebergaenge
                    .checked_add(1)
                    .ok_or(NumerikFehler::Zahlenbereich)?;
                let summe = z
                    .schaden
                    .checked_add(schaden)
                    .ok_or(NumerikFehler::Zahlenbereich)?;
                let phase = phasen_bei.iter().filter(|&&s| summe >= s).count() as u32;
                let ziel = neu
                    .entry(Zustand {
                        schaden: summe,
                        phase,
                    })
                    .or_insert(0.0);
                let produkt = m * anteil;
                let merged = *ziel + produkt;
                let produkt_fehler = if *m == 0.0 || anteil == 1.0 {
                    0.0
                } else {
                    numerik::rundungsfehler(produkt)?
                };
                let merge_fehler = if *ziel == 0.0 || produkt == 0.0 {
                    0.0
                } else {
                    numerik::rundungsfehler(merged)?
                };
                nachweis.fehler = numerik::add_oben(
                    nachweis.fehler,
                    numerik::add_oben(produkt_fehler, merge_fehler)?,
                )?;
                *ziel = merged;
            }
        }
        let ergebnis = Verteilung {
            masse: neu,
            numerik: Some(nachweis),
        };
        ergebnis.massen_nachweis()?;
        Ok(ergebnis)
    }
}

/// Die ganze Trefferfolge durchrechnen.
#[must_use]
pub fn verteilung_ueber(treffer: &[Treffer], p: f64, phasen_bei: &[u64]) -> Verteilung {
    verteilung_ueber_geprueft(treffer, p, phasen_bei)
        .expect("valid probabilities, damage ranges and binary64 DP contract")
}

/// Checked fixed-hit Bernoulli distribution; does not change combat/RNG code.
pub fn verteilung_ueber_geprueft(
    treffer: &[Treffer],
    p: f64,
    phasen_bei: &[u64],
) -> Result<Verteilung, NumerikFehler> {
    numerik::wahrscheinlichkeit(p)?;
    numerik::umgebung()?;
    u32::try_from(phasen_bei.len()).map_err(|_| NumerikFehler::Zahlenbereich)?;
    treffer.iter().try_fold(Verteilung::am_anfang(), |v, &t| {
        v.nach_treffer_geprueft(t, p, phasen_bei)
    })
}

#[cfg(test)]
mod zaehler_tests {
    use super::*;

    fn basis() -> Wurfschluessel {
        Wurfschluessel {
            lauf_startwert: 12345,
            begegnung: 7,
            durchgang: 3,
            bereich: Quellbereich::Krit,
            effekt: 11,
            akteur: 0,
            ziel: 1,
            zauber_ordinal: 4,
            ereignis_ordinal: 9,
            treffer_ordinal: 2,
            versionen: Versionen {
                algorithmus: 1,
                vertrag: 1,
                abbildung: 1,
            },
        }
    }

    /// **AC4: JEDES Feld verändert den Wurf.**
    ///
    /// Ein Feld, das nichts bewirkt, ist kein Feld — es sähe aus wie Trennung
    /// und wäre Kopplung. Geprüft wird deshalb einzeln, nicht als Menge.
    #[test]
    fn jedes_feld_des_schluessels_wirkt() {
        let b = basis();
        let ausgang = b.wurf();

        // **Die Vollständigkeit hängt am Compiler, nicht an der Zahl unten.**
        //
        // `geprueft == 13` zählt, wie viele Felder dieser Test ANFASST — nicht,
        // wie viele die Struktur HAT. Gemessen: Ein 14. Feld, das nicht in
        // `wurf()` eingeht, ließ alle zwölf Fälle grün durchlaufen. Der Wächter
        // prüfte die bestehenden Felder gut und wuchs nur nicht mit.
        //
        // Dieses Muster hat **kein `..`**. Kommt ein Feld dazu, bricht der BAU
        // hier — bevor irgendein Lauf grün melden kann. Die zweite Ebene ist
        // mit aufgezählt, damit auch eine neue Version nicht durchrutscht.
        //
        // **Und hier ist die Grenze dieser Deckung:** Sie erzwingt nicht, dass
        // jemand auch eine `pruefe!`-Zeile schreibt. Wer `probe_feld: _`
        // ergänzt, kommt durch. Der Unterschied ist, dass er es dann bewusst
        // tut, statt es zu übersehen — mehr leistet sie nicht, und wer mehr
        // hineinliest, hält sie für einen Beweis statt für einen Anstoß.
        let Wurfschluessel {
            lauf_startwert: _,
            begegnung: _,
            durchgang: _,
            bereich: _,
            effekt: _,
            akteur: _,
            ziel: _,
            zauber_ordinal: _,
            ereignis_ordinal: _,
            treffer_ordinal: _,
            versionen:
                Versionen {
                    algorithmus: _,
                    vertrag: _,
                    abbildung: _,
                },
        } = b;

        let mut abweichend = Vec::new();
        let mut geprueft = 0;

        macro_rules! pruefe {
            ($name:literal, $aend:expr) => {{
                let mut k = b;
                #[allow(clippy::redundant_closure_call)]
                ($aend)(&mut k);
                geprueft += 1;
                if k.wurf() == ausgang {
                    abweichend.push($name);
                }
            }};
        }

        pruefe!("lauf_startwert", |k: &mut Wurfschluessel| k
            .lauf_startwert +=
            1);
        pruefe!("begegnung", |k: &mut Wurfschluessel| k.begegnung += 1);
        pruefe!("durchgang", |k: &mut Wurfschluessel| k.durchgang += 1);
        pruefe!("bereich", |k: &mut Wurfschluessel| k.bereich =
            Quellbereich::Boss);
        pruefe!("effekt", |k: &mut Wurfschluessel| k.effekt += 1);
        pruefe!("akteur", |k: &mut Wurfschluessel| k.akteur += 1);
        pruefe!("ziel", |k: &mut Wurfschluessel| k.ziel += 1);
        pruefe!("zauber_ordinal", |k: &mut Wurfschluessel| k
            .zauber_ordinal +=
            1);
        pruefe!("ereignis_ordinal", |k: &mut Wurfschluessel| k
            .ereignis_ordinal +=
            1);
        pruefe!("treffer_ordinal", |k: &mut Wurfschluessel| k
            .treffer_ordinal +=
            1);
        pruefe!("algorithmus", |k: &mut Wurfschluessel| k
            .versionen
            .algorithmus +=
            1);
        pruefe!("vertrag", |k: &mut Wurfschluessel| k.versionen.vertrag += 1);
        pruefe!("abbildung", |k: &mut Wurfschluessel| k
            .versionen
            .abbildung += 1);

        assert_eq!(
            geprueft, 13,
            "Trägerzusicherung: es wurden wirklich alle 13 Felder angefasst"
        );
        assert!(
            abweichung_leer(&abweichend),
            "diese Felder ändern den Wurf NICHT: {abweichend:?}"
        );
        assert!(
            geprueft > 5,
            "AC4 verlangt mehr als fünf Felder — es sind {geprueft}"
        );
        println!("{geprueft} Felder, alle wirksam");
    }

    fn abweichung_leer(v: &[&str]) -> bool {
        v.is_empty()
    }

    /// **Der Kern des AC1-Beweises: der Wurf hängt NICHT an der Historie.**
    ///
    /// Zweimal derselbe Schlüssel ergibt denselben Wert — auch wenn dazwischen
    /// beliebig viele andere Würfe stattfanden. Bei einem zustandsbasierten
    /// Strom wäre das falsch, und dann müsste der Fortsetzungszustand in den
    /// DP-Schlüssel.
    #[test]
    fn derselbe_schluessel_liefert_denselben_wurf_unabhaengig_von_fremden_wuerfen() {
        let k = basis();
        let erst = k.wurf();

        // Dazwischen hundert fremde Würfe aus anderen Bereichen und Stellen.
        let mut fremd = 0u32;
        for i in 0..100u32 {
            let mut f = basis();
            f.bereich = Quellbereich::Boss;
            f.treffer_ordinal = i;
            fremd = fremd.wrapping_add(f.wurf());
        }

        assert_eq!(
            k.wurf(),
            erst,
            "der Wurf hat sich durch fremde Ziehungen verschoben"
        );
        assert_ne!(
            fremd, 0,
            "Trägerzusicherung: die fremden Würfe fanden statt"
        );
        println!("100 fremde Würfe, Ausgangswurf unverändert: {erst}");
    }

    /// Zwei Quellen am **selben** Treffer dürfen sich nicht koppeln.
    #[test]
    fn krit_und_ausloeser_am_selben_treffer_sind_getrennt() {
        let mut a = basis();
        a.bereich = Quellbereich::Krit;
        let mut b = basis();
        b.bereich = Quellbereich::Ausloeser;
        assert_ne!(
            a.wurf(),
            b.wurf(),
            "Krit und Auslöser am selben Treffer liefern denselben Wurf — genau \
             die Kopplung, gegen die AC4 den Bereich einführt"
        );
    }
}

#[cfg(test)]
mod dp_tests {
    use super::*;

    /// Die vollständige Aufzählung: alle 2^n Krit-Folgen einzeln.
    ///
    /// **Der Bezugspunkt, gegen den der DP bewiesen wird** (AC6, Nachweis 1).
    /// Bewusst so naiv wie möglich geschrieben — sie darf nichts mit dem DP
    /// teilen außer dem Ergebnis, sonst prüfte sie ihn gegen sich selbst.
    fn aufzaehlung(treffer: &[Treffer], p: f64, phasen_bei: &[u64]) -> Verteilung {
        let n = treffer.len();
        assert!(n <= 20, "2^{n} Folgen wären nicht mehr vertretbar");
        let mut masse: std::collections::BTreeMap<Zustand, f64> = std::collections::BTreeMap::new();

        for maske in 0u32..(1u32 << n) {
            let mut schaden = 0u64;
            let mut wahrscheinlichkeit = 1.0f64;
            for (i, t) in treffer.iter().enumerate() {
                if maske & (1 << i) == 0 {
                    schaden += u64::from(t.ohne_krit);
                    wahrscheinlichkeit *= 1.0 - p;
                } else {
                    schaden += u64::from(t.mit_krit);
                    wahrscheinlichkeit *= p;
                }
            }
            let phase = phasen_bei.iter().filter(|&&s| schaden >= s).count() as u32;
            *masse.entry(Zustand { schaden, phase }).or_insert(0.0) += wahrscheinlichkeit;
        }
        Verteilung {
            masse,
            ..Verteilung::default()
        }
    }

    fn gleiche_treffer(n: usize, ohne: u32, mit: u32) -> Vec<Treffer> {
        vec![
            Treffer {
                ohne_krit: ohne,
                mit_krit: mit
            };
            n
        ]
    }

    /// **AC6, Nachweis 1: der DP trifft die vollständige Aufzählung.**
    ///
    /// Verglichen wird Zustand für Zustand, nicht nur der Erwartungswert — zwei
    /// verschiedene Verteilungen können denselben Mittelwert haben.
    #[test]
    fn der_dp_trifft_die_vollstaendige_aufzaehlung() {
        // **Verschiedene Grundwerte, aber teils gleiche DIFFERENZEN.** Daran
        // hängt, ob der DP überhaupt etwas zusammenlegen kann: Zwei Pfade
        // landen genau dann im selben Zustand, wenn sich ihre Krit-Zuschläge
        // zur selben Summe ergänzen.
        //
        // Eine frühere Fassung nahm lauter verschiedene Differenzen (4, 7, 20,
        // 21, 50). Dann kollidiert nichts, der DP führt alle 32 Folgen einzeln
        // — und die Trägerzusicherung „hat zusammengelegt" fiel, BEVOR der
        // eigentliche Vergleich lief. Der Fall war rot, ohne den DP geprüft zu
        // haben.
        let treffer = vec![
            Treffer {
                ohne_krit: 7,
                mit_krit: 17,
            },
            Treffer {
                ohne_krit: 13,
                mit_krit: 23,
            },
            Treffer {
                ohne_krit: 40,
                mit_krit: 60,
            },
            Treffer {
                ohne_krit: 41,
                mit_krit: 61,
            },
            Treffer {
                ohne_krit: 99,
                mit_krit: 109,
            },
        ];
        // **Die Schwellen müssen INNERHALB der möglichen Summen liegen.**
        // Kleinste Summe 7+13+40+41+99 = 200, größte 17+23+60+61+109 = 270.
        // Schwellen darunter würden immer erreicht, darüber nie — beides prüft
        // den Phasenteil nicht. Die Trägerzusicherung unten fängt genau das,
        // und sie hat es in der ersten Fassung auch getan.
        let phasen = [215u64, 245];
        let p = 0.25;

        // Exercise the retained incremental point API against enumeration.
        let dp = treffer.iter().fold(Verteilung::am_anfang(), |v, &t| {
            v.nach_treffer(t, p, &phasen)
        });
        let roh = aufzaehlung(&treffer, p, &phasen);

        assert!(
            dp.zustaende() < 1 << treffer.len(),
            "Trägerzusicherung: der DP hat wirklich zusammengelegt ({} Zustände \
             statt {})",
            dp.zustaende(),
            1 << treffer.len()
        );
        assert!(
            roh.phase_erreicht(1) > 0.0 && roh.phase_erreicht(1) < 1.0,
            "Trägerzusicherung: die Phasenschwelle wird MANCHMAL erreicht — sonst \
             prüft der Vergleich den Phasenteil gar nicht"
        );

        assert_eq!(
            dp.zustaende(),
            roh.zustaende(),
            "verschiedene Zahl von Zuständen: {} gegen {}",
            dp.zustaende(),
            roh.zustaende()
        );
        for (z, m) in &roh.masse {
            let d = dp.masse.get(z).copied().unwrap_or(0.0);
            assert!(
                (d - m).abs() < 1e-12,
                "Zustand {z:?}: DP {d}, Aufzählung {m}"
            );
        }
        println!(
            "{} Zustände statt {} Folgen, Masse {:.12}",
            dp.zustaende(),
            1 << treffer.len(),
            dp.gesamtmasse()
        );
    }

    /// **AC6, Nachweis 2: die Masse bleibt 1.**
    #[test]
    fn die_wahrscheinlichkeitsmasse_bleibt_eins() {
        for p in [0.0, 0.1, 0.25, 0.5, 0.75, 1.0] {
            let v = verteilung_ueber(&gleiche_treffer(12, 10, 15), p, &[50, 100, 150]);
            let masse = v.gesamtmasse();
            assert!(
                (masse - 1.0).abs() < 1e-12,
                "bei p = {p} ist die Masse {masse}"
            );
        }
    }

    /// **AC6, Nachweis 3: Erwartungswert und Varianz treffen die geschlossene
    /// Formel.**
    ///
    /// Bei lauter gleichen Treffern ist die Zahl der Krits binomialverteilt:
    ///
    /// ```text
    /// E = n · (p·mit + (1-p)·ohne)
    /// Var = n · p·(1-p) · (mit - ohne)²
    /// ```
    #[test]
    fn erwartungswert_und_varianz_treffen_die_binomialformel() {
        let (n, ohne, mit) = (14usize, 10u32, 15u32);
        let p = 0.3;
        // Ohne Phasen: Sie ändern die Verteilung nicht, nur die Beschriftung.
        let v = verteilung_ueber(&gleiche_treffer(n, ohne, mit), p, &[]);

        let e_formel = n as f64 * (p * f64::from(mit) + (1.0 - p) * f64::from(ohne));
        let var_formel = n as f64 * p * (1.0 - p) * f64::from(mit - ohne).powi(2);

        assert!(
            (v.erwarteter_schaden() - e_formel).abs() < 1e-9,
            "Erwartungswert: DP {}, Formel {e_formel}",
            v.erwarteter_schaden()
        );
        assert!(
            (v.varianz() - var_formel).abs() < 1e-9,
            "Varianz: DP {}, Formel {var_formel}",
            v.varianz()
        );
        assert!(
            var_formel > 0.0,
            "Trägerzusicherung: die Varianz ist nicht null — sonst prüfte der \
             Vergleich zwei Nullen"
        );
        println!("E {e_formel}, Var {var_formel}, Zustände {}", v.zustaende());
    }

    /// **AC6, Nachweis 4: die Phasenwahrscheinlichkeit stimmt einzeln.**
    ///
    /// Gegengerechnet über die Aufzählung, nicht über eine zweite Formel — die
    /// Aufzählung ist der Bezugspunkt aus Nachweis 1.
    #[test]
    fn die_phasenwahrscheinlichkeit_stimmt_je_stufe() {
        let treffer = gleiche_treffer(10, 10, 15);
        let phasen = [60u64, 100, 140];
        let p = 0.4;

        let dp = verteilung_ueber(&treffer, p, &phasen);
        let roh = aufzaehlung(&treffer, p, &phasen);

        let mut wirklich_gestuft = 0;
        for k in 0..=phasen.len() as u32 {
            let a = dp.phase_erreicht(k);
            let b = roh.phase_erreicht(k);
            assert!((a - b).abs() < 1e-12, "Phase {k}: DP {a}, Aufzählung {b}");
            if a > 0.0 && a < 1.0 {
                wirklich_gestuft += 1;
            }
        }
        assert!(
            wirklich_gestuft >= 2,
            "Trägerzusicherung: mindestens zwei Phasen werden NICHT immer und \
             nicht nie erreicht ({wirklich_gestuft}) — sonst prüft der Fall nur \
             Nullen und Einsen"
        );
        println!("{wirklich_gestuft} Phasen mit echter Streuung");
    }

    /// **AC6, Nachweis 5: Monte-Carlo — und zwar ZULETZT.**
    ///
    /// Das Issue ist an dieser Stelle deutlich: „Ein Monte-Carlo-Vergleich
    /// allein ist keine Abnahme. Er ist eine Plausibilitätsprüfung: Zwei
    /// Verfahren können im Rahmen der Stichprobenstreuung übereinstimmen und
    /// beide falsch sein."
    ///
    /// Er steht hier trotzdem, weil er etwas prüft, das die Aufzählung **nicht**
    /// kann: Die Aufzählung teilt mit dem DP das Modell — beide gewichten
    /// dieselben zwei Ausgänge mit demselben `p`. Ein Denkfehler im Modell
    /// selbst fiele beiden gleichzeitig zu. Der Monte-Carlo-Lauf zieht dagegen
    /// **wirklich**, mit dem zählerbasierten Zufall aus AC4.
    #[test]
    fn monte_carlo_bestaetigt_den_dp_im_fehlerrahmen() {
        let treffer = gleiche_treffer(10, 10, 15);
        let p = 0.3;
        let dp = verteilung_ueber(&treffer, p, &[]);

        // Gezogen wird mit dem zählerbasierten Zufall: je Durchgang und Treffer
        // ein eigener Schlüssel, kein fortlaufender Strom.
        let durchgaenge = 20_000u32;
        let mut summe = 0.0f64;
        let mut quadrate = 0.0f64;
        for d in 0..durchgaenge {
            let mut schaden = 0u64;
            for (i, t) in treffer.iter().enumerate() {
                let k = Wurfschluessel {
                    lauf_startwert: 4242,
                    begegnung: 1,
                    durchgang: d,
                    bereich: Quellbereich::Krit,
                    effekt: 1,
                    akteur: 0,
                    ziel: 1,
                    zauber_ordinal: 0,
                    ereignis_ordinal: 0,
                    treffer_ordinal: i as u32,
                    versionen: Versionen {
                        algorithmus: 1,
                        vertrag: 1,
                        abbildung: 1,
                    },
                };
                schaden += u64::from(if k.kritet(p) { t.mit_krit } else { t.ohne_krit });
            }
            summe += schaden as f64;
            quadrate += (schaden as f64).powi(2);
        }

        let n = f64::from(durchgaenge);
        let mittel = summe / n;
        let varianz = (quadrate / n) - mittel * mittel;
        let stderr = (varianz / n).sqrt();
        let abstand = (mittel - dp.erwarteter_schaden()).abs();

        assert!(
            varianz > 0.0,
            "Trägerzusicherung: die gezogenen Durchgänge STREUEN — ohne Streuung \
             wäre der Vergleich wertlos"
        );
        assert!(
            abstand < 4.0 * stderr,
            "Monte-Carlo {mittel:.3} gegen DP {:.3}, Abstand {abstand:.3}, \
             erlaubt {:.3} (vier Standardfehler)",
            dp.erwarteter_schaden(),
            4.0 * stderr
        );
        println!(
            "MC {mittel:.3} ± {stderr:.3}, DP {:.3}, Abstand {abstand:.3} = {:.2} stderr",
            dp.erwarteter_schaden(),
            abstand / stderr
        );
    }

    /// Die Gegenprobe zum ganzen DP: Eine falsche Zusammenlegung fiele auf.
    ///
    /// Ohne sie bliebe offen, ob der Vergleich mit der Aufzählung überhaupt
    /// etwas unterscheidet.
    #[test]
    fn ohne_phasen_faellt_die_zustandszahl_deutlich() {
        let treffer = gleiche_treffer(14, 10, 15);
        let ohne_phasen = verteilung_ueber(&treffer, 0.3, &[]);
        assert_eq!(
            ohne_phasen.zustaende(),
            15,
            "bei 14 gleichen Treffern gibt es 15 mögliche Krit-Zahlen — mehr \
             Zustände hieße, dass nicht zusammengelegt wird"
        );

        // **Der Gewinn wird GEMESSEN, nicht behauptet.** Eine frühere Fassung
        // verglich hier zwei Konstanten miteinander; clippy hat sie als
        // „assertion is always true" gemeldet, und sie war es auch — der Fall
        // konnte nicht scheitern.
        let folgen = 1usize << treffer.len();
        let faktor = folgen / ohne_phasen.zustaende();
        assert!(
            faktor > 100,
            "Faktor nur {faktor} ({folgen} Folgen gegen {} Zustände) — dann wäre \
             die Zusammenlegung den Aufwand nicht wert",
            ohne_phasen.zustaende()
        );
        println!(
            "{folgen} Folgen gegen {} Zustände, Faktor {faktor}",
            ohne_phasen.zustaende()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wie der Gleitkomma-Vergleich heute entscheidet — der Bezugspunkt.
    ///
    /// Bewusst nachgebildet statt `chance` gerufen: `chance` zieht selbst und
    /// nähme uns die Kontrolle über den Wurfwert. Die eine Zeile ist wörtlich
    /// die aus `rng.rs`, und der Fall unten prüft beide gegeneinander über den
    /// **ganzen** Wertebereich, nicht an Stichproben.
    fn gleitkomma_vergleich(wert: u32, p: f64) -> bool {
        f64::from(wert) / 4_294_967_296.0 < p
    }

    /// **AC5: die ganzzahlige Schwelle entscheidet identisch.**
    ///
    /// Geprüft über alle Krit-Chancen, die aus PURITY entstehen können
    /// (`purity / 1000` für 0..=1000), und je Chance über die Wurfwerte an den
    /// **Rändern** der Entscheidung. Dort und nur dort kann eine Umrechnung
    /// kippen — in der Mitte des Bereichs stimmen beide trivial überein.
    #[test]
    fn schwelle_trifft_denselben_ausgang_wie_der_gleitkomma_vergleich() {
        let mut geprueft = 0u64;
        let mut abweichungen = Vec::new();

        for purity in 0u32..=1000 {
            let p = f64::from(purity) / 1000.0;
            let t = schwelle(p);

            // Die Randwerte der Entscheidung plus ein paar feste Punkte.
            let mut kandidaten = vec![0u32, 1, u32::MAX, u32::MAX - 1];
            for delta in 0..3u64 {
                if t >= delta {
                    let unten = t - delta;
                    if unten <= u64::from(u32::MAX) {
                        kandidaten.push(unten as u32);
                    }
                }
                let oben = t + delta;
                if oben <= u64::from(u32::MAX) {
                    kandidaten.push(oben as u32);
                }
            }

            for wert in kandidaten {
                let ganzzahlig = trifft(wert, t);
                let gleitkomma = gleitkomma_vergleich(wert, p);
                geprueft += 1;
                if ganzzahlig != gleitkomma {
                    abweichungen.push((purity, wert, ganzzahlig, gleitkomma));
                }
            }
        }

        assert!(
            geprueft > 5_000,
            "Trägerzusicherung: es wurden wirklich viele Paare geprüft ({geprueft})"
        );
        assert!(
            abweichungen.is_empty(),
            "{} Abweichungen, die ersten drei: {:?}",
            abweichungen.len(),
            &abweichungen[..abweichungen.len().min(3)]
        );
        println!("{geprueft} Paare geprüft, 0 Abweichungen");
    }

    /// Die Ränder verhalten sich wie die Kurzschlüsse in `chance`.
    #[test]
    fn an_den_raendern_wird_gar_nicht_entschieden() {
        assert_eq!(schwelle(0.0), 0, "bei p = 0 trifft kein Wurf");
        assert!(!trifft(0, schwelle(0.0)), "auch die Null nicht");

        let voll = schwelle(1.0);
        assert!(trifft(u32::MAX, voll), "bei p = 1 trifft jeder Wurf");
        assert!(trifft(0, voll));

        // Und negative oder übergroße Werte kippen nicht in die Gegenrichtung.
        assert_eq!(schwelle(-0.5), 0);
        assert_eq!(schwelle(1.5), schwelle(1.0));
    }

    /// **Die Gegenprobe: eine falsche Rundung fiele auf.**
    ///
    /// Ohne diesen Fall bliebe offen, ob der Vergleich oben überhaupt etwas
    /// unterscheidet — er wäre auch grün, wenn `schwelle` und der
    /// Gleitkomma-Weg beide dieselbe falsche Antwort gäben.
    #[test]
    fn mit_round_statt_ceil_gaebe_es_abweichungen() {
        const ZWEI_HOCH_32: f64 = 4_294_967_296.0;
        let mut abweichungen = 0;

        for purity in 1u32..1000 {
            let p = f64::from(purity) / 1000.0;
            let richtig = schwelle(p);
            let gerundet = (p * ZWEI_HOCH_32).round() as u64;
            if richtig != gerundet {
                // Genau am Grenzwert entscheiden die beiden verschieden.
                let wert = gerundet.min(u64::from(u32::MAX)) as u32;
                if trifft(wert, richtig) != trifft(wert, gerundet) {
                    abweichungen += 1;
                }
            }
        }

        assert!(
            abweichungen > 0,
            "round und ceil verhielten sich in KEINEM Fall verschieden — dann \
             prüft der Fall oben die Rundung gar nicht"
        );
        println!("round statt ceil: {abweichungen} unterscheidbare Fälle");
    }
}
