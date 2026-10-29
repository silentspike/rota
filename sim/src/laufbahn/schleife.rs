//! Eine Laufbahn: die Schleife ueber dem Kampf (Issue #922).
//!
//! ```text
//! Startstand
//!    -> Boss waehlen (was ist freigeschaltet?)
//!    -> Orakel: schafft dieser Stand diesen Boss?
//!    -> bei Sieg: Punkte, Erfahrung, Beutewurf
//!    -> besseres Stueck? anlegen, sonst zu Material
//!    -> Material in Aufwertung, hoechstens bis zur Stufe des Inhalts
//!    -> zurueck, Zaehler erhoehen
//! ```
//!
//! # Was hier eine Wahl ist, und dass sie benannt ist
//!
//! Das Spiel legt Regeln fest, nicht wie jemand spielt. Wie die
//! Referenz-Spielerin einen Boss waehlt, wann sie einen Fund anlegt und wohin
//! ihr Material geht, sind deshalb **Strategien** — jede als eigener Wert in
//! [`Laufbahnregeln`], jede im Fingerabdruck, jede in der Ausgabe genannt:
//!
//! | Wahl | Voreinstellung | Woher |
//! | --- | --- | --- |
//! | Bosswahl | erster unbesiegter Boss der offenen Instanzen, nach einer Niederlage ein Kampf am Farm-Boss | 12.7: innerhalb einer Instanz frei waehlbar — die Datenreihenfolge ist die Wahl |
//! | Anlegen | wenn der automatische Vergleich „besser" zeigt | 13.6: „Jeder Fund zeigt sofort, ob er besser ist" — `compareBagItem` |
//! | Aufwerten | niedrigstes getragenes Stueck zuerst, je eine Stufe | 13.6 sagt nicht, welches Stueck; die Wahl verteilt gleichmaessig |
//!
//! **Kein Mitleidszaehler, kein Duplikatschutz** (Masterplan, Beute-Tabelle;
//! GAME-DESIGN 21.1): Der Zustand traegt keinen Zaehler fuer Pech, und der
//! Beutewurf ([`roll_loot`]) hat keinen.

use crate::data::BOSSES;
use crate::gear::{QUALITY_COMMON, SLOT_COUNT};
use crate::rng::RandomStream;

use super::orakel::{Kampforakel, OrakelFehler, OrakelSchluessel};
use super::regeln::{
    boss_stufe, compare_bag_item, freigeschaltete_instanzen, grant_xp, instanz, material_for_mit,
    points_breakdown, roll_loot, strom_startwert, upgrade_item_mit, worn_from, xp_for_victory,
    Fundort, Punktequelle, Stueck, Stufenstand, Vorlage, COMPARE_BETTER, DROP_CHANCE_PER_MILLE,
    INSTANZEN, KATALOG, MATERIAL_PER_LEVEL, MATERIAL_PER_QUALITY, START_LEVEL,
};

/// Wann ein Fund angelegt wird — die **Anlege-Strategie**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anlegen {
    /// Wenn der automatische Vergleich des Spiels „besser" zeigt
    /// (`compareBagItem`, Urteil `COMPARE_BETTER`).
    Spielvergleich,
    /// Wenn die Fundstufe ueber der getragenen liegt — ohne Blick auf Werte.
    HoehereStufe,
}

/// Wohin das Material geht — die **Aufwert-Strategie**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aufwerten {
    /// Nach jedem Sieg: das getragene Stueck mit der niedrigsten Stufe um eine
    /// Stufe, solange Material und Obergrenze es zulassen.
    NiedrigsteZuerst,
    /// Nie — das Material bleibt liegen. Der Vergleichsweg fuer AC7.
    Nie,
}

/// Welche Instanzen offen sind — die **Freischaltbedingung**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freischaltung {
    /// Wie im Spiel: offen, wenn die verlangte Instanz abgeschlossen ist
    /// (`freigeschalteteInstanzen`).
    Hauptpfad,
    /// Alles von Anfang an offen — der Vergleichsweg, was die Bedingung
    /// kostet.
    AlleOffen,
}

