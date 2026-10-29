//! Such- und Prüfkampf (Issue #917, AC5).
//!
//! ```text
//! SearchFight   nur was das Ziel bestimmt          -> Millionen Kandidaten
//! AuditFight    zusätzlich das Zählwerk der Prüfung -> Elites, Fehlerfälle, Stichproben
//! ```
//!
//! **Beide Pfade müssen für dieselbe Eingabe dasselbe Ziel liefern, geprüft über
//! die Zustandsspur:** Vor jeder Entscheidung wird der Suchzustand beider Kämpfe
//! gehasht ([`CompactFightState::spur_hash`]), und die beiden Folgen müssen
//! Eintrag für Eintrag gleich sein. Gleiche Ergebnisse allein reichten nicht —
//! zwei Kämpfe können am Ende dieselbe Summe haben und unterwegs verschieden
//! gerechnet haben.

#[allow(dead_code)]
mod inkrementell_hilfe;

use std::cell::RefCell;

use inkrementell_hilfe::{kampf, wuerfelnder_kampf};
use rota_sim::apl::{parse, Apl, AplName};
use rota_sim::fight::{AuditFight, CompactFightState, Decide, Fight, FightConfig, SearchFight};
use rota_sim::{Millis, SPELL_COUNT};

/// Hasht vor jeder Entscheidung den Suchzustand und entscheidet wie die Liste.
struct Spur<'a> {
    apl: &'a Apl,
    hashes: RefCell<Vec<u64>>,
}

impl<'a> Spur<'a> {
    fn new(apl: &'a Apl) -> Self {
        Self {
            apl,
            hashes: RefCell::new(Vec::new()),
        }
    }
}

impl Decide for Spur<'_> {
    fn decide(&self, fight: &Fight) -> Option<usize> {
        self.hashes
            .borrow_mut()
            .push(fight.such_zustand().spur_hash());
        self.apl.decide(fight)
    }

    fn next_wakeup(&self, fight: &Fight) -> Option<Millis> {
        self.apl.next_wakeup(fight)
    }

    fn next_wakeup_ohne_gcd(&self, fight: &Fight) -> Option<Millis> {
        self.apl.next_wakeup_ohne_gcd(fight)
    }
}

/// Listen, die je einen Teil des Suchzustands lesen: Brand-Ticks (das
/// einzige Feld je Zauber, das eine Bedingung zählt), Verstärkungen,
/// Boss-Zauber, Ressource.
fn faelle() -> Vec<(&'static str, Apl, FightConfig)> {
    let liste = |name: &'static str, text: &str| {
        parse(AplName(name), text).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    vec![
        (
            "step3-good",
            liste("step3-good", include_str!("../apl/step3-good.apl")),
            kampf("boss-1-1", 5),
        ),
        (
            "brand-ticks",
            liste(
                "brand-ticks",
                "smoulder if dot.smoulder.ticks < 3\npyre if heat >= 15\nkindle\n",
            ),
            kampf("boss-1-1", 5),
        ),
        (
            "step553-alle-bedingt",
            liste(
                "step553-alle-bedingt",
                include_str!("../apl/step553-alle-bedingt.apl"),
            ),
            kampf("boss-2-2", 5),
        ),
        (
            "e797-mit-quench",
            liste(
                "e797-mit-quench",
                include_str!("../apl/e797-mit-quench.apl"),
            ),
            kampf("test-lethal-strike", 5),
        ),
        (
            "step3-good würfelnd",
            liste("step3-good", include_str!("../apl/step3-good.apl")),
            wuerfelnder_kampf("boss-1-1", 7),
        ),
    ]
}

