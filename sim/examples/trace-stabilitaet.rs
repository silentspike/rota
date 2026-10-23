//! Wie weit darf man an den Zahlen drehen, bevor die Cast-Folge kippt? (#910)
//!
//! ```text
//! cargo remote -c -- \
//!   build --release --example trace-stabilitaet
//! cd sim && ../target/release/examples/trace-stabilitaet
//! ```
//!
//! # Die Frage
//!
//! Vier Bauteile stehen auf einer Annahme: der Trace-Ledger, die Trace-Regionen,
//! die inverse Treppenfunktion des Balance-Optimierers und der zweiteilige
//! Zwischenspeicher-Schluessel. Die Annahme lautet: *Wenn man eine Zahl ein
//! wenig aendert, bleibt die Folge der gewirkten Zauber dieselbe.*
//!
//! Fuer **Rotationsaenderungen** ist sie gemessen widerlegt — 99,1 Prozent der
//! Traces weichen schon vor der Einfuegeposition ab. Fuer **Zahlenaenderungen**,
//! also genau das, was ein Balance-Optimierer bewegt, misst dieses Programm sie.
//!
//! # Was gemessen wird
//!
//! Je Groesse und Boss: Ab welcher relativen Aenderung weicht die Cast-Folge
//! zum ersten Mal ab? Die Antwort ist ein INTERVALL, keine Grenze — kippt sie
//! bei 10 Prozent und nicht bei 5, liegt die echte Grenze irgendwo dazwischen
//! und bleibt unbekannt.
//!
//! # Zwei Kontrollzeilen, und die zweite ist die wichtigere
//!
//! ```text
//! Aenderung 0        -> die Folge MUSS identisch sein
//! Aenderung x1000    -> die Folge MUSS abweichen
//! ```
//!
//! Die zweite hat in der Vormessung vom 2026-08-14 einen Fehlbefund verhindert,
//! der wie ein perfektes Ergebnis aussah: Der erste Lauf meldete ueberall
//! Stabilitaet, und die Kontrollzeile zeigte, dass ein Faktor 1000 die Folge
//! ebenfalls nicht bewegte. Der Grund war `stop_on_kill = false` — mit fester
//! Kampfdauer kann Schaden den Kontrollfluss ueberhaupt nicht beruehren.
//!
//! **Deshalb steht `stop_on_kill: true` in jeder Konfiguration hier.** Ohne das
//! misst der Lauf nichts und sieht dabei vollstaendig aus.
//!
//! # Was dieses Programm NICHT tut
//!
//! Es baut keinen Ledger und beweist nichts. Es sagt, **wo** sich ein genauer
//! Nachweis lohnt. Und ein schmales Intervall macht den Ledger nicht wertlos:
//! Eine Binaersuche nach der kleinsten wirksamen Verstaerkung prueft hunderte
//! Werte innerhalb genau eines solchen Intervalls.

use rota_sim::apl::Apl;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::DamageFactors;
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

// ---------------------------------------------------------------------------
// Was gedreht wird
// ---------------------------------------------------------------------------

/// Eine Groesse, die ein Balance-Optimierer bewegen wuerde.
struct Groesse {
    /// Wie sie im Bericht heisst.
    name: &'static str,
    /// Setzt sie auf das Vielfache ihres Ausgangswerts.
    dreh: fn(&mut DamageFactors, f64),
}

/// Die acht Faktoren aus [`DamageFactors`], jeder einzeln.
///
/// **Vollstaendig, nicht ausgewaehlt.** Die Struktur hat genau diese acht
/// Felder; ein neuntes wuerde hier fehlen und im Bericht nicht auffallen.
/// Deshalb prueft ein Testfall unten, dass die Liste so lang ist wie die
/// Struktur Felder hat.
const GROESSEN: &[Groesse] = &[
    Groesse {
        name: "essence",
        dreh: |f, x| f.essence *= x,
    },
    Groesse {
        name: "talents",
        dreh: |f, x| f.talents *= x,
    },
    Groesse {
        name: "gear_effects",
        dreh: |f, x| f.gear_effects *= x,
    },
    Groesse {
        name: "buffs",
        dreh: |f, x| f.buffs *= x,
    },
    Groesse {
        name: "synergy",
        dreh: |f, x| f.synergy *= x,
    },
    Groesse {
        name: "resistance",
        dreh: |f, x| f.resistance *= x,
    },
    Groesse {
        name: "vulnerability",
        dreh: |f, x| f.vulnerability *= x,
    },
    Groesse {
        name: "crit",
        dreh: |f, x| f.crit *= x,
    },
];

