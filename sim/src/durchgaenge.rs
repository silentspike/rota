//! Wie viele Durchgänge genügen? (Issue #908, AC1)
//!
//! # Warum das nicht `purity == 0` heißen darf
//!
//! Heute ist der kritische Treffer die einzige Zufallsquelle im Spielerkit.
//! Deshalb *stimmt* die Abkürzung `if purity == 0 { ein Durchgang }` — und
//! genau das macht sie gefährlich: Sobald ein legendärer Effekt, eine
//! Boss-Signaturregel oder ein neuer Zauber würfelt, rechnet der Läufer **einen**
//! Durchgang, wo er hunderte bräuchte. Das Ergebnis sähe plausibel aus, und
//! niemand bemerkte es.
//!
//! Deshalb wird die Bedingung **abgeleitet**, in zwei Stufen:
//!
//! | Stufe | Quelle | Frage |
//! | --- | --- | --- |
//! | 1 | [`crate::effects::RandomSourceMask::aus_knoten`] | Welche Ströme kann die **Beschreibung** überhaupt anfassen? |
//! | 2 | [`ziehende_stroeme`] | Welche davon ziehen in **dieser** Konfiguration wirklich? |
//!
//! `purity == 0` kommt darin vor — aber als Sonderfall *innerhalb* der zweiten
//! Stufe, nicht als Kriterium daneben.
//!
//! # Die zweite Stufe ist nicht optional
//!
//! Stufe 1 allein genügt nicht, und die Beschreibung sagt das selbst: „Leere
//! Maske ist hinreichend für ein Durchgang genügt, **nicht notwendig**."
//! Bei `purity = 0` tragen die Knoten weiterhin `CritRoll`, die Maske ist also
//! nicht leer — reduziert würde nie.
//!
//! # Was dabei herauskam und vorher niemand aufgeschrieben hatte
//!
//! Deterministisch ist nicht nur „nie kritisch", sondern **auch „immer
//! kritisch"**. [`crate::rng::RandomStream::chance`] zieht an beiden Enden
//! nicht:
//!
//! ```text
//! probability <= 0.0  ->  false, ohne Ziehung
//! probability >= 1.0  ->  true,  ohne Ziehung
//! ```
//!
//! Und `crit_chance()` liefert bei `purity >= 1000` genau `1.0`. Ein Lauf mit
//! vollem PURITY ist also **ebenso** auf einen Durchgang reduzierbar wie einer
//! mit gar keinem. Eine Bedingung `purity == 0` verschenkt diesen Fall
//! stillschweigend — die Ableitung nimmt ihn mit, ohne dass jemand daran denken
//! muss.
//!
//! # Warum die Ableitung trotzdem nachgeprüft wird
//!
//! Eine Voraussetzung ist Code, und Code hat Fehler. Beschreibt ein Effekt
//! seine Quelle falsch, reduziert die Ableitung einen Lauf, der streut. Dagegen
//! hilft kein zweiter Blick auf dieselbe Ableitung, sondern nur eine Messung am
//! Ergebnis: [`crate::fight::RunResult::random_draws`] muss nach einem
//! reduzierten Lauf **null** sein. Das prüft nicht die Mathematik, sondern ob
//! die Voraussetzung wirklich galt.

use crate::effects::{EffectNode, RandomSourceMask};
use crate::gear::WornValues;
use crate::rng::Stream;

/// Zieht ein Strom, dessen Ereignis mit dieser Wahrscheinlichkeit eintritt?
///
/// Nur ein Wert **echt zwischen** null und eins kostet eine Ziehung; an beiden
/// Enden antwortet [`crate::rng::RandomStream::chance`] ohne zu würfeln.
#[must_use]
pub fn zieht_bei(wahrscheinlichkeit: f64) -> bool {
    wahrscheinlichkeit > 0.0 && wahrscheinlichkeit < 1.0
}

/// Zieht dieser Strom in **dieser** Konfiguration?
///
/// # Der Auffangzweig fehlt mit Absicht (AC2)
///
/// Ein neuer Strom in [`Stream`] bricht hier die Übersetzung, bis jemand
/// entschieden hat, woran er hängt. Ein `_ => false` wäre bequem und würde den
/// neuen Strom stillschweigend für deterministisch erklären — also genau den
/// Fehler bauen, den dieses Modul verhindern soll.
#[must_use]
pub fn zieht(strom: Stream, worn: &WornValues) -> bool {
    match strom {
        // Beide Krit-Würfe hängen an derselben Chance: `roll_crit` fragt für
        // Treffer wie für DoT-Ticks `worn.crit_chance()`.
        Stream::Crit | Stream::DotTick => zieht_bei(worn.crit_chance()),
        // Reserviert und heute tot: Alle drei Koagulations-Wege sind
        // deterministisch (Design 21.1). Zieht hier je etwas, fängt es die
        // Nachprüfung über `random_draws` — nicht dieses `match`.
        Stream::Heal => false,
    }
}

