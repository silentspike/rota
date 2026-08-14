//! Wie stark wächst der Schadensdurchsatz mit der Gegenstandsstufe? (Issue #791)
//!
//! # Warum diese Messung fehlte
//!
//! #791 hält fest, dass die Boss-Lebenspunkte über die Instanzen **fallen**
//! (5575 → 5000 → 4450), während die Gegenstandsstufe **steigt** (1 → 7 → 13).
//! Wer instance-3 erreicht, findet dort leichtere Bosse und trägt bessere
//! Ausrüstung.
//!
//! Bevor eine einzige Lebenspunktzahl geändert wird, muss feststehen, **wie
//! viel** die Ausrüstung ausmacht. Ohne diese Zahl wäre jede neue Kurve
//! geraten — und `boss-1-2` zeigt, wie eng es ist: 6400 Lebenspunkte gegen
//! 6412 erreichbaren Schaden, 0,19 % Reserve.
//!
//! # Was `gear-balance.rs` nicht liefert
//!
//! Dort stehen zwei Messungen, und keine beantwortet diese Frage:
//!
//! | | |
//! | --- | --- |
//! | Messung 1 | Satz gegen Affix — **nennt sich selbst Modellrechnung** |
//! | Messung 2 | Klammerung der Formel — echte Messung, aber über Rundung |
//!
//! Messung 2 vergleicht `(Grund × f1) × f2` mit `Grund × (f1 × f2)`. Das ist
//! eine Aussage über Gleitkomma-Rundung, nicht über Durchsatz.
//!
//! # Und was in den Daten nicht steht
//!
//! Alle 27 Einträge in `data/items.json` tragen `itemLevel: 40`. Die Stufe
//! entsteht erst beim Fund (`itemLevelOf` in `src/loot/rates.ts`:
//! `(i-1)*6 + d*3 + e + 1`, also 1 / 7 / 13 für die drei Instanzen). Wer die
//! Skalierung an der Datei messen wollte, fände nichts.
//!
//! # Was hier gemessen wird
//!
//! Zwei Messreihen über die **volle** Spanne, Stufe 1 bis 85:
//!
//! 1. Qualität fest auf RARE — das isoliert `budget_of`, eine Variable.
//! 2. Qualität gekoppelt wie `qualityCapOf` sie vergibt — der Durchsatz, den
//!    ein Spieler wirklich erlebt.
//!
//! Die Differenz beider **ist** der Qualitätsanteil, gemessen statt
//! auseinandergerechnet. Jede Rotation wird je Lauf neu gesucht.

use rota_sim::balance::{run, BalanceConfig};
use rota_sim::data::boss_by_id;
use rota_sim::gear::{
    budget_of, distribute_budget, WornValues, QUALITY_HAS_EFFECT, QUALITY_NAMES, SLOT_COUNT,
};
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::sources::{base_permille, gear_effects_permille};

/// Die Stufen, an denen gemessen wird — **Endpunkte 1 und 85 sind Pflicht.**
///
/// 85 ist das Ende der Spanne ohne Endlos-Stufen: `itemLevelOf(14, 2, 0)` =
/// `(14-1)*6 + 2*3 + 0 + 1`. Die ersten drei Instanzen (1 / 7 / 13) decken nur
/// ein Sechstel davon ab; eine dort gemessene Zahl gilt für das Spiel nicht.
const STUFEN: [(u32, &str); 8] = [
    (1, "Instanz 1, Schwierigkeit 0"),
    (7, "Instanz 2, Schwierigkeit 0"),
    (13, "Instanz 3, Schwierigkeit 0"),
    (25, "Instanz 5, Schwierigkeit 0"),
    (43, "Instanz 8, Schwierigkeit 0"),
    (61, "Instanz 11, Schwierigkeit 0"),
    (79, "Instanz 14, Schwierigkeit 0"),
    (85, "Instanz 14, Schwierigkeit 2"),
];