/// Die geprueften relativen Aenderungen, aufsteigend.
///
/// Das Issue nennt ±1, ±10 und ±50 Prozent. 5 und 100 kommen dazu, weil die
/// Vormessung zwischen 1 und 5 Prozent die interessanten Sprünge fand.
///
/// **Aufsteigend ist Pflicht:** Gesucht ist die ERSTE Stufe, bei der die Folge
/// kippt. Eine unsortierte Liste faende irgendeine.
const STUFEN: &[f64] = &[0.01, 0.05, 0.10, 0.50, 1.00];

/// Die Bosse aus dem Bestand, an denen gemessen wird.
const BOSSE: &[&str] = &["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];

/// Die Startwerte. AC4 verlangt mindestens drei.
const STARTWERTE: &[u32] = &[0, 1, 2];

/// Die Rotationslaengen, aus denen die drei Rotationen entstehen.
///
/// Drei verschiedene greedy-Laeufe ergeben drei verschiedene Rotationen —
/// nicht dieselbe dreimal. Der Bericht druckt sie aus, damit das nachprüfbar
/// ist und nicht geglaubt werden muss.
const ROTATIONSLAENGEN: &[usize] = &[4, 6, 8];

// ---------------------------------------------------------------------------
// Die Cast-Folge
// ---------------------------------------------------------------------------

/// Was ein Kampf an Zaubern gewirkt hat, in der Reihenfolge.
///
/// # Warum das Protokoll und nicht `RunResult::casts`
///
/// `casts` ist eine ZAEHLUNG je Zauber. Zwei Kaempfe, die dieselben Zauber in
/// anderer Reihenfolge wirken, haben identische Zaehlungen und verschiedene
/// Folgen — und genau die Reihenfolge ist hier die Frage.
fn cast_folge(config: &FightConfig, apl: &Apl) -> Result<Vec<u8>, String> {
    let mut fight = Fight::new(*config);
    let ergebnis = fight.run_apl(apl);

    // **Der Ringpuffer ist die Falle dieser Messung.** Laeuft das Protokoll
    // ueber, fehlen die FRUEHEN Eintraege — und dann vergleicht man zwei
    // abgeschnittene Folgen, die beide gleich aussehen, weil ihr Anfang fehlt.
    // Das faellt nicht auf: Die Folge ist dann kuerzer, aber immer noch eine.
    if fight.log().overflowed() {
        return Err(format!(
            "Protokoll uebergelaufen ({} Eintraege Platz) — die Folge waere \
             abgeschnitten und der Vergleich wertlos",
            config.log_capacity
        ));
    }

    // Traeger-Zusicherung: Ein Kampf ohne Schaden hat auch keine Folge, die
    // sich aendern koennte.
    if ergebnis.damage == 0 {
        return Err("der Kampf richtet nichts aus — 0 Schaden".to_owned());
    }

    Ok(fight
        .log()
        .entries()
        .iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| e.spell)
        .collect())
}

/// Wie eine Groesse gedreht in eine Konfiguration eingeht.
fn mit_faktor(basis: &FightConfig, groesse: &Groesse, faktor: f64) -> FightConfig {
    let mut config = *basis;
    (groesse.dreh)(&mut config.factors, faktor);
    config
}

// ---------------------------------------------------------------------------
// Der Befund je Groesse und Boss
// ---------------------------------------------------------------------------

/// Was bei einer Groesse herauskam.
enum Befund {
    /// Die Folge kippte zuerst bei dieser relativen Aenderung.
    KipptBei(f64),
    /// Ueber den ganzen geprueften Bereich stabil — mit der groessten
    /// geprueften Stufe, damit „stabil" nicht mit „unendlich stabil"
    /// verwechselt wird.
    StabilBis(f64),
    /// Der Messweg erreicht diese Groesse nicht — Faktor 1000 bewegt nichts.
    ///
    /// **Das ist kein Befund ueber die Stabilitaet, sondern ueber den Weg.**
    /// Ohne diese Unterscheidung stuende hier „stabil", und genau das waere
    /// der Fehlbefund, vor dem AC3 warnt.
    Unerreichbar,
    /// Nicht messbar, mit Grund.
    Blockiert(String),
}

impl std::fmt::Display for Befund {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KipptBei(x) => write!(f, "{:>7.0} %", x * 100.0),
            // **AC2 woertlich:** „stabil bis" statt „ueber 100 %" — der
            // geprüfte Bereich endete, das ist etwas anderes als „kippt nie".
            Self::StabilBis(x) => write!(f, "stab>{:.0}%", x * 100.0),
            Self::Unerreichbar => write!(f, "{:>9}", "n/erreicht"),
            Self::Blockiert(_) => write!(f, "{:>9}", "BLOCKED"),
        }
    }
}

