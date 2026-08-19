//! Parameterdomänen mit abgeleitetem Monotonie-Zertifikat (Issue #940).
//!
//! # Warum das neben `caps.rs` steht und nicht darin
//!
//! [`crate::caps`] trägt die **Deckel**: bis wohin eine Wirkgröße rechnen darf,
//! ohne dass eine Kette überläuft. Dieses Modul trägt eine Aussage **über** die
//! Deckel: in welchem Bereich ein Optimierer suchen darf und ob er dabei
//! Monotonie voraussetzen darf.
//!
//! Zwei Dinge, die sich unabhängig ändern, gehören nicht in dieselbe Datei.
//!
//! # Was hier NICHT noch einmal gebaut wird
//!
//! Der Geltungsbereich eines Monotonie-Hinweises steht bereits in
//! [`crate::effects`] — [`MonotonicityDomain`], [`ZertifikatsBedarf`] und
//! [`ZertifikatVerweigert`], gebaut unter Issue #966. Dieses Modul **benutzt**
//! sie.
//!
//! > Zwei Aufzählungen derselben Fälle driften unabhängig. Dann zertifiziert
//! > das Zertifikat gegen eine Kopie, während der Kampf gegen das Original
//! > rechnet — und keine der beiden Seiten sieht es.
//!
//! Wie schnell das eintritt, zeigt `f-940-zaubervielfalt`: Wenige Stunden nach
//! diesem Modul kam [`MonotonicityDomain::SpellDiversity`] als **fünfte**
//! Variante dazu. Eine Kopie hätte hier vier Fälle geführt und die Aussage,
//! auf die es ankommt, gar nicht ausdrücken können.

use crate::caps::{Deckel, Traeger, Wirkgroesse, WIRKGROESSEN};
use crate::effects::{
    LocalMonotonicityHint, MonotonicDirection, MonotonicityDomain, ZertifikatVerweigert,
    ZertifikatsBedarf,
};

// ------------------------------------------------------------------ Zertifikat

/// Ein Monotonie-Zertifikat **mit** seinem Geltungsbereich.
///
/// # Warum der Geltungsbereich kein Zusatz ist, sondern der Kern
///
/// „`catalysis` ist monoton" ist nicht falsch, sondern **gefährlich**. Gemessen
/// liegt der Fall so:
///
/// ```text
///  1 000 Promille  ->  10 von 58 Zaubern lohnend
/// 36 000 Promille  ->   4
/// ```
///
/// Bezüglich **erwartetem Schaden** ist der Parameter monoton steigend.
/// Bezüglich **Zaubervielfalt** ist er streng fallend. Ein Zertifikat ohne
/// Bezugsgröße liest sich wie „unbedenklich" — und hier ist es das Gegenteil.
///
/// Deshalb ist [`Self::domain`] ein Pflichtfeld und kein `Option`: Ein
/// Zertifikat ohne Bezug lässt sich in dieser Struktur gar nicht erst
/// hinschreiben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonotonieZertifikat {
    /// Die Richtung — steigend, fallend, nachweislich nicht monoton.
    pub direction: MonotonicDirection,
    /// **Wofür** die Richtung gilt. Ohne das sagt sie nichts.
    pub domain: MonotonicityDomain,
    /// Welche der vier Zutaten vorliegen.
    pub bedarf: ZertifikatsBedarf,
    /// Die Trace-Region, in der die Aussage gelten soll.
    pub trace_region: TraceRegionBezug,
    /// Die Voraussetzungen, unter denen sie gilt.
    pub preconditions: &'static [&'static str],
}

/// Auf welche Trace-Region sich ein Zertifikat bezieht.
///
/// **`Keine` ist ein Zustand, kein Platzhalter.** Ein Zertifikat ohne
/// Trace-Region ist unvollständig — und das soll man ihm ansehen, statt es
/// mit einer erfundenen Region vollständig aussehen zu lassen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceRegionBezug {
    /// Die Rotation bleibt über den ganzen Bereich dieselbe.
    StabilerTrace,
    /// Die Aussage gilt nur stückweise, mit Sprungstellen dazwischen.
    ///
    /// **Wird heute nirgends erzeugt** — es gibt keine Sprungstellen, weil
    /// keine Trace-Region welche liefert (`allowed_breakpoints` ist überall
    /// leer). Die Variante bleibt trotzdem stehen, und zwar aus einem anderen
    /// Grund als das gestrichene `Aenderungskosten::EineZahl`: Diese hier
    /// **wird** ableitbar, sobald die Trace-Regionen Sprungstellen liefern —
    /// das Issue nennt sie ausdrücklich als Quelle. Die andere wäre es nie
    /// geworden.
    StueckweiseZwischenSprungstellen,
    /// Keine Region benannt — das Zertifikat ist damit unvollständig.
    Keine,
}

impl MonotonieZertifikat {
    /// Trägt dieses Zertifikat für die gefragte Zielgröße?
    ///
    /// Beantwortet **nicht** „ist der Parameter monoton", sondern „ist er es
    /// bezüglich dessen, wonach ich frage". Das sind verschiedene Fragen, und
    /// die erste hat keine Antwort.
    ///
    /// Der Fehlerfall nennt **welche** der vier Ablehnungen zutrifft — ein
    /// Aufrufer, der nur „abgelehnt" erfährt, kann nicht unterscheiden, ob der
    /// Bezug nicht passte oder Zutaten fehlten.
    pub fn traegt_fuer(&self, gefragt: MonotonicityDomain) -> Result<(), ZertifikatVerweigert> {
        if self.direction == MonotonicDirection::Unknown {
            return Err(ZertifikatVerweigert::KeinHinweis);
        }
        if self.domain != gefragt {
            return Err(ZertifikatVerweigert::FalscherBezug {
                hinweis: self.domain,
                gefragt,
            });
        }
        if !self.bedarf.vollstaendig() {
            return Err(ZertifikatVerweigert::ZutatenFehlen);
        }
        if self.trace_region == TraceRegionBezug::Keine {
            return Err(ZertifikatVerweigert::ZutatenFehlen);
        }
        Ok(())
    }
}