/// Die Qualitätsgrenze je Stufe, nach `qualityCapOf` in `src/loot/rates.ts`.
///
/// **Sie ist an die Instanz gekoppelt, nicht frei wählbar**: Instanz 1–3 gibt
/// höchstens RARE, 4–8 höchstens EPIC, ab 9 LEGENDARY. Ein Spieler auf Stufe 79
/// bekommt gar kein RARE mehr.
fn qualitaet_bei(stufe: u32) -> usize {
    let instanz = (stufe - 1) / 6 + 1;
    if instanz >= 9 {
        3
    } else if instanz >= 4 {
        2
    } else {
        1
    }
}

/// **Ein voller Satz, nicht ein Einzelstück.**
///
/// Ein Spieler trägt zehn Plätze. Die Skalierung an einem Stück zu messen
/// beantwortete eine andere Frage — nämlich die nach dem Beitrag eines
/// Fundstücks, nicht nach dem Stand einer Instanz.
fn ausruestung(item_level: u32, quality: usize) -> WornValues {
    let mut summe = WornValues::none();
    for _platz in 0..SLOT_COUNT {
        // Die Deklaration steht IN der Schleife: Außerhalb wäre die erste
        // Zuweisung nie gelesen, und `clippy -D warnings` bricht daran ab.
        let mut werte = [0u64; 4];
        // **`belegt` kommt aus den DATEN, nicht aus einer Wahl hier.**
        //
        // `distribute_budget` liest `QUALITY_AFFIX_SLOTS = [1, 2, 3, 3]`
        // (`gear.rs:141`) und belegt danach: RARE zwei Plätze, EPIC drei. Wer
        // diese Schleife für „vier Größen" hält, weil das Feld vier Einträge
        // hat, misst etwas anderes als das Spiel.
        //
        // Genau daran hing die Deutung des ganzen Befunds: Zwischen der
        // RARE- und der EPIC-Messung wandern **zwei** Dinge zugleich — das
        // Budget (Faktor 1,376) und die Zahl der belegten Töpfe (2 → 3). Die
        // 21,84-fache Differenz stammt fast vollständig vom zweiten.
        let belegt = distribute_budget(item_level, quality, &mut werte);
        // Die Affix-Arten reihum auf die belegten Plätze — so verteilt sich das
        // Budget über die belegten Größen statt sich auf einer zu stapeln.
        for (i, wert) in werte.iter().take(belegt).enumerate() {
            let w = u32::try_from(*wert).unwrap_or(u32::MAX);
            match i % 4 {
                0 => summe.catalysis += w,
                1 => summe.purity += w,
                2 => summe.yield_bonus += w,
                _ => summe.essence += w,
            }
        }
    }
    // **Topf 4 wird HEREINGEREICHT, nicht abgeleitet — und LEGENDARY braucht ihn.**
    //
    // Ohne die Zeilen unten wäre LEGENDARY in dieser Messung ein EPIC: Beide
    // haben drei Affix-Plätze, der einzige Unterschied ist `QUALITY_HAS_EFFECT`.
    // Damit wäre die obere Hälfte der Spanne ungültig, denn ab Instanz 9
    // (Stufe 49) ist alles legendär.
    //
    // `distribute_budget` rührt `gear_effects` nicht an, und das ist Absicht:
    // `best_build.rs:246` hält mit `assert_eq!(worn.gear_effects, 0, "Topf 4
    // gehoert der anderen Seite")` fest, dass die Zuleitung dem Aufrufer
    // gehört. Gäbe es zwei Wege zu dieser Zahl, liefen sie auseinander, sobald
    // einer nachgezogen wird — und beide lieferten weiter plausible Werte.
    //
    // Hier ist der Aufrufer, also rechnet er ihn: über `gear_effects_permille`,
    // dieselbe Funktion, die auch die Spielseite benutzt
    // (`gearEffectsPermille` in `src/gear/sources.ts`). Der Satzanteil bleibt 0
    // — diese Messung trägt keine Sätze.
    let legendaere = if QUALITY_HAS_EFFECT[quality] {
        u32::try_from(SLOT_COUNT).unwrap_or(0)
    } else {
        0
    };
    summe.gear_effects = gear_effects_permille(legendaere, 0);
    summe
}

fn config(boss: &'static rota_sim::data::Boss, worn: WornValues) -> BalanceConfig {
    BalanceConfig {
        iterations: 200,
        threads: 4,
        stop_on_kill: false,
        boss,
        worn,
        ..BalanceConfig::default()
    }
}

