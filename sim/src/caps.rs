//! Werte-Deckel je Wirkgroesse, als Tabelle (Issue #959).
//!
//! # Was ein Deckel hier ist — und was er nicht ist
//!
//! **Jeder Deckel ist so HOCH gesetzt, wie die Rechnung ihn traegt**, begrenzt
//! allein durch die Bitbreite der Zwischenwerte und die Grenze, bis zu der die
//! Browser-Seite exakt rechnet. Er ist eine **Rechenschranke**, keine
//! Balance-Aussage.
//!
//! Ein Deckel, der niedriger sitzt als technisch noetig, trifft eine
//! Spielentscheidung, ohne dass sie jemand als solche getroffen hat — und
//! niemand findet sie spaeter wieder. Deshalb steht zu jedem Deckel, **welche
//! Rechenkette** ihn begrenzt: Eine Deckelzahl ohne Herleitung ist von einer
//! geratenen nicht zu unterscheiden.
//!
//! # Der Ausruestungskollaps wird hier NICHT geloest
//!
//! Gemessen am 2026-08-18 traegt `catalysis` den Kollaps, und zwar ohne
//! Kipppunkt: 1000 Promille lassen 10 von 58 Zaubern lohnend, 4000 noch 7,
//! 12000 noch 5, 36000 noch 4. **Es gibt keine Kante, hinter der es kippt** —
//! jede Hoehe kostet Vielfalt, und wie viel davon gewollt ist, ist eine
//! Spielentscheidung. Wer hier einen Deckel enger zieht, um das zu erschlagen,
//! verstoesst gegen die Regel oben.
//!
//! # Die Browser-Grenze
//!
//! Das Spiel rechnet in JavaScript, also in `f64`. Ganze Zahlen sind dort bis
//! **2^53** exakt; darueber verliert die Mantisse Stellen, und die
//! Bitgleichheit zwischen Simulator und Spiel waere nicht mehr zu halten.
//! Diese Grenze ist in [`BROWSER_EXAKT_BIS`] festgehalten und bindet ueberall
//! dort, wo ein Zwischenwert ganzzahlig durch beide Umsetzungen laeuft.

use crate::factors::DamageFactors;
use crate::gear::WornValues;

/// Bis hierhin rechnet die Browser-Seite ganze Zahlen exakt.
///
/// `Number.MAX_SAFE_INTEGER` ist 2^53 − 1. Ein ganzzahliger Zwischenwert
/// darueber ist in JavaScript nicht mehr eindeutig darstellbar, und zwei
/// verschiedene Rechenwege koennen dasselbe Bitmuster liefern — genau das,
/// was die Paritaets-Pruefungen ausschliessen sollen.
pub const BROWSER_EXAKT_BIS: u64 = (1u64 << 53) - 1;

/// Warum eine Wirkgroesse **keinen** Deckel braucht.
///
/// # Warum das ein Typ ist und kein Kommentar
///
/// AC2 verlangt, dass eine Ausnahme ihre Begruendung **im Code** traegt und
/// maschinell auffindbar ist. Ein `//`-Kommentar erfuellt das nicht: Er laesst
/// sich nicht aufzaehlen, nicht pruefen und nicht in einen Bericht schreiben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AusnahmeGrund {
    /// Die Groesse geht ausschliesslich in einen **Nenner** ein.
    ///
    /// Ein groesserer Wert macht das Ergebnis kleiner; ein Ueberlauf nach oben
    /// ist damit ausgeschlossen, solange der Typ ihn traegt.
    NurNenner,
    /// Die Groesse ist an ihrer Wirkstelle bereits **gedeckelt**.
    ///
    /// Ein groesserer Wert aendert das Ergebnis nicht mehr. Ein zusaetzlicher
    /// Deckel waere eine zweite Wahrheit ueber dieselbe Grenze.
    AmEinsatzGedeckelt,
}

