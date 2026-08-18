//! Die Wirkungsbeschreibung stammt aus den Daten — nicht aus einer Liste.
//!
//! # Warum dieser Prueflauf noetig ist
//!
//! Ein Modul, das nie mit echten Daten gefuellt wird, ist eine gebaute Leitung
//! ohne Anschluss: Jede Zusicherung darin gilt, und keine davon betrifft das
//! Spiel. Diese Datei prueft die Speisung selbst — dass [`EFFECT_NODES`] aus
//! `data/*.json` entsteht und mit der Zauber-Tabelle daneben zusammenpasst.
//!
//! Die Kreuzprobe ist der Kern: [`SPELLS`] und `EFFECT_NODES` werden im selben
//! Bauschritt, aber aus getrennten Funktionen erzeugt. Laufen sie auseinander,
//! faellt es hier auf und nirgends sonst.

use rota_sim::data::{EFFECT_NODES, SPELLS};
use rota_sim::effects::registry::{alle_analysen, Verdict};
use rota_sim::effects::EffectOp;

/// Die elf Zusatz-Systeme der Bossdaten.
///
/// **Eine Liste, keine Zahl** — bei der Planung stand hier „sieben", und
/// nachgezaehlt waren es elf. Eine Zahl haette den Irrtum nicht gemeldet,
/// diese Liste nennt beim Namen, was fehlt.
const BOSS_SYSTEME: &[&str] = &[
    "energy",
    "doom",
    "affliction",
    "race",
    "runaway",
    "stacks",
    "escalation",
    "bossHeal",
    "timelineLoopMs",
    "resistances",
];

#[test]
fn die_beschreibung_ist_nicht_leer() {
    assert!(
        !EFFECT_NODES.is_empty(),
        "EFFECT_NODES ist leer — dann prueft jede Zusicherung darunter nichts."
    );
    // Die Zahl steht in der Meldung, nicht in der Bedingung: Sie darf wachsen.
    let signaturen = EFFECT_NODES
        .iter()
        .filter(|n| matches!(n.op, EffectOp::CustomSignature(_)))
        .count();
    assert!(
        signaturen < EFFECT_NODES.len(),
        "Alle {} Knoten sind Signaturen. Dann traegt kein einziger Zauber eine beschriebene \
         Wirkung, und die Beschreibung beschreibt nur Bosse.",
        EFFECT_NODES.len()
    );
}

#[test]
fn jeder_zauber_mit_grundschaden_hat_einen_schadensknoten() {
    // Die Kreuzprobe: SPELLS und EFFECT_NODES entstehen aus derselben Quelle,
    // aber in getrennten Funktionen. Ein Leser, der ein Feld mit dem falschen
    // JSON-Typ liest, laesst den Zauber hier fehlen — genau so fehlten beim
    // Bau von #911 vier Zauber mit Ressourcen-Wirkung, weil `resourceGain` ein
    // Objekt ist und keine Zahl.
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.base_damage == 0 {
            continue;
        }
        erwartet += 1;
        let hat_knoten = EFFECT_NODES
            .iter()
            .any(|n| n.origin == spell.id && matches!(n.op, EffectOp::DealDamage));
        if !hat_knoten {
            fehlend.push(spell.id);
        }
    }
    assert!(
        erwartet > 0,
        "Kein einziger Zauber traegt Grundschaden — dann prueft dieser Fall nichts."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {} Zaubern mit Grundschaden fehlen in der Wirkungsbeschreibung: {fehlend:?}",
        fehlend.len(),
        erwartet
    );
}

#[test]
fn jeder_zauber_mit_brand_hat_einen_brandknoten() {
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.dot.is_none() {
            continue;
        }
        erwartet += 1;
        if !EFFECT_NODES
            .iter()
            .any(|n| n.origin == spell.id && matches!(n.op, EffectOp::ApplyDot))
        {
            fehlend.push(spell.id);
        }
    }
    assert!(erwartet > 0, "Kein Zauber traegt einen Brand-Effekt.");
    assert!(
        fehlend.is_empty(),
        "{} von {erwartet} Zaubern mit Brand fehlen: {fehlend:?}",
        fehlend.len()
    );
}

#[test]
fn jeder_zauber_mit_abklingzeit_verkuerzung_hat_seinen_knoten() {
    // Der Fall, an dem sich die Planung geirrt hat: „der Abklingzeit-Ruecksetzer
    // kommt erst noch" — tatsaechlich tragen mehrere Zauber solche Effekte
    // bereits, und der Simulator kennt die Felder.
    let mut fehlend: Vec<&str> = Vec::new();
    let mut erwartet = 0usize;
    for spell in SPELLS.iter() {
        if spell.cooldown_reduction_ms == 0 && spell.cooldown_reset_charges == 0 {
            continue;
        }
        erwartet += 1;
        if !EFFECT_NODES.iter().any(|n| {
            n.origin == spell.id
                && matches!(n.op, EffectOp::ReduceCooldown | EffectOp::ResetCooldown)
        }) {
            fehlend.push(spell.id);
        }
    }
    assert!(
        erwartet > 0,
        "Kein Zauber verkuerzt oder setzt Abklingzeiten zurueck — dann prueft dieser Fall \
         nichts, und die Planungs-Aussage waere doch richtig gewesen."
    );
    assert!(
        fehlend.is_empty(),
        "{} von {erwartet} Zaubern mit Abklingzeit-Wirkung fehlen: {fehlend:?}",
        fehlend.len()
    );
}

