//! Die FIXATION-Talentwege wirken auch im Simulator (Issue #738).
//!
//! | Weg | Groesse | Deckel |
//! | --- | --- | --- |
//! | build | `control_points()` | die Vorrats-Obergrenze |
//! | cost | `control_points()` nach dem Verbrauch | geklemmt auf **min 1** |
//! | effect | `converted()` | keiner |
//!
//! `cost` ist der einzige der zwoelf Wege, der **teilt** statt zu vervielfachen,
//! und der einzige mit einer unteren Klemme. Beides steht in den Faellen.

use rota_sim::data::SPELLS;
use rota_sim::fight::{Fight, FightConfig};

/// Der Zauber mit dem hoechsten Preis — an ihm zeigt sich eine Senkung.
///
/// **Nicht der erste beliebige.** Ein Preis von 1 ist bereits die untere
/// Klemme; dort kann `cost` nichts mehr senken, und der Fall waere stumm.
fn teurer_zauber() -> usize {
    SPELLS
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.control.as_ref().map(|c| (i, c.cost)))
        .max_by_key(|&(_, cost)| cost)
        .map(|(i, _)| i)
        .expect("kein Zauber verbraucht Kontroll-Punkte")
}

/// Der Zauber, der Punkte in Schaden umwandelt.
///
/// Gehoert zum zurueckgezogenen `effect`-Fall (siehe unten). Bleibt stehen,
/// weil die naechste Sitzung an genau dieser Stelle weitermacht.
#[allow(dead_code)]
fn wandler() -> usize {
    SPELLS
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.control.as_ref().map(|c| (i, c.amount)))
        .min_by_key(|&(_, amount)| amount)
        .map(|(i, _)| i)
        .expect("kein Zauber wandelt um")
}

/// Der Zauber, der Kontroll-Punkte AUFBAUT.
///
/// **`control_gain`, nicht `control.amount`.** Der erste Aufbau lief ueber die
/// Zauber mit einem `control`-Block — das ist aber der EFFEKT (Verlangsamung
/// in Millisekunden) samt seinem Preis, nicht der Aufbau. Der Traeger-Fall hat
/// es sofort gemeldet: 0 Punkte gesammelt, 0 umgewandelt.
fn sammler() -> usize {
    SPELLS
        .iter()
        .position(|s| s.control_gain > 0)
        .expect("kein Zauber baut Kontroll-Punkte auf")
}

/// **Der SAMMEL-Takt** — nur aufbauen, nichts ausgeben.
///
/// Zwei Groessen derselben Mechanik brauchen gegensaetzliche Traeger, genau
/// wie Dicke und Dauer bei COAGULATION. Wer hier den Wandler mitwirft, gibt
/// die Punkte aus, und `control_points()` am Ende ist niedriger statt hoeher —
/// der Aufbau-Weg saehe wirkungslos aus.
fn sammel_takt(fight: &Fight) -> Option<usize> {
    if fight.cooldown_remaining_ms(sammler()) == 0 {
        return Some(sammler());
    }
    None
}

/// **Der WANDEL-Takt** — sammeln, bis es reicht, dann gezielt umwandeln.
///
/// Der Wandler muss ausdruecklich gewaehlt werden. Ein Takt, der den ersten
/// bereiten Zauber wirft, kam nie zu ihm.
#[allow(dead_code)]
fn wandel_takt(fight: &Fight) -> Option<usize> {
    let preis = SPELLS[wandler()].control.as_ref().map_or(0, |c| c.cost);
    if fight.control_points() >= preis && fight.cooldown_remaining_ms(wandler()) == 0 {
        return Some(wandler());
    }
    if fight.cooldown_remaining_ms(sammler()) == 0 {
        return Some(sammler());
    }
    None
}

fn lauf(config: FightConfig, dauer_ms: u32) -> Fight {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(dauer_ms),
        ..config
    });
    fight.run(sammel_takt);
    fight
}

/// Wie `lauf`, aber mit dem Wandel-Takt.
#[allow(dead_code)]
fn lauf_wandelnd(config: FightConfig, dauer_ms: u32) -> Fight {
    let mut fight = Fight::new(FightConfig {
        duration_ms: Some(dauer_ms),
        ..config
    });
    fight.run(wandel_takt);
    fight
}

