//! Die Fixierung greift in den Kampfverlauf ein (GAME-DESIGN 4.1, Issue #100).
//!
//! **Zwei Laeufe gegen denselben Boss, die sich in genau einer Sache
//! unterscheiden:** ob der Spieler seine Kontroll-Punkte ausgibt.
//!
//! | Ausgabe | ausgegeben | Phase | Schwelle | Zeitleiste greift bei |
//! | --- | --- | --- | --- | --- |
//! | nein | 0 | 1 | — | 4000, 12000, 19000 ms |
//! | ja | 15 | 0 | −10 | 4000, **15000**, 19000 ms |
//!
//! Zwei der drei Nachweise zeigen, dass sich etwas **nicht** aendert — und
//! genau die sind der eigentliche Gehalt:
//!
//! * Der **dritte** Schlag rueckt NICHT mit. Mit nur einem Schlag auf der
//!   Zeitleiste haetten beide STAY-Fassungen bestanden.
//! * Laeuft die Fixierung waehrend einer hoeheren Phase aus, **bleibt** die
//!   Phase. Ohne diesen Fall waere `check_phase` nur zufaellig richtig.

use serde_json::Value;

use rota_sim::data::{boss_by_id, index_of, SPELLS};
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const JSON: &str = include_str!("../../tests/parity/recordings/fixation.json");

fn data() -> Value {
    let value: Value = serde_json::from_str(JSON).expect("fixation.json ist kein JSON");
    assert_eq!(value["schema"].as_u64(), Some(1), "unbekannte Fassung");
    value
}

fn lauf(json: &Value, mit_ausgabe: bool) -> &Value {
    json["runs"]
        .as_array()
        .expect("runs fehlen")
        .iter()
        .find(|r| r["withSpending"].as_bool() == Some(mit_ausgabe))
        .unwrap_or_else(|| panic!("kein Lauf mit withSpending={mit_ausgabe}"))
}