/// Die Regeln der Schleife — **alles, was die Fortschritts-Ergebnisse
/// ungueltig macht, die Kampf-Tabelle aber nicht** (AC5).
///
/// Die Voreinstellung ist das Spiel: Fallrate und Katalog aus den Daten,
/// Material und Kosten aus `src/loot/upgrade.ts`, der Hauptpfad aus 12.7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laufbahnregeln {
    /// Wie oft ein Sieg etwas fallen laesst, in Promille.
    pub fallrate_promille: u32,
    /// Welche Vorlagen fallen koennen, als Indizes in [`KATALOG`] — die
    /// Beutetabelle.
    pub katalog: Vec<usize>,
    /// Material je nicht behaltenem Stueck, nach Qualitaet.
    pub material_je_qualitaet: [u64; 4],
    /// Was eine Aufwertungsstufe kostet.
    pub kosten_je_stufe: u64,
    /// Die Freischaltbedingung.
    pub freischaltung: Freischaltung,
    /// Die Anlege-Strategie.
    pub anlegen: Anlegen,
    /// Die Aufwert-Strategie.
    pub aufwerten: Aufwerten,
}

impl Default for Laufbahnregeln {
    fn default() -> Self {
        Self {
            fallrate_promille: DROP_CHANCE_PER_MILLE,
            katalog: (0..KATALOG.len()).collect(),
            material_je_qualitaet: MATERIAL_PER_QUALITY,
            kosten_je_stufe: MATERIAL_PER_LEVEL,
            freischaltung: Freischaltung::Hauptpfad,
            anlegen: Anlegen::Spielvergleich,
            aufwerten: Aufwerten::NiedrigsteZuerst,
        }
    }
}

/// Welcher Fund verfolgt wird: eine Vorlage ab einer Qualitaet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fundziel {
    /// Index in [`KATALOG`].
    pub vorlage: usize,
    /// Ab welcher Qualitaet er zaehlt.
    pub mindest_guete: usize,
}

/// Wie lange eine Laufbahn laeuft und was sie verfolgt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Messauftrag {
    /// Nach so vielen Siegen endet sie.
    pub siege: u32,
    /// Spaetestens nach so vielen Kaempfen — wer bis dahin nicht so viele
    /// Siege hat, steckt fest.
    pub max_kaempfe: u32,
    /// Der verfolgte Fund.
    pub fund: Fundziel,
}

/// Was nach einem Sieg feststeht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sieg {
    /// Ausgezahlte Punkte.
    pub punkte: i64,
    /// Summe der Stufen auf den zehn Plaetzen, leer zaehlt 0.
    pub stufensumme: u32,
    /// Spielerstufe danach.
    pub spielerstufe: u32,
    /// Der wievielte Kampf das war.
    pub kampf: u32,
}

/// Die Ausruestung in einem Augenblick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standbild {
    /// Summe der Stufen auf den zehn Plaetzen.
    pub stufensumme: u32,
    /// Wie viele getragene Stuecke ueber `common` liegen.
    pub ueber_common: u32,
    /// Wie viele Plaetze belegt sind.
    pub belegt: u32,
}

/// Wie viele Siege eine Laufbahn einzeln festhaelt — der groesste Sieg, an dem
/// der Bericht einen Verlauf ausgibt.
///
/// **Nicht jeden.** Die Frage nach Spielerstufe 20 braucht Laufbahnen ueber
/// 16 000 Siege; je Sieg einen Datensatz zu halten waeren bei 10 000
/// Laufbahnen 3,8 GB. Was ueber die ganze Laufbahn zaehlt, steht in
/// [`Laufbahn::siege_gesamt`], [`Laufbahn::punkte_gesamt`] und
/// [`Laufbahn::stufe_nach_kaempfen`].
pub const VERLAUF_SIEGE: usize = 150;

