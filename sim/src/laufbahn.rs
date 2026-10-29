//! Die Fortschritts-Simulation: die Schleife ueber dem Kampf (Issue #922,
//! Plan E11.4e, Fassung A).
//!
//! # Wozu
//!
//! Der Simulator rechnet, was ein Ausruestungsstand leistet. Diese Schleife
//! rechnet, **ob und wann** er erreicht wird: viele simulierte Laufbahnen aus
//! Siegen, Funden, Materialumwandlung, Punkte-Ausschuettung und Aufwertung.
//! Punktekurve, Aufwertung und Fallraten werden damit gemessen statt geraten —
//! der Auftrag aus dem Masterplan: „Das muss eigentlich durch unsere
//! Simulation getestet werden … ob die von dir vorgeschlagenen Settings nicht
//! zu viele oder zu wenige Punkte dann geben."
//!
//! # Aufbau
//!
//! | Teil | Inhalt |
//! | --- | --- |
//! | [`regeln`] | die Spielregeln ueber dem Kampf, gegen die Aufzeichnung `laufbahn-regeln.json` |
//! | [`orakel`] | Fassung A: Ausruestung × feste Referenz-Spielweise × Boss → Zelle, im `Ergebnisspeicher` |
//! | [`schleife`] | eine Laufbahn mit ihren benannten Strategien |
//! | [`auswertung`] | viele Laufbahnen auf vielen Straengen, Verteilungen, Bericht |
//!
//! Ausgefuehrt wird sie ueber `sim/examples/laufbahn.rs`.
//!
//! # Die Invalidierungsregeln — HIER und nur hier
//!
//! Zwei Fingerabdruecke, zwei Geltungsbereiche:
//!
//! | Aenderung | Kampf-Tabelle | Fortschritts-Ergebnis |
//! | --- | --- | --- |
//! | Fallrate | gueltig | **ungueltig** |
//! | Beutetabelle (Katalog) | gueltig | **ungueltig** |
//! | Aufwert-Regel (Material, Kosten, Strategie) | gueltig | **ungueltig** |
//! | Freischaltbedingung | gueltig | **ungueltig** |
//! | Anlege-Strategie | gueltig | **ungueltig** |
//! | Kampfsemantik, Referenz-Spielweise, Durchgaenge | **ungueltig** | **ungueltig** |
//!
//! Im Code steht das zweimal, und beide Male als Bauart statt als Satz:
//!
//! 1. **Der Schluessel einer Zelle kennt keine Schleifenregel.**
//!    [`orakel::SpeicherOrakel`] legt Zellen unter
//!    [`crate::speicher::ErgebnisSchluessel::fuer_lauf`] ab — Liste,
//!    Ausruestung, Talente, Boss, Kampfsemantik. Fallrate oder
//!    Anlege-Strategie kommen darin nicht vor, also findet ein Lauf mit
//!    anderer Fallrate jede schon gerechnete Zelle wieder. Das spart den
//!    Kampf; [`auswertung::rechne`] nutzt es, wenn mehrere Regelsaetze dieselben
//!    Laufbahnen rechnen.
//! 2. **[`invalidierung`]** vergleicht die beiden Fingerabdruecke und sagt,
//!    was ungueltig ist. Eine ungueltige Kampf-Tabelle macht den Fortschritt
//!    mit ungueltig — er steht auf ihr —, umgekehrt nicht.
//!
//! Die Kontrollprobe aus dem Plan steht in `sim/tests/laufbahn.rs`
//! (`fallrate_macht_den_fortschritt_ungueltig_nicht_die_kampftabelle`).
//!
//! # Was diese Simulation NICHT kann, benannt
//!
//! * **Fassung A, nicht B** — siehe [`orakel::FASSUNG_A`]. Eine feste
//!   Referenz-Spielweise ohne Talente auf Kampfstufe 1; Zauber-Draft,
//!   Leistenplaetze und Talentpunkte bewegen den Kampf hier nicht. Die
//!   Spielerstufe zaehlt nur fuer Erfahrung und Punkte (Abstandsfaktor).
//! * **Eine Schwierigkeitsstufe.** Es gibt sie weder im Spiel noch im
//!   Simulator (GAME-DESIGN 11.7, `app.ts`: `DIFFICULTY_NORMAL`); die
//!   Schleife laeuft auf normal und lehnt anderes ab.
//! * **Ohne Werte-Deckel ist die Endausbau-Aussage begrenzt** (Plan E11.4e):
//!   Konvergiert der Endausbau auf einen lohnenden Zauber, rechnet jede Kurve
//!   gegen einen Zustand, den es so nicht geben soll.
//! * **Keine Satz-Teile** — kein Spielboss laesst eines fallen; `build.rs`
//!   bricht, sobald einer es koennte.
//! * **Keine Pech-Grenze, kein Duplikatschutz** — mit Absicht (Masterplan,
//!   GAME-DESIGN 21.1).

