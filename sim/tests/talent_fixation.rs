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

use rota_sim::data::{ControlEffect, SPELLS};
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
/// **Gewaehlt am EFFEKT, nicht an einer Zahl.** Der erste Aufbau suchte "den
/// mit dem kleinsten `control.amount`" — ein Feld, das es so nicht gibt. Die
/// Daten fuehren drei getrennte Felder mit ihrer Einheit im Namen
/// (`amountMs`, `amountPercent`, `amountPerPoint`), und die Zahlen 8 und 10
/// gegen 2000-4000 sahen nur deshalb nach zwei Einheiten in einem Feld aus.
///
/// `ConvertToDamage` ist die Eigenschaft, die den Wandler ausmacht, und sie
/// ueberlebt jede Zahlenaenderung — dieselbe Regel wie bei den
/// SUBLIMATION-Helfern, die "der erste mit einer Schwelle" suchen.
///
/// **`AllThree` ist bewusst NICHT dabei.** `capstone` traegt ihn und wandelt
/// damit auch um; er kostet aber 8 statt 3, und ein Fall, der je nach
/// Katalogreihenfolge mal den einen und mal den anderen naehme, vergliche zwei
/// verschiedene Dinge.
fn wandler() -> usize {
    SPELLS
        .iter()
        .position(|s| {
            s.control
                .as_ref()
                .is_some_and(|c| c.effect == ControlEffect::ConvertToDamage)
        })
        .expect("kein Zauber wandelt Punkte in Schaden um")
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

/// Der Schaden EINES Zaubers, nicht der Gesamtschaden.
///
/// **`damage_by_spell` statt `converted()`.** Der naheliegende Getter zaehlt
/// die Gegenrichtung: `fight.rs:2409` addiert dort Schaden, der in SCHILD
/// umgewandelt wurde (QUICKSILVER) — nicht Kontroll-Punkte, die in Schaden
/// werden. Zwei Umwandlungen in entgegengesetzte Richtungen, ein Wort.
///
/// Und nicht der Gesamtschaden: Der haengt an einem Dutzend Groessen, ein
/// Unterschied dort sagte nichts ueber DIESEN Weg. `damage_by_spell` ist je
/// Zauber und existiert samt Leser in `ReplayResult` — kein Feld, das nur fuer
/// diese Probe gebaut wurde.
fn schaden_des_wandlers(permille: u32) -> u32 {
    let mut fight = Fight::new(FightConfig {
        talent_control_effect_permille: permille,
        duration_ms: Some(20_000),
        ..FightConfig::default()
    });
    let ergebnis = fight.run(wandel_takt);
    ergebnis.damage_by_spell[wandler()]
}

#[test]
fn der_weg_effect_wandelt_mehr_um() {
    let ohne = schaden_des_wandlers(0);
    let mit = schaden_des_wandlers(1000);

    assert!(
        ohne > 0,
        "der Wandler hat keinen Schaden gemacht — entweder kam er nicht zum \
         Zug oder seine Umwandlung erreicht die Schadensrechnung nicht; in \
         beiden Faellen misst der Vergleich unten zwei Nullen"
    );
    // Faktor 2 auf den Beitrag je Punkt. Dass es GENAU das Doppelte ist und
    // nicht nur mehr, trennt den Faktor von einem festen Zuschlag.
    assert_eq!(
        mit,
        ohne * 2,
        "der Weg effect wirkt nicht wie ein Faktor: {ohne} Schaden ohne \
         Talent, {mit} mit doppelter Wirkung"
    );
}

#[test]
fn der_weg_cost_senkt_den_preis_messbar() {
    // **Der Wirkungsfall, der lange gefehlt hat.** Die Kreuzprobe unten faengt
    // eine UMKEHRUNG (teurer statt billiger), nicht das Fehlen der Wirkung —
    // ihr `>=` laesst eine Null durch. Gemessen: Streicht man den Teiler
    // ersatzlos, blieb sie gruen.
    //
    // Sichtbar wird der gesenkte Preis am Schaden des Wandlers: Er wandelt je
    // PUNKT um, und wer weniger je Wurf zahlt, bringt mehr Punkte durch. Der
    // Zusammenhang ist mittelbar, aber er ist die Groesse, die der Spieler
    // spuert — im Sammel-Takt, wo nichts ausgegeben wird, zeigt sich nichts.
    let voll = schaden_des_wandlers_mit_preis(0);
    let halb = schaden_des_wandlers_mit_preis(1000);
    let drittel = schaden_des_wandlers_mit_preis(3000);

    assert!(
        voll > 0,
        "der Wandler hat keinen Schaden gemacht — der Vergleich maesse Nullen"
    );
    assert!(
        halb > voll && drittel > halb,
        "der Weg cost wirkt nicht: {voll} Schaden bei vollem Preis, {halb} bei \
         halbem, {drittel} bei einem Drittel — die Reihe muss steigen"
    );
}

/// Wie `schaden_des_wandlers`, aber der Talent-Weg ist der PREIS.
fn schaden_des_wandlers_mit_preis(permille: u32) -> u32 {
    let mut fight = Fight::new(FightConfig {
        talent_control_cost_permille: permille,
        duration_ms: Some(20_000),
        ..FightConfig::default()
    });
    let ergebnis = fight.run(wandel_takt);
    ergebnis.damage_by_spell[wandler()]
}

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
