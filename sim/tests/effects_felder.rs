//! Jedes Feld des Wirkungsknotens hat einen Leser (Issue #966, AC2).
//!
//! # Warum das ein eigener Prueflauf ist
//!
//! Ein Knotentyp mit zehn Feldern sieht vollstaendig aus. Ob die Felder
//! **gelesen** werden, sagt er nicht — und ein Feld ohne Leser ist genau die
//! „gepflegte Liste, nur eine Ebene tiefer versteckt", gegen die Epic E16
//! gebaut ist. Die Stop-Bedingung des Issues lautet deshalb: **kein Feld ohne
//! Leser.**
//!
//! # Die Frage muss richtig gestellt werden
//!
//! Der erste Entwurf mutierte je Feld **einen** Wert auf einen anderen und
//! fragte, ob sich etwas aendert. Damit meldete er `stacking` als ungelesen —
//! obwohl es einen Leser hat. Die gewaehlten zwei Werte (`None` und `Refresh`)
//! lagen zufaellig im selben Zweig. **Die Schwaeche lag in der Mutation, nicht
//! im Leser**, und haette ich sie „passend" gemacht, waere der Prueflauf
//! beliebig geworden.
//!
//! Deshalb probiert dieser Lauf **alle** sinnvollen Werte jedes Feldes durch
//! und fragt: Gibt es **zwei**, die zu verschiedenen Ableitungen fuehren? Das
//! ist die Frage, die „hat ein Leser" wirklich bedeutet, und sie laesst sich
//! nicht durch die Wahl der Mutation schoenen.
//!
//! # Die zweite Falle
//!
//! Jeder Knoten wird von jeder Analyse zuerst durch [`EffectNode::hat_luecke`]
//! geschickt. Ein Feld auf seinen `Unknown`-Wert zu setzen aendert deshalb
//! **immer** alle Ergebnisse — und misst nur, dass die Lueckenpruefung
//! funktioniert. Kein `Unknown` kommt in den Wertelisten vor, und
//! [`ohne_luecke`] sichert das ab. Ohne diese Zusicherung waere der ganze
//! Prueflauf gruen und bedeutungslos.

#[macro_use]
mod wirkungsanker;

use rota_sim::effects::passes::beispielknoten;
use rota_sim::effects::registry::{alle_analysen, AnalysisResult};
use rota_sim::effects::{
    Bounds, ControlFlowEffect, EffectNode, EffectOp, RandomnessSemantics, StackingSemantics,
    StateMask, Target, TimingSemantics, Trigger,
};
use wirkungsanker::{
    jede_zufallsquelle, jeder_ausloeser, jeder_baustein, jeder_hinweis, jedes_stapelverhalten,
    jedes_zeitverhalten, jedes_ziel,
};

/// Die Bausteine, ueber denen jedes Feld geprueft wird — **jeder**.
///
/// Mehrere, weil ein Feld nur fuer bestimmte Bausteine einen Leser haben kann:
/// `stacking` entscheidet bei einem Brand etwas anderes als bei einem
/// Ruecksetzer. Ein Feld gilt als gelesen, sobald es bei **irgendeinem**
/// Baustein zwei unterscheidbare Ableitungen erzeugt.
///
/// Bis #892 stand hier eine Liste ohne Anker: Ein neuer Baustein, in jeder
/// Analyse eingeordnet, fehlte ihr, ohne dass etwas rot wurde — und ein Feld,
/// das nur er liest, galt als ungelesen. Jetzt kommt sie aus
/// [`wirkungsanker::jeder_baustein`], deren Anker ein erschöpfender `match`
/// über dieselben Namen ist.
fn basis_bausteine() -> Vec<EffectOp> {
    jeder_baustein()
}

/// Nur die Werte, die eine Angabe sind — `Unknown` löst die Lückenprüfung aus
/// (siehe Modul-Kopf, „die zweite Falle“).
///
/// Gefiltert über die Methode des Typs, nicht über `hat_luecke`: Sonst wäre
/// [`kein_wert_der_wertelisten_erzeugt_eine_luecke`] durch Konstruktion grün.
fn angegeben<T: Copy>(alle: Vec<T>, unbekannt: fn(T) -> bool) -> Vec<T> {
    alle.into_iter().filter(|wert| !unbekannt(*wert)).collect()
}

