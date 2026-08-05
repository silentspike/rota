//! Die Schadensformel im Kampf (GAME-DESIGN 6.3).
//!
//! **Der wichtigste Test hier ist `jeder_schadensweg_geht_durch_die_formel`.**
//! Er ist aus einem echten Fehler entstanden: Beim Einbau der Faktoren blieb der
//! Entlade-Zauber PYRE angeschlossen an den alten Weg — sein Schaden lief an der
//! Formel vorbei. Aufgefallen ist das nicht am Test, sondern beim Nachrechnen
//! einer Skalierungszahl, die zu klein war.
//!
//! Der Fehler war deshalb so unauffällig, weil das Ergebnis **plausibel** blieb:
//! Alle sechs Faktoren zeigten denselben Zuwachs, nur eben einen zu kleinen. Wer
//! die Zahl nicht gegen eine Handrechnung hält, sieht nichts.

use rota_sim::apl::parse;
use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{rotation, DamageFactors, Fight, FightConfig, RunResult, NEUTRAL};

/// Die gute Rotation, wie sie im Repository liegt.
const GOOD: &str = include_str!("../apl/step1-good.apl");

fn lauf(factors: DamageFactors) -> RunResult {
    Fight::new(FightConfig {
        stop_on_kill: false,
        factors,
        ..FightConfig::default()
    })
    .run_apl(&parse("gut", GOOD).expect("gueltig"))
}

#[test]
fn jeder_schadensweg_geht_durch_die_formel() {
    // Drei Wege erzeugen Schaden, und jeder einzelne muss die Faktoren sehen:
    //   KINDLE    Sofortschaden beim Wirken
    //   SMOULDER  Schaden je Tick
    //   PYRE      Schaden aus der verbrauchten Ressource
    // Bleibt einer unveraendert, ist er nicht angeschlossen.
    let ohne = lauf(NEUTRAL);
    let mit = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });

    for (name, index) in [("KINDLE", KINDLE), ("SMOULDER", SMOULDER), ("PYRE", PYRE)] {
        assert!(
            ohne.damage_by_spell[index] > 0,
            "{name} macht ohne Faktoren keinen Schaden — der Test prueft nichts"
        );
        assert!(
            mit.damage_by_spell[index] > ohne.damage_by_spell[index],
            "{name} ist nicht an die Schadensformel angeschlossen: {} bleibt {} auch mit +10 %",
            ohne.damage_by_spell[index],
            mit.damage_by_spell[index]
        );
    }
}

#[test]
fn die_einzelwerte_stimmen_mit_der_handrechnung() {
    // Gegen von Hand gerechnete Zahlen, nicht gegen den Prüfling selbst.
    // Gute Rotation über die vollen 60 s: 30x KINDLE, 29 SMOULDER-Ticks, 5x PYRE.
    let mit = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });
    assert_eq!(mit.damage_by_spell[KINDLE], 30 * 110, "100 -> 110");
    assert_eq!(mit.damage_by_spell[SMOULDER], 29 * 44, "40 -> 44");
    assert_eq!(mit.damage_by_spell[PYRE], 5 * 396, "360 -> 396");
    assert_eq!(mit.damage, 30 * 110 + 29 * 44 + 5 * 396);
}

#[test]
fn die_rundung_frisst_kleine_zuwaechse() {
    // Ein Befund, kein Mangel: Bei +1 % steigt KINDLE von 100 auf 101, ein
    // SMOULDER-Tick aber von 40 auf 40,4 — und damit gerundet auf 40. Der
    // Zuwachs verschwindet.
    //
    // Fuer den Balance-Laeufer heisst das: Ein zu kleines Delta misst nicht die
    // Skalierung, sondern die Rundung. Deshalb steht die Zahl je Prozentpunkt
    // immer neben dem verwendeten Delta.
    let ohne = lauf(NEUTRAL);
    let ein_prozent = lauf(DamageFactors {
        essence: 1.01,
        ..NEUTRAL
    });
    assert_eq!(
        ein_prozent.damage_by_spell[SMOULDER], ohne.damage_by_spell[SMOULDER],
        "40 x 1,01 = 40,4 -> 40: der Zuwachs faellt der Rundung zum Opfer"
    );
    assert!(
        ein_prozent.damage_by_spell[KINDLE] > ohne.damage_by_spell[KINDLE],
        "100 x 1,01 = 101: hier bleibt der Zuwachs erhalten"
    );

    // Bei zehn Prozent ueberlebt auch der Tick.
    let zehn_prozent = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    });
    assert!(zehn_prozent.damage_by_spell[SMOULDER] > ohne.damage_by_spell[SMOULDER]);
}

