//! Die Pruef-Zauber tragen verschiedene Zielgroessen (Issue #489, #562).
//!
//! **Diese Datei prueft die DATEN, nicht die Wirkung.** Sie belegt, dass
//! `test-heat` das Feld `target: Resource` traegt — nicht, dass der Faktor im
//! Kampf etwas bewegt. Die Wirkungspruefung steht auf beiden Seiten:
//! `tests/unit/buff-targets.test.ts` im Spiel und `sim/tests/buff_wirkung.rs`
//! in Rust (Issue #502, seit dem 2026-08-07 geschlossen — der Kopf nannte es
//! bis dahin als offen, und wer das las, hielt die Rust-Wirkung fuer
//! ungeprueft).
//!
//! Kippte einer per Tippfehler auf `damage` zurueck, maessen die Faelle dort
//! stillschweigend mehrfach denselben Vorrat. Das ist der Zweck dieser Datei —
//! und ihre Grenze.
//!
//! # Warum der alte Kopf hier stand
//!
//! Er beschrieb eine Wirkungspruefung ("dass er den SCHADEN dabei nicht
//! bewegt, zeigt, dass die fuenf Vorraete wirklich getrennt sind") ueber
//! einer Datei, die nichts davon tut. Der Satz war wahr und zeigte auf die
//! falsche Datei — wer ihn las, hielt die Rust-Seite fuer geprueft. Genau so
//! ist sie abgenommen worden.
use rota_sim::data::{BuffTarget, SPELLS};

/// Findet einen Zauber ueber seine Kennung.
fn zauber(id: &str) -> &'static rota_sim::data::Spell {
    SPELLS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("Zauber {id} fehlt im erzeugten Datensatz"))
}

/// Jeder Verstaerker im Datensatz und die Groesse, auf die er wirkt.
///
/// **Diese Liste ist vollstaendig, und `die_liste_fuehrt_jeden_verstaerker`
/// haelt sie so.** Zweimal ist sie hinter dem Datensatz zurueckgeblieben:
/// `test-cdreduce` und `test-convert` kamen mit E10 dazu, ohne einzutragen
/// (#562), und die sechs SPIELBAREN fehlten von Anfang an ganz (#660).
///
/// Beide Male sah sie aus wie ein Waechter und war eine Stichprobe — gruen,
/// mit dem Namen einer Pruefung, und ohne irgendwo zu sagen, dass sie nur
/// einen Teil ihrer Gattung fuehrt.
const ERWARTET: &[(&str, BuffTarget)] = &[
    // Die Pruef-Zauber.
    ("test-buff", BuffTarget::Damage),
    ("test-control", BuffTarget::Control),
    ("test-heat", BuffTarget::Resource),
    ("test-tick", BuffTarget::Tick),
    ("test-stack", BuffTarget::Stack),
    ("test-cdreduce", BuffTarget::CdReduce),
    ("test-convert", BuffTarget::Convert),
    ("test-cap", BuffTarget::Cap),
    // **Und die SPIELBAREN — sie fehlten hier ganz** (Issue #660).
    //
    // Gemessen am 2026-08-08: `distil.buff.target` von `cdreduce` auf
    // `control` gedreht ergab VOR dem Export zwei rote Faelle, und beide
    // sagten nur "die Aufzeichnung passt nicht mehr zu den Daten". NACH
    // `npm run parity:export` war der Bestand vollstaendig gruen — mit einer
    // Verstaerkung, die auf die falsche Groesse wirkt.
    //
    // Wer den normalen Arbeitsablauf faehrt (Daten aendern, exportieren,
    // testen), bekam also gruen fuer eine kaputte Zuordnung. Diese Eintraege
    // haengen an keiner Aufzeichnung und bleiben deshalb auch nach einem
    // Export rot.
    ("slag", BuffTarget::Cap),
    ("flashpoint", BuffTarget::Resource),
    ("distil", BuffTarget::CdReduce),
    ("griplock", BuffTarget::Control),
    ("virulence", BuffTarget::Stack),
    ("quicksilver", BuffTarget::Convert),
];

#[test]
fn die_verstaerker_tragen_die_erwarteten_zielgroessen() {
    // **Zauber, die alle dasselbe Fenster oeffnen, belegten mehrfach
    // dasselbe.** Diese Pruefung faengt genau das: Kippt einer versehentlich
    // auf `Damage` zurueck — etwa durch einen Tippfehler im `target`-Feld, den
    // der Parser dann als "fehlt, also Schaden" liest —, messen die Faelle
    // anderswo stillschweigend mehrfach denselben Vorrat.
    for &(id, ziel) in ERWARTET {
        let buff = zauber(id)
            .buff
            .unwrap_or_else(|| panic!("{id} traegt gar keine Verstaerkung"));
        assert_eq!(buff.target, ziel, "{id} wirkt auf die falsche Groesse");
    }
}

/// **Der Waechter ueber der Liste** (Issue #660).
///
/// Die Liste oben ist eine Abschrift des Datensatzes, und Abschriften altern.
/// Zweimal ist genau das passiert, ohne dass ein Test es gemeldet haette —
/// beide Male fiel es nur auf, weil jemand aus einem anderen Grund in der
/// Datei war.
///
/// Dieser Fall dreht die Richtung um: Er geht vom DATENSATZ aus und verlangt,
/// dass jeder Verstaerker darin oben steht. Ein neuer Zauber mit `buff`-Feld
/// macht ihn rot, bis er eingetragen ist.
#[test]
fn die_liste_fuehrt_jeden_verstaerker_des_datensatzes() {
    let im_datensatz: Vec<&str> = SPELLS
        .iter()
        .filter(|s| s.buff.is_some())
        .map(|s| s.id)
        .collect();

    // **Die Zusicherung vor der Messung.** Faende der Filter nichts — etwa
    // weil `buff` anders heisst —, waere der Vergleich unten zwischen zwei
    // leeren Mengen und gruen, ohne etwas zu belegen.
    assert!(
        im_datensatz.len() >= 10,
        "nur {} Verstaerker im Datensatz gefunden — der Filter greift nicht",
        im_datensatz.len()
    );

    let fehlen: Vec<&str> = im_datensatz
        .iter()
        .copied()
        .filter(|id| !ERWARTET.iter().any(|(e, _)| e == id))
        .collect();
    assert!(
        fehlen.is_empty(),
        "diese Verstaerker stehen im Datensatz, aber in keiner Zeile von \
         ERWARTET: {fehlen:?} — bis sie eingetragen sind, ist ihre Zielgroesse \
         nur ueber Aufzeichnungen gedeckt, und die faerben nach einem \
         parity:export nicht mehr"
    );

    // Die Gegenrichtung: kein Eintrag ohne Zauber. Sonst bliebe eine Zeile
    // stehen, wenn ein Zauber verschwindet, und behauptete eine Deckung, die
    // es nicht gibt.
    let verwaist: Vec<&str> = ERWARTET
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !im_datensatz.contains(e))
        .collect();
    assert!(
        verwaist.is_empty(),
        "diese Eintraege haben keinen Zauber mehr im Datensatz: {verwaist:?}"
    );
}

#[test]
fn ohne_feld_bleibt_es_beim_schaden() {
    // Der bestehende Pruef-Zauber traegt kein `target`. Waere der Vorgabewert
    // etwas anderes als `Damage`, waeren alle Zauber von vor #489 still auf
    // eine andere Groesse gewandert.
    assert_eq!(zauber("test-buff").buff.unwrap().target, BuffTarget::Damage);
}