fn replay(json: &Value, mit_ausgabe: bool) -> rota_sim::ReplayResult {
    let eintrag = lauf(json, mit_ausgabe);
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
        name: format!("fixation-{mit_ausgabe}"),
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

/// Die Zeitpunkte, zu denen die Zeitleiste vorgerueckt ist.
fn greift_bei(spur: &[u32]) -> Vec<u32> {
    spur.windows(2)
        .enumerate()
        .filter(|(_, paar)| paar[1] != paar[0])
        .map(|(tick, _)| (tick as u32 + 1) * TICK_MS)
        .collect()
}

fn zahlen(eintrag: &Value, feld: &str) -> Vec<u32> {
    eintrag[feld]
        .as_array()
        .unwrap_or_else(|| panic!("{feld} fehlt"))
        .iter()
        .map(|v| u32::try_from(v.as_u64().expect("Zahl")).expect("32 Bit"))
        .collect()
}

#[test]
fn ac1_der_boss_verhaelt_sich_messbar_anders() {
    // **Nicht: ein Zaehler laeuft hoch.** FIXATION ist die einzige Schule, die
    // den KAMPFVERLAUF beeinflusst — ein Prueffall, der nur zeigt, dass Punkte
    // entstehen, belegt das nicht.
    let json = data();
    let ohne = lauf(&json, false);
    let mit = lauf(&json, true);

    assert_eq!(ohne["expected"]["controlSpent"].as_u64(), Some(0));
    assert!(mit["expected"]["controlSpent"].as_u64().expect("spent") > 0);

    // Der Unterschied steht am BOSS, nicht am Spieler.
    let zeiten_ohne = greift_bei(&zahlen(ohne, "timelineIndexPerTick"));
    let zeiten_mit = greift_bei(&zahlen(mit, "timelineIndexPerTick"));
    assert_ne!(
        zeiten_ohne, zeiten_mit,
        "die Zeitleiste laeuft in beiden Laeufen gleich — dann wirkt die \
         Fixierung nicht auf den Kampfverlauf"
    );

    // Und die Phase ebenso: gleiche Rotation, andere Phase.
    assert_ne!(
        ohne["expected"]["finalPhase"].as_u64(),
        mit["expected"]["finalPhase"].as_u64()
    );
}

#[test]
fn stay_verschiebt_nur_den_einen_schlag() {
    // **Der Nachweis, den die Entscheidung vom 2026-08-04 verlangt.** Mit nur
    // einem Schlag auf der Zeitleiste haetten beide Fassungen bestanden — die
    // verworfene, die alle folgenden mitschiebt, waere unsichtbar geblieben.
    // **Gegen den SIMULATOR gemessen, nicht gegen die Datei.** Die Datei kommt
    // vom Spiel; sie zu lesen belegte nur, dass das Spiel es richtig macht.
    // Waere der Simulator falsch, schluege allein die Pruefsumme an — und dann
    // sucht man den Fehler an der Stelle, an der er sich zeigt, statt dort, wo
    // er sitzt.
    let json = data();
    let ohne = greift_bei(&replay(&json, false).timeline_index);
    let mit = greift_bei(&replay(&json, true).timeline_index);

    assert_eq!(ohne.len(), 3, "die Zeitleiste braucht drei Eintraege");
    assert_eq!(mit.len(), 3);

    // Der erste ist unberuehrt: Die Ausgabe kam danach.
    assert_eq!(mit[0], ohne[0]);
    // Der zweite rueckt um genau den Betrag aus den Daten.
    let stay = SPELLS[index_of("stay").expect("stay")];
    let betrag = stay.control.expect("STAY gibt Punkte aus").amount;
    assert_eq!(mit[1], ohne[1] + betrag);
    // **Und der dritte bleibt, wo er war.** Das ist der ganze Nachweis.
    assert_eq!(
        mit[2], ohne[2],
        "der dritte Schlag ist mitgerueckt — STAY verschiebt aber nur EINEN \
         (GAME-DESIGN 4.1, \"einen Schlag verzoegern\")"
    );
}

#[test]
fn anchor_haelt_die_phase_zurueck_und_nimmt_keinen_wechsel_zurueck() {
    let json = data();
    let ohne = lauf(&json, false);
    let mit = lauf(&json, true);

    // Bei gleichem Schaden steht der Boss ohne Ausgabe schon in Phase 1.
    assert_eq!(ohne["expected"]["finalPhase"].as_u64(), Some(1));
    assert_eq!(mit["expected"]["finalPhase"].as_u64(), Some(0));
    assert!(mit["expected"]["phaseHoldPercent"].as_u64().expect("hold") > 0);

    // **Die Phase faellt nie zurueck** (GAME-DESIGN 11.1, eskalierend). Das
    // gilt in beiden Laeufen und ueber jeden einzelnen Takt — nicht nur am
    // Ende. Eine gesenkte Schwelle verschiebt den Wechsel; sie nimmt ihn nicht
    // zurueck.
    for eintrag in [ohne, mit] {
        let spur = zahlen(eintrag, "phasePerTick");
        for paar in spur.windows(2) {
            assert!(
                paar[1] >= paar[0],
                "die Phase ist zurueckgefallen: {} nach {}",
                paar[1],
                paar[0]
            );
        }
    }
}

#[test]
fn die_phase_bleibt_wenn_die_fixierung_waehrend_einer_hoeheren_phase_auslaeuft() {
    // **Der Zusatz aus Nachricht 710.** Ohne diesen Fall waere die Zeile
    // `if neue > self.phase` in `check_phase` nur zufaellig richtig: Solange
    // die Schwelle nie sinkt, kann die Phase gar nicht zurueckfallen.
    //
    // Hier wird sie gesenkt, der Boss wechselt trotzdem, und dann wird die
    // Senkung aufgehoben. Faellt er zurueck, ist die Zusicherung verletzt.
    let boss = boss_by_id("test-warden").expect("test-warden");
    let bind = index_of("bind").expect("bind");
    let anchor = index_of("anchor").expect("anchor");

    let mut fight = Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(60_000),
        ..FightConfig::default()
    });

    // Genug Punkte sammeln, dann ANCHOR wirken, dann weiter Schaden machen,
    // bis der Boss trotz gesenkter Schwelle wechselt.
    let mut presses = Vec::new();
    for tick in 0..6000 {
        presses.push(Press {
            tick,
            slot: if tick == 500 { 1 } else { 0 },
        });
    }
    let recording = Recording {
        name: "anchor-haelt".to_owned(),
        seed: 0,
        ticks: 6000,
        bindings: vec![
            Binding {
                slot: 0,
                spell: bind,
            },
            Binding {
                slot: 1,
                spell: anchor,
            },
        ],
        presses,
    };
    let ergebnis = fight.replay(&recording, true);

    assert!(
        fight.phase() > 0,
        "der Boss muss die Phase gewechselt haben, sonst prueft dieser Fall nichts"
    );
    assert!(
        !ergebnis.checksums.is_empty(),
        "der Lauf muss Takte gerechnet haben"
    );
    // Der Kern: Die Phase ist gestiegen und nie wieder gefallen.
    assert_eq!(fight.phase(), 1);
}