/// Eine gerechnete Laufbahn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laufbahn {
    /// Die ersten [`VERLAUF_SIEGE`] Siege, in Reihenfolge.
    pub siege: Vec<Sieg>,
    /// Wie viele Siege es insgesamt waren.
    pub siege_gesamt: u32,
    /// Alle ausgezahlten Punkte zusammen.
    pub punkte_gesamt: i64,
    /// Je Spielerstufe (Index = Stufe - 1): nach wie vielen Kaempfen sie
    /// zuerst erreicht war, `None` wenn nie.
    pub stufe_nach_kaempfen: Vec<Option<u32>>,
    /// Bei welchem Sieg (ab 1) der verfolgte Fund zum ersten Mal fiel.
    pub fund_beim_sieg: Option<u32>,
    /// Je Instanz in [`INSTANZEN`]: nach wie vielen Kaempfen sie offen war.
    pub geoeffnet_nach: Vec<Option<u32>>,
    /// Je Instanz: nach wie vielen Kaempfen jeder ihrer Bosse gefallen war.
    pub abgeschlossen_nach: Vec<Option<u32>>,
    /// Je Instanz: die Ausruestung im Augenblick der Oeffnung.
    pub stand_bei_oeffnung: Vec<Option<Standbild>>,
    /// Wie viele Kaempfe es insgesamt waren.
    pub kaempfe: u32,
    /// Der groesste Abstand einer getragenen Stufe UEBER der Obergrenze des
    /// aktuellen Inhalts, ueber die ganze Laufbahn. Hoechstens 0, solange die
    /// Regel aus 13.6 haelt.
    pub ueber_deckel: i64,
    /// Material, das am Ende liegt.
    pub material_ungenutzt: u64,
}

