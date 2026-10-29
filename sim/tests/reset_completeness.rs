//! Der zweite Lauf im Simulator (Issue #349, AC4 und AC7).
//!
//! WAS AUF DER SPIELSEITE GEMESSEN WURDE
//! -------------------------------------
//! `Combat.reset()` liess **sechsundzwanzig** Felder stehen — von den
//! Lebenspunkten ueber die Zeitleiste bis zum Todes-Zeitpunkt. Gefunden hat es
//! ein Waechter, der den zurueckgesetzten Kampf gegen einen frisch gebauten
//! vergleicht (`tests/unit/reset-completeness.test.ts`).
//!
//! WARUM ES HIER ANDERS GEPRUEFT WIRD — und was das NICHT abdeckt
//! --------------------------------------------------------------
//! **Rust hat keine Reflexion.** Es gibt keinen Weg, ueber die Felder von
//! `Fight` zu laufen, wie `Object.keys` es auf der Spielseite tut. Ein
//! Waechter, der eine Feldliste pflegt, waere genau das, was auf der anderen
//! Seite vermieden wurde: Wer ein Feld hinzufuegt und die Liste vergisst, hat
//! den Fehler gebaut, den die Liste verhindern sollte.
//!
//! Geprueft wird deshalb ueber das ERGEBNIS: Dieselbe Aufzeichnung, zweimal im
//! selben `Fight` gefahren, muss Takt fuer Takt dieselbe Pruefsummen-Folge
//! ergeben. Die Pruefsumme traegt siebenunddreissig Groessen; was in ihr steht,
//! ist damit bewacht.
//!
//! **Was sie NICHT traegt, bleibt hier ungeprueft** — `casts_cancelled` etwa,
//! oder `damage_by_spell`. Das ist eine ehrliche Grenze und keine
//! Vollstaendigkeit. Wer sie schliessen will, braucht entweder Reflexion oder
//! eine gepflegte Liste; beides hat einen Preis, und der ist hier hoeher als
//! der Gewinn: Die Zaehler ausserhalb der Pruefsumme beeinflussen keine
//! Rechnung, sonst staenden sie darin.

use serde_json::Value;

use rota_sim::checksum::record_fields;
use rota_sim::data::boss_by_id;
use rota_sim::replay::{first_divergence, Binding, Press, Recording};
use rota_sim::{Fight, FightConfig, TICK_MS};

const RECORDING_JSON: &str = include_str!("../../tests/parity/recordings/cooldown-precision.json");

fn recording() -> (Recording, &'static str) {
    let json: Value =
        serde_json::from_str(RECORDING_JSON).expect("cooldown-precision.json ist kein JSON");
    let bindings = json["bindings"]
        .as_array()
        .expect("bindings fehlen")
        .iter()
        .map(|entry| {
            let spell_id = entry["spellId"].as_str().expect("spellId fehlt");
            Binding {
                slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
                spell: rota_sim::data::index_of(spell_id)
                    .unwrap_or_else(|| panic!("Zauber {spell_id} unbekannt")),
            }
        })
        .collect();
    let presses = json["presses"]
        .as_array()
        .expect("presses fehlen")
        .iter()
        .map(|entry| Press {
            tick: u32::try_from(entry["tick"].as_u64().expect("tick")).expect("tick zu gross"),
            slot: u8::try_from(entry["slot"].as_u64().expect("slot")).expect("slot > 255"),
        })
        .collect();
    (
        Recording {
            name: "zweiter-lauf".to_owned(),
            seed: 0,
            ticks: u32::try_from(json["ticks"].as_u64().expect("ticks")).expect("ticks"),
            bindings,
            presses,
        },
        "first-crucible",
    )
}

fn fight_gegen(boss_id: &str, ticks: u32) -> Fight {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    Fight::new(FightConfig {
        seed: 0,
        boss,
        duration_ms: Some(ticks * TICK_MS),
        ..FightConfig::default()
    })
}

