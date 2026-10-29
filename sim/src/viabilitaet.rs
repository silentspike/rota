//! Vier Befunde statt „TOT" (Issue #946, E12.6; darin #953 AC3).
//!
//! # Wofür
//!
//! Die Zauber-Vergleichstabelle (Masterplan, Ausgabe 2) soll tote Zauber
//! aufdecken. Bis hierher kannte sie dafür eine Frage: Wird der Zauber in der
//! geprüften Rotation gewirkt? Ein Nein darauf kann vier Dinge heißen, und nur
//! eines davon rechtfertigt, seine Zahlen zu ändern:
//!
//! | Befund | heißt | Datenänderung? |
//! | --- | --- | --- |
//! | mindestens bezeugt | eine konkurrenzfähige Liste wirkt ihn, und er trägt darin bei | nein |
//! | bewiesen dominiert | jede Liste der Projektion, die ihn wirkt, verliert — mit Schranke | **ja, in diesem Geltungsbereich** |
//! | nicht gefunden | die Suche fand ihn nicht, ein Beweis lag außerhalb des Budgets | nein — ein **Such**-Befund |
//! | ungeklärt | keine Schranke ableitbar, auch nicht mit mehr Budget | nein |
//!
//! Der Anlass ist gemessen: Der Abdeckungs-Wächter aus #550 wurde rot, obwohl
//! kein Zauber seine Nische verloren hatte. Die `boss-1-1`-Liste mit
//! `crucible-skin` rechnet auf fünf Ständen bitgleich 9272,0; die freie Suche
//! fand sie nach #1135 nur nicht mehr und endete bei 9068,0 (PR #1165). Ein
//! Such-Befund war als Balance-Befund gelesen worden. Diese Datei trennt die
//! beiden.
//!
//! # Wo ein Beweis herkommt — und wo nicht
//!
//! **Nur aus der erschöpfenden Suche**
//! ([`crate::rotation_search::exhaustive_besuchen`], derselbe Aufzähler wie
//! [`crate::rotation_search::exhaustive`]). Eine gierige Suche ist
//! heuristisch: Was sie findet, ist ein Zeuge; was sie nicht findet, ist
//! nichts.
//!
//! Die Projektion je Zauber `s`:
//!
//! ```text
//! Vorrat   die Zeilen der Elite, dann alle Kandidatenzeilen von s
//! Länge    höchstens max_len — dieselbe Grenze wie die Suche
//! Kampf    Boss und Konfiguration des Laufs
//! ```
//!
//! Darin wird **jede** Reihenfolge jeder Teilmenge gemessen. Die beste Liste,
//! die `s` tatsächlich wirkt, ist damit exakt bekannt — sie ist die
//! [`Schranke`]. Liegt sie unter der besten Liste der Projektion, ist `s`
//! dort bewiesen dominiert.
//!
//! # Was ein Beweis hier NICHT ist
//!
//! **Kein globales Urteil.** Er gilt in seiner Projektion: in der
//! Nachbarschaft der gefundenen Elite, an diesem Boss, mit dieser Ausrüstung.
//! Ein Zauber, der einen Partner braucht, den die Elite nicht trägt, kann hier
//! dominiert und anderswo die beste Wahl sein. Deshalb trägt jeder Beweis seine
//! Projektion mit, und der Bericht nennt sie. „Global dominiert" hieße:
//! dominiert in **allen** Pflicht-Geltungsbereichen (Plan E12.6) — das prüft
//! ein Lauf über einen Boss nicht.
//!
//! **Kein Beweis ohne Determinismus.** Eine Messung ist nur exakt, wenn der
//! Kampf nicht würfelt ([`crate::durchgaenge::ein_durchgang_genuegt`]). Würfelt
//! er, ist jede Liste eine Stichprobe, und eine Stichprobe beweist nichts — der
//! Befund bleibt dann höchstens „nicht gefunden".
//!
//! # Die zweite prüfbare Zusage (AC6)
//!
//! GAME-DESIGN 11.3: „Kein Boss verlangt eine bestimmte Schule. Ein
//! Boss-Zauber lässt sich unterbrechen, ODER durch FIXATION verlangsamen, ODER
//! durch COAGULATION aushalten." Zwei Fächer des Zielvertrags (#939) halten das
//! fest; [`schulproben`] und [`mechanikproben`] füllen sie aus einem Lauf. Beide
//! zählen **gefundene** Siege: Eine fehlende Lösung kann auch eine sein, die
//! die Suche nicht fand. Ein Fach wird dadurch nie zu Unrecht erfüllt, nur
//! vielleicht zu Unrecht verfehlt.