/// Ein Deckel mit seiner Herleitung, oder eine begruendete Ausnahme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deckel {
    /// Obergrenze, hergeleitet aus einer benannten Rechenkette.
    Grenze {
        /// Der groesste zulaessige Wert.
        max: u64,
        /// Welche Bitbreite ihn begrenzt.
        bitbreite: &'static str,
        /// Die engste Rechenkette — die Stelle, die zuerst bricht.
        engste_kette: &'static str,
    },
    /// Kein Deckel noetig, mit maschinenlesbarer Begruendung.
    Ausnahme {
        /// Warum.
        grund: AusnahmeGrund,
        /// Die Kette, an der das nachvollziehbar ist.
        engste_kette: &'static str,
    },
}

impl Deckel {
    /// Ob dieser Wert die Schranke einhaelt.
    ///
    /// Eine Ausnahme haelt jeden Wert — das ist ihre Aussage.
    pub const fn haelt(&self, wert: u64) -> bool {
        match self {
            Deckel::Grenze { max, .. } => wert <= *max,
            Deckel::Ausnahme { .. } => true,
        }
    }

    /// Die Obergrenze, falls es eine gibt.
    pub const fn max(&self) -> Option<u64> {
        match self {
            Deckel::Grenze { max, .. } => Some(*max),
            Deckel::Ausnahme { .. } => None,
        }
    }

    /// Die Rechenkette, aus der sich Deckel oder Ausnahme ergibt.
    pub const fn engste_kette(&self) -> &'static str {
        match self {
            Deckel::Grenze { engste_kette, .. } | Deckel::Ausnahme { engste_kette, .. } => {
                engste_kette
            }
        }
    }
}

/// Eine Wirkgroesse mit ihrem Deckel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wirkgroesse {
    /// Der Feldname, wie er in der Struktur steht.
    pub name: &'static str,
    /// Zu welcher Traegerstruktur die Groesse gehoert.
    pub traeger: Traeger,
    /// Deckel oder Ausnahme.
    pub deckel: Deckel,
}

/// Woher eine Wirkgroesse stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traeger {
    /// [`DamageFactors`] — die Faktoren der Schadensformel, `f64`.
    Faktor,
    /// [`WornValues`] — die getragenen Ausruestungswerte, `u32` in Promille.
    Getragen,
}

// Die engste Kette der Schadensformel, mehrfach verwendet.
const KETTE_SCHADEN: &str =
    "factors::compute_damage — base_damage als f64, acht Faktoren multipliziert, \
     dann `round_js(value) as u32`; das SAETTIGT bei u32::MAX statt zu ueberlaufen, \
     und eine Saettigung ist eine stille falsche Zahl";

/// Der groesste Wert, den [`crate::factors::compute_damage`] zurueckgeben kann.
///
/// `as u32` saettigt — der Fehler waere also nicht ein Absturz, sondern eine
/// **stille falsche Zahl**. Deshalb ist das Produkt der Faktoren so begrenzt,
/// dass es diese Grenze nicht erreicht.
pub const SCHADEN_MAX: u64 = u32::MAX as u64;

/// Der groesste Wert, den `Fight::fehlend_prozent` traegt.
///
/// Dort steht `vessel_max.saturating_sub(player_hp) * 100 / vessel_max` — die
/// Multiplikation mit 100 laeuft in `u32` ueber, sobald die Differenz
/// `u32::MAX / 100` uebersteigt. **Das ist die engste Kette der Lebenspunkte**,
/// enger als `gear::vessel_with_gear`, das sauber auf `u32::MAX` saettigt.
pub const VESSEL_MAX: u64 = (u32::MAX as u64) / 100;