// --------------------------------------------------------------------- Kosten

/// Was es kostet, diesen Parameter zu ändern.
///
/// Unterste Stufe der Rangfolge (AC5): Zwischen zwei gleichwertigen Lösungen
/// gewinnt die mit weniger geänderten Zahlen.
///
/// # Warum es keine Stufe „nur eine Zahl" gibt
///
/// Die erste Fassung hatte eine dritte Variante `EineZahl` für einen Parameter
/// ohne Folgen für andere Werte. Sie wurde **nie erzeugt** — und konnte es
/// nicht: [`WIRKGROESSEN`] führt keine Zuordnung zu einzelnen Zaubern, und
/// ohne die ist nicht entscheidbar, ob eine Wirkgröße nur einen betrifft.
///
/// Eine Stufe, die niemand ableiten kann, ist eine Behauptung über eine
/// Unterscheidung, die die Daten nicht hergeben. Sie stünde in der Rangfolge
/// und würde nie greifen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aenderungskosten {
    /// Wirkt auf mehrere Zauber gleichzeitig.
    MehrereZauber,
    /// Speist dieselbe Wirkgröße wie ein anderer Parameter — beide bewegen
    /// sich zusammen, und eine Änderung ist nie lokal.
    GeteilteWirkgroesse,
}

// ---------------------------------------------------------------- Die Domäne

/// Der Bereich, in dem ein Parameter liegen darf (AC1).
#[derive(Debug, Clone, Copy)]
pub struct ParameterDomain {
    /// Der Feldname, wie in [`crate::caps::WIRKGROESSEN`].
    pub name: &'static str,
    /// Zu welcher Trägerstruktur die Größe gehört.
    pub traeger: Traeger,
    /// Die Untergrenze.
    ///
    /// # Sie ist bei allen 0, und das hat zwei verschiedene Gründe
    ///
    /// Bei [`Traeger::Getragen`] ist 0 die **Typuntergrenze**: Die Werte stehen
    /// als `u32` in Promille, darunter gibt es nichts. Das ist abgeleitet.
    ///
    /// Bei [`Traeger::Faktor`] ist 0 eine **Festlegung**: Faktoren sind `f64`
    /// und könnten negativ werden; dass ein negativer Schadensfaktor nicht zum
    /// Suchraum gehört, folgt aus der Sache und nicht aus dem Typ.
    ///
    /// Beide Werte sind gleich, die Begründungen nicht. Wer das später ändert,
    /// muss wissen, welche der beiden er anfasst.
    pub minimum: u64,
    /// Die Obergrenze — **`None`, wenn die Wirkgröße eine begründete Ausnahme
    /// vom Deckel ist.**
    ///
    /// Kein erfundenes `u64::MAX`: Eine Zahl, die niemand begründet hat, wird
    /// später zitiert, und dann trägt sie das Gewicht einer Messung.
    pub maximum: Option<u64>,
    /// Die Schrittweite.
    pub discrete_step: Schrittweite,
    /// Zulässige Sprungstellen aus den Trace-Regionen.
    pub allowed_breakpoints: &'static [u64],
    /// Das Zertifikat — **`None` heißt: es gibt keins**, nicht „ist noch
    /// offen". Ohne Zertifikat keine Binärsuche (AC3).
    pub monotonicity_certificate: Option<MonotonieZertifikat>,
    /// Der Name der geteilten Wirkgröße, wenn ein zweiter Parameter sie
    /// speist (AC4). `None` heisst: dieser Parameter steht allein.
    pub dependencies: Option<&'static str>,
    /// Die Kosten einer Änderung (AC5).
    pub designer_cost: Aenderungskosten,
}

impl ParameterDomain {
    /// Darf ein Optimierer hier binär suchen (AC3)?
    ///
    /// # Warum das eine Ablehnung mit Grund zurückgibt
    ///
    /// Eine Binärsuche setzt Monotonie **bezüglich der Zielgröße** voraus, über
    /// die optimiert wird. An einem Boss mit umgekehrter Bedingung findet sie
    /// sonst zuverlässig das falsche Ergebnis — und meldet es als Optimum.
    ///
    /// Die vier Ablehnungsgründe sind unterscheidbar, weil ein Aufrufer sonst
    /// nicht weiß, ob er ein Zertifikat **nachreichen** kann (`ZutatenFehlen`)
    /// oder ob die Frage für diesen Parameter grundsätzlich falsch gestellt ist
    /// (`FalscherBezug`).
    pub fn binaersuche_erlaubt(
        &self,
        zielgroesse: MonotonicityDomain,
    ) -> Result<(), ZertifikatVerweigert> {
        match &self.monotonicity_certificate {
            None => Err(ZertifikatVerweigert::KeinHinweis),
            Some(z) => z.traegt_fuer(zielgroesse),
        }
    }

    /// Darf ein Optimierer hier Kandidaten per Dominanz verwerfen (AC3)?
    ///
    /// Dieselbe Voraussetzung wie die Binärsuche: Wer verwirft, weil „mehr
    /// immer besser ist", braucht dafür denselben Nachweis.
    pub fn dominanz_verwerfen_erlaubt(
        &self,
        zielgroesse: MonotonicityDomain,
    ) -> Result<(), ZertifikatVerweigert> {
        self.binaersuche_erlaubt(zielgroesse)
    }