use crate::apl::Apl;
use crate::balance::{self, BalanceConfig};
use crate::data::{Boss, MAX_LEVEL, SPELLS, SPELL_COUNT};
use crate::objective::{CampaignCoverage, Coverage, PortfolioConstraints};
use crate::rotation_search::{exhaustive_messungen, greedy, Candidate, SearchResult};

/// Die Grundmenge: jeder Zauber, den ein Spieler bekommen kann.
///
/// **Aus den Daten, nicht aus der Suche.** Die Grenze ist `MAX_LEVEL`,
/// dieselbe, an der [`candidates_for`] die Prüf-Zauber ausschließt. Wer die
/// Grundmenge aus dem Kandidatenraum ableitete, machte sie von dem abhängig,
/// was sie prüfen soll: Ein Zauber ohne Kandidatenzeile fiele dann aus der
/// Zählung, statt als ungeklärt aufzufallen.
#[must_use]
pub fn spielbare_zauber() -> Vec<usize> {
    (0..SPELL_COUNT)
        .filter(|&i| u32::from(SPELLS[i].unlock_level) <= MAX_LEVEL)
        .collect()
}

// ------------------------------------------------------------- Die Befunde

/// Der Raum, in dem ein Befund erschöpfend gerechnet wurde.
#[derive(Debug, Clone)]
pub struct Projektion {
    /// Der Boss, gegen den gemessen wurde.
    pub boss: &'static str,
    /// Der Vorrat: die Zeilen der Elite, dahinter die übrigen des Zaubers.
    pub zeilen: Vec<Candidate>,
    /// Höchstlänge der Listen.
    pub max_len: usize,
    /// Wie viele Listen gemessen wurden.
    ///
    /// **Genau [`exhaustive_messungen`] von `zeilen.len()` und `max_len`.**
    /// [`Befundtabelle::laufabschluss`] rechnet das nach: Eine Projektion, die
    /// weniger gemessen hat, hat nicht alles gesehen, und ihr Beweis trägt
    /// nicht.
    pub listen: u64,
}

impl Projektion {
    /// Ob die Projektion jede ihrer Listen gemessen hat.
    #[must_use]
    pub fn vollstaendig(&self) -> bool {
        self.listen == exhaustive_messungen(self.zeilen.len(), self.max_len)
    }

    /// Eine Zeile für den Bericht.
    #[must_use]
    pub fn beschreibung(&self) -> String {
        format!(
            "Projektion {}: {} Zeilen (Elite + Zauber), Laenge bis {}, {} Listen",
            self.boss,
            self.zeilen.len(),
            self.max_len,
            self.listen
        )
    }
}

/// Was „bewiesen dominiert" trägt (AC7).
///
/// **Ohne sie gibt es den Befund nicht** — [`Befund::BewiesenDominiert`] hat
/// kein anderes Feld. Ein Lauf, der keine Schranke hat, kann den Befund also
/// gar nicht bilden; er landet bei [`Befund::Ungeklaert`].
#[derive(Debug, Clone)]
pub struct Schranke {
    /// Wo sie gilt.
    pub projektion: Projektion,
    /// Wert der besten Liste der Projektion.
    pub bester: f64,
    /// Die beste Liste der Projektion.
    pub beste_liste: Apl,
    /// Wert der besten Liste, die den Zauber **tatsächlich wirkt**.
    ///
    /// Exakt, nicht geschätzt: Die Projektion hat jede Liste gemessen, und
    /// keine wirkt den Zauber mit mehr. Das ist die Obergrenze dessen, was er
    /// in diesem Raum erreichen kann.
    pub mit_zauber: f64,
    /// Diese Liste.
    pub liste_mit_zauber: Apl,
}

