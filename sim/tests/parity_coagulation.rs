//! Die drei Wege der Koagulation (GAME-DESIGN 4.1, Issue #79).
//!
//! **Vier Laeufe gegen denselben Boss, die sich in genau einer Sache
//! unterscheiden:** welchen Verteidigungszauber der Spieler wirkt — keinen,
//! CRUST, SALVE oder TEMPER. Dasselbe Schadensprofil, dieselbe
//! Angriffsrotation, derselbe Startwert.
//!
//! | Weg | Leben | erlitten | Mechanismus | ausgeteilt |
//! | --- | --- | --- | --- | --- |
//! | ohne | **tot** bei 24 s | 5200 | — | 1600 |
//! | CRUST | 3400 | 1600 | 3600 abgefangen | 1500 |
//! | SALVE | 2800 | 5200 | 3000 geheilt | 1700 |
//! | TEMPER | 75 | 4925 | 300 umgewandelt | 1800 |
//!
//! **Das ist der Kern von AC1.** Drei Wege, die alle „irgendwie ueberleben
//! helfen", waeren kein Nachweis; drei Wege, die bei gleichem eingehenden
//! Schaden zu drei verschiedenen Endzustaenden fuehren, sind einer.
//!
//! Der Lauf ohne Verteidigung ist die Messlatte: Er zeigt, dass der Boss
//! ueberhaupt toedlich ist. Ohne ihn koennten alle drei Wege wirkungslos sein,
//! ohne dass es auffaellt.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/coagulation.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("coagulation.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn lauf<'a>(json: &'a Value, weg: &str) -> &'a Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["way"].as_str() == Some(weg))
        .unwrap_or_else(|| panic!("kein Lauf fuer Weg {weg}"))
}

