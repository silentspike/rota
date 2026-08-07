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

/// **Die GROESSE, nicht die Existenz** (Issue #547).
///
/// Der Fall darunter ist gegen eine Balance-Aenderung blind: Mit
/// `factor: 1.05` statt `1.5` wird aus fuenfzig Prozent Anhebung eine von
/// fuenf, und `> 20` bleibt wahr, weil 21 groesser als 20 ist.
///
/// # Warum 22 und nicht 30
///
/// Der angehobene Deckel liegt bei `round(20 * 1.5) = 30`, aber er wird hier
/// nie beruehrt: Das Fenster ist 4 GCDs lang, der GCD 1500 ms, also passen
/// drei BELLOWS-Wuerfe hinein — 3 x 8 = 24, abzueglich Verfall (1 je 2000 ms)
/// bleiben 22. Die oberen acht Punkte der Anhebung sind heute unerreichbar;
/// das ist ein Balance-Befund (Worker-2 in #547), kein Fehler.
///
/// Der Fall prueft die Groesse deshalb INDIREKT und trotzdem scharf: Bei
/// `1.05` klemmt der Deckel bei 21 und schneidet die 22 ab. Gemessen, nicht
/// hergeleitet — die Gegenprobe steht im PR.
///
/// Auf der Spiel-Seite geht es direkter: Dort klemmt `setResource` gegen
/// `resourceCap`, und der Fall prueft die 30 selbst. Ein `set_resource` gibt
/// es im Simulator nicht (fight.rs, Kommentar an der cap-Lesestelle).
///
/// # Die 22 steht HART und wird nicht gerechnet
///
/// Ein Sollwert aus derselben Quelle wie der Istwert ist keiner: Wer ihn aus
/// `slag.buff.factor` herleitete, laese bei der Gegenprobe die mutierte Zahl
/// mit und bliebe gruen.
#[test]
fn und_zwar_um_die_haelfte_nicht_nur_irgendwie() {
    assert_eq!(
        hoechster_vorrat(Some("slag"), &TRAEGER, 470),
        22,
        "SLAG hebt die Grenze zwar, aber nicht um die 50 % aus dem Design (#490)"
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

/// Ein Erzeuger nimmt nie weg — und nichts laeuft ueber (Issue #549).
///
/// # Der Fall, der den Simulator zum Absturz brachte
///
/// SLAG hebt die Grenze von 20 auf 30. Wer im Fenster ueber 20 sammelt und es
/// ablaufen laesst, steht ueber dem dann wieder geltenden Deckel. Ein Wurf
/// rechnete dort `self.resource - before` mit einem GROESSEREN `before` —
/// u16-Subtraktion, Absturz.
///
/// Reproduziert mit Worker-1s Ticks: BELLOWS auf 610 stuerzte ab, derselbe
/// Wurf auf 590 (Fenster noch offen) nicht. Die Gegenprobe steht unten als
/// eigener Fall, damit sichtbar bleibt, dass das Fenster der Unterschied ist.
#[test]
fn ein_erzeuger_ueber_dem_deckel_nimmt_nicht_weg() {
    // Drei BELLOWS im Fenster bringen den Vorrat ueber 20; der vierte faellt
    // NACH dem Fensterende (4 GCDs = 6000 ms = Tick 600).
    let mit = hoechster_vorrat(Some("slag"), &TRAEGER, 480);
    assert!(
        mit > 20,
        "der Vorrat steht nicht ueber dem Grund-Deckel ({mit}) — dann prueft \
         dieser Fall nichts"
    );

    let traeger_danach: [(&str, u32); 4] = [
        ("bellows", 160),
        ("bellows", 310),
        ("bellows", 460),
        ("bellows", 610),
    ];
    let danach = hoechster_vorrat(Some("slag"), &traeger_danach, 630);
    assert!(
        danach >= 20,
        "nach dem Wurf steht der Vorrat bei {danach} — ein Erzeuger hat \
         weggenommen"
    );
}

/// Die Gegenprobe: derselbe Wurf INNERHALB des Fensters.
///
/// Ohne sie belegte der Fall darueber nur, dass irgendein Lauf durchlaeuft —
/// nicht, dass das Fensterende der Unterschied ist.
#[test]
fn derselbe_wurf_innerhalb_des_fensters_laeuft_ebenso_durch() {
    let traeger: [(&str, u32); 4] = [
        ("bellows", 160),
        ("bellows", 310),
        ("bellows", 460),
        ("bellows", 590),
    ];
    let vorrat = hoechster_vorrat(Some("slag"), &traeger, 610);
    assert!(
        vorrat > 20,
        "im offenen Fenster muss der Vorrat ueber dem Grund-Deckel stehen, \
         steht aber bei {vorrat}"
    );
}