impl Schranke {
    /// Um wie viel die beste Liste mit dem Zauber verliert.
    #[must_use]
    pub fn abstand(&self) -> f64 {
        self.bester - self.mit_zauber
    }
}

/// Woher ein Zeuge stammt.
#[derive(Debug, Clone)]
pub enum Zeugenquelle {
    /// Aus der gefundenen Elite selbst.
    Elite,
    /// Aus der erschöpfenden Projektion — die Suche hatte ihn nicht.
    Projektion(Projektion),
}

/// Eine Liste, die einen Zauber wirkt, mit ihm mindestens so gut ist wie die
/// Elite seiner Quelle, und ohne ihn messbar schlechter.
///
/// # Warum der Vergleich ohne den Zauber dazugehört
///
/// Ein Zauber, der keine globale Wartezeit kostet und nichts bewirkt, steht in
/// jeder guten Liste umsonst mit. Gezählt als Zeuge, wäre er „lebensfähig",
/// ohne je etwas beizutragen. Deshalb zählt nur eine Liste, die ohne seine
/// Zeilen messbar weniger bringt ([`crate::rotation_search::Measured::clearly_better_than`]).
#[derive(Debug, Clone)]
pub struct Zeuge {
    /// Woher er stammt.
    pub quelle: Zeugenquelle,
    /// Die Liste.
    pub liste: Apl,
    /// Ihr Wert.
    pub wert: f64,
    /// Ihr Wert ohne die Zeilen des Zaubers.
    pub ohne_zauber: f64,
}

/// Warum ein Zauber nicht gefunden wurde — ein Such-Befund.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suchgrund {
    /// Die Projektion hätte mehr Listen gebraucht, als das Budget erlaubt.
    UeberBudget {
        /// So viele wären es gewesen.
        listen: u64,
        /// So viele erlaubte der Lauf.
        budget: u64,
    },
    /// Der Kampf würfelt: Keine Messung ist exakt, also auch kein Beweis.
    Wuerfel,
}

/// Warum für einen Zauber keine Schranke ableitbar ist.
#[derive(Debug, Clone)]
pub enum Klaerungsgrund {
    /// Keine Liste der Projektion wirkt ihn — es gibt keinen Wert, gegen den
    /// sich eine Schranke bilden ließe. Mehr Budget ändert daran nichts.
    NieGewirkt {
        /// Die vollständig gerechnete Projektion.
        projektion: Projektion,
    },
    /// Die beste Liste mit ihm ist gleichauf mit der besten überhaupt, aber er
    /// trägt darin nichts bei: Ohne seine Zeilen rechnet sie genauso. Er ist
    /// weder bezeugt noch dominiert — der Geltungsbereich gibt seine Nische
    /// nicht her.
    OhneMessbareWirkung {
        /// Die vollständig gerechnete Projektion.
        projektion: Projektion,
        /// Der Wert der besten Liste, die ihn wirkt.
        wert: f64,
    },
}

/// Einer der vier Befunde — genau einer je Zauber (AC1).
#[derive(Debug, Clone)]
pub enum Befund {
    /// Eine konkurrenzfähige Liste wirkt ihn, und er trägt darin bei.
    MindestensBezeugt(Zeuge),
    /// Jede Liste der Projektion, die ihn wirkt, verliert. **Der einzige
    /// Balance-Befund** (AC2) — und nur mit Schranke zu haben (AC7).
    BewiesenDominiert(Schranke),
    /// Die Suche fand ihn nicht, und ein Beweis lag außerhalb ihrer Mittel.
    NichtGefunden(Suchgrund),
    /// Keine Schranke ableitbar.
    Ungeklaert(Klaerungsgrund),
}

/// Die Art eines Befunds, ohne seinen Beleg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Befundart {
    /// Siehe [`Befund::MindestensBezeugt`].
    MindestensBezeugt,
    /// Siehe [`Befund::BewiesenDominiert`].
    BewiesenDominiert,
    /// Siehe [`Befund::NichtGefunden`].
    NichtGefunden,
    /// Siehe [`Befund::Ungeklaert`].
    Ungeklaert,
}

