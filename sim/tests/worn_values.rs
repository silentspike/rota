//! Die Ausruestungswerte kommen im Kampf an (GAME-DESIGN 6.7, Issue #177).
//!
//! # Warum jede Wirkung ihren eigenen Prueffall bekommt
//!
//! Die Auflage zu diesem Issue nennt den Grund und den Praezedenzfall: In #100
//! lief STALL durch die Pruefsumme, hatte aber keinen eigenen Nachweis — ein
//! gemeinsamer Prueffall ueber mehrere Wirkungen bleibt gruen, wenn nur **eine**
//! von ihnen wirkt.
//!
//! Hier waeren das vier Stellen, an denen es einzeln schiefgehen kann:
//!
//! | Wirkung                  | Eigener Prueffall                          |
//! | ------------------------ | ------------------------------------------ |
//! | Abklingzeiten            | `tempo_verkuerzt_abklingzeiten`            |
//! | DoT-Tick-Abstaende       | `tempo_verdichtet_dot_ticks`               |
//! | Kritrate (PURITY)        | `purity_erhoeht_die_kritrate`              |
//! | Kritschaden (YIELD)      | `yield_erhoeht_den_kritschaden`            |
//!
//! # Was hier NICHT geprueft wird, und warum
//!
//! **Die globale Wartezeit.** 6.7 fuehrt sie als Wirkort, 6.2 schliesst sie
//! ausdruecklich aus ("Der GCD sinkt ausschliesslich ueber den Talentbaum, nie
//! ueber Ausruestung", Festlegung vom 2026-07-29). Der Widerspruch ist gemeldet
//! und nicht von hier aus zu entscheiden. Bis die Antwort da ist, wirkt Tempo
//! auf die Wirkorte, in denen **beide** Kapitel uebereinstimmen.

use rota_sim::data::{KINDLE, PYRE, SMOULDER, SPELLS};
use rota_sim::gear::WornValues;
use rota_sim::haste::hasted_ms;
use rota_sim::rotation;
use rota_sim::{Fight, FightConfig, RunResult};

/// Ein Durchgang ueber die volle Kampfzeit mit den angegebenen Werten.
///
/// **`stop_on_kill: false`** — sonst faellt der Boss bei hohem Tempo frueher,
/// und ein kuerzerer Kampf sieht aus wie weniger Wirkung. `STEP1-SPEC` § 4
/// warnt genau davor.
fn lauf_mit(worn: WornValues) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        worn,
        ..FightConfig::default()
    })
    .run(rotation::good)
}

/// Derselbe Durchgang ohne jede Ausruestung — der Vergleichspunkt.
fn lauf_ohne() -> RunResult {
    lauf_mit(WornValues::none())
}

// ---------------------------------------------------------------------------
// AC7 — ohne getragene Ausruestung aendert sich nichts
// ---------------------------------------------------------------------------

#[test]
fn ohne_ausruestung_bleibt_die_referenz_auf_ihrem_wert() {
    // **Die Abnahme-Bedingung des Issues**, und sie steht bewusst zuerst.
    // Bewegt sich diese Zahl, ist etwas anderes passiert als beabsichtigt —
    // dann ist der Umbau nicht bruchfrei einfuehrbar, was er sein soll.
    let r = Fight::new(FightConfig {
        worn: WornValues::none(),
        ..FightConfig::default()
    })
    .run(rotation::good);

    assert_eq!(r.damage, 4240, "Schaden gesamt");
    assert_eq!(r.killed_at_ms, Some(40_510), "Sieg-Zeitpunkt");
    assert_eq!(
        r.random_draws, 0,
        "Ziehungen — ohne PURITY faellt kein Wurf"
    );
}

#[test]
fn ohne_ausruestung_faellt_kein_einziger_wurf() {
    // **Der Punkt, den man leicht uebersieht.** Ein Kritwurf bei einer Chance
    // von null waere rechnerisch folgenlos — und trotzdem sichtbar: Der
    // Ziehungszaehler geht in die Zustands-Pruefsumme ein (`checksum_at`).
    // Jeder Takt meldete dann eine Abweichung gegen das Spiel.
    //
    // Getragen wird die Zusicherung von `RandomStream::chance`, das bei 0
    // kurzschliesst. Diese Pruefung haelt fest, dass der Kampf sich darauf auch
    // wirklich verlaesst.
    let ohne = lauf_ohne();
    assert_eq!(ohne.random_draws, 0);

    // Und der Gegenfall: MIT PURITY wird gezogen. Ohne ihn bliebe die Zeile
    // darueber auch dann gruen, wenn der Kampf ueberhaupt nie wuerfelt.
    let mit = lauf_mit(WornValues {
        purity: 500,
        ..WornValues::none()
    });
    assert!(
        mit.random_draws > 0,
        "mit PURITY muss gezogen werden, sonst wirkt der Wert nicht"
    );
}