/// Sucht die erste Stufe, bei der die Folge abweicht.
///
/// **Mit eigener Traeger-Zusicherung je Groesse.** Das genuegt nicht einmal
/// zentral: Fuenf der acht Faktoren werden im Kampf aus anderen Quellen NEU
/// BERECHNET und ueberschreiben dabei den Wert aus der Konfiguration —
/// `synergy` aus dem Synergie-Fenster, `vulnerability` aus den Zerfallsstapeln,
/// `talents` aus `talents_base`, `resistance` je Zauber, `buffs` aus dem
/// Verstaerkungs-Zustand (`sim/src/fight.rs`, zwölf Zuweisungen an
/// `self.factors.*`).
///
/// Wer sie ueber `FightConfig` dreht, dreht an einem Wert, den der Kampf
/// wegwirft — und bekommt „stabil", weil nichts ankommt. Genau so entstand in
/// der ersten Fassung dieser Messung die Zeile „synergy stabil ueber den
/// ganzen Bereich": kein Befund ueber den Ledger, sondern einer ueber den
/// Messweg.
fn erste_abweichung(basis: &FightConfig, apl: &Apl, groesse: &Groesse) -> Befund {
    let referenz = match cast_folge(basis, apl) {
        Ok(f) => f,
        Err(grund) => return Befund::Blockiert(grund),
    };

    // Kommt diese Groesse ueberhaupt an? Faktor 1000 muss die Folge bewegen.
    match cast_folge(&mit_faktor(basis, groesse, 1000.0), apl) {
        Ok(f) if f == referenz => return Befund::Unerreichbar,
        Err(grund) => return Befund::Blockiert(grund),
        Ok(_) => {}
    }

    for &stufe in STUFEN {
        // Beide Richtungen: Ein Optimierer dreht nach oben UND unten, und die
        // Folge kann in einer Richtung frueher kippen als in der anderen.
        for faktor in [1.0 + stufe, 1.0 - stufe] {
            if faktor <= 0.0 {
                continue;
            }
            match cast_folge(&mit_faktor(basis, groesse, faktor), apl) {
                Ok(f) if f != referenz => return Befund::KipptBei(stufe),
                Ok(_) => {}
                Err(grund) => return Befund::Blockiert(grund),
            }
        }
    }
    Befund::StabilBis(*STUFEN.last().unwrap_or(&0.0))
}

// ---------------------------------------------------------------------------
// Die zwei Kontrollzeilen
// ---------------------------------------------------------------------------

/// Ob der Messweg ueberhaupt etwas sehen kann.
struct Kontrolle {
    /// Aenderung 0 — die Folge muss identisch sein.
    null_identisch: bool,
    /// Faktor 1000 — die Folge muss abweichen.
    tausend_kippt: bool,
    /// Wie lang die Referenzfolge war.
    laenge: usize,
}

fn kontrolle(basis: &FightConfig, apl: &Apl) -> Result<Kontrolle, String> {
    let referenz = cast_folge(basis, apl)?;
    let null = cast_folge(&mit_faktor(basis, &GROESSEN[0], 1.0), apl)?;
    let tausend = cast_folge(&mit_faktor(basis, &GROESSEN[0], 1000.0), apl)?;
    Ok(Kontrolle {
        null_identisch: null == referenz,
        tausend_kippt: tausend != referenz,
        laenge: referenz.len(),
    })
}

// ---------------------------------------------------------------------------
// Lauf
// ---------------------------------------------------------------------------

