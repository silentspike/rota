//! COAGULATIONs Schluesseltalent rechnet auf beiden Seiten gleich (#419).
//!
//! Ein brechender Schild legt eine HALBE Schicht nach — einmal je gelegtem
//! Schild. Die Begrenzung ist der Kern: Ohne sie waere die Schicht nicht mehr
//! aufzubrauchen.
//!
//! # Was die Paritaet traegt
//!
//! Die abgerundete Haelfte der ZULETZT GELEGTEN Dicke (nicht des Restes), und
//! dass der Schlag trotzdem durchkommt, soweit er die Schicht uebersteigt.
//! Dieselbe Rechnung steht in `combat.ts` und im Spiel-Fall
//! `tests/unit/keystone-shield-refund.test.ts`.

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Legt eine Schicht mit CRUST und schlaegt zweimal hart zu.
///
/// Gibt (Dicke der Schicht, Schild nach dem ersten Bruch, Schild nach dem
/// zweiten).
fn schild_nach(ticks: u32, shield_refund: bool) -> u32 {
    // **test-onslaught schlaegt hart genug**: Sein Zauber traegt 2000, die
    // Schicht 900. Ein eigener Treffer-Aufruf waere eine Methode, die es nur
    // fuer den Test gaebe — hier laeuft der Schlag den normalen Weg.
    //
    // **Der Zauber TRIFFT erst nach seiner Wirkzeit**, nicht wenn er beginnt
    // (castTimeMs 2000). Mein erster Entwurf mass bei 3500 ms und sah eine
    // unversehrte Schicht — der Schlag lag noch in der Luft.
    //
    //     3000 ms + 2000 Wirkzeit = 5000   erster Treffer  -> Nachlage
    //    10000 ms + 2000 Wirkzeit = 12000  zweiter Treffer -> nichts mehr
    let boss = boss_by_id("test-onslaught").expect("test-onslaught fehlt");
    let crust = index_of("crust").expect("crust fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        shield_refund,
        ..FightConfig::default()
    });
    let recording = Recording {
        name: "shield-refund".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![Binding {
            slot: 0,
            spell: crust,
        }],
        presses: vec![Press { tick: 0, slot: 0 }],
    };
    fight.replay(&recording, false);
    fight.shield()
}

/// Wie dick CRUST legt — vor jedem Treffer.
fn gelegte_dicke() -> u32 {
    schild_nach(10, false)
}

/// **Die Zusicherung vor der Messung.** Ohne Talent muss die Schicht nach dem
/// ersten Bruch weg sein — sonst misst der Vergleich unten nichts.
#[test]
fn ohne_talent_ist_die_schicht_weg() {
    assert!(
        gelegte_dicke() > 0,
        "CRUST legt keine Schicht — der Fall misst nichts"
    );
    assert_eq!(
        schild_nach(550, false),
        0,
        "ohne Talent darf nach dem Bruch nichts nachkommen"
    );
}

#[test]
fn mit_talent_kommt_die_haelfte_nach() {
    assert_eq!(
        schild_nach(550, true),
        gelegte_dicke() / 2,
        "mit Talent muss die halbe Schicht nachkommen"
    );
}

#[test]
fn und_nur_einmal() {
    assert!(schild_nach(550, true) > 0, "die erste Nachlage fehlt");
    assert_eq!(
        schild_nach(1250, true),
        0,
        "die zweite Nachlage darf es nicht geben"
    );
}

// ---------------------------------------------------------------------------
// Die Nachlage gibt nur der gelegte Schild, der bricht (#419)
// ---------------------------------------------------------------------------
//
// Spiegelbild des gleichnamigen Blocks in
// `tests/unit/keystone-shield-refund.test.ts`, mit demselben Ablauf gegen
// `test-vessel`:
//
//       0 ms  CRUST            900
//    5000 ms  Schlag (320)     faengt sie auf, 580 bleiben
//   12 000 ms                  CRUST laeuft ab, ungebrochen
//   13 000 ms  TEMPER, 14 500 ms KINDLE   -> 25 aus Umwandlung
//   15 000 ms  Ueberladung (160) bricht die 25
//
// Die Umwandlung wird nicht gelegt, sie waechst; sie schreibt weder Dicke
// noch Merker. Ohne Regel gab ihr Bruch die Haelfte der abgelaufenen CRUST.

