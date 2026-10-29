//! Gleichlauf der Regeln ueber dem Kampf (Issue #922).
//!
//! Die Aufzeichnung `tests/parity/recordings/laufbahn-regeln.json` entsteht
//! aus den Spielfunktionen selbst (`tests/parity/laufbahn-regeln.ts`). Dieser
//! Prueffall haelt jede Zeile gegen den Nachbau in
//! `sim/src/laufbahn/regeln.rs`: Beute, Aufwertung, Punkte, Erfahrung,
//! Freischaltung, getragene Werte, automatischer Vergleich, erwarteter Stand.
//!
//! **Jedes Feld einer Zeile wird verglichen**, nicht nur das Ergebnis, auf das
//! es ankommt — zwei Felder in einer Zeile sichern einander nicht. Und jeder
//! Abschnitt zaehlt vorher, dass er Zeilen hat: Eine Schleife ueber null
//! Zeilen ist gruen und prueft nichts.

use rota_sim::data::{BOSSES, MAX_LEVEL};
use rota_sim::gear::WornValues;
use rota_sim::laufbahn::regeln::{
    boss_stufe, candidates_for, compare_bag_item, effective_level_of, erwarteter_stand,
    estimate_damage_per_hit, freigeschaltete_instanzen, grant_xp, item_level_of, loot_stream_for,
    material_for, points_breakdown, progress_of, quality_cap_of, roll_loot, strom_startwert,
    upgrade_cap_of, upgrade_item, weights_for, worn_from, xp_for_victory, Fundort, Punktequelle,
    Stueck, Stufenstand, Vorlage, DROP_CHANCE_PER_MILLE, FELD_CATALYSIS, FELD_ESSENCE,
    FELD_GEAR_EFFECTS, FELD_PURITY, FELD_YIELD, HELD_POINTS_PER_PERCENT, INSTANZEN, KATALOG,
    LOOT_STREAM_INDEX, MATERIAL_PER_LEVEL, MATERIAL_PER_QUALITY, POINTS_EXPONENT,
    POINTS_PER_SECOND, REPEAT_PER_MILLE, WEIGHT_TOTAL, XP_PER_BOSS_LEVEL,
};
use serde_json::Value;

const AUFZEICHNUNG: &str = include_str!("../../tests/parity/recordings/laufbahn-regeln.json");

fn daten() -> Value {
    let wert: Value =
        serde_json::from_str(AUFZEICHNUNG).expect("laufbahn-regeln.json ist kein JSON");
    assert_eq!(wert["schema"].as_u64(), Some(1), "unbekanntes Schema");
    wert
}

/// Die Zeilen eines Abschnitts — mit der Zusicherung, dass es welche gibt.
fn zeilen(abschnitt: &str, mindestens: usize) -> Vec<String> {
    let liste: Vec<String> = daten()[abschnitt]
        .as_array()
        .unwrap_or_else(|| panic!("Abschnitt {abschnitt} fehlt"))
        .iter()
        .map(|z| z.as_str().expect("Zeile ist kein Text").to_owned())
        .collect();
    assert!(
        liste.len() >= mindestens,
        "{abschnitt}: nur {} Zeilen, erwartet mindestens {mindestens} — prueft der Fall noch etwas?",
        liste.len()
    );
    println!("{abschnitt}: {} Zeilen", liste.len());
    liste
}

/// Teilt eine Zeile am senkrechten Strich in ihre Teile, jedes in Felder.
fn teile(zeile: &str) -> Vec<Vec<String>> {
    zeile
        .split(" | ")
        .map(|t| t.split_whitespace().map(str::to_owned).collect())
        .collect()
}

fn ganz(feld: &str) -> i64 {
    feld.parse()
        .unwrap_or_else(|_| panic!("\"{feld}\" ist keine ganze Zahl"))
}

fn vielleicht(feld: &str) -> Option<f64> {
    (feld != "-").then(|| {
        feld.parse()
            .unwrap_or_else(|_| panic!("\"{feld}\" ist keine Zahl"))
    })
}

fn bits(feld: &str) -> u64 {
    u64::from_str_radix(feld, 16).unwrap_or_else(|_| panic!("\"{feld}\" ist kein Bitmuster"))
}

fn wahr(feld: &str) -> bool {
    match feld {
        "1" => true,
        "0" => false,
        anders => panic!("\"{anders}\" ist kein Wahrheitswert"),
    }
}

fn werte_felder(w: &WornValues) -> [u32; 7] {
    [
        w.catalysis,
        w.purity,
        w.yield_bonus,
        w.essence,
        w.vessel,
        w.ward,
        w.gear_effects,
    ]
}