#[test]
fn alle_elf_boss_systeme_sind_beschrieben() {
    let vorhanden: Vec<&str> = EFFECT_NODES
        .iter()
        .filter_map(|n| match n.op {
            EffectOp::CustomSignature(id) => Some(id.0),
            _ => None,
        })
        .collect();
    let mut fehlend: Vec<&str> = Vec::new();
    for system in BOSS_SYSTEME {
        if !vorhanden.contains(system) {
            fehlend.push(system);
        }
    }
    assert!(
        fehlend.is_empty(),
        "Diese Boss-Systeme haben keinen Knoten: {fehlend:?}. Sie stehen in `data/bosses/`, \
         werden aber von keiner Analyse gesehen."
    );
    // Das elfte System sind die Phasen — sie tragen einen eigenen Baustein und
    // stehen deshalb nicht in der Signatur-Liste.
    assert!(
        EFFECT_NODES
            .iter()
            .any(|n| matches!(n.op, EffectOp::ChangePhase)),
        "Kein einziger Phasen-Knoten — dabei fuehren 33 von 35 Bossdateien Phasen."
    );
}

#[test]
fn die_echten_knoten_sind_nicht_alle_konservativ() {
    // Waeren sie es, waere die ganze Beschreibung wertlos: Jede Optimierung
    // fiele zurueck, und niemand merkte den Unterschied zu „gar keine
    // Beschreibung".
    let mut mit_erlaubnis = 0usize;
    for knoten in EFFECT_NODES.iter() {
        if alle_analysen(knoten)
            .iter()
            .any(|r| matches!(r.verdict, Verdict::Permits))
        {
            mit_erlaubnis += 1;
        }
    }
    assert!(
        mit_erlaubnis > 0,
        "Kein einziger der {} Knoten bekommt von irgendeiner Analyse eine Erlaubnis. Dann ist \
         die Beschreibung eine gebaute Leitung ohne Anschluss.",
        EFFECT_NODES.len()
    );
}

#[test]
fn jeder_knoten_nennt_seine_herkunft() {
    for knoten in EFFECT_NODES.iter() {
        assert!(
            !knoten.origin.is_empty(),
            "Ein Knoten ({:?}) nennt keine Herkunft. Ohne sie ist ein Befund nicht auffindbar.",
            knoten.op
        );
    }
}

#[test]
fn ein_knoten_ohne_semantik_in_den_echten_daten_faellt_ueberall_zurueck() {
    // # Der Waechter fuer AC5 an echten Daten
    //
    // Im Bestand findet er nichts: Alle elf Boss-Systeme sind eingeordnet.
    // Kommt ein zwoelftes dazu, ohne dass jemand seine Semantik deklariert,
    // erzeugt `build.rs` einen Knoten ohne jede Angabe — und dieser Fall
    // haelt fest, dass der dann WIRKLICH ueberall zurueckfaellt statt
    // guenstig durchzulaufen.
    //
    // **Gemessen am 2026-08-18 als Differenz zweier Laeufe:** mit einem
    // testweise eingefuegten Feld `erfundenesZusatzSystem` in
    // `data/bosses/first-crucible.json` sah dieser Fall 1 Knoten und alle
    // Analysen fielen zurueck; ohne das Feld sieht er 0. Das Feld wurde
    // danach wieder entfernt.
    let ohne_semantik: Vec<&rota_sim::effects::EffectNode> = EFFECT_NODES
        .iter()
        .filter(|n| n.signature_ohne_semantik())
        .collect();

    eprintln!(
        "Knoten ohne Semantik in den echten Daten: {} von {}",
        ohne_semantik.len(),
        EFFECT_NODES.len()
    );

    for knoten in &ohne_semantik {
        let ergebnisse = alle_analysen(knoten);
        let zurueckgefallen = ergebnisse
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Conservative))
            .count();
        assert_eq!(
            zurueckgefallen,
            ergebnisse.len(),
            "Der Knoten \"{}\" hat keine Semantik, aber nur {zurueckgefallen} von {} Analysen \
             sind zurueckgefallen. Ein unbekanntes Zusatz-System darf nicht guenstig \
             durchlaufen — unbekannt muss langsam sein, nie falsch.",
            knoten.origin,
            ergebnisse.len()
        );
    }
}

#[test]
fn jeder_zauber_hat_mindestens_einen_knoten() {
    // # Die Pruefung, die von Anfang an gefehlt hat
    //
    // Beim ersten Bau von #911 bekamen **13 von 70** Zaubern gar keinen
    // Knoten: die sieben Kontroll-Zauber, `purge`, `quench`, `harden`,
    // `sepsis`, `decay` und `putrid-bloom`. Der Bau war gruen, alle
    // Zusicherungen hielten, und die Beschreibung war fuer ein Fuenftel der
    // Zauber blind.
    //
    // Die Einzelpruefungen darueber konnten das nicht finden: Sie fragen je
    // Wirkungsart, und wer gar keine der geprueften Arten traegt, faellt durch
    // jedes Raster. **Dieser Fall fragt von der anderen Seite** — nicht „hat
    // jede Art ihre Zauber?", sondern „hat jeder Zauber seine Beschreibung?".
    //
    // Ein Zauber ohne jede Wirkung ist selbst ein Befund und kein Grund, diese
    // Zusicherung aufzuweichen.
    let mut ohne: Vec<&str> = Vec::new();
    for spell in SPELLS.iter() {
        if !EFFECT_NODES.iter().any(|n| n.origin == spell.id) {
            ohne.push(spell.id);
        }
    }
    assert!(
        !SPELLS.is_empty(),
        "Es gibt keine Zauber — dann prueft dieser Fall nichts."
    );
    assert!(
        ohne.is_empty(),
        "{} von {} Zaubern haben KEINEN einzigen Knoten in der Wirkungsbeschreibung: \
         {ohne:?}. Entweder liest `knoten_fuer_zauber` ihre Felder nicht, oder ein neues \
         Wirkungsfeld ist noch nirgends eingeordnet.",
        ohne.len(),
        SPELLS.len()
    );
}