/// Die Tabelle aller Wirkgroessen.
///
/// **Vollstaendig gegenueber dem, was der Simulator kennt** — geprueft nicht
/// gegen diese Liste, sondern gegen die Typen: [`faktoren_aufgezaehlt`] und
/// [`getragene_aufgezaehlt`] destrukturieren ihre Struktur **ohne** `..`, ein
/// neues Feld bricht dort den Bau.
pub const WIRKGROESSEN: &[Wirkgroesse] = &[
    // ---- DamageFactors, f64 ------------------------------------------------
    Wirkgroesse {
        name: "essence",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "talents",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "gear_effects",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "buffs",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "synergy",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "resistance",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "vulnerability",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    Wirkgroesse {
        name: "crit",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette: KETTE_SCHADEN,
        },
    },
    // ---- WornValues, u32 in Promille ---------------------------------------
    Wirkgroesse {
        name: "catalysis",
        traeger: Traeger::Getragen,
        deckel: Deckel::Ausnahme {
            grund: AusnahmeGrund::NurNenner,
            engste_kette:
                "haste::hasted_ms — `base_ms * 1000 / (1000 + permille)` in u64; catalysis \
                 steht ALLEIN im Nenner, ein groesserer Wert macht die Zeit kuerzer und den \
                 Zwischenwert kleiner. Der Zaehler haengt nicht davon ab.",
        },
    },
    Wirkgroesse {
        name: "purity",
        traeger: Traeger::Getragen,
        deckel: Deckel::Ausnahme {
            grund: AusnahmeGrund::AmEinsatzGedeckelt,
            engste_kette:
                "gear::WornValues::crit_chance — `if purity >= 1000 { return 1.0 }`. Eine \
                 Wahrscheinlichkeit ueber eins gibt es nicht; jeder groessere Wert ist \
                 wirkungslos, nicht gefaehrlich.",
        },
    },
    Wirkgroesse {
        name: "yield_bonus",
        traeger: Traeger::Getragen,
        deckel: Deckel::Grenze {
            max: (u32::MAX as u64) - 1500,
            bitbreite: "u32 der Addition",
            engste_kette:
                "gear::WornValues::crit_factor — `CRIT_BASE_PERMILLE + yield_bonus` wird in \
                 u32 addiert, BEVOR nach f64 gewandelt wird. CRIT_BASE_PERMILLE ist 1500, \
                 also bleibt u32::MAX - 1500.",
        },
    },
    Wirkgroesse {
        name: "essence",
        traeger: Traeger::Getragen,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette:
                "gear::WornValues::essence_factor — `1.0 + essence / 1000.0` kann als f64 \
                 nicht ueberlaufen, aber der Faktor geht in compute_damage ein und teilt \
                 sich dessen Schranke.",
        },
    },
    Wirkgroesse {
        name: "vessel",
        traeger: Traeger::Getragen,
        deckel: Deckel::Grenze {
            max: VESSEL_MAX,
            bitbreite: "u32 der Multiplikation mit 100",
            engste_kette:
                "fight::Fight::fehlend_prozent — `vessel_max.saturating_sub(player_hp) * 100 \
                 / vessel_max`. Die Multiplikation mit 100 steht in u32 und laeuft ueber, \
                 sobald die Differenz u32::MAX/100 uebersteigt. gear::vessel_with_gear \
                 saettigt sauber auf u32::MAX und ist damit NICHT die engste Stelle.",
        },
    },
    Wirkgroesse {
        name: "ward",
        traeger: Traeger::Getragen,
        deckel: Deckel::Grenze {
            max: (u32::MAX as u64) / 2,
            bitbreite: "u32 der Addition mit dem Zersetzungs-Anteil",
            engste_kette:
                "fight.rs — `self.worn.ward + decay_permille` wird in u32 addiert, BEVOR \
                 data::PlayerBase::incoming_with_gear die Minderung bei 1.0 deckelt. Der \
                 Deckel bei 1.0 schuetzt also die Rechnung DAHINTER, nicht die Addition \
                 davor; die Haelfte von u32::MAX traegt jeden zweiten Summanden mit.",
        },
    },
    Wirkgroesse {
        name: "gear_effects",
        traeger: Traeger::Getragen,
        deckel: Deckel::Grenze {
            max: SCHADEN_MAX,
            bitbreite: "u32 des Rueckgabewerts von compute_damage",
            engste_kette:
                "gear::WornValues::gear_effect_factor — `1.0 + gear_effects / 1000.0` kann \
                 als f64 nicht ueberlaufen, aber der Faktor geht in compute_damage ein und \
                 teilt sich dessen Schranke.",
        },
    },
];

