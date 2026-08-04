//! Eingehender Schaden, Spielertod und Unterbrechen (Issue #110).
//!
//! **Zwei Laeufe gegen denselben Boss, die sich in genau einer Sache
//! unterscheiden:** Im einen wirkt der Spieler QUENCH, im anderen nicht. Alles
//! andere ist gleich — derselbe Startwert, dieselbe Rotation, dieselben
//! Zeitpunkte.
//!
//! Das Ergebnis trennt sich trotzdem vollstaendig:
//!
//! | | ohne QUENCH | mit QUENCH |
//! | --- | --- | --- |
//! | Boss-Zauber durchgekommen | 3 | 0 |
//! | erlittener Schaden | 5200 | 400 |
//! | Spieler | **tot** | lebt |
//!
//! Damit tragen zwei Laeufe drei Acceptance Criteria, und zwar so, dass **eine**
//! Ursache das ganze Ergebnis erklaert. Ein Prueffall, in dem sich zwei Dinge
//! gleichzeitig aendern, koennte das nicht.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, PLAYER};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

/// Der aufgezeichnete Kampf, wie das Spiel ihn geschrieben hat.
const JSON: &str = include_str!("../../tests/parity/recordings/player-damage.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("player-damage.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

/// Der Lauf mit oder ohne Unterbrechen, wie er in der Datei steht.
fn run(json: &Value, mit_quench: bool) -> &Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["withInterrupt"].as_bool() == Some(mit_quench))
        .unwrap_or_else(|| panic!("kein Lauf mit withInterrupt={mit_quench}"))
}