/// Druckt je Boss und Rotation einen Fingerabdruck der Cast-Folge.
///
/// # Wozu ein eigener Modus
///
/// AC5 fragt, ob ein EINZELNER Zauber-Grundschaden eine andere Grenze hat als
/// ein globaler Faktor. Der Grundschaden steht in `data/spells/*.json` und
/// wird zur BAUZEIT einkompiliert — er laesst sich zur Laufzeit nicht drehen,
/// weil es keinen zauber-spezifischen Faktor gibt (`DamageFactors` hat acht
/// GLOBALE Felder).
///
/// Der Weg ist deshalb: Wert in `data/` aendern, neu bauen, diesen Modus
/// fahren, Ausgaben vergleichen. Ein Fingerabdruck je Zeile macht den
/// Vergleich zwischen zwei Bauten zu einem `diff`.
fn folgen_drucken() {
    println!("CAST-FOLGEN, FINGERABDRUCK JE BOSS UND ROTATION");
    println!("(fuer den Vergleich ueber Bauten hinweg — AC5)\n");

    let alle = all_candidates();
    for &laenge in ROTATIONSLAENGEN {
        for &bossname in BOSSE {
            let Some(boss) = boss_by_id(bossname) else {
                continue;
            };
            let such_config = BalanceConfig {
                boss,
                threads: std::thread::available_parallelism()
                    .map_or(1, std::num::NonZeroUsize::get),
                stop_on_kill: true,
                ..BalanceConfig::default()
            };
            let apl = greedy(&such_config, &alle, laenge).best.apl;
            let basis = FightConfig {
                boss,
                stop_on_kill: true,
                ..FightConfig::default()
            };
            match cast_folge(&basis, &apl) {
                Ok(folge) => {
                    // FNV-1a ueber die Folge — dieselbe Rechnung wie die
                    // Zustands-Pruefsumme des Projekts, kein eigener
                    // Mechanismus.
                    let mut h = rota_sim::checksum::FNV_OFFSET;
                    for &spell in &folge {
                        h = rota_sim::checksum::mix_int(h, i32::from(spell));
                    }
                    println!(
                        "  len={laenge:<2} {bossname:<12} 0x{h:08x}  {:>3} Zauber",
                        folge.len()
                    );
                }
                Err(grund) => println!("  len={laenge:<2} {bossname:<12} BLOCKED — {grund}"),
            }
        }
    }
}