fn stueck_aus(feld: &str) -> Stueck {
    let t: Vec<&str> = feld.split(':').collect();
    assert_eq!(t.len(), 5, "Stueck \"{feld}\" hat nicht fuenf Teile");
    Stueck {
        vorlage: usize::try_from(ganz(t[0])).expect("Katalogindex"),
        item_level: u32::try_from(ganz(t[1])).expect("Stufe"),
        quality: usize::try_from(ganz(t[2])).expect("Guete"),
        upgrade_steps: u32::try_from(ganz(t[3])).expect("Aufwertung"),
        platz: (t[4] != "-").then(|| usize::try_from(ganz(t[4])).expect("Platz")),
    }
}

fn stueck_feld(s: &Stueck) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        s.vorlage,
        s.item_level,
        s.quality,
        s.upgrade_steps,
        s.platz.map_or_else(|| "-".to_owned(), |p| p.to_string())
    )
}

fn einzelkatalog() -> Vec<Vorlage> {
    // Dieselbe Wahl wie `einzelkatalog()` in der Aufzeichnung.
    vec![*KATALOG
        .iter()
        .find(|v| v.gebunden_an.is_empty() && v.satz == -1)
        .expect("kein ungebundenes Stueck ohne Satz")]
}

#[test]
fn die_konstanten_stimmen() {
    let d = daten();
    let k = &d["konstanten"];
    assert_eq!(
        k["dropChancePerMille"].as_u64(),
        Some(u64::from(DROP_CHANCE_PER_MILLE))
    );
    assert_eq!(k["weightTotal"].as_u64(), Some(u64::from(WEIGHT_TOTAL)));
    assert_eq!(
        k["lootStreamIndex"].as_u64(),
        Some(u64::from(LOOT_STREAM_INDEX))
    );
    assert_eq!(k["materialPerLevel"].as_u64(), Some(MATERIAL_PER_LEVEL));
    let tabelle: Vec<u64> = k["materialPerQuality"]
        .as_array()
        .expect("materialPerQuality")
        .iter()
        .map(|v| v.as_u64().expect("Zahl"))
        .collect();
    assert_eq!(tabelle, MATERIAL_PER_QUALITY.to_vec());
    assert_eq!(
        bits(k["pointsPerSecondBits"].as_str().expect("Text")),
        POINTS_PER_SECOND.to_bits()
    );
    assert_eq!(
        bits(k["pointsExponentBits"].as_str().expect("Text")),
        POINTS_EXPONENT.to_bits()
    );
    assert_eq!(
        bits(k["heldPointsPerPercentBits"].as_str().expect("Text")),
        HELD_POINTS_PER_PERCENT.to_bits()
    );
    assert_eq!(k["repeatPerMille"].as_f64(), Some(REPEAT_PER_MILLE));
    assert_eq!(k["xpPerBossLevel"].as_u64(), Some(XP_PER_BOSS_LEVEL));
    assert_eq!(k["maxLevel"].as_u64(), Some(u64::from(MAX_LEVEL)));
    let mut geprueft = 0;
    for zeile in k["lootStreamSeeds"].as_array().expect("lootStreamSeeds") {
        let f: Vec<i64> = zeile.as_str().expect("Text").split(' ').map(ganz).collect();
        let seed = u32::try_from(f[0]).expect("Startwert");
        assert_eq!(
            i64::from(strom_startwert(seed, LOOT_STREAM_INDEX)),
            f[1],
            "Startwert {seed}"
        );
        geprueft += 1;
    }
    for zeile in k["materialFor"].as_array().expect("materialFor") {
        let f: Vec<i64> = zeile.as_str().expect("Text").split(' ').map(ganz).collect();
        assert_eq!(material_for(f[0]) as i64, f[1], "materialFor({})", f[0]);
        geprueft += 1;
    }
    assert!(geprueft >= 15, "nur {geprueft} Konstanten-Zeilen");
}