/// Sichert, dass ein Knoten keine Luecke hat — sonst misst der Vergleich nur
/// [`EffectNode::hat_luecke`] und nicht den Leser des Feldes.
fn ohne_luecke(knoten: &EffectNode, wo: &str) {
    assert!(
        !knoten.hat_luecke(),
        "{wo}: Der Knoten traegt eine Luecke. Dann wuerde der Vergleich nur die \
         Lueckenpruefung ausloesen und ueber den Leser des Feldes NICHTS aussagen."
    );
}

/// Alle Setzer eines Feldes, jeder mit einem **gueltigen** Wert.
type Setzer = Box<dyn Fn(&mut EffectNode)>;

/// Ob zwei Werte des Feldes zu verschiedenen Ableitungen fuehren.
///
/// Vergleicht die **vollstaendigen** Ergebnisse, nicht nur die Verdikte: Drei
/// der Analysen sind beschreibend und tragen ihre Aussage in der `evidence`
/// (siehe Modul-Kopf von `effects::passes`). Wer nur Verdikte vergleicht,
/// haelt sie faelschlich fuer wirkungslos.
fn hat_leser(feld: &str, werte: &[Setzer]) -> bool {
    assert!(
        werte.len() >= 2,
        "{feld}: mit weniger als zwei Werten kann dieser Fall nichts feststellen."
    );
    for op in basis_bausteine() {
        let mut gesehen: Vec<Vec<AnalysisResult>> = Vec::new();
        for setzen in werte {
            let mut knoten = beispielknoten(op);
            setzen(&mut knoten);
            ohne_luecke(&knoten, &format!("{feld} bei {}", op.name()));
            let ergebnis = alle_analysen(&knoten);
            assert!(
                !ergebnis.is_empty(),
                "{feld}: keine Analysen registriert — dann prueft dieser Fall nichts."
            );
            if !gesehen.contains(&ergebnis) {
                gesehen.push(ergebnis);
            }
        }
        if gesehen.len() >= 2 {
            return true;
        }
    }
    false
}

/// Jedes Feld mit je allen sinnvollen Werten.
///
/// `op` steht nicht dabei: Es ist kein Feld neben den anderen, sondern die
/// Fallunterscheidung, ueber die jede Analyse ohnehin ihr `match` fuehrt.
///
/// # Seit #892 verankert
///
/// [`felder!`] verlangt jedes Feld des Knotens (ein Muster ohne `..`), und die
/// Werte der Aufzählungen kommen aus [`wirkungsanker`]. Bis dahin standen beide
/// als Listen hier, und die Zusicherung war `assert_eq!(geprueft, 10)` — eine
/// Zahl, die mit der Liste wuchs und nicht mit dem Knoten: Ein elftes Feld,
/// überall sonst eingeordnet, wäre an diesem Fall vorbeigewachsen. Ebenso ein
/// neuer Auslöser an der Werteliste von `trigger`.
fn felder() -> Vec<(&'static str, Vec<Setzer>)> {
    felder! {
        trigger: angegeben(jeder_ausloeser(), Trigger::ist_unbekannt),
        target: angegeben(jedes_ziel(), Target::ist_unbekannt),
        reads: [
            StateMask::RESOURCE,
            StateMask::BUFFS.union(StateMask::DOTS),
            StateMask::EMPTY,
        ],
        writes: [
            StateMask::BOSS_HP,
            StateMask::PLAYER_HP.union(StateMask::SHIELD),
            StateMask::EMPTY,
        ],
        timing: angegeben(jedes_zeitverhalten(), TimingSemantics::ist_unbekannt),
        stacking: angegeben(jedes_stapelverhalten(), StackingSemantics::ist_unbekannt),
        randomness: angegeben(jede_zufallsquelle(), RandomnessSemantics::ist_unbekannt),
        // Ohne Filter: Ein Hinweis ohne Richtung oder Bezug ist eine gültige
        // Deklaration, keine Lücke (Weg b, `effects_rueckfall.rs`).
        monotonicity: jeder_hinweis(),
        numeric_bounds: [
            Bounds::exact(0, 42),
            Bounds::UNKNOWN,
            Bounds {
                min: Some(0),
                max: None,
            },
        ],
        control_flow_effect: [
            ControlFlowEffect::NONE,
            ControlFlowEffect {
                branches_on_damage: true,
                branches_on_hp: false,
                branches_on_victory: false,
                unknown: false,
            },
            ControlFlowEffect {
                branches_on_damage: false,
                branches_on_hp: false,
                branches_on_victory: true,
                unknown: false,
            },
        ],
    }
}