#[test]
fn dieselbe_aufzeichnung_zweimal_ergibt_dieselbe_folge() {
    // **AC7.** Weicht der zweite Durchgang ab, hat `reset` etwas stehen
    // lassen, das in die Rechnung eingeht — und zwar genau dort, wo es die
    // Bitgleichheit mit dem Spiel betrifft.
    let (rec, boss_id) = recording();
    let mut fight = fight_gegen(boss_id, rec.ticks);

    let erster = fight.replay(&rec, true);
    fight.reset(rec.seed);
    let zweiter = fight.replay(&rec, true);

    assert!(
        !erster.checksums.is_empty(),
        "ohne Pruefsummen misst der Fall nichts"
    );
    assert_eq!(
        erster.checksums.len(),
        zweiter.checksums.len(),
        "die zwei Durchgaenge rechnen verschieden viele Takte"
    );

    if let Some(d) = first_divergence(&erster.checksums, &zweiter.checksums, TICK_MS) {
        panic!(
            "Der ZWEITE Durchgang laeuft anders als der erste:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             Lauf 1: {}\n  Lauf 2: {}\n  \
             `Fight::reset` hat einen Zustand stehen lassen, der in die \
             Rechnung eingeht.",
            d.tick, d.time_ms, d.expected, d.actual,
        );
    }
}

#[test]
fn der_schluss_stand_stimmt_in_beiden_durchgaengen() {
    // Die Summen daneben: Sie stehen nicht in der Pruefsumme dieses Takts,
    // sondern fassen den ganzen Lauf zusammen.
    let (rec, boss_id) = recording();
    let mut fight = fight_gegen(boss_id, rec.ticks);

    let erster = fight.replay(&rec, false);
    fight.reset(rec.seed);
    let zweiter = fight.replay(&rec, false);

    assert!(erster.damage > 0, "ohne Schaden misst der Fall nichts");
    assert_eq!(erster.damage, zweiter.damage, "verschiedener Schaden");
    assert_eq!(
        erster.killed_at_ms, zweiter.killed_at_ms,
        "verschiedene Zeit"
    );
    assert_eq!(erster.accepted, zweiter.accepted, "verschieden angenommen");
    assert_eq!(erster.rejected, zweiter.rejected, "verschieden abgelehnt");
    assert_eq!(
        erster.final_checksum, zweiter.final_checksum,
        "verschiedene Schluss-Pruefsumme"
    );
    assert_eq!(
        erster.casts, zweiter.casts,
        "verschieden viele Wirkungen je Zauber"
    );
}

#[test]
fn ein_frischer_fight_rechnet_wie_der_zurueckgesetzte() {
    // **AC4.** Der Vergleich, den die Spielseite ueber die Felder fuehrt, hier
    // ueber das Ergebnis: Ein zurueckgesetzter `Fight` muss rechnen wie ein
    // neu gebauter. Tut er es nicht, traegt er Zustand aus dem ersten Lauf.
    let (rec, boss_id) = recording();

    let mut wiederverwendet = fight_gegen(boss_id, rec.ticks);
    let _ = wiederverwendet.replay(&rec, false);
    wiederverwendet.reset(rec.seed);
    let nach_reset = wiederverwendet.replay(&rec, true);

    let mut frisch = fight_gegen(boss_id, rec.ticks);
    let frischer_lauf = frisch.replay(&rec, true);

    if let Some(d) = first_divergence(&frischer_lauf.checksums, &nach_reset.checksums, TICK_MS) {
        panic!(
            "Ein zurueckgesetzter Fight rechnet anders als ein frischer:\n  \
             erster abweichender Takt: {} (Kampfzeit {} ms)\n  \
             frisch:          {}\n  nach reset:      {}",
            d.tick, d.time_ms, d.expected, d.actual,
        );
    }
    assert_eq!(frischer_lauf.damage, nach_reset.damage);
}