/// Die Faktoren, **aus dem Typ aufgezaehlt**.
///
/// # Hier bricht der Bau bei einer neuen Wirkgroesse (AC4)
///
/// Die Destrukturierung nennt jedes Feld einzeln und benutzt **kein** `..`.
/// Kommt ein Feld zu [`DamageFactors`] dazu, meldet der Uebersetzer
/// `E0027: pattern does not mention field` — genauso wie ein `match` ohne
/// Auffangzweig bei einer neuen Aufzaehlungsvariante.
///
/// **Das ist der Unterschied zu einer gepflegten Liste:** Die Tabelle oben
/// koennte altern, diese Funktion nicht.
pub fn faktoren_aufgezaehlt(f: &DamageFactors) -> [(&'static str, f64); 8] {
    let DamageFactors {
        essence,
        talents,
        gear_effects,
        buffs,
        synergy,
        resistance,
        vulnerability,
        crit,
    } = f;
    [
        ("essence", *essence),
        ("talents", *talents),
        ("gear_effects", *gear_effects),
        ("buffs", *buffs),
        ("synergy", *synergy),
        ("resistance", *resistance),
        ("vulnerability", *vulnerability),
        ("crit", *crit),
    ]
}

/// Die getragenen Werte, **aus dem Typ aufgezaehlt**.
///
/// Derselbe Mechanismus wie [`faktoren_aufgezaehlt`]: kein `..`, also bricht
/// ein neues Feld den Bau.
pub fn getragene_aufgezaehlt(w: &WornValues) -> [(&'static str, u32); 7] {
    let WornValues {
        catalysis,
        purity,
        yield_bonus,
        essence,
        vessel,
        ward,
        gear_effects,
    } = w;
    [
        ("catalysis", *catalysis),
        ("purity", *purity),
        ("yield_bonus", *yield_bonus),
        ("essence", *essence),
        ("vessel", *vessel),
        ("ward", *ward),
        ("gear_effects", *gear_effects),
    ]
}

/// Der Eintrag zu einer Wirkgroesse, falls es einen gibt.
pub fn deckel_fuer(name: &str, traeger: Traeger) -> Option<&'static Wirkgroesse> {
    WIRKGROESSEN
        .iter()
        .find(|g| g.name == name && g.traeger == traeger)
}

/// Eine Ueberschreitung, wie AC0b sie zu melden verlangt.
///
/// Sie nennt **die Groesse, ihren Deckel und den Wert, den der Optimierer
/// erreichen wollte** — nicht bloss die Tatsache, dass etwas anstiess. Ohne
/// den gewollten Wert liesse sich nicht beurteilen, ob der Deckel knapp oder
/// weit verfehlt wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ueberschreitung {
    /// Welche Groesse.
    pub name: &'static str,
    /// Aus welcher Struktur.
    pub traeger: Traeger,
    /// Ihr Deckel.
    pub deckel: u64,
    /// Der Wert, der erreicht werden sollte.
    pub gewollt: u64,
}

impl Ueberschreitung {
    /// Um welchen Faktor der gewollte Wert ueber dem Deckel liegt.
    ///
    /// Steht in der Meldung, weil „knapp darueber" und „tausendfach darueber"
    /// verschiedene Befunde sind.
    pub fn ueberschuss(&self) -> f64 {
        if self.deckel == 0 {
            return f64::INFINITY;
        }
        self.gewollt as f64 / self.deckel as f64
    }
}

/// Prueft einen einzelnen Wert gegen seinen Deckel (AC0b).
///
/// Gibt `None` zurueck, wenn der Wert haelt **oder** die Groesse eine
/// begruendete Ausnahme ist.
pub fn pruefe(name: &'static str, traeger: Traeger, wert: u64) -> Option<Ueberschreitung> {
    let eintrag = deckel_fuer(name, traeger)?;
    if eintrag.deckel.haelt(wert) {
        return None;
    }
    Some(Ueberschreitung {
        name: eintrag.name,
        traeger,
        deckel: eintrag.deckel.max()?,
        gewollt: wert,
    })
}

/// Prueft einen ganzen Satz getragener Werte gegen die Tabelle.
///
/// **Ueber die Aufzaehlung aus dem Typ**, nicht ueber die Tabelle: Eine neue
/// Groesse wird dadurch mitgeprueft, sobald sie existiert.
pub fn pruefe_getragene(w: &WornValues) -> Vec<Ueberschreitung> {
    getragene_aufgezaehlt(w)
        .into_iter()
        .filter_map(|(name, wert)| pruefe(name, Traeger::Getragen, u64::from(wert)))
        .collect()
}