#[test]
fn such_und_pruefkampf_liefern_dasselbe_ziel_ueber_die_zustandsspur() {
    for (name, apl, config) in faelle() {
        let spur_pruef = Spur::new(&apl);
        let mut pruef = AuditFight::new(config);
        pruef.run_mit(&spur_pruef);

        let spur_such = Spur::new(&apl);
        let mut such = SearchFight::new(config);
        let ziel = such.run_mit(&spur_such);

        assert_eq!(ziel, pruef.zielwerte(), "{name}: verschiedenes Ziel");
        let pruef_spur = spur_pruef.hashes.into_inner();
        let such_spur = spur_such.hashes.into_inner();
        assert!(
            pruef_spur.len() > 10,
            "{name}: nur {} Entscheidungen",
            pruef_spur.len()
        );
        let erste = pruef_spur.iter().zip(&such_spur).position(|(p, s)| p != s);
        assert_eq!(
            (erste, such_spur.len()),
            (None, pruef_spur.len()),
            "{name}: die Zustandsspuren weichen ab (erste Abweichung, Länge der Suchspur)"
        );
    }
}

#[test]
fn der_suchkampf_fuehrt_das_zaehlwerk_der_pruefung_nicht() {
    let (name, apl, config) = faelle().swap_remove(0);
    let mut pruef = AuditFight::new(FightConfig {
        log_capacity: 4096,
        ..config
    });
    let ergebnis = pruef.run_apl(&apl);
    // Der Träger: Der Prüfkampf führt das Zählwerk tatsächlich.
    assert!(
        ergebnis.casts.iter().sum::<u32>() > 0,
        "{name}: keine Würfe gezählt"
    );
    assert!(ergebnis.damage_by_spell.iter().sum::<u32>() > 0);
    assert!(!pruef.log().is_empty());

    let mut such = SearchFight::new(FightConfig {
        log_capacity: 4096,
        ..config
    });
    let ziel = such.run_apl(&apl);
    assert_eq!(ziel, pruef.zielwerte());
    let kampf = such.kampf();
    for zauber in 0..SPELL_COUNT {
        assert_eq!(kampf.cast_count(zauber), 0, "{name}: Suchkampf zählt Würfe");
        assert_eq!(kampf.lost_dot_ticks(zauber), 0);
    }
    assert!(
        kampf.log().is_empty(),
        "{name}: der Suchkampf schreibt ein Protokoll"
    );
}

#[test]
fn der_verdichtete_zustand_traegt_nur_die_aktiven_zauber() {
    let (name, apl, config) = faelle().swap_remove(0);
    let spur = Spur::new(&apl);
    let mut such = SearchFight::new(config);
    such.run_mit(&spur);
    let zustand: CompactFightState = such.kampf().such_zustand();
    let aktiv = zustand.aktive_zauber().len();
    // Jeder Zauber, den der Prüfkampf gewirkt hat, trägt Zustand — mindestens
    // seine Abklingzeit. Fehlte einer, verlöre ein Suffix-Lauf ihn.
    let mut pruef = AuditFight::new(config);
    let gewirkt: Vec<usize> = pruef
        .run_apl(&apl)
        .casts
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .map(|(zauber, _)| zauber)
        .collect();
    assert!(!gewirkt.is_empty(), "{name}: nichts gewirkt");
    for zauber in &gewirkt {
        assert!(
            zustand.aktive_zauber().iter().any(|a| a == *zauber),
            "{name}: Zauber {zauber} wurde gewirkt und fehlt im verdichteten Zustand"
        );
    }
    assert!(
        aktiv < SPELL_COUNT,
        "{name}: alle {SPELL_COUNT} Zauber aktiv"
    );
    let kampf_bytes = std::mem::size_of::<Fight>();
    let zustand_bytes = std::mem::size_of::<CompactFightState>() + zustand.offene_ereignisse() * 16;
    println!(
        "{name}: Kampf {kampf_bytes} Byte, verdichteter Zustand {zustand_bytes} Byte \
         ({aktiv} aktive Zauber von {SPELL_COUNT}, {} offene Ereignisse)",
        zustand.offene_ereignisse()
    );
    assert!(zustand_bytes < kampf_bytes);
}
