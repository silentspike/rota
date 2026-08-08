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

/// Die globale Wartezeit: kein zweiter Druck davor.
const GCD_MS: u32 = 1500;

/// **Liegen die Traeger-Druecke weit genug auseinander?**
///
/// Der Pruef-Zauber loest bei Takt 0 selbst den GCD aus, und jeder Traeger
/// ebenso. Ein Druck vor Ablauf wird ABGEWIESEN — der Lauf sieht dann aus
/// wie einer, in dem die Verstaerkung nicht wirkt, obwohl der Zauber gar
/// nicht gewirkt hat.
///
/// Mir ist das zweimal passiert: einmal in `buff-targets.test.ts` (0 gegen
/// 2 Punkte, BIND hatte nie gewirkt) und einmal hier, nachdem ich die Falle
/// selbst gemeldet hatte. Deshalb steht sie jetzt als Zusicherung da und
/// nicht als Merksatz.
fn nach_dem_gcd(vorher: u32, jetzt: u32) {
    let abstand = (jetzt - vorher) * TAKT_MS;
    assert!(
        abstand >= GCD_MS,
        "Zwei Traeger-Druecke liegen {abstand} ms auseinander, die Wartezeit ist \
         {GCD_MS} ms — der zweite wird ABGEWIESEN, und der Fall misst einen Lauf, \
         in dem der Zauber nie gewirkt hat"
    );
}

/// **Drei Rollen, die ein Druck spielen kann** — und nur eine davon gehoert
/// ins Fenster:
///
/// | Rolle | was sie tut | Auflage |
/// | --- | --- | --- |
/// | Vorbereitung | bringt den Zustand in die Ausgangslage | nur GCD-Abstand |
/// | **Traeger** | **nutzt die verstaerkte Groesse** | **GCD UND im Fenster** |
/// | Zeuge | zeigt, was dabei herauskam | nur GCD-Abstand |
///
/// **"Nicht im Fenster" hat zwei verschiedene Gruende:** noch nicht wirksam
/// (Vorbereitung) gegen schon vorbei (Zeuge). Beide sind richtig.
///
/// # Warum das nicht offensichtlich ist
///
/// Bei `resource`, `tick`, `stack` und `control` fallen alle drei Rollen auf
/// denselben Zauber: KINDLE nutzt die Verstaerkung und zeigt sie zugleich.
/// Solange das so ist, sieht eine Zusicherung ueber "den Druck" wie eine
/// vollstaendige Regel aus — sie ist aber ein Sonderfall.
///
/// `cdreduce` ist der erste Fall, der ihn verlaesst: `pyre` MUSS hinter dem
/// Fenster liegen, weil seine Abklingzeit sonst gar nicht abgelaufen ist.
/// Eine Zusicherung, die jeden Druck ins Fenster zwingt, lehnte den einzig
/// richtigen Aufbau ab.
///
/// Dieselbe Form wie `casts` gegen "Wirkung": zwei Groessen, die im Normalfall
/// zusammenfallen und im interessanten Fall auseinandergehen.
///
/// # Diese Zusicherung gilt nur fuer TRAEGER
///
/// Nur fuer Traeger aufrufen — fuer Vorbereitung und Zeuge waere es falsch.
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
    // **`true`, nicht `false`.** Der zweite Parameter schaltet die Verlaeufe je
    // Takt ein (`shield`, `player_hp`, …). Mit `false` bleibt `r.shield` LEER,
    // und ein Fall, der daraus den Hoechststand liest, misst 0 gegen 0 — er
    // waere gruen und belegte nichts.
    let result = fight.replay(&recording, true);
    let res = fight.resource();
    (result, res)
}