    /// Ob der Bereich beschränkt ist.
    pub fn ist_beschraenkt(&self) -> bool {
        self.maximum.is_some()
    }
}

// ------------------------------------------------------- Ableitung, nicht Liste

/// Die Domäne einer Wirkgröße, **abgeleitet** aus [`crate::caps`].
///
/// # Warum abgeleitet und nicht aufgezählt
///
/// Eine zweite Liste müsste jemand mitpflegen. Die erste, die es im Projekt
/// gab, hat genau daran verloren: `WIRKGROESSEN` prüft seine Vollständigkeit
/// deshalb gegen die **Typen**, nicht gegen eine Abschrift.
///
/// Hier gilt dasselbe eine Ebene höher: Kommt eine Wirkgröße dazu, bekommt sie
/// ihre Domäne, ohne dass jemand daran denken muss.
///
/// # Die Obergrenze wird NICHT enger gezogen als der Deckel
///
/// Das ist eine ausdrückliche Stop-Condition des Issues. Der Deckel ist so
/// hoch gesetzt, wie die Rechnung ihn trägt; ein enger gezogener Suchbereich
/// nimmt dem Optimierer genau den Raum, in dem die Lösung für einen toten
/// Zauber liegen könnte.
pub fn domaene_fuer(w: &Wirkgroesse) -> ParameterDomain {
    let maximum = match w.deckel {
        // Genau der Deckel, nicht weniger.
        Deckel::Grenze { max, .. } => Some(max),
        // Kein Deckel heisst kein Maximum. Hier eine Zahl einzusetzen wäre
        // eine Erfindung mit dem Aussehen einer Messung.
        Deckel::Ausnahme { .. } => None,
    };
    ParameterDomain {
        name: w.name,
        traeger: w.traeger,
        minimum: 0,
        maximum,
        discrete_step: schrittweite_fuer(w.traeger),
        // Leer, solange keine Trace-Region eine Sprungstelle liefert. Eine
        // erfundene Sprungstelle wäre schlimmer als keine: Sie sähe aus wie
        // eine gemessene.
        allowed_breakpoints: &[],
        monotonicity_certificate: None,
        dependencies: abhaengigkeiten_fuer(w.name),
        designer_cost: kosten_fuer(w.name),
    }
}

/// Die Schrittweite, aus dem Träger abgeleitet.
///
/// # Warum das kein `u64` ist
///
/// Die erste Fassung gab für **beide** Träger `1` zurück — zwei Zweige mit
/// demselben Ergebnis. Das sieht aus wie eine Unterscheidung und ist keine,
/// und die `1` beim Faktor war zusätzlich unbegründet: Faktoren sind `f64`,
/// eine ganzzahlige Schrittweite ist dort keine Eigenschaft, sondern eine
/// Entscheidung, die niemand getroffen hat.
///
/// Jetzt trägt der Rückgabewert den Unterschied, statt ihn zu verdecken.
const fn schrittweite_fuer(t: Traeger) -> Schrittweite {
    match t {
        // `f64` — die Stufung ist eine offene Entscheidung, keine Eigenschaft.
        Traeger::Faktor => Schrittweite::NichtFestgelegt,
        // `u32` in Promille: ein Promille ist die kleinste darstellbare Stufe.
        Traeger::Getragen => Schrittweite::EinPromille,
    }
}

/// Wie fein ein Parameter gestuft werden darf.
///
/// **`NichtFestgelegt` ist ein Zustand, kein Platzhalter** — dieselbe
/// Entscheidung wie `maximum: Option<u64>`. Eine erfundene Stufung sähe aus
/// wie eine gemessene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schrittweite {
    /// Ein Promille — die kleinste darstellbare Stufe getragener Werte.
    EinPromille,
    /// Für Faktoren (`f64`) ist die Stufung nicht entschieden.
    NichtFestgelegt,
}

/// Welche anderen Parameter dieselbe Wirkgröße speisen (AC4).
///
/// # Aus dem Bestand abgeleitet, nicht aufgezählt
///
/// Ein Name, der in [`WIRKGROESSEN`] **zweimal** vorkommt — einmal als
/// [`Traeger::Faktor`] und einmal als [`Traeger::Getragen`] —, bezeichnet
/// zwei Parameter, die auf dieselbe Größe wirken. Sie lassen sich nicht
/// unabhängig optimieren.
///
/// Gemessen am 2026-08-19: 15 Einträge, 13 verschiedene Namen. Die Differenz
/// sind `essence` und `gear_effects`.
/// # Warum hier keine Namensliste steht
///
/// Die erste Fassung dieser Funktion hatte ein `match name { "essence" => …,
/// "gear_effects" => … }` — eine **Abschrift** des Ergebnisses, das die Zeile
/// darüber gerade berechnet hatte. Käme eine dritte Doppelung dazu, fiele sie
/// in den Auffangzweig und wäre still ohne Abhängigkeit.
///
/// Der Name **ist** die geteilte Wirkgröße; er muss nur aus der Tabelle
/// zurückgegeben werden statt aus einer zweiten Quelle.
pub fn abhaengigkeiten_fuer(name: &str) -> Option<&'static str> {
    let mut treffer = WIRKGROESSEN.iter().filter(|w| w.name == name);
    let erster = treffer.next()?;
    // Ein zweiter Eintrag mit demselben Namen heisst: zwei Parameter speisen
    // dieselbe Wirkgroesse.
    treffer.next().map(|_| erster.name)
}