/// Spielt einen der vier Laeufe im Simulator nach.
///
/// Die Druecke stehen in der Datei, statt hier aus der Regel nachgebaut zu
/// werden. Eine nachgebaute Regel, die minimal abweicht, ergaebe eine
/// Abweichung, die wie ein Rechenfehler aussieht — und man suchte sie in der
/// Kampflogik statt in der Rotation.
fn replay(json: &Value, weg: &str) -> rota_sim::ReplayResult {
    let eintrag = lauf(json, weg);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    let ticks = u32::try_from(eintrag["ticks"].as_u64().expect("ticks")).expect("passt");

    let bindings = eintrag["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|b| {
            let id = b["spellId"].as_str().expect("spellId");
            Binding {
                slot: u8::try_from(b["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(id).unwrap_or_else(|| panic!("Zauber {id} unbekannt")),
            }
        })
        .collect();
    let presses = eintrag["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|p| Press {
            tick: u32::try_from(p["tick"].as_u64().expect("tick")).expect("zu gross"),
            slot: u8::try_from(p["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();

    let recording = Recording {
        name: format!("coagulation-{weg}"),
        seed: 0,
        ticks,
        bindings,
        presses,
    };
    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    });
    fight.replay(&recording, true)
}

fn erwartet(json: &Value, weg: &str, feld: &str) -> u64 {
    lauf(json, weg)["expected"][feld]
        .as_u64()
        .unwrap_or_else(|| panic!("{weg}/{feld} fehlt"))
}

#[test]
fn ac1_die_drei_wege_fuehren_zu_verschiedenem_verhalten() {
    let json = data();

    // **Die Messlatte zuerst:** Ohne Verteidigung stirbt der Spieler. Ohne
    // diesen Nachweis koennten alle drei Wege wirkungslos sein.
    assert_eq!(erwartet(&json, "none", "playerHp"), 0);
    assert_eq!(
        lauf(&json, "none")["expected"]["diedAtMs"].as_i64(),
        Some(24_000)
    );

    // Alle drei retten ihn — und zwar auf drei verschiedene Arten.
    let mut endstaende = Vec::new();
    for weg in ["crust", "salve", "temper"] {
        let hp = erwartet(&json, weg, "playerHp");
        assert!(hp > 0, "Weg {weg} rettet den Spieler nicht");
        assert_eq!(
            lauf(&json, weg)["expected"]["diedAtMs"].as_i64(),
            Some(-1),
            "Weg {weg} laesst den Spieler sterben"
        );
        endstaende.push(hp);
    }

    // **Verschieden, nicht bloss ueberlebend.** Drei gleiche Endstaende waeren
    // drei Namen fuer dieselbe Mechanik.
    let einzeln: std::collections::BTreeSet<u64> = endstaende.iter().copied().collect();
    assert_eq!(einzeln.len(), 3, "zwei Wege enden gleich: {endstaende:?}");

    // Und jeder Weg wirkt ueber SEINEN Mechanismus, nicht ueber einen fremden.
    assert!(erwartet(&json, "crust", "shieldAbsorbed") > 0);
    assert_eq!(erwartet(&json, "crust", "healed"), 0);
    assert_eq!(erwartet(&json, "salve", "healed"), 3000);
    assert_eq!(erwartet(&json, "salve", "shieldAbsorbed"), 0);
    assert!(erwartet(&json, "temper", "converted") > 0);
    assert_eq!(erwartet(&json, "temper", "healed"), 0);
}

#[test]
fn ac1_ueberleben_kostet_schaden_und_zeit() {
    // Die Abwaegung, die die Schule ausmacht (GAME-DESIGN A.3): Jeder
    // Verteidigungszauber belegt einen GCD, der sonst einem Angriff gehoert
    // haette. Wer am besten ueberlebt, teilt am wenigsten aus.
    let json = data();
    let ohne = erwartet(&json, "none", "damageDealt");
    let crust = erwartet(&json, "crust", "damageDealt");

    assert!(
        crust < ohne,
        "CRUST muss Schaden kosten: {crust} gegen {ohne} ohne Verteidigung"
    );
    // Der Lauf mit dem meisten Leben teilt am wenigsten aus — sonst waere die
    // Schule geschenkt statt eine Abwaegung.
    let bester = ["crust", "salve", "temper"]
        .into_iter()
        .max_by_key(|w| erwartet(&json, w, "playerHp"))
        .expect("drei Wege");
    assert_eq!(bester, "crust");
    let schwaechster = ["crust", "salve", "temper"]
        .into_iter()
        .min_by_key(|w| erwartet(&json, w, "damageDealt"))
        .expect("drei Wege");
    assert_eq!(schwaechster, "crust");
}

#[test]
fn ac2_die_schutzschicht_nimmt_auf_und_stapelt_nach_den_regeln() {
    let json = data();
    let ergebnis = replay(&json, "crust");

    assert_eq!(
        u64::from(ergebnis.shield_absorbed),
        erwartet(&json, "crust", "shieldAbsorbed"),
        "Spiel und Simulator fangen verschieden viel ab"
    );

    // **Die Obergrenze greift.** Ein vierter Druck auf eine volle Schicht
    // erneuert die Laufzeit, erhoeht aber den Betrag nicht — sonst waere
    // maxStacks wirkungslos.
    let crust = SPELLS[index_of("crust").expect("crust")];
    let schild = crust.shield.expect("CRUST hat eine Schutzschicht");
    let hoechster = ergebnis.shield.iter().copied().max().expect("Spur leer");
    assert!(
        hoechster <= schild.amount * schild.max_stacks,
        "die Schicht ist ueber die Obergrenze gewachsen: {hoechster} > {}",
        schild.amount * schild.max_stacks
    );
    assert!(hoechster > schild.amount, "sie hat nie gestapelt");
}

#[test]
fn ac3_ein_defensiv_zauber_loest_den_vollen_gcd_aus() {
    // **Die Falle, andersherum als in #110-AC4:** „Der Defensiv-Zauber loest
    // den GCD aus" ist trivial gruen, wenn man ihn nur einmal wirkt. Dieser
    // Lauf zeigt beides in EINEM Durchgang — der Zauber setzt die Wartezeit,
    // UND ein Angriff im selben Takt scheitert daran.
    let boss = boss_by_id("test-onslaught").expect("test-onslaught");
    let kindle = index_of("kindle").expect("kindle");
    let recording = Recording {
        name: "gcd-defence".to_owned(),
        seed: 0,
        ticks: 10,
        bindings: vec![
            Binding {
                slot: 0,
                spell: kindle,
            },
            Binding {
                slot: 1,
                spell: index_of("crust").expect("crust"),
            },
        ],
        presses: vec![
            Press { tick: 0, slot: 1 }, // CRUST kommt durch und setzt die Wartezeit
            Press { tick: 0, slot: 0 }, // KINDLE scheitert daran
        ],
    };
    let mut fight = Fight::new(FightConfig {
        boss,
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);

    assert_eq!(
        ergebnis.casts[index_of("crust").expect("crust")],
        1,
        "CRUST muss durchkommen"
    );
    assert_eq!(
        ergebnis.casts[kindle], 0,
        "KINDLE muss an der Wartezeit scheitern — sonst hat CRUST keine gesetzt"
    );
    assert_eq!(ergebnis.rejected, 1);

    // Und die Gegenprobe zur Aufteilung: Alle drei Wege loesen den GCD aus.
    // Genau EIN Zauber im Spiel ist ausgenommen (GAME-DESIGN 6.6).
    for weg in ["crust", "salve", "temper"] {
        let spell = SPELLS[index_of(weg).expect(weg)];
        assert!(
            spell.triggers_gcd,
            "{weg} muss die volle Wartezeit ausloesen (GAME-DESIGN A.3)"
        );
    }
}

#[test]
fn ac4_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    for weg in ["none", "crust", "salve", "temper"] {
        let ergebnis = replay(&json, weg);
        let soll: Vec<u32> = lauf(&json, weg)["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
            .collect();

        assert!(!soll.is_empty());
        if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
            panic!(
                "Weg {weg} laeuft auseinander:\n  \
                 erster abweichender Takt: {} ({} ms)\n  \
                 Spiel: {}   Simulator: {}\n  gleiche Takte davor: {}\n  \
                 Spieler-HP dort: {:?}, Schutz: {:?}",
                d.tick,
                d.time_ms,
                d.expected,
                d.actual,
                d.matched_ticks,
                ergebnis.player_hp.get(d.tick as usize),
                ergebnis.shield.get(d.tick as usize),
            );
        }
        assert_eq!(ergebnis.checksums.len(), soll.len());
        assert_eq!(
            u64::from(ergebnis.damage_taken),
            erwartet(&json, weg, "damageTaken")
        );
    }
}