/// Mischt zwei Zahlen zu einem Startwert (murmur3, Endstufe).
///
/// Jede Laufbahn bekommt so einen eigenen Startwert aus Lauf-Startwert und
/// Nummer, und jeder Kampf darin einen aus dem Laufbahn-Startwert und seiner
/// Nummer — **unabhaengig davon, auf welchem Strang er laeuft**.
#[must_use]
pub const fn mische(a: u32, b: u32) -> u32 {
    let mut h = a ^ b.wrapping_mul(0x9e37_79b9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

/// Index des Stroms fuer die Siegwuerfe — er zieht nur, wenn eine Zelle
/// weder sicher gewinnt noch sicher verliert.
const SIEG_STROM: u32 = 5;

struct Zustand {
    stufe: Stufenstand,
    getragen: [Option<Stueck>; SLOT_COUNT],
    material: u64,
    besiegt: Vec<bool>,
    bestwert: Vec<Option<f64>>,
}

impl Zustand {
    fn neu() -> Self {
        Self {
            stufe: Stufenstand {
                level: START_LEVEL,
                xp: 0,
            },
            getragen: [None; SLOT_COUNT],
            material: 0,
            besiegt: vec![false; BOSSES.len()],
            bestwert: vec![None; BOSSES.len()],
        }
    }

    fn getragene(&self) -> Vec<Stueck> {
        self.getragen.iter().flatten().copied().collect()
    }

    fn standbild(&self) -> Standbild {
        let mut bild = Standbild {
            stufensumme: 0,
            ueber_common: 0,
            belegt: 0,
        };
        for stueck in self.getragen.iter().flatten() {
            bild.stufensumme += stueck.effektive_stufe();
            bild.belegt += 1;
            if stueck.quality > QUALITY_COMMON {
                bild.ueber_common += 1;
            }
        }
        bild
    }
}

/// Die Bosse der Spielinstanzen, in Instanz- und Datenreihenfolge, als
/// Indizes in [`BOSSES`].
fn inhalt() -> Vec<(usize, usize)> {
    let mut raus = Vec::new();
    for (instanz_index, instanz) in INSTANZEN.iter().enumerate() {
        for (boss_index, boss) in BOSSES.iter().enumerate() {
            if boss.instance == instanz.id {
                raus.push((instanz_index, boss_index));
            }
        }
    }
    raus
}

fn offene_instanzen(regeln: &Laufbahnregeln, zustand: &Zustand) -> Vec<usize> {
    match regeln.freischaltung {
        Freischaltung::Hauptpfad => freigeschaltete_instanzen(|i| zustand.besiegt[i]),
        Freischaltung::AlleOffen => (0..INSTANZEN.len()).collect(),
    }
}

fn abgeschlossen(instanz_index: usize, zustand: &Zustand) -> bool {
    let id = INSTANZEN[instanz_index].id;
    let mut drin = BOSSES
        .iter()
        .enumerate()
        .filter(|(_, b)| b.instance == id)
        .peekable();
    drin.peek().is_some() && drin.all(|(i, _)| zustand.besiegt[i])
}

/// Die Obergrenze der Aufwertung: die Stufe, die in der hoechsten offenen
/// Instanz faellt — der Inhalt, in dem die Spielerin steht.
fn deckel_instanz(offen: &[usize]) -> i32 {
    offen
        .iter()
        .map(|i| INSTANZEN[*i].number)
        .max()
        .map_or(1, |n| i32::try_from(n).unwrap_or(i32::MAX))
}

/// Rechnet eine Laufbahn.
///
/// `nummer` und `startwert` bestimmen sie vollstaendig; das Orakel liefert
/// nur, was der Kampf ergibt, und darf zwischen Laufbahnen geteilt werden —
/// eine Zelle ist eine reine Funktion ihres Schluessels.
///
/// # Errors
///
/// [`OrakelFehler`], wenn das Orakel eine Zelle nicht rechnen kann.
pub fn laufbahn(
    nummer: u32,
    startwert: u32,
    regeln: &Laufbahnregeln,
    auftrag: &Messauftrag,
    orakel: &mut impl Kampforakel,
) -> Result<Laufbahn, OrakelFehler> {
    let eigen = mische(startwert, nummer);
    let mut siegwurf = RandomStream::new(strom_startwert(eigen, SIEG_STROM));
    let katalog: Vec<Vorlage> = regeln.katalog.iter().map(|i| KATALOG[*i]).collect();
    let inhalt = inhalt();
    let mut zustand = Zustand::neu();
    let mut raus = Laufbahn {
        siege: Vec::with_capacity((auftrag.siege as usize).min(VERLAUF_SIEGE)),
        siege_gesamt: 0,
        punkte_gesamt: 0,
        stufe_nach_kaempfen: {
            let mut stufen = vec![None; crate::data::MAX_LEVEL as usize];
            stufen[START_LEVEL as usize - 1] = Some(0);
            stufen
        },
        fund_beim_sieg: None,
        geoeffnet_nach: vec![None; INSTANZEN.len()],
        abgeschlossen_nach: vec![None; INSTANZEN.len()],
        stand_bei_oeffnung: vec![None; INSTANZEN.len()],
        kaempfe: 0,
        ueber_deckel: i64::MIN,
        material_ungenutzt: 0,
    };
    let mut offen = offene_instanzen(regeln, &zustand);
    for i in &offen {
        raus.geoeffnet_nach[*i] = Some(0);
        raus.stand_bei_oeffnung[*i] = Some(zustand.standbild());
    }
    let mut nach_niederlage = false;

    while raus.siege_gesamt < auftrag.siege && raus.kaempfe < auftrag.max_kaempfe {
        // ---- Bosswahl
        let ziel = inhalt
            .iter()
            .find(|(i, b)| offen.contains(i) && !zustand.besiegt[*b])
            .map(|(_, b)| *b);
        let farm = inhalt
            .iter()
            .filter(|(_, b)| zustand.besiegt[*b])
            .max_by_key(|(i, _)| INSTANZEN[*i].number)
            .map(|(_, b)| *b);
        let boss_index = match (ziel, farm) {
            (Some(z), Some(f)) => {
                if nach_niederlage {
                    f
                } else {
                    z
                }
            }
            (Some(z), None) => z,
            (None, Some(f)) => f,
            (None, None) => break,
        };
        let boss = &BOSSES[boss_index];

        // ---- Kampf
        let getragene = zustand.getragene();
        let zelle = orakel.zelle(&OrakelSchluessel {
            ausstattung: worn_from(&getragene),
            boss: boss.id,
            schwierigkeit: 0,
            leiterstufe: 0,
        })?;
        raus.kaempfe += 1;
        let gewonnen = siegwurf.chance(zelle.siegquote());
        if !gewonnen {
            nach_niederlage = Some(boss_index) == ziel;
            continue;
        }
        nach_niederlage = false;

        // ---- Punkte und Erfahrung, beide auf der Stufe VOR dem Sieg
        let boss_level = i64::from(boss_stufe(boss.id));
        let spieler = i64::from(zustand.stufe.level);
        let quelle = match boss.runaway {
            Some(_) => {
                let hp = f64::from(boss.hp);
                let gehalten = if hp > 0.0 {
                    ((100.0 * (hp - zelle.schaden)) / hp).clamp(0.0, 100.0)
                } else {
                    0.0
                };
                Punktequelle {
                    victory: true,
                    remaining_ms: 0.0,
                    held_percent: Some(gehalten),
                }
            }
            None => Punktequelle {
                victory: true,
                remaining_ms: (f64::from(boss.overpressure_ms) - zelle.siegzeit_ms).max(0.0),
                held_percent: None,
            },
        };
        let punkte = points_breakdown(&quelle, boss_level, spieler, zustand.bestwert[boss_index]);
        let wert = quelle.held_percent.unwrap_or(quelle.remaining_ms);
        zustand.bestwert[boss_index] = Some(match zustand.bestwert[boss_index] {
            Some(vorher) => vorher.max(wert),
            None => wert,
        });
        zustand.stufe = grant_xp(zustand.stufe, xp_for_victory(boss_level, spieler));
        zustand.besiegt[boss_index] = true;

        // ---- Beute, aus dem eigenen Strom dieses Kampfes
        let instanz_nummer = instanz(boss.instance).map_or(1, |i| i.number);
        let mut strom = super::regeln::loot_stream_for(mische(eigen, raus.kaempfe));
        let fund = roll_loot(
            &mut strom,
            &Fundort {
                instance: i32::try_from(instanz_nummer).unwrap_or(1),
                difficulty: 0,
                endless_level: 0,
                boss_id: boss.id,
            },
            &katalog,
            regeln.fallrate_promille,
        );
        let sieg_nummer = raus.siege_gesamt + 1;
        if let Some(fund) = fund {
            let vorlage = regeln.katalog[fund.vorlage];
            if raus.fund_beim_sieg.is_none()
                && vorlage == auftrag.fund.vorlage
                && fund.quality >= auftrag.fund.mindest_guete
            {
                raus.fund_beim_sieg = Some(sieg_nummer);
            }
            anlegen_oder_umwandeln(
                regeln,
                &mut zustand,
                Stueck {
                    vorlage,
                    item_level: fund.item_level,
                    quality: fund.quality,
                    upgrade_steps: 0,
                    platz: None,
                },
            );
        }

        // ---- Freischaltung
        let vorher = offen;
        offen = offene_instanzen(regeln, &zustand);
        for i in &offen {
            if !vorher.contains(i) && raus.geoeffnet_nach[*i].is_none() {
                raus.geoeffnet_nach[*i] = Some(raus.kaempfe);
                raus.stand_bei_oeffnung[*i] = Some(zustand.standbild());
            }
        }
        for i in 0..INSTANZEN.len() {
            if raus.abgeschlossen_nach[i].is_none() && abgeschlossen(i, &zustand) {
                raus.abgeschlossen_nach[i] = Some(raus.kaempfe);
            }
        }

        // ---- Aufwertung, gegen die Obergrenze des aktuellen Inhalts
        let deckel = deckel_instanz(&offen);
        if regeln.aufwerten == Aufwerten::NiedrigsteZuerst {
            aufwerten(regeln, &mut zustand, deckel);
        }
        let grenze = i64::from(super::regeln::upgrade_cap_of(deckel, 0, 0));
        for stueck in zustand.getragen.iter().flatten() {
            raus.ueber_deckel = raus
                .ueber_deckel
                .max(i64::from(stueck.effektive_stufe()) - grenze);
        }

        raus.siege_gesamt = sieg_nummer;
        raus.punkte_gesamt += punkte.awarded;
        // Ein Sieg kann mehrere Stufen auf einmal bringen; jede bekommt
        // diesen Kampf als ersten, auf dem sie stand.
        for stufe in raus
            .stufe_nach_kaempfen
            .iter_mut()
            .take(zustand.stufe.level as usize)
        {
            stufe.get_or_insert(raus.kaempfe);
        }
        if raus.siege.len() < VERLAUF_SIEGE {
            raus.siege.push(Sieg {
                punkte: punkte.awarded,
                stufensumme: zustand.standbild().stufensumme,
                spielerstufe: zustand.stufe.level,
                kampf: raus.kaempfe,
            });
        }
    }
    raus.material_ungenutzt = zustand.material;
    Ok(raus)
}

fn anlegen_oder_umwandeln(regeln: &Laufbahnregeln, zustand: &mut Zustand, fund: Stueck) {
    let platz = KATALOG[fund.vorlage].slot;
    let anlegen = match regeln.anlegen {
        Anlegen::Spielvergleich => {
            let mut besitz = zustand.getragene();
            besitz.push(fund);
            compare_bag_item(&besitz, besitz.len() - 1).is_some_and(|v| v.verdict == COMPARE_BETTER)
        }
        Anlegen::HoehereStufe => {
            zustand.getragen[platz].is_none_or(|alt| fund.effektive_stufe() > alt.effektive_stufe())
        }
    };
    let weg = if anlegen {
        zustand.getragen[platz].replace(Stueck {
            platz: Some(platz),
            ..fund
        })
    } else {
        Some(fund)
    };
    // **Was nicht behalten wird, wird Material** (13.6) — auch das verdraengte
    // Stueck, und ohne seine Aufwertung: Sie ist stueckgebunden, und das ist
    // der bekannte Nachteil, den 13.6 selbst nennt.
    if let Some(stueck) = weg {
        zustand.material += material_for_mit(&regeln.material_je_qualitaet, stueck.quality as i64);
    }
}

/// Das Material in Aufwertung, je eine Stufe auf das niedrigste Stueck.
///
/// **Die Obergrenze steht hier NICHT** — nur in [`upgrade_item_mit`]. Gibt
/// die Regel fuer das niedrigste Stueck nichts mehr her, liegt jedes andere
/// schon mindestens so hoch, und die Schleife endet. Stuende die Grenze hier
/// ein zweites Mal als Filter, haelt die Laufbahn sie auch dann, wenn die
/// Regel sie verloere — und kein Prueffall saehe es.
fn aufwerten(regeln: &Laufbahnregeln, zustand: &mut Zustand, deckel: i32) {
    loop {
        if zustand.material < regeln.kosten_je_stufe {
            return;
        }
        let Some(platz) = (0..SLOT_COUNT)
            .filter_map(|p| zustand.getragen[p].map(|s| (p, s.effektive_stufe())))
            .min_by_key(|(p, stufe)| (*stufe, *p))
            .map(|(p, _)| p)
        else {
            return;
        };
        let Some(stueck) = zustand.getragen[platz].as_mut() else {
            return;
        };
        let vorher = stueck.effektive_stufe();
        let r = upgrade_item_mit(
            regeln.kosten_je_stufe,
            i64::from(vorher),
            regeln.kosten_je_stufe as i64,
            deckel,
            0,
            0,
        );
        if r.spent == 0 {
            return;
        }
        let gewonnen = u32::try_from(r.level - i64::from(vorher)).unwrap_or(0);
        stueck.upgrade_steps += gewonnen;
        zustand.material -= r.spent;
    }
}