/// Welche Ströme in dieser Konfiguration tatsächlich ziehen — Stufe 2.
#[must_use]
pub fn ziehende_stroeme(worn: &WornValues) -> RandomSourceMask {
    Stream::ALL
        .into_iter()
        .filter(|&strom| zieht(strom, worn))
        .fold(RandomSourceMask::KEINE, |acc, strom| {
            acc.union(RandomSourceMask::einer(strom))
        })
}

/// Die im Lauf wirklich aktiven Quellen — beide Stufen zusammen.
///
/// Der **Schnitt**, nicht die Vereinigung: Eine Quelle ist nur aktiv, wenn die
/// Beschreibung sie nennt *und* die Konfiguration sie ziehen lässt. Die
/// Vereinigung würde nie reduzieren, und die Konfiguration allein wüsste nicht,
/// ob überhaupt ein Effekt würfelt.
#[must_use]
pub fn aktive_quellen(knoten: &[EffectNode], worn: &WornValues) -> RandomSourceMask {
    RandomSourceMask::aus_knoten(knoten).schnitt(ziehende_stroeme(worn))
}

/// Genügt **ein** Durchgang für diesen Lauf?
///
/// Wahr heißt: Zwei Durchgänge mit verschiedenen Startwerten liefern denselben
/// Kampf, weil nirgends gezogen wird.
#[must_use]
pub fn ein_durchgang_genuegt(knoten: &[EffectNode], worn: &WornValues) -> bool {
    aktive_quellen(knoten, worn).ist_leer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EFFECT_NODES;

    fn mit_purity(purity: u32) -> WornValues {
        let mut worn = WornValues::none();
        worn.purity = purity;
        worn
    }

    #[test]
    fn zieht_nur_echt_zwischen_null_und_eins() {
        assert!(!zieht_bei(0.0), "null ist deterministisch");
        assert!(!zieht_bei(1.0), "eins ist ebenso deterministisch");
        assert!(!zieht_bei(-0.5), "unter null erst recht");
        assert!(!zieht_bei(1.5), "über eins erst recht");
        assert!(zieht_bei(0.5), "dazwischen wird gezogen");
        assert!(zieht_bei(f64::MIN_POSITIVE), "auch knapp über null");
    }

    #[test]
    fn ohne_purity_zieht_kein_strom() {
        let worn = mit_purity(0);
        assert_eq!(
            worn.crit_chance(),
            0.0,
            "Kontrollzeile: die Chance ist null"
        );
        assert!(ziehende_stroeme(&worn).ist_leer());
    }

    #[test]
    fn volles_purity_zieht_ebenfalls_nicht() {
        let worn = mit_purity(1000);
        assert_eq!(
            worn.crit_chance(),
            1.0,
            "Kontrollzeile: die Chance ist genau eins"
        );
        assert!(
            ziehende_stroeme(&worn).ist_leer(),
            "immer-kritisch ist so deterministisch wie nie-kritisch — genau der \
             Fall, den `purity == 0` verschenkt hätte"
        );
    }

    #[test]
    fn dazwischen_ziehen_beide_krit_stroeme() {
        let maske = ziehende_stroeme(&mit_purity(500));
        assert!(maske.enthaelt(Stream::Crit));
        assert!(maske.enthaelt(Stream::DotTick));
        assert!(!maske.enthaelt(Stream::Heal), "aus Heal zieht heute nichts");
        assert_eq!(maske.anzahl(), 2, "genau zwei Ströme, nicht alle drei");
    }

    #[test]
    fn der_schnitt_reduziert_und_die_vereinigung_taete_es_nie() {
        let worn = mit_purity(0);
        let beschreibung = RandomSourceMask::aus_knoten(EFFECT_NODES);

        assert!(
            !beschreibung.ist_leer(),
            "Trägerzusicherung: die Beschreibung NENNT Quellen — sonst prüfte \
             dieser Fall den Schnitt gar nicht"
        );
        assert!(
            aktive_quellen(EFFECT_NODES, &worn).ist_leer(),
            "der Schnitt mit einer leeren Konfigurationsmaske ist leer"
        );
        assert!(
            !beschreibung.union(ziehende_stroeme(&worn)).ist_leer(),
            "die Vereinigung wäre nicht leer — sie hätte nie reduziert"
        );
    }

    #[test]
    fn ein_durchgang_genuegt_bei_purity_null_und_voll() {
        assert!(ein_durchgang_genuegt(EFFECT_NODES, &mit_purity(0)));
        assert!(ein_durchgang_genuegt(EFFECT_NODES, &mit_purity(1000)));
        assert!(
            !ein_durchgang_genuegt(EFFECT_NODES, &mit_purity(500)),
            "bei halber Chance wird gewürfelt — hier darf nicht reduziert werden"
        );
    }
}
