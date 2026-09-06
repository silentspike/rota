//! Conservative binary64 enclosures for the existing Bernoulli DP (#965).
//!
//! The proof and the distinction from RNG/model error are in
//! `docs/KRIT-DP-NUMERIK.md`. No mass is pruned or renormalized.

use super::{Verteilung, Zustand};

/// A failed precondition is not an approximate numerical certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumerikFehler {
    /// Probability/quantile must be finite and in [0, 1].
    Wahrscheinlichkeit,
    /// An empty or externally constructed distribution has no DP provenance.
    KeinNachweis,
    /// An integer counter, damage sum, or floating bound overflowed.
    Zahlenbereich,
    /// Runtime arithmetic does not preserve binary64 subnormal/tie behavior.
    Rechenumgebung,
    /// Observed total mass contradicts the propagated bound.
    Massenerhalt,
}

impl std::fmt::Display for NumerikFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Krit-DP numeric contract: {self:?}")
    }
}

impl std::error::Error for NumerikFehler {}

/// The exact model statistic is in [untergrenze, obergrenze].
/// `wert` retains the existing binary64 point computation.
#[derive(Debug, Clone, Copy)]
pub struct StatistikGrenze {
    /// Existing binary64 point calculation, without normalization.
    pub wert: f64,
    /// Lower endpoint for the exact model statistic.
    pub untergrenze: f64,
    /// Upper endpoint for the exact model statistic.
    pub obergrenze: f64,
    /// Upper bound on |wert - exact model statistic|, rounded outward.
    pub absoluter_fehler: f64,
}

/// Damage coordinates, not a probability tolerance reused as a damage error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantilGrenze {
    /// Smallest damage not ruled out by the upper CDF enclosure.
    pub unterer_schaden: u64,
    /// Damage at which the lower CDF enclosure certifies the quantile,
    /// or the maximum reachable damage when mass uncertainty remains.
    pub oberer_schaden: u64,
}

/// Distribution error includes every stage, not just the final state count.
#[derive(Debug, Clone, Copy)]
pub struct MassenNachweis {
    /// Conservative upper bound on the sum of all absolute state-mass errors.
    pub l1_fehler: f64,
    /// Number of fixed-hit Bernoulli stages included in this certificate.
    pub schritte: u64,
    /// Executed nonzero-probability branches across every preceding stage.
    pub zustandsuebergaenge: u64,
    /// Number of distinct states in the resulting distribution.
    pub zustaende: usize,
    /// Observed total mass with summation and propagation errors enclosed.
    pub gesamtmasse: StatistikGrenze,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Fortpflanzung {
    pub(super) fehler: f64,
    pub(super) schritte: u64,
    pub(super) uebergaenge: u64,
}

pub(super) fn wahrscheinlichkeit(p: f64) -> Result<(), NumerikFehler> {
    if p.is_finite() && (0.0..=1.0).contains(&p) {
        Ok(())
    } else {
        Err(NumerikFehler::Wahrscheinlichkeit)
    }
}

pub(super) fn umgebung() -> Result<(), NumerikFehler> {
    // black_box prevents constant folding from concealing FTZ/DAZ or a changed
    // rounding mode in the executing thread. Rust's primitive-operation
    // semantics remain the compile-time contract (no algebraic/fast-math ops).
    use std::hint::black_box as b;
    let tiny = b(f64::from_bits(1));
    if (b(tiny) + b(tiny)).to_bits() != 2
        || (b(tiny) * b(1.0)).to_bits() != 1
        || (b(f64::MIN_POSITIVE) * b(0.5)).to_bits() != 1_u64 << 51
        || b(1.0) + b(f64::EPSILON / 2.0) != 1.0
        || b(1.0_f64.next_up()) + b(f64::EPSILON / 2.0) != 1.0_f64.next_up().next_up()
    {
        return Err(NumerikFehler::Rechenumgebung);
    }
    Ok(())
}

fn endlich(v: f64) -> Result<f64, NumerikFehler> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(NumerikFehler::Zahlenbereich)
    }
}