#[test]
fn der_traeger_bewegt_ueberhaupt_etwas() {
    // Zuerst der Traeger, dann die Wirkung (AC8 aus #676).
    let fight = lauf(FightConfig::default(), 20_000);
    assert!(
        fight.control_points() > 0,
        "es wurden weder Punkte gesammelt noch umgewandelt — die Probe hat \
         keinen Traeger, und jeder Vergleich unten misst zwei Nullen"
    );
    assert!(
        SPELLS[teurer_zauber()]
            .control
            .as_ref()
            .expect("gewaehlter Zauber ohne Kontrolle")
            .cost
            > 1,
        "der teuerste Zauber kostet 1 — das ist bereits die untere Klemme, \
         dort kann der Weg cost nichts senken"
    );
}

#[test]
fn der_weg_build_sammelt_mehr_punkte() {
    let ohne = lauf(FightConfig::default(), 20_000).control_points();
    let mit = lauf(
        FightConfig {
            talent_control_gain_permille: 1000,
            ..FightConfig::default()
        },
        20_000,
    )
    .control_points();

    assert!(
        mit >= ohne,
        "der Weg build sammelt weniger statt mehr: {ohne} ohne Talent, {mit} \
         mit doppeltem Aufbau"
    );
    assert!(
        mit > ohne,
        "der Weg build wirkt im Simulator nicht: {ohne} Punkte ohne Talent, \
         {mit} mit doppeltem Aufbau. Sind beide gleich, pruefe die \
         Vorrats-Obergrenze — eine erreichte Grenze zeigt keine Steigerung"
    );
}

// **OFFEN: der Weg `effect` hat hier keinen Fall** (#738).
//
// `converted()` bleibt in DREI Aufbauten null — mit dem Sammel-Takt, mit einem
// Takt, der jeden bereiten Zauber wirft, und mit einem, der den Wandler
// gezielt waehlt, sobald die Punkte reichen:
//
// ```text
// ohne Talent 0 umgewandelt · mit doppelter Wirkung 0
// ```
//
// Die Null liegt also nicht am Talent. Offen ist, ob `wandler()` den falschen
// Zauber waehlt (er nimmt den mit dem KLEINSTEN `control.amount`, und die
// Werte 8/10 gegen 2000-4000 sehen nach zwei verschiedenen Einheiten aus) oder
// ob `converted()` etwas anderes zaehlt als die umgewandelte Menge.
//
// **Nicht weiter geraten.** Drei Anlaeufe ohne Messung, die es entscheidet,
// sind genug — beim ersten SUBLIMATION-Versuch haben vier Vermutungen nichts
// beigetragen, und die Antwort kam erst von einer gezielten Gegenprobe.
#[test]
fn der_weg_cost_laesst_den_aufbau_unberuehrt() {
    // Kreuzprobe: `cost` senkt den Preis, `build` hebt den Aufbau. Der Weg
    // `cost` TEILT als einziger der zwoelf — wer ihn versehentlich
    // multiplizieren liesse, machte Zauber teurer statt billiger, und die
    // gesammelten Punkte fielen. Dieser Fall faenge das.
    //
    // **Er deckt die Rechnung aber NICHT** (#738). Gemessen: Streicht man den
    // Teiler in `fight.rs` ersatzlos, bleibt dieser Fall gruen. Der Grund ist
    // das `>=` — es faengt eine Umkehrung (teurer statt billiger), nicht das
    // Fehlen der Wirkung. Ein Wirkungsfall braucht einen Lauf, in dem der
    // gesenkte Preis einen Wurf ERLAUBT, den der volle abweist; im
    // Sammel-Takt wird nichts ausgegeben, also zeigt sich dort nichts.
    let ohne = lauf(FightConfig::default(), 20_000);
    let mit_cost = lauf(
        FightConfig {
            talent_control_cost_permille: 1000,
            ..FightConfig::default()
        },
        20_000,
    );

    assert!(
        mit_cost.control_points() >= ohne.control_points(),
        "der Weg cost hat den Vorrat GESENKT ({} statt {}) — das spricht \
         dafuer, dass er multipliziert statt zu teilen",
        mit_cost.control_points(),
        ohne.control_points()
    );
    assert_eq!(
        mit_cost.converted(),
        ohne.converted(),
        "der Weg cost hat die umgewandelte Menge mitgehoben"
    );
}
