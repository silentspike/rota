//! Die Deckel-Tabelle ist vollstaendig und ihre Deckel sind hergeleitet (#959).
//!
//! # Was hier NICHT geprueft werden kann
//!
//! AC4 — dass eine neue Wirkgroesse den **Bau** bricht — ist ein
//! Uebersetzungsfehler und aus einem Prueffall heraus nicht zeigbar: Gaebe es
//! die neue Groesse, liesse sich diese Datei nicht uebersetzen. Der Nachweis
//! steht im PR-Text als echter roter Lauf.
//!
//! Alles Uebrige ist zur Laufzeit sichtbar und steht hier.

use rota_sim::caps::{
    deckel_fuer, faktoren_aufgezaehlt, getragene_aufgezaehlt, pruefe, pruefe_getragene,
    AusnahmeGrund, Deckel, Traeger, Wirkgroesse, BROWSER_EXAKT_BIS, SCHADEN_MAX, VESSEL_MAX,
    WIRKGROESSEN,
};
use rota_sim::factors::NEUTRAL;
use rota_sim::gear::WornValues;

// ------------------------------------------------------------------- AC3

#[test]
fn jede_groesse_die_der_simulator_kennt_steht_in_der_tabelle() {
    // # Der Abgleich geht gegen die TYPEN, nicht gegen eine Liste
    //
    // `faktoren_aufgezaehlt` und `getragene_aufgezaehlt` destrukturieren ihre
    // Struktur ohne `..`. Was sie liefern, ist damit genau das, was der
    // Simulator kennt — und nicht das, was jemand aufgeschrieben hat.
    let mut fehlend: Vec<String> = Vec::new();
    let mut geprueft = 0usize;

    for (name, _) in faktoren_aufgezaehlt(&NEUTRAL) {
        geprueft += 1;
        if deckel_fuer(name, Traeger::Faktor).is_none() {
            fehlend.push(format!("Faktor {name}"));
        }
    }
    for (name, _) in getragene_aufgezaehlt(&WornValues::default()) {
        geprueft += 1;
        if deckel_fuer(name, Traeger::Getragen).is_none() {
            fehlend.push(format!("Getragen {name}"));
        }
    }

    assert!(
        geprueft > 0,
        "Keine Wirkgroesse aufgezaehlt — dann prueft dieser Fall nichts."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {geprueft} Wirkgroessen fehlen in der Deckel-Tabelle: {fehlend:?}. Eine \
         Groesse ohne Eintrag hat weder Deckel noch begruendete Ausnahme und erfuellt AC1 \
         nicht.",
        fehlend.len()
    );
}

#[test]
fn die_tabelle_enthaelt_nichts_was_der_simulator_nicht_kennt() {
    // Die Gegenrichtung: Ein Eintrag fuer eine Groesse, die es nicht mehr
    // gibt, ist eine gepflegte Liste, die in die andere Richtung altert. Er
    // wuerde nie auffallen — geprueft wird sonst nur, ob nichts FEHLT.
    let faktoren: Vec<&str> = faktoren_aufgezaehlt(&NEUTRAL)
        .iter()
        .map(|(n, _)| *n)
        .collect();
    let getragene: Vec<&str> = getragene_aufgezaehlt(&WornValues::default())
        .iter()
        .map(|(n, _)| *n)
        .collect();

    let mut verwaist: Vec<&str> = Vec::new();
    for g in WIRKGROESSEN {
        let bekannt = match g.traeger {
            Traeger::Faktor => faktoren.contains(&g.name),
            Traeger::Getragen => getragene.contains(&g.name),
        };
        if !bekannt {
            verwaist.push(g.name);
        }
    }
    assert!(
        verwaist.is_empty(),
        "Diese Eintraege der Tabelle gehoeren zu keiner Wirkgroesse, die der Simulator \
         kennt: {verwaist:?}"
    );
}