/// Welche Analysen ein Feld tatsaechlich lesen — **gemessen, nicht gepflegt**.
///
/// Eine von Hand geschriebene Liste („`trigger` hat zwei Leser") waere genau
/// die Sorte, gegen die dieses Modul gebaut ist: Sie altert still, sobald
/// jemand einen Leser ergaenzt oder entfernt. Diese Funktion probiert stattdessen
/// alle Werte durch und sammelt die Analysen, deren Ergebnis sich aendert.
fn leser_von(werte: &[Setzer]) -> Vec<&'static str> {
    let mut leser: Vec<&'static str> = Vec::new();
    for op in basis_bausteine() {
        let mut ergebnisse: Vec<Vec<AnalysisResult>> = Vec::new();
        for setzen in werte {
            let mut knoten = beispielknoten(op);
            setzen(&mut knoten);
            ergebnisse.push(alle_analysen(&knoten));
        }
        // Je Analyse pruefen, ob sie ueber die Werte hinweg variiert.
        for i in 0..ergebnisse[0].len() {
            let erstes = &ergebnisse[0][i];
            if ergebnisse.iter().any(|e| e[i] != *erstes) {
                let name = erstes.pass.0;
                if !leser.contains(&name) {
                    leser.push(name);
                }
            }
        }
    }
    leser.sort_unstable();
    leser
}