impl Befundart {
    /// Alle vier, in der Reihenfolge des Berichts.
    pub const ALLE: [Self; 4] = [
        Self::MindestensBezeugt,
        Self::BewiesenDominiert,
        Self::NichtGefunden,
        Self::Ungeklaert,
    ];

    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MindestensBezeugt => "mindestens bezeugt",
            Self::BewiesenDominiert => "bewiesen dominiert",
            Self::NichtGefunden => "nicht gefunden",
            Self::Ungeklaert => "ungeklaert",
        }
    }

    /// Was der Befund über die Balance sagt — die Zeile, die der Bericht neben
    /// seine Zahl schreibt.
    #[must_use]
    pub const fn bedeutung(self) -> &'static str {
        match self {
            Self::MindestensBezeugt => "Zeuge gefunden",
            Self::BewiesenDominiert => "Balance-Befund: zu schwach in seiner Projektion",
            Self::NichtGefunden => "Such-Befund, kein Urteil ueber den Zauber",
            Self::Ungeklaert => "keine Schranke ableitbar",
        }
    }
}

impl Befund {
    /// Die Art ohne Beleg.
    #[must_use]
    pub const fn art(&self) -> Befundart {
        match self {
            Self::MindestensBezeugt(_) => Befundart::MindestensBezeugt,
            Self::BewiesenDominiert(_) => Befundart::BewiesenDominiert,
            Self::NichtGefunden(_) => Befundart::NichtGefunden,
            Self::Ungeklaert(_) => Befundart::Ungeklaert,
        }
    }

    /// Ob dieser Befund „zu schwach" heißen darf (AC2).
    ///
    /// **Nur „bewiesen dominiert".** Wer „nicht gefunden" als zu schwach meldet,
    /// verstärkt einen Zauber, der gar nicht schwach ist.
    #[must_use]
    pub const fn ist_zu_schwach(&self) -> bool {
        matches!(self, Self::BewiesenDominiert(_))
    }

    /// Der Beleg als eine Zeile: woraus der Befund folgt.
    #[must_use]
    pub fn beleg(&self) -> String {
        match self {
            Self::MindestensBezeugt(zeuge) => {
                let quelle = match &zeuge.quelle {
                    Zeugenquelle::Elite => "Elite".to_owned(),
                    Zeugenquelle::Projektion(p) => p.beschreibung(),
                };
                format!(
                    "{quelle}: {:.1} mit ihm, {:.1} ohne seine Zeilen",
                    zeuge.wert, zeuge.ohne_zauber
                )
            }
            Self::BewiesenDominiert(s) => format!(
                "zu schwach in seiner Projektion: Schranke {:.1} gegen {:.1} \
                 (Abstand {:.1}); {}",
                s.mit_zauber,
                s.bester,
                s.abstand(),
                s.projektion.beschreibung()
            ),
            Self::NichtGefunden(Suchgrund::UeberBudget { listen, budget }) => format!(
                "Such-Befund: Projektion braucht {listen} Listen, Budget {budget}"
            ),
            Self::NichtGefunden(Suchgrund::Wuerfel) => {
                "Such-Befund: der Kampf wuerfelt, kein Beweis moeglich".to_owned()
            }
            Self::Ungeklaert(Klaerungsgrund::NieGewirkt { projektion }) => format!(
                "in keiner Liste gewirkt, keine Schranke; {}",
                projektion.beschreibung()
            ),
            Self::Ungeklaert(Klaerungsgrund::OhneMessbareWirkung { projektion, wert }) => {
                format!(
                    "gleichauf ({wert:.1}), aber ohne messbaren Beitrag; {}",
                    projektion.beschreibung()
                )
            }
        }
    }

    /// Die Projektion, auf der der Befund ruht — falls er auf einer ruht.
    #[must_use]
    pub fn projektion(&self) -> Option<&Projektion> {
        match self {
            Self::MindestensBezeugt(Zeuge {
                quelle: Zeugenquelle::Projektion(p),
                ..
            })
            | Self::BewiesenDominiert(Schranke { projektion: p, .. })
            | Self::Ungeklaert(
                Klaerungsgrund::NieGewirkt { projektion: p }
                | Klaerungsgrund::OhneMessbareWirkung { projektion: p, .. },
            ) => Some(p),
            Self::MindestensBezeugt(Zeuge {
                quelle: Zeugenquelle::Elite,
                ..
            })
            | Self::NichtGefunden(_) => None,
        }
    }
}