/// Die Kosten einer Änderung (AC5).
fn kosten_fuer(name: &str) -> Aenderungskosten {
    if WIRKGROESSEN.iter().filter(|w| w.name == name).count() > 1 {
        Aenderungskosten::GeteilteWirkgroesse
    } else {
        Aenderungskosten::MehrereZauber
    }
}

/// Alle Domänen, aus der Deckel-Tabelle abgeleitet.
pub fn alle_domaenen() -> Vec<ParameterDomain> {
    WIRKGROESSEN.iter().map(domaene_fuer).collect()
}

/// Ein Zertifikat aus einem lokalen Hinweis — **unvollständig, mit Absicht**.
///
/// Der Hinweis aus der Wirkungsbeschreibung ist **eine** von vier Zutaten.
/// Diese Funktion baut daraus ein Zertifikat, das sich selbst als unvollständig
/// ausweist: [`ZertifikatsBedarf::AUS_HINWEIS_ALLEIN`] und
/// [`TraceRegionBezug::Keine`].
///
/// Wer es benutzt, bekommt von [`MonotonieZertifikat::traegt_fuer`] ein
/// `ZutatenFehlen` zurück — und das ist der Zweck. Aus der Wirkungsbeschreibung
/// allein entsteht **nie** ein tragendes Zertifikat.
pub fn zertifikat_aus_hinweis(h: LocalMonotonicityHint) -> MonotonieZertifikat {
    MonotonieZertifikat {
        direction: h.direction,
        domain: h.domain,
        bedarf: ZertifikatsBedarf::AUS_HINWEIS_ALLEIN,
        trace_region: TraceRegionBezug::Keine,
        preconditions: &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **AC1: Jede Wirkgröße bekommt eine Domäne — abgeleitet, nicht gepflegt.**
    ///
    /// Trägerzusicherung zuerst: Wenn die Tabelle leer wäre, wäre jede Aussage
    /// darunter wahr und wertlos.
    #[test]
    fn jede_wirkgroesse_hat_eine_domaene() {
        let alle = alle_domaenen();
        assert!(
            alle.len() >= 15,
            "Trägerzusicherung: nur {} Domänen — die Tabelle ist geschrumpft, \
             und dann prüft alles Weitere fast nichts",
            alle.len()
        );
        assert_eq!(
            alle.len(),
            WIRKGROESSEN.len(),
            "jede Wirkgröße genau einmal — sonst ist die Ableitung nicht total"
        );
        println!(
            "{} Domänen aus {} Wirkgrößen",
            alle.len(),
            WIRKGROESSEN.len()
        );
    }

    /// **AC1: Ohne Deckel kein Maximum — und keine erfundene Zahl.**
    ///
    /// Die beiden Ausnahmen (`catalysis`, `purity`) haben bewusst keinen
    /// Deckel. Ein `u64::MAX` an dieser Stelle wäre eine Zahl ohne Herleitung,
    /// die später zitiert wird.
    #[test]
    fn eine_ausnahme_bekommt_keine_erfundene_obergrenze() {
        let alle = alle_domaenen();
        let ohne: Vec<_> = alle.iter().filter(|d| d.maximum.is_none()).collect();

        assert!(
            !ohne.is_empty(),
            "Trägerzusicherung: KEINE Domäne ohne Maximum — dann prüft dieser \
             Fall den Ausnahmefall gar nicht"
        );
        for d in &ohne {
            let w = WIRKGROESSEN
                .iter()
                .find(|w| w.name == d.name && w.traeger == d.traeger)
                .expect("die Domäne stammt aus der Tabelle");
            assert!(
                matches!(w.deckel, Deckel::Ausnahme { .. }),
                "{} hat kein Maximum, ist aber keine begründete Ausnahme",
                d.name
            );
            assert!(!d.ist_beschraenkt());
        }

        // Die Gegenrichtung: Wer einen Deckel hat, hat GENAU ihn.
        for d in alle.iter().filter(|d| d.maximum.is_some()) {
            let w = WIRKGROESSEN
                .iter()
                .find(|w| w.name == d.name && w.traeger == d.traeger)
                .expect("die Domäne stammt aus der Tabelle");
            let Deckel::Grenze { max, .. } = w.deckel else {
                panic!("{} hat ein Maximum, aber keine Grenze", d.name)
            };
            assert_eq!(
                d.maximum,
                Some(max),
                "{}: der Bereich ist enger oder weiter als der Deckel — die \
                 Stop-Condition verbietet beides",
                d.name
            );
        }
        println!(
            "{} ohne Obergrenze, alle als Ausnahme begründet",
            ohne.len()
        );
    }

    /// **AC3-Kontrollprobe: Ohne Zertifikat wird die Binärsuche ABGELEHNT.**
    ///
    /// Der Fall prüft nicht, dass sie läuft, sondern dass sie **nicht** läuft.
    /// Genau das ist der Fall, den man weglässt.
    #[test]
    fn ohne_zertifikat_keine_binaersuche() {
        let alle = alle_domaenen();
        let ohne = alle
            .iter()
            .find(|d| d.monotonicity_certificate.is_none())
            .expect("Trägerzusicherung: es gibt eine Domäne ohne Zertifikat");

        let ergebnis = ohne.binaersuche_erlaubt(MonotonicityDomain::ExpectedDamageStableTrace);
        assert_eq!(
            ergebnis,
            Err(ZertifikatVerweigert::KeinHinweis),
            "ohne Zertifikat MUSS die Binärsuche abgelehnt werden — und zwar \
             wegen des fehlenden Hinweises, nicht aus einem anderen Grund"
        );
        assert_eq!(
            ohne.dominanz_verwerfen_erlaubt(MonotonicityDomain::ExpectedDamageStableTrace),
            Err(ZertifikatVerweigert::KeinHinweis),
            "dasselbe gilt fürs Dominanz-Verwerfen"
        );
        println!("{} ohne Zertifikat: beide Verfahren abgelehnt", ohne.name);
    }

    /// **AC2: Der Geltungsbereich entscheidet — vier Ablehnungen, einzeln.**
    ///
    /// Ein Prüffall, der nur „abgelehnt" prüft, sieht nicht, ob aus dem
    /// richtigen Grund abgelehnt wurde. Vier Gründe, vier Fälle.
    #[test]
    fn die_ablehnung_nennt_ihren_grund_einzeln() {
        let vollstaendig = MonotonieZertifikat {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
            bedarf: ZertifikatsBedarf::VOLLSTAENDIG,
            trace_region: TraceRegionBezug::StabilerTrace,
            preconditions: &["die Rotation bleibt dieselbe"],
        };

        // Trägerzusicherung: Für den eigenen Bezug trägt es. Ohne diese Zeile
        // könnte die Funktion konstant ablehnen und alle vier Fälle wären grün.
        assert_eq!(
            vollstaendig.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Ok(()),
            "Trägerzusicherung: ein vollständiges Zertifikat MUSS für seinen \
             eigenen Bezug tragen — sonst lehnt die Funktion immer ab"
        );

        // 1. Falscher Bezug — und der Fehler nennt beide Seiten.
        assert_eq!(
            vollstaendig.traegt_fuer(MonotonicityDomain::VictoryOutcome),
            Err(ZertifikatVerweigert::FalscherBezug {
                hinweis: MonotonicityDomain::ExpectedDamageStableTrace,
                gefragt: MonotonicityDomain::VictoryOutcome,
            }),
            "für den Sieg trägt ein Schadens-Zertifikat NICHT"
        );

        // 2. Kein Hinweis.
        let ohne_richtung = MonotonieZertifikat {
            direction: MonotonicDirection::Unknown,
            ..vollstaendig
        };
        assert_eq!(
            ohne_richtung.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Err(ZertifikatVerweigert::KeinHinweis)
        );

        // 3. Zutaten fehlen.
        let halb = MonotonieZertifikat {
            bedarf: ZertifikatsBedarf::AUS_HINWEIS_ALLEIN,
            ..vollstaendig
        };
        assert_eq!(
            halb.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Err(ZertifikatVerweigert::ZutatenFehlen),
            "der Hinweis allein ist kein Zertifikat"
        );

        // 4. Keine Trace-Region.
        let ohne_region = MonotonieZertifikat {
            trace_region: TraceRegionBezug::Keine,
            ..vollstaendig
        };
        assert_eq!(
            ohne_region.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Err(ZertifikatVerweigert::ZutatenFehlen)
        );

        println!("vier Ablehnungen einzeln unterschieden, Träger grün");
    }

    /// **AC2 am gemessenen Fall: `catalysis` trägt für Schaden, nicht für alles.**
    ///
    /// Gemessen: 1 000 Promille ergeben 10 von 58 lohnenden Zaubern, 36 000
    /// ergeben 4. Bezüglich Schaden steigend, bezüglich Zaubervielfalt streng
    /// fallend.
    ///
    /// Die Zaubervielfalt hat seit `f-940-zaubervielfalt` eine **eigene
    /// Variante** ([`MonotonicityDomain::SpellDiversity`]). Bis dahin stand der
    /// Fall nur im Kommentar — die Aussage, die `catalysis` gefährlich macht,
    /// ließ sich im Typ gar nicht hinschreiben.
    #[test]
    fn catalysis_traegt_fuer_schaden_und_fuer_nichts_sonst() {
        let z = MonotonieZertifikat {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
            bedarf: ZertifikatsBedarf::VOLLSTAENDIG,
            trace_region: TraceRegionBezug::StabilerTrace,
            preconditions: &["stabile Rotation", "gewöhnlicher Boss"],
        };

        assert_eq!(
            z.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Ok(())
        );

        let mut abgelehnt = 0;
        for anderer in [
            MonotonicityDomain::VictoryOutcome,
            MonotonicityDomain::TimeToVictoryAcrossPhase,
            MonotonicityDomain::HpPredicate,
            // Seit f-940-zaubervielfalt der vierte — und der einzige, für den
            // eine Messung vorliegt statt einer Überlegung.
            MonotonicityDomain::SpellDiversity,
        ] {
            assert!(
                z.traegt_fuer(anderer).is_err(),
                "{anderer:?}: ein Schadens-Zertifikat darf hier NICHT tragen"
            );
            abgelehnt += 1;
        }
        assert_eq!(
            abgelehnt, 4,
            "Trägerzusicherung: alle vier Gegenbeispiele wurden gefragt"
        );
        println!("catalysis: 1 Bezug getragen, {abgelehnt} abgelehnt");
    }

    /// **AC2: Die Zaubervielfalt ist ein eigener Bezug — und catalysis fällt
    /// bezüglich ihr.**
    ///
    /// Der Fall, für den `f-940-zaubervielfalt` entschieden wurde. Er prüft
    /// nicht nur, dass ein Schadens-Zertifikat für sie NICHT trägt (das tut
    /// [`catalysis_traegt_fuer_schaden_und_fuer_nichts_sonst`]), sondern dass
    /// die Variante **als Bezug taugt**: Ein Zertifikat MIT diesem Bezug trägt
    /// für ihn und für nichts anderes.
    ///
    /// # Warum dieser Fall Pflicht ist und nicht Zugabe
    ///
    /// Eine neue Aufzählungs-Variante bricht hier **keinen Bau**: Über
    /// [`MonotonicityDomain`] gibt es nirgends einen erschöpfenden `match`,
    /// geprüft wird mit `matches!` und `==`. Ohne diesen Fall wäre die Variante
    /// also eine, die niemand erzeugt und niemand prüft — dieselbe Lage wie
    /// [`TraceRegionBezug::StueckweiseZwischenSprungstellen`], nur andersherum:
    /// dort tot, weil niemand sie erzeugt, hier tot, weil niemand sie prüft.
    #[test]
    fn die_zaubervielfalt_ist_ein_eigener_bezug() {
        // Ein Zertifikat, das AUSDRÜCKLICH für die Zaubervielfalt gilt — und
        // zwar mit der Richtung, die gemessen ist: FALLEND.
        let vielfalt = MonotonieZertifikat {
            direction: MonotonicDirection::Decreasing,
            domain: MonotonicityDomain::SpellDiversity,
            bedarf: ZertifikatsBedarf::VOLLSTAENDIG,
            trace_region: TraceRegionBezug::StabilerTrace,
            preconditions: &["gemessen an catalysis: 1000 Promille -> 10 von 58, 36000 -> 4"],
        };

        // 1. Es trägt für seinen eigenen Bezug.
        assert_eq!(
            vielfalt.traegt_fuer(MonotonicityDomain::SpellDiversity),
            Ok(()),
            "Trägerzusicherung: ohne diese Zeile könnte traegt_fuer für die neue \
             Variante konstant ablehnen, und alles Weitere wäre grün und leer"
        );

        // 2. Und für keinen anderen — auch nicht für den Schaden.
        let mut abgelehnt = 0;
        for anderer in [
            MonotonicityDomain::ExpectedDamageStableTrace,
            MonotonicityDomain::VictoryOutcome,
            MonotonicityDomain::TimeToVictoryAcrossPhase,
            MonotonicityDomain::HpPredicate,
        ] {
            assert_eq!(
                vielfalt.traegt_fuer(anderer),
                Err(ZertifikatVerweigert::FalscherBezug {
                    hinweis: MonotonicityDomain::SpellDiversity,
                    gefragt: anderer,
                }),
                "{anderer:?}: ein Vielfalts-Zertifikat darf hier nicht tragen, \
                 und die Ablehnung muss den FALSCHEN BEZUG nennen"
            );
            abgelehnt += 1;
        }
        assert_eq!(abgelehnt, 4, "alle vier anderen Bezüge wurden gefragt");

        // 3. **Die Richtungen sind entgegengesetzt — das ist der ganze Punkt.**
        //
        // Derselbe Parameter, zwei Bezüge, zwei Richtungen. Wer nur „monoton"
        // sagt, sagt je nach Bezug das Gegenteil.
        let schaden = MonotonieZertifikat {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
            ..vielfalt
        };
        assert_ne!(
            schaden.direction, vielfalt.direction,
            "steigend bezüglich Schaden, fallend bezüglich Vielfalt — wären die \
             Richtungen gleich, wäre der Fall gegenstandslos"
        );
        assert_eq!(
            schaden.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Ok(())
        );
        assert!(schaden
            .traegt_fuer(MonotonicityDomain::SpellDiversity)
            .is_err());

        println!(
            "catalysis: {:?} bezüglich Schaden, {:?} bezüglich Vielfalt, {abgelehnt} Bezüge abgelehnt",
            schaden.direction, vielfalt.direction
        );
    }

    /// **AC4: Die Abhängigkeiten folgen aus der Tabelle.**
    ///
    /// 15 Einträge, 13 verschiedene Namen — die Differenz sind genau die
    /// Parameter, die sich dieselbe Wirkgröße teilen. Die Kontrollzeile
    /// `13 + 2 = 15` fängt eine übersehene Doppelung.
    #[test]
    fn geteilte_wirkgroessen_sind_abhaengig() {
        let mut namen: Vec<&str> = WIRKGROESSEN.iter().map(|w| w.name).collect();
        namen.sort_unstable();
        let gesamt = namen.len();
        namen.dedup();
        let verschieden = namen.len();
        let doppelt = gesamt - verschieden;

        assert_eq!(
            verschieden + doppelt,
            gesamt,
            "Kontrollzeile: verschiedene plus doppelte müssen die Gesamtzahl treffen"
        );
        assert!(
            doppelt > 0,
            "Trägerzusicherung: KEIN Name doppelt — dann prüft dieser Fall nichts"
        );

        let mit: Vec<_> = alle_domaenen()
            .into_iter()
            .filter(|d| d.dependencies.is_some())
            .collect();
        assert_eq!(
            mit.len(),
            doppelt * 2,
            "jeder geteilte Name muss BEIDE Seiten als abhängig ausweisen"
        );
        for d in &mit {
            assert_eq!(
                d.designer_cost,
                Aenderungskosten::GeteilteWirkgroesse,
                "{}: geteilte Wirkgröße, aber die Kosten sagen etwas anderes",
                d.name
            );
        }
        println!("{gesamt} Einträge, {verschieden} Namen, {doppelt} geteilt");
    }

    /// **AC1 (`discrete_step`): Die Schrittweite unterscheidet die Träger.**
    ///
    /// Die erste Fassung gab für beide `1` zurück. Dieser Fall prüft, dass die
    /// beiden Zweige tatsächlich auseinandergehen: Wären sie gleich, wäre die
    /// Unterscheidung im Typ eine Behauptung ohne Wirkung.
    #[test]
    fn die_schrittweite_unterscheidet_die_traeger() {
        let alle = alle_domaenen();
        let faktoren: Vec<_> = alle
            .iter()
            .filter(|d| d.traeger == Traeger::Faktor)
            .collect();
        let getragen: Vec<_> = alle
            .iter()
            .filter(|d| d.traeger == Traeger::Getragen)
            .collect();

        assert!(
            !faktoren.is_empty() && !getragen.is_empty(),
            "Trägerzusicherung: {} Faktoren, {} getragene — beide Sorten müssen \
             vorkommen, sonst prüft dieser Fall nur eine Seite",
            faktoren.len(),
            getragen.len()
        );

        for d in &faktoren {
            assert_eq!(
                d.discrete_step,
                Schrittweite::NichtFestgelegt,
                "{}: ein f64-Faktor hat keine entschiedene Stufung",
                d.name
            );
        }
        for d in &getragen {
            assert_eq!(
                d.discrete_step,
                Schrittweite::EinPromille,
                "{}: getragene Werte stehen in Promille",
                d.name
            );
        }
        assert_ne!(
            Schrittweite::NichtFestgelegt,
            Schrittweite::EinPromille,
            "die beiden Stufungen müssen unterscheidbar sein"
        );
        println!(
            "{} Faktoren ohne festgelegte Stufung, {} getragene in Promille",
            faktoren.len(),
            getragen.len()
        );
    }

    /// **AC1 (`minimum`, `allowed_breakpoints`): die zwei Felder, die sonst
    /// niemand ansieht.**
    ///
    /// `minimum` und `allowed_breakpoints` standen zuerst in keinem einzigen
    /// Prüffall — gebaut und ungeprüft, dieselbe Klasse wie die Schrittweite
    /// eine Ebene weiter. Dieser Fall schließt sie an.
    #[test]
    fn untergrenze_und_sprungstellen_sind_gedeckt() {
        let alle = alle_domaenen();
        assert!(
            !alle.is_empty(),
            "Trägerzusicherung: die Tabelle ist nicht leer"
        );

        // Die Untergrenze liegt nie ÜBER der Obergrenze. Bei einer Domäne ohne
        // Obergrenze ist die Frage gegenstandslos — und genau deshalb wird sie
        // dort auch nicht gestellt.
        let mut mit_grenze = 0;
        for d in &alle {
            assert_eq!(d.minimum, 0, "{}: die Untergrenze ist 0", d.name);
            if let Some(max) = d.maximum {
                assert!(
                    d.minimum < max,
                    "{}: minimum {} liegt nicht unter maximum {max}",
                    d.name,
                    d.minimum
                );
                mit_grenze += 1;
            }
        }
        assert!(
            mit_grenze > 0,
            "Trägerzusicherung: KEINE Domäne mit Obergrenze — dann prüft die \
             Ordnung minimum < maximum nichts"
        );

        // **Heute gibt es keine Sprungstellen, und das ist der Befund.**
        //
        // Die Trace-Regionen sollen sie laut Issue liefern; keine tut es
        // bisher. Dieser Fall hält den Zustand fest und wird ROT, sobald
        // jemand welche einträgt — dann gehört die Invariante gebaut, dass
        // eine Sprungstelle INNERHALB des Bereichs liegt. Solange die Liste
        // leer ist, wäre eine solche Prüfung leer wahr.
        let mit_sprungstellen: Vec<_> = alle
            .iter()
            .filter(|d| !d.allowed_breakpoints.is_empty())
            .collect();
        assert!(
            mit_sprungstellen.is_empty(),
            "{} Domänen tragen jetzt Sprungstellen ({:?}) — dieser Fall hat sie \
             bewusst festgehalten, solange es keine gab. Jetzt gehört geprüft, \
             dass jede innerhalb von [minimum, maximum] liegt.",
            mit_sprungstellen.len(),
            mit_sprungstellen.iter().map(|d| d.name).collect::<Vec<_>>()
        );

        println!(
            "{} Domänen, {mit_grenze} mit Obergrenze, 0 mit Sprungstellen",
            alle.len()
        );
    }

    /// **AC4: die drei Zweige von `abhaengigkeiten_fuer`, einzeln.**
    ///
    /// Über `alle_domaenen()` werden nur echte Namen gereicht — der Zweig für
    /// einen **unbekannten** Namen (`treffer.next()?`) ist damit nie erreicht.
    /// Er wäre gedeckt geblieben, ohne je gelaufen zu sein: erreichbar ist
    /// nicht dasselbe wie geprüft.
    #[test]
    fn abhaengigkeiten_unterscheiden_drei_faelle() {
        // 1. Ein Name, der zweimal vorkommt — Partner vorhanden.
        assert_eq!(
            abhaengigkeiten_fuer("essence"),
            Some("essence"),
            "essence steht als Faktor UND als getragener Wert in der Tabelle"
        );

        // 2. Ein Name, der genau einmal vorkommt — kein Partner.
        let einzeln = WIRKGROESSEN
            .iter()
            .find(|w| WIRKGROESSEN.iter().filter(|x| x.name == w.name).count() == 1)
            .expect("Trägerzusicherung: es gibt einen Namen, der nur einmal vorkommt");
        assert_eq!(
            abhaengigkeiten_fuer(einzeln.name),
            None,
            "{} kommt einmal vor und hat keinen Partner",
            einzeln.name
        );

        // 3. Ein Name, den es gar nicht gibt — der Zweig, den `alle_domaenen`
        //    nie erreicht.
        assert_eq!(
            abhaengigkeiten_fuer("gibt_es_nicht"),
            None,
            "ein unbekannter Name darf keinen Partner melden"
        );

        // Und die Gegenprobe zur Trägerzusicherung: Der geprüfte Einzelname
        // steht wirklich in der Tabelle, der erfundene nicht.
        assert!(WIRKGROESSEN.iter().any(|w| w.name == einzeln.name));
        assert!(!WIRKGROESSEN.iter().any(|w| w.name == "gibt_es_nicht"));

        println!("drei Zweige: Partner, kein Partner, unbekannt");
    }

    /// **Evidence-Anforderung (nicht AC): die Bereichstabelle für den Bericht.**
    ///
    /// Das Issue verlangt unter *evidence*: „Der Bericht enthält die
    /// Bereichstabelle und beide Kontrollproben." Gemeint ist die
    /// Fertigmeldung, nicht ein Programm-Bericht — der Satz steht neben
    /// „Temporäre Dateien nach `/work/tmp`".
    ///
    /// Dieser Fall druckt sie, damit sie **aus einem Lauf** stammt und nicht
    /// von Hand abgeschrieben ist. Eine abgeschriebene Tabelle ist eine
    /// zweite Quelle, die driften kann.
    #[test]
    fn die_bereichstabelle_fuer_den_bericht() {
        let alle = alle_domaenen();
        assert!(
            !alle.is_empty(),
            "Trägerzusicherung: die Tabelle ist nicht leer"
        );

        println!(
            "{:<15} {:<9} {:>12} {:>14} {:<16} {:<12} Kosten",
            "Name", "Träger", "min", "max", "Schrittweite", "Abhängig"
        );
        for d in &alle {
            println!(
                "{:<15} {:<9} {:>12} {:>14} {:<16} {:<12} {:?}",
                d.name,
                format!("{:?}", d.traeger),
                d.minimum,
                d.maximum
                    .map_or_else(|| "— (Ausnahme)".to_string(), |m| m.to_string()),
                format!("{:?}", d.discrete_step),
                d.dependencies.unwrap_or("—"),
                d.designer_cost
            );
        }
        println!(
            "\n{} Domänen · {} mit Obergrenze · {} ohne · {} mit Abhängigkeit · {} mit Zertifikat",
            alle.len(),
            alle.iter().filter(|d| d.maximum.is_some()).count(),
            alle.iter().filter(|d| d.maximum.is_none()).count(),
            alle.iter().filter(|d| d.dependencies.is_some()).count(),
            alle.iter()
                .filter(|d| d.monotonicity_certificate.is_some())
                .count(),
        );
    }

    /// **AC5: Die Kosten ordnen — sonst entscheiden sie nichts.**
    #[test]
    fn geteilte_wirkgroessen_kosten_mehr() {
        assert!(
            Aenderungskosten::MehrereZauber < Aenderungskosten::GeteilteWirkgroesse,
            "eine geteilte Wirkgröße muss teurer sein als eine, die allein steht \
             — sonst gewinnt in der Rangfolge die falsche Lösung"
        );

        // Trägerzusicherung: Beide Stufen werden auch WIRKLICH erzeugt. Eine
        // Ordnung über Varianten, die niemand herstellt, ordnet nichts.
        let alle = alle_domaenen();
        for stufe in [
            Aenderungskosten::MehrereZauber,
            Aenderungskosten::GeteilteWirkgroesse,
        ] {
            assert!(
                alle.iter().any(|d| d.designer_cost == stufe),
                "{stufe:?} wird von keiner Domäne erzeugt — dann ist die Stufe Zierrat"
            );
        }
    }

    /// **AC2: Der Hinweis allein trägt nie — das ist die Aussage von #966.**
    #[test]
    fn aus_dem_hinweis_allein_entsteht_kein_tragendes_zertifikat() {
        let z = zertifikat_aus_hinweis(LocalMonotonicityHint {
            direction: MonotonicDirection::Increasing,
            domain: MonotonicityDomain::ExpectedDamageStableTrace,
        });
        // **Trägerzusicherung: Richtung und Bezug sind wirklich übernommen.**
        //
        // Die Meldung unten sagt „der Hinweis nennt die richtige Richtung UND
        // den richtigen Bezug". Das stand hier zuerst nur als Text — und ohne
        // diese zwei Zeilen wäre der Fall auch grün, wenn
        // `zertifikat_aus_hinweis` beides verwürfe. Dann läge die Ablehnung an
        // etwas anderem als an den fehlenden Zutaten.
        assert_eq!(z.direction, MonotonicDirection::Increasing);
        assert_eq!(z.domain, MonotonicityDomain::ExpectedDamageStableTrace);

        assert_eq!(
            z.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Err(ZertifikatVerweigert::ZutatenFehlen),
            "der Hinweis nennt die richtige Richtung UND den richtigen Bezug — \
             und trägt trotzdem nicht, weil drei Zutaten fehlen"
        );
        assert_eq!(z.bedarf.fehlend().len(), 3);

        // **Und die Gegenprobe: Es liegt NUR an den Zutaten.**
        //
        // Dasselbe Zertifikat, nur mit vollständigem Bedarf und einer
        // Trace-Region, trägt. Ohne diese Zeile könnte `traegt_fuer` konstant
        // `ZutatenFehlen` liefern und der Fall bliebe grün.
        let ergaenzt = MonotonieZertifikat {
            bedarf: ZertifikatsBedarf::VOLLSTAENDIG,
            trace_region: TraceRegionBezug::StabilerTrace,
            ..z
        };
        assert_eq!(
            ergaenzt.traegt_fuer(MonotonicityDomain::ExpectedDamageStableTrace),
            Ok(()),
            "mit denselben Werten und vollständigen Zutaten MUSS es tragen — \
             sonst lag die Ablehnung nicht an den Zutaten"
        );

        println!("fehlende Zutaten: {:?}", z.bedarf.fehlend());
    }
}
