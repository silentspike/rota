//! SUBLIMATIONs Schluesseltalent rechnet auf beiden Seiten gleich (#419).
//!
//! # Was geprueft wird
//!
//! Ohne das Talent stellt ein Ruecksetzer einen AUSWAHLMODUS scharf — die
//! Abklingzeit bleibt, bis ein weiterer Druck sie waehlt. Mit dem Talent
//! entfaellt die Wahl, und die LAENGSTE Restzeit ist sofort weg.
//!
//! # Warum hier keine Aufzeichnung steht
//!
//! Dieselbe Begruendung wie bei `keystone_split_spender.rs`: Was beide Seiten
//! teilen muessen, ist die REGEL — welcher Zauber getroffen wird und dass die
//! Auswahl entfaellt. Die steht hier als Messung, und dieselbe steht auf der
//! Spiel-Seite in `tests/unit/keystone-auto-reset.test.ts`.
//!
//! # Die Regel bei Gleichstand ist Teil der Paritaet
//!
//! Bei zwei gleich langen Restzeiten nimmt BEIDE Seiten den kleineren Platz
//! (`rest > laengste`, nicht `>=`). Ohne diese Festlegung waehlten sie
//! irgendwann verschieden — und die Bitgleichheit risse an einer Stelle, die
//! niemand mit dem Talent in Verbindung braechte.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Laesst PYRE und QUENCH auf Abklingzeit laufen und wirkt dann den
/// Ruecksetzer. Gibt die Restzeiten von PYRE und QUENCH danach.
fn nach_dem_ruecksetzer(auto_reset: bool) -> (u32, u32) {
    let boss = boss_by_id("first-crucible").expect("Boss fehlt");
    let pyre = index_of("pyre").expect("pyre fehlt");
    let quench = index_of("quench").expect("quench fehlt");
    // Der Ruecksetzer der Schule — ueber seine Ladungen gefunden, nicht ueber
    // den Namen: Wer ihn hier festschreibt, misst bei einer Umbenennung nichts
    // mehr und merkt es nicht.
    let reset = (0..rota_sim::data::SPELL_COUNT)
        .find(|i| rota_sim::data::SPELLS[*i].cooldown_reset_charges > 0)
        .expect("kein Zauber traegt cooldown_reset_charges");

    let mut fight = Fight::new(FightConfig {
        boss,
        auto_reset,
        ..FightConfig::default()
    });

    let bindings = vec![
        Binding {
            slot: 0,
            spell: pyre,
        },
        Binding {
            slot: 1,
            spell: quench,
        },
        Binding {
            slot: 2,
            spell: reset,
        },
    ];
    // PYRE bei Takt 0, QUENCH einen GCD spaeter, der Ruecksetzer danach. Damit
    // tragen die beiden VERSCHIEDENE Restzeiten, und "die laengste" ist eine
    // echte Auswahl statt eines Gleichstands.
    //
    // **150 Takte je Wartezeit, nicht 15.** TICK_MS ist 10, GCD_MS 1500 —
    // mein erster Entwurf setzte die Druecke auf 0/15/30 und damit alle drei
    // in dieselbe Wartezeit. Zwei davon wurden abgewiesen, der Ruecksetzer
    // wirkte nie, und der Fall sah aus wie ein kaputtes Talent.
    let presses = vec![
        Press { tick: 0, slot: 0 },
        Press { tick: 150, slot: 1 },
        Press { tick: 300, slot: 2 },
    ];

    let recording = Recording {
        name: "auto-reset".to_owned(),
        seed: 0,
        ticks: 400,
        bindings,
        presses,
    };
    fight.replay(&recording, false);

    (
        fight.cooldown_remaining_ms(pyre),
        fight.cooldown_remaining_ms(quench),
    )
}

/// **Die Zusicherung vor der Messung.** Ohne sie belegte der Fall unten
/// nichts: Stuenden beide Zauber ohnehin bereit, waere "Restzeit 0" kein
/// Ergebnis des Talents.
#[test]
fn ohne_talent_bleiben_beide_auf_abklingzeit() {
    let (pyre, quench) = nach_dem_ruecksetzer(false);
    assert!(
        pyre > 0,
        "PYRE steht nicht auf Abklingzeit — der Vergleich unten misst nichts"
    );
    assert!(
        quench > 0,
        "QUENCH steht nicht auf Abklingzeit — der Vergleich unten misst nichts"
    );
    assert_ne!(
        pyre, quench,
        "beide Restzeiten sind gleich — dann ist 'die laengste' keine Auswahl"
    );
}

#[test]
fn mit_talent_faellt_genau_die_laengste() {
    let (ohne_pyre, ohne_quench) = nach_dem_ruecksetzer(false);
    let (mit_pyre, mit_quench) = nach_dem_ruecksetzer(true);

    // PYRE lief zuerst und hat deshalb die KUERZERE Restzeit; QUENCH kam
    // spaeter und traegt die laengere. Getroffen werden muss QUENCH.
    assert!(
        ohne_quench > ohne_pyre,
        "der Aufbau stimmt nicht mehr: QUENCH muesste die laengere Restzeit tragen \
         (PYRE {ohne_pyre}, QUENCH {ohne_quench})"
    );
    assert_eq!(
        mit_quench, 0,
        "die laengste Restzeit wurde nicht zurueckgesetzt"
    );
    assert_eq!(
        mit_pyre, ohne_pyre,
        "die kuerzere Restzeit wurde mitgenommen — es trifft genau EINEN"
    );
}