#[test]
fn stall_dehnt_die_wirkzeit_und_vergroessert_das_unterbrechen_fenster() {
    // **Die Luecke aus der Fertigmeldung zu #100.** STALL war gebaut, aber im
    // Prueffall kam er nicht zum Zug: Die Rotation dort bevorzugt STAY und
    // ANCHOR, und der Boss-Zauber war vorbei, bevor STALL an die Reihe kam.
    //
    // „Die Wirkung laeuft durch die Pruefsumme" belegt nur, dass beide Seiten
    // dasselbe rechnen — nicht, dass sie das Richtige tun. Deshalb hier ein
    // eigener Nachweis.
    //
    // Der Boss wirkt von 4000 bis 7000 ms. BIND gibt 2 Punkte je 1500 ms, die
    // vier fuer STALL sind also ab etwa 3000 ms da.
    let boss = boss_by_id("test-warden").expect("test-warden");
    let bind = index_of("bind").expect("bind");
    let stall = index_of("stall").expect("stall");
    let stall_daten = SPELLS[stall].control.expect("STALL gibt Punkte aus");

    let bauen = |mit_stall: bool| {
        let mut presses = Vec::new();
        for tick in 0..800 {
            // Ab Takt 450 (4500 ms) laeuft der Boss-Zauber und die Punkte
            // reichen. Ein FENSTER, kein Zeitpunkt: Bei einem festen Takt
            // koennte die Wartezeit von BIND laufen, und ein gescheiterter
            // Druck verschoebe nichts — der Nachweis waere still weg.
            let im_fenster = mit_stall && (450..520).contains(&tick);
            presses.push(Press {
                tick,
                slot: if im_fenster { 1 } else { 0 },
            });
        }
        let recording = Recording {
            name: format!("stall-{mit_stall}"),
            seed: 0,
            // **Der Lauf endet zwischen beiden Trefferzeitpunkten.** Ohne
            // STALL trifft der Zauber bei 7000 ms, mit STALL bei 9000. Liefe
            // er bis 10000, traefe er in beiden Faellen — und der Unterschied
            // waere unsichtbar.
            ticks: 800,
            bindings: vec![
                Binding {
                    slot: 0,
                    spell: bind,
                },
                Binding {
                    slot: 1,
                    spell: stall,
                },
            ],
            presses,
        };
        let mut fight = Fight::new(FightConfig {
            seed: 0,
            boss,
            duration_ms: Some(8_000),
            ..FightConfig::default()
        });
        let ergebnis = fight.replay(&recording, true);
        (ergebnis, fight.damage_taken())
    };

    let (ohne, schaden_ohne) = bauen(false);
    let (mit, schaden_mit) = bauen(true);

    // **Die Ursache zuerst:** STALL muss ueberhaupt gewirkt worden sein. Eine
    // Ungleichung auf das Ergebnis faengt den Nullfall im ERGEBNIS — sie
    // faengt nicht, dass die Handlung ausgeblieben ist.
    assert!(
        mit.casts[stall] > 0,
        "STALL kam nicht zum Zug — dann prueft dieser Fall nichts"
    );
    assert_eq!(ohne.casts[stall], 0);

    // Ohne STALL trifft der Zauber und richtet Schaden an.
    assert!(
        schaden_ohne > 0,
        "der Boss-Zauber muss ohne Verlangsamung treffen"
    );

    // **Mit STALL trifft er spaeter** — innerhalb dieser acht Sekunden also
    // gar nicht mehr. Das ist die Wirkung: Die Wirkzeit dehnt sich, und das
    // Fenster zum Unterbrechen wird groesser.
    assert!(
        schaden_mit < schaden_ohne,
        "STALL hat den Zauber nicht verzoegert: {schaden_mit} gegen \
         {schaden_ohne} ohne Verlangsamung"
    );
    assert!(stall_daten.amount > 0);
}

#[test]
fn ac4_die_pruefsummen_stimmen_in_jedem_einzelnen_takt() {
    let json = data();
    for mit_ausgabe in [false, true] {
        let ergebnis = replay(&json, mit_ausgabe);
        let eintrag = lauf(&json, mit_ausgabe);
        let soll = zahlen(eintrag, "checksums");

        assert!(!soll.is_empty());
        if let Some(d) = first_divergence(&soll, &ergebnis.checksums, TICK_MS) {
            panic!(
                "Lauf mit_ausgabe={mit_ausgabe} laeuft auseinander:\n  \
                 erster abweichender Takt: {} ({} ms)\n  \
                 Spiel: {}   Simulator: {}\n  gleiche Takte davor: {}",
                d.tick, d.time_ms, d.expected, d.actual, d.matched_ticks
            );
        }
        assert_eq!(ergebnis.checksums.len(), soll.len());
    }
}