pub mod auswertung;
pub mod orakel;
pub mod regeln;
pub mod schleife;

use core::fmt;

use crate::data::BOSS;
use crate::speicher::{ErgebnisSchluessel, SemantikHash};

use orakel::{laufkonfiguration, Kampfeinstellung, Referenzspielweise};
use regeln::{
    Instanz, Vorlage, BOSS_STUFEN, EPIC_FROM_INSTANCE, FALLOFF_PER_MILLE, FIRST_ENDGAME_INSTANCE,
    HELD_POINTS_PER_PERCENT, INSTANCE_COUNT, INSTANZEN, KATALOG, POINTS_EXPONENT,
    POINTS_PER_SECOND, PROGRESS_WEIGHTS, REPEAT_PER_MILLE, XP_PER_BOSS_LEVEL, XP_TO_NEXT_LEVEL,
};
use schleife::{Anlegen, Aufwerten, Freischaltung, Laufbahnregeln};

/// FNV-1a ueber 64 Bit — dasselbe Verfahren wie in `speicher.rs`, dort privat.
#[derive(Debug, Clone, Copy)]
struct Fnv(u64);

impl Fnv {
    fn neu(format: &str) -> Self {
        let mut h = Self(0xcbf2_9ce4_8422_2325);
        h.text(format);
        h
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn zahl(&mut self, wert: u64) {
        self.bytes(&wert.to_le_bytes());
    }

    fn text(&mut self, wert: &str) {
        self.zahl(wert.len() as u64);
        self.bytes(wert.as_bytes());
    }
}

/// Fassung der Schleifenregeln. **Aendert sich, was eine Regel tut, ohne dass
/// sich eine ihrer Zahlen aendert, zaehlt man diese Kennung hoch** — sonst
/// bekaemen zwei verschiedene Schleifen denselben Fingerabdruck.
pub const SCHLEIFEN_FASSUNG: &str = "laufbahn-schleife/1";

/// Fingerabdruck der **Schleifenregeln** — alles, was das Fortschritts-
/// Ergebnis bestimmt und die Kampf-Tabelle nicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fortschrittsfingerabdruck(pub u64);

impl fmt::Display for Fortschrittsfingerabdruck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

fn vorlage_einruehren(h: &mut Fnv, v: &Vorlage) {
    h.text(v.id);
    h.zahl(v.slot as u64);
    h.zahl(u64::from(v.item_level));
    h.zahl(v.quality as u64);
    h.zahl(v.affixe.len() as u64);
    for feld in v.affixe {
        h.zahl(*feld as u64);
    }
    h.text(v.gebunden_an);
    h.zahl(i64::from(v.satz) as u64);
}

fn instanz_einruehren(h: &mut Fnv, i: &Instanz) {
    h.text(i.id);
    h.zahl(u64::from(i.number));
    h.zahl(u64::from(i.order));
    h.text(i.requires.unwrap_or(""));
}

impl Fortschrittsfingerabdruck {
    /// Der Fingerabdruck eines Regelsatzes, samt der festen Regeln, die er
    /// nicht einzeln fuehrt (Gewichtstabelle, Instanzen, Ertrag, Erfahrung).
    ///
    /// Die Destrukturierung benutzt kein `..`: Ein neues Feld in
    /// [`Laufbahnregeln`] bricht hier den Bau, bis es eingeordnet ist.
    #[must_use]
    pub fn aus(regeln: &Laufbahnregeln) -> Self {
        let Laufbahnregeln {
            fallrate_promille,
            katalog,
            material_je_qualitaet,
            kosten_je_stufe,
            freischaltung,
            anlegen,
            aufwerten,
        } = regeln;
        let mut h = Fnv::neu(SCHLEIFEN_FASSUNG);
        // ---- die einhaengbaren Regeln
        h.zahl(u64::from(*fallrate_promille));
        h.zahl(katalog.len() as u64);
        for index in katalog {
            vorlage_einruehren(&mut h, &KATALOG[*index]);
        }
        for m in material_je_qualitaet {
            h.zahl(*m);
        }
        h.zahl(*kosten_je_stufe);
        h.text(match freischaltung {
            Freischaltung::Hauptpfad => "hauptpfad",
            Freischaltung::AlleOffen => "alle-offen",
        });
        h.text(match anlegen {
            Anlegen::Spielvergleich => "spielvergleich",
            Anlegen::HoehereStufe => "hoehere-stufe",
        });
        h.text(match aufwerten {
            Aufwerten::NiedrigsteZuerst => "niedrigste-zuerst",
            Aufwerten::Nie => "nie",
        });
        // ---- die festen Regeln: Beute, Freischaltung, Ertrag, Erfahrung
        for zeile in PROGRESS_WEIGHTS {
            for g in zeile {
                h.zahl(u64::from(g));
            }
        }
        h.zahl(EPIC_FROM_INSTANCE as u64);
        h.zahl(FIRST_ENDGAME_INSTANCE as u64);
        h.zahl(INSTANCE_COUNT as u64);
        for i in &INSTANZEN {
            instanz_einruehren(&mut h, i);
        }
        for f in FALLOFF_PER_MILLE {
            h.zahl(u64::from(f));
        }
        for wert in [
            POINTS_PER_SECOND,
            POINTS_EXPONENT,
            HELD_POINTS_PER_PERCENT,
            REPEAT_PER_MILLE,
        ] {
            h.zahl(wert.to_bits());
        }
        h.zahl(XP_PER_BOSS_LEVEL);
        for b in XP_TO_NEXT_LEVEL {
            h.zahl(b);
        }
        for s in BOSS_STUFEN {
            h.zahl(u64::from(s));
        }
        Self(h.0)
    }
}

/// Fingerabdruck der **Kampf-Tabelle**: Kampfsemantik, Referenz-Spielweise,
/// Durchgaenge und Startwert der Zellen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Kampffingerabdruck(pub u64);

impl fmt::Display for Kampffingerabdruck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// Der Fingerabdruck der Kampf-Tabelle.
///
/// **Abgeleitet aus denselben Schluesseln, unter denen die Zellen liegen**:
/// [`SemantikHash::aus`] und [`ErgebnisSchluessel::fuer_lauf`] auf eine
/// Laufkonfiguration ohne Ausruestung. Was dort nicht steht, kann die Tabelle
/// nicht ungueltig machen — und umgekehrt.
#[must_use]
pub fn kampffingerabdruck(
    referenz: &Referenzspielweise,
    kampf: Kampfeinstellung,
) -> Kampffingerabdruck {
    let config = laufkonfiguration(BOSS, crate::gear::WornValues::none(), kampf);
    let mut h = Fnv::neu("laufbahn-kampf/1");
    let SemantikHash(semantik) = SemantikHash::aus(&config);
    h.zahl(semantik);
    h.text(&ErgebnisSchluessel::fuer_lauf(&config, &referenz.liste).aufbau);
    h.zahl(referenz.je_boss.len() as u64);
    for (boss, apl) in &referenz.je_boss {
        h.text(boss);
        h.text(&ErgebnisSchluessel::fuer_lauf(&config, apl).aufbau);
    }
    Kampffingerabdruck(h.0)
}

/// Was eine Aenderung ungueltig macht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invalidierung {
    /// Die Kampf-Tabelle (die Zellen im Ergebnisspeicher).
    pub kampftabelle: bool,
    /// Die Fortschritts-Ergebnisse (Verteilungen, Bericht).
    pub fortschritt: bool,
}

/// Was zwischen zwei Laeufen ungueltig wird — die Regel aus AC5.
#[must_use]
pub fn invalidierung(
    alt: (Fortschrittsfingerabdruck, Kampffingerabdruck),
    neu: (Fortschrittsfingerabdruck, Kampffingerabdruck),
) -> Invalidierung {
    let kampftabelle = alt.1 != neu.1;
    Invalidierung {
        kampftabelle,
        fortschritt: kampftabelle || alt.0 != neu.0,
    }
}