#[test]
fn ohne_ausruestung_laeuft_jede_zeit_unveraendert() {
    // Nicht nur das Endergebnis, sondern die Zeiten selbst: Wenn eine
    // Abklingzeit sich verschoebe und der Schaden zufaellig gleich bliebe,
    // faenge die Pruefung oben es nicht.
    for index in [KINDLE, SMOULDER, PYRE] {
        let spell = &SPELLS[index];
        assert_eq!(
            hasted_ms(spell.cooldown_ms, 0),
            spell.cooldown_ms,
            "Abklingzeit von {}",
            spell.id
        );
        if let Some(dot) = spell.dot {
            assert_eq!(
                hasted_ms(dot.tick_interval_ms, 0),
                dot.tick_interval_ms,
                "Tick-Abstand von {}",
                spell.id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC2 — zwei Wirkorte, zwei Nachweise
// ---------------------------------------------------------------------------

/// Eine Rotation, die **nur PYRE** wirkt, sobald er bereit ist.
///
/// Gebraucht fuer den Nachweis zu AC2: In `rotation::good` ist die Abklingzeit
/// gar nicht der Engpass. Gemessen am 2026-08-04 ueber die vollen 60 s:
///
/// ```text
/// +  0 Promille: PYRE-Wirkungen 5, Leerlauf 0 ms
/// +200 Promille: PYRE-Wirkungen 5, Leerlauf 0 ms
/// +500 Promille: PYRE-Wirkungen 5, Leerlauf 0 ms
/// ```
///
/// **Der Spieler handelt in jedem Wartezeit-Fenster** — die Zahl der Aktionen
/// haengt an der globalen Wartezeit, nicht an den Abklingzeiten. Eine kuerzere
/// Abklingzeit aendert erst dann etwas, wenn ein Zauber sonst NICHT bereit
/// gewesen waere. Mit dieser Rotation ist genau das der Fall.
///
/// Dass die Zahl der Aktionen am GCD haengt, ist auch der Grund, warum der
/// blockierte AC1 mehr als eine Formalie ist: Solange Tempo die Wartezeit nicht
/// beruehrt, verschiebt es die Rotation nur ueber die Abklingzeiten.
fn nur_pyre(fight: &Fight) -> Option<usize> {
    if fight.is_ready(PYRE) {
        Some(PYRE)
    } else {
        None
    }
}

#[test]
fn tempo_verkuerzt_abklingzeiten() {
    // **Der erste der beiden Wirkorte aus AC2.** Geprueft an PYRE, weil sein
    // Wert (5000 ms) als einziger ueber der Wartezeit liegt.
    let pyre_cd = SPELLS[PYRE].cooldown_ms;
    assert_eq!(pyre_cd, 5000, "die Datenlage, auf der dieser Fall beruht");

    // 5000 / 1,2 = 4166,67 -> 4170 (auf den Takt aufgerundet).
    assert_eq!(hasted_ms(pyre_cd, 200), 4170);

    let ohne = Fight::new(FightConfig {
        stop_on_kill: false,
        worn: WornValues::none(),
        ..FightConfig::default()
    })
    .run(nur_pyre);
    let mit = Fight::new(FightConfig {
        stop_on_kill: false,
        worn: WornValues {
            catalysis: 200,
            ..WornValues::none()
        },
        ..FightConfig::default()
    })
    .run(nur_pyre);

    assert!(
        mit.casts[PYRE] > ohne.casts[PYRE],
        "kuerzere Abklingzeit muss mehr PYRE-Wirkungen ergeben: {} gegen {}",
        mit.casts[PYRE],
        ohne.casts[PYRE]
    );

    // **Und die Zahl folgt aus der Abklingzeit, nicht aus irgendetwas.** Ohne
    // diese Bindung bliebe der Fall auch gruen, wenn die Wirkungen aus einem
    // ganz anderen Grund stiegen — etwa weil versehentlich die Wartezeit
    // mitskalierte, die hier ausdruecklich nicht mitskalieren soll.
    //
    // Gewirkt wird bei 0 und dann alle `abstand` ms, solange der Zeitpunkt
    // **echt vor** dem Kampfende liegt. Das `- 1` ist genau diese Strenge: Bei
    // einem Abstand von 5000 ms faellt der dreizehnte Zeitpunkt auf 60 000 und
    // damit auf das Kampfende — er zaehlt nicht mehr.
    //
    // Die erste Fassung rechnete `dauer / abstand + 1` und war damit bei jedem
    // Abstand falsch, der die Dauer glatt teilt. Aufgefallen ist es sofort, weil
    // der Fall ueber DREI Tempo-Werte laeuft: Bei 4170 ms stimmte die Formel,
    // bei 5000 ms nicht. Mit nur einem Wert waere es durchgegangen.
    let dauer = 60_000;
    for permille in [0, 200, 500] {
        let abstand = hasted_ms(pyre_cd, permille);
        let erwartet = (dauer - 1) / abstand + 1;
        let r = Fight::new(FightConfig {
            stop_on_kill: false,
            worn: WornValues {
                catalysis: permille,
                ..WornValues::none()
            },
            ..FightConfig::default()
        })
        .run(nur_pyre);
        assert_eq!(
            r.casts[PYRE], erwartet,
            "+{permille} Promille: Abstand {abstand} ms in {dauer} ms"
        );
    }

    // Die beiden Zahlen ausgeschrieben, damit im Diff sichtbar wird, was sich
    // bewegt: 60 000 / 5000 + 1 = 13 waere es ohne den Kampfschluss — gemessen
    // sind es 12, weil der Kampf bei genau 60 000 endet und der dreizehnte
    // Zeitpunkt nicht mehr hineinfaellt.
    assert_eq!(ohne.casts[PYRE], 12, "ohne Tempo");
    assert_eq!(mit.casts[PYRE], 15, "mit +20 % Tempo");
}

#[test]
fn tempo_verdichtet_dot_ticks() {
    // **Der zweite Wirkort aus AC2**, und er braucht seinen eigenen Lauf: Ein
    // gemeinsamer Prueffall mit der Abklingzeit bliebe gruen, wenn nur eine
    // der beiden Stellen verdrahtet waere.
    let dot = SPELLS[SMOULDER].dot.expect("SMOULDER traegt einen DoT");
    assert_eq!(dot.tick_interval_ms, 2000, "die Datenlage");
    assert_eq!(dot.duration_ms, 12_000, "die Datenlage");

    // 2000 / 1,2 = 1666,67 -> 1670.
    assert_eq!(hasted_ms(dot.tick_interval_ms, 200), 1670);

    let ohne = lauf_ohne();
    let mit = lauf_mit(WornValues {
        catalysis: 200,
        ..WornValues::none()
    });

    assert!(
        mit.dot_tick_counts[SMOULDER] > ohne.dot_tick_counts[SMOULDER],
        "dichtere Ticks muessen mehr Ticks ergeben: {} gegen {}",
        mit.dot_tick_counts[SMOULDER],
        ohne.dot_tick_counts[SMOULDER]
    );
}

// ---------------------------------------------------------------------------
// AC4 — die Dauer bleibt, die Ticks werden mehr
// ---------------------------------------------------------------------------

#[test]
fn der_dot_behaelt_seine_dauer_und_bekommt_mehr_ticks() {
    let dot = SPELLS[SMOULDER].dot.expect("SMOULDER traegt einen DoT");

    // Je Anwendung: 12 000 / 2000 = 6 Ticks ohne Tempo, 12 000 / 1670 = 7 mit.
    for permille in [0, 200, 500, 1000] {
        let abstand = hasted_ms(dot.tick_interval_ms, permille);
        let ticks = dot.duration_ms / abstand;
        assert!(
            ticks * abstand <= dot.duration_ms,
            "+{permille}: {ticks} Ticks à {abstand} ms passen nicht in {} ms",
            dot.duration_ms
        );
        // **Kein anteiliger letzter Tick** — einer mehr passte nicht.
        assert!((ticks + 1) * abstand > dot.duration_ms);
    }

    // Und im laufenden Kampf, nicht nur in der Rechnung: mehr Ticks je Anwendung.
    let ohne = lauf_ohne();
    let mit = lauf_mit(WornValues {
        catalysis: 200,
        ..WornValues::none()
    });
    let je_anwendung_ohne = ohne.dot_tick_counts[SMOULDER] / ohne.casts[SMOULDER];
    let je_anwendung_mit = mit.dot_tick_counts[SMOULDER] / mit.casts[SMOULDER];
    assert!(
        je_anwendung_mit > je_anwendung_ohne,
        "je Anwendung: {je_anwendung_mit} gegen {je_anwendung_ohne}"
    );
}

// ---------------------------------------------------------------------------
// AC5 — zwei Werte, zwei Nachweise
// ---------------------------------------------------------------------------

#[test]
fn purity_erhoeht_die_kritrate() {
    // **Der erste der beiden Werte aus AC5.** Gemessen an den Ziehungen und am
    // Schaden: Mehr PURITY heisst mehr Treffer, die den Kritfaktor bekommen.
    let ohne = lauf_ohne();
    let halb = lauf_mit(WornValues {
        purity: 500,
        ..WornValues::none()
    });
    let voll = lauf_mit(WornValues {
        purity: 1000,
        ..WornValues::none()
    });

    assert!(
        halb.damage > ohne.damage,
        "PURITY 50 % muss mehr Schaden ergeben: {} gegen {}",
        halb.damage,
        ohne.damage
    );
    assert!(
        voll.damage > halb.damage,
        "PURITY 100 % muss mehr ergeben als 50 %: {} gegen {}",
        voll.damage,
        halb.damage
    );

    // **Am Deckel ist Krit kein Zufall mehr** (6.7). Bei 100 % kritet jeder
    // Treffer, also ist der Schaden genau der 1,5-fache — und es faellt kein
    // einziger Wurf mehr, weil `chance` bei 1,0 kurzschliesst.
    assert_eq!(
        voll.random_draws, 0,
        "bei 100 % PURITY steht das Ergebnis fest, ein Wurf waere Verschwendung"
    );
    assert!(
        halb.random_draws > 0,
        "bei 50 % muss gewuerfelt werden — sonst waere der Wert wirkungslos"
    );
}

#[test]
fn yield_erhoeht_den_kritschaden() {
    // **Der zweite Wert aus AC5**, mit eigenem Lauf. Damit der Vergleich
    // ausschliesslich YIELD misst, kritet in beiden Laeufen JEDER Treffer
    // (PURITY 100 %) — sonst mischte sich der Zufall der Kritrate hinein.
    let nur_purity = lauf_mit(WornValues {
        purity: 1000,
        ..WornValues::none()
    });
    let mit_yield = lauf_mit(WornValues {
        purity: 1000,
        yield_bonus: 500,
        ..WornValues::none()
    });

    assert!(
        mit_yield.damage > nur_purity.damage,
        "YIELD muss den Kritschaden erhoehen: {} gegen {}",
        mit_yield.damage,
        nur_purity.damage
    );

    // **Der Gegenfall, ohne den die Zeile darueber wenig sagt:** YIELD ohne
    // PURITY ist wirkungslos. Wer nie kritet, hat von hoeherem Kritschaden
    // nichts — genau die Abwaegung, die 6.7 dem Spieler zumuten will.
    let ohne = lauf_ohne();
    let yield_allein = lauf_mit(WornValues {
        yield_bonus: 500,
        ..WornValues::none()
    });
    assert_eq!(
        yield_allein.damage, ohne.damage,
        "YIELD ohne PURITY darf nichts bewirken"
    );
}

#[test]
fn der_kritschaden_betraegt_ohne_yield_genau_das_anderthalbfache() {
    // Die Basis aus 6.7 in einer Zahl, die man nachrechnen kann: 1,5 ist in
    // IEEE754 exakt darstellbar, hier ist keine Naeherung noetig.
    let voll = WornValues {
        purity: 1000,
        ..WornValues::none()
    };
    assert_eq!(voll.crit_factor(), 1.5);
    assert_eq!(voll.crit_chance(), 1.0);

    let mit_yield = WornValues {
        purity: 1000,
        yield_bonus: 500,
        ..WornValues::none()
    };
    assert_eq!(mit_yield.crit_factor(), 2.0);

    // Und die Chance ueber dem Deckel bleibt eins, statt ueber eins zu gehen.
    let ueber = WornValues {
        purity: 5000,
        ..WornValues::none()
    };
    assert_eq!(ueber.crit_chance(), 1.0);
}

// ---------------------------------------------------------------------------
// Der Gegenfall zum ganzen Issue
// ---------------------------------------------------------------------------

#[test]
fn jeder_der_drei_werte_bewegt_etwas() {
    // **Ohne diesen Fall koennte einer der drei Werte stillschweigend gar nicht
    // verdrahtet sein**, und alle Pruefungen oben liefen trotzdem durch —
    // jede von ihnen prueft nur ihren eigenen Wert.
    let ohne = lauf_ohne();

    let nur_tempo = lauf_mit(WornValues {
        catalysis: 200,
        ..WornValues::none()
    });
    let nur_purity = lauf_mit(WornValues {
        purity: 1000,
        ..WornValues::none()
    });
    // YIELD braucht PURITY, um sichtbar zu werden — siehe der Gegenfall oben.
    let nur_yield = lauf_mit(WornValues {
        purity: 1000,
        yield_bonus: 1000,
        ..WornValues::none()
    });

    assert_ne!(nur_tempo.damage, ohne.damage, "CATALYSIS bewegt nichts");
    assert_ne!(nur_purity.damage, ohne.damage, "PURITY bewegt nichts");
    assert_ne!(nur_yield.damage, nur_purity.damage, "YIELD bewegt nichts");
}
