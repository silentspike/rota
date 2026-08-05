//! Die Schadensformel im Kampf (GAME-DESIGN 6.3).
//!
//! **Der wichtigste Test hier ist `jeder_schadensweg_geht_durch_die_formel`.**
//! Er ist aus einem echten Fehler entstanden: Beim Einbau der Faktoren blieb der
//! Entlade-Zauber PYRE angeschlossen an den alten Weg — sein Schaden lief an der
//! Formel vorbei. Aufgefallen ist das nicht am Test, sondern beim Nachrechnen
//! einer Skalierungszahl, die zu klein war.
//!
//! Der Fehler war deshalb so unauffällig, weil das Ergebnis **plausibel** blieb:
//! Alle sechs Faktoren zeigten denselben Zuwachs, nur eben einen zu kleinen. Wer
//! die Zahl nicht gegen eine Handrechnung hält, sieht nichts.

use rota_sim::apl::parse;
use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{rotation, DamageFactors, Fight, FightConfig, RunResult, NEUTRAL};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");

fn lauf(factors: DamageFactors) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        factors,
        ..FightConfig::default()
    })
    .run_apl(&parse("gut", GOOD).expect("gueltig"))
}

#[test]
fn jeder_schadensweg_geht_durch_die_formel() {
    // Drei Wege erzeugen Schaden, und jeder einzelne muss die Faktoren sehen:
    //   KINDLE    Sofortschaden beim Wirken
    //   SMOULDER  Schaden je Tick
    //   PYRE      Schaden aus der verbrauchten Ressource
    // Bleibt einer unveraendert, ist er nicht angeschlossen.
    let ohne = lauf(NEUTRAL);
    let mit = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });

    for (name, index) in [("KINDLE", KINDLE), ("SMOULDER", SMOULDER), ("PYRE", PYRE)] {
        assert!(
            ohne.damage_by_spell[index] > 0,
            "{name} macht ohne Faktoren keinen Schaden — der Test prueft nichts"
        );
        assert!(
            mit.damage_by_spell[index] > ohne.damage_by_spell[index],
            "{name} ist nicht an die Schadensformel angeschlossen: {} bleibt {} auch mit +10 %",
            ohne.damage_by_spell[index],
            mit.damage_by_spell[index]
        );
    }
}

#[test]
fn die_einzelwerte_stimmen_mit_der_handrechnung() {
    // Gegen von Hand gerechnete Zahlen, nicht gegen den Prüfling selbst.
    // Gute Rotation über die vollen 60 s: 30x KINDLE, 29 SMOULDER-Ticks, 5x PYRE.
    let mit = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });
    assert_eq!(mit.damage_by_spell[KINDLE], 30 * 110, "100 -> 110");
    assert_eq!(mit.damage_by_spell[SMOULDER], 29 * 44, "40 -> 44");
    assert_eq!(mit.damage_by_spell[PYRE], 5 * 396, "360 -> 396");
    assert_eq!(mit.damage, 30 * 110 + 29 * 44 + 5 * 396);
}

#[test]
fn die_rundung_frisst_kleine_zuwaechse() {
    // Ein Befund, kein Mangel: Bei +1 % steigt KINDLE von 100 auf 101, ein
    // SMOULDER-Tick aber von 40 auf 40,4 — und damit gerundet auf 40. Der
    // Zuwachs verschwindet.
    //
    // Fuer den Balance-Laeufer heisst das: Ein zu kleines Delta misst nicht die
    // Skalierung, sondern die Rundung. Deshalb steht die Zahl je Prozentpunkt
    // immer neben dem verwendeten Delta.
    let ohne = lauf(NEUTRAL);
    let ein_prozent = lauf(DamageFactors {
        essence: 1.01,
        ..NEUTRAL
    });
    assert_eq!(
        ein_prozent.damage_by_spell[SMOULDER], ohne.damage_by_spell[SMOULDER],
        "40 x 1,01 = 40,4 -> 40: der Zuwachs faellt der Rundung zum Opfer"
    );
    assert!(
        ein_prozent.damage_by_spell[KINDLE] > ohne.damage_by_spell[KINDLE],
        "100 x 1,01 = 101: hier bleibt der Zuwachs erhalten"
    );

    // Bei zehn Prozent ueberlebt auch der Tick.
    let zehn_prozent = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });
    assert!(zehn_prozent.damage_by_spell[SMOULDER] > ohne.damage_by_spell[SMOULDER]);
}

#[test]
fn neutrale_faktoren_aendern_am_gleichlauf_nichts() {
    // **Die Bedingung dafuer, dass #54 gueltig bleibt.** Mit neutralen Faktoren
    // muss der Simulator auf dieselben Zahlen kommen wie vorher — sonst waere
    // der Abgleich mit dem Spiel durch den Einbau der Formel gebrochen.
    let mit_apl = Fight::new(FightConfig::default()).run_apl(&parse("gut", GOOD).expect("gueltig"));
    let mit_funktion = Fight::new(FightConfig::default()).run(rotation::good);

    assert_eq!(mit_apl.damage, 4240);
    assert_eq!(mit_apl.killed_at_ms, Some(40_510));
    assert_eq!(mit_apl, mit_funktion);
}

#[test]
fn alle_faktoren_wirken_gleich_stark() {
    // Die Schadensformel multipliziert sie alle an derselben Stelle; keiner ist
    // bevorzugt.
    //
    // **`synergy` und `vulnerability` stehen NICHT in der Liste** — sie werden
    // vom Kampf selbst geschrieben (#97 und #318) und ueberschreiben einen von
    // aussen gesetzten Wert bei der naechsten Wirkung. Der Faktor waere in
    // diesem Lauf also gar nicht der, den die Liste setzt; der Fall pruefte
    // eine Zahl, die es nie gab.
    //
    // **Was damit auf der Rust-Seite NICHT mehr geprueft wird:** dass die
    // Synergie ueberhaupt gleich stark wirkt wie die uebrigen Faktoren. Die
    // Spielseite misst es in `tests/unit/synergy.test.ts` ueber den Kampf
    // (Verhaeltnis 1,25 zwischen einer Wirkung im Fenster und einer danach);
    // ein entsprechender Fall hier fehlt noch und ist als Luecke gemeldet.
    //
    // Die Pruefsumme faengt einen Unterschied trotzdem: `synergyUsed`,
    // `synergyAvailable` und `synergyOpenUntil` stehen seit #318 darin, und
    // `SYNERGY_BONUS_PERCENT` ist auf beiden Seiten dieselbe Zahl. Das ist keine Selbstverstaendlichkeit, sondern eine Eigenschaft
    // von 6.3 — und der Grund, warum in Schritt 1 alle Skalierungswerte gleich
    // sind. Erst Krit und Tempo trennen sie (beides noch nicht umgesetzt).
    let referenz = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    })
    .damage;

    for factors in [
        DamageFactors {
            talents: 1.10,
            ..NEUTRAL
        },
        DamageFactors {
            buffs: 1.10,
            ..NEUTRAL
        },
        DamageFactors {
            resistance: 1.10,
            ..NEUTRAL
        },
        DamageFactors {
            crit: 1.10,
            ..NEUTRAL
        },
    ] {
        assert_eq!(
            lauf(factors).damage,
            referenz,
            "ein Faktor wirkt anders als die uebrigen: {factors:?}"
        );
    }
}
