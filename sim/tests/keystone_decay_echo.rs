//! PUTREFACTIONs Schluesseltalent rechnet auf beiden Seiten gleich (#419).
//!
//! Ein ERLOSCHENER Brand legt weiter Zersetzungs-Stapel — bis die Lage
//! vergangen ist, die bei seinem Ende lag. Nur Stapel, kein Schaden.
//!
//! # Was die Paritaet traegt
//!
//! Drei Dinge, und jedes hat unten einen eigenen Fall: dass ueberhaupt
//! nachgelegt wird, dass dabei kein Schaden entsteht, und dass es ENDET.
//! Dieselben drei stehen im Spiel-Fall `tests/unit/keystone-decay-echo.test.ts`.
//!
//! # Der Vorgaenger war wirkungslos
//!
//! Der erste Entwurf liess beim Verfall eines Stapels jeden LAUFENDEN Brand
//! einmal ticken — nur ist der Brand dann laengst aus. Der erste Fall unten
//! haelt genau diese Luecke fest: Ohne sie haette das Talent keinen Traeger.
//!
//! # Die Rechnung zu jedem Zeitpunkt
//!
//! ROT: `tickIntervalMs` 1000, `durationMs` 4000, `decayPerTick` 1 — also vier
//! Ticks und vier Stapel. `DECAY_STACK_DURATION_MS` ist 12000, der letzte
//! Stapel liegt somit bis etwa 16000 ms.
//!
//! **Ein Lauf ueber `ticks` Takte fuehrt die Takte 0 bis `ticks - 1` aus.**
//! Gemessen: 400 Takte enden bei 3990 ms mit drei Ticks, 401 Takte bringen den
//! vierten. Die Zeitpunkte unten liegen deshalb bewusst auf Plateaus und nicht
//! an solchen Kanten:
//!
//! ```text
//!    401 Takte =  4010 ms   alle vier Ticks liegen
//!   2000 Takte = 20000 ms   ohne Talent leer, mit Talent noch drei Stapel
//!   4000 Takte = 40000 ms   auch mit Talent leer
//! ```

use rota_sim::data::{boss_by_id, index_of};
use rota_sim::fight::{Fight, FightConfig};
use rota_sim::replay::{Binding, Press, Recording};

/// Wirkt ROT auf Takt 0 und laesst den Kampf `ticks` Takte laufen.
///
/// Gibt (liegende Stapel, ausgeteilter Schaden, Brand-Ticks).
fn nach_rot(ticks: u32, decay_echo: bool) -> (usize, u32, u32) {
    let boss = boss_by_id("first-crucible").expect("first-crucible fehlt");
    let rot = index_of("rot").expect("rot fehlt");

    let mut fight = Fight::new(FightConfig {
        boss,
        decay_echo,
        ..FightConfig::default()
    });
    let recording = Recording {
        name: "decay-echo".to_owned(),
        seed: 0,
        ticks,
        bindings: vec![Binding {
            slot: 0,
            spell: rot,
        }],
        presses: vec![Press { tick: 0, slot: 0 }],
    };
    fight.replay(&recording, false);
    (
        fight.decay_stacks(),
        fight.damage(),
        fight.dot_tick_count(rot),
    )
}

/// **Die Zusicherung vor der Messung.** ROT muss ueberhaupt ticken und Stapel
/// legen — sonst vergleichen die Faelle unten zwei Nullen.
#[test]
fn rot_legt_ueberhaupt_stapel() {
    let (stapel, _, ticks) = nach_rot(401, false);
    assert_eq!(ticks, 4, "ROT muss vier Mal ticken (4000 ms / 1000 ms)");
    assert_eq!(stapel, 4, "vier Ticks legen vier Stapel (decayPerTick 1)");
}

#[test]
fn ohne_talent_ist_die_zersetzung_vorbei() {
    assert_eq!(
        nach_rot(2000, false).0,
        0,
        "ohne Talent darf nach dem Brand nichts nachwachsen"
    );
}

#[test]
fn mit_talent_liegen_dort_noch_stapel() {
    assert!(
        nach_rot(2000, true).0 > 0,
        "der erloschene Brand muss weiter gelegt haben"
    );
}

#[test]
fn und_er_legt_nur_stapel_keinen_schaden() {
    // **Der Unterschied zu "Lasting Rot"** — eine laengere Laufzeit gibt es
    // schon. Tick-Zahl UND Schaden muessen gleich bleiben.
    let (_, schaden_ohne, ticks_ohne) = nach_rot(2000, false);
    let (_, schaden_mit, ticks_mit) = nach_rot(2000, true);
    assert!(ticks_ohne > 0, "kein Tick — der Vergleich misst nichts");
    assert_eq!(
        ticks_mit, ticks_ohne,
        "der Nachlauf darf keinen Brand-Tick hinzufuegen"
    );
    assert_eq!(
        schaden_mit, schaden_ohne,
        "der Nachlauf darf keinen Schaden austeilen"
    );
}

#[test]
fn und_er_endet() {
    // **Ohne diesen Fall waere das Talent ein Selbstlaeufer.** Truege ein
    // nachgelegter Stapel den naechsten, liefe die Zersetzung nach einem
    // einzigen Tastendruck fuer immer weiter.
    assert!(
        nach_rot(2000, true).0 > 0,
        "der Nachlauf fehlt — der Fall misst nichts"
    );
    assert_eq!(
        nach_rot(4000, true).0,
        0,
        "der Nachlauf darf sich nicht selbst tragen"
    );
}
