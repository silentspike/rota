//! Die Zielgroessen-Verstaerkungen WIRKEN im Simulator (Issue #502).
//!
//! # Warum diese Datei noetig war
//!
//! `buff_targets.rs` prueft, dass die Pruef-Zauber verschiedene `target`
//! tragen — also die DATEN. Die Paritaet prueft, dass beide Seiten dieselbe
//! Zahl rechnen — also GLEICHHEIT. Keine von beiden prueft, dass der Faktor
//! im Kampf etwas BEWIRKT.
//!
//! Und die Paritaet kann es nicht: Alle Pruef-Zauber tragen `unlockLevel:
//! 101` und kommen in keiner Aufzeichnung vor. Beide Seiten koennten
//! identisch nichts tun, und alles waere gruen.
//!
//! # Zahlen, nicht Richtungen
//!
//! Jeder Fall nagelt den erwarteten Wert fest. Eine Richtung (`mit > ohne`)
//! ist erfuellt, sobald sich irgendetwas bewegt — auch wenn der Faktor
//! falsch herum oder auf die falsche Groesse wirkt.
//!
//! Die Zahlen stammen aus diesen Laeufen selbst. Das ist kein Zirkelschluss,
//! solange sie danach UNVERAENDERLICH dastehen und beim naechsten Umbau
//! brechen — dann verlangt der Bruch eine Erklaerung.
//!
//! # Die Fenster-Zusicherung prueft den AUFBAU, nicht das Ergebnis
//!
//! Das Fenster haelt drei Wartezeiten (4500 ms). Ein Traeger-Druck dahinter
//! misst einen Lauf OHNE Verstaerkung und meldet "wirkt nicht" — er
//! erfuende einen Fehler, den es nicht gibt. Genau das ist beim Erheben
//! dieser Zahlen einmal passiert (Druck bei 4800 ms).
//!
//! `im_fenster` faengt deshalb beide Richtungen: den falschen Alarm und das
//! falsche Gruen.

use rota_sim::data::index_of;
use rota_sim::replay::{Binding, Press, Recording};
use rota_sim::{Fight, FightConfig};

/// Laenge des Verstaerkungs-Fensters: drei Wartezeiten zu je 1500 ms.
const FENSTER_MS: u32 = 4500;
/// Der Logik-Takt des Spiels.
const TAKT_MS: u32 = 10;

/// **Liegt dieser Traeger-Druck noch im Fenster?**
///
/// Als Zusicherung im Fall selbst, nicht als Kommentar: Wer die Zeitpunkte
/// spaeter verschiebt, bekommt eine Meldung ueber den AUFBAU statt ein
/// stilles "wirkt nicht".
fn im_fenster(takt: u32) {
    assert!(
        takt * TAKT_MS < FENSTER_MS,
        "Der Traeger-Druck liegt bei {} ms, das Fenster endet bei {FENSTER_MS} ms — \
         dieser Fall misst einen Lauf OHNE Verstaerkung und meldete faelschlich \
         \"wirkt nicht\"",
        takt * TAKT_MS
    );
}

/// Faehrt einen Lauf: optional der Verstaerkungs-Zauber bei Takt 0,
/// danach die Traeger-Zauber zu den angegebenen Takten.
fn lauf(
    verstaerker: Option<&str>,
    traeger: &[(&str, u32)],
    ticks: u32,
) -> (rota_sim::replay::ReplayResult, u16) {
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
        name: "pruef".to_owned(),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    let mut fight = Fight::new(FightConfig::default());
    let result = fight.replay(&recording, false);
    let res = fight.resource();
    (result, res)
}

/// Ein Fall: gleicher Ablauf ohne und mit Verstärkung, beides festgenagelt.
macro_rules! wirkt {
    ($name:ident, $verstaerker:literal, $traeger:expr, $ticks:expr,
     $groesse:expr, $ohne:literal, $mit:literal) => {
        #[test]
        fn $name() {
            for (_, takt) in $traeger.iter() {
                im_fenster(*takt);
            }
            let a = $groesse(lauf(None, &$traeger, $ticks));
            let b = $groesse(lauf(Some($verstaerker), &$traeger, $ticks));

            // **Der Nullpunkt zuerst.** Ohne ihn belegte der Vergleich nur,
            // dass zwei Zahlen verschieden sind — nicht, dass der Träger
            // überhaupt etwas beiträgt.
            assert_eq!(
                a, $ohne,
                "der Lauf OHNE Verstärkung liefert {a} statt {}. Entweder hat sich \
                 eine Zahl in den Daten geändert, oder der Fall misst etwas anderes \
                 als beim Festnageln.",
                $ohne
            );
            assert_eq!(
                b, $mit,
                "der Lauf MIT Verstärkung liefert {b} statt {}. Die Verstärkung wirkt \
                 nicht, wirkt falsch herum, oder trifft die falsche Größe.",
                $mit
            );
        }
    };
}

wirkt!(
    resource_wirkt,
    "test-heat",
    [("kindle", 160u32)],
    200,
    |r: (rota_sim::replay::ReplayResult, u16)| u32::from(r.1),
    3,
    6
);

wirkt!(
    tick_wirkt,
    "test-tick",
    [("smoulder", 160u32)],
    900,
    |r: (rota_sim::replay::ReplayResult, u16)| r.0.damage,
    120,
    200
);

wirkt!(
    stack_wirkt,
    "test-stack",
    [("canker", 160u32), ("kindle", 320u32)],
    500,
    |r: (rota_sim::replay::ReplayResult, u16)| r.0.damage,
    154,
    162
);

wirkt!(
    control_wirkt,
    "test-control",
    [("bind", 160u32), ("wedge", 320u32)],
    500,
    |r: (rota_sim::replay::ReplayResult, u16)| r.0.damage,
    60,
    92
);