#[test]
fn der_waechter_nennt_das_feld_und_nicht_nur_die_abweichung() {
    // **Eine abweichende Pruefsumme sagt, DASS etwas steht — nicht WAS.**
    // `record_fields` zeichnet die Pruefsumme mit ihren Namen auf; damit
    // nennt dieser Fall die Stelle, statt den Suchenden auf 118 Werte
    // loszulassen.
    //
    // Genau so wurde die Luecke gefunden, die dieser Zweig behebt:
    //
    //     ABWEICHUNG [115] playerDamage: nach reset 4240, frisch 0
    //
    // Das ist das SPIEGELBILD des Falls aus #318: Dort raeumt der Simulator
    // die Zersetzungs-Stapel und das Spiel nicht, hier raeumte das Spiel
    // `playerDamage` und der Simulator nicht. Beide Male unsichtbar, weil kein
    // Paritaets-Weg zweimal faehrt.
    let (rec, boss_id) = recording();
    let mut wiederverwendet = fight_gegen(boss_id, rec.ticks);
    let _ = wiederverwendet.replay(&rec, false);
    wiederverwendet.reset(rec.seed);
    let frisch = fight_gegen(boss_id, rec.ticks);

    let nach_reset = record_fields(|| {
        let _ = wiederverwendet.checksum_at(0);
    });
    let neu = record_fields(|| {
        let _ = frisch.checksum_at(0);
    });

    assert!(
        nach_reset.len() > 100,
        "zu wenige Felder aufgezeichnet ({}) — die Aufzeichnung greift nicht",
        nach_reset.len()
    );
    assert_eq!(
        nach_reset.len(),
        neu.len(),
        "verschieden viele Felder in der Pruefsumme"
    );

    let mut abweichungen: Vec<String> = Vec::new();
    for (i, ((name_a, wert_a), (name_b, wert_b))) in nach_reset.iter().zip(neu.iter()).enumerate() {
        assert_eq!(name_a, name_b, "die Feldfolge selbst weicht ab, bei {i}");
        if (wert_a - wert_b).abs() > f64::EPSILON {
            abweichungen.push(format!(
                "[{i}] {name_a}: nach reset {wert_a}, frisch {wert_b}"
            ));
        }
    }
    assert!(
        abweichungen.is_empty(),
        "`Fight::reset` laesst Zustand stehen:\n  {}",
        abweichungen.join("\n  ")
    );
}

/// Der Feldname zu einer Stelle in der `Debug`-Auflistung.
///
/// Sie sieht aus wie `Fight { time: 0, damage: 4240, stack_gains: Cell { value: 40 }, ... }`.
///
/// **Gesucht ist der Name auf der OBERSTEN Ebene**, nicht der naechstgelegene.
/// Mein erster Entwurf ging rueckwaerts bis zum letzten `name:` und meldete
/// bei einem Unterschied in `stack_gains` das Feld `value` — der Name aus dem
/// `Cell` darin. Ein Waechter, der "value" meldet, schickt den Suchenden ins
/// Leere; es gibt keine Stelle dieses Namens.
///
/// Deshalb ueber die Klammertiefe: Gemerkt wird der letzte Name, der auf
/// Tiefe 1 stand.
fn feld_bei(text: &str, stelle: usize) -> String {
    let mut tiefe = 0_i32;
    let mut letzter = String::new();
    let mut wort = String::new();
    for (i, c) in text.char_indices() {
        if i >= stelle {
            break;
        }
        match c {
            '{' | '[' | '(' => {
                tiefe += 1;
                wort.clear();
            }
            '}' | ']' | ')' => {
                tiefe -= 1;
                wort.clear();
            }
            ',' => wort.clear(),
            ':' => {
                if tiefe == 1 {
                    letzter = wort.trim().to_owned();
                }
                wort.clear();
            }
            _ => wort.push(c),
        }
    }
    if letzter.is_empty() {
        "(unbekannt)".to_owned()
    } else {
        letzter
    }
}