/// Spielt einen der beiden Laeufe im Simulator nach.
fn replay(json: &Value, mit_quench: bool) -> (rota_sim::ReplayResult, &Value) {
    let lauf = run(json, mit_quench);
    let boss_id = json["bossId"].as_str().expect("bossId fehlt");
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));

    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: index_of(spell_id).unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = lauf["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();
    let ticks = u32::try_from(lauf["ticks"].as_u64().expect("ticks")).expect("ticks zu gross");

    let recording = Recording {
        name: json["name"].as_str().expect("name").to_owned(),
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
    let ergebnis = fight.replay(&recording, true);
    (ergebnis, lauf)
}

#[test]
fn ac1_ein_boss_mit_schaden_bringt_den_spieler_herunter() {
    let json = data();
    let (ohne, _) = replay(&json, false);
    let (mit, _) = replay(&json, true);

    assert!(
        ohne.damage_taken > 0 && mit.damage_taken > 0,
        "beide Laeufe muessen Schaden am Spieler zeigen"
    );
    assert!(
        *ohne.player_hp.last().expect("Spur leer") < PLAYER.vessel,
        "die Lebenspunkte muessen gefallen sein"
    );

    // **Die Gegenprobe: ein Boss OHNE Schadens-Eintraege laesst sie unberuehrt.**
    // Ohne sie belegte der Test nur, dass irgendetwas Lebenspunkte abzieht —
    // nicht, dass es die Zeitleiste dieses Bosses ist.
    let erster = boss_by_id("first-crucible").expect("first-crucible");
    assert!(
        erster.timeline.is_empty(),
        "der erste Boss darf keine Zeitleiste haben, sonst prueft die \
         Gegenprobe nichts"
    );
    let mut friedlich = Fight::new(FightConfig {
        boss: erster,
        ..FightConfig::default()
    });
    fn nur_kindle(f: &Fight) -> Option<usize> {
        let kindle = index_of("kindle").expect("kindle");
        if f.is_ready(kindle) {
            Some(kindle)
        } else {
            None
        }
    }
    friedlich.run(nur_kindle as fn(&Fight) -> Option<usize>);
    assert_eq!(friedlich.player_hp(), PLAYER.vessel);
    assert_eq!(friedlich.damage_taken(), 0);
    assert!(!friedlich.player_dead());
}

#[test]
fn ac2_der_spieler_stirbt_und_der_versuch_endet() {
    let json = data();
    let (ohne, lauf) = replay(&json, false);
    let erwartet = &lauf["expected"];

    assert_eq!(
        ohne.died_at_ms.map(u64::from),
        erwartet["diedAtMs"]
            .as_i64()
            .and_then(|v| u64::try_from(v).ok()),
        "Spiel und Simulator lassen den Spieler zu verschiedenen Zeitpunkten sterben"
    );
    assert!(ohne.died_at_ms.is_some(), "in diesem Lauf muss er sterben");
    assert_eq!(*ohne.player_hp.last().expect("Spur leer"), 0);

    // **Der Versuch endet — er laeuft nicht weiter.** Das ist der Kern von AC2:
    // Die Pruefsummen-Folge bricht beim Tod ab und geht nicht bis zum Ende der
    // aufgezeichneten Takte.
    let ticks = lauf["ticks"].as_u64().expect("ticks");
    assert!(
        (ohne.checksums.len() as u64) < ticks,
        "die Folge laeuft ueber den Tod hinaus weiter: {} Takte von {ticks}",
        ohne.checksums.len()
    );
    let tod_takt = ohne.died_at_ms.expect("gestorben") / TICK_MS;
    assert_eq!(
        ohne.checksums.len() as u32,
        tod_takt + 1,
        "der letzte Takt muss der des Todes sein"
    );
}

#[test]
fn ac3_ein_unterbrochener_zauber_richtet_nichts_an() {
    let json = data();
    let (ohne, lauf_ohne) = replay(&json, false);
    let (mit, lauf_mit) = replay(&json, true);

    // Nicht unterbrochen: voller Schaden, dreimal.
    assert_eq!(ohne.interrupts_missed, 3);
    assert_eq!(ohne.interrupts_landed, 0);

    // Unterbrochen: gar nichts, dreimal.
    assert_eq!(mit.interrupts_landed, 3);
    assert_eq!(mit.interrupts_missed, 0);

    // **Der Unterschied ist genau der Schaden der drei Zauber.** Von Hand
    // nachgerechnet und nicht bloss abgelesen: 3 x 2000 roh, davon 20 % WARD
    // ab, macht 4800.
    let unterschied = ohne.damage_taken - mit.damage_taken;
    assert_eq!(unterschied, 4800);
    assert_eq!(unterschied, PLAYER.incoming(2000) * 3);

    // Und beide Seiten sind sich darueber einig.
    for (ergebnis, lauf) in [(&ohne, lauf_ohne), (&mit, lauf_mit)] {
        let erwartet = &lauf["expected"];
        assert_eq!(
            u64::from(ergebnis.damage_taken),
            erwartet["damageTaken"].as_u64().expect("damageTaken"),
        );
        assert_eq!(
            u64::from(ergebnis.interrupts_landed),
            erwartet["interruptsLanded"].as_u64().expect("landed"),
        );
        assert_eq!(
            u64::from(ergebnis.interrupts_missed),
            erwartet["interruptsMissed"].as_u64().expect("missed"),
        );
    }
}

#[test]
fn ac4_unterbrechen_loest_keine_wartezeit_aus_jeder_andere_zauber_schon() {
    // **Die Falle, vor der das Issue ausdruecklich warnt:** „Unterbrechen loest
    // keinen GCD aus" ist trivial gruen, wenn im Prueflauf gar nichts einen GCD
    // ausloest. Dieser eine Lauf zeigt deshalb BEIDES.
    //
    // Drei Druecke, alle im selben Takt:
    //
    //   1. KINDLE   kommt durch und setzt die Wartezeit
    //   2. KINDLE   wird abgelehnt — die Wartezeit wirkt also wirklich
    //   3. QUENCH   kommt trotzdem durch — er ist die Ausnahme
    //
    // Faellt Nummer 2 weg, ist der Test wertlos: Dann koennte die Wartezeit
    // ueberhaupt nicht greifen, und Nummer 3 bewiese nichts.
    let boss = boss_by_id("test-onslaught").expect("test-onslaught");
    let kindle = index_of("kindle").expect("kindle");
    let quench = index_of("quench").expect("quench");

    let recording = Recording {
        name: "gcd-probe".to_owned(),
        seed: 0,
        ticks: 10,
        bindings: vec![
            Binding {
                slot: 0,
                spell: kindle,
            },
            Binding {
                slot: 3,
                spell: quench,
            },
        ],
        presses: vec![
            Press { tick: 0, slot: 0 },
            Press { tick: 0, slot: 0 },
            Press { tick: 0, slot: 3 },
        ],
    };
    let mut fight = Fight::new(FightConfig {
        boss,
        ..FightConfig::default()
    });
    let ergebnis = fight.replay(&recording, false);

    assert_eq!(
        ergebnis.casts[kindle], 1,
        "der zweite KINDLE-Druck muss an der Wartezeit scheitern — sonst \
         ist dieser Test wertlos"
    );
    assert_eq!(
        ergebnis.rejected, 1,
        "genau ein Druck muss abgelehnt worden sein"
    );
    assert_eq!(
        ergebnis.casts[quench], 1,
        "QUENCH muss trotz laufender Wartezeit durchkommen — er ist die \
         einzige Ausnahme im Spiel (GAME-DESIGN 6.6)"
    );

    // Und die Gegenprobe zur Aufteilung: Genau ein Zauber ist ausgenommen.
    let ohne_gcd = rota_sim::data::SPELLS
        .iter()
        .filter(|s| !s.triggers_gcd)
        .count();
    assert_eq!(ohne_gcd, 1);
}

#[test]
fn ac6_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    for mit_quench in [false, true] {
        let (ergebnis, lauf) = replay(&json, mit_quench);
        let erwartet: Vec<u32> = lauf["checksums"]
            .as_array()
            .expect("checksums fehlen")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
            .collect();

        assert!(!erwartet.is_empty());
        if let Some(d) = first_divergence(&erwartet, &ergebnis.checksums, TICK_MS) {
            panic!(
                "Lauf mit_quench={mit_quench} laeuft auseinander:\n  \
                 erster abweichender Takt: {} ({} ms)\n  \
                 Spiel: {}   Simulator: {}\n  \
                 gleiche Takte davor: {}\n  \
                 Spieler-HP dort: {:?}",
                d.tick,
                d.time_ms,
                d.expected,
                d.actual,
                d.matched_ticks,
                ergebnis.player_hp.get(d.tick as usize),
            );
        }
        assert_eq!(ergebnis.checksums.len(), erwartet.len());

        // Und die Lebenspunkt-Spur eigens: Die Pruefsumme sagt nur, DASS etwas
        // abweicht.
        let hp_erwartet: Vec<u32> = lauf["playerHpPerTick"]
            .as_array()
            .expect("playerHpPerTick fehlt")
            .iter()
            .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
            .collect();
        assert_eq!(ergebnis.player_hp, hp_erwartet);
    }
}

#[test]
fn ac5_ein_neuer_boss_mit_schaden_entsteht_ohne_programmcode() {
    // Wie in #96, nur mit den Feldern aus diesem Issue: Der Boss
    // `test-onslaught` ist allein durch eine Datendatei entstanden.
    let boss = boss_by_id("test-onslaught").expect(
        "Der Boss aus data/bosses/test-onslaught.json wurde nicht gefunden. \
         Liest build.rs das Verzeichnis noch?",
    );
    assert_eq!(boss.timeline.len(), 4);
    let zauber = boss
        .timeline
        .iter()
        .filter(|e| e.kind == rota_sim::data::TimelineKind::InterruptibleCast)
        .count();
    assert_eq!(zauber, 3, "drei unterbrechbare Zauber");
    assert!(
        boss.timeline.iter().any(|e| e.damage > 0),
        "ein Boss ohne Schaden pruefte hier nichts"
    );
    // Die Anzeigenamen sind englisch (Design-Gesetz 5) und kommen aus der Datei.
    assert!(boss.timeline.iter().any(|e| e.name == "Cascade"));
}
