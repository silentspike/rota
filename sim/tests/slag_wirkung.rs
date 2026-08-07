//! SLAG hebt die Hitze-Grenze — im Simulator, nicht nur im Spiel (Issue #490).
//!
//! # Warum dieser Fall eigenstaendig ist
//!
//! Der gemeinsame Wirkungs-Prueffall fuer die Zielgroessen liegt auf dem
//! #502-Zweig (`buff_wirkung.rs`). Dieser Zweig haengt an #504, nicht an
//! #502 — die beiden treffen sich erst beim Merge.
//!
//! Statt zu warten steht der Fall hier fuer sich. Beim Zusammenfuehren
//! gehoert er in `buff_wirkung.rs`; bis dahin ist die Wirkung belegt statt
//! angenommen.
//!
//! # BELLOWS, nicht KINDLE
//!
//! BELLOWS gibt 8 Hitze je Wurf und erreicht die Grenze in drei Wuerfen.
//! KINDLE gibt 3, der Verfall nimmt 1 je 2000 ms — der Vorrat kommt gar nicht
//! an die Grenze, und zwei Zahlen DARUNTER zu vergleichen belegt nichts.
//!
//! Genau das ist im Spiel-Prueffall zuerst passiert: 18 gegen 19, beide unter
//! 20, und der Fall haette "SLAG wirkt nicht" behauptet.

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig};

/// Faehrt einen Lauf und liefert den Vorrat am ENDE.
///
/// **Der Lauf endet direkt nach dem letzten Wurf**, nicht spaeter:
/// `ReplayResult` fuehrt keinen Vorrat-Verlauf, also gibt es nur den
/// Endstand — und der faellt mit jeder weiteren Sekunde durch den Verfall.
/// Bei Takt 470 ist er noch der Hoechststand.
fn hoechster_vorrat(verstaerker: Option<&str>, traeger: &[(&str, u32)], ticks: u32) -> u16 {
    let mut bindings = Vec::new();
    let mut presses = Vec::new();
    let mut slot = 0u8;
    if let Some(v) = verstaerker {
        bindings.push(Binding {
            slot,
            spell: index_of(v).unwrap_or_else(|| panic!("{v} fehlt")),
        });
        presses.push(Press { tick: 0, slot });
        slot += 1;
    }
    for (id, tick) in traeger {
        let s = slot;
        bindings.push(Binding {
            slot: s,
            spell: index_of(id).unwrap_or_else(|| panic!("{id} fehlt")),
        });
        presses.push(Press {
            tick: *tick,
            slot: s,
        });
        slot += 1;
    }
    let recording = Recording {
        name: "slag-pruef".to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    let mut fight = Fight::new(FightConfig::default());
    let _ = fight.replay(&recording, true);
    fight.resource()
}

const TRAEGER: [(&str, u32); 3] = [("bellows", 160), ("bellows", 310), ("bellows", 460)];

#[test]
fn ohne_slag_steht_der_vorrat_an_der_grenze() {
    // **Der Nullpunkt.** Ohne ihn belegte der Fall unten nur, dass zwei Zahlen
    // verschieden sind — nicht, dass die Grenze ueberhaupt erreicht wird.
    assert_eq!(
        hoechster_vorrat(None, &TRAEGER, 470),
        20,
        "der Vorrat erreicht die Grenze 20 nicht — dann misst der Fall unten nichts"
    );
}

#[test]
fn mit_slag_steigt_er_darueber() {
    let mit = hoechster_vorrat(Some("slag"), &TRAEGER, 470);
    assert!(
        mit > 20,
        "mit SLAG bleibt der Vorrat bei {mit} — die cap-Verstaerkung wirkt im Simulator nicht"
    );
}