#[test]
fn kein_einziges_feld_ueberlebt_das_zuruecksetzen() {
    // **Der Waechter, der MITWAECHST** — das Gegenstueck zu `Object.keys` auf
    // der Spielseite. Rust hat keine Reflexion, aber `#[derive(Debug)]` laesst
    // den Uebersetzer die Auflistung schreiben: Sie enthaelt jedes Feld, auch
    // das, das morgen dazukommt.
    //
    // Der Fall daneben (`der_waechter_nennt_das_feld_...`) prueft nur, was in
    // der PRUEFSUMME steht. Dieser hier prueft alles.
    let (rec, boss_id) = recording();
    let mut wiederverwendet = fight_gegen(boss_id, rec.ticks);
    let _ = wiederverwendet.replay(&rec, false);
    wiederverwendet.reset(rec.seed);
    let frisch = fight_gegen(boss_id, rec.ticks);

    let nach_reset = format!("{wiederverwendet:?}");
    let neu = format!("{frisch:?}");

    // Ohne diese Zusicherung liefe der Vergleich ueber zwei leere
    // Zeichenketten, und der Fall waere gruen, ohne etwas gesehen zu haben.
    assert!(
        nach_reset.len() > 1000,
        "die Debug-Auflistung ist zu kurz ({} Zeichen) — sie greift nicht",
        nach_reset.len()
    );

    if nach_reset != neu {
        let stelle = nach_reset
            .char_indices()
            .zip(neu.char_indices())
            .find(|((_, a), (_, b))| a != b)
            .map_or(0, |((i, _), _)| i);
        let feld = feld_bei(&nach_reset, stelle);
        let fenster = |s: &str| {
            let von = stelle.saturating_sub(20);
            let bis = (stelle + 60).min(s.len());
            s[von..bis].to_owned()
        };
        panic!(
            "`Fight::reset` laesst Zustand stehen.\n  \
             Feld:       {feld}\n  \
             nach reset: ...{}...\n  \
             frisch:     ...{}...",
            fenster(&nach_reset),
            fenster(&neu),
        );
    }
}

#[test]
fn zaehler_ausserhalb_der_pruefsumme_gehen_ebenfalls_zurueck() {
    // **Die Grenze des Fall darueber, und sie ist der Grund fuer diesen hier.**
    // Die Pruefsumme traegt nur, was die Rechnung beeinflusst. `stack_gains`
    // ist ein Zaehler fuer die Pruefung — er stand nach dem Reset auf 40,
    // waehrend ein frischer Fight bei 0 beginnt, und keine Pruefsumme haette
    // das gemeldet.
    //
    // Vollstaendig ist auch dieser Fall nicht: Er nennt die Zaehler, die es
    // heute gibt. Rust hat keine Reflexion — was hier nicht steht, ist
    // unbewacht, und das gehoert gesagt statt behauptet.
    let (rec, boss_id) = recording();
    let mut wiederverwendet = fight_gegen(boss_id, rec.ticks);
    let _ = wiederverwendet.replay(&rec, false);
    assert!(
        wiederverwendet.stack_gain_count() > 0,
        "ohne Vorrats-Gewinne misst der Fall nichts"
    );
    wiederverwendet.reset(rec.seed);

    let frisch = fight_gegen(boss_id, rec.ticks);
    assert_eq!(
        wiederverwendet.stack_gain_count(),
        frisch.stack_gain_count(),
        "stack_gain_count laeuft im zweiten Lauf weiter"
    );
    assert_eq!(
        wiederverwendet.sublimation_use_count(),
        frisch.sublimation_use_count(),
        "sublimation_use_count laeuft im zweiten Lauf weiter"
    );
    assert_eq!(
        wiederverwendet.decay_stacks(),
        frisch.decay_stacks(),
        "decay_stacks laeuft im zweiten Lauf weiter"
    );
}