#[test]
fn neutrale_faktoren_aendern_am_gleichlauf_nichts() {
    // **Die Bedingung dafuer, dass #54 gueltig bleibt.** Mit neutralen Faktoren
    // muss der Simulator auf dieselben Zahlen kommen wie vorher — sonst waere
    // der Abgleich mit dem Spiel durch den Einbau der Formel gebrochen.
    let mit_apl = Fight::new(FightConfig::default()).run_apl(&parse("gut", GOOD).expect("gueltig"));
    let mit_funktion = Fight::new(FightConfig::default()).run(rotation::good);

    assert_eq!(mit_apl.damage, 4240);
    assert_eq!(mit_apl.killed_at_ms, Some(40_510));
    assert_eq!(mit_apl, mit_funktion);
}

#[test]
fn alle_faktoren_wirken_gleich_stark() {
    // Die Schadensformel multipliziert sie alle an derselben Stelle; keiner ist
    // bevorzugt. Das ist keine Selbstverstaendlichkeit, sondern eine Eigenschaft
    // von 6.3 — und der Grund, warum in Schritt 1 alle Skalierungswerte gleich
    // sind.
    //
    // **Die Faktoren kommen aus GEPRUEFTE_FAKTOREN, nicht aus einer Liste
    // hier** (Issue #346). Sonst gaebe es zwei Listen: die, die geprueft wird,
    // und die, gegen die der Waechter vergleicht — und sie koennten
    // auseinanderlaufen. Genau davor schuetzt dieser Umbau.
    let referenz = lauf(DamageFactors {
        essence: 1.10,
        ..NEUTRAL
    })
    .damage;

    for name in GEPRUEFTE_FAKTOREN {
        let factors = mit_faktor(name, 1.10);
        assert_eq!(
            lauf(factors).damage,
            referenz,
            "der Faktor {name} wirkt anders als die uebrigen: {factors:?}"
        );
    }
}

/// Die Faktoren, deren Staerke [`alle_faktoren_wirken_gleich_stark`] prueft.
///
/// **Diese Liste ist die einzige Quelle.** Der Prueffall liest sie, und der
/// Waechter aus Issue #346 vergleicht sie gegen die Felder der Struktur. Zwei
/// getrennte Listen koennten auseinanderlaufen — und ihr Auseinanderlaufen
/// saehe aus wie Ruhe.
const GEPRUEFTE_FAKTOREN: &[&str] = &[
    "essence",
    "talents",
    "gear_effects",
    "buffs",
    "synergy",
    "resistance",
    "crit",
];

/// Setzt genau einen Faktor und laesst die uebrigen neutral.
///
/// **Der Uebersetzer erzwingt hier nichts** — ein unbekannter Name faellt
/// durch in den `panic`. Das ist Absicht: Die Namen stammen aus
/// [`GEPRUEFTE_FAKTOREN`], und ein Tippfehler dort soll laut scheitern statt
/// still einen Faktor zu ueberspringen.
fn mit_faktor(name: &str, wert: f64) -> DamageFactors {
    match name {
        "essence" => DamageFactors {
            essence: wert,
            ..NEUTRAL
        },
        "talents" => DamageFactors {
            talents: wert,
            ..NEUTRAL
        },
        "gear_effects" => DamageFactors {
            gear_effects: wert,
            ..NEUTRAL
        },
        "buffs" => DamageFactors {
            buffs: wert,
            ..NEUTRAL
        },
        "synergy" => DamageFactors {
            synergy: wert,
            ..NEUTRAL
        },
        "resistance" => DamageFactors {
            resistance: wert,
            ..NEUTRAL
        },
        "vulnerability" => DamageFactors {
            vulnerability: wert,
            ..NEUTRAL
        },
        "crit" => DamageFactors {
            crit: wert,
            ..NEUTRAL
        },
        anderer => panic!(
            "mit_faktor kennt {anderer} nicht — steht er in GEPRUEFTE_FAKTOREN, \
             fehlt hier ein Zweig"
        ),
    }
}