/// Ein Fall: gleicher Ablauf ohne und mit Verstärkung, beides festgenagelt.
macro_rules! wirkt {
    ($name:ident, $verstaerker:literal, $traeger:expr, $ticks:expr,
     $groesse:expr, $ohne:literal, $mit:literal) => {
        #[test]
        fn $name() {
            let mut vorher = 0u32;
            for (_, takt) in $traeger.iter() {
                im_fenster(*takt);
                nach_dem_gcd(vorher, *takt);
                vorher = *takt;
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

// **Die Grenze dieses Falls: er prueft die RICHTUNG, nicht die Zahl**
// (Issue #640). Die Schranken 120/200 sagen "mehr als ohne, weniger als
// unsinnig" — welcher ABSTAND herauskommt, steht hier nirgends.
//
// `tick` ist die einzige Zielgroesse, deren Faktor TEILT statt zu
// multiplizieren, und `dot_interval` verrechnet dazu noch eine Rundung und die
// Tempo-Rechnung. Bis #640 stand auf der Spiel-Seite genau derselbe
// Richtungs-Waechter (`tests/unit/buff-targets.test.ts`), und **kein
// Paritaets-Fall verglich die Zahlen** — `test-tick` wurde in keiner
// Aufzeichnung gedrueckt. Das tut jetzt `parity_tick_rundung`.
//
// Dieser Fall bleibt als Diagnose: Wird er rot und der Paritaets-Fall nicht,
// hat sich die Wirkung geaendert und nicht der Gleichlauf.
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

/// Wie oft dieser Zauber gewirkt hat.
///
/// **`casts` zählt Wirkungen, nicht Drücke** (`replay.rs:89`) — Drücke stehen
/// in `accepted`/`rejected`. Für die Abklingzeit ist das genau richtig: Ein
/// Zauber, der noch im Cooldown steht, wird nicht gewirkt.
fn wirkungen(r: &rota_sim::replay::ReplayResult, id: &str) -> u32 {
    r.casts[index_of(id).unwrap_or_else(|| panic!("{id} fehlt"))]
}

/// Die zwei Zielgrößen aus E10 — mit **Träger** und **Zeuge** getrennt.
///
/// # Warum die Trennung nötig ist
///
/// Bei den vier Fällen oben sind Träger und Zeuge derselbe Zauber: KINDLE
/// nutzt die Verstärkung und zeigt sie zugleich. Hier nicht:
///
/// - `vapour` **nutzt** die verstärkte Verkürzung — muss ins Fenster
/// - `pyre` **zeigt** nur, ob er durchkam — muss DAHINTER liegen, weil seine
///   Abklingzeit sonst gar nicht abgelaufen ist
///
/// Eine Zusicherung, die für jeden Druck das Fenster verlangt, lehnte den
/// richtigen Aufbau ab.
#[test]
fn cdreduce_wirkt() {
    // PYRE@1600 ms ist bereit ab 6600. VAPOUR@3200 verkürzt um 1000 → 5600,
    // verdoppelt um 2000 → 4600. Der Druck bei 5000 ms fällt genau dazwischen
    // — deshalb ist die Zahl der Wirkungen die Wirkung selbst.
    // pyre@160   VORBEREITUNG — setzt die Abklingzeit erst in Gang
    // vapour@320 TRÄGER       — nutzt die verstärkte Verkürzung
    // pyre@500   ZEUGE        — kam er durch? Muss HINTER dem Fenster liegen,
    //                           sonst wäre seine Abklingzeit gar nicht abgelaufen
    let traeger = [("pyre", 160u32), ("vapour", 320u32), ("pyre", 500u32)];
    im_fenster(320); // nur der Träger
    let mut vorher = 0;
    for (_, takt) in traeger.iter() {
        nach_dem_gcd(vorher, *takt);
        vorher = *takt;
    }

    let ohne = wirkungen(&lauf(None, &traeger, 700).0, "pyre");
    let mit = wirkungen(&lauf(Some("test-cdreduce"), &traeger, 700).0, "pyre");

    assert_eq!(
        ohne, 1,
        "ohne Verstärkung kommt der zweite PYRE bereits durch — dann misst der \
         Fall die Verkürzung nicht"
    );
    assert_eq!(mit, 2, "die cdreduce-Verstärkung wirkt nicht");
}

#[test]
fn convert_wirkt() {
    // TEMPER wandelt 25 % des AUSGETEILTEN Schadens — kein Boss-Schlag nötig.
    // KINDLE@3200 muss vor dem Fensterende bei 4500 liegen.
    // temper@160 VORBEREITUNG — öffnet die Umwandlung
    // kindle@320 TRÄGER       — teilt den Schaden aus, der gewandelt wird
    // Kein Zeuge: Der Schutzwert wird direkt gelesen. Deshalb dürfen hier
    // ausnahmsweise beide im Fenster liegen — und müssen es auch.
    let traeger = [("temper", 160u32), ("kindle", 320u32)];
    let mut vorher = 0;
    for (_, takt) in traeger.iter() {
        im_fenster(*takt);
        nach_dem_gcd(vorher, *takt);
        vorher = *takt;
    }

    let hoechster = |r: rota_sim::replay::ReplayResult| r.shield.iter().copied().max().unwrap_or(0);
    let ohne = hoechster(lauf(None, &traeger, 400).0);
    let mit = hoechster(lauf(Some("test-convert"), &traeger, 400).0);

    assert_eq!(ohne, 25, "ohne Verstärkung entsteht {ohne} Schutz statt 25");
    assert_eq!(
        mit, 50,
        "die convert-Verstärkung wirkt nicht: {mit} statt 50"
    );
}