#[test]
fn der_lauf_hinterlaesst_ueberhaupt_zustand() {
    // **Die Zusicherung, die den drei Faellen oben ihren Wert gibt** —
    // dieselbe Frage wie auf der Spielseite: Faehrt mein Lauf ueberhaupt
    // etwas, das Zustand hinterlaesst? Ein Durchgang, der nichts tut,
    // wiederholt sich trivial.
    let (rec, boss_id) = recording();
    let mut fight = fight_gegen(boss_id, rec.ticks);
    let lauf = fight.replay(&rec, false);

    assert!(lauf.damage > 0, "kein Schaden");
    assert!(lauf.accepted > 0, "keine angenommenen Druecke");
    assert!(
        lauf.casts.iter().filter(|&&c| c > 0).count() >= 2,
        "weniger als zwei Zauber gewirkt"
    );
}

// ---------------------------------------------------------------------------
// Die Schluesseltalente (#419, #418 AC6)
// ---------------------------------------------------------------------------
//
// **Die Aufzeichnung oben findet ihren Zustand nicht.** `cooldown-precision`
// faehrt ohne Talente, wirft kein ROT und legt keine Schicht, die ein Schlag
// bricht. `shield_last_amount` und `shield_refund_used` stehen nach ihr auf
// ihrem Ruhewert — und ein Feld, das nie falsch war, sieht nach dem Reset
// richtig aus.
//
// **Umskillen gibt es auf dieser Seite nicht als eigenen Weg.** Die Talente
// kommen ueber `FightConfig` und werden in `Fight::new` uebernommen; ein
// `Fight` mit anderer Verteilung ist ein neuer `Fight`, und der beginnt
// leer. Die Frage aus #418 AC6 ist hier deshalb genau die nach `reset`.

/// Die spielbare Verteilung mit beiden Schluesseltalenten, die Zustand anlegen
/// — als Promille, wie der Simulator sie bekommt.
fn mit_beiden_schluesseln(ticks: u32) -> Fight {
    Fight::new(FightConfig {
        seed: 0,
        boss: boss_by_id("test-onslaught").expect("test-onslaught fehlt"),
        duration_ms: Some(ticks * TICK_MS),
        // putrefaction-trunk 1 + coagulation-trunk 1, je 20 Promille Schaden.
        talent_permille: 40,
        talent_decay_ward_permille: 120,
        talent_shield_amount_permille: 100,
        decay_echo: true,
        shield_refund: true,
        ..FightConfig::default()
    })
}

/// Eine feste Runde: Belegung `zauber[i]` auf Platz `i`, Druck `(takt, platz)`.
fn runde(name: &str, ticks: u32, zauber: &[&str], druecke: &[(u32, u8)]) -> Recording {
    Recording {
        name: name.to_owned(),
        seed: 0,
        ticks,
        bindings: zauber
            .iter()
            .enumerate()
            .map(|(slot, id)| Binding {
                slot: u8::try_from(slot).expect("zu viele Plaetze"),
                spell: rota_sim::data::index_of(id).unwrap_or_else(|| panic!("{id} fehlt")),
            })
            .collect(),
        presses: druecke
            .iter()
            .map(|&(tick, slot)| Press { tick, slot })
            .collect(),
    }
}

/// CRUST bei 0 ms, ROT bei 1500 ms, bis 6000 ms. Der erste Schlag (5000 ms)
/// bricht die Schicht, die Nachlage liegt, ROT laeuft nach.
fn schluessel_runde() -> Recording {
    runde(
        "schluessel-reset",
        601,
        &["crust", "rot"],
        &[(0, 0), (150, 1)],
    )
}

#[test]
fn schluesseltalente_der_lauf_bewegt_ihren_zustand() {
    // **Die Zusicherung vor der Messung** — gelesen aus derselben
    // `Debug`-Auflistung, die der Waechter vergleicht.
    let rec = schluessel_runde();
    let mut fight = mit_beiden_schluesseln(rec.ticks);
    let _ = fight.replay(&rec, false);
    let auflistung = format!("{fight:?}");
    assert!(fight.shield() > 0, "die Nachlage liegt nicht");
    assert!(
        auflistung.contains("shield_refund_used: true"),
        "die Nachlage ist nicht verbraucht"
    );
    assert!(
        !auflistung.contains("shield_last_amount: 0,"),
        "keine Dicke gemerkt"
    );
}