pub(super) fn add_oben(a: f64, b: f64) -> Result<f64, NumerikFehler> {
    if a == 0.0 {
        return endlich(b);
    }
    if b == 0.0 {
        return endlich(a);
    }
    endlich((a + b).next_up())
}

pub(super) fn mul_oben(a: f64, b: f64) -> Result<f64, NumerikFehler> {
    if a == 0.0 || b == 0.0 {
        return Ok(0.0);
    }
    endlich((a * b).next_up())
}

/// One upward spacing encloses the absolute rounding error of a finite,
/// nonnegative basic-operation result, also at zero/subnormal results.
pub(super) fn rundungsfehler(v: f64) -> Result<f64, NumerikFehler> {
    if v < 0.0 {
        return Err(NumerikFehler::Zahlenbereich);
    }
    endlich(v.next_up() - v)
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Intervall {
    unten: f64,
    oben: f64,
}

impl Intervall {
    fn punkt(v: f64) -> Self {
        Self { unten: v, oben: v }
    }

    fn ganzzahl(v: u64) -> Self {
        let f = v as f64;
        if v <= 1_u64 << 53 {
            return Self::punkt(f);
        }
        Self {
            unten: f.next_down(),
            oben: f.next_up(),
        }
    }

    fn add(self, other: Self) -> Result<Self, NumerikFehler> {
        Ok(Self {
            unten: if self.unten == 0.0 {
                other.unten
            } else if other.unten == 0.0 {
                self.unten
            } else {
                (self.unten + other.unten).next_down().max(0.0)
            },
            oben: add_oben(self.oben, other.oben)?,
        })
    }

    fn mul(self, other: Self) -> Result<Self, NumerikFehler> {
        Ok(Self {
            unten: (self.unten * other.unten).next_down().max(0.0),
            oben: mul_oben(self.oben, other.oben)?,
        })
    }

    fn erweitert(self, fehler: f64) -> Result<Self, NumerikFehler> {
        Ok(Self {
            unten: (self.unten - fehler).next_down().max(0.0),
            oben: add_oben(self.oben, fehler)?,
        })
    }

    fn statistik(self, wert: f64) -> Result<StatistikGrenze, NumerikFehler> {
        endlich(wert)?;
        endlich(self.unten)?;
        endlich(self.oben)?;
        if self.unten > self.oben {
            return Err(NumerikFehler::Massenerhalt);
        }
        let abstand = (wert - self.unten).abs().max((self.oben - wert).abs());
        Ok(StatistikGrenze {
            wert,
            untergrenze: self.unten,
            obergrenze: self.oben,
            absoluter_fehler: if abstand == 0.0 {
                0.0
            } else {
                endlich(abstand.next_up())?
            },
        })
    }
}

impl Verteilung {
    fn fortpflanzung(&self) -> Result<Fortpflanzung, NumerikFehler> {
        umgebung()?;
        if self.masse.is_empty() {
            return Err(NumerikFehler::KeinNachweis);
        }
        let n = self.numerik.ok_or(NumerikFehler::KeinNachweis)?;
        if !n.fehler.is_finite()
            || n.fehler < 0.0
            || self.masse.values().any(|&m| !m.is_finite() || m < 0.0)
        {
            return Err(NumerikFehler::Zahlenbereich);
        }
        Ok(n)
    }

    fn summenintervall(
        &self,
        event: impl Fn(&Zustand) -> bool,
    ) -> Result<Intervall, NumerikFehler> {
        self.masse
            .iter()
            .filter(|(z, _)| event(z))
            .try_fold(Intervall::punkt(0.0), |s, (_, &m)| {
                s.add(Intervall::punkt(m))
            })
    }

    /// Existing point masses, in deterministic state order. No normalization.
    pub fn massen(&self) -> impl Iterator<Item = (Zustand, f64)> + '_ {
        self.masse.iter().map(|(&z, &m)| (z, m))
    }

    /// Checked mass conservation and an L1 bound on all individual masses.
    pub fn massen_nachweis(&self) -> Result<MassenNachweis, NumerikFehler> {
        let n = self.fortpflanzung()?;
        let gesamt = self.summenintervall(|_| true)?.erweitert(n.fehler)?;
        if gesamt.unten > 1.0 || gesamt.oben < 1.0 {
            return Err(NumerikFehler::Massenerhalt);
        }
        Ok(MassenNachweis {
            l1_fehler: n.fehler,
            schritte: n.schritte,
            zustandsuebergaenge: n.uebergaenge,
            zustaende: self.zustaende(),
            gesamtmasse: gesamt.statistik(self.gesamtmasse())?,
        })
    }

    /// Any fixed event on the DP state, including phase, victory, and CDF.
    /// The event is evaluated once per state, also for stateful closures.
    pub fn wahrscheinlichkeit_grenze(
        &self,
        event: impl Fn(&Zustand) -> bool,
    ) -> Result<StatistikGrenze, NumerikFehler> {
        let n = self.massen_nachweis()?;
        let mut summe = Intervall::punkt(0.0);
        let mut punkt = 0.0;
        for (z, &m) in &self.masse {
            if event(z) {
                summe = summe.add(Intervall::punkt(m))?;
                punkt += m;
            }
        }
        let mut grenzen = summe.erweitert(n.l1_fehler)?;
        grenzen.oben = grenzen.oben.min(1.0);
        grenzen.statistik(punkt)
    }

    /// Probability of reaching phase k in this fixed-hit model.
    pub fn phase_erreicht_grenze(&self, k: u32) -> Result<StatistikGrenze, NumerikFehler> {
        self.wahrscheinlichkeit_grenze(|z| z.phase >= k)
    }

    /// Probability of damage >= hp; not a certificate of a full combat model.
    pub fn siegwahrscheinlichkeit_grenze(&self, hp: u64) -> Result<StatistikGrenze, NumerikFehler> {
        self.wahrscheinlichkeit_grenze(|z| z.schaden >= hp)
    }

    fn moment(&self, quadrat: bool) -> Result<Intervall, NumerikFehler> {
        let n = self.massen_nachweis()?;
        let mut summe = Intervall::punkt(0.0);
        let mut maximum = 0.0_f64;
        for (z, &m) in &self.masse {
            let x = Intervall::ganzzahl(z.schaden);
            let gewicht = if quadrat { x.mul(x)? } else { x };
            maximum = maximum.max(gewicht.oben);
            summe = summe.add(gewicht.mul(Intervall::punkt(m))?)?;
        }
        let mut grenzen = summe.erweitert(mul_oben(n.l1_fehler, maximum)?)?;
        grenzen.oben = grenzen.oben.min(maximum);
        Ok(grenzen)
    }

    /// Mean enclosure, including integer-to-binary64 conversion and summation.
    pub fn erwarteter_schaden_grenze(&self) -> Result<StatistikGrenze, NumerikFehler> {
        self.moment(false)?.statistik(self.erwarteter_schaden())
    }

    /// Var(X) = E[X²] - E[X]², with separate outward moment bounds.
    /// Near cancellation the interval can be wide; no relative guarantee is made.
    pub fn varianz_grenze(&self) -> Result<StatistikGrenze, NumerikFehler> {
        let mittel = self.moment(false)?;
        let quadrat = self.moment(true)?;
        let mittel_quadrat = mittel.mul(mittel)?;
        Intervall {
            unten: (quadrat.unten - mittel_quadrat.oben).next_down().max(0.0),
            oben: (quadrat.oben - mittel_quadrat.unten).next_up().max(0.0),
        }
        .statistik(self.varianz())
    }

    /// Monotone square-root enclosure; no differentiability at zero assumed.
    pub fn standardabweichung_grenze(&self) -> Result<StatistikGrenze, NumerikFehler> {
        let v = self.varianz_grenze()?;
        Intervall {
            unten: v.untergrenze.sqrt().next_down().max(0.0),
            oben: v.obergrenze.sqrt().next_up(),
        }
        .statistik(v.wert.sqrt())
    }

    /// Q(q) = min{x: F(x) >= q}; Q(0) is the minimum reachable damage.
    /// If mass uncertainty crosses a jump, the returned damage interval spans
    /// that jump. In particular a tiny mass error need not mean a tiny q error.
    pub fn quantil_grenze(&self, q: f64) -> Result<QuantilGrenze, NumerikFehler> {
        wahrscheinlichkeit(q)?;
        let n = self.massen_nachweis()?;
        let min = self
            .masse
            .first_key_value()
            .ok_or(NumerikFehler::KeinNachweis)?
            .0
            .schaden;
        let max = self
            .masse
            .last_key_value()
            .ok_or(NumerikFehler::KeinNachweis)?
            .0
            .schaden;
        if q == 0.0 || q == 1.0 {
            let x = if q == 0.0 { min } else { max };
            return Ok(QuantilGrenze {
                unterer_schaden: x,
                oberer_schaden: x,
            });
        }
        let mut summe = Intervall::punkt(0.0);
        let mut unten = None;
        for (z, &m) in &self.masse {
            summe = summe.add(Intervall::punkt(m))?;
            let cdf = summe.erweitert(n.l1_fehler)?;
            if unten.is_none() && cdf.oben >= q {
                unten = Some(z.schaden);
            }
            if cdf.unten >= q {
                return Ok(QuantilGrenze {
                    unterer_schaden: unten.ok_or(NumerikFehler::Massenerhalt)?,
                    oberer_schaden: z.schaden,
                });
            }
        }
        Ok(QuantilGrenze {
            unterer_schaden: unten.unwrap_or(max),
            oberer_schaden: max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verteilung::Treffer;

    fn singleton(damage: u64) -> Verteilung {
        Verteilung {
            masse: std::collections::BTreeMap::from([(
                Zustand {
                    schaden: damage,
                    phase: 0,
                },
                1.0,
            )]),
            numerik: Some(Fortpflanzung::default()),
        }
    }

    #[test]
    fn integer_conversion_above_2_to_53_is_enclosed_and_overflow_is_rejected() {
        let v = singleton(u64::MAX - 1);
        let mean = v.erwarteter_schaden_grenze().unwrap();
        assert!(mean.untergrenze <= ((u64::MAX - 1) as f64).next_down());
        assert!(mean.obergrenze >= (u64::MAX - 1) as f64);
        assert!(mean.absoluter_fehler > 0.0);
        let variance = v.varianz_grenze().unwrap();
        assert_eq!(variance.untergrenze, 0.0);
        assert!(variance.obergrenze.is_finite());
        assert_eq!(
            v.nach_treffer_geprueft(
                Treffer {
                    ohne_krit: 2,
                    mit_krit: 2
                },
                0.5,
                &[]
            )
            .unwrap_err(),
            NumerikFehler::Zahlenbereich
        );
    }

    #[test]
    fn observed_mass_and_operation_counters_fail_closed() {
        let mut v = singleton(0);
        *v.masse.values_mut().next().unwrap() = 2.0;
        assert_eq!(
            v.massen_nachweis().unwrap_err(),
            NumerikFehler::Massenerhalt
        );
        *v.masse.values_mut().next().unwrap() = f64::NAN;
        assert_eq!(
            v.massen_nachweis().unwrap_err(),
            NumerikFehler::Zahlenbereich
        );
        let mut v = singleton(0);
        v.numerik.as_mut().unwrap().schritte = u64::MAX;
        assert_eq!(
            v.nach_treffer_geprueft(
                Treffer {
                    ohne_krit: 1,
                    mit_krit: 2
                },
                0.5,
                &[]
            )
            .unwrap_err(),
            NumerikFehler::Zahlenbereich
        );
    }

    #[test]
    fn absolute_roundoff_covers_zero_and_subnormal_results() {
        umgebung().unwrap();
        assert_eq!(rundungsfehler(0.0).unwrap().to_bits(), 1);
        assert_eq!(rundungsfehler(f64::from_bits(1)).unwrap().to_bits(), 1);
        assert_eq!(
            rundungsfehler(f64::INFINITY).unwrap_err(),
            NumerikFehler::Zahlenbereich
        );
        assert_eq!(mul_oben(f64::from_bits(1), 0.5).unwrap().to_bits(), 1);
    }
}