/// Der Ablauf bis `ticks - 1`; gibt den Kampf nach dem Lauf zurueck.
fn ablauf(ticks: u32, shield_refund: bool, plan: &[(u32, &str)], boss_id: &str) -> Fight {
    let mut zauber: Vec<&str> = Vec::new();
    for (_, id) in plan {
        if !zauber.contains(id) {
            zauber.push(id);
        }
    }
    let recording = Recording {
        name: "nachlage-nur-gelegt".to_owned(),
        seed: 0,
        ticks,
        bindings: zauber
            .iter()
            .enumerate()
            .map(|(slot, id)| Binding {
                slot: u8::try_from(slot).expect("zu viele Plaetze"),
                spell: index_of(id).unwrap_or_else(|| panic!("{id} fehlt")),
            })
            .collect(),
        presses: plan
            .iter()
            .map(|(tick, id)| Press {
                tick: *tick,
                slot: u8::try_from(zauber.iter().position(|z| z == id).expect("Platz"))
                    .expect("Platz"),
            })
            .collect(),
    };
    let mut fight = Fight::new(FightConfig {
        boss: boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt")),
        shield_refund,
        ..FightConfig::default()
    });
    let _ = fight.replay(&recording, false);
    fight
}

const ABLAUF: [(u32, &str); 3] = [(0, "crust"), (1300, "temper"), (1450, "kindle")];

#[test]
fn die_crust_laeuft_ungebrochen_ab_und_die_umwandlung_legt_eine_eigene_schicht() {
    // **Die Zusicherung vor der Messung.** Braeche die CRUST unter dem
    // Schlag, waere ihr Merker verbraucht, und der Fall unten waere gruen ohne
    // die Regel.
    let vor_ablauf = ablauf(1200, true, &ABLAUF, "test-vessel");
    assert_eq!(
        vor_ablauf.shield(),
        580,
        "die CRUST faengt den Schlag nicht auf"
    );
    let nach_ablauf = ablauf(1202, true, &ABLAUF, "test-vessel");
    assert_eq!(nach_ablauf.shield(), 0, "die CRUST laeuft nicht ab");
    assert_eq!(nach_ablauf.shield_absorbed(), 320);

    let vor_dem_bruch = ablauf(1500, true, &ABLAUF, "test-vessel");
    assert!(vor_dem_bruch.converted() > 0, "TEMPER wandelt nichts um");
    assert_eq!(vor_dem_bruch.shield(), vor_dem_bruch.converted());
    assert_eq!(
        vor_dem_bruch.shield_stacks(),
        0,
        "die Umwandlung ist keine gelegte Schicht"
    );
}

#[test]
fn bricht_die_umwandlungs_schicht_kommt_nichts_nach() {
    let ohne = ablauf(1501, false, &ABLAUF, "test-vessel");
    assert_eq!(ohne.shield(), 0, "die Ueberladung bricht die Schicht nicht");
    assert_eq!(ohne.shield_absorbed(), 320 + ohne.converted());

    let mit = ablauf(1501, true, &ABLAUF, "test-vessel");
    assert_eq!(
        mit.shield(),
        0,
        "die abgelaufene CRUST legt ihre Haelfte nach"
    );
    assert_eq!(mit.shield_stacks(), 0);
}

#[test]
fn liegt_die_umwandlung_auf_einer_gelegten_schicht_gibt_diese_ihre_haelfte() {
    // **Die Gegenrichtung**, damit die Regel nicht zu viel sperrt: Bricht
    // eine gelegte CRUST, auf der Umwandlung liegt, ist es die CRUST, die
    // bricht — und ihre Haelfte kommt nach. Gegen `test-onslaught` (2000 bei
    // 5000 ms).
    let plan = [(0, "crust"), (150, "temper"), (300, "kindle")];
    let gelegt = gelegte_dicke();
    let vor_dem_schlag = ablauf(500, true, &plan, "test-onslaught");
    assert!(
        vor_dem_schlag.shield() > gelegt,
        "auf der CRUST liegt keine Umwandlung"
    );
    let nach_dem_schlag = ablauf(501, true, &plan, "test-onslaught");
    assert_eq!(
        nach_dem_schlag.shield(),
        gelegt / 2,
        "die gelegte CRUST gibt ihre Haelfte nicht"
    );
}