/// Die Namen aller Felder von [`DamageFactors`], zur Laufzeit aus der Struktur
/// selbst (Issue #346).
///
/// # Warum aus der Debug-Ausgabe und nicht aus einer zweiten Liste
///
/// Die Debug-Ableitung **ist** die Struktur, nur als Text. Wer ein Feld
/// ergaenzt, ergaenzt damit auch diese Liste — ohne es zu wollen und ohne es
/// zu koennen. Eine handgefuehrte zweite Liste haette genau das Problem, das
/// dieser Waechter loesen soll: Sie kann zurueckbleiben, und ihr Schweigen
/// sieht aus wie Zustimmung.
fn alle_faktor_namen() -> Vec<String> {
    let text = format!("{NEUTRAL:?}");
    let inhalt = text
        .split_once('{')
        .and_then(|(_, rest)| rest.rsplit_once('}'))
        .map(|(mitte, _)| mitte)
        .unwrap_or("");
    inhalt
        .split(',')
        .filter_map(|teil| teil.split_once(':'))
        .map(|(name, _)| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// Faktoren, die der Kampf **selbst** schreibt — fuer sie ist ein
/// Staerke-Vergleich mit festem Wert nicht sinnvoll.
///
/// **Die Ausnahme muss hier stehen, sonst gibt es sie nicht.** Eine Ausnahme,
/// die man nennen muss, ist eine Entscheidung; eine, die durch Weglassen
/// entsteht, ist ein Versehen — und genau das war der Zustand: `gear_effects`
/// fiel bei Issue #195 aus der Pruefung, ohne dass jemand es bemerkte, weil
/// Fehlen und Ausnahme identisch aussehen.
const SCHREIBT_DER_KAMPF_SELBST: &[(&str, &str)] = &[(
    "vulnerability",
    "Der Kampf setzt ihn je Treffer aus dem Boss-Zustand (Issue #97). Ein \
     fester Wert wuerde beim ersten Schritt ueberschrieben.",
)];

#[test]
fn kein_faktor_faellt_lautlos_aus_der_staerke_pruefung() {
    // Issue #346. **Der Waechter zaehlt nicht, was er kennt — er vergleicht
    // mit dem, was da ist.**
    //
    // Bis hierher stand die Pruefliste allein da. Faellt ein Faktor heraus
    // oder kommt einer dazu, ohne dass jemand die Liste anfasst, schweigt
    // sie. Genau das ist zweimal passiert:
    //
    //   vulnerability   seit #97   bewusst, aber unbenannt
    //   gear_effects    seit #195  unbemerkt — von mir gebaut, von mir
    //                              uebersehen
    //
    // Der Waechter war nicht falsch gebaut, er war nicht mitgewachsen. Das
    // sieht identisch aus.
    let alle = alle_faktor_namen();

    // Der Gegenfall zuerst: Liefert die Namensliste nichts, ist unten alles
    // trivial erfuellt.
    assert!(
        alle.len() >= 8,
        "nur {} Faktornamen aus der Debug-Ausgabe gelesen — dann prueft der \
         Rest nichts. Hat sich das Debug-Format geaendert?",
        alle.len()
    );

    let ausgenommen: Vec<&str> = SCHREIBT_DER_KAMPF_SELBST.iter().map(|(n, _)| *n).collect();
    let fehlend: Vec<&String> = alle
        .iter()
        .filter(|name| !GEPRUEFTE_FAKTOREN.contains(&name.as_str()))
        .filter(|name| !ausgenommen.contains(&name.as_str()))
        .collect();

    assert!(
        fehlend.is_empty(),
        "Diese Faktoren stehen in DamageFactors, werden aber weder in \
         `alle_faktoren_wirken_gleich_stark` geprueft noch in \
         SCHREIBT_DER_KAMPF_SELBST begruendet ausgenommen: {fehlend:?}\n\
         Entweder gehoert der Faktor in die Pruefung — oder die Ausnahme \
         gehoert benannt. Beides ist eine Entscheidung; ihn wegzulassen ist \
         keine."
    );

    // Und die Gegenrichtung: Eine Ausnahme fuer einen Faktor, den es gar
    // nicht mehr gibt, ist eine Zeile, die niemand mehr prueft.
    for (name, _) in SCHREIBT_DER_KAMPF_SELBST {
        assert!(
            alle.iter().any(|f| f == name),
            "SCHREIBT_DER_KAMPF_SELBST nennt {name}, aber DamageFactors hat \
             kein solches Feld mehr"
        );
    }
}