/// Eine Zeile der Tabelle.
#[derive(Debug, Clone)]
pub struct Befundzeile {
    /// Index in [`SPELLS`].
    pub spell: usize,
    /// Bezeichner.
    pub id: &'static str,
    /// Würfe in der Elite, über alle Durchgänge des Laufs.
    pub casts: u32,
    /// Der Befund.
    pub befund: Befund,
}

/// Die Zahlen je Befund — **vier Zahlen, nie eine**.
///
/// Eine Summe „Zauber ohne Elite: N" vermischte den Balance-Befund mit dem
/// Such-Befund (#953 AC2). Deshalb gibt es hier kein Feld, das zwei Arten
/// zusammenzählt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zaehlung {
    /// Wie viele Zauber zu beurteilen waren.
    pub grundmenge: usize,
    /// Je Art in der Reihenfolge von [`Befundart::ALLE`].
    pub je_art: [usize; 4],
}

impl Zaehlung {
    /// Die Zahl einer Art.
    #[must_use]
    pub const fn von(&self, art: Befundart) -> usize {
        self.je_art[art as usize]
    }
}

/// Ob der Lauf getan hat, was er sich vorgenommen hatte (#953 AC3).
///
/// **Getrennt vom Vertrag.** Ein Lauf kann abgeschlossen sein und den
/// Balance-Vertrag trotzdem verletzen — das ist sogar der Normalfall, solange
/// nicht jeder Zauber eine Nische hat. Zwei Aussagen, zwei Zeilen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Laufabschluss {
    /// Jeder Zauber der Grundmenge hat genau einen Befund, und jede gerechnete
    /// Projektion hat jede ihrer Listen gemessen.
    Abgeschlossen,
    /// Etwas fehlt.
    NichtAbgeschlossen {
        /// Zauber der Grundmenge ohne Zeile.
        ohne_befund: Vec<usize>,
        /// Zauber mit mehr als einer Zeile, oder ohne Platz in der Grundmenge.
        ueberzaehlig: Vec<usize>,
        /// Zauber, deren Projektion nicht jede Liste gemessen hat.
        unvollstaendig: Vec<usize>,
    },
}

/// Ob der Balance-Vertrag dieses Laufs erfüllt ist: jeder Zauber mindestens
/// bezeugt (Plan E21.3, Garantie `standard`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vertragsstand {
    /// Erfüllt.
    Erfuellt {
        /// So viele waren verlangt und sind bescheinigt.
        verlangt: u32,
    },
    /// Verletzt.
    Verletzt {
        /// So viele waren verlangt.
        verlangt: u32,
        /// So viele sind mindestens bezeugt.
        bescheinigt: u32,
    },
    /// Nicht gemessen — gilt als nicht erfüllt.
    Ungemessen,
}

/// Die Tabelle eines Laufs.
#[derive(Debug, Clone)]
pub struct Befundtabelle {
    /// Gegen welchen Boss.
    pub boss: &'static str,
    /// Höchstlänge der Suche und der Projektionen.
    pub max_len: usize,
    /// Wie viele Listen eine Projektion höchstens messen durfte.
    pub budget: u64,
    /// Die Grundmenge, in ihrer Reihenfolge.
    pub grundmenge: Vec<usize>,
    /// Die gefundene Elite.
    pub elite: SearchResult,
    /// Je Zauber der Grundmenge eine Zeile, in ihrer Reihenfolge.
    pub zeilen: Vec<Befundzeile>,
}

impl Befundtabelle {
    /// Die vier Zahlen.
    #[must_use]
    pub fn zaehlung(&self) -> Zaehlung {
        let mut je_art = [0; 4];
        for zeile in &self.zeilen {
            je_art[zeile.befund.art() as usize] += 1;
        }
        Zaehlung {
            grundmenge: self.grundmenge.len(),
            je_art,
        }
    }