#[test]
fn kein_eintrag_steht_zweimal() {
    let mut gesehen: Vec<(&str, Traeger)> = Vec::new();
    for g in WIRKGROESSEN {
        let schluessel = (g.name, g.traeger);
        assert!(
            !gesehen.contains(&schluessel),
            "Die Groesse '{}' steht zweimal fuer denselben Traeger — dann gilt der erste \
             Deckel und der zweite ist tot, ohne dass es auffaellt.",
            g.name
        );
        gesehen.push(schluessel);
    }
    // Die Zahl in die Meldung, nicht in die Bedingung: Sie darf wachsen.
    assert!(
        !gesehen.is_empty(),
        "Die Tabelle ist leer — dann bestaetigt jede Pruefung darunter nichts."
    );
}

// ------------------------------------------------------------------- AC1/AC2

#[test]
fn jeder_eintrag_traegt_seine_herleitung() {
    // AC0 verlangt je Deckel die Herleitung, AC2 fuer jede Ausnahme eine
    // maschinell auffindbare Begruendung. Beides ist hier ein FELD, kein
    // Kommentar — sonst liesse es sich nicht pruefen.
    for g in WIRKGROESSEN {
        assert!(
            !g.deckel.engste_kette().is_empty(),
            "Die Groesse '{}' nennt keine engste Rechenkette. Eine Deckelzahl ohne \
             Herleitung ist von einer geratenen nicht zu unterscheiden.",
            g.name
        );
        assert!(
            g.deckel.engste_kette().len() > 40,
            "Die Herleitung von '{}' ist mit {} Zeichen zu knapp, um eine Kette zu benennen: \
             {:?}",
            g.name,
            g.deckel.engste_kette().len(),
            g.deckel.engste_kette()
        );
        match g.deckel {
            Deckel::Grenze { max, bitbreite, .. } => {
                assert!(max > 0, "Der Deckel von '{}' ist null.", g.name);
                assert!(
                    !bitbreite.is_empty(),
                    "Der Deckel von '{}' nennt keine Bitbreite.",
                    g.name
                );
            }
            Deckel::Ausnahme { grund, .. } => {
                // Der Grund ist eine Aufzaehlung, also aufzaehlbar und
                // pruefbar — genau das verlangt AC2.
                assert!(matches!(
                    grund,
                    AusnahmeGrund::NurNenner | AusnahmeGrund::AmEinsatzGedeckelt
                ));
            }
        }
    }
}

#[test]
fn es_gibt_beides_deckel_und_ausnahmen() {
    // Ohne diesen Fall koennte die Tabelle aus lauter Ausnahmen bestehen und
    // alle Pruefungen waeren gruen — eine Tabelle, die nichts begrenzt.
    let mit_grenze = WIRKGROESSEN
        .iter()
        .filter(|g| matches!(g.deckel, Deckel::Grenze { .. }))
        .count();
    let mit_ausnahme = WIRKGROESSEN
        .iter()
        .filter(|g| matches!(g.deckel, Deckel::Ausnahme { .. }))
        .count();
    assert!(
        mit_grenze > 0,
        "Kein einziger echter Deckel in der Tabelle — dann begrenzt sie nichts."
    );
    assert!(
        mit_ausnahme > 0,
        "Keine einzige Ausnahme — dann ist der Ausnahme-Zweig ungeprueft und koennte \
         beliebig kaputt sein."
    );
}

// ------------------------------------------------------------------- AC0