fn main() {
    if std::env::args().any(|a| a == "--folgen") {
        folgen_drucken();
        return;
    }
    println!("TRACE-STABILITAET BEI ZAHLENAENDERUNGEN (#910)");
    println!("=============================================\n");
    println!(
        "Geprueft: {} Groessen x {} Bosse x {} Rotationen x {} Startwerte",
        GROESSEN.len(),
        BOSSE.len(),
        ROTATIONSLAENGEN.len(),
        STARTWERTE.len()
    );
    println!(
        "Stufen: {}\n",
        STUFEN
            .iter()
            .map(|s| format!("{:.0} %", s * 100.0))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let alle = all_candidates();
    let mut zeilen_gesamt = 0usize;
    let mut blockiert_gesamt = 0usize;

    for &laenge in ROTATIONSLAENGEN {
        for &seed in STARTWERTE {
            for &bossname in BOSSE {
                let Some(boss) = boss_by_id(bossname) else {
                    println!("  {bossname}: BLOCKED — Boss fehlt");
                    continue;
                };

                // **`stop_on_kill: true` ist die entscheidende Einstellung.**
                // Ohne sie laeuft jeder Kampf die volle Zeit, und Schaden kann
                // den Kontrollfluss gar nicht beruehren — dann ist alles
                // „stabil" und die Messung wertlos.
                // **Die SUCHE darf parallel laufen, die MESSUNG nicht.**
                //
                // Die Rotationssuche ist über Stränge deterministisch — sie
                // zählt ihre Ergebnisse sortiert zusammen (GAME-DESIGN 18.4),
                // also findet sie mit acht Strängen dieselbe Rotation wie mit
                // einem. Die Messung darunter (`cast_folge`) rechnet ohnehin
                // einen einzelnen `Fight` und ist davon unberührt.
                //
                // Einsträngig gesucht brauchte der Lauf hochgerechnet 21
                // Minuten, und der Anteil lag fast vollständig in den
                // greedy-Läufen mit Länge 8.
                let such_config = BalanceConfig {
                    boss,
                    threads: std::thread::available_parallelism()
                        .map_or(1, std::num::NonZeroUsize::get),
                    stop_on_kill: true,
                    base_seed: seed,
                    ..BalanceConfig::default()
                };
                let gefunden = greedy(&such_config, &alle, laenge);
                let apl = gefunden.best.apl;

                let basis = FightConfig {
                    boss,
                    seed,
                    stop_on_kill: true,
                    ..FightConfig::default()
                };

                println!("Rotation aus {laenge} Zeilen, Startwert {seed}, {bossname}");
                println!(
                    "  gewaehlt: {}",
                    apl.rules
                        .iter()
                        .map(|r| rota_sim::data::SPELLS[r.spell].id)
                        .collect::<Vec<_>>()
                        .join(" > ")
                );

                match kontrolle(&basis, &apl) {
                    Err(grund) => {
                        println!("  BLOCKED — {grund}\n");
                        blockiert_gesamt += 1;
                        continue;
                    }
                    Ok(k) => {
                        println!(
                            "  Kontrolle: Aenderung 0 identisch {}   Faktor 1000 kippt {}   \
                             Folge {} Zauber",
                            if k.null_identisch { "JA" } else { "NEIN" },
                            if k.tausend_kippt { "JA" } else { "NEIN" },
                            k.laenge
                        );
                        if !k.null_identisch || !k.tausend_kippt {
                            // **Verwerfen, nicht berichten.** Ein Messweg, den
                            // ein Faktor 1000 nicht bewegt, misst den
                            // Kontrollfluss nicht.
                            println!(
                                "  ERGEBNIS VERWORFEN — die Kontrolle haelt nicht, \
                                 der Weg misst nichts\n"
                            );
                            blockiert_gesamt += 1;
                            continue;
                        }
                    }
                }

                for groesse in GROESSEN {
                    let befund = erste_abweichung(&basis, &apl, groesse);
                    if let Befund::Blockiert(ref grund) = befund {
                        println!("    {:<16} BLOCKED — {grund}", groesse.name);
                        blockiert_gesamt += 1;
                    } else {
                        println!("    {:<16} {befund}", groesse.name);
                    }
                    zeilen_gesamt += 1;
                }
                println!();
            }
        }
    }

    println!("Zusicherungen");
    println!("  {zeilen_gesamt} Messzeilen erzeugt, {blockiert_gesamt} blockiert.");
    if zeilen_gesamt == 0 {
        println!("  NICHTS GEMESSEN — das ist FEHLGESCHLAGEN, nicht bestanden.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Die Liste der Groessen muss so lang sein wie `DamageFactors` Felder hat.
    ///
    /// Ohne diesen Fall faellt ein neu hinzugekommener Faktor niemandem auf:
    /// Der Bericht ist dann vollstaendig fuer acht Groessen und stillschweigend
    /// unvollstaendig fuer neun.
    #[test]
    fn alle_faktoren_stehen_in_der_liste() {
        // `Debug` listet jedes Feld — der Uebersetzer schreibt die Aufzaehlung,
        // nicht eine von Hand gefuehrte Liste.
        let text = format!("{:?}", DamageFactors::default());
        let felder = text.matches(':').count();
        assert_eq!(
            GROESSEN.len(),
            felder,
            "DamageFactors hat {felder} Felder, die Liste kennt {} — \
             ein Faktor fehlt im Bericht",
            GROESSEN.len()
        );
    }

    /// Die Stufen muessen aufsteigen, sonst findet die Suche nicht die ERSTE.
    #[test]
    fn stufen_steigen_an() {
        for paar in STUFEN.windows(2) {
            assert!(paar[0] < paar[1], "{:?} ist nicht aufsteigend", STUFEN);
        }
        assert!(
            STUFEN.iter().all(|s| *s > 0.0),
            "eine Stufe ist nicht positiv"
        );
    }

    /// Jeder Drehgriff muss das Feld bewegen, das seinen NAMEN traegt.
    ///
    /// # Warum „genau ein Feld" nicht genuegt
    ///
    /// Die erste Fassung dieses Falls prueft nur, dass sich genau ein Feld
    /// aendert. Gemessen am 2026-08-18 mit einer Mutation: Ein Griff namens
    /// `talents`, der `f.essence` dreht, bewegt ebenfalls genau ein Feld — und
    /// besteht. Der Bericht zeigte dann zwei Zeilen unter zwei Namen, die
    /// dieselbe Groesse messen, und niemand haette es gesehen.
    ///
    /// Geprueft wird deshalb der NAME des geaenderten Feldes, nicht ihre Zahl.
    #[test]
    fn jeder_griff_dreht_das_feld_seines_namens() {
        // `Debug` von `DamageFactors` listet die Felder als "name: wert".
        let felder = |f: &DamageFactors| -> Vec<(String, String)> {
            format!("{f:?}")
                .trim_start_matches("DamageFactors {")
                .trim_end_matches('}')
                .split(',')
                .filter_map(|teil| {
                    let (n, w) = teil.split_once(':')?;
                    Some((n.trim().to_owned(), w.trim().to_owned()))
                })
                .collect()
        };

        let ruhe = DamageFactors::default();
        for groesse in GROESSEN {
            let mut gedreht = ruhe;
            (groesse.dreh)(&mut gedreht, 2.0);

            let bewegt: Vec<String> = felder(&ruhe)
                .into_iter()
                .zip(felder(&gedreht))
                .filter(|((_, a), (_, b))| a != b)
                .map(|((n, _), _)| n)
                .collect();

            assert_eq!(
                bewegt,
                vec![groesse.name.to_owned()],
                "der Griff \"{}\" bewegt {bewegt:?} statt sein eigenes Feld",
                groesse.name
            );
        }
    }
}