/// Dieselbe Ausrüstung, aber **alles auf CATALYSIS** statt auf die belegten Töpfe.
///
/// **Warum diese Messung entscheidet.** Zwischen RARE und EPIC liegen auf
/// Stufe 85 nur 37,6 % mehr Budget (7898 gegen 10866) — gemessen wurden aber
/// 21,8-fach mehr Schaden. Vier Größen in verschiedenen Töpfen werden laut
/// GAME-DESIGN 13.1b multipliziert; ein Zuwachs auf allen vieren könnte sich
/// also auftürmen.
///
/// Liegt alles auf EINER Größe, entfällt die Multiplikation. Fällt der
/// Unterschied dann auf etwa 1,4, sind es die Töpfe. Bleibt er bei 20, sitzt
/// er in einer einzelnen Größe — und dann ist die Frage, in welcher.
fn ausruestung_eine_groesse(item_level: u32, quality: usize) -> WornValues {
    let mut summe = WornValues::none();
    for _platz in 0..SLOT_COUNT {
        // Die Deklaration steht IN der Schleife: Außerhalb wäre die erste
        // Zuweisung nie gelesen, und `clippy -D warnings` bricht daran ab.
        let mut werte = [0u64; 4];
        let belegt = distribute_budget(item_level, quality, &mut werte);
        for wert in werte.iter().take(belegt) {
            summe.catalysis += u32::try_from(*wert).unwrap_or(u32::MAX);
        }
    }
    let legendaere = if QUALITY_HAS_EFFECT[quality] {
        u32::try_from(SLOT_COUNT).unwrap_or(0)
    } else {
        0
    };
    summe.gear_effects = gear_effects_permille(legendaere, 0);
    summe
}

/// Durchsatz mit **gesuchter** Rotation — was mit dieser Ausrüstung erreichbar ist.
fn durchsatz(boss: &'static rota_sim::data::Boss, worn: WornValues) -> f64 {
    let cfg = config(boss, worn);
    let gefunden = greedy(&cfg, &all_candidates(), 8);
    run(&cfg, &gefunden.best.apl).damage.mean
}

/// Durchsatz mit **fester** Rotation — nur die Ausrüstung ändert sich.
///
/// **Warum das die entscheidende Trennung ist.** Mit gesuchter Rotation fiel
/// der Wert von Stufe 1 auf Stufe 7 (13655 → 12671), obwohl das Budget stieg
/// (140 → 185). Zwei Ursachen sind möglich: Die Ausrüstung wirkt dort
/// schlechter, oder `greedy` findet einen schlechteren Pfad. Solange das offen
/// ist, misst die Zahl eine Summe aus Ausrüstung und Suchgüte und heißt
/// „Skalierung".
///
/// Hier liegt die Rotation fest — die bei Stufe 1 gefundene. Fällt der Wert
/// jetzt nicht mehr, war es die Suche.
fn durchsatz_fest(
    boss: &'static rota_sim::data::Boss,
    worn: WornValues,
    apl: &rota_sim::apl::Apl,
) -> f64 {
    run(&config(boss, worn), apl).damage.mean
}