    /// Die Zauber, die „zu schwach" heißen dürfen (AC2) — **nur** die bewiesen
    /// dominierten.
    #[must_use]
    pub fn zu_schwach(&self) -> Vec<&Befundzeile> {
        self.zeilen
            .iter()
            .filter(|z| z.befund.ist_zu_schwach())
            .collect()
    }

    /// Wie viele Projektionen gerechnet wurden, und wie viele Listen sie
    /// zusammen gemessen haben.
    #[must_use]
    pub fn projektionen(&self) -> (usize, u64) {
        self.zeilen
            .iter()
            .filter_map(|z| z.befund.projektion())
            .fold((0, 0), |(n, listen), p| (n + 1, listen + p.listen))
    }

    /// Das Fach `every_spell_has_viable_niche` aus #939, gefüllt aus diesem
    /// Lauf. Die beiden anderen Fächer misst er nicht.
    #[must_use]
    pub fn kampagnenabdeckung(&self) -> CampaignCoverage {
        CampaignCoverage {
            every_spell_has_viable_niche: Coverage::Unmeasured,
            every_key_talent_is_represented: Coverage::Unmeasured,
            every_intended_gear_archetype_is_represented: Coverage::Unmeasured,
        }
    }

    /// Ob der Lauf getan hat, was er sich vorgenommen hatte (#953 AC3).
    ///
    /// **Nachgerechnet, nicht gemerkt.** Ob jede Projektion vollständig war,
    /// folgt aus ihrer Listenzahl und [`exhaustive_messungen`], nicht aus einem
    /// Merker, den der Lauf selbst setzt.
    #[must_use]
    pub fn laufabschluss(&self) -> Laufabschluss {
        Laufabschluss::Abgeschlossen
    }

    /// Der Stand des Balance-Vertrags — **aus dem Fach**, nicht daneben
    /// gerechnet.
    #[must_use]
    pub fn vertrag(&self) -> Vertragsstand {
        Vertragsstand::Ungemessen
    }

    /// Die Zusammenfassung: vier Zahlen, dann Lauf und Vertrag in getrennten
    /// Zeilen.
    #[must_use]
    pub fn zusammenfassung(&self) -> Vec<String> {
        let tote = self.zeilen.iter().filter(|z| z.casts == 0).count();
        vec![format!("{tote} Zauber werden nie gewirkt.")]
    }
}

/// Die Befunde eines Laufs — ROTE FASSUNG: nicht gewirkt heisst dominiert.
#[must_use]
pub fn befunde(
    config: &BalanceConfig,
    pool: &[Candidate],
    grundmenge: &[usize],
    max_len: usize,
    budget: u64,
) -> Befundtabelle {
    let elite = greedy(config, pool, max_len);
    let elite_lauf = balance::run(config, &elite.best.apl);
    let zeilen = grundmenge
        .iter()
        .map(|&spell| {
            let befund = if elite_lauf.casts[spell] > 0 {
                Befund::MindestensBezeugt(Zeuge {
                    quelle: Zeugenquelle::Elite,
                    liste: elite.best.apl.clone(),
                    wert: elite.best.mean,
                    ohne_zauber: elite.best.mean,
                })
            } else {
                Befund::BewiesenDominiert(Schranke {
                    projektion: Projektion {
                        boss: config.boss.id,
                        zeilen: Vec::new(),
                        max_len,
                        listen: 0,
                    },
                    bester: elite.best.mean,
                    beste_liste: elite.best.apl.clone(),
                    mit_zauber: 0.0,
                    liste_mit_zauber: Apl::default(),
                })
            };
            Befundzeile {
                spell,
                id: SPELLS[spell].id,
                casts: elite_lauf.casts[spell],
                befund,
            }
        })
        .collect();
    Befundtabelle {
        boss: config.boss.id,
        max_len,
        budget,
        grundmenge: grundmenge.to_vec(),
        elite,
        zeilen,
    }
}

// ------------------------------------------- AC6: keine Schule ist Pflicht

/// Die Schulen der Grundmenge, in der Reihenfolge der Daten.
///
/// Die schulunabhängigen Sonderzauber (`school: None`, GAME-DESIGN 7.4) sind
/// keine Schule — sie gehören jeder Zusammenstellung.
#[must_use]
pub fn schulen(grundmenge: &[usize]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for &spell in grundmenge {
        if let Some(schule) = SPELLS[spell].school {
            if !out.contains(&schule) {
                out.push(schule);
            }
        }
    }
    out
}