#[test]
fn mehrfach_gelesene_felder_sind_benannt() {
    // # Warum dieser Fall existiert
    //
    // `kein_feld_ohne_leser` prueft „hat MINDESTENS EINEN Leser", nicht „jeder
    // Leser wirkt". Ein Feld mit zwei Lesern bleibt gelesen, wenn man einen
    // abklemmt — und der Waechter bleibt zu Recht gruen.
    //
    // **Genau daran ist am 2026-08-18 eine Abnahme fast gescheitert.** Der
    // Orchestrator klemmte einen der zwei `trigger`-Leser ab, sah Gruen und
    // stand kurz davor, „der Waechter greift nicht" zu melden. Er hatte recht
    // mit der Beobachtung und waere zum falschen Schluss gekommen.
    //
    // > Eine Mutation, die nur EINEN von mehreren Lesern trifft, belegt ueber
    // > den Waechter nichts.
    //
    // Wer ihn zum Anschlagen bringen will, muss **alle** Leser eines Feldes
    // treffen. Welche das sind, sieht man dem Code nicht an — deshalb steht es
    // hier, und zwar **gemessen** statt aufgeschrieben.
    let mut mehrfach: Vec<(&str, Vec<&'static str>)> = Vec::new();
    for (name, werte) in felder() {
        let leser = leser_von(&werte);
        assert!(
            !leser.is_empty(),
            "Feld '{name}' hat keinen Leser — das faengt `kein_feld_ohne_leser`, und dieser \
             Fall waere dann ohnehin gegenstandslos."
        );
        if leser.len() > 1 {
            mehrfach.push((name, leser));
        }
    }
    // Die Zahl steht in der MELDUNG, nicht in der Bedingung: Sie darf wachsen.
    eprintln!(
        "Felder mit mehr als einem Leser ({} Stueck):",
        mehrfach.len()
    );
    for (name, leser) in &mehrfach {
        eprintln!("  {name}: {leser:?}");
    }
    assert!(
        !mehrfach.is_empty(),
        "Kein einziges Feld hat mehrere Leser. Dann ist die Warnung dieses Falls \
         gegenstandslos — und wahrscheinlich stimmt die Messung nicht."
    );
}

/// Kein Feld des Wirkungsknotens darf ohne Leser sein (Stop-Bedingung #966).
///
/// # Wer diesen Waechter pruefen will, muss ALLE Leser eines Feldes treffen
///
/// Er prueft „hat **mindestens einen** Leser", nicht „jeder Leser wirkt". **8
/// der 10 Felder haben mehr als einen** — gemessen, siehe
/// [`mehrfach_gelesene_felder_sind_benannt`], das die Liste ausgibt. Wer einen
/// davon abklemmt, sieht Gruen, und das ist die richtige Antwort.
///
/// > **Eine Mutation, die nur einen von mehreren Lesern trifft, belegt ueber
/// > diesen Waechter nichts.**
///
/// Am 2026-08-18 ist eine Abnahme fast daran gescheitert: Ein abgeklemmter
/// `trigger`-Leser liess den Fall gruen, und beinahe waere „der Waechter greift
/// nicht" gemeldet worden — die Beobachtung stimmte, der Schluss nicht.
///
/// Die Liste der Leser steht bewusst **nicht** hier als Text. Sie wuerde
/// altern, sobald jemand einen Leser ergaenzt, und waere dann genau die
/// gepflegte Liste, gegen die dieses ganze Modul gebaut ist.
#[test]
fn kein_feld_ohne_leser() {
    // Kein `assert_eq!(geprueft, 10)` mehr: Dass jedes Feld dabei ist, sichert
    // der Anker von `felder!` beim Übersetzen. Die Zahl steht in der Meldung.
    let mut ohne: Vec<&str> = Vec::new();
    let mut geprueft = 0usize;
    for (name, werte) in felder() {
        geprueft += 1;
        if !hat_leser(name, &werte) {
            ohne.push(name);
        }
    }
    assert!(
        geprueft > 0,
        "Kein Feld geprueft — dann sagt dieser Fall nichts."
    );
    assert!(
        ohne.is_empty(),
        "Diese Felder des Wirkungsknotens haben KEINEN Leser — kein einziges Wertepaar fuehrt \
         zu verschiedenen Ableitungen: {ohne:?}. Ein Feld ohne Leser ist die gepflegte Liste \
         eine Ebene tiefer: Es sieht nach Semantik aus und traegt keine. (Stop-Bedingung von \
         Issue #966.) Geprueft wurden {geprueft} Felder."
    );
}

#[test]
fn kein_wert_der_wertelisten_erzeugt_eine_luecke() {
    // Die Zusicherung ueber die Zusicherung: Enthielte eine Werteliste einen
    // `Unknown`-Wert, waere `kein_feld_ohne_leser` gruen, ohne etwas ueber
    // Leser zu sagen — die Lueckenpruefung allein aendert schon jedes
    // Ergebnis. Dieser Fall haelt fest, dass ueber ALLE Felder, Werte und
    // Bausteine geprueft wird.
    let mut paare = 0usize;
    for (name, werte) in felder() {
        for setzen in &werte {
            for op in basis_bausteine() {
                let mut knoten = beispielknoten(op);
                setzen(&mut knoten);
                ohne_luecke(&knoten, &format!("{name} bei {}", op.name()));
                paare += 1;
            }
        }
    }
    let erwartet: usize =
        felder().iter().map(|(_, w)| w.len()).sum::<usize>() * basis_bausteine().len();
    assert_eq!(
        paare, erwartet,
        "Es wurden {paare} Paare geprueft, erwartet {erwartet} — die Schleife hat nicht alles \
         angesehen."
    );
    assert!(
        paare > 0,
        "Kein einziges Paar geprueft — dann sichert dieser Fall nichts ab."
    );
}