#[test]
fn die_deckel_liegen_so_hoch_wie_die_rechnung_traegt() {
    // AC0: nie enger als noetig. Die drei Zahlen, die aus der Bitbreite
    // folgen, werden hier gegen ihre Herleitung gerechnet — nicht abgelesen.
    assert_eq!(
        SCHADEN_MAX,
        u32::MAX as u64,
        "Der Schadensdeckel muss der volle u32-Bereich sein: `compute_damage` gibt u32 \
         zurueck, und `as u32` saettigt erst dort."
    );
    assert_eq!(
        VESSEL_MAX,
        (u32::MAX as u64) / 100,
        "Der Lebenspunkte-Deckel folgt aus `differenz * 100` in u32 — nicht aus einer \
         Spielentscheidung."
    );
    assert_eq!(
        BROWSER_EXAKT_BIS, 9_007_199_254_740_991,
        "Number.MAX_SAFE_INTEGER ist 2^53 - 1; darueber rechnet die Browser-Seite ganze \
         Zahlen nicht mehr exakt."
    );
    // Und die Gegenprobe zur Regel selbst: Kein Deckel darf UNTER dem liegen,
    // was ein u32 traegt, ohne dass die Herleitung eine engere Kette nennt.
    for g in WIRKGROESSEN {
        if let Deckel::Grenze { max, .. } = g.deckel {
            assert!(
                max >= VESSEL_MAX,
                "Der Deckel von '{}' liegt bei {max} und damit unter dem kleinsten \
                 hergeleiteten Wert ({VESSEL_MAX}). Ein Deckel, der enger sitzt als die \
                 Rechnung verlangt, ist eine Balance-Entscheidung im Gewand einer \
                 technischen.",
                g.name
            );
        }
    }
}

#[test]
fn der_deckel_von_yield_bonus_folgt_der_addition() {
    // Die einzige Groesse, deren Deckel NICHT aus der Schadenskette folgt,
    // sondern aus einer Addition davor. Wenn diese Herleitung stimmt, muss
    // Deckel + 1500 genau u32::MAX ergeben.
    let g =
        deckel_fuer("yield_bonus", Traeger::Getragen).expect("yield_bonus steht in der Tabelle");
    let max = g.deckel.max().expect("yield_bonus hat einen echten Deckel");
    assert_eq!(
        max + 1500,
        u32::MAX as u64,
        "CRIT_BASE_PERMILLE ist 1500 und wird in u32 addiert, bevor nach f64 gewandelt wird."
    );
}

// ------------------------------------------------------------------- AC0b

#[test]
fn ein_wert_ueber_dem_deckel_wird_gemeldet() {
    // AC0b: Die Meldung nennt Groesse, Deckel und den GEWOLLTEN Wert.
    let zu_gross = VESSEL_MAX + 1;
    let befund = pruefe("vessel", Traeger::Getragen, zu_gross)
        .expect("ein Wert ueber dem Deckel MUSS gemeldet werden");
    assert_eq!(befund.name, "vessel");
    assert_eq!(befund.deckel, VESSEL_MAX);
    assert_eq!(
        befund.gewollt, zu_gross,
        "Ohne den gewollten Wert liesse sich nicht beurteilen, ob der Deckel knapp oder \
         weit verfehlt wurde."
    );
    assert!(befund.ueberschuss() > 1.0);
}

#[test]
fn ein_wert_am_deckel_wird_nicht_gemeldet() {
    // Der Gegenpol: Ohne ihn koennte `pruefe` immer melden, und der Fall
    // darueber waere aus dem falschen Grund gruen.
    assert!(
        pruefe("vessel", Traeger::Getragen, VESSEL_MAX).is_none(),
        "Der Deckel selbst ist noch zulaessig — sonst waere die Grenze um eins zu eng."
    );
    assert!(pruefe("vessel", Traeger::Getragen, 0).is_none());
}

#[test]
fn eine_ausnahme_meldet_nie() {
    // Das ist die Aussage einer Ausnahme: Sie haelt jeden Wert.
    assert!(
        pruefe("catalysis", Traeger::Getragen, u64::MAX).is_none(),
        "catalysis steht allein im Nenner — kein Wert kann dort ueberlaufen."
    );
    assert!(
        pruefe("purity", Traeger::Getragen, u64::MAX).is_none(),
        "purity ist an der Wirkstelle bei 1000 gedeckelt — groessere Werte sind wirkungslos."
    );
}