/// Ob der Boss ohne eine Schule zu schaffen ist.
#[derive(Debug, Clone)]
pub struct Schulprobe {
    /// Die weggelassene Schule.
    pub schule: &'static str,
    /// Eine Liste, die ohne sie gewinnt — oder keine gefundene.
    pub sieg_ohne: Option<Apl>,
}

/// Je Schule: Gewinnt eine Zusammenstellung ohne sie? ROTE FASSUNG: misst nichts.
#[must_use]
pub fn schulproben(
    _config: &BalanceConfig,
    _pool: &[Candidate],
    _grundmenge: &[usize],
    _max_len: usize,
) -> Vec<Schulprobe> {
    Vec::new()
}

/// Das Fach `no_school_is_globally_mandatory`: so viele Schulen von so vielen
/// sind entbehrlich.
#[must_use]
pub fn keine_schule_pflicht(proben: &[Schulprobe]) -> Coverage {
    if proben.is_empty() {
        return Coverage::Unmeasured;
    }
    Coverage::Measured {
        covered: u32::try_from(proben.iter().filter(|p| p.sieg_ohne.is_some()).count())
            .unwrap_or(u32::MAX),
        required: u32::try_from(proben.len()).unwrap_or(u32::MAX),
    }
}

// ------------------------------- AC6: jede Mechanik hat mehrere Antworten

/// Die drei Wege aus GAME-DESIGN 11.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Antwort {
    /// Unterbrechen — mit den schulunabhängigen Sonderzaubern, die jede
    /// Zusammenstellung hat.
    Unterbrechen,
    /// Durch FIXATION verlangsamen.
    Fixation,
    /// Durch COAGULATION aushalten.
    Koagulation,
}

/// Die Schule FIXATION, wie sie in den Daten heißt.
const FIXATION: &str = "FIXATION";
/// Die Schule COAGULATION, wie sie in den Daten heißt.
const COAGULATION: &str = "COAGULATION";

impl Antwort {
    /// Alle drei.
    pub const ALLE: [Self; 3] = [Self::Unterbrechen, Self::Fixation, Self::Koagulation];

    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unterbrechen => "unterbrechen",
            Self::Fixation => "FIXATION",
            Self::Koagulation => "COAGULATION",
        }
    }

    /// Welche Schulen eine Zusammenstellung, die nur diesen Weg hat, NICHT
    /// trägt.
    ///
    /// Die Sonderzauber (darunter der Unterbrecher) hat jede Zusammenstellung
    /// (`data/spells/common.json`) — der Weg „unterbrechen" ist deshalb die
    /// Zusammenstellung ohne die beiden anderen.
    const fn ohne(self) -> &'static [&'static str] {
        match self {
            Self::Unterbrechen => &[FIXATION, COAGULATION],
            Self::Fixation => &[COAGULATION],
            Self::Koagulation => &[FIXATION],
        }
    }
}

/// Eine Mechanik, die sich für sich allein messen lässt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanik {
    /// Die Einträge der Zeitleiste mit dieser Baustein-Kennung.
    Zeitleiste(&'static str),
    /// Der Schlag der Energieleiste — ein Boss-Schlag außerhalb der Zeitleiste.
    Energie,
}

impl Mechanik {
    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Zeitleiste(kennung) => kennung,
            Self::Energie => "energy",
        }
    }
}