fn main() {
    println!("== Durchsatz nach Gegenstandsstufe (Issue #791) ==");
    println!();
    println!("  GEMESSEN AN EINEM LAUF, keine Modellrechnung.");
    println!(
        "  Rotation je Lauf neu gesucht, 200 Durchgaenge, voller Satz ({SLOT_COUNT} Plaetze)."
    );
    println!();

    let boss = boss_by_id("boss-1-2").expect("boss-1-2 fehlt");
    let ohne = durchsatz(boss, WornValues::none());
    println!("  {:<34} {:>10.0}", "ohne Ausruestung (NEUTRAL)", ohne);
    println!();

    // MESSUNG 0 — FESTE ROTATION. Trennt Ausruestung von Suchguete.
    let basis_cfg = config(boss, ausruestung(1, 1));
    let feste = greedy(&basis_cfg, &all_candidates(), 8).best.apl.clone();
    println!("  MESSUNG 0 — FESTE Rotation (die bei Stufe 1 gefundene), Qualitaet RARE");
    println!("    Nur die Ausruestung aendert sich. Faellt hier nichts, war der");
    println!("    Rueckgang bei Stufe 7 die SUCHE und nicht die Ausruestung.");
    let mut basis0 = 0.0;
    for (stufe, woher) in STUFEN {
        let wert = durchsatz_fest(boss, ausruestung(stufe, 1), &feste);
        if stufe == 1 {
            basis0 = wert;
        }
        let faktor = if basis0 > 0.0 { wert / basis0 } else { 0.0 };
        println!("    Stufe {stufe:>3}  {woher:<28}  {wert:>10.0}  Faktor {faktor:>6.2}");
    }
    println!();

    // MESSUNG 1 — Qualitaet FEST. Isoliert `budget_of`: eine Variable.
    println!("  MESSUNG 1 — Qualitaet fest auf RARE, nur die Stufe waechst");
    let mut basis = 0.0;
    for (stufe, woher) in STUFEN {
        let budget = budget_of(stufe, 1);
        let wert = durchsatz(boss, ausruestung(stufe, 1));
        if stufe == 1 {
            basis = wert;
        }
        let faktor = if basis > 0.0 { wert / basis } else { 0.0 };
        println!(
            "    Stufe {stufe:>3}  {woher:<28}  Budget {budget:>8}  {wert:>10.0}  Faktor {faktor:>5.2}"
        );
    }
    println!();

    // MESSUNG 2 — Qualitaet gekoppelt, wie das Spiel sie vergibt.
    //
    // **Der Faktor aus dieser Reihe ist eine UNTERGRENZE, keine Messung des
    // Spiels.** Er kam als 376 nach main (PR #844) und wird seither zitiert.
    //
    // Der Grund steht in Messung 6: Hier bekommt der Spieler nur Affixe. Das
    // Spiel gibt zusaetzlich IMMER die Grundwerte mit (`data/items.json`), und
    // weil epic nur drei Affix-Plaetze hat, kam `essence` reihum nie dran —
    // ausgerechnet die staerkste Groesse (Messung 5: 655432 gegen 122317).
    //
    // Mit dem Grundwert steht dort 2627 statt 376. Wer diese Zeile zitiert,
    // zitiert einen Spieler, den es nicht gibt.
    println!("  MESSUNG 2 — Qualitaet gekoppelt (qualityCapOf) — UNTERGRENZE, siehe Messung 6");
    let mut basis2 = 0.0;
    for (stufe, woher) in STUFEN {
        let q = qualitaet_bei(stufe);
        let wert = durchsatz(boss, ausruestung(stufe, q));
        if stufe == 1 {
            basis2 = wert;
        }
        let faktor = if basis2 > 0.0 { wert / basis2 } else { 0.0 };
        println!(
            "    Stufe {stufe:>3}  {:<10}  {woher:<28}  {wert:>10.0}  Faktor {faktor:>5.2}",
            QUALITY_NAMES[q]
        );
    }
    println!();

    // MESSUNG 6 — MIT den Grundwerten, so wie das Spiel sie vergibt.
    //
    // `data/items.json` sagt: „Ein Gegenstand traegt IMMER die drei Grundwerte
    // ESSENCE, VESSEL und WARD." Sie kommen aus `base_permille`, NICHT aus dem
    // Affix-Budget. In den Messungen 0 bis 5 fehlen sie — dort ist `essence`
    // eine der vier Affix-Arten, und weil `distribute_budget` bei EPIC nur drei
    // Plaetze belegt, bekommt es reihum nie etwas.
    //
    // Messung 5 zeigt, wie schwer das wiegt: essence allein liefert auf Stufe 85
    // 655432, catalysis nur 122317. Die Messungen davor haben also einen Spieler
    // gerechnet, dem der staerkste Wert fehlt.
    //
    // Ob das die FAKTOREN verschiebt, ist damit noch nicht gesagt — der Wert
    // fehlt in beiden Vergleichsgliedern und koennte sich kuerzen. Genau das
    // misst diese Reihe.
    // **Die 10 steht hier und nicht als Konstante** — auf der Rust-Seite gibt es
    // sie nicht. `sim/src/sources.rs:111` erwähnt sie im Kommentar
    // („stehen auf der TypeScript-Seite als `ESSENCE_AT_LEVEL_1`"), die Zahl
    // selbst liegt in `src/gear/sources.ts:65`. Ein abgeschriebener Wert ist ein
    // zweiter Weg zu derselben Zahl; hier ist er unvermeidlich, weil der
    // Simulator keinen Katalog hat — also steht wenigstens die Fundstelle daneben.
    const ESSENCE_AT_LEVEL_1: u32 = 10;
    println!("  MESSUNG 6 — Affixe PLUS Grundwert essence (base_permille, Basis 10)");
    let mut basis6 = 0.0;
    for (stufe, woher) in STUFEN {
        let mut worn = ausruestung(stufe, qualitaet_bei(stufe));
        // Zehn Plaetze, jeder traegt den Grundwert seiner Stufe.
        let grund =
            base_permille(stufe, ESSENCE_AT_LEVEL_1) * u32::try_from(SLOT_COUNT).unwrap_or(1);
        worn.essence += grund;
        let wert = durchsatz(boss, worn);
        if stufe == 1 {
            basis6 = wert;
        }
        let faktor = if basis6 > 0.0 { wert / basis6 } else { 0.0 };
        println!(
            "    Stufe {stufe:>3}  {woher:<28}  Grundwert {grund:>6}  {wert:>12.0}  Faktor {faktor:>8.2}"
        );
    }
    println!();

    // MESSUNG 7 — die Gegenprobe zu vessel und ward.
    //
    // `data/items.json` nennt DREI Grundwerte: essence, vessel, ward. Messung 6
    // gibt nur essence mit. Der Code sagt, warum das reichen sollte: `vessel_max`
    // sind die Lebenspunkte (`fight.rs:600`), `ward` die Schadensreduktion — und
    // dieser Aufbau faehrt `stop_on_kill: false`, der Spieler stirbt also nicht.
    //
    // **Das ist ein Code-Argument und damit keine Messung.** Wenn der Aufbau die
    // beiden Groessen per Konstruktion wirkungslos macht, muss das sichtbar sein:
    // gleiche Zahl wie Messung 6. Weicht sie ab, ist meine Begruendung falsch —
    // und dann steht der PR auf einer Luecke, die ich fuer geschlossen hielt.
    println!("  MESSUNG 7 — Gegenprobe: essence PLUS vessel PLUS ward");
    for (stufe, _woher) in STUFEN {
        let mut worn = ausruestung(stufe, qualitaet_bei(stufe));
        let plaetze = u32::try_from(SLOT_COUNT).unwrap_or(1);
        worn.essence += base_permille(stufe, ESSENCE_AT_LEVEL_1) * plaetze;
        worn.vessel += base_permille(stufe, ESSENCE_AT_LEVEL_1) * plaetze;
        worn.ward += base_permille(stufe, ESSENCE_AT_LEVEL_1) * plaetze;
        println!("    Stufe {stufe:>3}  {:>12.0}", durchsatz(boss, worn));
    }
    println!();

    // MESSUNG 5 — GLEICHES Budget, jeweils auf EINE andere Groesse.
    //
    // Messung 3 und 4 sagen, dass die ZAHL der Toepfe den Faktor traegt. Sie
    // sagen nicht, ob die vier Groessen gleich viel wert sind. Liegt CATALYSIS
    // weit vorn, waere "die Zahl der Affix-Plaetze" die falsche Ueberschrift —
    // dann traegt die AUSWAHL, nicht die Anzahl.
    println!("  MESSUNG 5 — gleiches Budget, jeweils ALLES auf eine andere Groesse");
    for stufe in [13u32, 85] {
        let budget = budget_of(stufe, 1);
        let je = u32::try_from(budget).unwrap_or(0) * u32::try_from(SLOT_COUNT).unwrap_or(1);
        let vier = [
            (
                "catalysis",
                WornValues {
                    catalysis: je,
                    ..WornValues::none()
                },
            ),
            (
                "purity",
                WornValues {
                    purity: je,
                    ..WornValues::none()
                },
            ),
            (
                "yield",
                WornValues {
                    yield_bonus: je,
                    ..WornValues::none()
                },
            ),
            (
                "essence",
                WornValues {
                    essence: je,
                    ..WornValues::none()
                },
            ),
        ];
        for (name, worn) in vier {
            println!(
                "    Stufe {stufe:>3}  nur {name:<10} {:>12.0}",
                durchsatz(boss, worn)
            );
        }
    }
    println!();

    // MESSUNG 4 — GLEICHES Budget, verschieden viele Toepfe.
    //
    // Messung 3 zeigt: vier Groessen ergeben 21,84, eine ergibt 1,60. Offen
    // blieb, ob das am hoeheren Budget von EPIC liegt oder an der Zahl der
    // belegten Toepfe — denn `distribute_budget` belegt bei RARE ZWEI Plaetze
    // und bei EPIC DREI (QUALITY_AFFIX_SLOTS = [1, 2, 3, 3]). Beide wandern
    // zugleich.
    //
    // Hier wandert nur eins: dasselbe Budget, einmal auf zwei und einmal auf
    // drei Groessen verteilt. Was uebrig bleibt, ist der Preis eines Topfes.
    println!("  MESSUNG 4 — GLEICHES Budget, zwei gegen drei Toepfe");
    for stufe in [13u32, 85] {
        let budget = budget_of(stufe, 1);
        let je_zwei =
            u32::try_from(budget / 2).unwrap_or(0) * u32::try_from(SLOT_COUNT).unwrap_or(1);
        let je_drei =
            u32::try_from(budget / 3).unwrap_or(0) * u32::try_from(SLOT_COUNT).unwrap_or(1);
        let zwei = WornValues {
            catalysis: je_zwei,
            purity: je_zwei,
            ..WornValues::none()
        };
        let drei = WornValues {
            catalysis: je_drei,
            purity: je_drei,
            yield_bonus: je_drei,
            ..WornValues::none()
        };
        let w2 = durchsatz(boss, zwei);
        let w3 = durchsatz(boss, drei);
        println!(
            "    Stufe {stufe:>3}  Budget {budget:>6}  zwei Toepfe {w2:>10.0}  drei Toepfe {w3:>10.0}  Verhaeltnis {:>6.2}",
            w3 / w2
        );
    }
    println!();

    // MESSUNG 3 — EINE Groesse statt vier. Trennt Toepfe von Einzelgroesse.
    println!("  MESSUNG 3 — dasselbe Budget, aber ALLES auf CATALYSIS");
    println!("    Vergleich RARE gegen EPIC auf Stufe 85, einmal auf vier Groessen");
    println!("    verteilt und einmal auf eine. Der Unterschied der Verhaeltnisse");
    println!("    sagt, ob die Multiplikation der Toepfe den Faktor erzeugt.");
    for stufe in [13u32, 85] {
        let vier_rare = durchsatz(boss, ausruestung(stufe, 1));
        let vier_epic = durchsatz(boss, ausruestung(stufe, 2));
        let eine_rare = durchsatz(boss, ausruestung_eine_groesse(stufe, 1));
        let eine_epic = durchsatz(boss, ausruestung_eine_groesse(stufe, 2));
        println!(
            "    Stufe {stufe:>3}  VIER Groessen: rare {vier_rare:>10.0}  epic {vier_epic:>10.0}  Verhaeltnis {:>6.2}",
            vier_epic / vier_rare
        );
        println!(
            "    Stufe {stufe:>3}  EINE  Groesse: rare {eine_rare:>10.0}  epic {eine_epic:>10.0}  Verhaeltnis {:>6.2}",
            eine_epic / eine_rare
        );
    }
    println!();

    println!("  DIE ZAHL, die #791 braucht, steht in der Zeile 'Stufe 85' —");
    println!("  einmal je Messreihe. Ihre DIFFERENZ ist der Qualitaetsanteil.");
    println!();
    println!("  Steigt das Budget zwischen zwei Stufen NICHT, ist das der Befund");
    println!("  und nicht der Faktor: budget_of rechnet ganzzahlig (x 21 / 20),");
    println!("  und eine Abrundung je Schritt summiert sich ueber 84 Schritte.");
}