#[test]
fn schluesseltalente_kein_feld_ueberlebt_das_zuruecksetzen() {
    let rec = schluessel_runde();
    let mut wiederverwendet = mit_beiden_schluesseln(rec.ticks);
    let _ = wiederverwendet.replay(&rec, false);
    wiederverwendet.reset(rec.seed);
    let frisch = mit_beiden_schluesseln(rec.ticks);

    let nach_reset = format!("{wiederverwendet:?}");
    let neu = format!("{frisch:?}");
    // Nicht nur das erste abweichende Feld: Wer behebt, soll alle auf einmal
    // sehen. Verglichen wird Feld fuer Feld auf der obersten Ebene.
    let felder = |s: &str| -> Vec<String> {
        let mut tiefe = 0_i32;
        let mut aktuell = String::new();
        let mut liste = Vec::new();
        for c in s.chars() {
            match c {
                '{' | '[' | '(' => tiefe += 1,
                '}' | ']' | ')' => tiefe -= 1,
                _ => {}
            }
            if c == ',' && tiefe == 1 {
                liste.push(aktuell.trim().to_owned());
                aktuell.clear();
            } else {
                aktuell.push(c);
            }
        }
        liste.push(aktuell.trim().to_owned());
        liste
    };
    let (a, b) = (felder(&nach_reset), felder(&neu));
    assert_eq!(a.len(), b.len(), "verschieden viele Felder");
    assert!(
        a.len() > 100,
        "zu wenige Felder ({}) — die Zerlegung greift nicht",
        a.len()
    );
    let abweichend: Vec<String> = a
        .iter()
        .zip(b.iter())
        .filter(|(x, y)| x != y)
        .map(|(x, y)| {
            // Der Name des Felds und die Stelle, an der es abweicht — bei
            // `factors` stehen sonst acht gleiche Werte vor dem einen anderen.
            let name = x.split(':').next().unwrap_or("?");
            let stelle = x
                .chars()
                .zip(y.chars())
                .position(|(p, q)| p != q)
                .unwrap_or(0);
            let fenster = |s: &str| {
                s.chars()
                    .skip(stelle.saturating_sub(30))
                    .take(70)
                    .collect::<String>()
            };
            format!(
                "{name}: nach reset ...{}... | frisch ...{}...",
                fenster(x),
                fenster(y)
            )
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "`Fight::reset` laesst Zustand der Schluesseltalente stehen:\n  {}",
        abweichend.join("\n  ")
    );
}

#[test]
fn schluesseltalente_retry_schenkt_keine_nachlage_aus_dem_vorigen_kampf() {
    // **Was ein stehengebliebenes Feld kostet — gemessen**, Spiegelbild des
    // gleichnamigen Falls in `tests/unit/reset-completeness.test.ts`.
    //
    // Eine Schicht aus Umwandlung (TEMPER) wird nicht gelegt, sie waechst: Sie
    // schreibt weder `shield_last_amount` noch `shield_refund_used`. Bricht
    // sie, gibt die Nachlage die Haelfte dessen, was dort STEHT — im frischen
    // Kampf 0, also nichts. Stand dort noch die CRUST aus dem vorigen Durchgang,
    // legt der Bruch eine halbe CRUST nach, die niemand gewirkt hat.
    //
    // Fuer den Simulator ist das kein Randfall: Die Balance-Laeufe setzen
    // denselben `Fight` fuer jede Runde zurueck, statt ihn neu zu bauen.
    let erster = runde("crust-ohne-bruch", 201, &["crust"], &[(0, 0)]);
    let zweiter = runde(
        "temper-dann-bruch",
        601,
        &["temper", "kindle"],
        &[(0, 0), (150, 1), (300, 1), (450, 1)],
    );
    let nachlage = |ticks: u32| {
        Fight::new(FightConfig {
            seed: 0,
            boss: boss_by_id("test-onslaught").expect("test-onslaught fehlt"),
            duration_ms: Some(ticks * TICK_MS),
            talent_permille: 20,
            talent_shield_amount_permille: 100,
            shield_refund: true,
            ..FightConfig::default()
        })
    };

    let mut wiederholt = nachlage(zweiter.ticks);
    let vorher = wiederholt.replay(&erster, false);
    assert!(
        wiederholt.shield() > 0,
        "CRUST liegt nicht — der erste Durchgang misst nichts"
    );
    assert_eq!(
        vorher.damage_taken, 0,
        "der erste Durchgang darf die Schicht nicht brechen"
    );
    wiederholt.reset(0);
    let _ = wiederholt.replay(&zweiter, false);

    let mut frisch = nachlage(zweiter.ticks);
    let _ = frisch.replay(&zweiter, false);

    assert!(
        frisch.converted() > 0,
        "TEMPER wandelt nichts um — der Fall misst nichts"
    );
    assert!(
        frisch.shield_absorbed() > 0,
        "der Schlag trifft die Schicht nicht"
    );
    assert_eq!(
        wiederholt.shield(),
        frisch.shield(),
        "der zweite Durchgang erbt eine Nachlage aus dem ersten"
    );
}

/// Ein zurückgesetzter Kampf rechnet an **jedem** Boss wie ein frischer
/// (gefunden bei der Messung zu #917).
///
/// Die Fälle darüber fahren eine Aufzeichnung ohne Verhängnis-Uhr. `reset`
/// ließ deshalb `next_doom_charge_at` (#561) unbemerkt stehen: Wer die Uhr
/// im ersten Durchgang senkte, bekam im zweiten eine Ladung zur alten Zeit
/// zurück. Gemessen an boss-1-2 mit `purge if doom >= 40` vor `kindle`,
/// nach demselben Vorlauf: 1700 Schaden im wiederverwendeten Kampf, 3300
/// frisch. `balance::run_range` setzt seinen Kampf je Startwert so zurück.
///
/// Geprüft wird über das Ergebnis, an allen Bossen, nach jedem Vorlauf.
#[test]
fn ein_zurueckgesetzter_kampf_rechnet_an_jedem_boss_wie_ein_frischer() {
    use rota_sim::apl::{parse, AplName};
    use rota_sim::data::BOSSES;

    let listen = [
        ("senkt-die-uhr", "purge if doom >= 40\nkindle\n"),
        ("step3-good", include_str!("../apl/step3-good.apl")),
        ("e797-mit-quench", include_str!("../apl/e797-mit-quench.apl")),
        (
            "step553-alle-bedingt",
            include_str!("../apl/step553-alle-bedingt.apl"),
        ),
    ];
    let apls: Vec<_> = listen
        .iter()
        .map(|(name, text)| (*name, parse(AplName(name), text).expect("Liste")))
        .collect();
    let purge = rota_sim::data::index_of("purge").expect("purge fehlt");
    let mut gesenkt = 0;
    for boss in &BOSSES {
        let config = FightConfig {
            seed: 5,
            duration_ms: Some(60_000),
            stop_on_kill: false,
            log_capacity: 0,
            boss,
            ..FightConfig::default()
        };
        for (vorlauf_name, vorlauf) in &apls {
            for (name, apl) in &apls {
                let mut wiederholt = Fight::new(config);
                let erster = wiederholt.run_apl(vorlauf);
                gesenkt += erster.casts[purge];
                wiederholt.reset(5);
                let zweiter = wiederholt.run_apl(apl);
                let frisch = Fight::new(config).run_apl(apl);
                assert_eq!(
                    zweiter, frisch,
                    "{}: {name} nach {vorlauf_name} rechnet anders als frisch",
                    boss.id
                );
            }
        }
    }
    // Der Träger: Ohne eine Senkung im Vorlauf prüfte der Fall die Ladung nie.
    assert!(gesenkt > 0, "kein Vorlauf hat die Uhr gesenkt");
}