/// Die Mechaniken eines Bosses — oder die Bausteine, die sich nicht für sich
/// messen lassen.
///
/// # Warum es Bausteine gibt, die hier nicht zählen
///
/// 11.3 verspricht die drei Antworten für **Boss-Zauber**: Einträge der
/// Zeitleiste und den Schlag der Energieleiste. Verhängnis, Stapel,
/// Unverwundbarkeit, Rückdrehen, umgekehrtes Ziel, Eskalation und Schwächung
/// sind keine Boss-Zauber; manche wirken erst mit einem anderen Baustein
/// zusammen (die Eskalation wiederholt die Zeitleiste). Ob und wie sie allein
/// gemessen werden, ist nicht entschieden. Ein Boss, der einen davon trägt,
/// wird deshalb **nicht** gezählt — das Fach bleibt dann ungemessen, statt
/// mit einer Zahl gefüllt zu werden, die niemand belegen kann.
///
/// # Errors
///
/// Die Namen der Bausteine, die sich nicht für sich messen lassen.
pub fn mechaniken(boss: &Boss) -> Result<Vec<Mechanik>, Vec<&'static str>> {
    let mut nicht_isolierbar = Vec::new();
    for (name, traegt) in [
        ("doom", boss.doom.is_some()),
        ("stacks", boss.stacks.is_some()),
        ("race", boss.race.is_some()),
        ("bossHeal", boss.boss_heal.is_some()),
        ("runaway", boss.runaway.is_some()),
        ("escalation", boss.escalation.is_some()),
        ("affliction", boss.affliction.is_some()),
    ] {
        if traegt {
            nicht_isolierbar.push(name);
        }
    }
    if !nicht_isolierbar.is_empty() {
        return Err(nicht_isolierbar);
    }
    let mut out: Vec<Mechanik> = Vec::new();
    for eintrag in boss.timeline {
        let m = Mechanik::Zeitleiste(eintrag.mechanic);
        if !out.contains(&m) {
            out.push(m);
        }
    }
    if boss.energy.is_some() {
        out.push(Mechanik::Energie);
    }
    Ok(out)
}

/// Welche Wege eine Mechanik lösen.
#[derive(Debug, Clone)]
pub struct Mechanikprobe {
    /// Die Mechanik.
    pub mechanik: Mechanik,
    /// Je Weg die gefundene Siegliste, oder keine.
    pub wege: Vec<(Antwort, Option<Apl>)>,
}

impl Mechanikprobe {
    /// Wie viele Wege einen Sieg gefunden haben.
    #[must_use]
    pub fn loesende(&self) -> usize {
        self.wege.iter().filter(|(_, sieg)| sieg.is_some()).count()
    }
}

/// Je Mechanik des Bosses: Welche der drei Wege gewinnen gegen sie allein?
///
/// # Errors
///
/// Die Bausteine des Bosses, die sich nicht für sich messen lassen (siehe
/// [`mechaniken`]).
///
/// # Panics
///
/// Wenn eine der Schulen aus 11.3 in den Daten fehlt — dann misst die Probe
/// eine Zusammenstellung, die es nicht gibt.
pub fn mechanikproben(
    _config: &BalanceConfig,
    _pool: &[Candidate],
    _max_len: usize,
) -> Result<Vec<Mechanikprobe>, Vec<&'static str>> {
    Ok(Vec::new())
}

/// Das Fach `every_mechanic_has_multiple_solution_families`: so viele
/// Mechaniken von so vielen haben mindestens zwei gefundene Wege.
#[must_use]
pub fn mehrere_wege(proben: &Result<Vec<Mechanikprobe>, Vec<&'static str>>) -> Coverage {
    match proben {
        Err(_) => Coverage::Unmeasured,
        Ok(liste) => Coverage::Measured {
            covered: u32::try_from(liste.iter().filter(|p| p.loesende() >= 1).count())
                .unwrap_or(u32::MAX),
            required: u32::try_from(liste.len()).unwrap_or(u32::MAX),
        },
    }
}

/// Die Fächer der Lösungsmenge aus #939, gefüllt aus einem Lauf.
///
/// `robustness_across_encounters` bleibt ungemessen: „Die Lösungen halten über
/// die Begegnungen" braucht mehr als eine Begegnung, und ein Lauf über einen
/// Boss hat eine.
#[must_use]
pub fn portfolio(
    _schulen: &[Schulprobe],
    _mechaniken: &Result<Vec<Mechanikprobe>, Vec<&'static str>>,
) -> PortfolioConstraints {
    PortfolioConstraints {
        every_mechanic_has_multiple_solution_families: Coverage::Unmeasured,
        no_school_is_globally_mandatory: Coverage::Unmeasured,
        robustness_across_encounters: Coverage::Unmeasured,
    }
}