#[test]
fn der_katalog_ist_derselbe() {
    let liste = zeilen("katalog", 20);
    assert_eq!(liste.len(), KATALOG.len(), "Zahl der Vorlagen");
    for (zeile, v) in liste.iter().zip(KATALOG.iter()) {
        let f: Vec<&str> = zeile.split(' ').collect();
        assert_eq!(f[0], v.id, "{zeile}");
        assert_eq!(ganz(f[1]), v.slot as i64, "{zeile}: Platz");
        assert_eq!(ganz(f[2]), i64::from(v.item_level), "{zeile}: Stufe");
        assert_eq!(ganz(f[3]), v.quality as i64, "{zeile}: Qualitaet");
        assert_eq!(
            f[4],
            if v.gebunden_an.is_empty() {
                "-"
            } else {
                v.gebunden_an
            },
            "{zeile}"
        );
        assert_eq!(ganz(f[5]), i64::from(v.satz), "{zeile}: Satz");
        let felder: Vec<usize> = f
            .get(6)
            .map(|arten| {
                arten
                    .split(',')
                    .filter(|a| !a.is_empty())
                    .map(|a| match a {
                        "catalysis" => FELD_CATALYSIS,
                        "purity" => FELD_PURITY,
                        "yield" => FELD_YIELD,
                        "essence" => FELD_ESSENCE,
                        "gearEffects" => FELD_GEAR_EFFECTS,
                        anders => panic!("{zeile}: unbekannte Art {anders}"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(felder, v.affixe.to_vec(), "{zeile}: Affixe");
    }
    for zeile in zeilen("kandidaten", 6) {
        let t = teile(&zeile);
        let boss = &t[0][0];
        let ids: Vec<&str> = candidates_for(boss, &KATALOG)
            .into_iter()
            .map(|i| KATALOG[i].id)
            .collect();
        assert_eq!(ids.join(","), t[1].join(","), "Kandidaten fuer {boss}");
    }
}

#[test]
fn instanzen_und_boss_stufen_sind_dieselben() {
    let liste = zeilen("instanzen", 6);
    assert_eq!(liste.len(), INSTANZEN.len(), "Zahl der Instanzen");
    for (zeile, i) in liste.iter().zip(INSTANZEN.iter()) {
        let f: Vec<&str> = zeile.split(' ').collect();
        assert_eq!(f[0], i.id, "{zeile}");
        assert_eq!(ganz(f[1]), i64::from(i.number), "{zeile}: number");
        assert_eq!(ganz(f[2]), i64::from(i.order), "{zeile}: order");
        assert_eq!(f[3], i.requires.unwrap_or("-"), "{zeile}: requires");
    }
    for zeile in zeilen("bossStufen", 21) {
        let f: Vec<&str> = zeile.split(' ').collect();
        assert_eq!(ganz(f[1]), i64::from(boss_stufe(f[0])), "{zeile}: Stufe");
        let boss = BOSSES
            .iter()
            .find(|b| b.id == f[0])
            .expect("Boss fehlt im Simulator");
        assert_eq!(f[2], boss.instance, "{zeile}: Instanz");
    }
}

#[test]
fn gewichte_obergrenze_und_fundstufe_stimmen() {
    for zeile in zeilen("beute", 500) {
        let t = teile(&zeile);
        let ein: Vec<i32> = t[0]
            .iter()
            .map(|f| i32::try_from(ganz(f)).expect("i32"))
            .collect();
        let aus: Vec<i64> = t[1].iter().map(|f| ganz(f)).collect();
        let (i, d, e) = (ein[0], ein[1], ein[2]);
        let w = weights_for(i, d, e);
        assert_eq!(
            w.iter().map(|g| i64::from(*g)).collect::<Vec<_>>(),
            aus[0..4].to_vec(),
            "{zeile}: Gewichte"
        );
        assert_eq!(quality_cap_of(i) as i64, aus[4], "{zeile}: Obergrenze");
        assert_eq!(
            progress_of(i, d, e) as i64,
            aus[5],
            "{zeile}: Fortschrittsstufe"
        );
        assert_eq!(
            i64::from(item_level_of(i, d, e)),
            aus[6],
            "{zeile}: Fundstufe"
        );
    }
}

#[test]
fn jeder_beutewurf_faellt_gleich() {
    let alle = KATALOG.to_vec();
    let einer = einzelkatalog();
    let mut gefallen = 0;
    let mut leer = 0;
    for zeile in zeilen("wuerfe", 1000) {
        let t = teile(&zeile);
        let ein = &t[0];
        let aus = &t[1];
        let seed = u32::try_from(ganz(&ein[0])).expect("Startwert");
        let ort = Fundort {
            instance: i32::try_from(ganz(&ein[1])).expect("Instanz"),
            difficulty: i32::try_from(ganz(&ein[2])).expect("Schwierigkeit"),
            endless_level: i32::try_from(ganz(&ein[3])).expect("Endlos"),
            boss_id: &ein[4],
        };
        let katalog = match ein[5].as_str() {
            "alle" => &alle,
            "einer" => &einer,
            anders => panic!("{zeile}: Katalog {anders}"),
        };
        let rate = u32::try_from(ganz(&ein[6])).expect("Rate");
        let mut strom = loot_stream_for(seed);
        let fund = roll_loot(&mut strom, &ort, katalog, rate);
        match fund {
            None => {
                assert_eq!(&aus[0..3], ["-", "-", "-"], "{zeile}: Rust wirft nichts");
                leer += 1;
            }
            Some(f) => {
                assert_eq!(aus[0], katalog[f.vorlage].id, "{zeile}: Vorlage");
                assert_eq!(ganz(&aus[1]), i64::from(f.item_level), "{zeile}: Stufe");
                assert_eq!(ganz(&aus[2]), f.quality as i64, "{zeile}: Qualitaet");
                gefallen += 1;
            }
        }
        assert_eq!(
            ganz(&aus[3]),
            i64::from(strom.draws()),
            "{zeile}: Ziehungen"
        );
    }
    // Beide Ausgaenge muessen vorkommen, sonst prueft die Reihe nur einen.
    println!("gefallen {gefallen}, leer {leer}");
    assert!(
        gefallen > 100 && leer > 100,
        "gefallen {gefallen}, leer {leer}"
    );
}

#[test]
fn aufwertung_und_obergrenze_stimmen() {
    let mut gedeckelt = 0;
    for zeile in zeilen("aufwertung", 700) {
        let t = teile(&zeile);
        let ein: Vec<i64> = t[0].iter().map(|f| ganz(f)).collect();
        let aus = &t[1];
        let (i, d, e) = (
            i32::try_from(ein[2]).expect("i32"),
            i32::try_from(ein[3]).expect("i32"),
            i32::try_from(ein[4]).expect("i32"),
        );
        let r = upgrade_item(ein[0], ein[1], i, d, e);
        assert_eq!(r.level, ganz(&aus[0]), "{zeile}: Stufe");
        assert_eq!(r.spent as i64, ganz(&aus[1]), "{zeile}: verbraucht");
        assert_eq!(r.capped_by_content, wahr(&aus[2]), "{zeile}: gedeckelt");
        assert_eq!(
            i64::from(upgrade_cap_of(i, d, e)),
            ganz(&aus[3]),
            "{zeile}: Obergrenze"
        );
        if r.capped_by_content {
            gedeckelt += 1;
        }
    }
    assert!(gedeckelt > 50, "nur {gedeckelt} gedeckelte Faelle");
}

#[test]
fn punkte_stimmen_bis_aufs_bit() {
    let mut wiederholt = 0;
    let mut ausgezahlt = 0;
    for zeile in zeilen("punkte", 1500) {
        let t = teile(&zeile);
        let ein = &t[0];
        let aus = &t[1];
        let quelle = Punktequelle {
            victory: wahr(&ein[0]),
            remaining_ms: vielleicht(&ein[1]).expect("Restzeit"),
            held_percent: vielleicht(&ein[2]),
        };
        let r = points_breakdown(&quelle, ganz(&ein[3]), ganz(&ein[4]), vielleicht(&ein[5]));
        assert_eq!(r.units.to_bits(), bits(&aus[0]), "{zeile}: Einheiten");
        assert_eq!(r.base, ganz(&aus[1]), "{zeile}: Kurve");
        assert_eq!(r.previous_base, ganz(&aus[2]), "{zeile}: Bestwert-Kurve");
        assert_eq!(r.repeat_only, wahr(&aus[3]), "{zeile}: nur Wiederholung");
        assert_eq!(
            i64::from(r.falloff_per_mille),
            ganz(&aus[4]),
            "{zeile}: Abstandsfaktor"
        );
        assert_eq!(r.awarded, ganz(&aus[5]), "{zeile}: ausgezahlt");
        wiederholt += usize::from(r.repeat_only);
        ausgezahlt += usize::from(r.awarded > 0);
    }
    assert!(
        wiederholt > 50 && ausgezahlt > 200,
        "wiederholt {wiederholt}, ausgezahlt {ausgezahlt}"
    );
}

#[test]
fn erfahrung_und_aufstieg_stimmen() {
    for zeile in zeilen("erfahrungSieg", 40) {
        let t = teile(&zeile);
        assert_eq!(
            xp_for_victory(ganz(&t[0][0]), ganz(&t[0][1])),
            ganz(&t[1][0]),
            "{zeile}"
        );
    }
    let mut aufgestiegen = 0;
    for zeile in zeilen("erfahrungAufstieg", 100) {
        let t = teile(&zeile);
        let vorher = Stufenstand {
            level: u32::try_from(ganz(&t[0][0])).expect("Stufe"),
            xp: ganz(&t[0][1]),
        };
        let r = grant_xp(vorher, ganz(&t[0][2]));
        assert_eq!(i64::from(r.level), ganz(&t[1][0]), "{zeile}: Stufe");
        assert_eq!(r.xp, ganz(&t[1][1]), "{zeile}: Erfahrung");
        aufgestiegen += usize::from(r.level > vorher.level);
    }
    assert!(aufgestiegen > 20, "nur {aufgestiegen} Aufstiege");
}

#[test]
fn freischaltung_stimmt() {
    for zeile in zeilen("freischaltung", 40) {
        let (links, rechts) = zeile.split_once(" | ").expect("Trenner");
        let besiegt: Vec<&str> = if links == "-" {
            Vec::new()
        } else {
            links.split(',').collect()
        };
        let offen: Vec<&str> = freigeschaltete_instanzen(|i| besiegt.contains(&BOSSES[i].id))
            .into_iter()
            .map(|i| INSTANZEN[i].id)
            .collect();
        assert_eq!(offen.join(","), rechts, "besiegt: {links}");
    }
}

#[test]
fn effektive_stufe_stimmt() {
    for zeile in zeilen("effektiveStufe", 140) {
        let t = teile(&zeile);
        let wert = |f: &str| (f != "-").then(|| ganz(f));
        assert_eq!(
            i64::from(effective_level_of(
                wert(&t[0][0]),
                wert(&t[0][1]),
                ganz(&t[0][2])
            )),
            ganz(&t[1][0]),
            "{zeile}"
        );
    }
}

#[test]
fn getragene_werte_und_vergleich_stimmen() {
    let mut verglichen = 0;
    let mut besser = 0;
    for zeile in zeilen("getragen", 80) {
        let teile: Vec<&str> = zeile.split(" | ").collect();
        let stuecke: Vec<Stueck> = if teile[0] == "-" {
            Vec::new()
        } else {
            teile[0].split(',').map(stueck_aus).collect()
        };
        let erwartet: Vec<&str> = teile[1].split(' ').collect();
        let werte = worn_from(&stuecke);
        let felder = werte_felder(&werte);
        for (i, feld) in felder.iter().enumerate() {
            assert_eq!(i64::from(*feld), ganz(erwartet[i]), "{zeile}: Feld {i}");
        }
        assert_eq!(
            estimate_damage_per_hit(&werte).to_bits(),
            bits(erwartet[7]),
            "{zeile}: Schaetzung"
        );
        // `-` steht fuer „keine Stuecke" UND fuer „ein Stueck, getragen" — die
        // Zahl der Stuecke trennt die beiden Lesarten.
        let vergleiche: Vec<&str> = if stuecke.is_empty() {
            assert_eq!(teile[2], "-", "{zeile}: Vergleiche ohne Stuecke");
            Vec::new()
        } else {
            teile[2].split(',').collect()
        };
        assert_eq!(
            vergleiche.len(),
            stuecke.len(),
            "{zeile}: Zahl der Vergleiche"
        );
        for (index, soll) in vergleiche.iter().enumerate() {
            match compare_bag_item(&stuecke, index) {
                None => assert_eq!(*soll, "-", "{zeile}: Stueck {index}"),
                Some(v) => {
                    let (urteil, prozent) = soll.split_once(':').expect("Urteil:Bits");
                    assert_eq!(
                        i64::from(v.verdict),
                        ganz(urteil),
                        "{zeile}: Urteil {index}"
                    );
                    assert_eq!(
                        v.damage_percent.to_bits(),
                        bits(prozent),
                        "{zeile}: Prozent {index}"
                    );
                    verglichen += 1;
                    besser += usize::from(v.verdict == 1);
                }
            }
        }
    }
    assert!(
        verglichen > 50 && besser > 5,
        "verglichen {verglichen}, besser {besser}"
    );
}

#[test]
fn der_erwartete_stand_ist_derselbe() {
    for zeile in zeilen("erwartet", 50) {
        let teile: Vec<&str> = zeile.split(" | ").collect();
        let kopf: Vec<i64> = teile[0].split(' ').map(ganz).collect();
        let stand = erwarteter_stand(
            i32::try_from(kopf[0]).expect("Vorstufe"),
            usize::try_from(kopf[1]).expect("Plaetze"),
        );
        let text = if stand.is_empty() {
            "-".to_owned()
        } else {
            stand.iter().map(stueck_feld).collect::<Vec<_>>().join(",")
        };
        assert_eq!(text, teile[1], "{zeile}: Stuecke");
        let felder = werte_felder(&worn_from(&stand));
        let soll: Vec<i64> = teile[2].split(' ').map(ganz).collect();
        assert_eq!(
            felder.iter().map(|f| i64::from(*f)).collect::<Vec<_>>(),
            soll,
            "{zeile}: Werte"
        );
    }
}