#[test]
fn ein_kuenstlich_zu_kleiner_deckel_loest_die_meldung_aus() {
    // Die Gegenprobe, die die Verification strategy verlangt: „Eine Meldung,
    // die nie erschienen ist, ist nicht geprueft."
    //
    // Hier wird nicht die Tabelle mutiert, sondern ein Deckel von Hand
    // gebaut — das prueft dieselbe Entscheidungslogik ohne globalen Zustand.
    let winzig = Deckel::Grenze {
        max: 1,
        bitbreite: "kuenstlich, nur fuer diesen Prueffall",
        engste_kette: "kein echter Rechenweg — dieser Deckel existiert nur, um zu zeigen, \
                       dass die Schranke ueberhaupt greifen kann",
    };
    assert!(winzig.haelt(1), "Der Deckelwert selbst muss halten.");
    assert!(
        !winzig.haelt(2),
        "Ein Wert ueber dem Deckel MUSS als Verletzung erkannt werden — sonst greift die \
         Schranke nie und AC0b waere unpruefbar."
    );
    assert_eq!(winzig.max(), Some(1));

    let ausnahme = Deckel::Ausnahme {
        grund: AusnahmeGrund::NurNenner,
        engste_kette: "kein echter Rechenweg — nur fuer diesen Prueffall",
    };
    assert!(ausnahme.haelt(u64::MAX));
    assert_eq!(ausnahme.max(), None);
}

#[test]
fn ein_ganzer_satz_getragener_werte_wird_geprueft() {
    let ohne = WornValues::default();
    assert!(
        pruefe_getragene(&ohne).is_empty(),
        "Ein leerer Satz darf nichts melden."
    );

    let zu_gross = WornValues {
        vessel: u32::MAX,
        ..Default::default()
    };
    let befunde = pruefe_getragene(&zu_gross);
    assert_eq!(
        befunde.len(),
        1,
        "Genau eine Groesse ueberschreitet — gemeldet wurden {befunde:?}"
    );
    assert_eq!(befunde[0].name, "vessel");
    assert!(
        befunde[0].ueberschuss() > 99.0,
        "u32::MAX liegt hundertfach ueber dem Deckel; der Ueberschuss muss das zeigen, \
         damit 'knapp darueber' und 'weit darueber' unterscheidbar bleiben."
    );
}

#[test]
fn die_aufzaehlungen_decken_ihre_strukturen_ab() {
    // Die Zusicherung in den Datensatz: Ohne sie koennte eine Aufzaehlung
    // leer laufen und jeder Abgleich darueber waere gruen.
    assert_eq!(
        faktoren_aufgezaehlt(&NEUTRAL).len(),
        8,
        "DamageFactors hat acht Felder; weicht die Zahl ab, hat die Destrukturierung ein \
         Feld verloren — und dann haette der Bau brechen muessen."
    );
    assert_eq!(
        getragene_aufgezaehlt(&WornValues::default()).len(),
        7,
        "WornValues hat sieben Felder."
    );
    assert_eq!(
        WIRKGROESSEN.len(),
        15,
        "Acht Faktoren plus sieben getragene Werte. Diese Zahl steht in der MELDUNG einer \
         Abweichung, damit sie beim Wachsen auffaellt — der eigentliche Schutz ist die \
         Destrukturierung, nicht diese Zahl."
    );
}

/// Ein Eintrag laesst sich von aussen bauen — sonst waere der Typ nicht
/// benutzbar und die Tabelle nicht erweiterbar.
#[test]
fn ein_eintrag_ist_von_aussen_konstruierbar() {
    let g = Wirkgroesse {
        name: "prueffall",
        traeger: Traeger::Faktor,
        deckel: Deckel::Grenze {
            max: 7,
            bitbreite: "kuenstlich",
            engste_kette: "kein echter Rechenweg — nur fuer diesen Prueffall gebaut",
        },
    };
    assert!(g.deckel.haelt(7));
    assert!(!g.deckel.haelt(8));
}
